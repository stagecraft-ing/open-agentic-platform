// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! JSON extractor.
//!
//! Parses the file. If parsing succeeds, emits a short summary (list length
//! or top-level keys) followed by the pretty-printed content. If parsing
//! fails, emits the raw file contents — matches the Python reference.

use anyhow::Result;
use serde_json::Value;
use std::path::Path;

pub fn extract(path: &Path) -> Result<String> {
    let raw = std::fs::read_to_string(path)?;
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return Ok(raw);
    };

    let mut parts: Vec<String> = Vec::new();
    match &value {
        Value::Array(arr) => {
            parts.push(format!("[JSON summary: list length = {}]", arr.len()));
            if let Some(first) = arr.first() {
                parts.push("[First element (pretty):]".to_string());
                if let Ok(p) = serde_json::to_string_pretty(first) {
                    parts.push(p);
                }
            }
            parts.push("\n[Full content:]".to_string());
        }
        Value::Object(map) => {
            let keys: Vec<&str> = map.keys().take(20).map(|s| s.as_str()).collect();
            parts.push(format!(
                "[JSON summary: object with {} keys: {:?}]",
                map.len(),
                keys
            ));
            parts.push("\n[Full content:]".to_string());
        }
        _ => {}
    }

    let pretty = serde_json::to_string_pretty(&value)?;
    parts.push(pretty);
    Ok(parts.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn object_summary_then_full_content() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, r#"{{"a":1,"b":[2,3]}}"#).unwrap();
        let out = extract(tmp.path()).unwrap();
        assert!(out.contains("[JSON summary: object with 2 keys"));
        assert!(out.contains("[Full content:]"));
        assert!(out.contains("\"a\": 1"));
    }

    #[test]
    fn array_includes_first_element_pretty() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, r#"[{{"id":1}},{{"id":2}}]"#).unwrap();
        let out = extract(tmp.path()).unwrap();
        assert!(out.contains("[JSON summary: list length = 2]"));
        assert!(out.contains("[First element (pretty):]"));
    }

    #[test]
    fn invalid_json_returns_raw() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "this is not valid json").unwrap();
        let out = extract(tmp.path()).unwrap();
        assert_eq!(out, "this is not valid json");
    }
}
