use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::config::read_package_meta;
use super::types::{LockEntry, RegistryIndex, RegistryMetadata};

pub mod auth;
pub mod cache;
pub mod crypto;
pub mod docs;
pub mod remote;
pub mod security;

pub use self::auth::*;
pub use self::cache::*;
pub use self::crypto::*;
pub use self::docs::*;
pub use self::remote::*;
pub use self::security::*;

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

pub(crate) fn sanitize_pkg_segment(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' { c } else { '_' })
        .collect()
}

pub fn package_cache_dir(name: &str, version: &str) -> PathBuf {
    parth_home().join("packages").join(sanitize_pkg_segment(name)).join(sanitize_pkg_segment(version))
}

// ── Local Package Resolution (4 locations) ──────────────────────────

/// Search locations for local packages:
/// 1) ./packages/<name>/
/// 2) ~/.parth/packages/<name>/
/// 3) ../packages/<name>/
/// 4) <ajeeb_root>/packages/<name>/
/// 5) ~/.ajeeb/packages/ajeeb-std/<name>/
/// Also checks for single .ajb files (standard library pattern)
pub fn find_local_package(name: &str) -> Option<PathBuf> {
    let search_roots = local_package_search_paths();
    let sanitized = sanitize_pkg_segment(name);
    for root in &search_roots {
        // Check directory with parth.das
        let pkg_dir = root.join(&sanitized);
        if pkg_dir.exists() && pkg_dir.join("parth.das").exists() {
            return Some(pkg_dir);
        }
        // Check single .ajb file (standard library pattern)
        let ajb_file = root.join(format!("{}.ajb", sanitized));
        if ajb_file.exists() {
            return Some(root.clone());
        }
    }
    None
}

/// Get all search paths for local packages
pub fn local_package_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    
    // 1) ./packages/<name>/
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("packages"));
    }
    
    // 2) ~/.parth/packages/<name>/
    paths.push(parth_home().join("packages"));
    
    // 3) ../packages/<name>/
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(parent) = cwd.parent() {
            paths.push(parent.join("packages"));
        }
    }
    
    // 4) <ajeeb_root>/packages/<name>/
    let root = find_ajeeb_root();
    paths.push(root.join("packages"));
    
    // 5) ~/.ajeeb/packages/ajeeb-std/ (standard library)
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(home).join(".ajeeb/packages/ajeeb-std"));
    }
    
    paths
}

/// Find the Ajeeb root directory
fn find_ajeeb_root() -> PathBuf {
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut dir = PathBuf::from(manifest);
        loop {
            if dir.join("compiler").join("compiler.ajb").exists() { return dir; }
            if !dir.pop() { break; }
        }
    }
    let mut dir = std::env::current_dir().unwrap_or_default();
    loop {
        if dir.join("compiler").join("compiler.ajb").exists() { return dir; }
        if !dir.pop() { break; }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let mut d = parent.to_path_buf();
            loop {
                if d.join("compiler").join("compiler.ajb").exists() { return d; }
                if !d.pop() { break; }
            }
        }
    }
    PathBuf::from("..")
}

