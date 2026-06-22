use std::fs;

use super::{keys_dir, signatures_dir, sanitize_pkg_segment, package_cache_dir, compute_dir_checksum};
use super::super::types::PackageSignature;

/// Generate a new Ed25519 keypair and save to ~/.parth/keys/
pub fn generate_keypair() -> Result<(String, String), String> {
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    let dir = keys_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create keys dir: {}", e))?;

    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();

    let secret_hex = hex::encode(signing_key.to_bytes());
    let public_hex = hex::encode(verifying_key.to_bytes());

    // Save secret key (readable only by owner)
    let sk_path = dir.join("secret.key");
    fs::write(&sk_path, &secret_hex).map_err(|e| format!("Cannot write secret key: {}", e))?;
    #[cfg(unix)]
    { let _ = std::process::Command::new("chmod").args(["600", &sk_path.to_string_lossy()]).status(); }

    // Save public key
    let pk_path = dir.join("public.key");
    fs::write(&pk_path, &public_hex).map_err(|e| format!("Cannot write public key: {}", e))?;

    Ok((secret_hex, public_hex))
}

/// Load the default keypair. If it doesn't exist, generate one.
pub fn load_or_generate_keypair() -> Result<(ed25519_dalek::SigningKey, ed25519_dalek::VerifyingKey), String> {
    let sk_path = keys_dir().join("secret.key");
    let pk_path = keys_dir().join("public.key");

    if sk_path.exists() && pk_path.exists() {
        let sk_hex = fs::read_to_string(&sk_path).map_err(|e| format!("Cannot read secret key: {}", e))?;
        let sk_hex = sk_hex.trim();
        let sk_bytes = hex::decode(sk_hex).map_err(|e| format!("Invalid secret key hex: {}", e))?;
        let sk_array: [u8; 32] = sk_bytes.try_into().map_err(|_| "Invalid secret key length".to_string())?;
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&sk_array);
        let verifying_key = signing_key.verifying_key();
        Ok((signing_key, verifying_key))
    } else {
        generate_keypair()?;
        load_or_generate_keypair()
    }
}

/// Load a public key by signer name (or "default")
pub fn load_public_key(signer: &str) -> Result<ed25519_dalek::VerifyingKey, String> {
    let pk_path = if signer == "default" || signer.is_empty() {
        keys_dir().join("public.key")
    } else {
        keys_dir().join(format!("{}.pub", sanitize_pkg_segment(signer)))
    };
    if !pk_path.exists() {
        return Err(format!("Public key not found for '{}' at {}", signer, pk_path.display()));
    }
    let pk_hex = fs::read_to_string(&pk_path).map_err(|e| format!("Cannot read public key: {}", e))?;
    let pk_hex = pk_hex.trim();
    let pk_bytes = hex::decode(pk_hex).map_err(|e| format!("Invalid public key hex: {}", e))?;
    let pk_array: [u8; 32] = pk_bytes.try_into().map_err(|_| "Invalid public key length".to_string())?;
    Ok(ed25519_dalek::VerifyingKey::from_bytes(&pk_array).map_err(|e| format!("Invalid public key: {}", e))?)
}

/// Sign a package with Ed25519. Returns the hex-encoded signature.
pub fn sign_package(name: &str, version: &str, signer: &str) -> Result<PackageSignature, String> {
    let cached = package_cache_dir(name, version);
    if !cached.exists() {
        return Err(format!("Package '{}@{}' not in cache", name, version));
    }
    let hash = compute_dir_checksum(&cached)?;
    let (signing_key, verifying_key) = load_or_generate_keypair()?;
    let public_key_hex = hex::encode(verifying_key.to_bytes());

    // Sign the checksum hash
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

    // Save signature to signatures directory
    let sig_dir = signatures_dir().join(sanitize_pkg_segment(name));
    fs::create_dir_all(&sig_dir).map_err(|e| format!("Cannot create sig dir: {}", e))?;
    let sig_path = sig_dir.join(format!("{}.sig", sanitize_pkg_segment(version)));
    let sig_content = serialize_signature(&sig);
    fs::write(&sig_path, &sig_content).map_err(|e| format!("Cannot write signature: {}", e))?;

    Ok(sig)
}

pub fn serialize_signature(sig: &PackageSignature) -> String {
    format!(
        "signer = \"{}\"\nhash = \"{}\"\nsignature_hex = \"{}\"\npublic_key_hex = \"{}\"\ntimestamp = {}\n",
        sig.signer, sig.hash, sig.signature_hex, sig.public_key_hex, sig.timestamp
    )
}

pub fn deserialize_signature(content: &str) -> Result<PackageSignature, String> {
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

/// Verify a package signature using Ed25519
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

    // Verify Ed25519 signature
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
