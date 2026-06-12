use std::fs;
use std::path::Path;

use super::types::PkgDep;

const PACKAGE_NAME_KEY: &str = "name";
const DEPS_SECTION: &str = "dependencies";
const PKG_SECTION: &str = "package";

/// Parse a parth.das file, returning (package_name, version, dependencies)
pub fn read_config(path: &Path) -> Result<(String, String, Vec<PkgDep>), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    let mut pkg_name = String::from("project");
    let mut pkg_version = String::new();
    let mut deps = Vec::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("//") {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            current_section = t[1..t.len() - 1].trim().to_string();
            continue;
        }
        if let Some(eq) = t.find('=') {
            let key = t[..eq].trim();
            let val = t[eq + 1..].trim().trim_matches('"');
            if current_section == PKG_SECTION {
                if key == PACKAGE_NAME_KEY {
                    pkg_name = val.to_string();
                } else if key == "version" {
                    pkg_version = val.to_string();
                }
            } else if current_section == DEPS_SECTION {
                deps.push(PkgDep {
                    name: key.to_string(),
                    version_req: val.to_string(),
                });
            }
        }
    }

    Ok((pkg_name, pkg_version, deps))
}

/// Update the [dependencies] section of a parth.das file.
/// Preserves other sections. Rebuilds the [dependencies] section from scratch.
pub fn update_deps(path: &Path, new_deps: &[PkgDep]) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;

    let mut result = String::new();
    let mut in_deps = false;
    let mut needs_blank = false;
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            let sec = t[1..t.len() - 1].trim();
            let was_in_deps = in_deps;
            in_deps = sec == DEPS_SECTION;
            if was_in_deps && !in_deps && needs_blank {
                result.push('\n');
            }
            needs_blank = false;
            result.push_str(line);
            result.push('\n');
            if in_deps {
                for dep in new_deps {
                    result.push_str(&format!("{} = \"{}\"\n", dep.name, dep.version_req));
                }
                if !new_deps.is_empty() {
                    needs_blank = true;
                }
            }
            continue;
        }
        if in_deps {
            if let Some(_) = t.find('=') {
                continue;
            }
            if t.is_empty() || t.starts_with('#') || t.starts_with("//") {
                continue;
            }
            result.push_str(line);
            result.push('\n');
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }

    if !content.contains("[dependencies]") {
        result.push_str("\n[dependencies]\n");
        for dep in new_deps {
            result.push_str(&format!("{} = \"{}\"\n", dep.name, dep.version_req));
        }
        result.push('\n');
    }

    fs::write(path, result).map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
    Ok(())
}

/// Read package metadata from a package's parth.das
pub fn read_package_meta(path: &Path) -> Result<(String, String, Vec<PkgDep>), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    let mut pkg_name = String::new();
    let mut pkg_version = String::new();
    let mut deps = Vec::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("//") {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            current_section = t[1..t.len() - 1].trim().to_string();
            continue;
        }
        if let Some(eq) = t.find('=') {
            let key = t[..eq].trim();
            let val = t[eq + 1..].trim().trim_matches('"');
            if current_section == PKG_SECTION {
                if key == PACKAGE_NAME_KEY {
                    pkg_name = val.to_string();
                } else if key == "version" {
                    pkg_version = val.to_string();
                }
            } else if current_section == DEPS_SECTION {
                deps.push(PkgDep {
                    name: key.to_string(),
                    version_req: val.to_string(),
                });
            }
        }
    }

    Ok((pkg_name, pkg_version, deps))
}
