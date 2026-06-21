use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::paths::{metadata_dir, sanitize_pkg_segment};
use super::super::types::{RegistryIndex, RegistryMetadata};

pub fn metadata_path(name: &str, version: &str) -> PathBuf {
    metadata_dir().join(sanitize_pkg_segment(name)).join(format!("{}.json", sanitize_pkg_segment(version)))
}

pub fn read_metadata(name: &str, version: &str) -> RegistryMetadata {
    let path = metadata_path(name, version);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
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

pub fn yank_package(name: &str, version: &str) -> Result<(), String> {
    let mut meta = read_metadata(name, version);
    meta.yanked = true;
    write_metadata(name, version, &meta)
}

pub fn unyank_package(name: &str, version: &str) -> Result<(), String> {
    let mut meta = read_metadata(name, version);
    meta.yanked = false;
    write_metadata(name, version, &meta)
}

pub fn is_yanked(name: &str, version: &str) -> bool {
    read_metadata(name, version).yanked
}

pub fn read_index() -> RegistryIndex {
    let path = super::paths::index_path();
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
    let path = super::paths::index_path();
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

    if Path::new("parth.das").exists() {
        if let Ok(cfg) = super::super::config::read_config(Path::new("parth.das")) {
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
