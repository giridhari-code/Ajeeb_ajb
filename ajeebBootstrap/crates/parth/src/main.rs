use std::env;

mod config;
mod registry;
mod resolver;
mod types;
mod cmds;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        cmd_help();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "new" => cmds::cmd_new(&args[2..]),
        "init" => cmds::cmd_init(),
        "add" => cmds::cmd_add(&args[2..]),
        "remove" => cmds::cmd_remove(&args[2..]),
        "build" => {
            if args.len() > 2 && args[2].ends_with(".ajb") {
                cmds::cmd_build_file(&args[2..]);
            } else if args.len() > 2 {
                eprintln!("Usage: parth build [file.ajb]");
                std::process::exit(1);
            } else {
                cmds::cmd_build();
            }
        }
        "run" => {
            if args.len() > 2 {
                cmds::cmd_run_file(&args[2..]);
            } else {
                cmds::cmd_run();
            }
        }
        "test" => cmds::cmd_test(),
        "fmt" => cmds::cmd_fmt(&args[2..]),
        "doc" => cmds::cmd_doc(),
        "publish" => cmds::cmd_publish(&args[2..]),
        "update" => cmds::cmd_update(),
        "tree" => cmds::cmd_tree(),
        "why" => cmds::cmd_why(&args[2..]),
        "outdated" => cmds::cmd_outdated(),
        "upgrade" => cmds::cmd_upgrade(&args[2..]),
        "info" => cmds::cmd_info(),
        "search" => cmds::cmd_search(&args[2..]),
        "install" => cmds::cmd_install(&args[2..]),
        "login" => cmds::cmd_login(&args[2..]),
        "logout" => cmds::cmd_logout(),
        "whoami" => cmds::cmd_whoami(),
        "sign" => cmds::cmd_sign(&args[2..]),
        "verify" => cmds::cmd_verify(&args[2..]),
        "keygen" => cmds::cmd_keygen(),
        "yank" => cmds::cmd_yank(&args[2..]),
        "unyank" => cmds::cmd_unyank(&args[2..]),
        "audit" => cmds::cmd_audit(&args[2..]),
        "cache" => cmds::cmd_cache(&args[2..]),
        "workspace" => cmds::cmd_workspace(&args[2..]),
        "link" => cmds::cmd_link(&args[2..]),
        "list" => cmds::cmd_list(),
        "version" => cmd_version(),
        "clean" => cmds::cmd_clean(),
        "help" | "-h" | "--help" => cmd_help(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }
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

fn cmd_version() {
    println!("parth 0.1.0 — Ajeeb Package Manager");
    if let Ok(content) = std::fs::read_to_string("parth.das") {
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
