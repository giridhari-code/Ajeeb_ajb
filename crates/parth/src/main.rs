use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

mod config;
mod registry;
mod resolver;
mod types;

use config::ProjectConfig;

fn find_ajeeb_root() -> PathBuf {
    if let Ok(manifest) = env::var("CARGO_MANIFEST_DIR") {
        let mut dir = PathBuf::from(manifest);
        loop {
            if dir.join("compiler").join("compiler.ajb").exists() { return dir; }
            if !dir.pop() { break; }
        }
    }
    let mut dir = env::current_dir().unwrap_or_default();
    loop {
        if dir.join("compiler").join("compiler.ajb").exists() { return dir; }
        if !dir.pop() { break; }
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            let mut d = parent.to_path_buf();
            loop {
                if d.join("compiler").join("compiler.ajb").exists() { return d; }
                if !d.pop() { break; }
            }
        }
    }
    PathBuf::from("..")
}

fn get_registry_url(cfg: &ProjectConfig) -> String {
    if !cfg.registry_url.is_empty() { return cfg.registry_url.clone(); }
    env::var("PARTH_REGISTRY").unwrap_or_else(|_| "local".to_string())
}

fn cmd_new(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: parth new <project-name>");
        std::process::exit(1);
    }
    let raw_name = &args[0];
    if !raw_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        eprintln!("Error: project name must contain only letters, numbers, '_', and '-'");
        std::process::exit(1);
    }
    let dir = PathBuf::from(raw_name);
    if dir.exists() {
        eprintln!("Error: directory '{}' already exists", raw_name);
        std::process::exit(1);
    }
    fs::create_dir_all(dir.join("src")).expect("Cannot create src dir");
    fs::create_dir_all(dir.join("build")).expect("Cannot create build dir");

    let das = format!(
        "[package]\n\
         name = \"{name}\"\n\
         version = \"0.1.0\"\n\
         author = \"\"\n\
         description = \"\"\n\
         registry = \"\"\n\
         \n\
         [dependencies]\n\
         \n\
         [features]\n\
         \n\
         [profile.dev]\n\
         opt-level = \"0\"\n\
         debug = \"true\"\n\
         \n\
         [profile.release]\n\
         opt-level = \"3\"\n\
         debug = \"false\"\n\
         lto = \"true\"\n\
         \n\
         [runtime]\n\
         max_threads = \"8\"\n\
         log_level = \"info\"\n\
         \n\
         [compiler]\n\
         target = \"native\"\n\
         output = \"build/\"\n\
         runtime = \"runtime/ajeeb_runtime.c\"\n",
        name = raw_name
    );
    fs::write(dir.join("parth.das"), das).expect("Cannot write parth.das");
    let main_ajb = "function main(): int {\n    println(\"Hello from Ajeeb!\");\n    return 0;\n}\n";
    fs::write(dir.join("src").join("main.ajb"), main_ajb).expect("Cannot write main.ajb");
    println!("✓ Created Ajeeb project '{}'", raw_name);
}

