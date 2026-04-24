// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Artifact extraction library.
//!
//! Mirrors a `raw/` tree of business documents into an `extracted/` tree of
//! plain-text `.txt` files, with one `.txt` per source input. Each extracted
//! file starts with a fixed header block capturing provenance (source path,
//! size, mtime, extraction date) so downstream pipeline stages can verify
//! which raw file produced which derived text without re-stat'ing the source.
//!
//! Rust port of `extract_artifacts.py` from goa-software-factory. The header
//! format, directory layout, ZIP handling, and cache semantics are bit-for-bit
//! compatible so a project that was previously extracted via the Python
//! script can be re-run with this crate without invalidating existing caches.
//!
//! # Supported formats
//!
//! | Extension | Backend                     | Notes                                           |
//! | --------- | --------------------------- | ----------------------------------------------- |
//! | `.docx`   | OOXML over `zip`+`quick-xml`| Paragraphs (with heading styles) + tables.      |
//! | `.xlsx`   | `calamine`                  | Sheets, rows as TSV.                            |
//! | `.pptx`   | OOXML over `zip`+`quick-xml`| Slide titles, text frames, tables, notes.       |
//! | `.pdf`    | `pdf-extract`               | Text per page; empty-page structural fallback.  |
//! | `.json`   | `serde_json`                | Pretty-print with size summary.                 |
//! | `.pbix`   | ZIP + text decode           | Extract Report/Layout, DataModelSchema, etc.    |
//! | `.zip`    | `zip`                       | Walk archive, recurse into supported entries.   |
//!
//! # Layout
//!
//! For a raw file at `<root>/raw/a/b/c.docx`, the extractor writes:
//!
//! - `<root>/extracted/a/b/c.docx.txt`
//!
//! For a zip archive at `<root>/raw/a/b/bundle.zip` containing `sheet.xlsx`,
//! the extractor writes:
//!
//! - `<root>/extracted/a/b/bundle.zip.txt`  (archive manifest + per-entry log)
//! - `<root>/extracted/a/b/bundle/sheet.xlsx.txt`

pub mod cache;
pub mod dispatch;
pub mod extractors;
pub mod header;

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Outcome classification for a single file processed by [`process_file`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Freshly extracted this run.
    Ok,
    /// Destination was already up-to-date; no work done.
    Cached,
    /// Extraction was attempted but failed; the destination file contains a
    /// failure header and the exception detail so downstream stages can
    /// record the failure without re-running.
    Error,
    /// The file's extension is not in the dispatcher table; no file was
    /// written.
    SkipUnsupported,
}

impl Status {
    pub fn tag(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Cached => "cached",
            Status::Error => "error",
            Status::SkipUnsupported => "skip-unsupported",
        }
    }
}

/// Result of extracting a single file: the status classification and a
/// human-readable message (what was written / why skipped).
#[derive(Debug, Clone)]
pub struct ProcessOutcome {
    pub status: Status,
    pub message: String,
}

impl ProcessOutcome {
    pub fn ok(message: impl Into<String>) -> Self {
        Self { status: Status::Ok, message: message.into() }
    }
    pub fn cached(message: impl Into<String>) -> Self {
        Self { status: Status::Cached, message: message.into() }
    }
    pub fn error(message: impl Into<String>) -> Self {
        Self { status: Status::Error, message: message.into() }
    }
    pub fn skip_unsupported(message: impl Into<String>) -> Self {
        Self { status: Status::SkipUnsupported, message: message.into() }
    }
}

/// Aggregate counts for a batch extraction run.
#[derive(Debug, Default, Clone, Copy)]
pub struct RunCounts {
    pub ok: u64,
    pub cached: u64,
    pub error: u64,
    pub skip_unsupported: u64,
}

impl RunCounts {
    pub fn record(&mut self, status: Status) {
        match status {
            Status::Ok => self.ok += 1,
            Status::Cached => self.cached += 1,
            Status::Error => self.error += 1,
            Status::SkipUnsupported => self.skip_unsupported += 1,
        }
    }
    pub fn has_errors(&self) -> bool {
        self.error > 0
    }
}

/// Options controlling a batch run.
#[derive(Debug, Clone)]
pub struct RunOptions {
    /// Path to the `raw/` directory.
    pub raw_dir: PathBuf,
    /// Path to the `extracted/` directory.
    pub extracted_dir: PathBuf,
    /// If true, re-extract even when the destination is up-to-date.
    pub force: bool,
}

