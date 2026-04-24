// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! PBIX (Power BI) extractor.
//!
//! PBIX is a ZIP archive with a fixed set of well-known entries. We don't
//! try to interpret the Power BI data model — we extract the human-readable
//! JSON/text layers so a downstream agent can at least see the report
//! layout and the data model schema.

use anyhow::{Context, Result};
use serde_json::Value;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const TARGETS: &[&str] = &[
    "Report/Layout",
    "DataModelSchema",
    "Metadata",
    "Version",
    "Settings",
    "Connections",
];

pub fn extract(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut archive = ::zip::ZipArchive::new(file)
        .with_context(|| format!("not a valid pbix archive: {}", path.display()))?;

    let mut out = String::new();
    out.push_str("[PBIX is a Power BI binary container (Zip). Extracted human-readable parts only.]\n");

    let names: Vec<String> = archive.file_names().map(|s| s.to_string()).collect();
    out.push_str(&format!("\n[Archive entries ({}):]\n", names.len()));
    for n in &names {
        out.push_str(&format!(" - {n}\n"));
    }

    let available: std::collections::HashSet<&str> = names.iter().map(|s| s.as_str()).collect();
    for target in TARGETS {
        if !available.contains(target) {
            continue;
        }
        match read_entry_text(&mut archive, target) {
            Ok(text) => {
                out.push_str(&format!("\n===== {target} =====\n"));
                // Try to reparse as JSON for pretty printing.
                let pretty = match serde_json::from_str::<Value>(&text) {
                    Ok(v) => serde_json::to_string_pretty(&v).unwrap_or(text),
                    Err(_) => text,
                };
                out.push_str(&pretty);
                out.push('\n');
            }
            Err(e) => {
                out.push_str(&format!("\n[Could not read {target}: {e}]\n"));
            }
        }
    }

    Ok(out)
}

fn read_entry_text(
    archive: &mut ::zip::ZipArchive<File>,
    name: &str,
) -> Result<String> {
    let mut f = archive.by_name(name)?;
    let mut buf = Vec::with_capacity(f.size() as usize);
    f.read_to_end(&mut buf)?;

    // Power BI often writes UTF-16LE; fall back to UTF-8 then Latin-1.
    if buf.starts_with(&[0xFF, 0xFE]) {
        // BOM-marked UTF-16LE
        let utf16: Vec<u16> = buf[2..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return Ok(String::from_utf16_lossy(&utf16));
    }
    if buf.len() >= 2 && buf.iter().step_by(2).skip(1).all(|&b| b == 0) {
        // heuristic: UTF-16LE without BOM (every other byte is zero)
        let utf16: Vec<u16> = buf
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        return Ok(String::from_utf16_lossy(&utf16));
    }
    match std::str::from_utf8(&buf) {
        Ok(s) => Ok(s.to_string()),
        Err(_) => Ok(buf.iter().map(|&b| b as char).collect()),
    }
}
