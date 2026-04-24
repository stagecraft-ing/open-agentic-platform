// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Plain-text passthrough extractor.
//!
//! Markdown, plaintext, and CSV are already text on disk — there is nothing
//! to "extract". Emitting an empty body would hide the content from every
//! downstream classifier and embedder, so the body of the derived `.txt`
//! is simply the file's UTF-8 contents.
//!
//! Non-UTF-8 bytes are replaced with U+FFFD rather than failing, matching
//! how we treat heterogeneous knowledge uploads elsewhere in the pipeline.

use anyhow::Result;
use std::path::Path;

pub fn extract(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn passes_utf8_through_verbatim() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "# Heading\n\nbody — café").unwrap();
        let out = extract(tmp.path()).unwrap();
        assert_eq!(out, "# Heading\n\nbody — café");
    }

    #[test]
    fn replaces_invalid_utf8_instead_of_erroring() {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(&[0x68, 0x69, 0xff, 0xfe]).unwrap();
        let out = extract(tmp.path()).unwrap();
        assert!(out.starts_with("hi"));
        assert!(out.contains('\u{FFFD}'));
    }
}
