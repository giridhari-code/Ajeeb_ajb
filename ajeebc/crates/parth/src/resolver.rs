use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::Path;

use super::registry::{ensure_package, read_package_deps, remote_fetch_index};
use super::types::{Decision, LockEntry, LockFile, PkgDep, RegistryIndex, Version, VersionConstraint};

const LOCK_FILE: &str = "parth.lock";
const LOCK_VERSION: &str = "parth-lock-v2";

// ── Lock file v2 (resolved transitive versions) ────────────────────

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
                signature: String::new(),
                signer: String::new(),
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
                    "signature" => entry.signature = val.to_string(),
                    "signer" => entry.signer = val.to_string(),
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
            if !entry.signature.is_empty() {
                content.push_str(&format!("signature = \"{}\"\n", entry.signature));
            }
            if !entry.signer.is_empty() {
                content.push_str(&format!("signer = \"{}\"\n", entry.signer));
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

// ── PubGrub-style backtracking resolver ────────────────────────────

fn parse_version(s: &str) -> Option<Version> { Version::parse(s) }

fn sorted_versions<'a>(versions: &'a HashMap<String, String>) -> Vec<(Version, &'a str)> {
    let mut result: Vec<(Version, &str)> = Vec::new();
    for (ver_str, checksum) in versions {
        if let Some(v) = parse_version(ver_str) {
            result.push((v, checksum.as_str()));
        }
    }
    result.sort_by(|a, b| b.0.cmp(&a.0));
    result
}

fn resolve_version(
    name: &str, constraint: &VersionConstraint,
    index: &RegistryIndex, lock: &LockFile, registry_url: &str,
) -> Result<(Version, String), String> {
    // Check lock first
    if let Some(entry) = lock.get(name) {
        if let Some(pinned) = parse_version(&entry.version) {
            if constraint.matches(&pinned) {
                return Ok((pinned, entry.checksum.clone()));
            }
        }
    }
    // Try remote registry first if URL is set
    if !registry_url.is_empty() {
        let remote_idx = remote_fetch_index(registry_url, name);
        if let Some(versions) = remote_idx.get(name) {
            for (ver, checksum) in sorted_versions(versions) {
                if constraint.matches(&ver) {
                    return Ok((ver, checksum.to_string()));
                }
            }
        }
    }
    // Fall back to local index
    if let Some(versions) = index.get(name) {
        for (ver, checksum) in sorted_versions(versions) {
            if constraint.matches(&ver) {
                return Ok((ver, checksum.to_string()));
            }
        }
    }
    Err(format!("Cannot resolve '{}' matching '{}': not found", name, constraint))
}

