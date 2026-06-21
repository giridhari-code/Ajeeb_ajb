use std::fs;
use std::path::{Path, PathBuf};

use super::super::config;
use super::super::registry;
use super::super::resolver;
use super::super::types;

pub fn cmd_add(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: parth add <package>[@<version>]");
        std::process::exit(1);
    }
    let spec = &args[0];
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found");
        std::process::exit(1);
    }

    let (pkg_name, version_req) = if let Some(at) = spec.find('@') {
        (spec[..at].to_string(), spec[at + 1..].to_string())
    } else {
        (spec.clone(), "*".to_string())
    };
    let original_req = version_req.clone();

    let cfg = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });

    let mut deps = cfg.deps.clone();
    if deps.iter().any(|d| d.name == pkg_name) {
        println!("ℹ️  '{}' is already a dependency", pkg_name);
        return;
    }

    match registry::find_local_package(&pkg_name) {
        Some(local_path) => {
            println!("📦 Found '{}' locally at {}", pkg_name, local_path.display());
            match registry::link_local_package(&local_path, &pkg_name) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("❌ Could not link '{}': {}", pkg_name, e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            let _ = registry::download_package(&pkg_name, &"latest".to_string(), "");
        }
    }

    let new_dep = types::PkgDep { name: pkg_name.clone(), version_req };
    let mut all_deps = deps.clone();
    all_deps.push(new_dep);

    match resolver::resolve_and_cache(&all_deps, Path::new("."), "") {
        Ok((_resolved, _lock)) => {
            deps.push(types::PkgDep { name: pkg_name.clone(), version_req: original_req });
            config::update_deps(Path::new("parth.das"), &deps).unwrap_or_else(|e| {
                eprintln!("Error: {}", e); std::process::exit(1);
            });
            println!("✓ Added '{}'", pkg_name);
        }
        Err(e) => {
            eprintln!("❌ Could not add '{}': {}", pkg_name, e);
            std::process::exit(1);
        }
    }
}

pub fn cmd_remove(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: parth remove <package>");
        std::process::exit(1);
    }
    let name = &args[0];
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found"); std::process::exit(1);
    }
    let (_, _, deps) = config::read_config_basic(Path::new("parth.das"));
    let new_deps: Vec<types::PkgDep> = deps.into_iter().filter(|d| d.name != *name).collect();
    config::update_deps(Path::new("parth.das"), &new_deps).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });
    let mut lock = resolver::read_lock(Path::new("."));
    lock.remove(name);
    resolver::write_lock(&lock, Path::new(".")).unwrap_or_default();
    println!("✓ Removed '{}'", name);
}

