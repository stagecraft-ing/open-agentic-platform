// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! DOCX extractor.
//!
//! DOCX is OOXML: a ZIP archive with `word/document.xml` as the main
//! content stream. We preserve paragraph-level structure and heading styles,
//! and render tables as pipe-delimited rows bracketed by `--- TABLE N ---`
//! markers. The output layout mirrors the Python reference implementation
//! so cached `.txt` files produced by the legacy script don't need to be
//! regenerated.
//!
//! Structure-aware rendering rules:
//!
//! 1. Walk the document in order. A paragraph outside any table renders
//!    as a line. An empty paragraph renders as a blank line. A paragraph
//!    whose `w:pStyle` starts with `Heading` renders as `## [StyleName] text`
//!    surrounded by blank lines.
//! 2. A table renders as `--- TABLE N ---` header, one line per row with
//!    cells joined by " | ", and `--- END TABLE ---` footer. Cell contents
//!    are the concatenated text of the cell's paragraphs with newlines
//!    collapsed to " | " (matching python-docx's `cell.text`).

use anyhow::{Context, Result};
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use std::fs::File;
use std::path::Path;

pub fn extract(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut archive = ::zip::ZipArchive::new(file)
        .with_context(|| format!("not a valid docx archive: {}", path.display()))?;
    let xml = super::ooxml_text::read_entry_bytes(&mut archive, "word/document.xml")?;
    render(&xml)
}

fn render(xml: &[u8]) -> Result<String> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    let mut parts: Vec<String> = Vec::new();
    let mut table_counter: u32 = 0;

    // Paragraph-scope state (in_p is implicit in the p_text lifecycle)
    let mut p_style: Option<String> = None;
    let mut p_text = String::new();

    // Table-scope state
    let mut in_tbl = false;
    let mut current_table_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut in_tc = false;
    let mut tc_paragraphs: Vec<String> = Vec::new();

    // Text-node scope (within <w:t>)
    let mut in_t = false;
    let mut t_buf = String::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(ref e) => match e.local_name().as_ref() {
                b"tbl" => {
                    in_tbl = true;
                    table_counter += 1;
                    current_table_rows.clear();
                }
                b"tr" if in_tbl => {
                    current_row.clear();
                }
                b"tc" if in_tbl => {
                    in_tc = true;
                    tc_paragraphs.clear();
                }
                b"p" => {
                    p_style = None;
                    p_text.clear();
                }
                b"pStyle" => {
                    if let Some(v) = attr_val(e, b"val") {
                        p_style = Some(v);
                    }
                }
                b"t" => {
                    in_t = true;
                    t_buf.clear();
                }
                _ => {}
            },
            Event::Empty(ref e) => {
                if e.local_name().as_ref() == b"pStyle"
                    && let Some(v) = attr_val(e, b"val")
                {
                    p_style = Some(v);
                }
            }
            Event::End(ref e) => match e.local_name().as_ref() {
                b"t" => {
                    in_t = false;
                    p_text.push_str(&t_buf);
                    t_buf.clear();
                }
                b"p" => {
                    // Commit the paragraph. If inside a <w:tc>, collect; else emit.
                    let rendered = render_paragraph(p_style.as_deref(), &p_text);
                    if in_tc {
                        tc_paragraphs.push(p_text.trim_end().to_string());
                    } else if !in_tbl {
                        parts.push(rendered);
                    }
                    p_style = None;
                    p_text.clear();
                }
                b"tc" if in_tbl => {
                    in_tc = false;
                    // python-docx cell.text: join with " | " after collapsing newlines
                    let joined = tc_paragraphs
                        .iter()
                        .map(|s| s.replace('\n', " | "))
                        .map(|s| s.trim().to_string())
                        .collect::<Vec<_>>()
                        .join(" | ");
                    current_row.push(joined);
                    tc_paragraphs.clear();
                }
                b"tr" if in_tbl => {
                    current_table_rows.push(std::mem::take(&mut current_row));
                }
                b"tbl" => {
                    in_tbl = false;
                    parts.push(format!("\n--- TABLE {} ---", table_counter));
                    for row in &current_table_rows {
                        parts.push(row.join(" | "));
                    }
                    parts.push("--- END TABLE ---\n".to_string());
                    current_table_rows.clear();
                }
                _ => {}
            },
            Event::Text(ref t) if in_t => {
                let chunk = t
                    .unescape()
                    .map(|s| s.into_owned())
                    .unwrap_or_else(|_| String::from_utf8_lossy(t.as_ref()).into_owned());
                t_buf.push_str(&chunk);
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(parts.join("\n"))
}

fn render_paragraph(style: Option<&str>, text: &str) -> String {
    let trimmed = text.trim_end();
    let style = style.unwrap_or("");
    if style.starts_with("Heading") {
        format!("\n## [{}] {}\n", display_style(style), trimmed)
    } else if trimmed.is_empty() {
        String::new()
    } else {
        trimmed.to_string()
    }
}

/// Python-docx exposes style `name` with a space between "Heading" and the
/// level digit (e.g. "Heading 1"), while OOXML's `w:pStyle val` is packed
/// ("Heading1"). We normalise to the spaced form for compatibility with
/// text caches produced by the Python reference implementation.
fn display_style(raw: &str) -> String {
    if let Some(rest) = raw.strip_prefix("Heading")
        && !rest.is_empty()
        && rest.chars().next().unwrap().is_ascii_digit()
    {
        return format!("Heading {rest}");
    }
    raw.to_string()
}

fn attr_val(e: &BytesStart, name: &[u8]) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.local_name().as_ref() == name {
            let s = attr
                .unescape_value()
                .unwrap_or_else(|_| String::from_utf8_lossy(&attr.value).into_owned().into())
                .to_string();
            return Some(s);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Overview</w:t></w:r>
    </w:p>
    <w:p><w:r><w:t>Intro paragraph.</w:t></w:r></w:p>
    <w:p/>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading2"/></w:pPr>
      <w:r><w:t>Details</w:t></w:r>
    </w:p>
    <w:tbl>
      <w:tr>
        <w:tc><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc>
      </w:tr>
      <w:tr>
        <w:tc><w:p><w:r><w:t>1</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>2</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;

    #[test]
    fn renders_headings_and_tables() {
        let out = render(DOC_XML).unwrap();
        assert!(out.contains("## [Heading 1] Overview"));
        assert!(out.contains("Intro paragraph."));
        assert!(out.contains("## [Heading 2] Details"));
        assert!(out.contains("\n--- TABLE 1 ---\n"));
        assert!(out.contains("A | B"));
        assert!(out.contains("1 | 2"));
        assert!(out.contains("--- END TABLE ---"));
    }
}
