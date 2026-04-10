// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Audit logging for permission decisions (spec 068, NF-003).
//!
//! Append-only JSONL log at `~/.claude/audit/permissions.jsonl`.
//! Rotates at 10 MB, retaining the last 5 rotations (R-003 mitigation).

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::Utc;
use serde::Serialize;

use crate::permission::MatchedRule;

/// A single audit log entry (NF-003).
#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub decision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_rule: Option<MatchedRule>,
}

/// Timestamped wrapper written to the JSONL file.
#[derive(Serialize)]
struct TimestampedEntry<'a> {
    timestamp: String,
    #[serde(flatten)]
    entry: &'a AuditEntry,
}

const MAX_SIZE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB
const MAX_ROTATIONS: usize = 5;

/// Thread-safe append-only audit logger.
pub struct AuditLogger {
    path: PathBuf,
    writer: Mutex<Option<BufWriter<File>>>,
}

impl AuditLogger {
    /// Create a logger writing to the given path.
    /// Creates parent directories if needed.
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            path,
            writer: Mutex::new(Some(BufWriter::new(file))),
        })
    }

    /// Create a logger at the default path (`~/.claude/audit/permissions.jsonl`).
    pub fn default_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".claude").join("audit").join("permissions.jsonl"))
    }

    /// Log a permission decision.
    pub fn log(&self, entry: AuditEntry) {
        let timestamped = TimestampedEntry {
            timestamp: Utc::now().to_rfc3339(),
            entry: &entry,
        };

        let mut guard = match self.writer.lock() {
            Ok(g) => g,
            Err(_) => return, // Poisoned — silently skip.
        };

        if let Some(writer) = guard.as_mut() {
            if let Ok(line) = serde_json::to_string(&timestamped) {
                let _ = writeln!(writer, "{}", line);
                let _ = writer.flush();
            }
        }

        // Check rotation after write.
        drop(guard);
        self.maybe_rotate();
    }

    fn maybe_rotate(&self) {
        let size = fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);

        if size < MAX_SIZE_BYTES {
            return;
        }

        // Rotate: permissions.jsonl → permissions.jsonl.1 → .2 → … → .5 (delete oldest).
        let mut guard = match self.writer.lock() {
            Ok(g) => g,
            Err(_) => return,
        };

        // Close current writer.
        *guard = None;

        // Shift existing rotations.
        for i in (1..MAX_ROTATIONS).rev() {
            let from = rotation_path(&self.path, i);
            let to = rotation_path(&self.path, i + 1);
            if from.exists() {
                let _ = fs::rename(&from, &to);
            }
        }

        // Move current to .1.
        let _ = fs::rename(&self.path, rotation_path(&self.path, 1));

        // Delete oldest if it exists.
        let oldest = rotation_path(&self.path, MAX_ROTATIONS + 1);
        if oldest.exists() {
            let _ = fs::remove_file(oldest);
        }

        // Reopen.
        if let Ok(file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            *guard = Some(BufWriter::new(file));
        }
    }
}

fn rotation_path(base: &Path, n: usize) -> PathBuf {
    let mut p = base.as_os_str().to_owned();
    p.push(format!(".{n}"));
    PathBuf::from(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn creates_parent_dirs_and_writes() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("audit").join("permissions.jsonl");
        let logger = AuditLogger::new(path.clone()).unwrap();

        logger.log(AuditEntry {
            tool_name: "Bash".into(),
            file_path: None,
            command: Some("cargo test".into()),
            decision: "Allow".into(),
            matched_rule: None,
        });

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"tool_name\":\"Bash\""));
        assert!(content.contains("\"timestamp\":"));
    }

    #[test]
    fn rotation_path_format() {
        let base = PathBuf::from("/tmp/permissions.jsonl");
        assert_eq!(
            rotation_path(&base, 1),
            PathBuf::from("/tmp/permissions.jsonl.1")
        );
        assert_eq!(
            rotation_path(&base, 5),
            PathBuf::from("/tmp/permissions.jsonl.5")
        );
    }
}
