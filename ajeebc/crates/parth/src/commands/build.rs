use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config;
use crate::find_ajeeb_root;
use crate::resolver;
use crate::types;

pub fn cmd_build_file(args: &[String]) {
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

pub fn cmd_run_file(args: &[String]) {
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

pub fn cmd_build() {
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
    let entry_path = if cfg.entry.is_empty() {
        project_dir.join("src/main.ajb")
    } else {
        project_dir.join(&cfg.entry)
    };
    let entry_name = entry_path.file_name().unwrap_or_default().to_string_lossy().to_string();
    all_ajb_files.push(entry_path);

    // ── STEP 4: Combine all .ajb sources ──
    println!("🔨 Compiling: {}{}", entry_name,
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
    let native_binary = root.join("build/ajeeb_compiler");
    let native_binary = if native_binary.exists() { native_binary } else { root.join("build/ajeebc") };
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
pub fn find_dep(name: &str) -> Option<PathBuf> {
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
pub fn dep_search_paths() -> Vec<PathBuf> {
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
pub fn collect_ajb_files(pkg_dir: &Path, files: &mut Vec<PathBuf>) {
    // Check src/ subdirectory first
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
    // Also check root level .ajb files (for std packages without src/ dir)
    if let Ok(entries) = fs::read_dir(pkg_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() && p.extension().map(|e| e == "ajb").unwrap_or(false) {
                files.push(p);
            }
        }
    }
}

pub fn read_config_basic_for_build() -> (String, String, String) {
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

pub fn cmd_run() {
    let root = find_ajeeb_root();
    let parthi_bin = root.join("build/parthi");
    let cfg = config::read_config(Path::new("parth.das")).unwrap_or_default();
    let entry = if cfg.entry.is_empty() {
        String::from("src/main.ajb")
    } else {
        cfg.entry
    };

    if !Path::new(&entry).exists() {
        eprintln!("Error: {} not found. 'parth init' karo pehle.", entry);
        std::process::exit(1);
    }

    if parthi_bin.exists() {
        println!("🚀 Running with ParthI...\n");
        let status = Command::new(&parthi_bin)
            .arg(&entry)
            .status()
            .expect("Failed to run parthi");
        std::process::exit(status.code().unwrap_or(1));
    }

    eprintln!("Error: build/parthi not found. 'bash install.sh' karo pehle.");
    std::process::exit(1);
}

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