fn cmd_add(args: &[String]) {
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
    let registry_url = get_registry_url(&cfg);

    let mut deps = cfg.deps.clone();
    if deps.iter().any(|d| d.name == pkg_name) {
        println!("ℹ️  '{}' is already a dependency", pkg_name);
        return;
    }

    let _ = registry::download_package(&pkg_name, &"latest".to_string(), &registry_url);

    let new_dep = types::PkgDep { name: pkg_name.clone(), version_req };
    let mut all_deps = deps.clone();
    all_deps.push(new_dep);

    match resolver::resolve_and_cache(&all_deps, Path::new("."), &registry_url) {
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

fn cmd_remove(args: &[String]) {
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

fn collect_library_sources(deps: &[types::PkgDep]) -> (String, Vec<String>) {
    let mut combined = String::new();
    let mut runtime_c_files: Vec<String> = Vec::new();

    for dep in deps {
        let lib_dir = Path::new("libs").join(&dep.name);
        if !lib_dir.exists() {
            eprintln!("⚠️  Library '{}' not found in libs/", dep.name);
            continue;
        }

        let src_dir = lib_dir.join("src");
        if src_dir.exists() {
            if let Ok(entries) = fs::read_dir(&src_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map(|e| e == "ajb").unwrap_or(false) {
                        if let Ok(content) = fs::read_to_string(&p) {
                            combined.push_str("\n// --- ");
                            combined.push_str(dep.name.as_str());
                            combined.push_str(": ");
                            combined.push_str(p.file_name().unwrap().to_string_lossy().as_ref());
                            combined.push_str(" ---\n");
                            combined.push_str(&content);
                            combined.push('\n');
                        }
                    }
                }
            }
        }

        let runtime_dir = lib_dir.join("runtime");
        if runtime_dir.exists() {
            if let Ok(entries) = fs::read_dir(&runtime_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map(|e| e == "c").unwrap_or(false) {
                        runtime_c_files.push(p.to_string_lossy().to_string());
                    }
                }
            }
        }
    }

    (combined, runtime_c_files)
}

fn cmd_build() {
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found"); std::process::exit(1);
    }
    let cfg = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });
    let registry_url = get_registry_url(&cfg);

    if !cfg.deps.is_empty() {
        match resolver::resolve_and_cache(&cfg.deps, Path::new("."), &registry_url) {
            Ok((_resolved, lock)) => {
                match resolver::compilation_order(&lock) {
                    Ok(order) => { if !order.is_empty() { println!("📦 Dependencies: {}", order.join(", ")); } }
                    Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
                }
            }
            Err(e) => { eprintln!("❌ Dependency resolution failed: {}", e); std::process::exit(1); }
        }
    }

    let profile = cfg.profiles.first().cloned().unwrap_or_default();
    let opt_flag = if profile.opt_level > 0 { format!("-O{}", profile.opt_level) } else { String::new() };

    let (name, output_dir, runtime) = read_config_basic_for_build();
    let root = find_ajeeb_root();
    let runtime_src = root.join(&runtime);
    let runtime_src_str = runtime_src.to_string_lossy().to_string();

    let (lib_ajb_sources, lib_runtime_c) = collect_library_sources(&cfg.deps);

    let entry = "src/main.ajb";
    let mut user_src = fs::read_to_string(entry).unwrap_or_default();
    user_src.push('\n');
    user_src.push_str(&lib_ajb_sources);

    let combined_path = "build/combined.ajb";
    fs::write(combined_path, &user_src).unwrap_or_else(|e| {
        eprintln!("Error writing combined source: {}", e); std::process::exit(1);
    });

    let status = Command::new("cargo")
        .args(["run", "-p", "ajeeb-compiler", "--bin", "ajeeb_compiler",
               "--", combined_path, "build/output.c"])
        .current_dir(&root)
        .status().expect("Failed to run compiler");
    if !status.success() { eprintln!("❌ Compilation failed"); std::process::exit(1); }

    let out_path = format!("{}output.c", output_dir);

    let bin_path = format!("{}{}", output_dir, name);
    let mut gcc_args: Vec<String> = vec![out_path.clone(), runtime_src_str.clone(), "-o".to_string(), bin_path.clone()];
    for dep in &cfg.deps {
        let lib_dir = Path::new("libs").join(&dep.name);
        let runtime_dir = lib_dir.join("runtime");
        if runtime_dir.exists() {
            let h_path = runtime_dir.join(format!("{}_runtime.h", dep.name));
            if h_path.exists() {
                gcc_args.push("-include".to_string());
                gcc_args.push(h_path.to_string_lossy().into_owned());
            }
        }
    }
    gcc_args.extend(lib_runtime_c.iter().cloned());
    if !opt_flag.is_empty() { gcc_args.push(opt_flag.clone()); }
    gcc_args.extend_from_slice(&["-Wall".to_string(), "-Wno-int-to-pointer-cast".to_string(), "-Wno-pointer-to-int-cast".to_string(), "-Wno-int-conversion".to_string()]);

    let status = Command::new("gcc").args(&gcc_args).status().expect("Failed to run gcc");
    if !status.success() { eprintln!("❌ GCC compilation failed"); std::process::exit(1); }
    println!("✓ Build: {} (opt-level: {})", bin_path, profile.opt_level);

    // Build workspace members
    for member in &cfg.workspace {
        let member_dir = Path::new(&member.path);
        if !member_dir.exists() {
            eprintln!("⚠️  Workspace member '{}' not found, skipping", member.path);
            continue;
        }
        if !member_dir.join("parth.das").exists() {
            eprintln!("⚠️  Workspace member '{}' has no parth.das, skipping", member.path);
            continue;
        }
        println!("\n📦 Building workspace member: {}", member.path);
        let status = Command::new("cargo")
            .args(["run", "-p", "parth", "--", "build"])
            .current_dir(member_dir)
            .status()
            .expect("Failed to run parth build for workspace member");
        if !status.success() {
            eprintln!("❌ Workspace member '{}' build failed", member.path);
            std::process::exit(1);
        }
        println!("✓ Workspace member '{}' built successfully", member.path);
    }
}