/// Backtracking dependency resolver.
/// Uses DFS with backtracking on conflict.
pub fn resolve_and_cache(
    deps: &[PkgDep], project_dir: &Path, registry_url: &str,
) -> Result<(Vec<PkgDep>, LockFile), String> {
    let index = read_local_index();
    let existing_lock = read_lock(project_dir);
    let mut new_lock: LockFile = existing_lock.clone();
    let mut resolved: HashMap<String, Version> = HashMap::new();
    let mut resolved_order: Vec<String> = Vec::new();
    let mut queue: VecDeque<PkgDep> = deps.iter().cloned().collect();

    // For backtracking: keep a trail of decisions
    let mut decision_trail: Vec<Decision> = Vec::new();
    // Track which versions were tried for each package (for backtracking)
    let mut tried: HashMap<String, Vec<(Version, String)>> = HashMap::new();

    while let Some(dep) = queue.pop_front() {
        let constraint = VersionConstraint::parse(&dep.version_req)
            .unwrap_or(VersionConstraint::Any);

        // Check if already resolved
        if let Some(existing_ver) = resolved.get(&dep.name) {
            if constraint.matches(existing_ver) {
                continue;
            }
            // Try backtracking: find another version that satisfies both
            let mut backtracked = false;
            while let Some(prev_decision) = decision_trail.pop() {
                // Remove everything after this point
                let mut to_remove: Vec<String> = Vec::new();
                for (pkg, _) in &resolved {
                    if *pkg != prev_decision.package {
                        to_remove.push(pkg.clone());
                    }
                }
                for pkg in &to_remove {
                    resolved.remove(pkg);
                    resolved_order.retain(|x| x != pkg);
                    new_lock.remove(pkg);
                }

                // Re-add the previous decision's dependency deps to the queue
                for td in &prev_decision.dependencies {
                    if !resolved.contains_key(&td.name) {
                        queue.push_front(td.clone());
                    }
                }

                // Try the next version for the conflict package
                let pkg_name = prev_decision.package.clone();
                let tried_entries = tried.entry(pkg_name.clone()).or_default();
                tried_entries.push((prev_decision.version.clone(), String::new()));

                let existing_lock_clone = existing_lock.clone();
                let resolve_result = try_alternate_version(
                    &pkg_name, &constraint, &index, &existing_lock_clone,
                    registry_url, tried_entries,
                );

                match resolve_result {
                    Ok((new_ver, new_checksum)) => {
                        // Cache and add transitive deps
                        ensure_package(&pkg_name, &new_ver.to_string(), &new_checksum)?;
                        let transitive = read_package_deps(&pkg_name, &new_ver.to_string());
                        let lock_entry = make_lock_entry_with_registry(
                            &pkg_name, &new_ver.to_string(), registry_url,
                        )?;
                        new_lock.insert(pkg_name.clone(), lock_entry);
                        resolved.insert(pkg_name.clone(), new_ver.clone());
                        resolved_order.push(pkg_name.clone());

                        for td in transitive {
                            if !resolved.contains_key(&td.name) {
                                queue.push_front(td);
                            }
                        }

                        backtracked = true;
                        break;
                    }
                    Err(_) => {
                        // Keep backtracking
                        continue;
                    }
                }
            }
            if backtracked { continue; }

            return Err(format!(
                "Dependency conflict for '{}': cannot satisfy constraint '{}'",
                dep.name, dep.version_req
            ));
        }

        // Resolve from registry/lock
        let (version, checksum) = match resolve_version(
            &dep.name, &constraint, &index, &existing_lock, registry_url,
        ) {
            Ok(v) => v,
            Err(e) => {
                // Try with backtracking
                if decision_trail.is_empty() { return Err(e); }
                let mut backtracked = false;
                while let Some(prev) = decision_trail.pop() {
                    let pkg_name = prev.package.clone();
                    let tried_entries = tried.entry(pkg_name.clone()).or_default();
                    tried_entries.push((prev.version.clone(), String::new()));

                    let existing_lock_clone = existing_lock.clone();
                    if let Ok((new_ver, new_chk)) = try_alternate_version(
                        &pkg_name, &constraint, &index, &existing_lock_clone,
                        registry_url, tried_entries,
                    ) {
                        ensure_package(&pkg_name, &new_ver.to_string(), &new_chk)?;
                        let transitive = read_package_deps(&pkg_name, &new_ver.to_string());
                        let lock_entry = make_lock_entry_with_registry(
                            &pkg_name, &new_ver.to_string(), registry_url,
                        )?;
                        new_lock.insert(pkg_name.clone(), lock_entry);
                        resolved.insert(pkg_name.clone(), new_ver.clone());

                        for td in transitive {
                            if !resolved.contains_key(&td.name) {
                                queue.push_front(td);
                            }
                        }
                        backtracked = true;
                        break;
                    }
                }
                if backtracked { continue; }
                return Err(e);
            }
        };

        ensure_package(&dep.name, &version.to_string(), &checksum)?;
        let transitive = read_package_deps(&dep.name, &version.to_string());

        let mut transitive_pinned: Vec<PkgDep> = Vec::new();
        for td in &transitive {
            if let Some(entry) = existing_lock.get(&td.name) {
                transitive_pinned.push(PkgDep {
                    name: td.name.clone(),
                    version_req: format!("={}", entry.version),
                });
            } else {
                transitive_pinned.push(td.clone());
            }
        }

        // Record the decision for backtracking
        decision_trail.push(Decision {
            package: dep.name.clone(),
            version: version.clone(),
            dependencies: transitive.clone(),
            level: decision_trail.len(),
        });

        let lock_entry = make_lock_entry_with_registry(
            &dep.name, &version.to_string(), registry_url,
        )?;
        new_lock.insert(dep.name.clone(), lock_entry);
        resolved.insert(dep.name.clone(), version);
        resolved_order.push(dep.name.clone());

        for td in transitive_pinned {
            if let Some(existing_ver) = resolved.get(&td.name) {
                let td_constraint = VersionConstraint::parse(&td.version_req)
                    .unwrap_or(VersionConstraint::Any);
                if !td_constraint.matches(existing_ver) {
                    return Err(format!(
                        "Dependency conflict for '{}': need '{}' but already resolved to {}",
                        td.name, td.version_req, existing_ver
                    ));
                }
                continue;
            }
            queue.push_back(td);
        }
    }

    // Clean stale entries from lock
    let active: HashSet<String> = resolved.keys().cloned().collect();
    new_lock.retain(|k, _| active.contains(k));

    write_lock(&new_lock, project_dir)?;

    let result: Vec<PkgDep> = resolved_order.iter()
        .map(|name| {
            let ver = resolved.get(name).unwrap();
            PkgDep { name: name.clone(), version_req: ver.to_string() }
        })
        .collect();

    Ok((result, new_lock))
}

