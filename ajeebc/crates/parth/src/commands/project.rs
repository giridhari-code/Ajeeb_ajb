use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::config;
use crate::find_ajeeb_root;
use crate::registry;
use crate::resolver;

pub fn cmd_info() {
    let content = fs::read_to_string("parth.das").unwrap_or_default();
    println!("📦 parth.das:\n{}", content);
    if Path::new("parth.lock").exists() {
        let lock = fs::read_to_string("parth.lock").unwrap_or_default();
        println!("🔒 parth.lock:\n{}", lock);
    }
}

pub fn cmd_version() {
    println!("parth 1.0.0 — Ajeeb Package Manager");
    if let Ok(content) = fs::read_to_string("parth.das") {
        for line in content.lines() {
            let t = line.trim();
            if let Some(eq) = t.find('=') {
                let key = t[..eq].trim();
                let val = t[eq + 1..].trim().trim_matches('"');
                if key == "name" {
                    print!("{} v", val);
                } else if key == "version" {
                    println!("{}", val);
                }
            }
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

pub fn cmd_doc_open() {
    cmd_doc();
    let doc_path = Path::new("build/doc.html");
    if doc_path.exists() {
        let _ = Command::new("xdg-open").arg(doc_path.to_str().unwrap()).spawn();
        let _ = Command::new("open").arg(doc_path.to_str().unwrap()).spawn();
    }
}

pub fn cmd_ls() {
    let mut packages: Vec<(String, String, String)> = Vec::new();

    if Path::new("parth.das").exists() {
        if let Ok(cfg) = config::read_config(&std::env::current_dir().unwrap().join("parth.das")) {
            packages.push((cfg.pkg_name.clone(), cfg.pkg_version.clone(), ".".to_string()));
        }
    }

    let packages_dir = Path::new("packages");
    if packages_dir.exists() {
        if let Ok(entries) = fs::read_dir(packages_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.join("parth.das").exists() {
                    if let Ok(cfg) = config::read_config(&p.join("parth.das")) {
                        packages.push((cfg.pkg_name.clone(), cfg.pkg_version.clone(), p.display().to_string()));
                    }
                }
            }
        }
    }

    if packages.is_empty() {
        println!("No packages found.");
    } else {
        println!("{:<30} {:<12} {}", "Package", "Version", "Path");
        println!("{}", "-".repeat(60));
        for (name, version, path) in &packages {
            println!("{:<30} {:<12} {}", name, version, path);
        }
    }
}

pub fn cmd_list() {
    println!("📦 Available packages:");
    let mut found = false;

    // 1. Global packages (~/.parth/packages/)
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

    // 2. Local packages (./packages/)
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

pub fn read_package_version_from_dir(pkg_dir: &Path) -> Option<String> {
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
    // Also remove the project binary
    let (name, _, _) = super::build::read_config_basic_for_build();
    let bin_path = build_dir.join(&name);
    if bin_path.exists() {
        let _ = fs::remove_file(&bin_path);
        println!("🗑️  Removed {}", bin_path.display());
    }
    println!("🧹 Cleaned build directory");
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

    // Read package name from parth.das
    let cfg = config::read_config(&source_path.join("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });

    if cfg.pkg_name.is_empty() || cfg.pkg_name == "project" {
        eprintln!("Error: package name must be set in [package] section");
        std::process::exit(1);
    }

    // Get version from parth.das
    let version = if cfg.pkg_version.is_empty() { "0.1.0" } else { &cfg.pkg_version };

    // Copy to ~/.parth/packages/<name>/
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let global_dir = PathBuf::from(home).join(".parth").join("packages").join(&cfg.pkg_name);

    // Remove existing if present
    if global_dir.exists() {
        let _ = fs::remove_dir_all(&global_dir);
    }

    // Copy the package
    if let Err(e) = registry::copy_dir_recursive(&source_path, &global_dir) {
        eprintln!("❌ Link failed: {}", e);
        std::process::exit(1);
    }

    println!("🔗 Linked: {} v{}", cfg.pkg_name, version);
    println!("   Path: {}", global_dir.display());
}

pub fn cmd_fmt(args: &[String]) {
    let root = find_ajeeb_root();
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "ajeeb-fmt", "--"]);
    cmd.args(args);
    let status = cmd.current_dir(&root).status().expect("Failed to run ajeeb-fmt");
    std::process::exit(status.code().unwrap_or(1));
}

pub fn cmd_lint(args: &[String]) {
    let target = if !args.is_empty() { args[0].clone() } else { "src/".to_string() };
    let path = Path::new(&target);
    let mut issues = 0;
    let mut files_checked = 0;

    fn lint_file(path: &Path, issues: &mut i32) {
        if let Ok(content) = fs::read_to_string(path) {
            for (i, line) in content.lines().enumerate() {
                let line_num = i + 1;
                if line.contains("== \"\"") || line.contains("!= \"\"") {
                    eprintln!("  {}:{}: use strEq() instead of == for string comparison", path.display(), line_num);
                    *issues += 1;
                }
                if line.trim().starts_with("// TODO") {
                    eprintln!("  {}:{}: TODO found", path.display(), line_num);
                    *issues += 1;
                }
                if line.len() > 120 {
                    eprintln!("  {}:{}: line exceeds 120 characters ({})", path.display(), line_num, line.len());
                    *issues += 1;
                }
                if line.ends_with(' ') || line.ends_with('\t') {
                    eprintln!("  {}:{}: trailing whitespace", path.display(), line_num);
                    *issues += 1;
                }
            }
        }
    }

    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map(|e| e == "ajb").unwrap_or(false) {
                    lint_file(&p, &mut issues);
                    files_checked += 1;
                }
            }
        }
    } else if path.exists() {
        lint_file(path, &mut issues);
        files_checked = 1;
    } else {
        eprintln!("Error: {} not found", target);
        std::process::exit(1);
    }

    println!("Linted {} files: {} issues found", files_checked, issues);
    if issues > 0 { std::process::exit(1); }
}

