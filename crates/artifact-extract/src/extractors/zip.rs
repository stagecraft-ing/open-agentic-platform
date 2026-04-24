// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! ZIP archive walker.
//!
//! A raw file with extension `.zip` is processed specially: we walk every
//! entry, recurse into supported formats, and produce two layers of output:
//!
//! 1. A per-archive manifest at `extracted/<dirs>/<name>.zip.txt` containing
//!    the archive listing and a per-entry status log.
//! 2. One `.txt` per supported entry at `extracted/<dirs>/<name>/<entry>.txt`
//!    (so downstream stages see the same paths as if the archive were
//!    unpacked on disk).
//!
//! The entry-level extraction uses a temp file on disk because some format
//! backends (notably `calamine` and `pdf-extract`) want real paths.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::time::SystemTime;

use tempfile::NamedTempFile;

use crate::dispatch;
use crate::header;
use crate::{ProcessOutcome, RunOptions, Status, cache};

pub fn process_zip(src: &Path, opts: &RunOptions) -> ProcessOutcome {
    let meta = match std::fs::metadata(src) {
        Ok(m) => m,
        Err(e) => return ProcessOutcome::error(format!("stat failed: {e}")),
    };
    let src_size = meta.len();
    let src_mtime = cache::mtime_as_system(&meta);

    let rel_zip_posix = match src.strip_prefix(&opts.raw_dir) {
        Ok(r) => r
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => src.display().to_string(),
    };

    let manifest_dst = match crate::target_for_raw(src, opts) {
        Ok(p) => p,
        Err(e) => return ProcessOutcome::error(e.to_string()),
    };
    if let Some(parent) = manifest_dst.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        return ProcessOutcome::error(format!("mkdir {}: {e}", parent.display()));
    }

    let file = match File::open(src) {
        Ok(f) => f,
        Err(e) => return ProcessOutcome::error(format!("open zip: {e}")),
    };
    let mut archive = match ::zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(e) => return ProcessOutcome::error(format!("not a valid zip: {e}")),
    };

    let mut ok = 0u64;
    let mut cached = 0u64;
    let mut errors = 0u64;
    let mut skipped = 0u64;
    let mut messages: Vec<String> = Vec::new();
    let mut manifest_lines: Vec<String> = Vec::new();

    // First pass for manifest listing — also counts non-directory entries.
    let mut file_count = 0u64;
    let archive_len = archive.len();
    let mut listing_lines: Vec<String> = Vec::new();
    for i in 0..archive_len {
        let entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.is_dir() {
            continue;
        }
        file_count += 1;
        let size = entry.size();
        let (y, m, d) = entry
            .last_modified()
            .map(|dt| (dt.year() as i32, dt.month() as u32, dt.day() as u32))
            .unwrap_or((1980, 1, 1));
        listing_lines.push(format!(
            "  {size:>12}  {y:04}-{m:02}-{d:02}  {name}",
            name = entry.name()
        ));
    }
    manifest_lines.push(format!("[Archive entries ({file_count} files):]"));
    manifest_lines.extend(listing_lines);

    // Second pass for per-entry extraction.
    for i in 0..archive.len() {
        // Re-collect the entry name + size before we pull bytes (we need a
        // stable borrow of the entry to read it into a temp file).
        let (entry_name, is_dir, entry_size) = {
            let e = match archive.by_index(i) {
                Ok(e) => e,
                Err(_) => continue,
            };
            (e.name().to_string(), e.is_dir(), e.size())
        };
        if is_dir {
            continue;
        }

        let ext = Path::new(&entry_name)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();
        let ext_dot = format!(".{ext}");

        let dst = match crate::target_for_zip_entry(src, &entry_name, opts) {
            Ok(p) => p,
            Err(e) => {
                errors += 1;
                messages.push(format!("  [error] {entry_name}: {e}"));
                continue;
            }
        };

        let Some(dispatch_entry) = dispatch::lookup(&ext_dot) else {
            skipped += 1;
            messages.push(format!("  [skip-unsupported] {entry_name}"));
            continue;
        };

        if !opts.force && !cache::should_extract(src_mtime, &dst) {
            cached += 1;
            messages.push(format!("  [cached] {entry_name}"));
            continue;
        }

        if let Some(parent) = dst.parent()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            errors += 1;
            messages.push(format!("  [error] {entry_name}: mkdir {e}"));
            continue;
        }

        // Copy the entry to a temp file so format libs get a real path.
        let suffix = Path::new(&entry_name)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| format!(".{s}"))
            .unwrap_or_default();
        let mut tmp = match NamedTempFile::with_suffix(&suffix) {
            Ok(t) => t,
            Err(e) => {
                errors += 1;
                messages.push(format!("  [error] {entry_name}: tempfile {e}"));
                continue;
            }
        };
        {
            let mut entry = match archive.by_index(i) {
                Ok(e) => e,
                Err(e) => {
                    errors += 1;
                    messages.push(format!("  [error] {entry_name}: by_index {e}"));
                    continue;
                }
            };
            let mut bytes = Vec::with_capacity(entry_size as usize);
            if let Err(e) = entry.read_to_end(&mut bytes) {
                errors += 1;
                messages.push(format!("  [error] {entry_name}: read {e}"));
                continue;
            }
            if let Err(e) = tmp.write_all(&bytes) {
                errors += 1;
                messages.push(format!("  [error] {entry_name}: write tmp {e}"));
                continue;
            }
        }
        let tmp_path = tmp.path().to_path_buf();

        let head = header::write_header(
            &rel_zip_posix,
            dispatch_entry.label,
            entry_size,
            src_mtime,
            Some(&entry_name),
        );

        let (body, ok_this) = match (dispatch_entry.extract)(&tmp_path) {
            Ok(b) => (b, true),
            Err(e) => (format!("[EXTRACTION FAILED: {e}]\n\n{e:#}"), false),
        };
        let _ = tmp; // close after extractor is done

        let full = format!("{head}{body}");
        if let Err(e) = std::fs::write(&dst, full) {
            errors += 1;
            messages.push(format!("  [error] {entry_name}: write dst {e}"));
            continue;
        }
        if ok_this {
            ok += 1;
            messages.push(format!(
                "  [ok] {entry_name} -> {} ({} chars)",
                display_rel(&dst, opts),
                crate::format_with_thousands(body.len())
            ));
        } else {
            errors += 1;
            messages.push(format!("  [error] {entry_name}: extract failed"));
        }
    }

    let manifest_header =
        header::write_header(&rel_zip_posix, "Zip Archive", src_size, src_mtime, None);
    let body = format!(
        "{}\n\n[Per-entry extraction results:]\n{}",
        manifest_lines.join("\n"),
        messages.join("\n")
    );
    if let Err(e) = std::fs::write(&manifest_dst, format!("{manifest_header}{body}")) {
        return ProcessOutcome::error(format!("write manifest: {e}"));
    }

    let status = if errors > 0 { Status::Error } else { Status::Ok };
    let summary = format!(
        "zip {rel_zip_posix}: {ok} extracted, {cached} cached, {skipped} unsupported, {errors} errors"
    );
    ProcessOutcome {
        status,
        message: summary,
    }
}

fn display_rel(p: &Path, opts: &RunOptions) -> String {
    let root = opts
        .extracted_dir
        .parent()
        .unwrap_or(&opts.extracted_dir);
    match p.strip_prefix(root) {
        Ok(r) => r
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/"),
        Err(_) => p.display().to_string(),
    }
}

// Unused helper kept for symmetry with the library SystemTime plumbing; a
// future change may decorate messages with archive timestamps.
#[allow(dead_code)]
fn fmt_mtime(_: SystemTime) -> String {
    String::new()
}