fn read_config_basic_for_build() -> (String, String, String) {
    let content = fs::read_to_string("parth.das").unwrap_or_default();
    let mut name = String::from("project");
    let mut output = String::from("build/");
    let mut runtime = String::from("runtime/ajeeb_runtime.c");
    let mut current_section = String::new();
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            current_section = t[1..t.len() - 1].trim().to_string();
        } else if let Some(eq) = t.find('=') {
            let key = t[..eq].trim();
            let val = t[eq + 1..].trim().trim_matches('"');
            if current_section == "package" && key == "name" { name = val.to_string(); }
            else if current_section == "compiler" && key == "output" { output = val.to_string(); }
            else if current_section == "compiler" && key == "runtime" { runtime = val.to_string(); }
        }
    }
    (name, output, runtime)
}

fn cmd_run() {
    cmd_build();
    let (name, output_dir, _) = read_config_basic_for_build();
    let bin_path = format!("{}{}", output_dir, name);
    let status = Command::new(&bin_path).status().expect("Failed to run binary");
    std::process::exit(status.code().unwrap_or(1));
}

fn cmd_test() {
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
            .status()
            .unwrap_or_default();

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

fn cmd_info() {
    let content = fs::read_to_string("parth.das").unwrap_or_default();
    println!("📦 parth.das:\n{}", content);
    if Path::new("parth.lock").exists() {
        let lock = fs::read_to_string("parth.lock").unwrap_or_default();
        println!("🔒 parth.lock:\n{}", lock);
    }
}

fn cmd_search(args: &[String]) {
    let query = args.first().map(|s| s.as_str()).unwrap_or("");

    let cfg = if Path::new("parth.das").exists() {
        config::read_config(Path::new("parth.das")).unwrap_or_default()
    } else {
        ProjectConfig::default()
    };
    let registry_url = get_registry_url(&cfg);

    let results = registry::search_packages(query, &registry_url);
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

fn cmd_yank(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: parth yank <package> <version>");
        std::process::exit(1);
    }
    match registry::yank_package(&args[0], &args[1]) {
        Ok(()) => println!("✓ Yanked '{}@{}'", args[0], args[1]),
        Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
    }
}

fn cmd_unyank(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: parth unyank <package> <version>");
        std::process::exit(1);
    }
    match registry::unyank_package(&args[0], &args[1]) {
        Ok(()) => println!("✓ Un-yanked '{}@{}'", args[0], args[1]),
        Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
    }
}

fn cmd_keygen() {
    match registry::generate_keypair() {
        Ok((_, pub_hex)) => {
            println!("🔑 Ed25519 keypair generated");
            println!("📁 Keys stored in: {}", registry::keys_dir().display());
            println!("🔓 Public key: {}...", &pub_hex[..16]);
        }
        Err(e) => { eprintln!("❌ Key generation failed: {}", e); std::process::exit(1); }
    }
}

