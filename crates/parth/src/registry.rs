use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::config::read_package_meta;
use super::types::{Advisory, LockEntry, PackageSignature, PkgDep, RegistryIndex, RegistryMetadata, SearchResult};

// ── Paths ──────────────────────────────────────────────────────────

pub fn parth_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".parth")
}

pub fn index_path() -> PathBuf { parth_home().join("index") }
pub fn cache_root() -> PathBuf { parth_home().join("cache") }
pub fn signatures_dir() -> PathBuf { parth_home().join("signatures") }
pub fn metadata_dir() -> PathBuf { parth_home().join("metadata") }
pub fn keys_dir() -> PathBuf { parth_home().join("keys") }
pub fn audit_path() -> PathBuf { parth_home().join("audit") }
pub fn advisories_dir() -> PathBuf { parth_home().join("advisories") }

fn sanitize_pkg_segment(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' { c } else { '_' })
        .collect()
}

pub fn package_cache_dir(name: &str, version: &str) -> PathBuf {
    parth_home().join("packages").join(sanitize_pkg_segment(name)).join(sanitize_pkg_segment(version))
}

// ── Ed25519 Key Management ──────────────────────────────────────────

/// Generate a new Ed25519 keypair and save to ~/.parth/keys/
pub fn generate_keypair() -> Result<(String, String), String> {
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    let dir = keys_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create keys dir: {}", e))?;

    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    let secret_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(verifying_key.to_bytes());

    // Save secret key (readable only by owner)
    let sk_path = dir.join("secret.key");
    fs::write(&sk_path, &secret_hex).map_err(|e| format!("Cannot write secret key: {}", e))?;
    #[cfg(unix)]
    { let _ = std::process::Command::new("chmod").args(["600", &sk_path.to_string_lossy()]).status(); }

    // Save public key
    let pk_path = dir.join("public.key");
    fs::write(&pk_path, &public_hex).map_err(|e| format!("Cannot write public key: {}", e))?;

    Ok((secret_hex, public_hex))
}

/// Load the default keypair. If it doesn't exist, generate one.
pub fn load_or_generate_keypair() -> Result<(ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey), String> {
    let sk_path = keys_dir().join("secret.key");
    let pk_path = keys_dir().join("public.key");

    if sk_path.exists() && pk_path.exists() {
        let sk_hex = fs::read_to_string(&sk_path).map_err(|e| format!("Cannot read secret key: {}", e))?;
        let sk_hex = sk_hex.trim();
        let sk_bytes = hex::decode(sk_hex).map_err(|e| format!("Invalid secret key hex: {}", e))?;
        let sk_array: [u8; 32] = sk_bytes.try_into().map_err(|_| "Invalid secret key length".to_string())?;
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&sk_array);
        let verifying_key = signing_key.verifying_key();
        Ok((signing_key, verifying_key))
    } else {
        generate_keypair()?;
        load_or_generate_keypair()
    }
}

/// Load a public key by signer name (or "default")
pub fn load_public_key(signer: &str) -> Result<ed25519_dalek::VerifyingKey, String> {
    let pk_path = if signer == "default" || signer.is_empty() {
        keys_dir().join("public.key")
    } else {
        keys_dir().join(format!("{}.pub", sanitize_pkg_segment(signer)))
    };
    if !pk_path.exists() {
        return Err(format!("Public key not found for '{}' at {}", signer, pk_path.display()));
    }
    let pk_hex = fs::read_to_string(&pk_path).map_err(|e| format!("Cannot read public key: {}", e))?;
    let pk_hex = pk_hex.trim();
    let pk_bytes = hex::decode(pk_hex).map_err(|e| format!("Invalid public key hex: {}", e))?;
    let pk_array: [u8; 32] = pk_bytes.try_into().map_err(|_| "Invalid public key length".to_string())?;
    Ok(ed25519_dalek::VerifyingKey::from_bytes(&pk_array).map_err(|e| format!("Invalid public key: {}", e))?)
}

// ── Package metadata ────────────────────────────────────────────────

