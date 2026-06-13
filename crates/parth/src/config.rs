use std::fs;
use std::path::Path;

use super::types::{Feature, PkgDep, Profile, WorkspaceMember};

const DEPS_SECTION: &str = "dependencies";
const PKG_SECTION: &str = "package";

/// Full project configuration including workspace, features, profiles
#[derive(Default, Clone)]
pub struct ProjectConfig {
    pub pkg_name: String,
    pub pkg_version: String,
    pub pkg_description: String,
    pub pkg_author: String,
    pub pkg_homepage: String,
    pub pkg_license: String,
    pub deps: Vec<PkgDep>,
    pub features: Vec<Feature>,
    pub profiles: Vec<Profile>,
    pub workspace: Vec<WorkspaceMember>,
    pub registry_url: String,
}

fn find_section<'a>(lines: &'a [&str], start: usize, section: &str) -> Option<usize> {
    for (i, line) in lines.iter().enumerate().skip(start) {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            if t[1..t.len() - 1].trim() == section {
                return Some(i);
            }
        }
    }
    None
}

/// Read a full project config
pub fn read_config(path: &Path) -> Result<ProjectConfig, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    let lines: Vec<&str> = content.lines().collect();

    let mut pkg_name = String::from("project");
    let mut pkg_version = String::new();
    let mut pkg_description = String::new();
    let mut pkg_author = String::new();
    let mut pkg_homepage = String::new();
    let mut pkg_license = String::new();
    let mut registry_url = String::new();
    let mut deps = Vec::new();
    let mut features = Vec::new();
    let mut profiles = vec![Profile::default()];
    let mut workspace = Vec::new();

    // Parse package section
    if let Some(pkg_start) = find_section(&lines, 0, PKG_SECTION) {
        for i in pkg_start + 1..lines.len() {
            let line = lines[i].trim();
            if line.starts_with('[') { break; }
            if let Some(eq) = line.find('=') {
                let key = line[..eq].trim();
                let val = line[eq + 1..].trim().trim_matches('"');
                match key {
                    "name" => pkg_name = val.to_string(),
                    "version" => pkg_version = val.to_string(),
                    "description" => pkg_description = val.to_string(),
                    "author" => pkg_author = val.to_string(),
                    "homepage" => pkg_homepage = val.to_string(),
                    "license" => pkg_license = val.to_string(),
                    "registry" => registry_url = val.to_string(),
                    _ => {}
                }
            }
        }
    }

    // Parse dependencies
    if let Some(dep_start) = find_section(&lines, 0, DEPS_SECTION) {
        for i in dep_start + 1..lines.len() {
            let line = lines[i].trim();
            if line.starts_with('[') { break; }
            if let Some(eq) = line.find('=') {
                let key = line[..eq].trim();
                let val = line[eq + 1..].trim().trim_matches('"');
                deps.push(PkgDep { name: key.to_string(), version_req: val.to_string() });
            }
        }
    }

    // Parse features
    if let Some(feat_start) = find_section(&lines, 0, "features") {
        for i in feat_start + 1..lines.len() {
            let line = lines[i].trim();
            if line.starts_with('[') { break; }
            if let Some(eq) = line.find('=') {
                let key = line[..eq].trim().to_string();
                let val = line[eq + 1..].trim().trim_matches('"');
                let feature_deps: Vec<String> = val.split(',').map(|s| s.trim().to_string()).collect();
                features.push(Feature { name: key, deps: feature_deps });
            }
        }
    }

    // Parse profiles
    if let Some(prof_start) = find_section(&lines, 0, "profile.release") {
        let mut p = Profile::release();
        for i in prof_start + 1..lines.len() {
            let line = lines[i].trim();
            if line.starts_with('[') { break; }
            if let Some(eq) = line.find('=') {
                let key = line[..eq].trim();
                let val = line[eq + 1..].trim().trim_matches('"');
                match key {
                    "opt-level" => p.opt_level = val.parse().unwrap_or(3),
                    "debug" => p.debug = val == "true",
                    "lto" => p.lto = val == "true",
                    _ => {}
                }
            }
        }
        profiles.push(p);
    }

    // Parse workspace
    if let Some(ws_start) = find_section(&lines, 0, "workspace") {
        for i in ws_start + 1..lines.len() {
            let line = lines[i].trim();
            if line.starts_with('[') { break; }
            if let Some(eq) = line.find('=') {
                let _key = line[..eq].trim();
                let val = line[eq + 1..].trim().trim_matches('"');
                workspace.push(WorkspaceMember { path: val.to_string() });
            }
        }
    }

    Ok(ProjectConfig { pkg_name, pkg_version, pkg_description, pkg_author, pkg_homepage, pkg_license, deps, features, profiles, workspace, registry_url })
}

pub fn read_config_basic(path: &Path) -> (String, String, Vec<PkgDep>) {
    match read_config(path) {
        Ok(cfg) => (cfg.pkg_name, cfg.pkg_version, cfg.deps),
        Err(_) => (String::from("project"), String::new(), Vec::new()),
    }
}

pub fn update_deps(path: &Path, new_deps: &[PkgDep]) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    let mut result = String::new();
    let mut in_deps = false;
    let mut deps_written = false;

    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            let sec = t[1..t.len() - 1].trim();
            if in_deps && !deps_written {
                for dep in new_deps {
                    result.push_str(&format!("{} = \"{}\"\n", dep.name, dep.version_req));
                }
                deps_written = true;
            }
            in_deps = sec == DEPS_SECTION;
            result.push_str(line);
            result.push('\n');
            continue;
        }
        if in_deps {
            if let Some(_) = t.find('=') { continue; }
            if t.is_empty() || t.starts_with('#') || t.starts_with("//") { continue; }
        }
        result.push_str(line);
        result.push('\n');
    }

    if !deps_written {
        result.push_str("\n[dependencies]\n");
        for dep in new_deps {
            result.push_str(&format!("{} = \"{}\"\n", dep.name, dep.version_req));
        }
    }

    fs::write(path, result).map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
    Ok(())
}

pub fn read_package_meta(path: &Path) -> Result<(String, String, Vec<PkgDep>), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    let mut pkg_name = String::new();
    let mut pkg_version = String::new();
    let mut deps = Vec::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("//") { continue; }
        if t.starts_with('[') && t.ends_with(']') {
            current_section = t[1..t.len() - 1].trim().to_string();
            continue;
        }
        if let Some(eq) = t.find('=') {
            let key = t[..eq].trim();
            let val = t[eq + 1..].trim().trim_matches('"');
            if current_section == PKG_SECTION {
                if key == "name" { pkg_name = val.to_string(); }
                else if key == "version" { pkg_version = val.to_string(); }
            } else if current_section == DEPS_SECTION {
                deps.push(PkgDep { name: key.to_string(), version_req: val.to_string() });
            }
        }
    }
    Ok((pkg_name, pkg_version, deps))
}
