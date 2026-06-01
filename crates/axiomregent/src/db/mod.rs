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

/// Bounded retries around the resolve-ports → `start_node` window. Because
/// hiqlite has no fd-passing API, ports are resolved by binding `:0`, reading
/// the assigned number, releasing, and handing the concrete port to hiqlite to
/// rebind. Another process can claim a freed port in that narrow window
/// (TOCTOU); when that happens hiqlite's own bind fails, so we re-resolve a
/// fresh pair and try again rather than surfacing a transient bind race as a
/// hard startup error.
const HIQLITE_BIND_ATTEMPTS: u32 = 3;

/// Initialise a single-node hiqlite instance rooted at `data_dir`.
///
/// The database file is `axiomregent.db` inside `data_dir`. All schema tables
/// are created (idempotently) before the client is returned.
pub async fn init_hiqlite(data_dir: &Path) -> Result<Client> {
    let data_dir_str = data_dir.to_string_lossy().to_string();

    // Resolve concrete loopback ports for the single-node Raft + API
    // listeners. hiqlite records these addresses in its Raft membership and
    // the in-process client dials `addr_api` for its background WS stream.
    // Advertising `:0` directly leaves port 0 in the membership — never a
    // connectable target — so the client floods stderr with EADDRNOTAVAIL
    // (os error 49) once per second. We grab two free loopback ports the same
    // way `main.rs` resolves the probe port (bind `:0` → read `local_addr` →
    // release), then hand the concrete ports to hiqlite, which rebinds them.
    // The single-node data path runs in-process regardless of the WS stream;
    // this purely quiets that background task. A lost bind race re-resolves
    // a fresh pair (see HIQLITE_BIND_ATTEMPTS) rather than failing startup.
    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 1..=HIQLITE_BIND_ATTEMPTS {
        let (raft_port, api_port) = free_loopback_pair()?;

        let config = NodeConfig {
            node_id: 1,
            nodes: vec![Node {
                id: 1,
                addr_raft: format!("127.0.0.1:{raft_port}"),
                addr_api: format!("127.0.0.1:{api_port}"),
            }],
            data_dir: data_dir_str.clone().into(),
            filename_db: "axiomregent.db".into(),
            secret_raft: "axiomregent-raft-00".into(),
            secret_api: "axiomregent-api-000".into(),
            log_statements: false,
            ..NodeConfig::default()
        };

        match hiqlite::start_node(config).await {
            Ok(client) => {
                // migrate stays outside the retry: a DDL error is a real
                // failure to propagate, not a bind race to retry.
                migrate(&client).await?;
                return Ok(client);
            }
            Err(e) => {
                log::warn!(
                    "init_hiqlite: start_node bind attempt {attempt}/{HIQLITE_BIND_ATTEMPTS} failed: {e}"
                );
                last_err = Some(e.into());
            }
        }
    }
    Err(last_err
        .unwrap_or_else(|| anyhow::anyhow!("init_hiqlite: exhausted bind attempts")))
}

/// Resolve two distinct free loopback ports. Both listeners are held until
/// both numbers have been read so the OS cannot hand the same port to the
/// raft and api listeners; both are then released for hiqlite to rebind.
fn free_loopback_pair() -> Result<(u16, u16)> {
    let raft = std::net::TcpListener::bind("127.0.0.1:0")?;
    let api = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok((raft.local_addr()?.port(), api.local_addr()?.port()))
    // both listeners drop here, freeing the ports for hiqlite to rebind
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The raft and api listeners must never collapse onto the same port —
    /// holding both listeners until both numbers are read is what guarantees
    /// it. Regression here would feed hiqlite two identical addresses.
    #[test]
    fn free_loopback_pair_yields_two_distinct_ports() {
        let (raft, api) = free_loopback_pair().expect("loopback binds in test env");
        assert_ne!(raft, 0, "raft port must be a concrete resolved port");
        assert_ne!(api, 0, "api port must be a concrete resolved port");
        assert_ne!(raft, api, "raft and api ports must be distinct");
    }
}