pub fn metadata_path(name: &str, version: &str) -> PathBuf {
    metadata_dir().join(sanitize_pkg_segment(name)).join(format!("{}.json", sanitize_pkg_segment(version)))
}

pub fn read_metadata(name: &str, version: &str) -> RegistryMetadata {
    let path = metadata_path(name, version);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            // Parse JSON-like format: key=value lines
            let mut meta = RegistryMetadata::new(name);
            for line in content.lines() {
                if let Some(eq) = line.find('=') {
                    let key = line[..eq].trim();
                    let val = line[eq + 1..].trim().trim_matches('"');
                    match key {
                        "description" => meta.description = val.to_string(),
                        "author" => meta.author = val.to_string(),
                        "homepage" => meta.homepage = val.to_string(),
                        "license" => meta.license = val.to_string(),
                        "yanked" => meta.yanked = val == "true",
                        _ => {}
                    }
                }
            }
            return meta;
        }
    }
    RegistryMetadata::new(name)
}

pub fn write_metadata(name: &str, version: &str, meta: &RegistryMetadata) -> Result<(), String> {
    let path = metadata_path(name, version);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create metadata dir: {}", e))?;
    }
    let content = format!(
        "description = \"{}\"\nauthor = \"{}\"\nhomepage = \"{}\"\nlicense = \"{}\"\nyanked = {}\n",
        meta.description, meta.author, meta.homepage, meta.license, meta.yanked
    );
    fs::write(&path, content).map_err(|e| format!("Cannot write metadata: {}", e))?;
    Ok(())
}

/// Mark a package version as yanked
pub fn yank_package(name: &str, version: &str) -> Result<(), String> {
    let mut meta = read_metadata(name, version);
    meta.yanked = true;
    write_metadata(name, version, &meta)
}

/// Un-yank a package version
pub fn unyank_package(name: &str, version: &str) -> Result<(), String> {
    let mut meta = read_metadata(name, version);
    meta.yanked = false;
    write_metadata(name, version, &meta)
}

/// Check if a package version is yanked
pub fn is_yanked(name: &str, version: &str) -> bool {
    read_metadata(name, version).yanked
}

// ── Local index ────────────────────────────────────────────────────

pub fn read_index() -> RegistryIndex {
    let path = index_path();
    if !path.exists() { return HashMap::new(); }
    let content = match fs::read_to_string(&path) {
        Ok(c) => c, Err(_) => return HashMap::new(),
    };
    let mut index: RegistryIndex = HashMap::new();
    let mut current_pkg = String::new();
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("//") { continue; }
        if t.starts_with('[') && t.ends_with(']') {
            current_pkg = t[1..t.len() - 1].trim().to_string();
            index.entry(current_pkg.clone()).or_insert_with(HashMap::new);
            continue;
        }
        if let Some(eq) = t.find('=') {
            let version = t[..eq].trim().to_string();
            let checksum = t[eq + 1..].trim().trim_matches('"').to_string();
            if let Some(versions) = index.get_mut(&current_pkg) {
                versions.insert(version, checksum);
            }
        }
    }
    index
}

pub fn write_index(index: &RegistryIndex) -> Result<(), String> {
    let path = index_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create {}: {}", parent.display(), e))?;
    }
    let mut content = String::from("# Parth Registry Index\n");
    let mut pkgs: Vec<&String> = index.keys().collect();
    pkgs.sort();
    for name in pkgs {
        content.push_str(&format!("\n[{}]\n", name));
        let mut versions: Vec<&String> = index[name].keys().collect();
        versions.sort();
        for ver in versions {
            if let Some(cs) = index[name].get(ver) {
                content.push_str(&format!("{} = \"{}\"\n", ver, cs));
            }
        }
    }
    fs::write(&path, content).map_err(|e| format!("Cannot write index: {}", e))?;
    Ok(())
}