pub fn cmd_sanitize(args: &[String]) {
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found"); std::process::exit(1);
    }
    let cfg = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });
    let entry = if !args.is_empty() && args[0].ends_with(".ajb") { args[0].clone() }
    else { format!("src/{}.ajb", cfg.pkg_name) };

    println!("Running sanitizer checks on {}...", entry);
    println!("  Checking: memory safety, bounds, use-after-free, null deref");

    let build_out = Command::new("ajeebc")
        .args([&entry, "-o", "build/sanitize_check"])
        .output()
        .or_else(|_| Command::new("./target/debug/ajeebc").args([&entry, "-o", "build/sanitize_check"]).output());

    match build_out {
        Ok(o) if o.status.success() => {
            let run_out = Command::new("./build/sanitize_check")
                .env("AJEEB_SANITIZE", "1")
                .output();
            match run_out {
                Ok(o) if o.status.success() => {
                    println!("No memory safety issues detected!");
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    if !stderr.is_empty() { println!("  Runtime: {}", stderr.trim()); }
                }
                Ok(o) => {
                    eprintln!("Sanitizer found issues:");
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    if !stdout.is_empty() { eprintln!("  {}", stdout.trim()); }
                    if !stderr.is_empty() { eprintln!("  {}", stderr.trim()); }
                    std::process::exit(1);
                }
                Err(e) => { eprintln!("Cannot run binary: {}", e); std::process::exit(1); }
            }
        }
        Ok(o) => {
            eprintln!("Build failed:");
            eprintln!("{}", String::from_utf8_lossy(&o.stderr));
            std::process::exit(1);
        }
        Err(e) => { eprintln!("Cannot run ajeebc: {}", e); std::process::exit(1); }
    }
}

