// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Extension → extractor dispatch table.
//!
//! Each entry pairs a human label with a function pointer returning the
//! extracted body text as a single `String`. The label is written into the
//! extracted file's header.
//!
//! `.zip` is *not* in this table — ZIPs are handled by the caller because
//! they recurse into a different destination layout than single-file inputs.

use std::path::Path;

use anyhow::Result;

pub struct DispatchEntry {
    pub label: &'static str,
    pub extract: fn(&Path) -> Result<String>,
}

pub fn lookup(ext_dot_lower: &str) -> Option<&'static DispatchEntry> {
    TABLE.iter().find(|e| e.ext == ext_dot_lower).map(|e| &e.entry)
}

pub fn is_supported(ext_dot_lower: &str) -> bool {
    TABLE.iter().any(|e| e.ext == ext_dot_lower)
}

struct Row {
    ext: &'static str,
    entry: DispatchEntry,
}

// The dispatch table. Order is display order only — lookups are linear but
// the table is tiny, so this is simpler than a HashMap.
static TABLE: &[Row] = &[
    Row {
        ext: ".docx",
        entry: DispatchEntry {
            label: "Microsoft Word Document",
            extract: crate::extractors::docx::extract,
        },
    },
    Row {
        ext: ".pptx",
        entry: DispatchEntry {
            label: "Microsoft PowerPoint Presentation",
            extract: crate::extractors::pptx::extract,
        },
    },
    Row {
        ext: ".xlsx",
        entry: DispatchEntry {
            label: "Microsoft Excel Spreadsheet",
            extract: crate::extractors::xlsx::extract,
        },
    },
    Row {
        ext: ".pdf",
        entry: DispatchEntry {
            label: "PDF Document",
            extract: crate::extractors::pdf::extract,
        },
    },
    Row {
        ext: ".json",
        entry: DispatchEntry {
            label: "JSON Data File",
            extract: crate::extractors::json::extract,
        },
    },
    Row {
        ext: ".pbix",
        entry: DispatchEntry {
            label: "Power BI Report",
            extract: crate::extractors::pbix::extract,
        },
    },
];
