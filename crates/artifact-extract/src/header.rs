// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Fixed artifact-header format.
//!
//! Every extracted `.txt` file starts with this header block so downstream
//! stages can verify the provenance of the derived text without re-stat'ing
//! the raw source. The format is a byte-for-byte port of the Python
//! `header()` function in `extract_artifacts.py` — the only intentional
//! change is that timestamps are serialised with microsecond precision (Rust
//! default) instead of Python's tuneable precision. Both parse as RFC 3339.

use std::time::SystemTime;

use chrono::{DateTime, SecondsFormat, Utc};

/// Render the header block for a single extracted file.
///
/// * `source_file` — POSIX-formatted path relative to `raw/`.
/// * `file_type`   — human label from the dispatcher table (`"Microsoft Word Document"`).
/// * `source_size` — size of the raw file in bytes.
/// * `source_mtime` — mtime of the raw file.
/// * `archive_entry` — when the source is an entry inside a ZIP, the entry's name within the archive.
pub fn write_header(
    source_file: &str,
    file_type: &str,
    source_size: u64,
    source_mtime: SystemTime,
    archive_entry: Option<&str>,
) -> String {
    let src_mtime: DateTime<Utc> = source_mtime.into();
    let now: DateTime<Utc> = Utc::now();

    let mut out = String::with_capacity(256);
    out.push_str("=== EXTRACTED ARTIFACT ===\n");
    out.push_str(&format!("Source file: {source_file}\n"));
    out.push_str(&format!("File type: {file_type}\n"));
    out.push_str(&format!("Source size (bytes): {source_size}\n"));
    out.push_str(&format!(
        "Source mtime: {}\n",
        src_mtime.to_rfc3339_opts(SecondsFormat::AutoSi, true)
    ));
    out.push_str(&format!(
        "Extraction date: {}\n",
        now.to_rfc3339_opts(SecondsFormat::AutoSi, true)
    ));
    if let Some(entry) = archive_entry {
        out.push_str(&format!("Archive entry: {entry}\n"));
    }
    out.push_str("==========================\n\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn header_contains_all_required_lines() {
        let h = write_header(
            "a/b.docx",
            "Microsoft Word Document",
            12_345,
            UNIX_EPOCH + Duration::from_secs(1_700_000_000),
            None,
        );
        assert!(h.starts_with("=== EXTRACTED ARTIFACT ===\n"));
        assert!(h.contains("Source file: a/b.docx\n"));
        assert!(h.contains("File type: Microsoft Word Document\n"));
        assert!(h.contains("Source size (bytes): 12345\n"));
        assert!(h.contains("Source mtime: 2023-11-14T"));
        assert!(h.contains("Extraction date: "));
        assert!(h.ends_with("==========================\n\n"));
        assert!(!h.contains("Archive entry:"));
    }

    #[test]
    fn header_includes_archive_entry_when_given() {
        let h = write_header(
            "bundle.zip",
            "Microsoft Word Document",
            100,
            UNIX_EPOCH,
            Some("inner/doc.docx"),
        );
        assert!(h.contains("Archive entry: inner/doc.docx\n"));
    }
}
