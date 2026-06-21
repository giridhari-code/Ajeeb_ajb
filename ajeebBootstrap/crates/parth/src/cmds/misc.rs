use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use super::super::config;
use super::super::registry;
use super::super::resolver;
use super::util::find_ajeeb_root;

pub fn cmd_test() {
    let test_dir = Path::new("tests");
    if !test_dir.exists() {
        eprintln!("Error: tests/ directory not found");
        std::process::exit(1);
    }
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut entries: Vec<_> = fs::read_dir(test_dir)
        .unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); })
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ex| ex == "ajb").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let root = find_ajeeb_root();

    for entry in &entries {
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        print!("  {} ... ", name);
        std::io::stdout().flush().ok();

        let status = Command::new("cargo")
            .args(["run", "-p", "ajeeb-compiler", "--bin", "ajeeb_compiler",
                   "--", &path.to_string_lossy()])
            .current_dir(&root)
            .status().unwrap_or_default();

        if status.success() {
            println!("PASS");
            passed += 1;
        } else {
            println!("FAIL");
            failed += 1;
        }
    }

    println!("\nTest results: {} passed, {} failed", passed, failed);
    if failed > 0 { std::process::exit(1); }
}

pub fn cmd_cache(args: &[String]) {
    match args.first().map(|s| s.as_str()).unwrap_or("info") {
        "info" => {
            if let Ok(size) = registry::get_cache_size() {
                println!("💾 Cache size: {} bytes", size);
            }
            println!("📁 Cache location: {}", registry::cache_root().display());
            println!("📦 Packages: {}", registry::parth_home().join("packages").display());
        }
        "clear" => {
            match registry::clear_cache() {
                Ok(()) => println!("✓ Cache cleared"),
                Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
            }
        }
        "prune" => {
            let lock = if Path::new("parth.lock").exists() {
                resolver::read_lock(Path::new("."))
            } else {
                std::collections::HashMap::new()
            };
            let packages_dir = registry::parth_home().join("packages");
            if packages_dir.exists() {
                let mut removed = 0u64;
                if let Ok(entries) = fs::read_dir(&packages_dir) {
                    for entry in entries.flatten() {
                        let pkg_name = entry.file_name().to_string_lossy().to_string();
                        if lock.contains_key(&pkg_name) { continue; }
                        if entry.path().is_dir() {
                            let _ = fs::remove_dir_all(&entry.path());
                            removed += 1;
                        }
                    }
                }
                println!("✓ Pruned {} unused packages", removed);
            }
        }
        "put" => {
            if args.len() < 3 {
                eprintln!("Usage: parth cache put <key> <value>");
                std::process::exit(1);
            }
            let key = &args[1];
            let value = &args[2];
            match registry::cache_put(key, value.as_bytes()) {
                Ok(hash) => println!("🔑 {} -> {}", key, hash),
                Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
            }
        }
        "get" => {
            if args.len() < 2 {
                eprintln!("Usage: parth cache get <key>");
                std::process::exit(1);
            }
            let key = &args[1];
            let hash = match registry::cache_lookup(key) {
                Some(h) => h,
                None => { eprintln!("❌ Key not found in cache: {}", key); std::process::exit(1); }
            };
            match registry::cache_get(&hash) {
                Some(data) => {
                    match String::from_utf8(data) {
                        Ok(s) => println!("{}", s),
                        Err(_) => eprintln!("(binary data, use 'parth cache lookup' for hash)"),
                    }
                }
                None => { eprintln!("❌ Hash not found in cache: {}", hash); std::process::exit(1); }
            }
        }
        "lookup" => {
            if args.len() < 2 {
                eprintln!("Usage: parth cache lookup <key>");
                std::process::exit(1);
            }
            let key = &args[1];
            match registry::cache_lookup(key) {
                Some(hash) => println!("🔑 {} -> {}", key, hash),
                None => { eprintln!("❌ Key not found in cache: {}", key); std::process::exit(1); }
            }
        }
        _ => {
            eprintln!("Usage: parth cache <info|clear|prune|put|get|lookup>");
            std::process::exit(1);
        }
    }
}