pub fn cmd_bench(args: &[String]) {
    let bench_dir = Path::new("benches");
    if !bench_dir.exists() {
        eprintln!("Error: no benches/ directory found. Create it and add benchmark files.");
        std::process::exit(1);
    }
    let filter = if !args.is_empty() { &args[0] } else { "" };
    let mut count = 0;
    let mut failed = 0;
    if let Ok(entries) = fs::read_dir(bench_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "ajb").unwrap_or(false) {
                let name = path.file_stem().unwrap().to_string_lossy();
                if !filter.is_empty() && !name.contains(filter) { continue; }
                print!("bench {} ... ", name);
                let start = Instant::now();
                let output = Command::new("ajeebc")
                    .arg(path.to_str().unwrap())
                    .output()
                    .or_else(|_| Command::new("./target/debug/ajeebc").arg(path.to_str().unwrap()).output());
                match output {
                    Ok(o) if o.status.success() => {
                        let elapsed = start.elapsed();
                        println!("ok ({:.3?})", elapsed);
                        count += 1;
                    }
                    Ok(o) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        println!("FAILED");
                        if !stderr.is_empty() { eprintln!("  {}", stderr.lines().next().unwrap_or("")); }
                        failed += 1;
                        count += 1;
                    }
                    Err(e) => {
                        println!("FAILED ({})", e);
                        failed += 1;
                        count += 1;
                    }
                }
            }
        }
    }
    println!("\n{} benchmarks: {} passed, {} failed", count, count - failed, failed);
    if failed > 0 { std::process::exit(1); }
}

pub fn cmd_package() {
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found"); std::process::exit(1);
    }
    let cfg = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });
    let pkg_name = &cfg.pkg_name;
    let pkg_version = &cfg.pkg_version;
    let tarball = format!("{}-{}.tar.gz", pkg_name, pkg_version);

    println!("Packaging {} v{}...", pkg_name, pkg_version);

    let mut files_to_pack: Vec<String> = vec!["parth.das".to_string()];
    let src_dir = Path::new("src");
    if src_dir.exists() {
        if let Ok(entries) = fs::read_dir(src_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map(|e| e == "ajb").unwrap_or(false) {
                    files_to_pack.push(p.to_string_lossy().to_string());
                }
            }
        }
    }

    let output = Command::new("tar")
        .args(["czf", &tarball])
        .args(&files_to_pack)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            println!("Created {}", tarball);
            if let Ok(meta) = fs::metadata(&tarball) {
                println!("  Size: {} bytes", meta.len());
            }
        }
        _ => {
            println!("Would package:");
            for f in &files_to_pack { println!("  {}", f); }
        }
    }
}

pub fn cmd_generate_lockfile() {
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found"); std::process::exit(1);
    }
    let cfg = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });
    if cfg.deps.is_empty() {
        println!("No dependencies. Lockfile not needed.");
        return;
    }
    match resolver::resolve_and_cache(&cfg.deps, Path::new("."), "") {
        Ok((resolved, _lock)) => {
            println!("Generated parth.lock with {} dependencies", resolved.len());
        }
        Err(e) => {
            eprintln!("Failed to generate lockfile: {}", e);
            std::process::exit(1);
        }
    }
}

pub fn cmd_vendor() {
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found"); std::process::exit(1);
    }
    let cfg = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });
    if cfg.deps.is_empty() {
        println!("No dependencies to vendor.");
        return;
    }

    let vendor_dir = Path::new("vendor");
    fs::create_dir_all(vendor_dir).expect("Cannot create vendor dir");

    println!("Vendoring dependencies into vendor/...");
    match resolver::resolve_and_cache(&cfg.deps, Path::new("."), "") {
        Ok((resolved, _lock)) => {
            for dep in &resolved {
                let dep_dir = vendor_dir.join(&dep.name);
                fs::create_dir_all(&dep_dir).ok();
                let cache_path: String = std::env::var("HOME").unwrap_or_default();
                let cache_base = Path::new(&cache_path).join(".parth/cache").join(&dep.name).join(&dep.version_req);
                if cache_base.exists() {
                    let _ = Command::new("cp").args(["-r", cache_base.to_str().unwrap(), dep_dir.to_str().unwrap()]).output();
                }
                println!("  {}@{}", dep.name, dep.version_req);
            }
            println!("Vendored {} dependencies", resolved.len());
        }
        Err(e) => { eprintln!("Vendor failed: {}", e); std::process::exit(1); }
    }
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
