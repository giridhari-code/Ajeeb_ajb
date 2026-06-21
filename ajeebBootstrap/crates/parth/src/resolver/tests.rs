use std::fs;

use super::{compilation_order, read_lock, write_lock};
use crate::types::{LockEntry, LockFile, PkgDep, Version, VersionConstraint};

fn v(s: &str) -> Version { Version::parse(s).unwrap() }

fn make_lock(entries: &[(&str, &str, &[(&str, &str)])]) -> LockFile {
    let mut lock = LockFile::new();
    for (name, version, deps) in entries {
        let deps_vec: Vec<PkgDep> = deps.iter().map(|(n, v)| PkgDep {
            name: n.to_string(), version_req: v.to_string(),
        }).collect();
        lock.insert(name.to_string(), LockEntry {
            version: version.to_string(), checksum: "".to_string(),
            dependencies: deps_vec, registry: String::new(),
        });
    }
    lock
}

#[test]
fn test_compilation_order_linear() {
    let lock = make_lock(&[
        ("A", "1.0.0", &[("B", "1.0.0")]),
        ("B", "1.0.0", &[("C", "1.0.0")]),
        ("C", "1.0.0", &[]),
    ]);
    let order = compilation_order(&lock).unwrap();
    assert_eq!(order, vec!["C", "B", "A"]);
}

#[test]
fn test_compilation_order_diamond() {
    let lock = make_lock(&[
        ("A", "1.0.0", &[("B", "1.0.0"), ("C", "1.0.0")]),
        ("B", "1.0.0", &[("D", "1.0.0")]),
        ("C", "1.0.0", &[("D", "1.0.0")]),
        ("D", "1.0.0", &[]),
    ]);
    let order = compilation_order(&lock).unwrap();
    assert!(order.iter().position(|n| n == "D") < order.iter().position(|n| n == "B"));
    assert!(order.iter().position(|n| n == "D") < order.iter().position(|n| n == "C"));
    assert!(order.iter().position(|n| n == "B") < order.iter().position(|n| n == "A"));
}

#[test]
fn test_compilation_order_circular() {
    let lock = make_lock(&[
        ("A", "1.0.0", &[("B", "1.0.0")]),
        ("B", "1.0.0", &[("A", "1.0.0")]),
    ]);
    assert!(compilation_order(&lock).is_err());
}

#[test]
fn test_lock_v2_roundtrip() {
    let dir = std::env::temp_dir().join(format!("parth_test_v2_{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let mut lock = LockFile::new();
    lock.insert("foo".to_string(), LockEntry {
        version: "1.0.0".to_string(), checksum: "abc".to_string(),
        dependencies: vec![PkgDep { name: "bar".into(), version_req: "^1.0.0".into() }],
        registry: "https://registry.example.com".into(),
    });
    write_lock(&lock, &dir).unwrap();
    let read = read_lock(&dir);
    assert!(read.contains_key("foo"));
    assert_eq!(read["foo"].version, "1.0.0");
    assert_eq!(read["foo"].registry, "https://registry.example.com");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_compound_constraint() {
    let c = VersionConstraint::parse(">=1.0.0 <2.0.0").unwrap();
    assert!(c.matches(&Version::parse("1.5.0").unwrap()));
    assert!(!c.matches(&Version::parse("2.0.0").unwrap()));

    let c = VersionConstraint::parse("^1.0.0 || ^2.0.0").unwrap();
    assert!(c.matches(&Version::parse("1.5.0").unwrap()));
    assert!(c.matches(&Version::parse("2.0.0").unwrap()));
    assert!(!c.matches(&Version::parse("3.0.0").unwrap()));
}
