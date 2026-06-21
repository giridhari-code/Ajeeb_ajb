use std::fs;
use std::path::PathBuf;

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

pub fn find_local_package(name: &str) -> Option<PathBuf> {
    let search_roots = local_package_search_paths();
    for root in &search_roots {
        let pkg_dir = root.join(sanitize_pkg_segment(name));
        if pkg_dir.exists() && pkg_dir.join("parth.das").exists() {
            return Some(pkg_dir);
        }
    }
    None
}

pub fn local_package_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("packages"));
    }
    paths.push(parth_home().join("packages"));
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(parent) = cwd.parent() {
            paths.push(parent.join("packages"));
        }
    }
    let root = find_ajeeb_root();
    paths.push(root.join("packages"));
    paths
}

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
