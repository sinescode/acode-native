//! SHA-256 file hashing with SIMD acceleration.
//!
//! Provides fast checksum computation for files — useful for verifying
//! plugin downloads, detecting file changes, and integrity checks.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

fn digest_to_hex(digest: &[u8]) -> String {
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Result of hashing a file or buffer.
#[derive(Debug, Clone, Serialize)]
pub struct HashResult {
    /// Hex-encoded SHA-256 digest (lowercase)
    pub hex: String,
    /// Raw bytes of the digest
    pub bytes: Vec<u8>,
    /// Size of the input in bytes
    pub input_size: u64,
}

// ---------------------------------------------------------------------------
// File hashing
// ---------------------------------------------------------------------------

/// Compute the SHA-256 hash of a file at the given path.
///
/// Uses a 64KB buffer for streaming reads, avoiding memory pressure
/// even for large files.
pub fn hash_file(path: &Path) -> Result<HashResult, String> {
    let mut file = File::open(path)
        .map_err(|e| format!("Failed to open file: {}", e))?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 65536]; // 64 KB buffer
    let mut total_bytes: u64 = 0;

    loop {
        let bytes_read = file.read(&mut buffer)
            .map_err(|e| format!("Failed to read file: {}", e))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
        total_bytes += bytes_read as u64;
    }

    let digest = hasher.finalize();
    let hex = digest_to_hex(&digest);

    Ok(HashResult {
        hex,
        bytes: digest.to_vec(),
        input_size: total_bytes,
    })
}

/// Compute the SHA-256 hash of a byte buffer.
pub fn hash_bytes(data: &[u8]) -> HashResult {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();

    HashResult {
        hex: digest_to_hex(&digest),
        bytes: digest.to_vec(),
        input_size: data.len() as u64,
    }
}

/// Compute the SHA-256 hash of a string.
pub fn hash_string(s: &str) -> HashResult {
    hash_bytes(s.as_bytes())
}

/// Verify a file against an expected hex digest.
/// Returns `Ok(true)` if the hash matches, `Ok(false)` if it doesn't.
pub fn verify_file(path: &Path, expected_hex: &str) -> Result<bool, String> {
    let result = hash_file(path)?;
    Ok(result.hex.eq_ignore_ascii_case(expected_hex))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_hash_known_string() {
        // SHA-256("hello world") is known
        let result = hash_string("hello world");
        assert_eq!(
            result.hex,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
        assert_eq!(result.input_size, 11);
    }

    #[test]
    fn test_hash_bytes_empty() {
        let result = hash_bytes(b"");
        assert_eq!(
            result.hex,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(result.input_size, 0);
    }

    #[test]
    fn test_hash_file() {
        let mut tmp = std::env::temp_dir();
        tmp.push("acode_hash_test.txt");
        let mut f = std::fs::File::create(&tmp).unwrap();
        f.write_all(b"test content").unwrap();

        let result = hash_file(&tmp).unwrap();
        assert_eq!(result.hex.len(), 64);
        assert_eq!(result.input_size, 12);

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_verify_file() {
        let mut tmp = std::env::temp_dir();
        tmp.push("acode_verify_test.txt");
        let mut f = std::fs::File::create(&tmp).unwrap();
        f.write_all(b"verify me").unwrap();

        let result = hash_file(&tmp).unwrap();
        assert!(verify_file(&tmp, &result.hex).unwrap());
        assert!(!verify_file(&tmp, "deadbeef").unwrap());

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_hash_deterministic() {
        let a = hash_string("same input");
        let b = hash_string("same input");
        assert_eq!(a.hex, b.hex);
    }

    #[test]
    fn test_hash_different() {
        let a = hash_string("input A");
        let b = hash_string("input B");
        assert_ne!(a.hex, b.hex);
    }
}
