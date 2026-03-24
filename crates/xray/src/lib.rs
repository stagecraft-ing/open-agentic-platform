// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Feature: XRAY_ANALYSIS
// Spec: spec/xray/analysis.md

use anyhow::Result;
use std::path::{Path, PathBuf};

// Export modules so they can be used
pub mod canonical;
pub mod digest;
pub mod docs;
pub mod hash;
pub mod language;
pub mod loc;
pub mod schema;
pub mod tools;
pub mod traversal;
pub mod write;

#[cfg(test)]
mod invariant_tests;

// Re-export key structs
pub use docs::DocsGenerator;
pub use schema::XrayIndex;

/// Run a complete scan sequence on a target directory
pub fn scan_target(target: &Path, output: Option<PathBuf>) -> Result<XrayIndex> {
    let repo_slug = target
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // 1. Scan Target
    let scan_result = traversal::scan_target(target)?;

    // 2. Build Index
    let mut index = XrayIndex {
        root: repo_slug.clone(),
        target: target.to_string_lossy().to_string(),
        files: scan_result.files,
        stats: scan_result.stats,
        languages: scan_result.languages,
        top_dirs: scan_result.top_dirs,
        module_files: scan_result.module_files,
        ..Default::default()
    };

    // 3. Compute digest
    let digest_str = digest::calculate_digest(&index)?;
    index.digest = digest_str;

    // 4. Serialize (validation step)
    let bytes = canonical::to_canonical_json(&index)?;

    // 5. Determine output path and write if requested
    if let Some(out_dir) = output {
        let out_file = out_dir.join("index.json");
        write::write_atomic(&out_file, &bytes)?;
        println!("XRAY scan complete. Digest: {}", index.digest);
        println!("Written to: {}", out_file.display());
    }

    Ok(index)
}
