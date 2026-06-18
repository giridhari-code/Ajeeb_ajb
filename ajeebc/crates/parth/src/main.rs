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

fn cmd_init() {
    if Path::new("parth.das").exists() {
        eprintln!("Error: parth.das already exists in current directory");
        std::process::exit(1);
    }
    fs::create_dir_all("src").expect("Cannot create src dir");
    fs::create_dir_all("build").expect("Cannot create build dir");

    let name = std::env::current_dir()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_else(|| "project".to_string());
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
         [profile.dev]\n\
         opt-level = \"0\"\n\
         debug = \"true\"\n\
         \n\
         [profile.release]\n\
         opt-level = \"3\"\n\
         debug = \"false\"\n\
         lto = \"true\"\n",
        name = name
    );
    fs::write("parth.das", das).expect("Cannot write parth.das");
    let main_ajb = "function main(): int {\n    println(\"Hello from Ajeeb!\");\n    return 0;\n}\n";
    fs::write("src/main.ajb", main_ajb).expect("Cannot write main.ajb");
    println!("✓ Initialized Ajeeb project in current directory");
    println!("📦 Name: {}", name);
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
    println!("✓ Created '{}' — Ajeeb project", raw_name);
    println!("");
    println!("  Next steps:");
    println!("  cd {}", raw_name);
    println!("  parth run");
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

fn cmd_build_file(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: parth build <file.ajb>");
        std::process::exit(1);
    }
    let file_path = &args[0];
    if !file_path.ends_with(".ajb") {
        eprintln!("Error: expected a .ajb file");
        std::process::exit(1);
    }

    // Resolve absolute paths to work regardless of where cargo runs
    let abs_file_path = std::fs::canonicalize(file_path).unwrap_or_else(|e| {
        eprintln!("Error: cannot resolve path '{}': {}", file_path, e);
        std::process::exit(1);
    });
    let project_dir = abs_file_path.parent().unwrap();
    let build_dir = project_dir.join("build");
    fs::create_dir_all(&build_dir).ok();
    let _output_c = build_dir.join("output.c");

    let root = find_ajeeb_root();
    let bin_name = build_dir.join(abs_file_path.file_stem().unwrap());
    let runtime_src = root.join("runtime/ajeeb_runtime.c");
    let native_binary = root.join("build/ajeebc");

    if native_binary.exists() {
        println!("⚡ Using ajeebc compiler");
        let output_ll = build_dir.join("output.ll");
        let status = Command::new(&native_binary)
            .args([&abs_file_path.to_string_lossy().to_string(), &output_ll.to_string_lossy().to_string(), "--skip-run"])
            .current_dir(&root)
            .status()
            .expect("Failed to run ajeebc");
        if !status.success() {
            eprintln!("❌ Compilation failed");
            std::process::exit(1);
        }
        let asm_file = build_dir.join("output.s");
        let llc_status = Command::new("llc")
            .args(["-O2", &output_ll.to_string_lossy(), "-o", &asm_file.to_string_lossy()])
            .status();
        match llc_status {
            Ok(s) if s.success() => {
                let gcc_status = Command::new("gcc")
                    .args([
                        &asm_file.to_string_lossy(),
                        &runtime_src.to_string_lossy(),
                        "-o", &bin_name.to_string_lossy(),
                        "-Wall", "-Wno-int-to-pointer-cast", "-Wno-pointer-to-int-cast",
                    ])
                    .status().expect("Failed to run gcc");
                if !gcc_status.success() {
                    eprintln!("❌ Native compilation failed");
                    std::process::exit(1);
                }
                println!("✓ Built: {}", bin_name.display());
            }
            Ok(_) => { eprintln!("❌ llc failed"); std::process::exit(1); }
            Err(e) => { eprintln!("❌ Could not run llc: {}", e); std::process::exit(1); }
        }
    } else {
        // Fall back to Rust interpreter + LLVM pipeline
        println!("🔧 Using Rust interpreter");
        let status = Command::new("cargo")
            .args(["run", "-p", "ajeeb-compiler", "--bin", "ajeeb_compiler",
                   "--", &abs_file_path.to_string_lossy().to_string(), "--skip-run"])
            .current_dir(&root)
            .status().expect("Failed to run compiler");
        if !status.success() {
            eprintln!("❌ Compilation failed");
            std::process::exit(1);
        }

        let llvm_ir = root.join("build/output.ll");
        let asm_file = build_dir.join("output.s");
        let llc_status = Command::new("llc")
            .args(["-O2", &llvm_ir.to_string_lossy(), "-o", &asm_file.to_string_lossy()])
            .status();
        match llc_status {
            Ok(s) if s.success() => {
                let gcc_status = Command::new("gcc")
                    .args([
                        &asm_file.to_string_lossy(),
                        &runtime_src.to_string_lossy(),
                        "-o", &bin_name.to_string_lossy(),
                        "-lm", "-ldl", "-Wl,--allow-multiple-definition",
                    ])
                    .status().expect("Failed to run gcc");
                if !gcc_status.success() {
                    eprintln!("❌ Native compilation failed");
                    std::process::exit(1);
                }
                println!("✓ Built: {}", bin_name.display());
            }
            Ok(_) => { eprintln!("❌ LLVM -> asm compilation failed"); std::process::exit(1); }
            Err(e) => { eprintln!("❌ Could not run llc: {}", e); std::process::exit(1); }
        }
    }
    println!("  Run with: parth run {}", file_path);
}

