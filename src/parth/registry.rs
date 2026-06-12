use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::config::read_package_meta;
use super::types::{LockEntry, PkgDep, RegistryIndex};

/// Get the parth home directory (~/.parth)
pub fn parth_home() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".parth")
}

/// ~/.parth/index — the registry index file
pub fn index_path() -> PathBuf {
    parth_home().join("index")
}

/// ~/.parth/packages/<name>/<version>/
pub fn package_cache_dir(name: &str, version: &str) -> PathBuf {
    parth_home()
        .join("packages")
        .join(name)
        .join(version)
}

/// Read the registry index file
pub fn read_index() -> RegistryIndex {
    let path = index_path();
    if !path.exists() {
        return HashMap::new();
    }
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };

    let mut index: RegistryIndex = HashMap::new();
    let mut current_pkg = String::new();

    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("//") {
            continue;
        }
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

/// Write the registry index
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

    fs::write(&path, content).map_err(|e| format!("Cannot write index {}: {}", path.display(), e))?;
    Ok(())
}

/// Register a package version in the index
pub fn register_package(name: &str, version: &str, checksum: &str) -> Result<(), String> {
    let mut index = read_index();
    index
        .entry(name.to_string())
        .or_insert_with(HashMap::new)
        .insert(version.to_string(), checksum.to_string());
    write_index(&index)
}

/// Ensure a package is cached locally.
/// Checks cache, then local registry (~/.parth/packages/).
pub fn ensure_package(name: &str, version: &str, _checksum: &str) -> Result<(), String> {
    let cached = package_cache_dir(name, version);

    if cached.join("parth.das").exists() {
        return Ok(());
    }

    Err(format!(
        "Package '{}@{}' not found. Use `parth publish` first or add to ~/.parth/packages/",
        name, version
    ))
}

/// Compute SHA256 checksum of directory contents (sorted for reproducibility)
pub fn compute_dir_checksum(dir: &Path) -> Result<String, String> {
    let mut entries: Vec<String> = Vec::new();
    collect_files(dir, dir, &mut entries).map_err(|e| format!("Cannot read {}: {}", dir.display(), e))?;
    entries.sort();

    let mut input = String::new();
    for entry in &entries {
        let path = dir.join(entry);
        let content = fs::read_to_string(&path).map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
        input.push_str(&format!("{}:{}\n", entry, content));
    }

    // Write to temp file then hash it
    let tmp = std::env::temp_dir().join(format!("parth_checksum_{}", std::process::id()));
    fs::write(&tmp, &input).map_err(|e| format!("Cannot write temp file: {}", e))?;

    let out = std::process::Command::new("sha256sum")
        .arg(&tmp)
        .output()
        .map_err(|e| format!("Cannot run sha256sum: {}", e))?;

    let _ = fs::remove_file(&tmp);

    let hash = String::from_utf8_lossy(&out.stdout);
    let hash = hash.split_whitespace().next().unwrap_or("").to_string();
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

/// Package a src/ directory for publishing
pub fn package_src(pkg_dir: &Path, name: &str, version: &str) -> Result<PathBuf, String> {
    let src_dir = pkg_dir.join("src");
    if !src_dir.exists() {
        return Err("No src/ directory found".to_string());
    }

    let cache_dir = package_cache_dir(name, version);
    fs::create_dir_all(&cache_dir).map_err(|e| format!("Cannot create cache dir: {}", e))?;

    // Copy src/ contents to cache
    copy_dir_recursive(&src_dir, &cache_dir.join("src"))?;

    // Also copy parth.das to cache
    let das_src = pkg_dir.join("parth.das");
    if das_src.exists() {
        fs::copy(&das_src, cache_dir.join("parth.das")).map_err(|e| format!("Cannot copy parth.das: {}", e))?;
    }

    Ok(cache_dir)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("Cannot create {}: {}", dst.display(), e))?;
    for entry in fs::read_dir(src).map_err(|e| format!("Cannot read {}: {}", src.display(), e))? {
        let entry = entry.map_err(|e| format!("Dir entry error: {}", e))?;
        let ty = entry.file_type().map_err(|e| format!("File type error: {}", e))?;
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(&entry.path(), &dst.join(entry.file_name()))
                .map_err(|e| format!("Cannot copy {}: {}", entry.path().display(), e))?;
        }
    }
    Ok(())
}

/// Read lock entry dependencies from cached package
pub fn read_package_deps(name: &str, version: &str) -> Vec<PkgDep> {
    let pkg_dir = package_cache_dir(name, version);
    let das_path = pkg_dir.join("parth.das");
    if !das_path.exists() {
        return Vec::new();
    }
    match read_package_meta(&das_path) {
        Ok((_, _, deps)) => deps,
        Err(_) => Vec::new(),
    }
}

/// Create a LockEntry for a cached package
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
    })
}