pub fn cmd_workspace(args: &[String]) {
    if args.is_empty() {
        let cfg = config::read_config(Path::new("parth.das")).unwrap_or_default();
        if cfg.workspace.is_empty() {
            println!("No workspace members configured.");
        } else {
            println!("📦 Workspace members:");
            for m in &cfg.workspace {
                println!("  {}", m.path);
            }
        }
        return;
    }

    match args[0].as_str() {
        "add" if args.len() >= 2 => {
            let member_path = &args[1];
            let content = fs::read_to_string("parth.das").unwrap_or_default();
            let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
            if !content.contains("[workspace]") {
                lines.push(String::new());
                lines.push("[workspace]".to_string());
            }
            let mut in_ws = false;
            for i in 0..lines.len() {
                if lines[i].trim() == "[workspace]" { in_ws = true; continue; }
                if in_ws && lines[i].starts_with('[') { break; }
                if in_ws && lines[i].trim().is_empty() {
                    lines[i] = format!("members = \"{}\"", member_path);
                    in_ws = false;
                }
            }
            if in_ws {
                lines.push(format!("members = \"{}\"", member_path));
            }
            let result = lines.join("\n");
            fs::write("parth.das", result).unwrap_or_else(|e| {
                eprintln!("Error writing parth.das: {}", e); std::process::exit(1);
            });
            println!("✓ Added workspace member '{}'", member_path);
        }
        _ => {
            eprintln!("Usage: parth workspace [add <path>]");
            std::process::exit(1);
        }
    }
}

pub fn cmd_fmt(args: &[String]) {
    let root = find_ajeeb_root();
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "ajeeb-fmt", "--"]);
    cmd.args(args);
    let status = cmd.current_dir(&root).status().expect("Failed to run ajeeb-fmt");
    std::process::exit(status.code().unwrap_or(1));
}

pub fn cmd_clean() {
    let build_dir = Path::new("build");
    if !build_dir.exists() { return; }

    let patterns = ["output.c", "output2.c", "combined.ajb", "output.s", "output.ll"];
    for pattern in &patterns {
        let path = build_dir.join(pattern);
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    }
    if let Ok(entries) = fs::read_dir(build_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "o").unwrap_or(false) {
                let _ = fs::remove_file(&p);
            }
        }
    }
    let content = fs::read_to_string("parth.das").unwrap_or_default();
    let mut name = String::from("project");
    let mut current_section = String::new();
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            current_section = t[1..t.len() - 1].trim().to_string();
        } else if let Some(eq) = t.find('=') {
            let key = t[..eq].trim();
            let val = t[eq + 1..].trim().trim_matches('"');
            if current_section == "package" && key == "name" { name = val.to_string(); }
        }
    }
    let bin_path = build_dir.join(&name);
    if bin_path.exists() {
        let _ = fs::remove_file(&bin_path);
        println!("🗑️  Removed {}", bin_path.display());
    }
    println!("🧹 Cleaned build directory");
}

pub fn cmd_audit(_args: &[String]) {
    if !Path::new("parth.lock").exists() {
        eprintln!("No lock file found. Run `parth build` first.");
        return;
    }
    let lock = resolver::read_lock(Path::new("."));

    let scan_issues = registry::security_scan(&lock);
    if !scan_issues.is_empty() {
        println!("🔒 Security issues:");
        for issue in &scan_issues {
            println!("  ⚠️  {}", issue);
        }
    } else {
        println!("🔒 No security issues found.");
    }

    let advisories = registry::audit_deps(&lock);
    if !advisories.is_empty() {
        println!("\n📋 Vulnerabilities found:");
        for adv in &advisories {
            println!("  ❌ {}: {} ({})", adv.id, adv.description, adv.severity);
        }
    } else {
        println!("📋 No known vulnerabilities.");
    }

    if let Ok(size) = registry::get_cache_size() {
        if size > 0 {
            println!("\n💾 Cache size: {} bytes", size);
        }
    }

    if scan_issues.is_empty() && advisories.is_empty() {
        println!("✅ All checks passed.");
    }
}