fn try_alternate_version(
    name: &str, constraint: &VersionConstraint,
    index: &RegistryIndex, _lock: &LockFile,
    registry_url: &str, tried: &[(Version, String)],
) -> Result<(Version, String), String> {
    let tried_versions: HashSet<String> = tried.iter().map(|(v, _)| v.to_string()).collect();

    // Check remote registry
    if !registry_url.is_empty() {
        let remote_idx = remote_fetch_index(registry_url, name);
        if let Some(versions) = remote_idx.get(name) {
            for (ver, checksum) in sorted_versions(versions) {
                if constraint.matches(&ver) && !tried_versions.contains(&ver.to_string()) {
                    return Ok((ver, checksum.to_string()));
                }
            }
        }
    }

    // Check local index
    if let Some(versions) = index.get(name) {
        for (ver, checksum) in sorted_versions(versions) {
            if constraint.matches(&ver) && !tried_versions.contains(&ver.to_string()) {
                return Ok((ver, checksum.to_string()));
            }
        }
    }

    Err(format!("No alternative version for '{}' matching '{}'", name, constraint))
}

fn read_local_index() -> RegistryIndex {
    use std::path::PathBuf;
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let path = PathBuf::from(home).join(".parth").join("index");
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

    // Try to read existing signature for this package
    let (signature, signer) = match super::registry::read_signature(name, version) {
        Some(sig) => (sig.signature_hex, sig.public_key_hex),
        None => (String::new(), String::new()),
    };

    Ok(LockEntry {
        version: version.to_string(),
        checksum,
        dependencies: deps,
        registry: registry.to_string(),
        signature,
        signer,
    })
}

