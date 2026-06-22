use std::fs;
use std::path::Path;

use crate::config;
use crate::registry;
use crate::resolver;
use crate::types;

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

    // Try to find package locally first
    match registry::find_local_package(&pkg_name) {
        Some(local_path) => {
            println!("📦 Found '{}' locally at {}", pkg_name, local_path.display());
            // Copy to cache
            match registry::link_local_package(&local_path, &pkg_name) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("❌ Could not link '{}': {}", pkg_name, e);
                    std::process::exit(1);
                }
            }
        }
        None => {
            // Try to download (will also search local paths)
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

pub fn cmd_update() {
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found"); std::process::exit(1);
    }
    let cfg = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });
    if cfg.deps.is_empty() {
        println!("No dependencies to update.");
        return;
    }

    // Delete existing lock to force re-resolution
    let lock_path = Path::new("parth.lock");
    if lock_path.exists() {
        let _ = fs::remove_file(lock_path);
    }

    match resolver::resolve_and_cache(&cfg.deps, Path::new("."), "") {
        Ok((resolved, _lock)) => {
            println!("✓ Dependencies updated:");
            for dep in &resolved {
                println!("  {}@{}", dep.name, dep.version_req);
            }
        }
        Err(e) => {
            eprintln!("❌ Update failed: {}", e);
            std::process::exit(1);
        }
    }
}

pub fn cmd_upgrade(args: &[String]) {
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found");
        std::process::exit(1);
    }
    let mut cfg = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });

    if args.is_empty() {
        // Upgrade all: delete lock and re-resolve
        let lock_path = Path::new("parth.lock");
        if lock_path.exists() {
            let _ = fs::remove_file(lock_path);
        }
        println!("✓ Lock file removed. Run `parth build` to re-resolve all dependencies");
        return;
    }

    let pkg_name = &args[0];
    let new_constraint = args.get(1).cloned().unwrap_or_else(|| "*".to_string());
    if resolver::upgrade_dep(&mut cfg.deps, pkg_name, &new_constraint) {
        config::update_deps(Path::new("parth.das"), &cfg.deps).unwrap_or_else(|e| {
            eprintln!("Error: {}", e); std::process::exit(1);
        });
        println!("✓ Upgraded '{}' to constraint '{}'", pkg_name, new_constraint);
    } else {
        eprintln!("❌ Package '{}' not found in dependencies", pkg_name);
        std::process::exit(1);
    }
}

pub fn cmd_tree() {
    let lock = if Path::new("parth.lock").exists() {
        resolver::read_lock(Path::new("."))
    } else {
        eprintln!("No parth.lock found. Run `parth build` first.");
        std::process::exit(1);
    };
    resolver::print_tree(&lock);
}

pub fn cmd_why(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: parth why <package>");
        std::process::exit(1);
    }
    let lock = resolver::read_lock(Path::new("."));
    let explanations = resolver::why(&lock, &args[0]);
    for line in &explanations {
        println!("{}", line);
    }
}

pub fn cmd_outdated() {
    let _cfg = if Path::new("parth.das").exists() {
        config::read_config(Path::new("parth.das")).unwrap_or_default()
    } else {
        eprintln!("No parth.das found");
        std::process::exit(1);
    };
    let lock = resolver::read_lock(Path::new("."));
    if lock.is_empty() {
        eprintln!("No parth.lock found. Run `parth build` first.");
        std::process::exit(1);
    }
    let outdated = resolver::check_outdated(&lock, "");
    if outdated.is_empty() {
        println!("✓ All dependencies are up to date");
    } else {
        println!("📦 Outdated dependencies:");
        for (name, current, latest) in &outdated {
            println!("  {}: {} -> {}", name, current, latest);
        }
    }
}
