use std::path::Path;

use super::super::config;
use super::super::resolver;

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

pub fn cmd_upgrade(args: &[String]) {
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found");
        std::process::exit(1);
    }
    let mut cfg = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });

    if args.is_empty() {
        let lock_path = Path::new("parth.lock");
        if lock_path.exists() {
            let _ = std::fs::remove_file(lock_path);
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

    let lock_path = Path::new("parth.lock");
    if lock_path.exists() {
        let _ = std::fs::remove_file(lock_path);
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
