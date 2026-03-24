// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

use anyhow::Result;
use axiomregent::config::{BlobBackend, Compression, StorageConfig};
use axiomregent::snapshot::lease::LeaseStore;
use axiomregent::snapshot::store::Store;
use axiomregent::snapshot::tools::SnapshotTools;
use axiomregent::workspace::WorkspaceTools;
use std::sync::Arc;

#[test]
fn test_apply_patch_determinism() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let config = StorageConfig {
        data_dir: dir.path().to_path_buf(),
        blob_backend: BlobBackend::Fs,
        compression: Compression::None,
    };
    let store = Arc::new(Store::new(config)?);
    let lease_store = Arc::new(LeaseStore::new());

    // Setup tools
    let _snap_tools = SnapshotTools::new(lease_store.clone(), store.clone());
    let workspace_tools = WorkspaceTools::new(lease_store.clone(), store.clone());

    // 1. Create Base Snapshot
    let t1 = "base content\n";
    let h1 = store.put_blob(t1.as_bytes())?;

    // Manually create snapshot for simplicity (using internal Store API)
    // In real usage, one might use snapshot_create, but that needs explicit paths.
    // Store::put_snapshot needs manifest bytes.
    let manifest_json = format!(
        r#"{{"entries": [{{"path": "file.txt", "blob": "{}", "size": {}}}]}}"#,
        h1,
        t1.len()
    );

    let base_sid = "snap-base";
    let repo_root = "/repo";
    let head_sha = "sha-base";
    let fingerprint = r#"{"head_oid": "sha-base", "status_hash": "status1"}"#; // Approximate FP format

    store.put_snapshot(
        base_sid,
        repo_root,
        head_sha,
        fingerprint,
        manifest_json.as_bytes(),
        None,
        None,
        None,
    )?;

    // 2. Define Patch
    // Modifies file.txt: "base content" -> "new content"
    let patch = r#"diff --git a/file.txt b/file.txt
index 0000000..1111111 100644
--- a/file.txt
+++ b/file.txt
@@ -1 +1 @@
-base content
+new content
"#;

    // 3. Apply Patch First Time
    let res1 = workspace_tools.apply_patch(
        std::path::Path::new(repo_root),
        patch,
        "snapshot",
        None,                       // lease_id
        Some(base_sid.to_string()), // snapshot_id
        None,                       // strip
        false,                      // reject
        false,                      // dry
    )?;

    let sid1 = res1["snapshot_id"].as_str().unwrap().to_string();

    // 4. Apply Patch Second Time
    let res2 = workspace_tools.apply_patch(
        std::path::Path::new(repo_root),
        patch,
        "snapshot",
        None,                       // lease_id
        Some(base_sid.to_string()), // snapshot_id
        None,                       // strip
        false,                      // reject
        false,                      // dry
    )?;

    let sid2 = res2["snapshot_id"].as_str().unwrap().to_string();

    // 5. Verification
    assert_eq!(sid1, sid2, "Snapshot IDs must be deterministic");

    // Check metadata of new snapshot
    let info = store
        .get_snapshot_info(&sid1)?
        .expect("Snapshot should exist");
    assert_eq!(info.repo_root, repo_root);
    assert_eq!(info.head_sha, head_sha);
    assert_eq!(info.fingerprint_json, fingerprint); // Should inherit base fingerprint

    // Check lineage
    assert_eq!(info.derived_from.as_deref(), Some(base_sid));
    assert!(info.applied_patch_hash.is_some());

    // Check content
    // We can't easily check content without listing entries, but determinism is the main goal here.

    Ok(())
}
