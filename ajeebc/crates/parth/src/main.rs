use std::env;
use std::path::PathBuf;

mod commands;
mod config;
mod registry;
mod resolver;
mod types;

use config::ProjectConfig;

use commands::build::*;
use commands::deps::*;
use commands::init::*;
use commands::project::*;
use commands::registry::*;

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

fn cmd_help() {
    println!("Ajeeb Package Manager — parth v1.0.1");
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
    println!("  bench [filter]   Run benchmarks in benches/ directory");
    println!("  test             Run all tests in tests/ directory");
    println!("  fmt [files..]    Format Ajeeb source files");
    println!("  lint [path]      Lint Ajeeb source files");
    println!("  doc [--open]     Generate documentation (use --open to auto-open)");
    println!("  sanitize [file]  Run sanitizer checks (memory safety, bounds)");
    println!("  package          Package into tarball without publishing");
    println!("  generate-lockfile Generate parth.lock without building");
    println!("  vendor           Vendor dependencies into vendor/ directory");
    println!("  ls               List workspace packages");
    println!("  clean            Remove build artifacts");
    println!("  bootstrap        Verify self-hosting (Gen0→Gen1→Gen2)");
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
        "bench" => cmd_bench(&args[2..]),
        "lint" => cmd_lint(&args[2..]),
        "sanitize" => cmd_sanitize(&args[2..]),
        "package" => cmd_package(),
        "generate-lockfile" => cmd_generate_lockfile(),
        "vendor" => cmd_vendor(),
        "ls" => cmd_ls(),
        "doc" => {
            if args.len() > 2 && args[2] == "--open" { cmd_doc_open(); }
            else { cmd_doc(); }
        }
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
        "bootstrap" => cmd_bootstrap(),
        "help" | "-h" | "--help" => cmd_help(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }
}
