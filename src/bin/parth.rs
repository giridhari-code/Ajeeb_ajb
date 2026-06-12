use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn find_ajeeb_root() -> PathBuf {
    let exe = env::current_exe().expect("Cannot find parth binary path");
    let mut dir = exe.parent().unwrap();
    for _ in 0..3 {
        if dir.join("compiler").join("compiler.ajb").exists() {
            return dir.to_path_buf();
        }
        if let Some(parent) = dir.parent() {
            dir = parent;
        } else {
            break;
        }
    }
    PathBuf::from("..")
}

fn read_config() -> (String, String, String) {
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

fn cmd_new(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: parth new <project-name>");
        std::process::exit(1);
    }
    let name = &args[0];
    let dir = PathBuf::from(name);
    if dir.exists() {
        eprintln!("Error: directory '{}' already exists", name);
        std::process::exit(1);
    }
    fs::create_dir_all(dir.join("src")).expect("Cannot create src dir");
    fs::create_dir_all(dir.join("build")).expect("Cannot create build dir");

    let das = format!(
        "[package]\n\
         name = \"{}\"\n\
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
        name
    );
    fs::write(dir.join("parth.das"), das).expect("Cannot write parth.das");

    let main_ajb =
        "function main(): int {\n    println(\"Hello from Ajeeb!\");\n    return 0;\n}\n";
    fs::write(dir.join("src").join("main.ajb"), main_ajb).expect("Cannot write main.ajb");

    println!("✓ Created Ajeeb project '{}'", name);
}

fn cmd_build() {
    let (name, output_dir, runtime) = read_config();
    let root = find_ajeeb_root();
    let compiler_src = root.join("compiler").join("compiler.ajb");
    let runtime_src = root.join(&runtime);
    let runtime_src_str = runtime_src.to_string_lossy().to_string();
    let compiler_src_str = compiler_src.to_string_lossy().to_string();

    let status = Command::new("cargo")
        .args(&[
            "run",
            "--bin",
            "ajeeb_compiler",
            "--manifest-path",
            root.join("Cargo.toml").to_string_lossy().as_ref(),
            "--",
            &compiler_src_str,
            "src/main.ajb",
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
        .args(&[
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
    let (name, output_dir, _) = read_config();
    cmd_build();
    let bin_path = format!("{}{}", output_dir, name);
    let status = Command::new(&bin_path)
        .status()
        .expect("Failed to run binary");
    std::process::exit(status.code().unwrap_or(1));
}

fn cmd_info() {
    let content = fs::read_to_string("parth.das").unwrap_or_default();
    println!("📦 parth.das contents:\n");
    println!("{}", content);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: parth <command> [args]");
        eprintln!("Commands:");
        eprintln!("  new <name>    Create a new Ajeeb project");
        eprintln!("  build         Build the current project");
        eprintln!("  run           Build and run the current project");
        eprintln!("  info          Show project info");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "new" => cmd_new(&args[2..]),
        "build" => cmd_build(),
        "run" => cmd_run(),
        "info" => cmd_info(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }
}
