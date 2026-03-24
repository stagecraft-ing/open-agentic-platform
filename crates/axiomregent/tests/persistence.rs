// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

use axiomregent::config::{BlobBackend, Compression, StorageConfig};
use axiomregent::snapshot::store::{Entry, Manifest, Store};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_persistence_survives_restart() -> anyhow::Result<()> {
    // Setup
    let dir = tempdir()?;
    let config = StorageConfig {
        data_dir: dir.path().to_path_buf(),
        blob_backend: BlobBackend::Fs,
        compression: Compression::None,
    };

    let content = b"Hello Persistence";
    let snap_id =
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".to_string();
    let blob_hash_expected = {
        let store = Store::new(config.clone())?;
        let hash = store.put_blob(content)?;

        // create snapshot manually
        let manifest = Manifest::new(vec![Entry {
            path: "file.txt".to_string(),
            blob: hash.clone(),
            size: content.len() as u64,
        }]);
        let manifest_bytes = serde_json::to_vec(&manifest)?;

        store.put_snapshot(
            &snap_id,
            "/tmp/repo/test",
            "sha256:dummy_head",
            r#"{"head_oid":"dummy"}"#,
            &manifest_bytes,
            None,
            None,
            None,
        )?;
        hash
    };

    // "Restart" -> New Store
    let store2 = Store::new(config.clone())?;

    // Verify blob
    let blob = store2.get_blob(&blob_hash_expected)?;
    assert!(blob.is_some(), "Blob should be present after restart");
    assert_eq!(blob.unwrap(), content);

    // Verify snapshot
    let entries = store2.list_snapshot_entries(&snap_id)?;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].path, "file.txt");
    assert_eq!(entries[0].blob, blob_hash_expected);

    Ok(())
}

#[test]
fn test_persistence_corruption_missing_blob_file() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let config = StorageConfig {
        data_dir: dir.path().to_path_buf(),
        blob_backend: BlobBackend::Fs,
        compression: Compression::None,
    };

    let store = Store::new(config.clone())?;
    let content = b"I will be deleted";
    let hash = store.put_blob(content)?;

    // Manually delete the file
    // Need to reconstruct path logic or search for it
    // Implementation detail: .axiomregent/data/blobs/sha256/xx/xxxx...
    let algo = "sha256";
    let parts: Vec<&str> = hash.split(':').collect();
    let val = parts[1];
    let prefix = &val[0..2];
    let blob_path = dir.path().join("blobs").join(algo).join(prefix).join(val);

    assert!(blob_path.exists());
    fs::remove_file(blob_path)?;

    // store.get_blob should return Ok(None) because underlying FsBlobStore returns Ok(None) if file missing
    let result = store.get_blob(&hash)?;
    assert!(result.is_none(), "Should return None if file is missing");

    Ok(())
}

#[test]
fn test_invariant_missing_blob_row() -> anyhow::Result<()> {
    // Test that putting a snapshot referring to a non-existent blob hash fails (refcount update check)
    let dir = tempdir()?;
    let config = StorageConfig {
        data_dir: dir.path().to_path_buf(),
        blob_backend: BlobBackend::Fs,
        compression: Compression::None,
    };

    let store = Store::new(config)?;
    let fake_hash = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    let manifest = Manifest::new(vec![Entry {
        path: "ghost.txt".to_string(),
        blob: fake_hash.to_string(),
        size: 100,
    }]);
    let manifest_bytes = serde_json::to_vec(&manifest)?;

    let result = store.put_snapshot(
        "snap-ghost",
        "/tmp",
        "sha256:head",
        "{}",
        &manifest_bytes,
        None,
        None,
        None,
    );

    assert!(
        result.is_err(),
        "Should fail because blob row is missing in DB"
    );
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Referenced blob not found in DB"));

    Ok(())
}