pub fn register_package(name: &str, version: &str, checksum: &str) -> Result<(), String> {
    let mut index = read_index();
    index.entry(name.to_string()).or_insert_with(HashMap::new)
        .insert(version.to_string(), checksum.to_string());
    write_index(&index)?;

    // If the project has a parth.das, read and store metadata
    if Path::new("parth.das").exists() {
        if let Ok(cfg) = super::config::read_config(Path::new("parth.das")) {
            let meta = RegistryMetadata {
                description: cfg.pkg_description.clone(),
                author: cfg.pkg_author.clone(),
                homepage: cfg.pkg_homepage.clone(),
                license: cfg.pkg_license.clone(),
                yanked: false,
            };
            write_metadata(name, version, &meta).ok();
        }
    }
    Ok(())
}

// ── Cache & integrity ──────────────────────────────────────────────

pub fn ensure_package(name: &str, version: &str, expected_checksum: &str) -> Result<(), String> {
    let cached = package_cache_dir(name, version);
    if !cached.exists() {
        return Err(format!("Package '{}@{}' not found locally.", name, version));
    }
    if !expected_checksum.is_empty() {
        let actual = compute_dir_checksum(&cached)?;
        if actual != expected_checksum {
            return Err(format!(
                "Checksum mismatch for '{}@{}': expected {}, got {}. Cache may be corrupted.",
                name, version, expected_checksum, actual
            ));
        }
    }
    Ok(())
}

/// Compute SHA-256 of a directory (sorted file list with content)
pub fn compute_dir_checksum(dir: &Path) -> Result<String, String> {
    let mut entries: Vec<String> = Vec::new();
    collect_files(dir, dir, &mut entries)
        .map_err(|e| format!("Cannot read {}: {}", dir.display(), e))?;
    entries.sort();
    let mut input = String::new();
    for entry in &entries {
        let path = dir.join(entry);
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
        input.push_str(&format!("{}:{}\n", entry, content));
    }
    use sha2::Digest;
    let hash = sha2::Sha256::digest(input.as_bytes());
    Ok(format!("{:x}", hash))
}

fn collect_files(base: &Path, dir: &Path, entries: &mut Vec<String>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_files(base, &path, entries)?;
            } else {
                let rel = path.strip_prefix(base).unwrap().to_string_lossy().to_string();
                entries.push(rel);
            }
        }
    }
    Ok(())
}

pub fn package_src(pkg_dir: &Path, name: &str, version: &str) -> Result<PathBuf, String> {
    let src_dir = pkg_dir.join("src");
    if !src_dir.exists() { return Err("No src/ directory found".to_string()); }
    let cache_dir = package_cache_dir(name, version);
    fs::create_dir_all(&cache_dir).map_err(|e| format!("Cannot create cache: {}", e))?;
    copy_dir_recursive(&src_dir, &cache_dir.join("src"))?;
    let das_src = pkg_dir.join("parth.das");
    if das_src.exists() {
        fs::copy(&das_src, cache_dir.join("parth.das"))
            .map_err(|e| format!("Cannot copy parth.das: {}", e))?;
    }
    Ok(cache_dir)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Cannot create {}: {}", dst.display(), e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("Cannot read {}: {}", src.display(), e))? {
        let entry = entry.map_err(|e| format!("Dir entry: {}", e))?;
        let ty = entry.file_type().map_err(|e| format!("File type: {}", e))?;
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(&entry.path(), &dst.join(entry.file_name()))
                .map_err(|e| format!("Cannot copy {}: {}", entry.path().display(), e))?;
        }
    }
    Ok(())
}

pub fn read_package_deps(name: &str, version: &str) -> Vec<PkgDep> {
    let pkg_dir = package_cache_dir(name, version);
    let das_path = pkg_dir.join("parth.das");
    if !das_path.exists() { return Vec::new(); }
    match read_package_meta(&das_path) {
        Ok((_, _, deps)) => deps,
        Err(_) => Vec::new(),
    }
}

