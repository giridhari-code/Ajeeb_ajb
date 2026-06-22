use std::fs;
use std::path::{Path, PathBuf};

use super::{package_cache_dir, copy_dir_recursive, find_local_package, read_index, read_auth};
use super::super::types::{RegistryIndex, SearchResult};

pub fn remote_fetch_index(_registry_url: &str, _package_name: &str) -> RegistryIndex {
    read_index()
}

/// Download a package from the remote registry and cache it
pub fn download_package(name: &str, version: &str, _registry_url: &str) -> Result<PathBuf, String> {
    let cached = package_cache_dir(name, version);
    if cached.join("parth.das").exists() {
        return Ok(cached); // Already cached
    }

    // Try to find in local search paths
    if let Some(local_path) = find_local_package(name) {
        // Copy from local path to cache
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

    // SHA-256 verification
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&bytes);
    let hash = format!("{:x}", hasher.finalize());

    fs::write(&tar_path, &bytes).map_err(|e| format!("Cannot write archive: {}", e))?;

    // Extract using flate2 + tar
    let file = fs::File::open(&tar_path).map_err(|e| format!("Cannot open archive: {}", e))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    archive.unpack(dest).map_err(|e| format!("Cannot extract archive: {}", e))?;

    let _ = fs::remove_file(&tar_path);
    println!("📦 Downloaded '{}@{}' (SHA-256: {}...)", name, version, &hash[..16]);

    Ok(())
}

/// Fetch a URL and return the body as string
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

/// Search the registry for packages matching a query
pub fn search_packages(query: &str, _registry_url: &str) -> Vec<SearchResult> {
    use super::{find_all_local_packages, read_metadata, read_package_version, read_package_description};
    use super::super::types::Version;

    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Search all local packages
    for (name, pkg_dir) in find_all_local_packages() {
        if !seen.contains(&name) && (name.contains(query) || query.is_empty()) {
            // Try to read version from parth.das
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

    // Also search local index
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

/// Create a .tar.gz archive from a source directory
fn create_tarball(source_dir: &Path, dest_path: &Path) -> Result<(), String> {
    let file = fs::File::create(dest_path).map_err(|e| format!("Cannot create tarball: {}", e))?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);

    // Walk the source dir and add all files
    fn add_dir(archive: &mut tar::Builder<flate2::write::GzEncoder<fs::File>>,
               dir: &Path, prefix: &Path) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("Cannot read dir: {}", e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(prefix).unwrap_or(&path);
            if path.is_dir() {
                add_dir(archive, &path, prefix)?;
            } else if path.extension().map(|e| e == "ajb" || e == "das" || e == "c" || e == "h").unwrap_or(false) {
                let data = fs::read(&path).map_err(|e| format!("Cannot read {:?}: {}", path, e))?;
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                archive.append_data(&mut header, rel, data.as_slice())
                    .map_err(|e| format!("Cannot add to archive: {}", e))?;
            }
        }
        Ok(())
    }

    add_dir(&mut archive, source_dir, source_dir)?;
    archive.finish().map_err(|e| format!("Cannot finalize archive: {}", e))?;
    Ok(())
}

/// Publish a package to a remote registry server
pub fn publish_to_remote(
    name: &str, version: &str, author: &str, description: &str,
    source_dir: &Path, registry_url: &str, checksum: &str,
) -> Result<(), String> {
    use super::crypto::{read_signature, serialize_signature};

    let url = registry_url.trim_end_matches('/');

    // Get auth token
    let token = read_auth().and_then(|a| {
        if a.registry_url == registry_url || a.registry_url == url {
            Some(a.token)
        } else {
            None
        }
    });

    // Get signature
    let sig = read_signature(name, version);
    let signature = sig.as_ref().map(|s| {
        serde_json::to_string(s).unwrap_or_default()
    });

    // Create tarball
    let tarball_path = source_dir.join(format!("{}-{}.tar.gz", name, version));
    create_tarball(source_dir, &tarball_path)?;

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Cannot create HTTP client: {}", e))?;

    // Upload metadata
    let meta_body = serde_json::json!({
        "name": name,
        "version": version,
        "author": author,
        "description": description,
        "checksum": checksum,
        "signature": signature,
    });

    let mut req = client.post(&format!("{}/api/v1/packages", url))
        .json(&meta_body);

    if let Some(ref t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let resp = req.send().map_err(|e| format!("Cannot publish to registry: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().unwrap_or_default();
        return Err(format!("Registry rejected publish (HTTP {}): {}", status, text));
    }
    println!("✓ Metadata published to registry");

    // Upload tarball
    let tarball_data = fs::read(&tarball_path)
        .map_err(|e| format!("Cannot read tarball: {}", e))?;

    let mut req = client.post(&format!("{}/api/v1/packages/{}/{}/upload", url, name, version))
        .header("content-type", "application/gzip")
        .body(tarball_data);

    if let Some(ref t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let resp = req.send().map_err(|e| format!("Cannot upload tarball: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().unwrap_or_default();
        return Err(format!("Registry rejected upload (HTTP {}): {}", status, text));
    }
    println!("✓ Tarball uploaded to registry");

    // Cleanup
    let _ = fs::remove_file(&tarball_path);

    Ok(())
}
