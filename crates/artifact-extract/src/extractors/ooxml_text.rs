// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Shared OOXML helpers used by DOCX and PPTX extractors.
//!
//! DOCX and PPTX are both ZIP archives of XML files. Both use the same
//! `<w:t>` / `<a:t>` text-element pattern for paragraph content — the only
//! difference is the namespace. These helpers read a file out of a zip, scan
//! for text nodes, and collect plain strings.

use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::io::{Read, Seek};

/// Read the full bytes of an archive entry into memory.
pub fn read_entry_bytes<R: Read + Seek>(
    archive: &mut ::zip::ZipArchive<R>,
    name: &str,
) -> Result<Vec<u8>> {
    let mut file = archive
        .by_name(name)
        .with_context(|| format!("archive entry missing: {name}"))?;
    let mut buf = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut buf)
        .with_context(|| format!("read archive entry: {name}"))?;
    Ok(buf)
}

/// Collect all `<w:t>`/`<a:t>`-style text nodes into a vector of plain
/// strings. Ignores empty strings. Works for DOCX and PPTX regardless of
/// namespace — we match by local name.
pub fn collect_text_nodes(xml: &[u8]) -> Result<Vec<String>> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut out: Vec<String> = Vec::new();
    let mut in_text = false;
    let mut current = String::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(ref e) if is_text_element(e.local_name().as_ref()) => {
                in_text = true;
                current.clear();
            }
            Event::End(ref e) if is_text_element(e.local_name().as_ref()) => {
                if in_text && !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
                in_text = false;
            }
            Event::Empty(ref e) if is_text_element(e.local_name().as_ref()) => {
                // Some producers emit self-closed empty <w:t/> — nothing to collect.
            }
            Event::Text(ref t) if in_text => {
                let chunk = t
                    .unescape()
                    .map(|s| s.into_owned())
                    .unwrap_or_else(|_| String::from_utf8_lossy(t.as_ref()).into_owned());
                current.push_str(&chunk);
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(out)
}

fn is_text_element(local: &[u8]) -> bool {
    // DOCX uses `w:t`; PPTX uses `a:t`. We match by local name only.
    local == b"t"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_docx_style_text_runs() {
        let xml = br#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body><w:p><w:r><w:t>Hello</w:t></w:r><w:r><w:t xml:space="preserve"> World</w:t></w:r></w:p></w:body>
        </w:document>"#;
        let out = collect_text_nodes(xml).unwrap();
        assert_eq!(out, vec!["Hello".to_string(), " World".to_string()]);
    }

    #[test]
    fn ignores_non_t_elements() {
        let xml = br#"<root><x>skip</x><w:t xmlns:w="w">keep</w:t></root>"#;
        let out = collect_text_nodes(xml).unwrap();
        assert_eq!(out, vec!["keep".to_string()]);
    }

    #[test]
    fn survives_self_closed_empty_text() {
        let xml = br#"<root><w:t xmlns:w="w"/><w:t xmlns:w="w">after</w:t></root>"#;
        let out = collect_text_nodes(xml).unwrap();
        assert_eq!(out, vec!["after".to_string()]);
    }
}
