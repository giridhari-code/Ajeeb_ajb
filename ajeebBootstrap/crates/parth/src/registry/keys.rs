use std::fs;

use super::paths::keys_dir;

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

    let sk_path = dir.join("secret.key");
    fs::write(&sk_path, &secret_hex).map_err(|e| format!("Cannot write secret key: {}", e))?;
    #[cfg(unix)]
    { let _ = std::process::Command::new("chmod").args(["600", &sk_path.to_string_lossy()]).status(); }

    let pk_path = dir.join("public.key");
    fs::write(&pk_path, &public_hex).map_err(|e| format!("Cannot write public key: {}", e))?;

    Ok((secret_hex, public_hex))
}

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

pub fn load_public_key(signer: &str) -> Result<ed25519_dalek::VerifyingKey, String> {
    let pk_path = if signer == "default" || signer.is_empty() {
        keys_dir().join("public.key")
    } else {
        keys_dir().join(format!("{}.pub", signer.chars()
            .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' { c } else { '_' })
            .collect::<String>()))
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