fn cmd_run_file(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: parth run <file.ajb> [args...]");
        std::process::exit(1);
    }
    let file_path = &args[0];
    if !file_path.ends_with(".ajb") {
        eprintln!("Error: expected a .ajb file");
        std::process::exit(1);
    }
    if !Path::new(file_path).exists() {
        eprintln!("Error: '{}' not found", file_path);
        std::process::exit(1);
    }

    let root = find_ajeeb_root();
    let parthi_bin = root.join("build/parthi");

    // Check if ParthI is available
    if parthi_bin.exists() {
        println!("🚀 Running with ParthI (MIR interpreter)...\n");
        let extra_args: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
        let mut cmd = Command::new(&parthi_bin);
        cmd.arg(file_path);
        cmd.args(&extra_args);
        let run_status = cmd.status().expect("Failed to run parthi");
        std::process::exit(run_status.code().unwrap_or(0));
    }

    // Check if --native flag is passed
    let is_native = args.len() > 1 && args[1] == "--native";

    if !is_native {
        // Default: run with ParthI interpreter
        println!("🚀 Running with ParthI...\n");
        let extra_args: Vec<&str> = if args.len() > 1 && args[1] == "--native" {
            args[2..].iter().map(|s| s.as_str()).collect()
        } else {
            args[1..].iter().map(|s| s.as_str()).collect()
        };
        let mut cmd = Command::new(&parthi_bin);
        cmd.arg(file_path);
        cmd.args(&extra_args);
        let run_status = cmd.status().expect("Failed to run parthi");
        std::process::exit(run_status.code().unwrap_or(0));
    }

    // With --native flag: compile to native binary then run
    let stem = Path::new(file_path)
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let bin_path = format!("build/{}", stem);

    fs::create_dir_all("build").ok();

    let native = root.join("build/ajeebc");
    let output_ll = "build/output.ll";

    if native.exists() {
        println!("⚡ Compiling with ajeebc...");
        let compile_status = Command::new(&native)
            .args([file_path, output_ll, "--skip-run"])
            .current_dir(&root)
            .status()
            .expect("Failed to run ajeebc");
        if !compile_status.success() {
            eprintln!("❌ Compilation failed");
            std::process::exit(1);
        }

        let runtime = root.join("runtime/ajeeb_runtime.c");
        let asm_file = "build/output.s";

        println!("🔧 Assembling with llc...");
        let llc_status = Command::new("llc")
            .args(["-O2", output_ll, "-o", asm_file])
            .status();
        match llc_status {
            Ok(s) if s.success() => {
                println!("🔨 Linking → {}", bin_path);
                let gcc_status = Command::new("gcc")
                    .args([
                        asm_file,
                        &runtime.to_string_lossy(),
                        "-o", &bin_path,
                        "-Wno-int-to-pointer-cast",
                        "-Wno-pointer-to-int-cast",
                    ])
                    .status()
                    .expect("Failed to run gcc");
                if !gcc_status.success() {
                    eprintln!("❌ GCC failed");
                    std::process::exit(1);
                }
            }
            Ok(_) => { eprintln!("❌ llc failed"); std::process::exit(1); }
            Err(e) => { eprintln!("❌ Could not run llc: {}", e); std::process::exit(1); }
        }
    } else {
        eprintln!("❌ build/ajeebc not found! Run 'bash install.sh' first.");
        std::process::exit(1);
    }

    if Path::new(&bin_path).exists() {
        println!("🚀 Running {}...\n", bin_path);
        let extra_args: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
        let run_status = Command::new(&bin_path)
            .args(&extra_args)
            .status()
            .expect("Failed to run binary");
        std::process::exit(run_status.code().unwrap_or(0));
    } else {
        eprintln!("❌ Binary not found at {}", bin_path);
        std::process::exit(1);
    }
}

