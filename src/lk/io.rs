use std::fs::File;
use std::path::Path;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use rand::Rng;
use sha2::{Digest, Sha256};

use super::schema::LkRoot;
use super::LkError;

pub fn read_lk_file(path: &Path) -> Result<LkRoot, LkError> {
    let file = File::open(path)?;
    let decoder = GzDecoder::new(file);
    let root: LkRoot = serde_json::from_reader(decoder)?;
    Ok(root)
}

pub fn write_lk_file(path: &Path, root: &LkRoot) -> Result<(), LkError> {
    let file = File::create(path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    serde_json::to_writer(encoder, root)?;
    Ok(())
}

/// Compute SHA-256 hash of the compact JSON serialization of the resources array.
pub fn compute_hash(root: &LkRoot) -> String {
    let json = serde_json::to_string(&root.resources).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate an 8-character lowercase alphanumeric ID matching Legend Keeper's format.
pub fn generate_id() -> String {
    let chars: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::rng();
    (0..8)
        .map(|_| chars[rng.random_range(0..chars.len())] as char)
        .collect()
}
