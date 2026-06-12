use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use super::registry::{ensure_package, make_lock_entry, read_index, read_package_deps};
use super::types::{LockEntry, LockFile, PkgDep, RegistryIndex, Version, VersionConstraint};

const LOCK_FILE: &str = "parth.lock";

/// Read the lock file
pub fn read_lock(path: &Path) -> LockFile {
    let lock_path = path.join(LOCK_FILE);
    if !lock_path.exists() {
        return HashMap::new();
    }
    let content = match fs::read_to_string(&lock_path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };

    let mut lock: LockFile = HashMap::new();
    let mut current_pkg = String::new();
    let mut current_entry: Option<LockEntry> = None;

    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("//") {
            continue;
        }
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

/// Write the lock file
pub fn write_lock(lock: &LockFile, path: &Path) -> Result<(), String> {
    let lock_path = path.join(LOCK_FILE);
    let mut content = String::from("# Parth Lock File\n");

    let mut names: Vec<&String> = lock.keys().collect();
    names.sort();
    for name in names {
        if let Some(entry) = lock.get(name) {
            content.push_str(&format!("\n[{}]\n", name));
            content.push_str(&format!("version = \"{}\"\n", entry.version));
            content.push_str(&format!("checksum = \"{}\"\n", entry.checksum));
            if !entry.dependencies.is_empty() {
                let deps_str: Vec<String> = entry
                    .dependencies
                    .iter()
                    .map(|d| format!("{}@{}", d.name, d.version_req))
                    .collect();
                content.push_str(&format!("dependencies = \"{}\"\n", deps_str.join(", ")));
            }
        }
    }

    fs::write(&lock_path, content).map_err(|e| format!("Cannot write {}: {}", lock_path.display(), e))?;
    Ok(())
}

/// Parse version strings into Version for comparison. Skips unparseable entries.
fn parse_version(v: &str) -> Option<Version> {
    Version::parse(v)
}

/// Collect all versions from the registry index for a package, sorted newest-first.
fn sorted_versions<'a>(versions: &'a HashMap<String, String>) -> Vec<(Version, &'a str)> {
    let mut result: Vec<(Version, &str)> = Vec::new();
    for (ver_str, checksum) in versions {
        if let Some(v) = parse_version(ver_str) {
            result.push((v, checksum.as_str()));
        }
    }
    result.sort_by(|a, b| b.0.cmp(&a.0)); // newest first
    result
}

/// Resolve a single dependency: find best matching version.
/// Checks lock file first (for pinned versions), then the registry index.
fn resolve_version(
    name: &str,
    constraint: &VersionConstraint,
    index: &RegistryIndex,
    lock: &LockFile,
) -> Result<(Version, String), String> {
    // Check lock first (pinned version)
    if let Some(entry) = lock.get(name) {
        if let Some(pinned) = parse_version(&entry.version) {
            if constraint.matches(&pinned) {
                return Ok((pinned, entry.checksum.clone()));
            }
        }
    }

    // Resolve from index
    if let Some(versions) = index.get(name) {
        for (ver, checksum) in sorted_versions(versions) {
            if constraint.matches(&ver) {
                return Ok((ver, checksum.to_string()));
            }
        }
    }

    Err(format!(
        "Cannot resolve '{}' matching '{}': not found in registry index",
        name, constraint
    ))
}