fn cmd_build() {
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found"); std::process::exit(1);
    }
    let project_dir = std::env::current_dir().unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });
    let cfg = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error: {}", e); std::process::exit(1);
    });

    let (name, output_dir, runtime) = read_config_basic_for_build();
    let root = find_ajeeb_root();

    // Use absolute paths throughout
    let build_dir = project_dir.join(&output_dir);
    fs::create_dir_all(&build_dir).ok();
    let combined_path = build_dir.join("combined.ajb");
    let bin_path = build_dir.join(&name);
    let runtime_src = root.join(&runtime);

    // ── STEP 1 & 2: Resolve dependencies ──
    println!("📦 Resolving dependencies...");
    let mut all_ajb_files: Vec<PathBuf> = Vec::new();
    let mut all_runtime_c: Vec<PathBuf> = Vec::new();

    for dep in &cfg.deps {
        let dep_path = find_dep(&dep.name);
        match dep_path {
            Some(path) => {
                println!("  ✓ {} v{} → {}", dep.name, dep.version_req, path.display());
                // Collect .ajb source files
                collect_ajb_files(&path, &mut all_ajb_files);
                // Collect runtime .c files
                let rc = path.join("runtime").join(format!("{}_runtime.c", dep.name));
                if rc.exists() {
                    all_runtime_c.push(rc);
                }
            }
            None => {
                eprintln!("❌ Dep not found: {}", dep.name);
                eprintln!("   Run: parth link <path>");
                std::process::exit(1);
            }
        }
    }

    // ── STEP 3: Add project source ──
    let entry = project_dir.join("src/main.ajb");
    all_ajb_files.push(entry);

    // ── STEP 4: Combine all .ajb sources ──
    println!("🔨 Compiling: src/main.ajb{}", 
        if !cfg.deps.is_empty() {
            format!(" + {}", cfg.deps.iter().map(|d| d.name.as_str()).collect::<Vec<_>>().join(", "))
        } else {
            String::new()
        }
    );

    let combined = all_ajb_files.iter()
        .filter_map(|f| {
            let content = fs::read_to_string(f).ok()?;
            let stem = f.file_stem()?.to_string_lossy();
            Some(format!("\n// --- {} ---\n{}", stem, content))
        })
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(&combined_path, &combined).unwrap_or_else(|e| {
        eprintln!("Error writing combined source: {}", e); std::process::exit(1);
    });

    // ── STEP 5: Compile ──
    let native_binary = root.join("build/ajeebc");
    let combined_str = combined_path.to_string_lossy().to_string();

    if native_binary.exists() {
        println!("⚡ Using self-hosted compiler (ajeebc)");
        let output_ll = build_dir.join("output.ll");
        let output_s = build_dir.join("output.s");
        // Run ajeebc from root dir so it can find runtime/ajeeb_runtime.c
        let status = Command::new(&native_binary)
            .args([&combined_str, &output_ll.to_string_lossy().to_string(), "--skip-run"])
            .current_dir(&root)
            .status()
            .expect("Failed to run ajeebc");
        if !status.success() {
            eprintln!("❌ Self-hosted compilation failed");
            std::process::exit(1);
        }

        // ── STEP 6: llc → .s → gcc → binary
        println!("🔧 Assembling with llc...");
        let llc_status = Command::new("llc")
            .args(["-O2", &output_ll.to_string_lossy(), "-o", &output_s.to_string_lossy()])
            .status();
        match llc_status {
            Ok(s) if s.success() => {
                println!("🔗 Linking...");
                let mut gcc_args: Vec<String> = vec![
                    output_s.to_string_lossy().to_string(),
                    runtime_src.to_string_lossy().to_string(),
                ];
                for rc in &all_runtime_c {
                    gcc_args.push(rc.to_string_lossy().to_string());
                }
                gcc_args.extend([
                    "-o".to_string(), bin_path.to_string_lossy().to_string(),
                    "-Wall".to_string(), "-Wno-int-to-pointer-cast".to_string(), "-Wno-pointer-to-int-cast".to_string(),
                ]);
                let gcc_status = Command::new("gcc")
                    .args(&gcc_args)
                    .status().expect("Failed to run gcc");
                if !gcc_status.success() {
                    eprintln!("❌ Native compilation failed");
                    std::process::exit(1);
                }
                println!("✅ Built: {}", bin_path.display());
            }
            Ok(_) => { eprintln!("❌ llc failed"); std::process::exit(1); }
            Err(e) => { eprintln!("❌ Could not run llc: {}", e); std::process::exit(1); }
        }
    } else {
        // Fall back to Rust interpreter + LLVM pipeline
        println!("🔧 Using Rust interpreter");
        let status = Command::new("cargo")
            .args(["run", "-p", "ajeeb-compiler", "--bin", "ajeeb_compiler",
                   "--", &combined_str, "--skip-run"])
            .current_dir(&root)
            .status().expect("Failed to run compiler");
        if !status.success() { eprintln!("❌ Compilation failed"); std::process::exit(1); }

        let llvm_ir = root.join("build/output.ll");
        let asm_file = build_dir.join("output.s");
        let llc_status = Command::new("llc")
            .args(["-O2", &llvm_ir.to_string_lossy(), "-o", &asm_file.to_string_lossy()])
            .status();
        match llc_status {
            Ok(s) if s.success() => {
                // Link with runtime .c files
                let mut gcc_args: Vec<String> = vec![
                    asm_file.to_string_lossy().to_string(),
                    runtime_src.to_string_lossy().to_string(),
                ];
                for rc in &all_runtime_c {
                    gcc_args.push(rc.to_string_lossy().to_string());
                }
                gcc_args.extend([
                    "-o".to_string(), bin_path.to_string_lossy().to_string(),
                    "-lm".to_string(), "-ldl".to_string(), "-Wl,--allow-multiple-definition".to_string(),
                ]);

                let gcc_status = Command::new("gcc")
                    .args(&gcc_args)
                    .status().expect("Failed to run gcc");
                if !gcc_status.success() {
                    eprintln!("❌ Native compilation failed");
                    std::process::exit(1);
                }
                println!("✅ Built: {}", bin_path.display());
            }
            Ok(_) => { eprintln!("❌ LLVM -> asm compilation failed"); std::process::exit(1); }
            Err(e) => { eprintln!("❌ Could not run llc: {}", e); std::process::exit(1); }
        }
    }

    // Build workspace members
    for member in &cfg.workspace {
        let member_dir = project_dir.join(&member.path);
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
            .current_dir(&member_dir)
            .status()
            .expect("Failed to run parth build for workspace member");
        if !status.success() {
            eprintln!("❌ Workspace member '{}' build failed", member.path);
            std::process::exit(1);
        }
        println!("✓ Workspace member '{}' built successfully", member.path);
    }
}

