use std::fs;
use std::path::Path;

use super::signing::read_signature;
use super::auth::read_auth;

fn create_tarball(source_dir: &Path, dest_path: &Path) -> Result<(), String> {
    let file = fs::File::create(dest_path).map_err(|e| format!("Cannot create tarball: {}", e))?;
    let encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut archive = tar::Builder::new(encoder);

    fn add_dir(archive: &mut tar::Builder<flate2::write::GzEncoder<fs::File>>,
               dir: &Path, prefix: &Path) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("Cannot read dir: {}", e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let rel = path.strip_prefix(prefix).unwrap_or(&path);
            if path.is_dir() {
                add_dir(archive, &path, prefix)?;
            } else if path.extension().map(|e| e == "ajb" || e == "das" || e == "c" || e == "h").unwrap_or(false) {
                let data = fs::read(&path).map_err(|e| format!("Cannot read {:?}: {}", path, e))?;
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                archive.append_data(&mut header, rel, data.as_slice())
                    .map_err(|e| format!("Cannot add to archive: {}", e))?;
            }
        }
        Ok(())
    }

    add_dir(&mut archive, source_dir, source_dir)?;
    archive.finish().map_err(|e| format!("Cannot finalize archive: {}", e))?;
    Ok(())
}

pub fn publish_to_remote(
    name: &str, version: &str, author: &str, description: &str,
    source_dir: &Path, registry_url: &str, checksum: &str,
) -> Result<(), String> {
    let url = registry_url.trim_end_matches('/');

    let token = read_auth().and_then(|a| {
        if a.registry_url == registry_url || a.registry_url == url {
            Some(a.token)
        } else {
            None
        }
    });

    let sig = read_signature(name, version);
    let signature = sig.as_ref().map(|s| {
        serde_json::to_string(s).unwrap_or_default()
    });

    let tarball_path = source_dir.join(format!("{}-{}.tar.gz", name, version));
    create_tarball(source_dir, &tarball_path)?;

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("Cannot create HTTP client: {}", e))?;

    let meta_body = serde_json::json!({
        "name": name,
        "version": version,
        "author": author,
        "description": description,
        "checksum": checksum,
        "signature": signature,
    });

    let mut req = client.post(&format!("{}/api/v1/packages", url))
        .json(&meta_body);

    if let Some(ref t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let resp = req.send().map_err(|e| format!("Cannot publish to registry: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().unwrap_or_default();
        return Err(format!("Registry rejected publish (HTTP {}): {}", status, text));
    }
    println!("✓ Metadata published to registry");

    let tarball_data = fs::read(&tarball_path)
        .map_err(|e| format!("Cannot read tarball: {}", e))?;

    let mut req = client.post(&format!("{}/api/v1/packages/{}/{}/upload", url, name, version))
        .header("content-type", "application/gzip")
        .body(tarball_data);

    if let Some(ref t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
    }

    let resp = req.send().map_err(|e| format!("Cannot upload tarball: {}", e))?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().unwrap_or_default();
        return Err(format!("Registry rejected upload (HTTP {}): {}", status, text));
    }
    println!("✓ Tarball uploaded to registry");

    let _ = fs::remove_file(&tarball_path);

    Ok(())
}
