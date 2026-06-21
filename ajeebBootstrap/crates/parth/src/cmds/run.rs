use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::util::find_ajeeb_root;

pub fn cmd_run() {
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

fn build_with_compiler(file_path: &Path, build_dir: &Path, bin_name: &PathBuf, root: &Path) {
    let native = root.join("build/ajeebc");
    let output_ll = build_dir.join("output.ll");

    if native.exists() {
        println!("⚡ Compiling with ajeebc...");
        let compile_status = Command::new(&native)
            .args([&file_path.to_string_lossy().to_string(), &output_ll.to_string_lossy().to_string(), "--skip-run"])
            .current_dir(&root)
            .status()
            .expect("Failed to run ajeebc");
        if !compile_status.success() {
            eprintln!("❌ Compilation failed");
            std::process::exit(1);
        }

        let runtime = root.join("runtime/ajeeb_runtime.c");
        let asm_file = build_dir.join("output.s");

        println!("🔧 Assembling with llc...");
        let llc_status = Command::new("llc")
            .args(["-O2", &output_ll.to_string_lossy(), "-o", &asm_file.to_string_lossy()])
            .status();
        match llc_status {
            Ok(s) if s.success() => {
                println!("🔨 Linking → {}", bin_name.display());
                let gcc_status = Command::new("gcc")
                    .args([
                        &asm_file.to_string_lossy(),
                        &runtime.to_string_lossy(),
                        "-o", &bin_name.to_string_lossy(),
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

    if parthi_bin.exists() {
        println!("🚀 Running with ParthI (MIR interpreter)...\n");
        let extra_args: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();
        let mut cmd = Command::new(&parthi_bin);
        cmd.arg(file_path);
        cmd.args(&extra_args);
        let run_status = cmd.status().expect("Failed to run parthi");
        std::process::exit(run_status.code().unwrap_or(0));
    }

    let is_native = args.len() > 1 && args[1] == "--native";

    if !is_native {
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

    let stem = Path::new(file_path)
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let bin_path = format!("build/{}", stem);

    fs::create_dir_all("build").ok();
    let build_dir = Path::new("build");
    let bin_pathbuf = build_dir.join(&stem);

    build_with_compiler(Path::new(file_path), build_dir, &bin_pathbuf, &root);

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