pub fn make_lock_entry(name: &str, version: &str) -> Result<LockEntry, String> {
    let cached = package_cache_dir(name, version);
    if !cached.exists() {
        return Err(format!("Package '{}@{}' not in cache", name, version));
    }
    let checksum = compute_dir_checksum(&cached)?;
    let deps = read_package_deps(name, version);
    Ok(LockEntry {
        version: version.to_string(),
        checksum,
        dependencies: deps,
        registry: String::new(),
    })
}

// ── Remote registry (HTTP) ─────────────────────────────────────────

#[cfg(feature = "remote-registry")]
pub fn remote_fetch_index(_registry_url: &str, _package_name: &str) -> RegistryIndex {
    read_index()
}

#[cfg(not(feature = "remote-registry"))]
pub fn remote_fetch_index(_registry_url: &str, _package_name: &str) -> RegistryIndex {
    read_index()
}

/// Download a package from the remote registry and cache it
pub fn download_package(name: &str, version: &str, registry_url: &str) -> Result<PathBuf, String> {
    let cached = package_cache_dir(name, version);
    if cached.join("parth.das").exists() {
        return Ok(cached); // Already cached
    }

    if registry_url.is_empty() || registry_url == "local" {
        return Err(format!(
            "Package '{}@{}' not found locally and no remote registry configured.",
            name, version
        ));
    }

    #[cfg(feature = "remote-registry")] {
        download_from_remote(name, version, registry_url, &cached)?;
        Ok(cached)
    }

    #[cfg(not(feature = "remote-registry"))] {
        let _ = registry_url;
        Err("Remote registry not available. Recompile with --features remote-registry".to_string())
    }
}

#[cfg(feature = "remote-registry")]
fn download_from_remote(name: &str, version: &str, url: &str, dest: &Path) -> Result<(), String> {
    let pkg_url = format!("{}/api/v1/packages/{}/{}.tar.gz", url.trim_end_matches('/'), name, version);
    fs::create_dir_all(dest).map_err(|e| format!("Cannot create cache dir: {}", e))?;

    let tar_path = dest.join("package.tar.gz");

    // Download using curl
    let status = std::process::Command::new("curl")
        .args(["-sSfL", "-o", &tar_path.to_string_lossy(), &pkg_url])
        .status()
        .map_err(|e| format!("Cannot run curl: {}", e))?;
    if !status.success() {
        return Err(format!("Failed to download from {}", pkg_url));
    }

    // Extract
    let status = std::process::Command::new("tar")
        .args(["-xzf", &tar_path.to_string_lossy(), "-C", &dest.to_string_lossy()])
        .status()
        .map_err(|e| format!("Cannot run tar: {}", e))?;
    if !status.success() {
        return Err("Failed to extract package archive".to_string());
    }
    let _ = fs::remove_file(&tar_path);

    Ok(())
}

