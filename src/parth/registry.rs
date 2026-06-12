use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::config::read_package_meta;
use super::types::{Advisory, LockEntry, PackageSignature, PkgDep, RegistryIndex, SearchResult};

// ── Paths ──────────────────────────────────────────────────────────

pub fn parth_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".parth")
}

pub fn index_path() -> PathBuf { parth_home().join("index") }
pub fn cache_root() -> PathBuf { parth_home().join("cache") }
pub fn signatures_dir() -> PathBuf { parth_home().join("signatures") }
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
    write_index(&index)
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
    let nonce: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64).unwrap_or(0);
    let tmp = std::env::temp_dir().join(format!("parth_sum_{:016x}", nonce));
    fs::write(&tmp, &input).map_err(|e| format!("Cannot write temp: {}", e))?;
    let out = std::process::Command::new("sha256sum").arg(&tmp).output()
        .map_err(|e| format!("Cannot run sha256sum: {}", e))?;
    let _ = fs::remove_file(&tmp);
    let hash = String::from_utf8_lossy(&out.stdout);
    let hash = hash.split_whitespace().next().unwrap_or("").to_string();
    if hash.is_empty() { return Err("sha256sum returned empty".to_string()); }
    Ok(hash)
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
    // In production this fetches from the registry API
    // For now, try local cache first
    read_index()
}

#[cfg(not(feature = "remote-registry"))]
pub fn remote_fetch_index(_registry_url: &str, _package_name: &str) -> RegistryIndex {
    read_index()
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
            results.push(SearchResult {
                name: name.clone(),
                latest_version: latest,
                description: String::new(),
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

// ── Package signing ────────────────────────────────────────────────

pub fn sign_package(name: &str, version: &str, signer: &str) -> Result<PackageSignature, String> {
    let cached = package_cache_dir(name, version);
    if !cached.exists() {
        return Err(format!("Package '{}@{}' not in cache", name, version));
    }
    let hash = compute_dir_checksum(&cached)?;
    let sig_dir = signatures_dir().join(sanitize_pkg_segment(name));
    fs::create_dir_all(&sig_dir).map_err(|e| format!("Cannot create sig dir: {}", e))?;

    let sig_path = sig_dir.join(format!("{}.sig", sanitize_pkg_segment(version)));
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0);

    // Create a signature file (simplified — in production use GPG/minisign)
    let sig_content = format!("{}:{}:{}:{}\n", signer, hash, timestamp, name);
    fs::write(&sig_path, &sig_content)
        .map_err(|e| format!("Cannot write signature: {}", e))?;

    Ok(PackageSignature {
        signer: signer.to_string(),
        hash,
        signature: sig_content,
        timestamp,
    })
}

pub fn verify_signature(name: &str, version: &str) -> Result<PackageSignature, String> {
    let sig_dir = signatures_dir().join(sanitize_pkg_segment(name));
    let sig_path = sig_dir.join(format!("{}.sig", sanitize_pkg_segment(version)));
    if !sig_path.exists() {
        return Err(format!("No signature found for '{}@{}'", name, version));
    }
    let content = fs::read_to_string(&sig_path)
        .map_err(|e| format!("Cannot read signature: {}", e))?;
    let parts: Vec<&str> = content.trim().split(':').collect();
    if parts.len() < 3 {
        return Err("Invalid signature format".to_string());
    }
    let cached = package_cache_dir(name, version);
    if !cached.exists() {
        return Err(format!("Package '{}@{}' not in cache", name, version));
    }
    let actual_hash = compute_dir_checksum(&cached)?;
    if parts[1] != actual_hash {
        return Err(format!(
            "Signature hash mismatch for '{}@{}': expected {}, got {}",
            name, version, parts[1], actual_hash
        ));
    }
    Ok(PackageSignature {
        signer: parts[0].to_string(),
        hash: parts[1].to_string(),
        signature: content.clone(),
        timestamp: parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
    })
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
        // Check for unverified packages
        let sig_dir = signatures_dir().join(sanitize_pkg_segment(name));
        let sig_path = sig_dir.join(format!("{}.sig", sanitize_pkg_segment(&entry.version)));
        if !sig_path.exists() {
            issues.push(format!("{}@{}: unsigned package — supply chain risk", name, entry.version));
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
