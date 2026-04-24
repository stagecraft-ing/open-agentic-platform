// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! PPTX extractor.
//!
//! PPTX is OOXML: a ZIP archive with `ppt/slides/slide{N}.xml` per slide
//! and `ppt/notesSlides/notesSlide{N}.xml` for speaker notes. Each slide
//! XML embeds the slide title, text frames (with paragraphs and runs),
//! and optional tables.
//!
//! Rendering rules (match the Python reference):
//!
//! 1. For each slide, emit `===== SLIDE N =====`.
//! 2. If the slide has a title text frame, emit `# TITLE: {text}`.
//! 3. Emit every non-title paragraph line in order of appearance.
//! 4. For any table shape, emit `--- SLIDE TABLE ---` ... rows ... `--- END SLIDE TABLE ---`.
//! 5. If notes exist, emit `--- NOTES ---` ... text ... `--- END NOTES ---`.
//!
//! Heuristics: we detect the title shape by looking for `<p:ph type="title">`
//! or `<p:ph type="ctrTitle">` in the shape properties; all other text
//! frames are treated as body content.

use anyhow::{Context, Result};
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;
use std::fs::File;
use std::path::Path;

pub fn extract(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut archive = ::zip::ZipArchive::new(file)
        .with_context(|| format!("not a valid pptx archive: {}", path.display()))?;

    let mut slide_names: Vec<String> = archive
        .file_names()
        .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
        .map(|s| s.to_string())
        .collect();
    slide_names.sort_by_key(|n| slide_index(n));

    let mut parts: Vec<String> = Vec::new();
    for (i, name) in slide_names.iter().enumerate() {
        let idx = i + 1;
        parts.push(format!("\n===== SLIDE {idx} ====="));
        let xml = super::ooxml_text::read_entry_bytes(&mut archive, name)?;
        let rendered = render_slide(&xml)?;
        if let Some(title) = rendered.title {
            parts.push(format!("# TITLE: {title}"));
        }
        parts.extend(rendered.body_lines);
        for table in rendered.tables {
            parts.push("--- SLIDE TABLE ---".to_string());
            for row in table {
                parts.push(row.join(" | "));
            }
            parts.push("--- END SLIDE TABLE ---".to_string());
        }
        // Look for matching notesSlide and emit if present.
        let notes_name = format!("ppt/notesSlides/notesSlide{idx}.xml");
        if archive.file_names().any(|n| n == notes_name) {
            let notes_xml = super::ooxml_text::read_entry_bytes(&mut archive, &notes_name)?;
            if let Some(notes) = render_notes(&notes_xml)? {
                parts.push("--- NOTES ---".to_string());
                parts.push(notes);
                parts.push("--- END NOTES ---".to_string());
            }
        }
    }

    Ok(parts.join("\n"))
}

fn slide_index(name: &str) -> u32 {
    // ppt/slides/slide{N}.xml → N
    let base = name.rsplit('/').next().unwrap_or(name);
    let stripped = base.strip_prefix("slide").unwrap_or(base);
    let digits: String = stripped.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().unwrap_or(u32::MAX)
}

struct SlideRender {
    title: Option<String>,
    body_lines: Vec<String>,
    tables: Vec<Vec<Vec<String>>>,
}