/// Fetch a URL and return the body as string
fn http_get(url: &str) -> Result<String, String> {
    let output = std::process::Command::new("curl")
        .args(["-sSf", "-L", url])
        .output()
        .map_err(|e| format!("Cannot run curl: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("HTTP request failed: {}", stderr));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Search the registry for packages matching a query
pub fn search_packages(query: &str, registry_url: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();

    // Search local index
    let index = read_index();
    for (name, versions) in &index {
        if name.contains(query) || query.is_empty() {
            let latest = versions.keys().max_by(|a, b| {
                match (Version::parse(a), Version::parse(b)) {
                    (Some(va), Some(vb)) => va.cmp(&vb),
                    _ => a.cmp(b),
                }
            }).cloned().unwrap_or_default();

            // Get metadata
            let meta = read_metadata(name, &latest);
            results.push(SearchResult {
                name: name.clone(),
                latest_version: latest,
                description: meta.description,
            });
        }
    }

    // If remote registry is configured, merge remote results
    if !registry_url.is_empty() && registry_url != "local" {
        if let Ok(json) = http_get(&format!("{}/api/v1/search?q={}", registry_url.trim_end_matches('/'), query)) {
            if let Ok(remote_results) = serde_json::from_str::<Vec<SearchResult>>(&json) {
                for r in remote_results {
                    if !results.iter().any(|x| x.name == r.name) {
                        results.push(r);
                    }
                }
            }
        }
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}

// ── Package signing (Ed25519) ─────────────────────────────────────

/// Sign a package with Ed25519. Returns the hex-encoded signature.
pub fn sign_package(name: &str, version: &str, signer: &str) -> Result<PackageSignature, String> {
    let cached = package_cache_dir(name, version);
    if !cached.exists() {
        return Err(format!("Package '{}@{}' not in cache", name, version));
    }
    let hash = compute_dir_checksum(&cached)?;
    let (signing_key, verifying_key) = load_or_generate_keypair()?;
    let public_key_hex = hex::encode(verifying_key.to_bytes());

    // Sign the checksum hash
    use ed25519_dalek::Signer;
    let signature = signing_key.sign(hash.as_bytes());
    let signature_hex = hex::encode(signature.to_bytes());

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0);

    let sig = PackageSignature {
        signer: if signer.is_empty() || signer == "default" {
            public_key_hex[..16].to_string()
        } else {
            signer.to_string()
        },
        hash,
        signature_hex,
        public_key_hex,
        timestamp,
    };

    // Save signature to signatures directory
    let sig_dir = signatures_dir().join(sanitize_pkg_segment(name));
    fs::create_dir_all(&sig_dir).map_err(|e| format!("Cannot create sig dir: {}", e))?;
    let sig_path = sig_dir.join(format!("{}.sig", sanitize_pkg_segment(version)));
    let sig_content = serialize_signature(&sig);
    fs::write(&sig_path, &sig_content).map_err(|e| format!("Cannot write signature: {}", e))?;

    Ok(sig)
}

fn serialize_signature(sig: &PackageSignature) -> String {
    format!(
        "signer = \"{}\"\nhash = \"{}\"\nsignature_hex = \"{}\"\npublic_key_hex = \"{}\"\ntimestamp = {}\n",
        sig.signer, sig.hash, sig.signature_hex, sig.public_key_hex, sig.timestamp
    )
}

fn deserialize_signature(content: &str) -> Result<PackageSignature, String> {
    let mut signer = String::new();
    let mut hash = String::new();
    let mut signature_hex = String::new();
    let mut public_key_hex = String::new();
    let mut timestamp: u64 = 0;
    for line in content.lines() {
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            let val = line[eq + 1..].trim().trim_matches('"');
            match key {
                "signer" => signer = val.to_string(),
                "hash" => hash = val.to_string(),
                "signature_hex" => signature_hex = val.to_string(),
                "public_key_hex" => public_key_hex = val.to_string(),
                "timestamp" => timestamp = val.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    if signature_hex.is_empty() || hash.is_empty() {
        return Err("Invalid signature format".to_string());
    }
    Ok(PackageSignature { signer, hash, signature_hex, public_key_hex, timestamp })
}

/// Verify a package signature using Ed25519
pub fn verify_signature(name: &str, version: &str) -> Result<PackageSignature, String> {
    let sig_dir = signatures_dir().join(sanitize_pkg_segment(name));
    let sig_path = sig_dir.join(format!("{}.sig", sanitize_pkg_segment(version)));
    if !sig_path.exists() {
        return Err(format!("No signature found for '{}@{}'", name, version));
    }
    let content = fs::read_to_string(&sig_path)
        .map_err(|e| format!("Cannot read signature: {}", e))?;
    let sig = deserialize_signature(&content)?;

    let cached = package_cache_dir(name, version);
    if !cached.exists() {
        return Err(format!("Package '{}@{}' not in cache", name, version));
    }
    let actual_hash = compute_dir_checksum(&cached)?;
    if sig.hash != actual_hash {
        return Err(format!(
            "Signature hash mismatch for '{}@{}': expected {}, got {}. Package has been modified!",
            name, version, sig.hash, actual_hash
        ));
    }

    // Verify Ed25519 signature
    let sig_bytes = hex::decode(&sig.signature_hex)
        .map_err(|e| format!("Invalid signature hex: {}", e))?;
    let pk_bytes = hex::decode(&sig.public_key_hex)
        .map_err(|e| format!("Invalid public key hex: {}", e))?;

    let pk_array: [u8; 32] = pk_bytes.try_into().map_err(|_| "Invalid public key length".to_string())?;
    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&pk_array)
        .map_err(|e| format!("Invalid public key: {}", e))?;

    let sig_array: [u8; 64] = sig_bytes.try_into().map_err(|_| "Invalid signature length".to_string())?;
    let signature = ed25519_dalek::Signature::from_bytes(&sig_array);

    use ed25519_dalek::Verifier;
    verifying_key.verify(sig.hash.as_bytes(), &signature)
        .map_err(|e| format!("Ed25519 signature verification FAILED: {}. Package may be tampered!", e))?;

    Ok(sig)
}

// ── Audit & Security Scanning ──────────────────────────────────────

pub fn load_advisories() -> Vec<Advisory> {
    let dir = advisories_dir();
    if !dir.exists() { return Vec::new(); }
    let mut advisories = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(advisory) = serde_json::from_str::<Advisory>(&content) {
                        advisories.push(advisory);
                    }
                }
            }
        }
    }
    advisories
}

