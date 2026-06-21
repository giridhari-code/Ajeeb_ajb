use std::fs;

use super::paths::{package_cache_dir, signatures_dir, sanitize_pkg_segment};
use super::cache::compute_dir_checksum;
use super::keys::load_or_generate_keypair;
use super::super::types::PackageSignature;

pub(crate) fn serialize_signature(sig: &PackageSignature) -> String {
    format!(
        "signer = \"{}\"\nhash = \"{}\"\nsignature_hex = \"{}\"\npublic_key_hex = \"{}\"\ntimestamp = {}\n",
        sig.signer, sig.hash, sig.signature_hex, sig.public_key_hex, sig.timestamp
    )
}

pub(crate) fn deserialize_signature(content: &str) -> Result<PackageSignature, String> {
    let mut signer = String::new();
    let mut hash = String::new();
    let mut signature_hex = String::new();
    let mut public_key_hex = String::new();
    let mut timestamp: u64 = 0;
    for line in content.lines() {
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim();
            let val = line[eq + 1..].trim().trim_matches('"');
            match key {
                "signer" => signer = val.to_string(),
                "hash" => hash = val.to_string(),
                "signature_hex" => signature_hex = val.to_string(),
                "public_key_hex" => public_key_hex = val.to_string(),
                "timestamp" => timestamp = val.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    if signature_hex.is_empty() || hash.is_empty() {
        return Err("Invalid signature format".to_string());
    }
    Ok(PackageSignature { signer, hash, signature_hex, public_key_hex, timestamp })
}

pub fn sign_package(name: &str, version: &str, signer: &str) -> Result<PackageSignature, String> {
    let cached = package_cache_dir(name, version);
    if !cached.exists() {
        return Err(format!("Package '{}@{}' not in cache", name, version));
    }
    let hash = compute_dir_checksum(&cached)?;
    let (signing_key, verifying_key) = load_or_generate_keypair()?;
    let public_key_hex = hex::encode(verifying_key.to_bytes());

    use ed25519_dalek::Signer;
    let signature = signing_key.sign(hash.as_bytes());
    let signature_hex = hex::encode(signature.to_bytes());

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0);

    let sig = PackageSignature {
        signer: if signer.is_empty() || signer == "default" {
            public_key_hex[..16].to_string()
        } else {
            signer.to_string()
        },
        hash,
        signature_hex,
        public_key_hex,
        timestamp,
    };

    let sig_dir = signatures_dir().join(sanitize_pkg_segment(name));
    fs::create_dir_all(&sig_dir).map_err(|e| format!("Cannot create sig dir: {}", e))?;
    let sig_path = sig_dir.join(format!("{}.sig", sanitize_pkg_segment(version)));
    let sig_content = serialize_signature(&sig);
    fs::write(&sig_path, &sig_content).map_err(|e| format!("Cannot write signature: {}", e))?;

    Ok(sig)
}

pub fn verify_signature(name: &str, version: &str) -> Result<PackageSignature, String> {
    let sig_dir = signatures_dir().join(sanitize_pkg_segment(name));
    let sig_path = sig_dir.join(format!("{}.sig", sanitize_pkg_segment(version)));
    if !sig_path.exists() {
        return Err(format!("No signature found for '{}@{}'", name, version));
    }
    let content = fs::read_to_string(&sig_path)
        .map_err(|e| format!("Cannot read signature: {}", e))?;
    let sig = deserialize_signature(&content)?;

    let cached = package_cache_dir(name, version);
    if !cached.exists() {
        return Err(format!("Package '{}@{}' not in cache", name, version));
    }
    let actual_hash = compute_dir_checksum(&cached)?;
    if sig.hash != actual_hash {
        return Err(format!(
            "Signature hash mismatch for '{}@{}': expected {}, got {}. Package has been modified!",
            name, version, sig.hash, actual_hash
        ));
    }

    let sig_bytes = hex::decode(&sig.signature_hex)
        .map_err(|e| format!("Invalid signature hex: {}", e))?;
    let pk_bytes = hex::decode(&sig.public_key_hex)
        .map_err(|e| format!("Invalid public key hex: {}", e))?;

    let pk_array: [u8; 32] = pk_bytes.try_into().map_err(|_| "Invalid public key length".to_string())?;
    let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&pk_array)
        .map_err(|e| format!("Invalid public key: {}", e))?;

    let sig_array: [u8; 64] = sig_bytes.try_into().map_err(|_| "Invalid signature length".to_string())?;
    let signature = ed25519_dalek::Signature::from_bytes(&sig_array);

    use ed25519_dalek::Verifier;
    verifying_key.verify(sig.hash.as_bytes(), &signature)
        .map_err(|e| format!("Ed25519 signature verification FAILED: {}. Package may be tampered!", e))?;

    Ok(sig)
}

pub fn read_signature(name: &str, version: &str) -> Option<PackageSignature> {
    let sig_path = signatures_dir()
        .join(sanitize_pkg_segment(name))
        .join(format!("{}.sig", sanitize_pkg_segment(version)));
    if !sig_path.exists() {
        return None;
    }
    let content = fs::read_to_string(&sig_path).ok()?;
    let sig = deserialize_signature(&content).ok()?;
    Some(sig)
}
