use std::fs;
use std::path::{Path, PathBuf};

pub fn cmd_init() {
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

pub fn cmd_new(args: &[String]) {
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