/// Find a dependency in local locations
fn find_dep(name: &str) -> Option<PathBuf> {
    let search_paths = dep_search_paths();
    for base in &search_paths {
        let pkg_dir = base.join(name);
        if pkg_dir.exists() && pkg_dir.join("parth.das").exists() {
            return Some(pkg_dir);
        }
    }
    None
}

/// Get all search paths for dependencies
fn dep_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    
    // 1) ./packages/<name>/
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("packages"));
    }
    
    // 2) ~/.parth/packages/<name>/
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    paths.push(PathBuf::from(home).join(".parth").join("packages"));
    
    // 3) <ajeeb_root>/packages/<name>/
    let root = find_ajeeb_root();
    paths.push(root.join("packages"));
    
    paths
}

/// Collect .ajb source files from a package
fn collect_ajb_files(pkg_dir: &Path, files: &mut Vec<PathBuf>) {
    let src_dir = pkg_dir.join("src");
    if src_dir.exists() {
        if let Ok(entries) = fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().map(|e| e == "ajb").unwrap_or(false) {
                    files.push(p);
                }
            }
        }
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
    let root = find_ajeeb_root();
    let parthi_bin = root.join("build/parthi");
    let entry = "src/main.ajb";

    if !Path::new(entry).exists() {
        eprintln!("Error: src/main.ajb not found. 'parth init' karo pehle.");
        std::process::exit(1);
    }

    if parthi_bin.exists() {
        println!("🚀 Running with ParthI...\n");
        let status = Command::new(&parthi_bin)
            .arg(entry)
            .status()
            .expect("Failed to run parthi");
        std::process::exit(status.code().unwrap_or(1));
    }

    eprintln!("Error: build/parthi not found. 'bash install.sh' karo pehle.");
    std::process::exit(1);
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
    let ajeebc = root.join("build/ajeebc");
    let parthi = root.join("build/parthi");

    for entry in &entries {
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        print!("  {} ... ", name);
        std::io::stdout().flush().ok();

        // Use parthi for quick interpret, fall back to cargo run for full test
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

fn cmd_login(args: &[String]) {
    let registry_url = args.first().map(|s| s.as_str()).unwrap_or("https://registry.ajeeb.dev");
    match registry::login(registry_url) {
        Ok(info) => println!("✓ Logged in as '{}'", info.username),
        Err(e) => { eprintln!("❌ Login failed: {}", e); std::process::exit(1); }
    }
}

fn cmd_logout() {
    match registry::logout() {
        Ok(()) => {}
        Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
    }
}

fn cmd_whoami() {
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

fn cmd_doc() {
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

fn cmd_tree() {
    let lock = if Path::new("parth.lock").exists() {
        resolver::read_lock(Path::new("."))
    } else {
        eprintln!("No parth.lock found. Run `parth build` first.");
        std::process::exit(1);
    };
    resolver::print_tree(&lock);
}

fn cmd_why(args: &[String]) {
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

fn cmd_outdated() {
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

fn cmd_upgrade(args: &[String]) {
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

    // Try to find package locally first
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
            // Try to download (will also search local paths)
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

fn cmd_publish(_args: &[String]) {
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
    let (name, _, _) = read_config_basic_for_build();
    let bin_path = build_dir.join(&name);
    if bin_path.exists() {
        let _ = fs::remove_file(&bin_path);
        println!("🗑️  Removed {}", bin_path.display());
    }
    println!("🧹 Cleaned build directory");
}

fn cmd_link(args: &[String]) {
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

fn cmd_list() {
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

fn cmd_help() {
    println!("Ajeeb Package Manager — parth v0.1.0");
    println!();
    println!("USAGE:");
    println!("  parth <command> [arguments]");
    println!();
    println!("COMMANDS:");
    println!("  new <name>       Create a new Ajeeb project");
    println!("  init             Initialize project in current directory");
    println!("  add <pkg>[@v]    Add a dependency");
    println!("  remove <pkg>     Remove a dependency");
    println!("  update           Update all dependencies");
    println!("  tree             Show dependency tree");
    println!("  why <pkg>        Explain why a package is included");
    println!("  outdated         Check for outdated dependencies");
    println!("  upgrade [pkg]    Upgrade dependencies");
    println!("  build [file.ajb] Compile current project or single file");
    println!("  run [file.ajb]   Run with ParthI interpreter (fast!)");
    println!("                   Examples: parth run hello.ajb");
    println!("                             parth run (runs src/main.ajb)");
    println!("                             parth run file.ajb --native (compile + run)");
    println!("  test             Run all tests in tests/ directory");
    println!("  fmt [files..]    Format Ajeeb source files");
    println!("  doc              Generate documentation from /// comments");
    println!("  clean            Remove build artifacts");
    println!("  info             Show project info from parth.das");
    println!("  version          Show parth and project version");
    println!("  help             Show this help message");
    println!("  search <query>   Search packages");
    println!("  install <pkg>    Install a package");
    println!("  publish [url]    Publish the package");
    println!("  login [url]      Authenticate with a registry");
    println!("  logout           Remove stored credentials");
    println!("  whoami           Show current user");
    println!("  sign <pkg> <v>   Sign a package");
    println!("  verify <p> <v>   Verify package signature");
    println!("  keygen           Generate Ed25519 signing keypair");
    println!("  yank <pkg> <v>   Yank a package version");
    println!("  unyank <pkg> <v>  Un-yank a package version");
    println!("  audit            Security audit");
    println!("  cache <cmd>      Cache management");
    println!("  workspace <cmd>  Workspace management");
    println!("  link <path>      Link a local package to cache");
    println!("  list             Show all available packages");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        cmd_help();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "new" => cmd_new(&args[2..]),
        "init" => cmd_init(),
        "add" => cmd_add(&args[2..]),
        "remove" => cmd_remove(&args[2..]),
        "build" => {
            if args.len() > 2 && args[2].ends_with(".ajb") {
                cmd_build_file(&args[2..]);
            } else if args.len() > 2 {
                eprintln!("Usage: parth build [file.ajb]");
                std::process::exit(1);
            } else {
                cmd_build();
            }
        }
        "run" => {
            if args.len() > 2 {
                cmd_run_file(&args[2..]);
            } else {
                cmd_run();
            }
        }
        "test" => cmd_test(),
        "fmt" => cmd_fmt(&args[2..]),
        "doc" => cmd_doc(),
        "publish" => cmd_publish(&args[2..]),
        "update" => cmd_update(),
        "tree" => cmd_tree(),
        "why" => cmd_why(&args[2..]),
        "outdated" => cmd_outdated(),
        "upgrade" => cmd_upgrade(&args[2..]),
        "info" => cmd_info(),
        "search" => cmd_search(&args[2..]),
        "install" => cmd_install(&args[2..]),
        "login" => cmd_login(&args[2..]),
        "logout" => cmd_logout(),
        "whoami" => cmd_whoami(),
        "sign" => cmd_sign(&args[2..]),
        "verify" => cmd_verify(&args[2..]),
        "keygen" => cmd_keygen(),
        "yank" => cmd_yank(&args[2..]),
        "unyank" => cmd_unyank(&args[2..]),
        "audit" => cmd_audit(&args[2..]),
        "cache" => cmd_cache(&args[2..]),
        "workspace" => cmd_workspace(&args[2..]),
        "link" => cmd_link(&args[2..]),
        "list" => cmd_list(),
        "version" => cmd_version(),
        "clean" => cmd_clean(),
        "help" | "-h" | "--help" => cmd_help(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }
}
