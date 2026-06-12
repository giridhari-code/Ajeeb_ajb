use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// Parth internals
mod parth_mod {
    include!("../parth/mod.rs");
}
use parth_mod::{config, registry, resolver, types::PkgDep};

fn find_ajeeb_root() -> PathBuf {
    if let Ok(manifest) = env::var("CARGO_MANIFEST_DIR") {
        let root = PathBuf::from(manifest);
        if root.join("compiler").join("compiler.ajb").exists() {
            return root;
        }
    }
    let mut dir = env::current_dir().unwrap_or_default();
    loop {
        if dir.join("compiler").join("compiler.ajb").exists() {
            return dir;
        }
        if !dir.pop() {
            break;
        }
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            let mut d = parent.to_path_buf();
            loop {
                if d.join("compiler").join("compiler.ajb").exists() {
                    return d;
                }
                if !d.pop() {
                    break;
                }
            }
        }
    }
    PathBuf::from("..")
}

fn read_config_basic() -> (String, String, String) {
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
            if current_section == "package" && key == "name" {
                name = val.to_string();
            } else if current_section == "compiler" && key == "output" {
                output = val.to_string();
            } else if current_section == "compiler" && key == "runtime" {
                runtime = val.to_string();
            }
        }
    }
    (name, output, runtime)
}

// ── Commands ────────────────────────────────────────────────────────

fn cmd_new(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: parth new <project-name>");
        std::process::exit(1);
    }
    let raw_name = &args[0];
    // Validate project name
    if !raw_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        eprintln!("Error: project name must only contain letters, numbers, '_', and '-'");
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
         \n\
         [dependencies]\n\
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

    let main_ajb =
        "function main(): int {\n    println(\"Hello from Ajeeb!\");\n    return 0;\n}\n";
    fs::write(dir.join("src").join("main.ajb"), main_ajb).expect("Cannot write main.ajb");

    println!("✓ Created Ajeeb project '{}'", raw_name);
}

fn cmd_add(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: parth add <package>[@<version>]");
        std::process::exit(1);
    }
    let spec = args[0].clone();
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found in current directory");
        std::process::exit(1);
    }

    let (pkg_name, version_req) = if let Some(at) = spec.find('@') {
        let n = spec[..at].to_string();
        let v = spec[at + 1..].to_string();
        (n, v)
    } else {
        (spec.clone(), "*".to_string())
    };
    let original_req = version_req.clone();

    // Read current deps
    let (_proj_name, _version, mut deps) = config::read_config(Path::new("parth.das"))
        .unwrap_or_else(|e| {
            eprintln!("Error reading parth.das: {}", e);
            std::process::exit(1);
        });

    // Check if already added
    if deps.iter().any(|d| d.name == pkg_name) {
        println!("ℹ️  '{}' is already a dependency", pkg_name);
        return;
    }

    // Resolve and cache the package (pass ALL deps to detect conflicts)
    let new_dep = PkgDep {
        name: pkg_name.clone(),
        version_req,
    };
    let mut all_deps = deps.clone();
    all_deps.push(new_dep);
    let project_dir = Path::new(".");
    match resolver::resolve_and_cache(&all_deps, project_dir) {
        Ok((_resolved, _lock)) => {
            deps.push(PkgDep {
                name: pkg_name.clone(),
                version_req: original_req,
            });
            config::update_deps(Path::new("parth.das"), &deps).unwrap_or_else(|e| {
                eprintln!("Error updating parth.das: {}", e);
                std::process::exit(1);
            });
            println!("✓ Added '{}' to dependencies", pkg_name);
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
        eprintln!("Error: no parth.das found");
        std::process::exit(1);
    }

    let (_, _, deps) = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error reading parth.das: {}", e);
        std::process::exit(1);
    });

    let new_deps: Vec<PkgDep> = deps.into_iter().filter(|d| d.name != *name).collect();

    config::update_deps(Path::new("parth.das"), &new_deps).unwrap_or_else(|e| {
        eprintln!("Error updating parth.das: {}", e);
        std::process::exit(1);
    });

    // Remove from lock file too
    let project_dir = Path::new(".");
    let mut lock = resolver::read_lock(project_dir);
    lock.remove(name);
    resolver::write_lock(&lock, project_dir).unwrap_or_else(|e| {
        eprintln!("Warning: could not update lock file: {}", e);
    });

    println!("✓ Removed '{}' from dependencies", name);
}

