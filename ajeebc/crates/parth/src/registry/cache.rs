use std::fs;
use std::path::Path;

use super::{cache_root, sanitize_pkg_segment};

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
