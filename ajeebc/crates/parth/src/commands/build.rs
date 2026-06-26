use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config;

/// Find a binary by name using the proper search order:
/// 1. Bundled beside parth executable
/// 2. ~/.ajeeb/bin/<name>
/// 3. PATH lookup via `which`
/// Returns None if not found.
fn find_installed_bin(name: &str) -> Option<PathBuf> {
    // 1. Bundled beside parth executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let bundled = exe_dir.join(name);
            if bundled.exists() {
                return Some(bundled);
            }
        }
    }

    // 2. ~/.ajeeb/bin/<name>
    if let Ok(home) = std::env::var("HOME") {
        let home_bin = PathBuf::from(home).join(".ajeeb/bin").join(name);
        if home_bin.exists() {
            return Some(home_bin);
        }
    }

    // 3. PATH lookup
    if let Ok(output) = Command::new("which").arg(name).output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                let p = PathBuf::from(&path);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }

    None
}

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

    let abs_file_path = std::fs::canonicalize(file_path).unwrap_or_else(|e| {
        eprintln!("Error: cannot resolve path '{}': {}", file_path, e);
        std::process::exit(1);
    });
    let project_dir = abs_file_path.parent().unwrap();
    let build_dir = project_dir.join("build");
    fs::create_dir_all(&build_dir).ok();
    let bin_name = build_dir.join(abs_file_path.file_stem().unwrap());

    let compiler_bin = find_installed_bin("ajeebc")
        .or_else(|| find_installed_bin("ajeeb_compiler"));
    let runtime_src = find_installed_runtime();

    match compiler_bin {
        Some(native_binary) => {
            println!("⚡ Using ajeebc compiler");
            let output_ll = build_dir.join("output.ll");
            let status = Command::new(&native_binary)
                .args([&abs_file_path.to_string_lossy(), &output_ll.to_string_lossy(), "--emit-llvm-only"])
                .current_dir(&project_dir)
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
                    let gcc_args = vec![
                        asm_file.to_string_lossy().to_string(),
                        runtime_src.to_string_lossy().to_string(),
                        "-o".to_string(), bin_name.to_string_lossy().to_string(),
                        "-no-pie".to_string(), "-lm".to_string(), "-ldl".to_string(),
                        "-Wno-int-to-pointer-cast".to_string(), "-Wno-pointer-to-int-cast".to_string(),
                    ];
                    let gcc_status = Command::new("gcc").args(&gcc_args).status().expect("Failed to run gcc");
                    if !gcc_status.success() {
                        eprintln!("❌ Native compilation failed");
                        std::process::exit(1);
                    }
                    println!("✓ Built: {}", bin_name.display());
                }
                Ok(_) => { eprintln!("❌ llc failed"); std::process::exit(1); }
                Err(e) => { eprintln!("❌ Could not run llc: {}", e); std::process::exit(1); }
            }
        }
        None => {
            eprintln!("❌ ajeebc compiler not found.");
            eprintln!("   Searched:");
            eprintln!("     - bundled beside parth");
            eprintln!("     - ~/.ajeeb/bin/ajeebc");
            eprintln!("     - PATH");
            eprintln!();
            eprintln!("   Install ajeebc first:");
            eprintln!("     curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash");
            std::process::exit(1);
        }
    }
    println!("  Run with: parth run {}", file_path);
}

/// Find the runtime C file by checking common locations
fn find_installed_runtime() -> PathBuf {
    // Check ~/.ajeeb/bin/ajeeb_runtime.c
    if let Ok(home) = std::env::var("HOME") {
        let home_runtime = PathBuf::from(home).join(".ajeeb/bin/ajeeb_runtime.c");
        if home_runtime.exists() {
            return home_runtime;
        }
    }
    // Check bundled beside parth
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let bundled = exe_dir.join("ajeeb_runtime.c");
            if bundled.exists() {
                return bundled;
            }
        }
    }
    PathBuf::from("runtime/ajeeb_runtime.c")
}