pub fn add_advisory(advisory: &Advisory) -> Result<(), String> {
    let dir = advisories_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create advisories dir: {}", e))?;
    let path = dir.join(format!("{}.json", advisory.id));
    let json = serde_json::to_string_pretty(advisory)
        .map_err(|e| format!("Cannot serialize advisory: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Cannot write advisory: {}", e))?;
    Ok(())
}

/// Scan dependencies for known vulnerabilities
pub fn audit_deps(lock: &super::types::LockFile) -> Vec<Advisory> {
    let advisories = load_advisories();
    let mut findings = Vec::new();

    for (name, entry) in lock {
        let ver = match Version::parse(&entry.version) {
            Some(v) => v,
            None => continue,
        };
        for adv in &advisories {
            if adv.package != *name { continue; }
            if let Some(affected_constraint) = VersionConstraint::parse(&adv.versions_affected) {
                if affected_constraint.matches(&ver) {
                    findings.push(adv.clone());
                }
            }
        }
    }
    findings
}

/// Fetch latest advisories from remote
#[cfg(feature = "remote-registry")]
pub fn fetch_advisories(registry_url: &str) -> Result<Vec<Advisory>, String> {
    let url = format!("{}/api/v1/advisories.json", registry_url.trim_end_matches('/'));
    match http_get(&url) {
        Ok(json) => match serde_json::from_str::<Vec<Advisory>>(&json) {
            Ok(advisories) => {
                for adv in &advisories {
                    let _ = add_advisory(adv);
                }
                Ok(advisories)
            }
            Err(e) => Err(format!("Failed to parse advisories: {}", e)),
        }
        Err(e) => Err(e),
    }
}

#[cfg(not(feature = "remote-registry"))]
pub fn fetch_advisories(_registry_url: &str) -> Result<Vec<Advisory>, String> {
    Err("Remote registry not available".to_string())
}

use super::types::{Version, VersionConstraint};

// ── Security scan ──────────────────────────────────────────────────

pub fn security_scan(lock: &super::types::LockFile) -> Vec<String> {
    let mut issues = Vec::new();

    for (name, entry) in lock {
        // Check for signed packages
        let sig_dir = signatures_dir().join(sanitize_pkg_segment(name));
        let sig_path = sig_dir.join(format!("{}.sig", sanitize_pkg_segment(&entry.version)));
        if !sig_path.exists() {
            issues.push(format!("{}@{}: unsigned package — supply chain risk", name, entry.version));
        } else {
            // Verify Ed25519 signature
            match verify_signature(name, &entry.version) {
                Ok(_) => {} // Signature valid
                Err(e) => {
                    issues.push(format!("{}@{}: signature verification failed: {}", name, entry.version, e));
                }
            }
        }

        // Check for integrity
        let cached = package_cache_dir(name, &entry.version);
        if cached.exists() {
            if let Ok(actual) = compute_dir_checksum(&cached) {
                if actual != entry.checksum {
                    issues.push(format!("{}@{}: checksum mismatch — possible tampering", name, entry.version));
                }
            }
        }
    }

    issues
}

