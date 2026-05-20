//! File-type section parsers for diff-to-section attribution (spec 152 §2).
//!
//! This module is the **partial activation** entry point. Only the Makefile
//! parser is implemented in this session. All other file types fall back to
//! whole-file authority, which is the correct behaviour per spec 152 §2.2:
//! "If H falls outside any named section … the gate falls back to whole-file
//! authority for that hunk."
//!
//! Adding parsers for Markdown, GitHub workflow YAML, Rust/TypeScript
//! regions, and TOML/JSON is deferred; each new parser slots in as a
//! submodule here without touching the caller.

pub mod makefile;

pub use makefile::MakefileSection;
