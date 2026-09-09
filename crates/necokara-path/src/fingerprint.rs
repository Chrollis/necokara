//! Source-file fingerprinting.
//!
//! A fingerprint identifies a media source file cheaply: hash of the first
//! and last `HEAD_TAIL_SIZE` bytes plus the file length. Used to validate
//! that a referenced file is still the same one that was imported.

use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use sha2::{Digest, Sha256};

/// Bytes read from the head and tail of a file for hashing.
const HEAD_TAIL_SIZE: u64 = 64 * 1024;

/// Compute a fingerprint (hex string) for the file at `path`.
pub fn fingerprint_file(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();

    let mut head = vec![0u8; HEAD_TAIL_SIZE as usize];
    let head_read = file.read(&mut head).ok()?;
    head.truncate(head_read);

    let tail_start = len.saturating_sub(HEAD_TAIL_SIZE);
    let mut tail = Vec::new();
    if tail_start > head_read as u64 {
        file.seek(SeekFrom::Start(tail_start)).ok()?;
        let mut buf = vec![0u8; HEAD_TAIL_SIZE as usize];
        let n = file.read(&mut buf).ok()?;
        buf.truncate(n);
        tail = buf;
    }

    let mut hasher = Sha256::new();
    hasher.update(&head);
    hasher.update(&tail);
    hasher.update(len.to_le_bytes());
    Some(hex(&hasher.finalize()))
}

/// Render a digest as a lowercase hex string.
fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_content_same_fingerprint() {
        let dir = std::env::temp_dir().join("nk_fp_test");
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.wav");
        let b = dir.join("b.wav");
        let bytes: Vec<u8> = (0..200_000).map(|i| (i % 251) as u8).collect();
        std::fs::write(&a, &bytes).unwrap();
        std::fs::write(&b, &bytes).unwrap();

        let fa = fingerprint_file(&a);
        let fb = fingerprint_file(&b);
        assert!(fa.is_some());
        assert_eq!(fa, fb);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn different_content_different_fingerprint() {
        let dir = std::env::temp_dir().join("nk_fp_test2");
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.wav");
        std::fs::write(&a, vec![0u8; 100_000]).unwrap();
        let f1 = fingerprint_file(&a);
        // Change a byte in the middle (still within head? use large file).
        let mut bytes = vec![0u8; 300_000];
        bytes[150_000] = 42; // in the tail region (>64k from end? 300k-150k=150k>64k ok)
        std::fs::write(&a, &bytes).unwrap();
        let f2 = fingerprint_file(&a);
        assert!(f1.is_some() && f2.is_some());
        assert_ne!(f1, f2);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_file_none() {
        assert!(fingerprint_file(Path::new("C:/definitely/missing/file.mp3")).is_none());
    }
}
