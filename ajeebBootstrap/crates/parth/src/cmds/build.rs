use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::super::config;
use super::util::find_ajeeb_root;

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

fn dep_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.join("packages"));
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    paths.push(PathBuf::from(home).join(".parth").join("packages"));

    let root = find_ajeeb_root();
    paths.push(root.join("packages"));

    paths
}

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

fn build_with_compiler(file_path: &Path, build_dir: &Path, bin_name: &PathBuf, root: &Path) {
    let native_binary = root.join("build/ajeebc");
    if native_binary.exists() {
        println!("⚡ Using ajeebc compiler");
        let output_ll = build_dir.join("output.ll");
        let status = Command::new(&native_binary)
            .args([&file_path.to_string_lossy().to_string(), &output_ll.to_string_lossy().to_string(), "--skip-run"])
            .current_dir(&root)
            .status()
            .expect("Failed to run ajeebc");
        if !status.success() {
            eprintln!("❌ Compilation failed");
            std::process::exit(1);
        }
        let asm_file = build_dir.join("output.s");
        let runtime_src = root.join("runtime/ajeeb_runtime.c");
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
        println!("🔧 Using Rust interpreter");
        let status = Command::new("cargo")
            .args(["run", "-p", "ajeeb-compiler", "--bin", "ajeeb_compiler",
                   "--", &file_path.to_string_lossy().to_string(), "--skip-run"])
            .current_dir(&root)
            .status().expect("Failed to run compiler");
        if !status.success() {
            eprintln!("❌ Compilation failed");
            std::process::exit(1);
        }

        let llvm_ir = root.join("build/output.ll");
        let asm_file = build_dir.join("output.s");
        let runtime_src = root.join("runtime/ajeeb_runtime.c");
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
    let root = find_ajeeb_root();
    let bin_name = build_dir.join(abs_file_path.file_stem().unwrap());

    build_with_compiler(&abs_file_path, &build_dir, &bin_name, &root);
    println!("  Run with: parth run {}", file_path);
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

    let build_dir = project_dir.join(&output_dir);
    fs::create_dir_all(&build_dir).ok();
    let combined_path = build_dir.join("combined.ajb");
    let bin_path = build_dir.join(&name);
    let runtime_src = root.join(&runtime);

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

    let entry = project_dir.join("src/main.ajb");
    all_ajb_files.push(entry);

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

    let native_binary = root.join("build/ajeebc");
    let combined_str = combined_path.to_string_lossy().to_string();

    if native_binary.exists() {
        println!("⚡ Using self-hosted compiler (ajeebc)");
        let output_ll = build_dir.join("output.ll");
        let output_s = build_dir.join("output.s");
        let status = Command::new(&native_binary)
            .args([&combined_str, &output_ll.to_string_lossy().to_string(), "--skip-run"])
            .current_dir(&root)
            .status()
            .expect("Failed to run ajeebc");
        if !status.success() {
            eprintln!("❌ Self-hosted compilation failed");
            std::process::exit(1);
        }

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
