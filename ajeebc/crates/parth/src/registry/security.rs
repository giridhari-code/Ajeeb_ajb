use std::fs;

use super::{advisories_dir, signatures_dir, package_cache_dir, sanitize_pkg_segment, compute_dir_checksum};
use super::super::types::{Advisory, LockFile, Version, VersionConstraint};
use super::crypto::verify_signature;

pub fn load_advisories() -> Vec<Advisory> {
    let dir = advisories_dir();
    if !dir.exists() { return Vec::new(); }
    let mut advisories = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(advisory) = serde_json::from_str::<Advisory>(&content) {
                        advisories.push(advisory);
                    }
                }
            }
        }
    }
    advisories
}

pub fn add_advisory(advisory: &Advisory) -> Result<(), String> {
    let dir = advisories_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create advisories dir: {}", e))?;
    let path = dir.join(format!("{}.json", advisory.id));
    let json = serde_json::to_string_pretty(advisory)
        .map_err(|e| format!("Cannot serialize advisory: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Cannot write advisory: {}", e))?;
    Ok(())
}

/// Scan dependencies for known vulnerabilities
pub fn audit_deps(lock: &LockFile) -> Vec<Advisory> {
    let advisories = load_advisories();
    let mut findings = Vec::new();

    for (name, entry) in lock {
        let ver = match Version::parse(&entry.version) {
            Some(v) => v,
            None => continue,
        };
        for adv in &advisories {
            if adv.package != *name { continue; }
            if let Some(affected_constraint) = VersionConstraint::parse(&adv.versions_affected) {
                if affected_constraint.matches(&ver) {
                    findings.push(adv.clone());
                }
            }
        }
    }
    findings
}

/// Fetch latest advisories from remote
pub fn fetch_advisories(registry_url: &str) -> Result<Vec<Advisory>, String> {
    use super::remote::http_get;

    let url = format!("{}/api/v1/advisories.json", registry_url.trim_end_matches('/'));
    match http_get(&url) {
        Ok(json) => match serde_json::from_str::<Vec<Advisory>>(&json) {
            Ok(advisories) => {
                for adv in &advisories {
                    let _ = add_advisory(adv);
                }
                Ok(advisories)
            }
            Err(e) => Err(format!("Failed to parse advisories: {}", e)),
        }
        Err(e) => Err(e),
    }
}

pub fn security_scan(lock: &LockFile) -> Vec<String> {
    let mut issues = Vec::new();

    for (name, entry) in lock {
        // Check for signed packages
        let sig_dir = signatures_dir().join(sanitize_pkg_segment(name));
        let sig_path = sig_dir.join(format!("{}.sig", sanitize_pkg_segment(&entry.version)));
        if !sig_path.exists() {
            issues.push(format!("{}@{}: unsigned package — supply chain risk", name, entry.version));
        } else {
            // Verify Ed25519 signature
            match verify_signature(name, &entry.version) {
                Ok(_) => {} // Signature valid
                Err(e) => {
                    issues.push(format!("{}@{}: signature verification failed: {}", name, entry.version, e));
                }
            }
        }

        // Check for integrity
        let cached = package_cache_dir(name, &entry.version);
        if cached.exists() {
            if let Ok(actual) = compute_dir_checksum(&cached) {
                if actual != entry.checksum {
                    issues.push(format!("{}@{}: checksum mismatch — possible tampering", name, entry.version));
                }
            }
        }
    }

    issues
}