fn cmd_update() {
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
    let registry_url = get_registry_url(&cfg);

    // Delete existing lock to force re-resolution
    let lock_path = Path::new("parth.lock");
    if lock_path.exists() {
        let _ = fs::remove_file(lock_path);
    }

    match resolver::resolve_and_cache(&cfg.deps, Path::new("."), &registry_url) {
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

fn cmd_install(args: &[String]) {
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

    let cfg = if Path::new("parth.das").exists() {
        config::read_config(Path::new("parth.das")).unwrap_or_default()
    } else {
        ProjectConfig::default()
    };
    let registry_url = get_registry_url(&cfg);

    match registry::download_package(&name, &version, &registry_url) {
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

fn cmd_publish(args: &[String]) {
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

    let registry_arg = args.first().map(|s| s.as_str()).unwrap_or(&cfg.registry_url);
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

    if !registry_arg.is_empty() && registry_arg != "local" {
        println!("ℹ️  Published locally. To push: rsync -avz ~/.parth/ user@host:~/.parth/");
    }
}

fn cmd_sign(args: &[String]) {
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

fn cmd_verify(args: &[String]) {
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

fn cmd_audit(_args: &[String]) {
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

fn cmd_cache(args: &[String]) {
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
        _ => {
            eprintln!("Usage: parth cache <info|clear|prune>");
            std::process::exit(1);
        }
    }
}

fn cmd_workspace(args: &[String]) {
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

fn cmd_fmt(args: &[String]) {
    let root = find_ajeeb_root();
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "ajeeb-fmt", "--"]);
    cmd.args(args);
    let status = cmd.current_dir(&root).status().expect("Failed to run ajeeb-fmt");
    std::process::exit(status.code().unwrap_or(1));
}

fn cmd_version() {
    println!("parth 0.1.0 — Ajeeb Package Manager");
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

fn cmd_clean() {
    let patterns = ["build/output.c", "build/output2.c", "build/*.o"];
    for pattern in &patterns {
        let path = Path::new(pattern);
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }
    if let Ok(entries) = fs::read_dir("build") {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "o").unwrap_or(false) {
                let _ = fs::remove_file(&p);
            }
        }
    }
    println!("🧹 Cleaned build directory");
}

fn cmd_help() {
    println!("Ajeeb Package Manager — parth v0.1.0");
    println!();
    println!("USAGE:");
    println!("  parth <command> [arguments]");
    println!();
    println!("COMMANDS:");
    println!("  new <name>       Create a new Ajeeb project");
    println!("  add <pkg>[@v]    Add a dependency");
    println!("  remove <pkg>     Remove a dependency");
    println!("  update           Update all dependencies");
    println!("  build            Compile current project");
    println!("  run              Build and run current project");
    println!("  test             Run all tests in tests/ directory");
    println!("  fmt [files..]    Format Ajeeb source files");
    println!("  clean            Remove build artifacts");
    println!("  info             Show project info from parth.das");
    println!("  version          Show parth and project version");
    println!("  help             Show this help message");
    println!("  search <query>   Search packages");
    println!("  install <pkg>    Install a package");
    println!("  publish [url]    Publish the package");
    println!("  sign <pkg> <v>   Sign a package");
    println!("  verify <p> <v>   Verify package signature");
    println!("  keygen           Generate Ed25519 signing keypair");
    println!("  yank <pkg> <v>   Yank a package version");
    println!("  unyank <pkg> <v>  Un-yank a package version");
    println!("  audit            Security audit");
    println!("  cache <cmd>      Cache management");
    println!("  workspace <cmd>  Workspace management");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        cmd_help();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "new" => cmd_new(&args[2..]),
        "add" => cmd_add(&args[2..]),
        "remove" => cmd_remove(&args[2..]),
        "build" => cmd_build(),
        "run" => cmd_run(),
        "test" => cmd_test(),
        "fmt" => cmd_fmt(&args[2..]),
        "publish" => cmd_publish(&args[2..]),
        "update" => cmd_update(),
        "info" => cmd_info(),
        "search" => cmd_search(&args[2..]),
        "install" => cmd_install(&args[2..]),
        "sign" => cmd_sign(&args[2..]),
        "verify" => cmd_verify(&args[2..]),
        "keygen" => cmd_keygen(),
        "yank" => cmd_yank(&args[2..]),
        "unyank" => cmd_unyank(&args[2..]),
        "audit" => cmd_audit(&args[2..]),
        "cache" => cmd_cache(&args[2..]),
        "workspace" => cmd_workspace(&args[2..]),
        "version" => cmd_version(),
        "clean" => cmd_clean(),
        "help" | "-h" | "--help" => cmd_help(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }
}
