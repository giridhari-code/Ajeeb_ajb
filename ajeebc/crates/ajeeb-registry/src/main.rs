use axum::{
    Router, extract::{Path, Query, State},
    http::{StatusCode, HeaderMap},
    response::{IntoResponse, Json},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use tower_http::cors::CorsLayer;
use tracing::info;

// ── Types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageIndex {
    versions: HashMap<String, VersionMeta>,
    metadata: PkgMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VersionMeta {
    checksum: String,
    signature: Option<String>,
    published_at: String,
    yanked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PkgMeta {
    description: String,
    author: String,
    homepage: String,
    license: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenEntry {
    username: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

#[derive(Debug, Serialize)]
struct SearchResult {
    name: String,
    latest_version: String,
    description: String,
}

#[derive(Debug, Deserialize)]
struct PublishRequest {
    name: String,
    version: String,
    author: String,
    description: String,
    checksum: String,
    signature: Option<String>,
}

#[derive(Debug, Serialize)]
struct MeResponse {
    username: String,
    token: String,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    token: String,
    username: String,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize)]
struct MetadataResponse {
    name: String,
    versions: Vec<String>,
    description: String,
    author: String,
    homepage: String,
    license: String,
    created_at: String,
}

// ── State ───────────────────────────────────────────────────────────

struct AppState {
    data_dir: PathBuf,
    tokens: Mutex<HashMap<String, TokenEntry>>,
}

type SharedState = Arc<AppState>;

// ── Helpers ─────────────────────────────────────────────────────────

fn index_path(state: &AppState) -> PathBuf {
    state.data_dir.join("packages").join("index.json")
}

fn packages_dir(state: &AppState) -> PathBuf {
    state.data_dir.join("packages")
}

fn tarball_path(state: &AppState, name: &str, version: &str) -> PathBuf {
    packages_dir(state)
        .join(sanitize(name))
        .join(format!("{}.tar.gz", version))
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' { c } else { '_' })
        .collect()
}

fn load_index(state: &AppState) -> HashMap<String, PackageIndex> {
    let path = index_path(state);
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

fn save_index(state: &AppState, index: &HashMap<String, PackageIndex>) -> Result<(), String> {
    let path = index_path(state);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(index).map_err(|e| format!("Serialize error: {}", e))?;
    fs::write(&path, &json).map_err(|e| format!("Cannot write index: {}", e))?;
    Ok(())
}

fn load_tokens(state: &AppState) -> HashMap<String, TokenEntry> {
    let path = state.data_dir.join("tokens.json");
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

fn save_tokens(state: &AppState, tokens: &HashMap<String, TokenEntry>) -> Result<(), String> {
    let path = state.data_dir.join("tokens.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(tokens).map_err(|e| format!("Serialize error: {}", e))?;
    fs::write(&path, &json).map_err(|e| format!("Cannot write tokens: {}", e))?;
    Ok(())
}

fn verify_token(state: &AppState, headers: &HeaderMap) -> Result<String, StatusCode> {
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token_hash = hex::encode(Sha256::digest(auth.as_bytes()));
    let tokens = state.tokens.lock().unwrap();
    tokens
        .get(&token_hash)
        .map(|t| t.username.clone())
        .ok_or(StatusCode::UNAUTHORIZED)
}

// ── Handlers ────────────────────────────────────────────────────────

/// List all packages (simple HTML index)
async fn list_packages(State(state): State<SharedState>) -> impl IntoResponse {
    let index = load_index(&state);
    let mut names: Vec<&String> = index.keys().collect();
    names.sort();

    let mut html = String::from("<html><head><title>Ajeeb Registry</title></head><body>");
    html.push_str("<h1>📦 Ajeeb Registry</h1>");
    html.push_str("<p>API: <code>/api/v1/...</code></p><ul>");
    for name in &names {
        html.push_str(&format!("<li><a href=\"/api/v1/packages/{}\">{}</a></li>", name, name));
    }
    html.push_str("</ul></body></html>");
    (StatusCode::OK, [("content-type", "text/html")], html)
}

/// GET /api/v1/packages/{name} - get package metadata
async fn get_package_metadata(
    State(state): State<SharedState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let index = load_index(&state);
    let safe = sanitize(&name);

    match index.get(&safe) {
        Some(pkg) => {
            let mut versions: Vec<String> = pkg.versions.keys().cloned().collect();
            versions.sort_by(|a, b| {
                let va: Vec<u64> = a.split('.').filter_map(|s| s.parse().ok()).collect();
                let vb: Vec<u64> = b.split('.').filter_map(|s| s.parse().ok()).collect();
                va.cmp(&vb)
            });

            let resp = MetadataResponse {
                name: safe,
                versions,
                description: pkg.metadata.description.clone(),
                author: pkg.metadata.author.clone(),
                homepage: pkg.metadata.homepage.clone(),
                license: pkg.metadata.license.clone(),
                created_at: pkg.metadata.created_at.clone(),
            };
            (StatusCode::OK, Json(resp)).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Package '{}' not found", name) }),
        ).into_response(),
    }
}

/// GET /api/v1/packages/{name}/{version} - download tarball
/// Also handles {version}.tar.gz suffix (parth appends it)
async fn download_package(
    State(state): State<SharedState>,
    Path((name, version_raw)): Path<(String, String)>,
) -> impl IntoResponse {
    // Strip .tar.gz if present (parth appends it to the URL)
    let version = version_raw.strip_suffix(".tar.gz").unwrap_or(&version_raw).to_string();
    let tarball = tarball_path(&state, &name, &version);
    if tarball.exists() {
        match fs::read(&tarball) {
            Ok(data) => {
                let headers = [
                    ("content-type", "application/gzip"),
                    ("content-disposition", &format!("attachment; filename=\"{}-{}.tar.gz\"", name, version)),
                ];
                (StatusCode::OK, headers, data).into_response()
            }
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: "Cannot read package file".to_string() }),
            ).into_response(),
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Package '{}@{}' not found", name, version) }),
        ).into_response()
    }
}

/// GET /api/v1/search?q=... - search packages
async fn search_packages(
    State(state): State<SharedState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let index = load_index(&state);
    let query = params.q.unwrap_or_default().to_lowercase();

    let mut results: Vec<SearchResult> = Vec::new();
    for (name, pkg) in &index {
        if query.is_empty() || name.to_lowercase().contains(&query) || pkg.metadata.description.to_lowercase().contains(&query) {
            let latest = pkg.versions.keys()
                .max_by(|a, b| {
                    let va: Vec<u64> = a.split('.').filter_map(|s| s.parse().ok()).collect();
                    let vb: Vec<u64> = b.split('.').filter_map(|s| s.parse().ok()).collect();
                    va.cmp(&vb)
                })
                .cloned()
                .unwrap_or_default();
            results.push(SearchResult {
                name: name.clone(),
                latest_version: latest,
                description: pkg.metadata.description.clone(),
            });
        }
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    (StatusCode::OK, Json(results))
}

/// GET /api/v1/me - validate token
async fn me(State(state): State<SharedState>, headers: HeaderMap) -> impl IntoResponse {
    match verify_token(&state, &headers) {
        Ok(username) => {
            let auth = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
            let token = auth.strip_prefix("Bearer ").unwrap_or("").to_string();
            (StatusCode::OK, Json(MeResponse { username, token })).into_response()
        }
        Err(status) => (
            status,
            Json(ErrorResponse { error: "Invalid or missing token".to_string() }),
        ).into_response(),
    }
}

/// POST /api/v1/packages - publish a package (auth required)
async fn publish_package(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Json(payload): Json<PublishRequest>,
) -> impl IntoResponse {
    let username = match verify_token(&state, &headers) {
        Ok(u) => u,
        Err(status) => {
            return (
                status,
                Json(ErrorResponse { error: "Authentication required".to_string() }),
            ).into_response();
        }
    };

    let safe_name = sanitize(&payload.name);
    let safe_version = sanitize(&payload.version);

    // Validate semver
    let parts: Vec<&str> = payload.version.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.parse::<u64>().is_err()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "Invalid semantic version (must be X.Y.Z)".to_string() }),
        ).into_response();
    }

    let mut index = load_index(&state);

    // Create or update package entry
    let pkg_entry = index.entry(safe_name.clone()).or_insert_with(|| PackageIndex {
        versions: HashMap::new(),
        metadata: PkgMeta {
            description: payload.description.clone(),
            author: payload.author.clone(),
            homepage: String::new(),
            license: String::new(),
            created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        },
    });

    // Update metadata if new fields provided
    if !payload.description.is_empty() {
        pkg_entry.metadata.description = payload.description.clone();
    }
    if !payload.author.is_empty() {
        pkg_entry.metadata.author = payload.author.clone();
    }

    // Check if version already exists
    if pkg_entry.versions.contains_key(&safe_version) {
        // Allow re-publish with same checksum (idempotent)
        let existing = &pkg_entry.versions[&safe_version];
        if existing.checksum != payload.checksum {
            return (
                StatusCode::CONFLICT,
                Json(ErrorResponse {
                    error: format!("Version '{}' already exists with different checksum. Use `parth yank` first.", payload.version),
                }),
            ).into_response();
        }
        return (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "message": "Version already published with matching checksum"})),
        ).into_response();
    }

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    pkg_entry.versions.insert(safe_version.clone(), VersionMeta {
        checksum: payload.checksum.clone(),
        signature: payload.signature,
        published_at: now,
        yanked: false,
    });

    // Save index
    if let Err(e) = save_index(&state, &index) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: format!("Failed to save index: {}", e) }),
        ).into_response();
    }

    info!("Package '{}@{}' published by '{}'", safe_name, safe_version, username);
    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "status": "ok",
            "name": safe_name,
            "version": safe_version,
            "checksum": payload.checksum,
        })),
    )
    .into_response()
}

