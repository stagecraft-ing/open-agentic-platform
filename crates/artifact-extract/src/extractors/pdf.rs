// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! PDF extractor — backed by `pdf-extract`.
//!
//! `pdf-extract` emits the document's text as a single string. We don't
//! get per-page boundaries or table detection out of the box the way
//! pdfplumber does, so the Rust port is more limited than the Python
//! reference.
//!
//! We emit:
//!
//! ```text
//! [PDF text extracted via pdf-extract; per-page boundaries and table
//!  detection are not available in this Rust port — treat the body as
//!  a single continuous text stream.]
//!
//! <extracted text, or a structural warning if empty>
//! ```
//!
//! When extraction yields no text we emit the same warning header the
//! Python reference used, so downstream consumers can classify empty
//! PDFs consistently across both ports.

use anyhow::{Context, Result};
use std::path::Path;

pub fn extract(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .with_context(|| format!("pdf parse failed: {}", path.display()))?;

    let mut out = String::new();
    out.push_str(
        "[PDF text extracted via pdf-extract; per-page boundaries and table detection are not available in this Rust port — treat the body as a single continuous text stream.]\n\n",
    );

    if text.trim().is_empty() {
        out.push_str(
            "[WARNING: no text could be extracted from this PDF. Likely a scanned document, a vector-only diagram with fonts flattened to paths, or an encrypted file. OCR is not possible from this PDF alone; obtain the source document (Word/Visio/etc.) for a text-accurate extraction.]\n",
        );
    } else {
        out.push_str(&text);
    }

    Ok(out)
}