pub fn cmd_run_file(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: parth run <file.ajb> [--native]");
        std::process::exit(1);
    }
    let file_path = &args[0];
    if !file_path.ends_with(".ajb") {
        eprintln!("Error: expected a .ajb file"); std::process::exit(1);
    }
    if !Path::new(file_path).exists() {
        eprintln!("Error: '{}' not found", file_path); std::process::exit(1);
    }
    let is_native = args.len() > 1 && args[1] == "--native";

    if is_native {
        let compiler_bin = find_installed_bin("ajeebc")
            .or_else(|| find_installed_bin("ajeeb_compiler"))
            .unwrap_or_else(|| {
                eprintln!("❌ No compiler found. Install ajeebc first.");
                eprintln!("   curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash");
                std::process::exit(1);
            });
        let stem = Path::new(file_path).file_stem().unwrap().to_string_lossy().to_string();
        let bin_path = PathBuf::from("build").join(&*stem);
        fs::create_dir_all("build").ok();
        let output_ll = "build/output.ll";
        let output_s = "build/output.s";

        println!("⚡ Compiling...");
        let status = Command::new(&compiler_bin).args([file_path, output_ll, "--emit-llvm-only"]).status();
        if !status.map(|s| s.success()).unwrap_or(false) { eprintln!("❌ Compile failed"); std::process::exit(1); }

        println!("🔧 Assembling with llc...");
        let llc_status = Command::new("llc").args(["-O2", output_ll, "-o", output_s]).status();
        if !llc_status.map(|s| s.success()).unwrap_or(false) { eprintln!("❌ llc failed"); std::process::exit(1); }

        let runtime_src = find_installed_runtime();
        println!("🔗 Linking...");
        let gcc_status = Command::new("gcc").args(["-no-pie", output_s, &runtime_src.to_string_lossy(), "-o", &bin_path.to_string_lossy(), "-lm", "-ldl", "-Wno-int-to-pointer-cast", "-Wno-pointer-to-int-cast"]).status();
        if !gcc_status.map(|s| s.success()).unwrap_or(false) { eprintln!("❌ Link failed"); std::process::exit(1); }

        println!("🚀 Running...\n");
        let run_status = Command::new(&bin_path).status().expect("run failed");
        std::process::exit(run_status.code().unwrap_or(0));
    }

    // interpreter mode: try parthi first, then ajeebc --interpret
    if let Some(p) = find_installed_bin("parthi") {
        println!("🚀 Running with ParthI...\n");
        let s = Command::new(&p).arg(file_path).status().expect("parthi failed");
        std::process::exit(s.code().unwrap_or(0));
    }
    if let Some(p) = find_installed_bin("ajeebc").or_else(|| find_installed_bin("ajeeb_compiler")) {
        println!("🚀 Running with ajeebc --interpret...\n");
        let s = Command::new(&p).arg("--interpret").arg(file_path).status().expect("ajeebc failed");
        std::process::exit(s.code().unwrap_or(0));
    }

    eprintln!("❌ No interpreter found.");
    eprintln!("   Install ajeebc or parthi:");
    eprintln!("   curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash");
    std::process::exit(1);
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

    let (name, output_dir, _runtime_rel) = read_config_basic_for_build();
    let build_dir = project_dir.join(&output_dir);
    fs::create_dir_all(&build_dir).ok();
    let combined_path = build_dir.join("combined.ajb");
    let bin_path = build_dir.join(&name);

    let compiler_bin = find_installed_bin("ajeebc")
        .or_else(|| find_installed_bin("ajeeb_compiler"));
    let runtime_src = find_installed_runtime();

    // ── STEP 1 & 2: Resolve dependencies ──
    println!("📦 Resolving dependencies...");
    let mut all_ajb_files: Vec<PathBuf> = Vec::new();
    let mut all_runtime_c: Vec<PathBuf> = Vec::new();

    for dep in &cfg.deps {
        let dep_path = find_dep(&dep.name);
        match dep_path {
            Some(path) => {
                println!("  ✓ {} v{} → {}", dep.name, dep.version_req, path.display());
                collect_ajb_files(&path, &mut all_ajb_files);
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
    let combined_str = combined_path.to_string_lossy().to_string();

    match compiler_bin {
        Some(compiler) => {
            println!("⚡ Using ajeebc compiler");
            let output_ll = build_dir.join("output.ll");
            let output_s = build_dir.join("output.s");
            let output_ll_str = output_ll.to_string_lossy().to_string();
            let status = Command::new(&compiler)
                .args([&combined_str, &output_ll_str, "--emit-llvm-only"])
                .current_dir(&project_dir)
                .status()
                .expect("Failed to run ajeebc");
            if !status.success() {
                eprintln!("❌ Compilation failed");
                std::process::exit(1);
            }

            println!("🔧 Assembling with llc...");
            let llc_status = Command::new("llc")
                .args(["-O2", &output_ll_str, "-o", &output_s.to_string_lossy()])
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
                        "-no-pie".to_string(), "-lm".to_string(), "-ldl".to_string(),
                        "-Wno-int-to-pointer-cast".to_string(), "-Wno-pointer-to-int-cast".to_string(),
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
        }
        None => {
            eprintln!("❌ ajeebc compiler not found.");
            eprintln!("   Searched:");
            eprintln!("     - bundled beside parth");
            eprintln!("     - ~/.ajeeb/bin/ajeebc");
            eprintln!("     - PATH");
            eprintln!();
            eprintln!("   Install ajeebc first:");
            eprintln!("     curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash");
            std::process::exit(1);
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
        let status = Command::new("parth")
            .args(["build"])
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

    // 2) ~/.ajeeb/packages/<name>/
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(home).join(".ajeeb").join("packages"));
    }

    // 3) ~/.parth/packages/<name>/
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(home).join(".parth").join("packages"));
    }

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

    // Try parthi first
    if let Some(p) = find_installed_bin("parthi") {
        println!("🚀 Running with ParthI...\n");
        let status = Command::new(&p).arg(&entry).status().expect("Failed to run parthi");
        std::process::exit(status.code().unwrap_or(1));
    }

    // Fall back to ajeebc --interpret
    if let Some(p) = find_installed_bin("ajeebc").or_else(|| find_installed_bin("ajeeb_compiler")) {
        println!("🚀 Running with ajeebc --interpret...\n");
        let status = Command::new(&p).arg("--interpret").arg(&entry).status().expect("Failed to run ajeebc");
        std::process::exit(status.code().unwrap_or(1));
    }

    eprintln!("❌ No interpreter found.");
    eprintln!("   Install ajeebc or parthi:");
    eprintln!("   curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash");
    std::process::exit(1);
}

