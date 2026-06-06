use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

/// Compute SHA256 hash of a file.
pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Failed to open file for hashing: {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer)
            .context("Failed to read file during hashing")?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Compute SHA256 hash of a byte slice.
pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Verify a file's SHA256 hash against an expected value.
pub fn verify_sha256(path: &Path, expected: &str) -> Result<bool> {
    let actual = sha256_file(path)?;
    Ok(actual == expected)
}