pub fn cmd_install(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: parth install <package>[@<version>]");
        std::process::exit(1);
    }
    let spec = &args[0];
    let (name, version) = if let Some(at) = spec.find('@') {
        (spec[..at].to_string(), spec[at + 1..].to_string())
    } else {
        (spec.clone(), String::new())
    };

    match registry::find_local_package(&name) {
        Some(local_path) => {
            println!("📦 Found '{}' locally at {}", name, local_path.display());
            match registry::link_local_package(&local_path, &name) {
                Ok(path) => {
                    println!("✓ Installed '{}' from local path: {}", name, path.display());
                    if Path::new("parth.das").exists() {
                        let deps = vec![types::PkgDep { name: name.clone(), version_req: format!("={}", version) }];
                        config::update_deps(Path::new("parth.das"), &deps).unwrap_or_else(|e| {
                            eprintln!("Warning: could not update parth.das: {}", e);
                        });
                    }
                }
                Err(e) => {
                    eprintln!("❌ Install failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            match registry::download_package(&name, &version, "") {
                Ok(path) => {
                    println!("✓ Installed '{}@{}' to {}", name, version, path.display());
                    if Path::new("parth.das").exists() {
                        let deps = vec![types::PkgDep { name: name.clone(), version_req: format!("={}", version) }];
                        config::update_deps(Path::new("parth.das"), &deps).unwrap_or_else(|e| {
                            eprintln!("Warning: could not update parth.das: {}", e);
                        });
                    }
                }
                Err(e) => {
                    eprintln!("❌ Install failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

pub fn cmd_publish(_args: &[String]) {
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found"); std::process::exit(1);
    }
    let cfg = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });
    if cfg.pkg_name.is_empty() || cfg.pkg_name == "project" {
        eprintln!("Error: package name must be set in [package] section");
        std::process::exit(1);
    }
    if cfg.pkg_version.is_empty() {
        eprintln!("Error: version must be set in [package] section");
        std::process::exit(1);
    }

    let pkg_dir = Path::new(".");
    let cache_dir = registry::package_src(pkg_dir, &cfg.pkg_name, &cfg.pkg_version).unwrap_or_else(|e| {
        eprintln!("❌ Package failed: {}", e); std::process::exit(1);
    });
    let checksum = registry::compute_dir_checksum(&cache_dir).unwrap_or_else(|e| {
        eprintln!("❌ Checksum failed: {}", e); std::process::exit(1);
    });
    registry::register_package(&cfg.pkg_name, &cfg.pkg_version, &checksum).unwrap_or_else(|e| {
        eprintln!("❌ Registry update failed: {}", e); std::process::exit(1);
    });

    match registry::sign_package(&cfg.pkg_name, &cfg.pkg_version, "default") {
        Ok(sig) => println!("🔑 Signed (hash: {}...)", &sig.hash[..16]),
        Err(e) => eprintln!("Warning: signing failed: {}", e),
    }

    println!("✓ Published '{}@{}' (checksum: {}...)", cfg.pkg_name, cfg.pkg_version, &checksum[..16]);
    println!("📦 Package cached at: {}", cache_dir.display());
}

pub fn cmd_sign(args: &[String]) {
    if args.len() < 1 {
        eprintln!("Usage: parth sign <package> [version] [signer]");
        std::process::exit(1);
    }
    let name = &args[0];
    let version = args.get(1).cloned().unwrap_or_default();
    let signer = args.get(2).cloned().unwrap_or_else(|| "default".to_string());

    let (pkg_name, pkg_version) = if name == "." {
        if !Path::new("parth.das").exists() {
            eprintln!("Error: no parth.das found"); std::process::exit(1);
        }
        let cfg = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
            eprintln!("Error: {}", e); std::process::exit(1);
        });
        (cfg.pkg_name, if version.is_empty() { cfg.pkg_version } else { version })
    } else {
        (name.clone(), version)
    };

    match registry::sign_package(&pkg_name, &pkg_version, &signer) {
        Ok(sig) => println!("🔑 Signed '{}@{}' (signer: {}, hash: {}...)",
            pkg_name, pkg_version, sig.signer, &sig.hash[..16]),
        Err(e) => { eprintln!("❌ Signing failed: {}", e); std::process::exit(1); }
    }
}

pub fn cmd_verify(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: parth verify <package> <version>");
        std::process::exit(1);
    }
    match registry::verify_signature(&args[0], &args[1]) {
        Ok(sig) => println!("✓ Verified '{}@{}' (signer: {}, timestamp: {})",
            args[0], args[1], sig.signer, sig.timestamp),
        Err(e) => { eprintln!("❌ Verification failed: {}", e); std::process::exit(1); }
    }
}

pub fn cmd_yank(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: parth yank <package> <version>");
        std::process::exit(1);
    }
    match registry::yank_package(&args[0], &args[1]) {
        Ok(()) => println!("✓ Yanked '{}@{}'", args[0], args[1]),
        Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
    }
}

pub fn cmd_unyank(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: parth unyank <package> <version>");
        std::process::exit(1);
    }
    match registry::unyank_package(&args[0], &args[1]) {
        Ok(()) => println!("✓ Un-yanked '{}@{}'", args[0], args[1]),
        Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
    }
}

pub fn cmd_keygen() {
    match registry::generate_keypair() {
        Ok((_, pub_hex)) => {
            println!("🔑 Ed25519 keypair generated");
            println!("📁 Keys stored in: {}", registry::keys_dir().display());
            println!("🔓 Public key: {}...", &pub_hex[..16]);
        }
        Err(e) => { eprintln!("❌ Key generation failed: {}", e); std::process::exit(1); }
    }
}

