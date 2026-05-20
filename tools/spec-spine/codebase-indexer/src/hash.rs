//! Content hash computation (SHA-256 over sorted input files).
//!
//! Follows the spec-compiler pattern: `<repo-relative-path>\0<normalized-content>`
//! pairs sorted by path, fed into SHA-256.

use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Compute SHA-256 content hash over all input files.
pub fn compute_content_hash(
    repo_root: &Path,
    input_files: &[PathBuf],
) -> Result<String, std::io::Error> {
    let mut pieces: Vec<(String, Vec<u8>)> = Vec::new();

    for path in input_files {
        if !path.is_file() {
            continue;
        }
        let raw = fs::read_to_string(path)?;
        let normalized = normalize_text(&raw);
        let rel = normalize_repo_path(repo_root, path);
        let mut buf = rel.as_bytes().to_vec();
        buf.push(0);
        buf.extend_from_slice(&normalized);
        pieces.push((rel, buf));
    }

    pieces.sort_by(|a, b| a.0.cmp(&b.0));

    let mut hasher = Sha256::new();
    for (_, buf) in pieces {
        hasher.update(&buf);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Normalize text: strip BOM, convert CRLF/CR to LF.
fn normalize_text(s: &str) -> Vec<u8> {
    let s = s.strip_prefix('\u{feff}').unwrap_or(s);
    let s = s.replace("\r\n", "\n").replace('\r', "\n");
    s.into_bytes()
}

fn normalize_repo_path(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
