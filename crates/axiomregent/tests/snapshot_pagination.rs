// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

use anyhow::Result;
use axiomregent::config::{BlobBackend, Compression, StorageConfig};
use axiomregent::snapshot::lease::LeaseStore;
use axiomregent::snapshot::store::Store;
use axiomregent::snapshot::tools::SnapshotTools;
use std::sync::Arc;

fn setup() -> Result<(
    Arc<Store>,
    Arc<LeaseStore>,
    SnapshotTools,
    tempfile::TempDir,
)> {
    let dir = tempfile::tempdir()?;
    let data_dir = dir.path().join("data");
    std::fs::create_dir(&data_dir)?;
    let repo_dir = dir.path().join("repo");
    std::fs::create_dir(&repo_dir)?;

    let config = StorageConfig {
        data_dir,
        blob_backend: BlobBackend::Fs,
        compression: Compression::None,
    };
    let store = Arc::new(Store::new(config)?);
    let lease_store = Arc::new(LeaseStore::new());
    let tools = SnapshotTools::new(lease_store.clone(), store.clone());

    // Init git repo
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_dir)
        .output()?;
    std::process::Command::new("git")
        .arg("config")
        .arg("user.email")
        .arg("you@example.com")
        .current_dir(&repo_dir)
        .output()?;
    std::process::Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("Your Name")
        .current_dir(&repo_dir)
        .output()?;

    Ok((store, lease_store, tools, dir))
}

#[test]
fn test_worktree_pagination() -> Result<()> {
    let (_, _, tools, dir) = setup()?;
    let root = dir.path().join("repo");

    // Create 10 files: f0.txt ... f9.txt
    for i in 0..10 {
        std::fs::write(root.join(format!("f{}.txt", i)), "content")?;
    }

    // List all
    let res = tools.snapshot_list(&root, "", "worktree", None, None, None, None)?;
    let entries = res["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 10);
    assert_eq!(res["total"], 10);
    assert_eq!(res["truncated"], false);

    // Page 1: Limit 4, Offset 0 -> f0..f3
    let res = tools.snapshot_list(&root, "", "worktree", None, None, Some(4), Some(0))?;
    let entries = res["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(res["total"], 10);
    assert_eq!(res["truncated"], true); // 0+4 < 10
    assert_eq!(entries[0]["path"], "f0.txt");
    assert_eq!(entries[3]["path"], "f3.txt");

    // Page 2: Limit 4, Offset 4 -> f4..f7
    let res = tools.snapshot_list(&root, "", "worktree", None, None, Some(4), Some(4))?;
    let entries = res["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(res["truncated"], true); // 4+4 < 10
    assert_eq!(entries[0]["path"], "f4.txt");

    // Page 3: Limit 4, Offset 8 -> f8..f9
    let res = tools.snapshot_list(&root, "", "worktree", None, None, Some(4), Some(8))?;
    let entries = res["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(res["truncated"], false); // 8+4 >= 10
    assert_eq!(entries[0]["path"], "f8.txt");

    Ok(())
}

#[test]
fn test_snapshot_pagination() -> Result<()> {
    let (_, _, tools, dir) = setup()?;
    let root = dir.path().join("repo");

    // Create 10 files and snapshot
    let mut paths = Vec::new();
    for i in 0..10 {
        let name = format!("f{}.txt", i);
        std::fs::write(root.join(&name), "content")?;
        paths.push(name);
    }

    let snap_res = tools.snapshot_create(&root, None, Some(paths))?;
    let snap_id = snap_res["snapshot_id"].as_str().unwrap().to_string();

    // List all
    let res = tools.snapshot_list(
        &root,
        "",
        "snapshot",
        None,
        Some(snap_id.clone()),
        None,
        None,
    )?;
    let entries = res["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 10);
    assert_eq!(res["total"], 10);

    // Page: Limit 3, Offset 2 -> f2..f4
    let res = tools.snapshot_list(
        &root,
        "",
        "snapshot",
        None,
        Some(snap_id.clone()),
        Some(3),
        Some(2),
    )?;
    let entries = res["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(res["total"], 10);
    assert_eq!(entries[0]["path"], "f2.txt");
    assert_eq!(entries[2]["path"], "f4.txt");

    Ok(())
}