pub fn cmd_login(args: &[String]) {
    let registry_url = args.first().map(|s| s.as_str()).unwrap_or("https://registry.ajeeb.dev");
    match registry::login(registry_url) {
        Ok(info) => println!("✓ Logged in as '{}'", info.username),
        Err(e) => { eprintln!("❌ Login failed: {}", e); std::process::exit(1); }
    }
}

pub fn cmd_logout() {
    match registry::logout() {
        Ok(()) => {}
        Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
    }
}

pub fn cmd_whoami() {
    match registry::read_auth() {
        Some(info) => {
            println!("👤 {}", info.username);
            println!("🔗 {}", info.registry_url);
        }
        None => {
            println!("Not logged in. Use `parth login` to authenticate.");
        }
    }
}

pub fn cmd_doc() {
    let name = if Path::new("parth.das").exists() {
        let cfg = config::read_config(Path::new("parth.das")).unwrap_or_default();
        cfg.pkg_name
    } else {
        "project".to_string()
    };

    match registry::generate_project_docs(&name) {
        Ok(()) => {}
        Err(e) => { eprintln!("❌ Documentation generation failed: {}", e); std::process::exit(1); }
    }
}

pub fn cmd_link(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: parth link <path>");
        std::process::exit(1);
    }
    let path = &args[0];
    let source_path = std::fs::canonicalize(path).unwrap_or_else(|e| {
        eprintln!("Error: cannot resolve path '{}': {}", path, e);
        std::process::exit(1);
    });

    if !source_path.join("parth.das").exists() {
        eprintln!("Error: '{}' is not a valid package (no parth.das found)", path);
        std::process::exit(1);
    }

    let cfg = config::read_config(&source_path.join("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });

    if cfg.pkg_name.is_empty() || cfg.pkg_name == "project" {
        eprintln!("Error: package name must be set in [package] section");
        std::process::exit(1);
    }

    let version = if cfg.pkg_version.is_empty() { "0.1.0" } else { &cfg.pkg_version };

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let global_dir = PathBuf::from(home).join(".parth").join("packages").join(&cfg.pkg_name);

    if global_dir.exists() {
        let _ = fs::remove_dir_all(&global_dir);
    }

    if let Err(e) = registry::copy_dir_recursive(&source_path, &global_dir) {
        eprintln!("❌ Link failed: {}", e);
        std::process::exit(1);
    }

    println!("🔗 Linked: {} v{}", cfg.pkg_name, version);
    println!("   Path: {}", global_dir.display());
}

fn read_package_version_from_dir(pkg_dir: &Path) -> Option<String> {
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

pub fn cmd_list() {
    println!("📦 Available packages:");
    let mut found = false;

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let global_dir = PathBuf::from(home).join(".parth").join("packages");
    if global_dir.exists() {
        if let Ok(entries) = fs::read_dir(&global_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let version = read_package_version_from_dir(&entry.path())
                        .unwrap_or_else(|| "0.1.0".to_string());
                    println!("  {} v{} (global)", name, version);
                    found = true;
                }
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        let local_dir = cwd.join("packages");
        if local_dir.exists() {
            if let Ok(entries) = fs::read_dir(&local_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let version = read_package_version_from_dir(&entry.path())
                            .unwrap_or_else(|| "0.1.0".to_string());
                        println!("  {} v{} (local)", name, version);
                        found = true;
                    }
                }
            }
        }
    }

    if !found {
        println!("  (no packages found)");
    }
}

pub fn cmd_search(args: &[String]) {
    let query = args.first().map(|s| s.as_str()).unwrap_or("");

    let results = registry::search_packages(query, "");
    if results.is_empty() {
        println!("No packages found matching '{}'", query);
        return;
    }
    println!("📦 Search results for '{}':", query);
    for r in &results {
        let desc = if r.description.is_empty() { "".to_string() } else { format!(" — {}", r.description) };
        println!("  \u{1b}[1m{}@{}\u{1b}[0m{}", r.name, r.latest_version, desc);
    }
    println!("{} packages found", results.len());
}