/// Resolve all dependencies with conflict detection.
/// Returns (resolved deps, lock file entries).
pub fn resolve_and_cache(
    deps: &[PkgDep],
    project_dir: &Path,
) -> Result<(Vec<PkgDep>, LockFile), String> {
    let index = read_index();
    let existing_lock = read_lock(project_dir);
    let mut new_lock: LockFile = existing_lock.clone();
    let mut resolved: HashMap<String, Version> = HashMap::new();   // name → resolved version
    let mut resolved_order: Vec<String> = Vec::new();             // insertion order
    let mut queue: Vec<PkgDep> = deps.to_vec();

    while let Some(dep) = queue.pop() {
        let constraint = VersionConstraint::parse(&dep.version_req)
            .unwrap_or(VersionConstraint::Any);

        if let Some(existing_ver) = resolved.get(&dep.name) {
            if constraint.matches(existing_ver) {
                continue;
            } else {
                // Conflict! Build a detailed error message
                let mut conflict_info = String::new();
                // Find the original constraint that resolved this version
                for (rd_name, rd_ver) in &resolved {
                    if rd_name == &dep.name {
                        conflict_info = format!(
                            "already resolved to {}",
                            rd_ver
                        );
                        break;
                    }
                }
                return Err(format!(
                    "Dependency conflict for '{}': {} requires version matching '{}', but it is {}",
                    dep.name, conflict_info, dep.version_req, existing_ver
                ));
            }
        }

        // Not yet resolved — find the best version from index
        let (version, checksum) = resolve_version(&dep.name, &constraint, &index, &existing_lock)?;

        // Ensure the package is cached
        ensure_package(&dep.name, &version.to_string(), &checksum)?;

        let transitive = read_package_deps(&dep.name, &version.to_string());

        // Add to lock
        let lock_entry = make_lock_entry(&dep.name, &version.to_string())?;
        new_lock.insert(dep.name.clone(), lock_entry);

        resolved.insert(dep.name.clone(), version);
        resolved_order.push(dep.name.clone());

        for td in transitive {
            if let Some(existing_ver) = resolved.get(&td.name) {
                let td_constraint = VersionConstraint::parse(&td.version_req)
                    .unwrap_or(VersionConstraint::Any);
                if !td_constraint.matches(existing_ver) {
                    return Err(format!(
                        "Dependency conflict for '{}': {} requires version matching '{}', but it is already resolved to {}",
                        td.name, dep.name, td.version_req, existing_ver
                    ));
                }
                continue;
            }
            queue.push(td);
        }
    }

    // Write lock file
    write_lock(&new_lock, project_dir)?;

    // Build resolved list in insertion order
    let result: Vec<PkgDep> = resolved_order
        .iter()
        .map(|name| {
            let ver = resolved.get(name).unwrap();
            PkgDep {
                name: name.clone(),
                version_req: ver.to_string(),
            }
        })
        .collect();

    Ok((result, new_lock))
}