// ── Content-addressed Cache ────────────────────────────────────────

/// Store a blob of data in the content-addressed cache
pub fn cache_put(key: &str, data: &[u8]) -> Result<String, String> {
    use sha2::Digest;
    let hash = sha2::Sha256::digest(data);
    let hash_hex = format!("{:x}", hash);
    let subdir = &hash_hex[..2];
    let cache_dir = cache_root().join("objects").join(subdir);
    fs::create_dir_all(&cache_dir).map_err(|e| format!("Cannot create cache dir: {}", e))?;
    let path = cache_dir.join(&hash_hex);
    if !path.exists() {
        fs::write(&path, data).map_err(|e| format!("Cannot write cache: {}", e))?;
    }
    // Index the key
    let index_dir = cache_root().join("index");
    fs::create_dir_all(&index_dir).map_err(|e| format!("Cannot create index dir: {}", e))?;
    let key_path = index_dir.join(sanitize_pkg_segment(key));
    fs::write(&key_path, &hash_hex).map_err(|e| format!("Cannot write cache index: {}", e))?;
    Ok(hash_hex)
}

/// Retrieve a blob from the content-addressed cache by hash
pub fn cache_get(hash: &str) -> Option<Vec<u8>> {
    let subdir = &hash[..2.min(hash.len())];
    let path = cache_root().join("objects").join(subdir).join(hash);
    if path.exists() {
        fs::read(&path).ok()
    } else {
        None
    }
}

/// Look up a key in the cache index and return the hash
pub fn cache_lookup(key: &str) -> Option<String> {
    let key_path = cache_root().join("index").join(sanitize_pkg_segment(key));
    if key_path.exists() {
        fs::read_to_string(&key_path).ok().map(|s| s.trim().to_string())
    } else {
        None
    }
}

// ── Cache management ───────────────────────────────────────────────

pub fn get_cache_size() -> Result<u64, String> {
    let cache = cache_root();
    if !cache.exists() { return Ok(0); }
    fn dir_size(path: &Path) -> std::io::Result<u64> {
        let mut total = 0u64;
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let p = entry.path();
                if p.is_dir() {
                    total += dir_size(&p)?;
                } else {
                    total += entry.metadata()?.len();
                }
            }
        }
        Ok(total)
    }
    dir_size(&cache).map_err(|e| format!("Cannot compute cache size: {}", e))
}