/// POST /api/v1/login - authenticate and get token
async fn login(
    State(state): State<SharedState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    if req.username.is_empty() || req.password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: "Username and password required".to_string() }),
        ).into_response();
    }

    // Generate a simple token (username:random_hex)
    let random_bytes: [u8; 16] = rand::random();
    let token = format!("{}:{}", req.username, hex::encode(random_bytes));

    let token_hash = hex::encode(Sha256::digest(token.as_bytes()));
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let mut tokens = load_tokens(&state);
    tokens.insert(token_hash, TokenEntry {
        username: req.username.clone(),
        created_at: now,
    });

    if let Err(e) = save_tokens(&state, &tokens) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: format!("Failed to save token: {}", e) }),
        ).into_response();
    }

    info!("User '{}' logged in", req.username);
    (
        StatusCode::OK,
        Json(LoginResponse {
            token,
            username: req.username,
        }),
    )
    .into_response()
}

/// GET /api/v1/advisories.json - advisories list
async fn advisories(State(state): State<SharedState>) -> impl IntoResponse {
    let path = state.data_dir.join("advisories.json");
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(content) => {
                return (StatusCode::OK, [("content-type", "application/json")], content).into_response();
            }
            Err(_) => {}
        }
    }
    (StatusCode::OK, [("content-type", "application/json")], "[]".to_string()).into_response()
}