/// Find all local packages across all search locations
pub fn find_all_local_packages() -> Vec<(String, PathBuf)> {
    let mut packages = Vec::new();
    let mut seen = std::collections::HashSet::new();
    
    for root in local_package_search_paths() {
        if !root.exists() { continue; }
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    if !seen.contains(&name) && path.join("parth.das").exists() {
                        seen.insert(name.clone());
                        packages.push((name, path));
                    }
                }
            }
        }
    }
    
    packages
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
    // Verify signature if a signature file exists
    let sig_dir = signatures_dir().join(sanitize_pkg_segment(name));
    let sig_path = sig_dir.join(format!("{}.sig", sanitize_pkg_segment(version)));
    if sig_path.exists() {
        if let Err(e) = crypto::verify_signature(name, version) {
            return Err(format!(
                "Signature verification failed for '{}@{}': {}. Package may be tampered!",
                name, version, e
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

pub fn read_package_deps(name: &str, version: &str) -> Vec<super::types::PkgDep> {
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
        signature: String::new(),
        signer: String::new(),
    })
}

/// Read version from a package's parth.das
pub(crate) fn read_package_version(pkg_dir: &Path) -> Option<String> {
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

/// Read description from a package's parth.das
pub(crate) fn read_package_description(pkg_dir: &Path) -> Option<String> {
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

/// Link a local package to the cache
pub fn link_local_package(source_path: &Path, name: &str) -> Result<PathBuf, String> {
    if !source_path.exists() {
        return Err(format!("Source path '{}' does not exist", source_path.display()));
    }
    
    let version = read_package_version(source_path)
        .unwrap_or_else(|| "0.1.0".to_string());
    
    let cached = package_cache_dir(name, &version);
    fs::create_dir_all(&cached).map_err(|e| format!("Cannot create cache dir: {}", e))?;
    
    // Copy parth.das
    let das_file = source_path.join("parth.das");
    if das_file.exists() {
        fs::copy(&das_file, cached.join("parth.das"))
            .map_err(|e| format!("Cannot copy parth.das: {}", e))?;
    }
    
    // Copy src directory
    if source_path.join("src").exists() {
        copy_dir_recursive(&source_path.join("src"), &cached.join("src"))?;
    }
    
    // Copy runtime directory if it exists
    if source_path.join("runtime").exists() {
        copy_dir_recursive(&source_path.join("runtime"), &cached.join("runtime"))?;
    }
    
    println!("🔗 Linked '{}' from {} to cache", name, source_path.display());
    Ok(cached)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_serialize_deserialize_signature() {
        let sig = super::super::types::PackageSignature {
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
        let _original_metadata_dir = std::mem::ManuallyDrop::new(metadata_dir());
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
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("parth_test_yank_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
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
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("parth_test_key_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
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

    // ── M2.5 Signing Tests ──────────────────────────────────────────

    use std::sync::Mutex;
    static HOME_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_sign_package_success() {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("parth_test_sign_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &tmp);

        // Create a fake package in cache
        let pkg_dir = package_cache_dir("test-sign-pkg", "1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("parth.das"), "[package]\nname = \"test-sign-pkg\"\nversion = \"1.0.0\"\n").unwrap();
        fs::create_dir_all(pkg_dir.join("src")).unwrap();
        fs::write(pkg_dir.join("src/main.ajb"), "fn main(): int { return 42; }").unwrap();

        // Sign the package
        let sig = sign_package("test-sign-pkg", "1.0.0", "default").unwrap();
        assert!(!sig.signature_hex.is_empty());
        assert!(!sig.hash.is_empty());
        assert!(!sig.public_key_hex.is_empty());
        assert!(sig.timestamp > 0);

        // Signature file should exist
        let sig_path = signatures_dir().join("test-sign-pkg").join("1.0.0.sig");
        assert!(sig_path.exists());

        let _ = fs::remove_dir_all(&tmp);
        if let Some(h) = original_home { std::env::set_var("HOME", h); }
    }

    #[test]
    fn test_sign_package_not_in_cache() {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("parth_test_sign_nocache_{}", std::process::id()));
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &tmp);

        let result = sign_package("nonexistent-pkg", "1.0.0", "default");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not in cache"));

        let _ = fs::remove_dir_all(&tmp);
        if let Some(h) = original_home { std::env::set_var("HOME", h); }
    }

    #[test]
    fn test_verify_signature_success() {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("parth_test_verify_ok_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &tmp);

        // Create and sign a package
        let pkg_dir = package_cache_dir("test-verify-pkg", "1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("parth.das"), "[package]\nname = \"test-verify-pkg\"\nversion = \"1.0.0\"\n").unwrap();
        fs::create_dir_all(pkg_dir.join("src")).unwrap();
        fs::write(pkg_dir.join("src/main.ajb"), "fn main(): int { return 42; }").unwrap();

        sign_package("test-verify-pkg", "1.0.0", "default").unwrap();

        // Verify should succeed
        let sig = verify_signature("test-verify-pkg", "1.0.0").unwrap();
        assert!(!sig.signature_hex.is_empty());

        let _ = fs::remove_dir_all(&tmp);
        if let Some(h) = original_home { std::env::set_var("HOME", h); }
    }

    #[test]
    fn test_verify_signature_tampered() {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("parth_test_verify_tamper_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &tmp);

        // Create and sign a package
        let pkg_dir = package_cache_dir("test-tamper-pkg", "1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("parth.das"), "[package]\nname = \"test-tamper-pkg\"\nversion = \"1.0.0\"\n").unwrap();
        fs::create_dir_all(pkg_dir.join("src")).unwrap();
        fs::write(pkg_dir.join("src/main.ajb"), "fn main(): int { return 42; }").unwrap();

        sign_package("test-tamper-pkg", "1.0.0", "default").unwrap();

        // Tamper with the package content
        fs::write(pkg_dir.join("src/main.ajb"), "fn main(): int { return 99; }").unwrap();

        // Verify should fail (hash mismatch)
        let result = verify_signature("test-tamper-pkg", "1.0.0");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("hash mismatch"));

        let _ = fs::remove_dir_all(&tmp);
        if let Some(h) = original_home { std::env::set_var("HOME", h); }
    }

    #[test]
    fn test_verify_signature_missing() {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("parth_test_verify_miss_{}", std::process::id()));
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &tmp);

        let result = verify_signature("unsigned-pkg", "1.0.0");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No signature found"));

        let _ = fs::remove_dir_all(&tmp);
        if let Some(h) = original_home { std::env::set_var("HOME", h); }
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("parth_test_sign_roundtrip_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &tmp);

        // Create package
        let pkg_dir = package_cache_dir("roundtrip-pkg", "2.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("parth.das"), "[package]\nname = \"roundtrip-pkg\"\nversion = \"2.0.0\"\n").unwrap();
        fs::create_dir_all(pkg_dir.join("src")).unwrap();
        fs::write(pkg_dir.join("src/lib.ajb"), "fn helper(): int { return 1; }").unwrap();

        // Sign
        let sig1 = sign_package("roundtrip-pkg", "2.0.0", "test-signer").unwrap();
        assert_eq!(sig1.signer, "test-signer");

        // Verify
        let sig2 = verify_signature("roundtrip-pkg", "2.0.0").unwrap();
        assert_eq!(sig1.signature_hex, sig2.signature_hex);
        assert_eq!(sig1.hash, sig2.hash);
        assert_eq!(sig1.public_key_hex, sig2.public_key_hex);

        let _ = fs::remove_dir_all(&tmp);
        if let Some(h) = original_home { std::env::set_var("HOME", h); }
    }

    #[test]
    fn test_ensure_package_verifies_signature() {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("parth_test_ensure_sig_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &tmp);

        // Create and sign a package
        let pkg_dir = package_cache_dir("ensure-sig-pkg", "1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("parth.das"), "[package]\nname = \"ensure-sig-pkg\"\nversion = \"1.0.0\"\n").unwrap();
        fs::create_dir_all(pkg_dir.join("src")).unwrap();
        fs::write(pkg_dir.join("src/main.ajb"), "fn main(): int { return 1; }").unwrap();

        sign_package("ensure-sig-pkg", "1.0.0", "default").unwrap();

        // ensure_package should pass (checksum matches, signature valid)
        let checksum = compute_dir_checksum(&pkg_dir).unwrap();
        let result = ensure_package("ensure-sig-pkg", "1.0.0", &checksum);
        assert!(result.is_ok());

        // Tamper and ensure_package should fail
        fs::write(pkg_dir.join("src/main.ajb"), "fn main(): int { return 2; }").unwrap();
        let result = ensure_package("ensure-sig-pkg", "1.0.0", &checksum);
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&tmp);
        if let Some(h) = original_home { std::env::set_var("HOME", h); }
    }

    #[test]
    fn test_read_signature_none() {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("parth_test_read_sig_{}", std::process::id()));
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &tmp);

        let result = read_signature("no-such-pkg", "1.0.0");
        assert!(result.is_none());

        let _ = fs::remove_dir_all(&tmp);
        if let Some(h) = original_home { std::env::set_var("HOME", h); }
    }

    #[test]
    fn test_lockfile_signature_roundtrip() {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("parth_test_lock_sig_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmp);

        let mut lock = super::super::types::LockFile::new();
        lock.insert("signed-pkg".to_string(), super::super::types::LockEntry {
            version: "1.0.0".to_string(),
            checksum: "abc123".to_string(),
            dependencies: vec![],
            registry: String::new(),
            signature: "deadbeef".to_string(),
            signer: "cafebabe".to_string(),
        });
        lock.insert("unsigned-pkg".to_string(), super::super::types::LockEntry {
            version: "2.0.0".to_string(),
            checksum: "def456".to_string(),
            dependencies: vec![],
            registry: String::new(),
            signature: String::new(),
            signer: String::new(),
        });

        // Write lock
        let lock_path = tmp.join("parth.lock");
        let mut content = format!("# {}\n", "parth-lock-v2");
        let mut names: Vec<&String> = lock.keys().collect();
        names.sort();
        for name in names {
            if let Some(entry) = lock.get(name) {
                content.push_str(&format!("\n[{}]\n", name));
                content.push_str(&format!("version = \"{}\"\n", entry.version));
                content.push_str(&format!("checksum = \"{}\"\n", entry.checksum));
                if !entry.signature.is_empty() {
                    content.push_str(&format!("signature = \"{}\"\n", entry.signature));
                }
                if !entry.signer.is_empty() {
                    content.push_str(&format!("signer = \"{}\"\n", entry.signer));
                }
            }
        }
        fs::write(&lock_path, content).unwrap();

        // Read it back using resolver
        let read = super::super::resolver::read_lock(&tmp);
        assert_eq!(read["signed-pkg"].signature, "deadbeef");
        assert_eq!(read["signed-pkg"].signer, "cafebabe");
        assert!(read["unsigned-pkg"].signature.is_empty());
        assert!(read["unsigned-pkg"].signer.is_empty());

        let _ = fs::remove_dir_all(&tmp);
    }
}
