pub mod serialize;

use crate::ast::Stmt;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const CACHE_FORMAT_VERSION: u64 = 5;  // Bump when serialization format changes

#[derive(Clone, Debug)]
pub struct CacheEntry {
    pub source_path: PathBuf,
    pub source_mtime: SystemTime,
    pub stmts: Vec<Stmt>,
}

pub struct ModuleCache {
    cache_dir: PathBuf,
    // For tracking source file timestamps
    source_times: Vec<(PathBuf, SystemTime)>,
}

impl ModuleCache {
    pub fn new(cache_dir: PathBuf) -> Self {
        ModuleCache {
            cache_dir,
            source_times: Vec::new(),
        }
    }

    pub fn add_source(&mut self, path: &Path) {
        if let Ok(mtime) = fs::metadata(path).and_then(|m| m.modified()) {
            self.source_times.push((path.to_path_buf(), mtime));
        }
    }

    // Check if cache is valid: all cached mtimes match current file mtimes
    fn validate(&self, hash: u64) -> bool {
        let bin_path = self.cache_dir.join(format!("{:016x}.bin", hash));
        if !bin_path.exists() {
            return false;
        }
        // Read stored mtimes from the .bin file (first section contains mtime data)
        let data = match fs::read(&bin_path) {
            Ok(d) => d,
            Err(_) => return false,
        };
        let mut cursor = std::io::Cursor::new(data.as_slice());
        let stored_count = match read_u64_le(&mut cursor) {
            Some(n) => n as usize,
            None => return false,
        };
        for _ in 0..stored_count {
            let path_len = match read_u64_le(&mut cursor) {
                Some(n) => n as usize,
                None => return false,
            };
            let mut path_bytes = vec![0u8; path_len];
            if cursor.read(&mut path_bytes).ok() != Some(path_len) {
                return false;
            }
            let stored_path = String::from_utf8_lossy(&path_bytes).to_string();
            let stored_mtime_secs = match read_u64_le(&mut cursor) {
                Some(n) => n,
                None => return false,
            };
            let stored_mtime_nanos = match read_u64_le(&mut cursor) {
                Some(n) => n,
                None => return false,
            };
            // Check current mtime
            if let Ok(meta) = fs::metadata(&stored_path) {
                if let Ok(mtime) = meta.modified() {
                    let dur = mtime.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                    if dur.as_secs() != stored_mtime_secs || dur.subsec_nanos() as u64 != stored_mtime_nanos {
                        return false;
                    }
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }

    // Hash based only on the entry (first) source path so the filename is stable across both
    // load and save. Mtime validation for ALL sources is done inside the .bin file.
    fn compute_hash(&self) -> u64 {
        let mut h: u64 = 0xdeadbeefcafe1234;
        if let Some((path, _)) = self.source_times.first() {
            let p = path.to_string_lossy();
            for b in p.bytes() {
                h = h.wrapping_mul(16777619) ^ b as u64;
            }
        }
        h
    }

    pub fn load(&self) -> Option<Vec<Stmt>> {
        let hash = self.compute_hash();
        if !self.validate(hash) {
            return None;
        }
        let cache_path = self.cache_dir.join(format!("{:016x}.bin", hash));
        let data = fs::read(&cache_path).ok()?;
        let mut cursor = std::io::Cursor::new(data.as_slice());

        // Read and check format version
        let version = read_u64_le(&mut cursor)?;
        if version != CACHE_FORMAT_VERSION {
            return None;  // Cache format mismatch, reparse
        }

        // Skip mtime block (we already validated)
        let count = read_u64_le(&mut cursor)? as usize;
        for _ in 0..count {
            let path_len = read_u64_le(&mut cursor)? as usize;
            cursor.set_position(cursor.position() + path_len as u64);
            cursor.set_position(cursor.position() + 16); // skip two u64 mtime fields
        }

        // Read statements
        let stmt_count = read_u64_le(&mut cursor)? as usize;
        let mut stmts = Vec::with_capacity(stmt_count);
        for _ in 0..stmt_count {
            stmts.push(serialize::read_stmt(&mut cursor)?);
        }
        Some(stmts)
    }

    pub fn save(&self, stmts: &[Stmt]) {
        fs::create_dir_all(&self.cache_dir).ok();
        let hash = self.compute_hash();

        let mut data = Vec::new();

        // Write format version (must be first)
        write_u64_le(&mut data, CACHE_FORMAT_VERSION);

        // Write mtime block for validation
        write_u64_le(&mut data, self.source_times.len() as u64);
        for (path, mtime) in &self.source_times {
            let p = path.to_string_lossy();
            write_u64_le(&mut data, p.len() as u64);
            data.extend_from_slice(p.as_bytes());
            let dur = mtime.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            write_u64_le(&mut data, dur.as_secs());
            write_u64_le(&mut data, dur.subsec_nanos() as u64);
        }

        // Write statements
        write_u64_le(&mut data, stmts.len() as u64);
        for stmt in stmts {
            serialize::write_stmt(&mut data, stmt);
        }

        // Write to .bin file (contains both mtime block and serialized statements)
        let bin_path = self.cache_dir.join(format!("{:016x}.bin", hash));
        fs::write(&bin_path, &data).ok();
    }
}

// ── Binary Serialization Helpers ─────────────────────────────────

pub(super) fn write_u64_le(data: &mut Vec<u8>, v: u64) {
    data.extend_from_slice(&v.to_le_bytes());
}

pub(super) fn read_u64_le(cursor: &mut std::io::Cursor<&[u8]>) -> Option<u64> {
    let mut buf = [0u8; 8];
    if cursor.read(&mut buf).ok()? != 8 {
        return None;
    }
    Some(u64::from_le_bytes(buf))
}