fn render_slide(xml: &[u8]) -> Result<SlideRender> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();

    let mut title: Option<String> = None;
    let mut body_lines: Vec<String> = Vec::new();
    let mut tables: Vec<Vec<Vec<String>>> = Vec::new();

    // Shape scope
    let mut shape_depth = 0_i32;
    let mut shape_is_title = false;
    let mut collected_shape_paragraphs: Vec<String> = Vec::new();

    // Table scope
    let mut in_tbl = false;
    let mut current_rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut in_tc = false;
    let mut tc_paragraphs: Vec<String> = Vec::new();

    // Paragraph scope (in_p tracked implicitly via p_text lifecycle)
    let mut p_text = String::new();

    // Text scope
    let mut in_t = false;
    let mut t_buf = String::new();

    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(ref e) => match e.local_name().as_ref() {
                b"sp" => {
                    shape_depth += 1;
                    shape_is_title = false;
                    collected_shape_paragraphs.clear();
                }
                b"ph" => {
                    // Placeholder — type="title" or "ctrTitle" marks the title shape.
                    if let Some(t) = attr_val(e, b"type")
                        && (t == "title" || t == "ctrTitle")
                    {
                        shape_is_title = true;
                    }
                }
                b"tbl" => {
                    in_tbl = true;
                    current_rows.clear();
                }
                b"tr" if in_tbl => {
                    current_row.clear();
                }
                b"tc" if in_tbl => {
                    in_tc = true;
                    tc_paragraphs.clear();
                }
                b"p" => {
                    p_text.clear();
                }
                b"t" => {
                    in_t = true;
                    t_buf.clear();
                }
                _ => {}
            },
            Event::Empty(ref e) => {
                if e.local_name().as_ref() == b"ph"
                    && let Some(t) = attr_val(e, b"type")
                    && (t == "title" || t == "ctrTitle")
                {
                    shape_is_title = true;
                }
            }
            Event::End(ref e) => match e.local_name().as_ref() {
                b"t" => {
                    in_t = false;
                    p_text.push_str(&t_buf);
                    t_buf.clear();
                }
                b"p" => {
                    let line = p_text.trim().to_string();
                    if in_tc {
                        tc_paragraphs.push(line);
                    } else if !in_tbl && !line.is_empty() {
                        collected_shape_paragraphs.push(line);
                    }
                    p_text.clear();
                }
                b"tc" if in_tbl => {
                    in_tc = false;
                    let joined = tc_paragraphs
                        .iter()
                        .map(|s| s.replace('\n', " | "))
                        .collect::<Vec<_>>()
                        .join(" | ");
                    current_row.push(joined.trim().to_string());
                    tc_paragraphs.clear();
                }
                b"tr" if in_tbl => {
                    current_rows.push(std::mem::take(&mut current_row));
                }
                b"tbl" => {
                    in_tbl = false;
                    tables.push(std::mem::take(&mut current_rows));
                }
                b"sp" => {
                    shape_depth -= 1;
                    if shape_is_title {
                        if title.is_none()
                            && let Some(first) = collected_shape_paragraphs.first()
                        {
                            title = Some(first.clone());
                        }
                        // Include any remaining title paragraphs as body too
                        for line in collected_shape_paragraphs.iter().skip(1) {
                            body_lines.push(line.clone());
                        }
                    } else {
                        body_lines.extend(std::mem::take(&mut collected_shape_paragraphs));
                    }
                    collected_shape_paragraphs.clear();
                    shape_is_title = false;
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

    // Drop unused shape tracker.
    let _ = shape_depth;

    Ok(SlideRender {
        title,
        body_lines,
        tables,
    })
}

fn render_notes(xml: &[u8]) -> Result<Option<String>> {
    let nodes = super::ooxml_text::collect_text_nodes(xml)?;
    if nodes.is_empty() {
        return Ok(None);
    }
    let text = nodes.join("\n").trim().to_string();
    if text.is_empty() { Ok(None) } else { Ok(Some(text)) }
}

fn attr_val(e: &BytesStart, name: &[u8]) -> Option<String> {
    for attr in e.attributes().flatten() {
        if attr.key.local_name().as_ref() == name {
            return Some(
                attr.unescape_value()
                    .unwrap_or_else(|_| String::from_utf8_lossy(&attr.value).into_owned().into())
                    .to_string(),
            );
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLIDE_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <p:cSld><p:spTree>
    <p:sp>
      <p:nvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr>
      <p:txBody><a:p><a:r><a:t>Quarterly Review</a:t></a:r></a:p></p:txBody>
    </p:sp>
    <p:sp>
      <p:txBody>
        <a:p><a:r><a:t>First bullet</a:t></a:r></a:p>
        <a:p><a:r><a:t>Second bullet</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>
    <p:graphicFrame>
      <a:tbl>
        <a:tr>
          <a:tc><a:txBody><a:p><a:r><a:t>H1</a:t></a:r></a:p></a:txBody></a:tc>
          <a:tc><a:txBody><a:p><a:r><a:t>H2</a:t></a:r></a:p></a:txBody></a:tc>
        </a:tr>
        <a:tr>
          <a:tc><a:txBody><a:p><a:r><a:t>v1</a:t></a:r></a:p></a:txBody></a:tc>
          <a:tc><a:txBody><a:p><a:r><a:t>v2</a:t></a:r></a:p></a:txBody></a:tc>
        </a:tr>
      </a:tbl>
    </p:graphicFrame>
  </p:spTree></p:cSld>
</p:sld>"#;

    #[test]
    fn slide_renders_title_body_and_table() {
        let r = render_slide(SLIDE_XML).unwrap();
        assert_eq!(r.title, Some("Quarterly Review".to_string()));
        assert_eq!(
            r.body_lines,
            vec!["First bullet".to_string(), "Second bullet".to_string()]
        );
        assert_eq!(r.tables.len(), 1);
        assert_eq!(r.tables[0][0], vec!["H1".to_string(), "H2".to_string()]);
        assert_eq!(r.tables[0][1], vec!["v1".to_string(), "v2".to_string()]);
    }
}
