//! XML formatting utilities for tool responses.
//!
//! This module provides functions to convert tool responses from JSON to
//! XML-tagged format that is optimized for LLM consumption. The format uses
//! simple XML tags without indentation for a clean, parseable output.
//!
//! # Example
//!
//! ```
//! use serde_json::json;
//! use gitctx::xml_format::to_xml;
//!
//! let value = json!({
//!     "success": true,
//!     "path": "src/main.rs",
//!     "content": "fn main() { }"
//! });
//!
//! let xml = to_xml(&value);
//! // Output:
//! // <success>true</success>
//! // <path>src/main.rs</path>
//! // <content>
//! // fn main() { }
//! // </content>
//! ```

use serde_json::Value;

/// Fields that typically contain multi-line code content.
/// These fields will have their content on a new line after the opening tag.
const CODE_FIELDS: &[&str] = &[
    "content",
    "patch",
    "diff",
    "diff_hunk",
    "body",
];

/// Convert a serde_json::Value to XML-tagged format.
///
/// The output uses simple XML tags without indentation. Code content fields
/// are formatted with the content on a new line for better readability.
///
/// # Arguments
///
/// * `value` - The JSON value to convert (should be an object at the root level)
///
/// # Returns
///
/// XML-formatted string representation of the value.
///
/// # Example
///
/// ```
/// use serde_json::json;
/// use gitctx::xml_format::to_xml;
///
/// let value = json!({"name": "test", "count": 42});
/// let xml = to_xml(&value);
/// assert!(xml.contains("<name>test</name>"));
/// assert!(xml.contains("<count>42</count>"));
/// ```
pub fn to_xml(value: &Value) -> String {
    let mut output = String::new();

    match value {
        Value::Object(obj) => {
            for (key, val) in obj {
                format_value(&mut output, key, val);
            }
        }
        _ => {
            // If root is not an object, wrap it in a response tag
            format_value(&mut output, "response", value);
        }
    }

    output
}

/// Format a single key-value pair as XML.
///
/// Handles different value types appropriately:
/// - Null: outputs empty self-closing tag
/// - Bool/Number: outputs inline value
/// - String: outputs inline or multi-line depending on field name
/// - Array: outputs items with singularized tag names
/// - Object: outputs nested tags
fn format_value(output: &mut String, tag: &str, value: &Value) {
    match value {
        Value::Null => {
            output.push_str(&format!("<{}/>\n", tag));
        }
        Value::Bool(b) => {
            output.push_str(&format!("<{}>{}</{}>\n", tag, b, tag));
        }
        Value::Number(n) => {
            output.push_str(&format!("<{}>{}</{}>\n", tag, n, tag));
        }
        Value::String(s) => {
            if CODE_FIELDS.contains(&tag) && s.contains('\n') {
                // Multi-line code content: put on new line
                output.push_str(&format!("<{}>\n{}\n</{}>\n", tag, s, tag));
            } else if CODE_FIELDS.contains(&tag) && !s.is_empty() {
                // Single-line code content: still put on new line for consistency
                output.push_str(&format!("<{}>\n{}\n</{}>\n", tag, s, tag));
            } else {
                // Regular string: inline
                output.push_str(&format!("<{}>{}</{}>\n", tag, escape_xml(s), tag));
            }
        }
        Value::Array(arr) => {
            output.push_str(&format!("<{}>\n", tag));

            let item_tag = singularize(tag);
            for item in arr {
                format_value(output, &item_tag, item);
            }

            output.push_str(&format!("</{}>\n", tag));
        }
        Value::Object(obj) => {
            output.push_str(&format!("<{}>\n", tag));

            for (key, val) in obj {
                format_value(output, key, val);
            }

            output.push_str(&format!("</{}>\n", tag));
        }
    }
}

/// Escape XML special characters in a string.
///
/// This is only applied to non-code strings to prevent breaking XML parsing
/// while preserving code content exactly as-is.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Convert a plural tag name to singular for array items.
///
/// Examples:
/// - "files" -> "file"
/// - "entries" -> "entry"
/// - "matches" -> "match"
/// - "commits" -> "commit"
fn singularize(tag: &str) -> String {
    if let Some(stem) = tag.strip_suffix("ies") {
        // entries -> entry
        format!("{stem}y")
    } else if tag.ends_with("ches") || tag.ends_with("shes") || tag.ends_with("xes") {
        // matches -> match, branches -> branch
        tag[..tag.len() - 2].to_string()
    } else if tag.ends_with('s') && !tag.ends_with("ss") {
        // files -> file, commits -> commit
        tag[..tag.len() - 1].to_string()
    } else {
        // Fallback: use item suffix
        format!("{}_item", tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_object() {
        let value = json!({
            "success": true,
            "count": 42
        });
        let xml = to_xml(&value);
        assert!(xml.contains("<success>true</success>"));
        assert!(xml.contains("<count>42</count>"));
    }

    #[test]
    fn test_string_value() {
        let value = json!({
            "path": "src/main.rs"
        });
        let xml = to_xml(&value);
        assert_eq!(xml, "<path>src/main.rs</path>\n");
    }

    #[test]
    fn test_code_content() {
        let value = json!({
            "content": "fn main() {\n    println!(\"Hello\");\n}"
        });
        let xml = to_xml(&value);
        assert!(xml.contains("<content>\n"));
        assert!(xml.contains("fn main()"));
        assert!(xml.contains("</content>"));
    }

    #[test]
    fn test_array() {
        let value = json!({
            "files": [
                {"path": "a.rs"},
                {"path": "b.rs"}
            ]
        });
        let xml = to_xml(&value);
        assert!(xml.contains("<files>"));
        assert!(xml.contains("<file>"));
        assert!(xml.contains("</file>"));
        assert!(xml.contains("</files>"));
    }

    #[test]
    fn test_null_value() {
        let value = json!({
            "error": null
        });
        let xml = to_xml(&value);
        assert!(xml.contains("<error/>"));
    }

    #[test]
    fn test_escape_xml() {
        let value = json!({
            "message": "a < b && c > d"
        });
        let xml = to_xml(&value);
        assert!(xml.contains("&lt;"));
        assert!(xml.contains("&gt;"));
        assert!(xml.contains("&amp;"));
    }

    #[test]
    fn test_singularize() {
        assert_eq!(singularize("files"), "file");
        assert_eq!(singularize("entries"), "entry");
        assert_eq!(singularize("matches"), "match");
        assert_eq!(singularize("branches"), "branch");
        assert_eq!(singularize("commits"), "commit");
    }
}
