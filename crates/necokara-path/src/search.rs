//! Offline search: when a material's file can no longer be resolved (it was
//! moved or renamed), search candidate directories.
//!
//! Two match channels are produced:
//!  - by fingerprint (content hash) — the reliable "same file" match,
//!    searched over files with the same extension in the candidate roots;
//!  - by file name — a cheaper fallback.
//! Callers prefer `by_fp` when non-empty.

use std::path::{Path, PathBuf};

use crate::fingerprint::fingerprint_file;

/// The two match channels from an offline search.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchResult {
    /// Files whose fingerprint matches (preferred).
    pub by_fp: Vec<PathBuf>,
    /// Files whose name matches.
    pub by_name: Vec<PathBuf>,
}

impl SearchResult {
    /// The first result, preferring `by_fp` over `by_name`.
    pub fn first(&self) -> Option<&PathBuf> {
        self.by_fp
            .first()
            .or_else(|| self.by_name.first())
    }

    /// Whether no candidate was found in either channel.
    pub fn is_empty(&self) -> bool {
        self.by_fp.is_empty() && self.by_name.is_empty()
    }
}

/// Find files named `file_name` inside each candidate directory
/// (non-recursive). The same physical file reached through overlapping
/// candidates is reported once.
pub fn find_by_name_many(file_name: &str, candidates: &[PathBuf]) -> Vec<PathBuf> {
    let mut hits = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for dir in candidates {
        let hit = dir.join(file_name);
        if !hit.is_file() {
            continue;
        }
        // Canonicalize to deduplicate overlapping candidate directories.
        let key = std::fs::canonicalize(&hit).unwrap_or(hit.clone());
        if seen.insert(key) {
            hits.push(hit);
        }
    }
    hits
}

/// Find files with the given extension inside each candidate directory
/// (non-recursive) whose fingerprint matches `fingerprint`.
pub fn find_by_ext_and_fp(
    candidates: &[PathBuf],
    ext: &str,
    fingerprint: &str,
) -> Vec<PathBuf> {
    let mut hits = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for dir in candidates {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            // Extension match (case-insensitive).
            let matches_ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case(ext))
                .unwrap_or(false);
            if !matches_ext {
                continue;
            }
            if fingerprint_file(&path).as_deref() == Some(fingerprint) {
                let key = std::fs::canonicalize(&path).unwrap_or(path.clone());
                if seen.insert(key) {
                    hits.push(path);
                }
            }
        }
    }
    hits
}

/// Candidate search roots given the project root and the original parent
/// directory of the missing file: the original parent and the project root.
pub fn candidate_roots(project_root: &Path, original_parent: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if original_parent.is_dir() {
        v.push(original_parent.to_path_buf());
    }
    v.push(project_root.to_path_buf());
    v
}
