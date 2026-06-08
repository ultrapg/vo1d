use anyhow::{bail, Context, Result};
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
/// Returns an error with both hashes on mismatch for debugging.
pub fn verify_sha256(path: &Path, expected: &str) -> Result<bool> {
    let actual = sha256_file(path)?;
    if actual == expected {
        Ok(true)
    } else {
        Err(anyhow::anyhow!(
            "SHA256 mismatch for '{}':\n  expected: {}\n  actual:   {}",
            path.display(), expected, actual
        ))
    }
}

/// Streaming SHA256 hasher for incremental verification during download.
pub struct StreamingSha256 {
    hasher: Sha256,
}

impl StreamingSha256 {
    pub fn new() -> Self {
        Self { hasher: Sha256::new() }
    }

    /// Feed a chunk of data into the hash computation.
    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    /// Finalize and return the hex-encoded hash.
    pub fn finalize(self) -> String {
        hex::encode(self.hasher.finalize())
    }

    /// Verify the accumulated hash against an expected value.
    /// Returns descriptive error on mismatch.
    pub fn verify(self, expected: &str) -> Result<()> {
        let actual = self.finalize();
        if actual == expected {
            Ok(())
        } else {
            bail!(
                "SHA256 checksum mismatch:\n  expected: {}\n  actual:   {}\n\
                 The downloaded file may be corrupted or incomplete.",
                expected, actual,
            )
        }
    }
}