/// Topological sort with cycle detection
pub fn compilation_order(lock: &LockFile) -> Result<Vec<String>, String> {
    let mut order = Vec::new();
    let mut visited = HashSet::new();
    let mut in_stack = HashSet::new();

    let mut names: Vec<String> = lock.keys().cloned().collect();
    names.sort();

    fn visit(
        name: &str, lock: &LockFile,
        visited: &mut HashSet<String>, in_stack: &mut HashSet<String>,
        stack_path: &mut Vec<String>, order: &mut Vec<String>,
    ) -> Result<(), String> {
        if visited.contains(name) { return Ok(()); }
        if in_stack.contains(name) {
            return Err(format!("Circular dependency: {} → {}", stack_path.join(" → "), name));
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

// ── Extended commands ──────────────────────────────────────────────

/// Print dependency tree from a lock file
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

/// Explain why a package is included in the dependency tree
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

/// Check for outdated dependencies by comparing lock file versions against registry
pub fn check_outdated(lock: &LockFile, registry_url: &str) -> Vec<(String, String, String)> {
    use super::registry::remote_fetch_index;
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

/// Upgrade a specific dependency to latest matching constraint
pub fn upgrade_dep(deps: &mut Vec<PkgDep>, name: &str, new_constraint: &str) -> bool {
    for dep in deps.iter_mut() {
        if dep.name == name {
            dep.version_req = new_constraint.to_string();
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version { Version::parse(s).unwrap() }

    fn make_lock(entries: &[(&str, &str, &[(&str, &str)])]) -> LockFile {
        let mut lock = LockFile::new();
        for (name, version, deps) in entries {
            let deps_vec: Vec<PkgDep> = deps.iter().map(|(n, v)| PkgDep {
                name: n.to_string(), version_req: v.to_string(),
            }).collect();
            lock.insert(name.to_string(), LockEntry {
                version: version.to_string(), checksum: "".to_string(),
                dependencies: deps_vec, registry: String::new(),
                signature: String::new(), signer: String::new(),
            });
        }
        lock
    }

    #[test]
    fn test_compilation_order_linear() {
        let lock = make_lock(&[
            ("A", "1.0.0", &[("B", "1.0.0")]),
            ("B", "1.0.0", &[("C", "1.0.0")]),
            ("C", "1.0.0", &[]),
        ]);
        let order = compilation_order(&lock).unwrap();
        assert_eq!(order, vec!["C", "B", "A"]);
    }

    #[test]
    fn test_compilation_order_diamond() {
        let lock = make_lock(&[
            ("A", "1.0.0", &[("B", "1.0.0"), ("C", "1.0.0")]),
            ("B", "1.0.0", &[("D", "1.0.0")]),
            ("C", "1.0.0", &[("D", "1.0.0")]),
            ("D", "1.0.0", &[]),
        ]);
        let order = compilation_order(&lock).unwrap();
        assert!(order.iter().position(|n| n == "D") < order.iter().position(|n| n == "B"));
        assert!(order.iter().position(|n| n == "D") < order.iter().position(|n| n == "C"));
        assert!(order.iter().position(|n| n == "B") < order.iter().position(|n| n == "A"));
    }

    #[test]
    fn test_compilation_order_circular() {
        let lock = make_lock(&[
            ("A", "1.0.0", &[("B", "1.0.0")]),
            ("B", "1.0.0", &[("A", "1.0.0")]),
        ]);
        assert!(compilation_order(&lock).is_err());
    }

    #[test]
    fn test_lock_v2_roundtrip() {
        let dir = std::env::temp_dir().join(format!("parth_test_v2_{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let mut lock = LockFile::new();
        lock.insert("foo".to_string(), LockEntry {
            version: "1.0.0".to_string(), checksum: "abc".to_string(),
            dependencies: vec![PkgDep { name: "bar".into(), version_req: "^1.0.0".into() }],
            registry: "https://registry.example.com".into(),
            signature: String::new(), signer: String::new(),
        });
        write_lock(&lock, &dir).unwrap();
        let read = read_lock(&dir);
        assert!(read.contains_key("foo"));
        assert_eq!(read["foo"].version, "1.0.0");
        assert_eq!(read["foo"].registry, "https://registry.example.com");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_compound_constraint() {
        let c = VersionConstraint::parse(">=1.0.0 <2.0.0").unwrap();
        assert!(c.matches(&Version::parse("1.5.0").unwrap()));
        assert!(!c.matches(&Version::parse("2.0.0").unwrap()));

        let c = VersionConstraint::parse("^1.0.0 || ^2.0.0").unwrap();
        assert!(c.matches(&Version::parse("1.5.0").unwrap()));
        assert!(c.matches(&Version::parse("2.0.0").unwrap()));
        assert!(!c.matches(&Version::parse("3.0.0").unwrap()));
    }
}