/// Top-level batch entry point. Walks `opts.raw_dir` recursively, dispatches
/// each file through [`process_file`], and emits per-file events to the
/// caller via `on_file`.
pub fn run(
    opts: &RunOptions,
    mut on_file: impl FnMut(&Path, &ProcessOutcome),
) -> Result<RunCounts, ExtractError> {
    if !opts.raw_dir.is_dir() {
        return Err(ExtractError::MissingRawDir(opts.raw_dir.clone()));
    }
    std::fs::create_dir_all(&opts.extracted_dir)
        .map_err(|e| ExtractError::Io(opts.extracted_dir.clone(), e))?;

    let mut counts = RunCounts::default();
    let mut sources: Vec<PathBuf> = walkdir::WalkDir::new(&opts.raw_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect();
    sources.sort();

    for src in &sources {
        let outcome = process_file(src, opts);
        counts.record(outcome.status);
        on_file(src, &outcome);
    }

    Ok(counts)
}

/// Extract a single file. Dispatches on extension; handles the ZIP case by
/// recursing over archive entries.
pub fn process_file(src: &Path, opts: &RunOptions) -> ProcessOutcome {
    let ext = src
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();

    if ext == "zip" {
        return extractors::zip::process_zip(src, opts);
    }

    let ext_dot = format!(".{}", ext);
    let Some(entry) = dispatch::lookup(&ext_dot) else {
        return ProcessOutcome::skip_unsupported(format!("unsupported extension .{ext}"));
    };

    let dst = match target_for_raw(src, opts) {
        Ok(p) => p,
        Err(e) => return ProcessOutcome::error(e.to_string()),
    };

    let meta = match std::fs::metadata(src) {
        Ok(m) => m,
        Err(e) => return ProcessOutcome::error(format!("stat failed: {e}")),
    };
    let src_mtime = cache::mtime_as_system(&meta);

    if !opts.force && !cache::should_extract(src_mtime, &dst) {
        return ProcessOutcome::cached(format!(
            "up-to-date -> {}",
            relpath_for_display(&dst, opts)
        ));
    }

    if let Some(parent) = dst.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return ProcessOutcome::error(format!("mkdir {}: {e}", parent.display()));
    }

    let rel = src.strip_prefix(&opts.raw_dir).unwrap_or(src);
    let rel_posix = posix_path_display(rel);
    let size = meta.len();

    let header_block = header::write_header(
        &rel_posix,
        entry.label,
        size,
        src_mtime,
        None,
    );

    let body_result = (entry.extract)(src);
    let (body, status, summary) = match body_result {
        Ok(body) => {
            let len = body.len();
            (body, Status::Ok, format!("wrote {} ({} chars)", relpath_for_display(&dst, opts), format_with_thousands(len)))
        }
        Err(e) => (
            format!("[EXTRACTION FAILED: {e}]\n\n{e:#}"),
            Status::Error,
            e.to_string(),
        ),
    };

    let full = format!("{header_block}{body}");
    if let Err(e) = std::fs::write(&dst, full) {
        return ProcessOutcome::error(format!("write {}: {e}", dst.display()));
    }

    ProcessOutcome { status, message: summary }
}

/// Compute the destination `.txt` for a given raw file: mirror the tree,
/// append `.txt` to the filename.
pub fn target_for_raw(src: &Path, opts: &RunOptions) -> Result<PathBuf, ExtractError> {
    let rel = src
        .strip_prefix(&opts.raw_dir)
        .map_err(|_| ExtractError::PathNotUnderRaw(src.to_path_buf()))?;
    let file_name = rel
        .file_name()
        .ok_or_else(|| ExtractError::PathNotUnderRaw(src.to_path_buf()))?;
    let mut new_name = file_name.to_os_string();
    new_name.push(".txt");
    let mut dst = opts.extracted_dir.clone();
    if let Some(parent) = rel.parent() {
        dst.push(parent);
    }
    dst.push(new_name);
    Ok(dst)
}

/// Compute the destination `.txt` for a zip entry: archive at
/// `raw/<dirs>/<name>.zip` puts each entry under `extracted/<dirs>/<name>/...`
/// so consumers see the same paths as an unpacked folder.
pub fn target_for_zip_entry(
    zip_path: &Path,
    entry_name: &str,
    opts: &RunOptions,
) -> Result<PathBuf, ExtractError> {
    let rel = zip_path
        .strip_prefix(&opts.raw_dir)
        .map_err(|_| ExtractError::PathNotUnderRaw(zip_path.to_path_buf()))?;
    let container = rel.with_extension("");
    let mut dst = opts.extracted_dir.clone();
    dst.push(container);
    dst.push(format!("{entry_name}.txt"));
    Ok(dst)
}

fn relpath_for_display(p: &Path, opts: &RunOptions) -> String {
    let root = opts
        .extracted_dir
        .parent()
        .unwrap_or(&opts.extracted_dir);
    match p.strip_prefix(root) {
        Ok(r) => posix_path_display(r),
        Err(_) => posix_path_display(p),
    }
}

fn posix_path_display(p: &Path) -> String {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

pub(crate) fn format_with_thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out.chars().rev().collect()
}

#[derive(Debug, Error)]
pub enum ExtractError {
    #[error("raw directory does not exist: {0}")]
    MissingRawDir(PathBuf),
    #[error("path is not under the configured raw dir: {0}")]
    PathNotUnderRaw(PathBuf),
    #[error("io error on {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),
}
