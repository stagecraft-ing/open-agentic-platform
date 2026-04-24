// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! XLSX extractor — backed by `calamine`.
//!
//! Each worksheet renders as:
//!
//! ```text
//! ===== SHEET: {name} =====
//! (rows={N}, cols={M})
//! <tab-separated values row-by-row; trailing empty cells trimmed>
//! ```
//!
//! Matches the Python script's `extract_xlsx`:
//! - Newlines within a cell become ` | `
//! - Tabs and carriage returns are replaced with single spaces
//! - `None` values render as empty strings
//! - Trailing empty cells are dropped

use anyhow::{Context, Result};
use calamine::{Data, Reader, Xlsx, open_workbook};
use std::path::Path;

pub fn extract(path: &Path) -> Result<String> {
    let mut workbook: Xlsx<_> =
        open_workbook(path).with_context(|| format!("open xlsx: {}", path.display()))?;

    let mut parts: Vec<String> = Vec::new();

    // Preserve the workbook's sheet order.
    let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
    for sheet_name in sheet_names {
        parts.push(format!("\n===== SHEET: {sheet_name} ====="));
        let range = match workbook.worksheet_range(&sheet_name) {
            Ok(r) => r,
            Err(e) => {
                parts.push(format!("(could not open sheet: {e})"));
                continue;
            }
        };
        let (rows, cols) = range.get_size();
        parts.push(format!("(rows={rows}, cols={cols})"));
        for row in range.rows() {
            let mut cells: Vec<String> = row.iter().map(cell_to_string).collect();
            while cells.last().is_some_and(|s| s.is_empty()) {
                cells.pop();
            }
            parts.push(cells.join("\t"));
        }
    }

    Ok(parts.join("\n"))
}

fn cell_to_string(cell: &Data) -> String {
    let raw = match cell {
        Data::Empty => return String::new(),
        Data::String(s) => s.clone(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => {
            if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{:.0}", f)
            } else {
                f.to_string()
            }
        }
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::Error(e) => format!("#{:?}", e),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
    };
    raw.replace(['\t', '\r'], " ").replace('\n', " | ")
}
