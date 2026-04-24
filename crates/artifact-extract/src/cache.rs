// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! mtime-based extraction cache.
//!
//! Matches the Python script's `should_extract` contract: a destination is
//! considered up-to-date when its mtime is greater than or equal to the
//! source's mtime. This is intentionally coarse — we don't hash content —
//! because the only goal is to skip work when nothing has changed.

use std::fs::Metadata;
use std::path::Path;
use std::time::SystemTime;

/// Extract the mtime of a file's metadata, falling back to `UNIX_EPOCH` on
/// platforms where the syscall is not available (so the cache decision
/// becomes "always extract" rather than a panic).
pub fn mtime_as_system(meta: &Metadata) -> SystemTime {
    meta.modified().unwrap_or(SystemTime::UNIX_EPOCH)
}

/// Returns true if the destination should be (re)extracted: it either does
/// not exist yet, or its mtime is strictly older than the source's mtime.
pub fn should_extract(src_mtime: SystemTime, dst: &Path) -> bool {
    let Ok(dst_meta) = std::fs::metadata(dst) else {
        return true; // destination missing → extract
    };
    let dst_mtime = mtime_as_system(&dst_meta);
    src_mtime > dst_mtime
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, SystemTime};
    use tempfile::tempdir;

    #[test]
    fn missing_destination_triggers_extraction() {
        let dir = tempdir().unwrap();
        let dst = dir.path().join("missing.txt");
        assert!(should_extract(SystemTime::now(), &dst));
    }

    #[test]
    fn older_destination_triggers_extraction() {
        let dir = tempdir().unwrap();
        let dst = dir.path().join("old.txt");
        fs::write(&dst, "x").unwrap();
        // Source mtime is in the future relative to dst.
        let future = SystemTime::now() + Duration::from_secs(3600);
        assert!(should_extract(future, &dst));
    }
}
