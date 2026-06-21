use std::fs;
use std::path::PathBuf;
use std::io::{self, Write as IoWrite};

use super::paths::parth_home;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct AuthInfo {
    pub token: String,
    pub username: String,
    pub registry_url: String,
}

pub fn auth_path() -> PathBuf { parth_home().join("auth.json") }

pub fn login(registry_url: &str) -> Result<AuthInfo, String> {
    print!("Token: ");
    io::stdout().flush().map_err(|e| format!("IO error: {}", e))?;
    let mut token = String::new();
    io::stdin().read_line(&mut token).map_err(|e| format!("Cannot read token: {}", e))?;
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err("Token cannot be empty".to_string());
    }

    print!("Username: ");
    io::stdout().flush().map_err(|e| format!("IO error: {}", e))?;
    let mut username = String::new();
    io::stdin().read_line(&mut username).map_err(|e| format!("Cannot read username: {}", e))?;
    let username = username.trim().to_string();
    if username.is_empty() {
        return Err("Username cannot be empty".to_string());
    }

    let info = AuthInfo {
        token: token.clone(),
        username: username.clone(),
        registry_url: registry_url.to_string(),
    };

    if let Ok(client) = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        let resp = client
            .get(&format!("{}/api/v1/me", registry_url.trim_end_matches('/')))
            .header("Authorization", format!("Bearer {}", token))
            .send();
        match resp {
            Ok(r) if r.status().is_success() => {
                let _ = r.text();
            }
            Ok(r) => {
                eprintln!("⚠️  Token validation failed (HTTP {}), but saving anyway", r.status());
            }
            Err(_) => {
                eprintln!("⚠️  Could not reach registry for validation, but saving token");
            }
        }
    }

    let auth_file = auth_path();
    let auth_dir = auth_file.parent().unwrap();
    fs::create_dir_all(auth_dir).map_err(|e| format!("Cannot create auth dir: {}", e))?;
    let json = serde_json::to_string(&info).map_err(|e| format!("Cannot serialize auth: {}", e))?;
    fs::write(auth_path(), &json).map_err(|e| format!("Cannot write auth: {}", e))?;

    #[cfg(unix)]
    { let _ = std::process::Command::new("chmod").args(["600", &auth_path().to_string_lossy()]).status(); }

    println!("✓ Authenticated as '{}' on {}", username, registry_url);
    Ok(info)
}

pub fn logout() -> Result<(), String> {
    let path = auth_path();
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Cannot remove auth file: {}", e))?;
        println!("✓ Logged out");
    } else {
        println!("ℹ️  Not logged in");
    }
    Ok(())
}

pub fn read_auth() -> Option<AuthInfo> {
    let path = auth_path();
    if !path.exists() {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn get_auth_token(registry_url: &str) -> Option<String> {
    let auth = read_auth()?;
    if auth.registry_url == registry_url || registry_url.is_empty() {
        Some(auth.token)
    } else {
        None
    }
}
