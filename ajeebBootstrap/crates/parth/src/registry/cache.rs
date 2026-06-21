use std::fs;
use std::path::{Path, PathBuf};

use sha2::Digest;

use super::paths::{package_cache_dir, sanitize_pkg_segment};
use super::super::types::{LockEntry, PkgDep};

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

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
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
    match super::super::config::read_package_meta(&das_path) {
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

pub fn cache_put(key: &str, data: &[u8]) -> Result<String, String> {
    let hash = sha2::Sha256::digest(data);
    let hash_hex = format!("{:x}", hash);
    let subdir = &hash_hex[..2];
    let cache_dir = super::paths::cache_root().join("objects").join(subdir);
    fs::create_dir_all(&cache_dir).map_err(|e| format!("Cannot create cache dir: {}", e))?;
    let path = cache_dir.join(&hash_hex);
    if !path.exists() {
        fs::write(&path, data).map_err(|e| format!("Cannot write cache: {}", e))?;
    }
    let index_dir = super::paths::cache_root().join("index");
    fs::create_dir_all(&index_dir).map_err(|e| format!("Cannot create index dir: {}", e))?;
    let key_path = index_dir.join(sanitize_pkg_segment(key));
    fs::write(&key_path, &hash_hex).map_err(|e| format!("Cannot write cache index: {}", e))?;
    Ok(hash_hex)
}

pub fn cache_get(hash: &str) -> Option<Vec<u8>> {
    let subdir = &hash[..2.min(hash.len())];
    let path = super::paths::cache_root().join("objects").join(subdir).join(hash);
    if path.exists() {
        fs::read(&path).ok()
    } else {
        None
    }
}

pub fn cache_lookup(key: &str) -> Option<String> {
    let key_path = super::paths::cache_root().join("index").join(sanitize_pkg_segment(key));
    if key_path.exists() {
        fs::read_to_string(&key_path).ok().map(|s| s.trim().to_string())
    } else {
        None
    }
}

pub fn get_cache_size() -> Result<u64, String> {
    let cache = super::paths::cache_root();
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
    let cache = super::paths::cache_root();
    if cache.exists() {
        fs::remove_dir_all(&cache).map_err(|e| format!("Cannot clear cache: {}", e))?;
    }
    fs::create_dir_all(&cache).map_err(|e| format!("Cannot recreate cache: {}", e))?;
    Ok(())
}
