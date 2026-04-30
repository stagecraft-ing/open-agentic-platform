// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Hiqlite database initialisation and schema migrations for axiomregent.
//!
//! Call [`init_hiqlite`] once at startup to obtain a [`hiqlite::Client`] with
//! all tables created. The node runs in single-node mode (no real Raft peers)
//! and is strictly local — suitable for a desktop agent process.

use std::borrow::Cow;
use std::path::Path;

use anyhow::Result;
use hiqlite::{Client, Node, NodeConfig};

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

/// DDL statements executed in order at startup (all idempotent).
const SCHEMA_SQL: &[&str] = &[
    r#"CREATE TABLE IF NOT EXISTS checkpoints (
        checkpoint_id TEXT PRIMARY KEY,
        repo_root     TEXT NOT NULL,
        parent_id     TEXT,
        label         TEXT,
        head_sha      TEXT,
        fingerprint   TEXT NOT NULL,
        state_hash    TEXT NOT NULL,
        merkle_root   TEXT NOT NULL,
        file_count    INTEGER NOT NULL,
        total_bytes   INTEGER NOT NULL,
        created_at    TEXT NOT NULL,
        metadata      TEXT,
        project_id    TEXT,
        branch_name   TEXT,
        run_id        TEXT
    )"#,
    r#"CREATE TABLE IF NOT EXISTS manifest_entries (
        checkpoint_id TEXT NOT NULL,
        path          TEXT NOT NULL,
        blob_hash     TEXT NOT NULL,
        size_bytes    INTEGER NOT NULL,
        permissions   INTEGER,
        PRIMARY KEY (checkpoint_id, path)
    )"#,
    r#"CREATE TABLE IF NOT EXISTS blob_refs (
        blob_hash   TEXT PRIMARY KEY,
        ref_count   INTEGER NOT NULL DEFAULT 1,
        size_bytes  INTEGER NOT NULL,
        compression TEXT NOT NULL DEFAULT 'lz4'
    )"#,
    r#"CREATE TABLE IF NOT EXISTS leases (
        lease_id      TEXT PRIMARY KEY,
        repo_root     TEXT NOT NULL,
        fingerprint   TEXT NOT NULL,
        touched_files TEXT NOT NULL,
        grants        TEXT NOT NULL,
        issued_at     TEXT NOT NULL,
        expires_at    TEXT NOT NULL
    )"#,
    r#"CREATE TABLE IF NOT EXISTS runs (
        run_id       TEXT PRIMARY KEY,
        skill_name   TEXT NOT NULL,
        repo_root    TEXT NOT NULL,
        status       TEXT NOT NULL,
        exit_code    INTEGER,
        log_path     TEXT,
        started_at   TEXT NOT NULL,
        completed_at TEXT
    )"#,
    r#"CREATE TABLE IF NOT EXISTS embeddings (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        project_name  TEXT NOT NULL,
        file_path     TEXT NOT NULL,
        block_type    TEXT NOT NULL,
        function_name TEXT,
        code_content  TEXT NOT NULL,
        vector        BLOB NOT NULL,
        call_edges    TEXT,
        indexed_at    TEXT NOT NULL
    )"#,
    "CREATE INDEX IF NOT EXISTS idx_embeddings_project ON embeddings(project_name)",
    r#"CREATE TABLE IF NOT EXISTS audit_log (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        tool_name       TEXT NOT NULL,
        tier            INTEGER NOT NULL,
        repo_root       TEXT,
        lease_id        TEXT,
        policy_decision TEXT,
        timestamp       TEXT NOT NULL,
        metadata        TEXT
    )"#,
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Initialise a single-node hiqlite instance rooted at `data_dir`.
///
/// The database file is `axiomregent.db` inside `data_dir`. All schema tables
/// are created (idempotently) before the client is returned.
pub async fn init_hiqlite(data_dir: &Path) -> Result<Client> {
    let data_dir_str = data_dir.to_string_lossy().to_string();

    let config = NodeConfig {
        node_id: 1,
        nodes: vec![Node {
            id: 1,
            addr_raft: "127.0.0.1:0".to_string(),
            addr_api: "127.0.0.1:0".to_string(),
        }],
        data_dir: data_dir_str.into(),
        filename_db: "axiomregent.db".into(),
        secret_raft: "axiomregent-raft-00".into(),
        secret_api: "axiomregent-api-000".into(),
        log_statements: false,
        ..NodeConfig::default()
    };

    let client = hiqlite::start_node(config).await?;
    migrate(&client).await?;
    Ok(client)
}

// ---------------------------------------------------------------------------
// Internal
// ---------------------------------------------------------------------------

/// Run all DDL migrations. Every statement is idempotent (`IF NOT EXISTS`).
async fn migrate(client: &Client) -> Result<()> {
    for ddl in SCHEMA_SQL {
        client.execute(Cow::Borrowed(*ddl), vec![]).await?;
    }

    // Additive column migrations — best-effort: SQLite returns an error when
    // the column already exists, so we ignore "duplicate column" failures.
    let additive: &[&str] = &[
        "ALTER TABLE checkpoints ADD COLUMN project_id TEXT",
        "ALTER TABLE checkpoints ADD COLUMN branch_name TEXT",
        "ALTER TABLE checkpoints ADD COLUMN run_id TEXT",
    ];
    for ddl in additive {
        let _ = client.execute(Cow::Borrowed(*ddl), vec![]).await;
    }

    Ok(())
}