pub fn cmd_test() {
    let test_dir = Path::new("tests");
    if !test_dir.exists() {
        println!("ℹ️  No tests/ directory found.");
        println!("   Create tests/ directory with .ajb test files, then run 'parth test' again.");
        std::process::exit(0);
    }
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut entries: Vec<_> = fs::read_dir(test_dir)
        .unwrap_or_else(|e| { eprintln!("Error: {}", e); std::process::exit(1); })
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ex| ex == "ajb").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        println!("ℹ️  No .ajb test files in tests/ directory.");
        std::process::exit(0);
    }

    let compiler_bin = find_installed_bin("ajeebc")
        .or_else(|| find_installed_bin("ajeeb_compiler"))
        .unwrap_or_else(|| {
            eprintln!("❌ No compiler binary found.");
            eprintln!("   Install ajeebc first:");
            eprintln!("   curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash");
            std::process::exit(1);
        });

    for entry in &entries {
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        print!("  {} ... ", name);
        std::io::stdout().flush().ok();

        let status = Command::new(&compiler_bin)
            .args(["--interpret", &path.to_string_lossy()])
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

pub fn cmd_bootstrap() {
    let gen0 = find_installed_bin("ajeebc")
        .or_else(|| find_installed_bin("ajeeb_compiler"))
        .unwrap_or_else(|| {
            eprintln!("❌ No compiler binary found.");
            eprintln!("   Install ajeebc first:");
            eprintln!("   curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash");
            std::process::exit(1);
        });

    let runtime_src = find_installed_runtime();
    let compiler_ajb = PathBuf::from("compiler/compiler.ajb");
    if !compiler_ajb.exists() {
        // Try finding from ajeeb_root or current dir
        eprintln!("❌ compiler/compiler.ajb not found in current directory.");
        eprintln!("   Run this command from the ajeeb_compiler repository root.");
        std::process::exit(1);
    }

    let is_rust = gen0.file_name().unwrap().to_string_lossy().contains("ajeeb_compiler");
    let gen0_label = if is_rust { "Rust" } else { "ajeebc" };

    println!("═══════════════════════════════════════════");
    println!("🔄 Ajeeb Self-Hosting Bootstrap");
    println!("═══════════════════════════════════════════");
    println!();

    // Step 1: Gen0 → Gen1 (native via LLVM)
    println!("[1/4] Gen0 ({}) compiles compiler.ajb → Gen1 (native)", gen0_label);
    let compiler_bin = PathBuf::from("build/compiler");
    let compiler_ajb_str = compiler_ajb.to_string_lossy().to_string();
    let status = Command::new(&gen0)
        .args([&compiler_ajb_str, "--skip-run"])
        .status()
        .expect("Failed to run Gen0");
    if !status.success() {
        eprintln!("❌ Gen0 compilation failed");
        std::process::exit(1);
    }
    if !compiler_bin.exists() {
        eprintln!("❌ Gen1 binary not found at build/compiler");
        std::process::exit(1);
    }
    println!("  ✓ Gen1: {} bytes", fs::metadata(&compiler_bin).map(|m| m.len()).unwrap_or(0));
    println!();

    // Step 2: Gen1 runs on compiler.ajb → C code
    println!("[2/4] Gen1 runs on compiler.ajb → C code");
    let output_c = PathBuf::from("build/output_bootstrap.c");
    let status = Command::new(&compiler_bin)
        .args([&compiler_ajb_str])
        .status()
        .expect("Failed to run Gen1");
    if !status.success() {
        eprintln!("❌ Gen1 C codegen failed");
        std::process::exit(1);
    }
    let gen1_c = PathBuf::from("build/output.c");
    if gen1_c.exists() {
        fs::copy(&gen1_c, &output_c).ok();
    }
    println!("  ✓ C output: {} lines", fs::read_to_string(&output_c).map(|s| s.lines().count()).unwrap_or(0));
    println!();

    // Step 3: Compile Gen1's C output → Gen2
    println!("[3/4] Compile Gen1's C output → Gen2");
    let gen2_bin = PathBuf::from("build/compiler_gen2");
    let status = Command::new("gcc")
        .args([
            &output_c.to_string_lossy(),
            &runtime_src.to_string_lossy(),
            "-o", &gen2_bin.to_string_lossy(),
            "-Wno-int-to-pointer-cast",
            "-Wno-pointer-to-int-cast",
            "-ldl", "-lm",
        ])
        .status()
        .expect("Failed to run gcc");
    if !status.success() {
        eprintln!("❌ Gen2 compilation failed");
        std::process::exit(1);
    }
    println!("  ✓ Gen2: {} bytes", fs::metadata(&gen2_bin).map(|m| m.len()).unwrap_or(0));
    println!();

    // Step 4: Gen2 runs on compiler.ajb → C code, compare with Gen1
    println!("[4/4] Gen2 runs on compiler.ajb → compare with Gen1");
    let status = Command::new(&gen2_bin)
        .args([&compiler_ajb_str])
        .status()
        .expect("Failed to run Gen2");
    if !status.success() {
        eprintln!("❌ Gen2 C codegen failed");
        std::process::exit(1);
    }
    let gen2_c = PathBuf::from("build/output.c");
    if gen2_c.exists() && output_c.exists() {
        let gen1_content = fs::read_to_string(&output_c).unwrap_or_default();
        let gen2_content = fs::read_to_string(&gen2_c).unwrap_or_default();
        if gen1_content == gen2_content {
            println!("  ✓ Gen1 C output == Gen2 C output (IDENTICAL)");
            println!();
            println!("═══════════════════════════════════════════");
            println!("✅ BOOTSTRAP SUCCESS — Self-hosting verified!");
            println!("═══════════════════════════════════════════");
        } else {
            eprintln!("  ✗ Gen1 C output != Gen2 C output");
            eprintln!("  Gen1: {} lines", gen1_content.lines().count());
            eprintln!("  Gen2: {} lines", gen2_content.lines().count());
            std::process::exit(1);
        }
    } else {
        eprintln!("❌ Could not compare C outputs");
        std::process::exit(1);
    }
}
