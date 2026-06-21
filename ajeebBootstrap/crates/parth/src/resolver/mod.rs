use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::registry::{read_package_deps, remote_fetch_index};
use super::types::{LockEntry, LockFile, PkgDep, RegistryIndex, Version};

mod resolve;
#[cfg(test)]
mod tests;

pub use resolve::{resolve_and_cache, compilation_order};

const LOCK_FILE: &str = "parth.lock";
const LOCK_VERSION: &str = "parth-lock-v2";

pub fn read_lock(path: &Path) -> LockFile {
    let lock_path = path.join(LOCK_FILE);
    if !lock_path.exists() { return HashMap::new(); }
    let content = match fs::read_to_string(&lock_path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    let mut lock: LockFile = HashMap::new();
    let mut current_pkg = String::new();
    let mut current_entry: Option<LockEntry> = None;

    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("//") { continue; }
        if t.starts_with('[') && t.ends_with(']') {
            if let (name, Some(entry)) = (current_pkg.clone(), current_entry.take()) {
                lock.insert(name, entry);
            }
            current_pkg = t[1..t.len() - 1].trim().to_string();
            if let Some(stripped) = current_pkg.strip_prefix("package.") {
                current_pkg = stripped.to_string();
            }
            current_entry = Some(LockEntry {
                version: String::new(),
                checksum: String::new(),
                dependencies: Vec::new(),
                registry: String::new(),
            });
            continue;
        }
        if let Some(eq) = t.find('=') {
            let key = t[..eq].trim();
            let val = t[eq + 1..].trim().trim_matches('"');
            if let Some(ref mut entry) = current_entry {
                match key {
                    "version" => entry.version = val.to_string(),
                    "checksum" => entry.checksum = val.to_string(),
                    "registry" => entry.registry = val.to_string(),
                    "dependencies" => {
                        if !val.is_empty() {
                            for part in val.split(',') {
                                let part = part.trim();
                                if let Some(at) = part.find('@') {
                                    entry.dependencies.push(PkgDep {
                                        name: part[..at].trim().to_string(),
                                        version_req: part[at + 1..].trim().to_string(),
                                    });
                                } else if !part.is_empty() {
                                    entry.dependencies.push(PkgDep {
                                        name: part.to_string(),
                                        version_req: "*".to_string(),
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    if let (name, Some(entry)) = (current_pkg, current_entry) {
        lock.insert(name, entry);
    }
    lock
}

pub fn write_lock(lock: &LockFile, path: &Path) -> Result<(), String> {
    let lock_path = path.join(LOCK_FILE);
    let mut content = format!("# {}\n", LOCK_VERSION);
    let mut names: Vec<&String> = lock.keys().collect();
    names.sort();
    for name in names {
        if let Some(entry) = lock.get(name) {
            content.push_str(&format!("\n[{}]\n", name));
            content.push_str(&format!("version = \"{}\"\n", entry.version));
            content.push_str(&format!("checksum = \"{}\"\n", entry.checksum));
            if !entry.registry.is_empty() {
                content.push_str(&format!("registry = \"{}\"\n", entry.registry));
            }
            if !entry.dependencies.is_empty() {
                let deps_str: Vec<String> = entry.dependencies.iter()
                    .map(|d| format!("{}@{}", d.name, d.version_req)).collect();
                content.push_str(&format!("dependencies = \"{}\"\n", deps_str.join(", ")));
            }
        }
    }
    fs::write(&lock_path, content).map_err(|e| format!("Cannot write {}: {}", lock_path.display(), e))?;
    Ok(())
}

fn read_local_index() -> RegistryIndex {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let path = std::path::PathBuf::from(home).join(".parth").join("index");
    if !path.exists() { return HashMap::new(); }
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
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

fn make_lock_entry_with_registry(name: &str, version: &str, registry: &str) -> Result<LockEntry, String> {
    let cached = super::registry::package_cache_dir(name, version);
    if !cached.exists() {
        return Err(format!("Package '{}@{}' not in cache", name, version));
    }
    let checksum = super::registry::compute_dir_checksum(&cached)?;
    let deps = read_package_deps(name, version);
    Ok(LockEntry {
        version: version.to_string(),
        checksum,
        dependencies: deps,
        registry: registry.to_string(),
    })
}

pub fn print_tree(lock: &LockFile) {
    let order = match compilation_order(lock) {
        Ok(o) => o,
        Err(e) => { eprintln!("❌ {}", e); return; }
    };
    println!("📦 Dependency tree:");
    let mut seen = std::collections::HashSet::new();
    for pkg in &order {
        print_dep_tree(lock, pkg, 0, &mut seen);
    }
}

fn print_dep_tree(lock: &LockFile, name: &str, depth: usize, seen: &mut std::collections::HashSet<String>) {
    let indent = "  ".repeat(depth);
    let marker = if seen.contains(name) { " (already shown)" } else { "" };
    println!("{}{} {}{}", indent, if depth == 0 { "├──" } else { "├──" }, name, marker);
    if seen.contains(name) { return; }
    seen.insert(name.to_string());
    if let Some(entry) = lock.get(name) {
        for dep in &entry.dependencies {
            print_dep_tree(lock, &dep.name, depth + 1, seen);
        }
    }
}

pub fn why(lock: &LockFile, package_name: &str) -> Vec<String> {
    let mut explanations = Vec::new();
    for (name, entry) in lock {
        for dep in &entry.dependencies {
            if dep.name == package_name {
                explanations.push(format!("'{}' requires '{}' (constraint: {})", name, dep.name, dep.version_req));
            }
        }
    }
    if explanations.is_empty() {
        if lock.contains_key(package_name) {
            explanations.push(format!("'{}' is a direct dependency", package_name));
        } else {
            explanations.push(format!("'{}' not found in lock file", package_name));
        }
    }
    explanations
}

pub fn check_outdated(lock: &LockFile, registry_url: &str) -> Vec<(String, String, String)> {
    let mut outdated = Vec::new();
    for (name, entry) in lock {
        let index = remote_fetch_index(registry_url, name);
        if let Some(versions) = index.get(name) {
            let current = Version::parse(&entry.version);
            let mut latest: Option<Version> = None;
            for ver_str in versions.keys() {
                if let Some(v) = Version::parse(ver_str) {
                    match &latest {
                        Some(l) if v > *l => latest = Some(v),
                        None => latest = Some(v),
                        _ => {}
                    }
                }
            }
            if let (Some(cur), Some(latest_v)) = (current, latest) {
                if latest_v > cur {
                    outdated.push((name.clone(), cur.to_string(), latest_v.to_string()));
                }
            }
        }
    }
    outdated
}

pub fn upgrade_dep(deps: &mut Vec<PkgDep>, name: &str, new_constraint: &str) -> bool {
    for dep in deps.iter_mut() {
        if dep.name == name {
            dep.version_req = new_constraint.to_string();
            return true;
        }
    }
    false
}