/// Upload a tarball (separate from metadata publish)
async fn upload_tarball(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((name, version)): Path<(String, String)>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    if let Err(status) = verify_token(&state, &headers) {
        return (
            status,
            Json(ErrorResponse { error: "Authentication required".to_string() }),
        ).into_response();
    }

    let safe_name = sanitize(&name);
    let safe_version = sanitize(&version);
    let tarball = tarball_path(&state, &safe_name, &safe_version);
    if let Some(parent) = tarball.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: format!("Cannot create dir: {}", e) }),
            ).into_response();
        }
    }

    if let Err(e) = fs::write(&tarball, &body) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: format!("Cannot write tarball: {}", e) }),
        ).into_response();
    }

    info!("Tarball uploaded: '{}@{}' ({} bytes)", safe_name, safe_version, body.len());
    (
        StatusCode::CREATED,
        Json(serde_json::json!({"status": "ok", "size": body.len()})),
    )
    .into_response()
}

// ── Main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let data_dir = std::env::var("AJeEB_REGISTRY_DATA")
        .unwrap_or_else(|_| "registry-data".to_string());
    let bind = std::env::var("AJeEB_REGISTRY_BIND")
        .unwrap_or_else(|_| "0.0.0.0:9876".to_string());

    fs::create_dir_all(&data_dir).expect("Cannot create data directory");
    info!("Ajeeb Registry starting on {}, data dir: {}", bind, data_dir);

    // Generate a default admin token if none exist
    let state = {
        let data_path = PathBuf::from(&data_dir);
        let state = Arc::new(AppState {
            data_dir: data_path,
            tokens: Mutex::new(HashMap::new()),
        });

        // Load existing tokens
        let tokens = load_tokens(&state);
        if tokens.is_empty() {
            let admin_hash = hex::encode(sha2::Sha256::digest(b"admin").as_slice());
            let admin_token = format!("admin:{}", &admin_hash[..16]);
            let token_hash = hex::encode(Sha256::digest(admin_token.as_bytes()));
            let mut new_tokens = HashMap::new();
            new_tokens.insert(token_hash, TokenEntry {
                username: "admin".to_string(),
                created_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            });
            let _ = save_tokens(&state, &new_tokens);
            info!("Generated default admin token: {}", admin_token);
        }

        // Re-acquire tokens lock with loaded data
        let loaded_tokens = load_tokens(&state);
        *state.tokens.lock().unwrap() = loaded_tokens;
        state
    };

    let app = Router::new()
        .route("/", get(list_packages))
        .route("/api/v1/search", get(search_packages))
        .route("/api/v1/me", get(me))
        .route("/api/v1/login", post(login))
        .route("/api/v1/packages", post(publish_package))
        .route("/api/v1/packages/{name}", get(get_package_metadata))
        .route("/api/v1/packages/{name}/{version}", get(download_package))
        .route("/api/v1/packages/{name}/{version}/upload", post(upload_tarball))
        .route("/api/v1/advisories.json", get(advisories))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind).await.unwrap();
    info!("Listening on http://{}", bind);
    axum::serve(listener, app).await.unwrap();
}