fn cmd_build() {
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found");
        std::process::exit(1);
    }

    // Resolve dependencies first
    let (_name, _ver, deps) = config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
        eprintln!("Error reading parth.das: {}", e);
        std::process::exit(1);
    });

    if !deps.is_empty() {
        let project_dir = Path::new(".");
        match resolver::resolve_and_cache(&deps, project_dir) {
            Ok((_resolved, lock)) => {
                match resolver::compilation_order(&lock) {
                    Ok(order) => {
                        if !order.is_empty() {
                            println!("📦 Dependencies: {}", order.join(", "));
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Dependency resolution failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    let (name, output_dir, runtime) = read_config_basic();
    let root = find_ajeeb_root();
    let runtime_src = root.join(&runtime);
    let runtime_src_str = runtime_src.to_string_lossy().to_string();

    let entry = "src/main.ajb";

    let status = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "ajeeb_compiler",
            "--manifest-path",
            root.join("Cargo.toml").to_string_lossy().as_ref(),
            "--",
            entry,
        ])
        .status()
        .expect("Failed to run compiler");
    if !status.success() {
        eprintln!("❌ Compilation failed");
        std::process::exit(1);
    }

    let out_path = format!("{}output.c", output_dir);
    let bin_path = format!("{}{}", output_dir, name);
    let status = Command::new("gcc")
        .args([
            &out_path,
            &runtime_src_str,
            "-o",
            &bin_path,
            "-Wall",
            "-Wno-int-to-pointer-cast",
            "-Wno-pointer-to-int-cast",
        ])
        .status()
        .expect("Failed to run gcc");
    if !status.success() {
        eprintln!("❌ GCC compilation failed");
        std::process::exit(1);
    }
    println!("✓ Build complete: {}", bin_path);
}

fn cmd_run() {
    let (_name, _output_dir, _) = read_config_basic();
    cmd_build();
    let (name, output_dir, _) = read_config_basic();
    let bin_path = format!("{}{}", output_dir, name);
    let entry = "src/main.ajb";
    let status = Command::new(&bin_path)
        .arg(entry)
        .status()
        .expect("Failed to run binary");
    std::process::exit(status.code().unwrap_or(1));
}

fn cmd_info() {
    let content = fs::read_to_string("parth.das").unwrap_or_default();
    println!("📦 parth.das:\n");
    println!("{}", content);

    // Show lock file if it exists
    if Path::new("parth.lock").exists() {
        let lock_content = fs::read_to_string("parth.lock").unwrap_or_default();
        println!("🔒 parth.lock:\n");
        println!("{}", lock_content);
    }
}

fn cmd_publish(args: &[String]) {
    if !Path::new("parth.das").exists() {
        eprintln!("Error: no parth.das found");
        std::process::exit(1);
    }

    let (pkg_name, pkg_version, _deps) =
        config::read_config(Path::new("parth.das")).unwrap_or_else(|e| {
            eprintln!("Error reading parth.das: {}", e);
            std::process::exit(1);
        });

    if pkg_name.is_empty() || pkg_name == "project" {
        eprintln!("Error: package name must be set in [package] section of parth.das");
        std::process::exit(1);
    }
    if pkg_version.is_empty() || pkg_version == "0.1.0" {
        // Allow if explicitly set
        let content = fs::read_to_string("parth.das").unwrap_or_default();
        if !content.contains("version =") {
            eprintln!("Error: version must be set in [package] section");
            std::process::exit(1);
        }
    }

    // If a registry URL is given, use it; otherwise local
    let registry_arg = args.first().map(|s| s.as_str());

    let pkg_dir = Path::new(".");
    let cache_dir = registry::package_src(pkg_dir, &pkg_name, &pkg_version).unwrap_or_else(|e| {
        eprintln!("❌ Package failed: {}", e);
        std::process::exit(1);
    });

    let checksum = registry::compute_dir_checksum(&cache_dir).unwrap_or_else(|e| {
        eprintln!("❌ Checksum failed: {}", e);
        std::process::exit(1);
    });

    registry::register_package(&pkg_name, &pkg_version, &checksum).unwrap_or_else(|e| {
        eprintln!("❌ Registry update failed: {}", e);
        std::process::exit(1);
    });

    println!("✓ Published '{}@{}' (checksum: {})", pkg_name, pkg_version, &checksum[..16]);

    if let Some(url) = registry_arg {
        println!("ℹ️  To publish to remote registry, push ~/.parth/index and ~/.parth/packages/ to {}", url);
    } else {
        println!("ℹ️  Published to local registry (~/.parth/)");
    }
}

// ── Main ────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: parth <command> [args]");
        eprintln!("Commands:");
        eprintln!("  new <name>      Create a new Ajeeb project");
        eprintln!("  add <pkg>[@v]   Add a dependency");
        eprintln!("  remove <pkg>    Remove a dependency");
        eprintln!("  build           Build the current project");
        eprintln!("  run             Build and run the current project");
        eprintln!("  publish [url]   Publish the current package");
        eprintln!("  info            Show project info");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "new" => cmd_new(&args[2..]),
        "add" => cmd_add(&args[2..]),
        "remove" => cmd_remove(&args[2..]),
        "build" => cmd_build(),
        "run" => cmd_run(),
        "publish" => cmd_publish(&args[2..]),
        "info" => cmd_info(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }
}
