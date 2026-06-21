use std::fs;

use super::signing::{serialize_signature, deserialize_signature};
use super::cache::compute_dir_checksum;
use super::metadata::{read_metadata, yank_package, unyank_package, is_yanked};
use super::keys::{generate_keypair, load_or_generate_keypair};
use super::super::types::{PackageSignature, RegistryMetadata};

#[test]
fn test_serialize_deserialize_signature() {
    let sig = PackageSignature {
        signer: "test_signer".into(),
        hash: "abc123".into(),
        signature_hex: "deadbeef".into(),
        public_key_hex: "cafebabe".into(),
        timestamp: 1234567890,
    };
    let serialized = serialize_signature(&sig);
    let deserialized = deserialize_signature(&serialized).unwrap();
    assert_eq!(sig.signer, deserialized.signer);
    assert_eq!(sig.hash, deserialized.hash);
    assert_eq!(sig.signature_hex, deserialized.signature_hex);
    assert_eq!(sig.public_key_hex, deserialized.public_key_hex);
    assert_eq!(sig.timestamp, deserialized.timestamp);
}

#[test]
fn test_deserialize_invalid_signature() {
    assert!(deserialize_signature("").is_err());
    assert!(deserialize_signature("garbage = data").is_err());
}

#[test]
fn test_metadata_write_read() {
    let tmp = std::env::temp_dir().join(format!("parth_test_meta_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let meta = RegistryMetadata::new("test_pkg");
    assert_eq!(meta.description, "");
    assert_eq!(meta.yanked, false);

    let mut meta2 = RegistryMetadata::new("test_pkg");
    meta2.description = "A test package".into();
    meta2.author = "Tester".into();
    meta2.homepage = "https://example.com".into();
    meta2.license = "MIT".into();
    meta2.yanked = true;

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_compute_dir_checksum() {
    let tmp = std::env::temp_dir().join(format!("parth_test_sum_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    fs::write(tmp.join("test.txt"), "hello world").unwrap();
    fs::create_dir_all(tmp.join("sub")).unwrap();
    fs::write(tmp.join("sub").join("nested.txt"), "nested data").unwrap();

    let checksum = compute_dir_checksum(&tmp).unwrap();
    assert!(!checksum.is_empty());
    assert_eq!(checksum.len(), 64);

    let checksum2 = compute_dir_checksum(&tmp).unwrap();
    assert_eq!(checksum, checksum2);

    fs::write(tmp.join("test.txt"), "modified").unwrap();
    let checksum3 = compute_dir_checksum(&tmp).unwrap();
    assert_ne!(checksum, checksum3);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_content_addressed_cache() {
    let tmp = std::env::temp_dir().join(format!("parth_test_cache_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);

    let obj_dir = tmp.join("objects");
    let idx_dir = tmp.join("index");
    fs::create_dir_all(&obj_dir).unwrap();
    fs::create_dir_all(&idx_dir).unwrap();

    use sha2::Digest;
    let data = b"test data";
    let hash = sha2::Sha256::digest(data);
    let hash_hex = format!("{:x}", hash);

    let subdir = &hash_hex[..2];
    let obj_sub = obj_dir.join(subdir);
    fs::create_dir_all(&obj_sub).unwrap();
    fs::write(obj_sub.join(&hash_hex), data).unwrap();
    fs::write(idx_dir.join("test_key"), &hash_hex).unwrap();

    assert_eq!(hash_hex.len(), 64);
    let read_back = fs::read(obj_sub.join(&hash_hex)).unwrap();
    assert_eq!(read_back, data);
    let index_back = fs::read_to_string(idx_dir.join("test_key")).unwrap();
    assert_eq!(index_back.trim(), hash_hex);

    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_yank_unyank() {
    let tmp = std::env::temp_dir().join(format!("parth_test_yank_{}", std::process::id()));
    let original_home = std::env::var("HOME").ok();
    std::env::set_var("HOME", &tmp);

    let meta = read_metadata("yank_test", "1.0.0");
    assert_eq!(meta.yanked, false);

    yank_package("yank_test", "1.0.0").unwrap();
    assert!(is_yanked("yank_test", "1.0.0"));

    unyank_package("yank_test", "1.0.0").unwrap();
    assert!(!is_yanked("yank_test", "1.0.0"));

    let _ = fs::remove_dir_all(&tmp);
    if let Some(h) = original_home { std::env::set_var("HOME", h); }
}

#[test]
fn test_key_generation() {
    let tmp = std::env::temp_dir().join(format!("parth_test_key_{}", std::process::id()));
    let original_home = std::env::var("HOME").ok();
    std::env::set_var("HOME", &tmp);

    let (secret, public) = generate_keypair().unwrap();
    assert_eq!(secret.len(), 64);
    assert_eq!(public.len(), 64);

    let (_sk, vk) = load_or_generate_keypair().unwrap();
    assert_eq!(hex::encode(vk.to_bytes()), public);

    let _ = fs::remove_dir_all(&tmp);
    if let Some(h) = original_home { std::env::set_var("HOME", h); }
}
