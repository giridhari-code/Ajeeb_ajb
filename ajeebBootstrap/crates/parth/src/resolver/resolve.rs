use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use crate::registry::{ensure_package, read_package_deps, remote_fetch_index};
use crate::types::{Decision, LockFile, PkgDep, RegistryIndex, Version, VersionConstraint};

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
    if let Some(entry) = lock.get(name) {
        if let Some(pinned) = parse_version(&entry.version) {
            if constraint.matches(&pinned) {
                return Ok((pinned, entry.checksum.clone()));
            }
        }
    }
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
    if let Some(versions) = index.get(name) {
        for (ver, checksum) in sorted_versions(versions) {
            if constraint.matches(&ver) {
                return Ok((ver, checksum.to_string()));
            }
        }
    }
    Err(format!("Cannot resolve '{}' matching '{}': not found", name, constraint))
}

pub fn resolve_and_cache(
    deps: &[PkgDep], project_dir: &Path, registry_url: &str,
) -> Result<(Vec<PkgDep>, LockFile), String> {
    let index = super::read_local_index();
    let existing_lock = super::read_lock(project_dir);
    let mut new_lock: LockFile = existing_lock.clone();
    let mut resolved: HashMap<String, Version> = HashMap::new();
    let mut resolved_order: Vec<String> = Vec::new();
    let mut queue: VecDeque<PkgDep> = deps.iter().cloned().collect();

    let mut decision_trail: Vec<Decision> = Vec::new();
    let mut tried: HashMap<String, Vec<(Version, String)>> = HashMap::new();

    while let Some(dep) = queue.pop_front() {
        let constraint = VersionConstraint::parse(&dep.version_req)
            .unwrap_or(VersionConstraint::Any);

        if let Some(existing_ver) = resolved.get(&dep.name) {
            if constraint.matches(existing_ver) {
                continue;
            }
            let mut backtracked = false;
            while let Some(prev_decision) = decision_trail.pop() {
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

                for td in &prev_decision.dependencies {
                    if !resolved.contains_key(&td.name) {
                        queue.push_front(td.clone());
                    }
                }

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
                        ensure_package(&pkg_name, &new_ver.to_string(), &new_checksum)?;
                        let transitive = read_package_deps(&pkg_name, &new_ver.to_string());
                        let lock_entry = super::make_lock_entry_with_registry(
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

        let (version, checksum) = match resolve_version(
            &dep.name, &constraint, &index, &existing_lock, registry_url,
        ) {
            Ok(v) => v,
            Err(e) => {
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
                        let lock_entry = super::make_lock_entry_with_registry(
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

        decision_trail.push(Decision {
            package: dep.name.clone(),
            version: version.clone(),
            dependencies: transitive.clone(),
            level: decision_trail.len(),
        });

        let lock_entry = super::make_lock_entry_with_registry(
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

    let active: HashSet<String> = resolved.keys().cloned().collect();
    new_lock.retain(|k, _| active.contains(k));

    super::write_lock(&new_lock, project_dir)?;

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

    if let Some(versions) = index.get(name) {
        for (ver, checksum) in sorted_versions(versions) {
            if constraint.matches(&ver) && !tried_versions.contains(&ver.to_string()) {
                return Ok((ver, checksum.to_string()));
            }
        }
    }

    Err(format!("No alternative version for '{}' matching '{}'", name, constraint))
}

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
