use std::fs;
use std::path::Path;

use crate::config;
use crate::registry;
use crate::resolver;
use crate::types;

pub fn cmd_search(args: &[String]) {
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

pub fn cmd_install(args: &[String]) {
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
                    // Verify signature if available
                    let pkg_version = if version.is_empty() {
                        registry::read_package_version(&local_path).unwrap_or_default()
                    } else {
                        version.clone()
                    };
                    if !pkg_version.is_empty() {
                        match registry::verify_signature(&name, &pkg_version) {
                            Ok(_) => println!("🔑 Signature verified for '{}@{}'", name, pkg_version),
                            Err(e) => eprintln!("⚠️  Warning: signature not verified for '{}@{}': {}", name, pkg_version, e),
                        }
                    }
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
                    // Verify signature if available
                    match registry::verify_signature(&name, &version) {
                        Ok(_) => println!("🔑 Signature verified for '{}@{}'", name, version),
                        Err(e) => eprintln!("⚠️  Warning: signature not verified for '{}@{}': {}", name, version, e),
                    }
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

pub fn cmd_publish(_args: &[String]) {
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
        Err(e) => {
            eprintln!("❌ Publish failed: package signing is required but failed: {}", e);
            eprintln!("   Generate a keypair with `parth keygen` and try again.");
            std::process::exit(1);
        }
    }

    println!("✓ Published '{}@{}' (checksum: {}...)", cfg.pkg_name, cfg.pkg_version, &checksum[..16]);
    println!("📦 Package cached at: {}", cache_dir.display());
}

pub fn cmd_login(args: &[String]) {
    let registry_url = args.first().map(|s| s.as_str()).unwrap_or("https://registry.ajeeb.dev");
    match registry::login(registry_url) {
        Ok(info) => println!("✓ Logged in as '{}'", info.username),
        Err(e) => { eprintln!("❌ Login failed: {}", e); std::process::exit(1); }
    }
}

pub fn cmd_logout() {
    match registry::logout() {
        Ok(()) => {}
        Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
    }
}

pub fn cmd_whoami() {
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

pub fn cmd_yank(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: parth yank <package> <version>");
        std::process::exit(1);
    }
    match registry::yank_package(&args[0], &args[1]) {
        Ok(()) => println!("✓ Yanked '{}@{}'", args[0], args[1]),
        Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
    }
}

pub fn cmd_unyank(args: &[String]) {
    if args.len() < 2 {
        eprintln!("Usage: parth unyank <package> <version>");
        std::process::exit(1);
    }
    match registry::unyank_package(&args[0], &args[1]) {
        Ok(()) => println!("✓ Un-yanked '{}@{}'", args[0], args[1]),
        Err(e) => { eprintln!("❌ {}", e); std::process::exit(1); }
    }
}

pub fn cmd_sign(args: &[String]) {
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

pub fn cmd_verify(args: &[String]) {
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

pub fn cmd_audit(_args: &[String]) {
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

pub fn cmd_keygen() {
    match registry::generate_keypair() {
        Ok((_, pub_hex)) => {
            println!("🔑 Ed25519 keypair generated");
            println!("📁 Keys stored in: {}", registry::keys_dir().display());
            println!("🔓 Public key: {}...", &pub_hex[..16]);
        }
        Err(e) => { eprintln!("❌ Key generation failed: {}", e); std::process::exit(1); }
    }
}