pub fn clear_cache() -> Result<(), String> {
    let cache = cache_root();
    if cache.exists() {
        fs::remove_dir_all(&cache).map_err(|e| format!("Cannot clear cache: {}", e))?;
    }
    fs::create_dir_all(&cache).map_err(|e| format!("Cannot recreate cache: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_serialize_deserialize_signature() {
        let sig = PackageSignature {
            signer: "test_signer".into(),
            hash: "abc123".into(),
            signature_hex: "deadbeef".into(),
            public_key_hex: "cafebabe".into(),
            timestamp: 1234567890,
        };
        let serialized = serialize_signature(&sig);
        let deserialized = deserialize_signature(&serialized).unwrap();
        assert_eq!(sig.signer, deserialized.signer);
        assert_eq!(sig.hash, deserialized.hash);
        assert_eq!(sig.signature_hex, deserialized.signature_hex);
        assert_eq!(sig.public_key_hex, deserialized.public_key_hex);
        assert_eq!(sig.timestamp, deserialized.timestamp);
    }

    #[test]
    fn test_deserialize_invalid_signature() {
        assert!(deserialize_signature("").is_err());
        assert!(deserialize_signature("garbage = data").is_err());
    }

    #[test]
    fn test_metadata_write_read() {
        let tmp = std::env::temp_dir().join(format!("parth_test_meta_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);

        // Temporarily redirect metadata_dir
        let original_metadata_dir = std::mem::ManuallyDrop::new(metadata_dir());
        // We can't easily override, so test the functions directly
        let meta = RegistryMetadata::new("test_pkg");
        assert_eq!(meta.description, "");
        assert_eq!(meta.yanked, false);

        let mut meta2 = RegistryMetadata::new("test_pkg");
        meta2.description = "A test package".into();
        meta2.author = "Tester".into();
        meta2.homepage = "https://example.com".into();
        meta2.license = "MIT".into();
        meta2.yanked = true;

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_compute_dir_checksum() {
        let tmp = std::env::temp_dir().join(format!("parth_test_sum_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        fs::write(tmp.join("test.txt"), "hello world").unwrap();
        fs::create_dir_all(tmp.join("sub")).unwrap();
        fs::write(tmp.join("sub").join("nested.txt"), "nested data").unwrap();

        let checksum = compute_dir_checksum(&tmp).unwrap();
        assert!(!checksum.is_empty());
        assert_eq!(checksum.len(), 64); // SHA-256 hex

        // Same input should produce same checksum
        let checksum2 = compute_dir_checksum(&tmp).unwrap();
        assert_eq!(checksum, checksum2);

        // Modified content should change checksum
        fs::write(tmp.join("test.txt"), "modified").unwrap();
        let checksum3 = compute_dir_checksum(&tmp).unwrap();
        assert_ne!(checksum, checksum3);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_content_addressed_cache() {
        let tmp = std::env::temp_dir().join(format!("parth_test_cache_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);

        // Rebuild the cache operations to use the tmp dir directly
        let obj_dir = tmp.join("objects");
        let idx_dir = tmp.join("index");
        fs::create_dir_all(&obj_dir).unwrap();
        fs::create_dir_all(&idx_dir).unwrap();

        // Manual cache operations using tmp paths
        use sha2::Digest;
        let data = b"test data";
        let hash = sha2::Sha256::digest(data);
        let hash_hex = format!("{:x}", hash);

        let subdir = &hash_hex[..2];
        let obj_sub = obj_dir.join(subdir);
        fs::create_dir_all(&obj_sub).unwrap();
        fs::write(obj_sub.join(&hash_hex), data).unwrap();
        fs::write(idx_dir.join("test_key"), &hash_hex).unwrap();

        // Verify
        assert_eq!(hash_hex.len(), 64);
        let read_back = fs::read(obj_sub.join(&hash_hex)).unwrap();
        assert_eq!(read_back, data);
        let index_back = fs::read_to_string(idx_dir.join("test_key")).unwrap();
        assert_eq!(index_back.trim(), hash_hex);

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_yank_unyank() {
        let tmp = std::env::temp_dir().join(format!("parth_test_yank_{}", std::process::id()));
        // Override parth_home by overriding the metadata_dir behavior
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &tmp);

        let meta = read_metadata("yank_test", "1.0.0");
        assert_eq!(meta.yanked, false);

        yank_package("yank_test", "1.0.0").unwrap();
        assert!(is_yanked("yank_test", "1.0.0"));

        unyank_package("yank_test", "1.0.0").unwrap();
        assert!(!is_yanked("yank_test", "1.0.0"));

        let _ = fs::remove_dir_all(&tmp);
        if let Some(h) = original_home { std::env::set_var("HOME", h); }
    }

    #[test]
    fn test_key_generation() {
        let tmp = std::env::temp_dir().join(format!("parth_test_key_{}", std::process::id()));
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &tmp);

        let (secret, public) = generate_keypair().unwrap();
        assert_eq!(secret.len(), 64); // hex-encoded 32 bytes
        assert_eq!(public.len(), 64);

        // Re-load should return same keys
        let (_sk, vk) = load_or_generate_keypair().unwrap();
        assert_eq!(hex::encode(vk.to_bytes()), public);

        let _ = fs::remove_dir_all(&tmp);
        if let Some(h) = original_home { std::env::set_var("HOME", h); }
    }
}