/// Get compilation order (topological sort by dependencies).
/// Detects and reports circular dependencies.
pub fn compilation_order(lock: &LockFile) -> Result<Vec<String>, String> {
    let mut order = Vec::new();
    let mut visited = HashSet::new();      // fully processed
    let mut in_stack = HashSet::new();     // currently on the recursion stack

    let mut names: Vec<String> = lock.keys().cloned().collect();
    names.sort();

    fn visit(
        name: &str,
        lock: &LockFile,
        visited: &mut HashSet<String>,
        in_stack: &mut HashSet<String>,
        stack_path: &mut Vec<String>,
        order: &mut Vec<String>,
    ) -> Result<(), String> {
        if visited.contains(name) {
            return Ok(());
        }
        if in_stack.contains(name) {
            // Found a cycle — build a readable path
            let cycle_start = stack_path.iter().position(|n| n == name).unwrap_or(0);
            let _cycle: Vec<&str> = stack_path[cycle_start..].iter().map(|s| s.as_str()).collect();
            return Err(format!(
                "Circular dependency detected: {} → {}",
                stack_path.join(" → "),
                name
            ));
        }

        in_stack.insert(name.to_string());
        stack_path.push(name.to_string());

        if let Some(entry) = lock.get(name) {
            for dep in &entry.dependencies {
                visit(&dep.name, lock, visited, in_stack, stack_path, order)?;
            }
        }

        stack_path.pop();
        in_stack.remove(name);
        visited.insert(name.to_string());
        order.push(name.to_string());
        Ok(())
    }

    let mut stack_path = Vec::new();
    for name in &names {
        visit(name, lock, &mut visited, &mut in_stack, &mut stack_path, &mut order)?;
    }

    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parth_mod::types::VersionConstraint;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    // ── resolve_version tests ────────────────────────────────────────

    fn make_index(entries: &[(&str, &str, &str)]) -> RegistryIndex {
        let mut idx = RegistryIndex::new();
        for (pkg, ver, chk) in entries {
            idx.entry(pkg.to_string())
                .or_insert_with(HashMap::new)
                .insert(ver.to_string(), chk.to_string());
        }
        idx
    }

    #[test]
    fn test_resolve_exact() {
        let idx = make_index(&[("foo", "1.0.0", "abc123")]);
        let lock = LockFile::new();
        let c = VersionConstraint::parse("1.0.0").unwrap();
        let (ver, chk) = resolve_version("foo", &c, &idx, &lock).unwrap();
        assert_eq!(ver, v("1.0.0"));
        assert_eq!(chk, "abc123");
    }

    #[test]
    fn test_resolve_caret_picks_newest() {
        let idx = make_index(&[("foo", "1.0.0", "a"), ("foo", "1.5.0", "b"), ("foo", "1.9.0", "c"), ("foo", "2.0.0", "d")]);
        let lock = LockFile::new();
        let c = VersionConstraint::parse("^1.0.0").unwrap();
        let (ver, _) = resolve_version("foo", &c, &idx, &lock).unwrap();
        assert_eq!(ver, v("1.9.0"));
    }

    #[test]
    fn test_resolve_caret_excludes_major() {
        let idx = make_index(&[("foo", "1.0.0", "a"), ("foo", "2.0.0", "b")]);
        let lock = LockFile::new();
        let c = VersionConstraint::parse("^1.0.0").unwrap();
        let (ver, _) = resolve_version("foo", &c, &idx, &lock).unwrap();
        assert_eq!(ver.major, 1);
    }

    #[test]
    fn test_resolve_gte_picks_newest() {
        let idx = make_index(&[("foo", "1.0.0", "a"), ("foo", "2.0.0", "b")]);
        let lock = LockFile::new();
        let c = VersionConstraint::parse(">=1.0.0").unwrap();
        let (ver, _) = resolve_version("foo", &c, &idx, &lock).unwrap();
        assert_eq!(ver, v("2.0.0"));
    }

    #[test]
    fn test_resolve_multi_digit() {
        let idx = make_index(&[("foo", "1.9.0", "a"), ("foo", "1.10.0", "b")]);
        let lock = LockFile::new();
        let c = VersionConstraint::parse(">=1.9.0").unwrap();
        let (ver, _) = resolve_version("foo", &c, &idx, &lock).unwrap();
        assert_eq!(ver, v("1.10.0"), "must pick the newest satisfying version");
    }

    #[test]
    fn test_resolve_not_found() {
        let idx = RegistryIndex::new();
        let lock = LockFile::new();
        let c = VersionConstraint::parse("1.0.0").unwrap();
        let result = resolve_version("nonexistent", &c, &idx, &lock);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_lock_pinned() {
        let idx = make_index(&[("foo", "1.0.0", "a"), ("foo", "2.0.0", "b")]);
        let mut lock = LockFile::new();
        lock.insert("foo".to_string(), LockEntry {
            version: "1.0.0".to_string(),
            checksum: "abc".to_string(),
            dependencies: vec![],
        });
        let c = VersionConstraint::parse("1.0.0").unwrap();
        let (ver, chk) = resolve_version("foo", &c, &idx, &lock).unwrap();
        assert_eq!(ver, v("1.0.0"));
        assert_eq!(chk, "abc");
    }

    // ── Conflict detection tests ─────────────────────────────────────



    // ── compilation_order tests ──────────────────────────────────────

    fn make_lock(entries: &[(&str, &str, &[(&str, &str)])]) -> LockFile {
        let mut lock = LockFile::new();
        for (name, version, deps) in entries {
            let deps_vec: Vec<PkgDep> = deps.iter().map(|(n, v)| PkgDep {
                name: n.to_string(),
                version_req: v.to_string(),
            }).collect();
            lock.insert(name.to_string(), LockEntry {
                version: version.to_string(),
                checksum: "".to_string(),
                dependencies: deps_vec,
            });
        }
        lock
    }

    #[test]
    fn test_compilation_order_linear() {
        // A depends on B, B depends on C
        let lock = make_lock(&[
            ("A", "1.0.0", &[("B", "1.0.0")]),
            ("B", "1.0.0", &[("C", "1.0.0")]),
            ("C", "1.0.0", &[]),
        ]);
        let order = compilation_order(&lock).unwrap();
        assert_eq!(order, vec!["C", "B", "A"]);
    }

    #[test]
    fn test_compilation_order_independent() {
        let lock = make_lock(&[
            ("A", "1.0.0", &[]),
            ("B", "1.0.0", &[]),
            ("C", "1.0.0", &[]),
        ]);
        let order = compilation_order(&lock).unwrap();
        assert_eq!(order, vec!["A", "B", "C"]);
    }

    #[test]
    fn test_compilation_order_diamond() {
        // A depends on B and C, both B and C depend on D
        let lock = make_lock(&[
            ("A", "1.0.0", &[("B", "1.0.0"), ("C", "1.0.0")]),
            ("B", "1.0.0", &[("D", "1.0.0")]),
            ("C", "1.0.0", &[("D", "1.0.0")]),
            ("D", "1.0.0", &[]),
        ]);
        let order = compilation_order(&lock).unwrap();
        // D must come before B and C; B and C before A
        assert!(order.iter().position(|n| n == "D") < order.iter().position(|n| n == "B"));
        assert!(order.iter().position(|n| n == "D") < order.iter().position(|n| n == "C"));
        assert!(order.iter().position(|n| n == "B") < order.iter().position(|n| n == "A"));
        assert!(order.iter().position(|n| n == "C") < order.iter().position(|n| n == "A"));
    }

    #[test]
    fn test_compilation_order_circular_detected() {
        let lock = make_lock(&[
            ("A", "1.0.0", &[("B", "1.0.0")]),
            ("B", "1.0.0", &[("A", "1.0.0")]),
        ]);
        let result = compilation_order(&lock);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Circular"), "error should mention circular dependency, got: {}", err);
    }

    #[test]
    fn test_compilation_order_self_loop() {
        let lock = make_lock(&[
            ("A", "1.0.0", &[("A", "1.0.0")]),
        ]);
        let result = compilation_order(&lock);
        assert!(result.is_err());
    }

    #[test]
    fn test_compilation_order_no_cycle() {
        // Without circular dependency, should succeed
        let lock = make_lock(&[
            ("A", "1.0.0", &[("B", "1.0.0")]),
            ("B", "1.0.0", &[("C", "1.0.0")]),
            ("C", "1.0.0", &[]),
        ]);
        assert!(compilation_order(&lock).is_ok());
    }

    // ── Lock file validatation tests ─────────────────────────────────

    #[test]
    fn test_read_lock_empty() {
        let dir = std::env::temp_dir().join(format!("parth_test_lock_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let lock = read_lock(&dir);
        assert!(lock.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_read_roundtrip() {
        let dir = std::env::temp_dir().join(format!("parth_test_roundtrip_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);

        let mut lock = LockFile::new();
        lock.insert("foo".to_string(), LockEntry {
            version: "1.0.0".to_string(),
            checksum: "abc123".to_string(),
            dependencies: vec![
                PkgDep { name: "bar".to_string(), version_req: "^1.0.0".to_string() },
            ],
        });

        write_lock(&lock, &dir).unwrap();
        let read_back = read_lock(&dir);

        assert_eq!(read_back.len(), 1);
        let entry = read_back.get("foo").unwrap();
        assert_eq!(entry.version, "1.0.0");
        assert_eq!(entry.checksum, "abc123");
        assert_eq!(entry.dependencies.len(), 1);
        assert_eq!(entry.dependencies[0].name, "bar");

        let _ = fs::remove_dir_all(&dir);
    }
}


