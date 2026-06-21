use std::fs;
use std::path::{Path, PathBuf};

use super::paths::{package_cache_dir, find_local_package};
use super::cache::copy_dir_recursive;
use super::metadata::{read_index, read_metadata};
use super::super::types::{RegistryIndex, SearchResult, Version};

pub fn remote_fetch_index(_registry_url: &str, _package_name: &str) -> RegistryIndex {
    read_index()
}

pub fn download_package(name: &str, version: &str, _registry_url: &str) -> Result<PathBuf, String> {
    let cached = package_cache_dir(name, version);
    if cached.join("parth.das").exists() {
        return Ok(cached);
    }

    if let Some(local_path) = find_local_package(name) {
        fs::create_dir_all(&cached).map_err(|e| format!("Cannot create cache dir: {}", e))?;
        if let Some(das_file) = local_path.join("parth.das").as_path().to_str() {
            let _ = fs::copy(das_file, cached.join("parth.das"));
        }
        if local_path.join("src").exists() {
            copy_dir_recursive(&local_path.join("src"), &cached.join("src"))?;
        }
        return Ok(cached);
    }

    Err(format!(
        "Package '{}' not found locally. Searched in:\n  1) ./packages/\n  2) ~/.parth/packages/\n  3) ../packages/\n  4) <ajeeb_root>/packages/",
        name
    ))
}

fn download_from_remote(name: &str, version: &str, url: &str, dest: &Path) -> Result<(), String> {
    let pkg_url = format!("{}/api/v1/packages/{}/{}.tar.gz", url.trim_end_matches('/'), name, version);
    fs::create_dir_all(dest).map_err(|e| format!("Cannot create cache dir: {}", e))?;

    let tar_path = dest.join("package.tar.gz");

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| format!("Cannot create HTTP client: {}", e))?;

    let response = client.get(&pkg_url)
        .send()
        .map_err(|e| format!("Cannot download from {}: {}", pkg_url, e))?;

    if !response.status().is_success() {
        return Err(format!("Failed to download from {} (HTTP {})", pkg_url, response.status()));
    }

    let bytes = response.bytes().map_err(|e| format!("Cannot read response: {}", e))?;

    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&bytes);
    let hash = format!("{:x}", hasher.finalize());

    fs::write(&tar_path, &bytes).map_err(|e| format!("Cannot write archive: {}", e))?;

    let file = fs::File::open(&tar_path).map_err(|e| format!("Cannot open archive: {}", e))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(dest).map_err(|e| format!("Cannot extract archive: {}", e))?;

    let _ = fs::remove_file(&tar_path);
    println!("📦 Downloaded '{}@{}' (SHA-256: {}...)", name, version, &hash[..16]);

    Ok(())
}

pub fn http_get(url: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Cannot create HTTP client: {}", e))?;

    let response = client.get(url)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP request failed: HTTP {}", response.status()));
    }

    response.text().map_err(|e| format!("Cannot read response: {}", e))
}

pub fn search_packages(query: &str, _registry_url: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for (name, pkg_dir) in super::paths::find_all_local_packages() {
        if !seen.contains(&name) && (name.contains(query) || query.is_empty()) {
            let version = read_package_version(&pkg_dir).unwrap_or_else(|| "0.1.0".to_string());
            let description = read_package_description(&pkg_dir).unwrap_or_default();
            results.push(SearchResult {
                name: name.clone(),
                latest_version: version,
                description,
            });
            seen.insert(name);
        }
    }

    let index = read_index();
    for (name, versions) in &index {
        if !seen.contains(name) && (name.contains(query) || query.is_empty()) {
            let latest = versions.keys().max_by(|a, b| {
                match (Version::parse(a), Version::parse(b)) {
                    (Some(va), Some(vb)) => va.cmp(&vb),
                    _ => a.cmp(b),
                }
            }).cloned().unwrap_or_default();

            let meta = read_metadata(name, &latest);
            results.push(SearchResult {
                name: name.clone(),
                latest_version: latest,
                description: meta.description,
            });
            seen.insert(name.clone());
        }
    }

    results.sort_by(|a, b| a.name.cmp(&b.name));
    results
}

fn read_package_version(pkg_dir: &Path) -> Option<String> {
    let das_path = pkg_dir.join("parth.das");
    if !das_path.exists() { return None; }
    let content = fs::read_to_string(&das_path).ok()?;
    let mut in_package = false;
    for line in content.lines() {
        let t = line.trim();
        if t == "[package]" { in_package = true; continue; }
        if t.starts_with('[') && t.ends_with(']') { in_package = false; continue; }
        if in_package {
            if let Some(eq) = t.find('=') {
                let key = t[..eq].trim();
                let val = t[eq + 1..].trim().trim_matches('"');
                if key == "version" { return Some(val.to_string()); }
            }
        }
    }
    None
}

fn read_package_description(pkg_dir: &Path) -> Option<String> {
    let das_path = pkg_dir.join("parth.das");
    if !das_path.exists() { return None; }
    let content = fs::read_to_string(&das_path).ok()?;
    let mut in_package = false;
    for line in content.lines() {
        let t = line.trim();
        if t == "[package]" { in_package = true; continue; }
        if t.starts_with('[') && t.ends_with(']') { in_package = false; continue; }
        if in_package {
            if let Some(eq) = t.find('=') {
                let key = t[..eq].trim();
                let val = t[eq + 1..].trim().trim_matches('"');
                if key == "description" { return Some(val.to_string()); }
            }
        }
    }
    None
}

pub fn link_local_package(source_path: &Path, name: &str) -> Result<PathBuf, String> {
    if !source_path.exists() {
        return Err(format!("Source path '{}' does not exist", source_path.display()));
    }

    let version = read_package_version(source_path)
        .unwrap_or_else(|| "0.1.0".to_string());

    let cached = package_cache_dir(name, &version);
    fs::create_dir_all(&cached).map_err(|e| format!("Cannot create cache dir: {}", e))?;

    let das_file = source_path.join("parth.das");
    if das_file.exists() {
        fs::copy(&das_file, cached.join("parth.das"))
            .map_err(|e| format!("Cannot copy parth.das: {}", e))?;
    }

    if source_path.join("src").exists() {
        copy_dir_recursive(&source_path.join("src"), &cached.join("src"))?;
    }

    if source_path.join("runtime").exists() {
        copy_dir_recursive(&source_path.join("runtime"), &cached.join("runtime"))?;
    }

    println!("🔗 Linked '{}' from {} to cache", name, source_path.display());
    Ok(cached)
}
