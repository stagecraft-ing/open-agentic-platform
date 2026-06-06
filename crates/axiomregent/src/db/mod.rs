// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus

//! Hiqlite database initialisation and schema migrations for axiomregent.
//!
//! Call [`init_hiqlite`] once at startup to obtain a [`hiqlite::Client`] with
//! all tables created. The node runs in single-node mode (no real Raft peers)
//! and is strictly local — suitable for a desktop agent process.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

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
    // Self-heal a port-0-poisoned data dir left by a pre-fix binary BEFORE we
    // try to start the node. Otherwise hiqlite reloads the committed
    // 127.0.0.1:0 membership ("node 1 raft is already initialized") and the
    // in-process API client floods stderr forever, regardless of this binary's
    // concrete-port fix. Best-effort: a failure of the heal *check* itself
    // (e.g. a permission error scanning the dir) must not block startup.
    if let Err(e) = heal_port_zero_membership(data_dir) {
        log::warn!("init_hiqlite: port-0 membership self-heal check failed (continuing): {e}");
    }

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
// Port-0 membership self-heal (spec 183 — follow-up to the #274/#275 fix)
// ---------------------------------------------------------------------------

/// Detect and neutralise a data dir whose committed Raft membership advertises
/// `127.0.0.1:0`.
///
/// Pre-fix axiomregent binaries (before `free_loopback_pair`) handed hiqlite a
/// `:0` node address. hiqlite/openraft persists node membership in committed
/// Raft state, so such a dir reloads the port-0 address on *every* restart
/// ("node 1 raft is already initialized") and the in-process API client floods
/// stderr dialing 127.0.0.1:0 (`os error 49`) ~once per second — the
/// concrete-port fix only ever helped a fresh init. The address is woven
/// through the committed log + snapshot, so there is no in-place edit; the only
/// clean removal is to start a fresh store.
///
/// This moves the poisoned dir **aside** (a numbered sibling) rather than
/// deleting it, so the migration is recoverable — the local hiqlite store is a
/// regenerable checkpoint/cache (spec 041), but we still never destroy it
/// unilaterally. The next `init_hiqlite` then creates a fresh dir with concrete
/// loopback ports. Only the raft control-plane files (logs + snapshots) are
/// scanned for the signature — never the SQLite state-machine db, whose user
/// data could hold a false-positive string — so a healthy dir with concrete
/// ports is never touched.
fn heal_port_zero_membership(data_dir: &Path) -> Result<()> {
    if !data_dir.exists() || !membership_has_port_zero(data_dir) {
        return Ok(());
    }
    let aside = next_available_sibling(data_dir, "port0-corrupt");
    log::warn!(
        "init_hiqlite: detected port-0 Raft membership in {} (pre-fix data dir — \
         the cause of the 127.0.0.1:0 reconnect flood). Moving it aside to {} and \
         starting a fresh store with concrete loopback ports. The old store is \
         preserved (renamed, not deleted) and can be recovered manually.",
        data_dir.display(),
        aside.display(),
    );
    std::fs::rename(data_dir, &aside)?;
    Ok(())
}

/// True if any raft control-plane file under `data_dir` contains the port-0
/// node-address signature. Scans logs + snapshots only; the SQLite
/// state-machine db is deliberately excluded (it holds user data that could
/// contain a literal `127.0.0.1:0`, and the committed membership we care about
/// is always present in the log/snapshot anyway).
fn membership_has_port_zero(data_dir: &Path) -> bool {
    const CONTROL_PLANE: &[&str] = &[
        "logs",
        "logs_cache",
        "state_machine/snapshots",
        "state_machine_cache/snapshots",
    ];
    CONTROL_PLANE
        .iter()
        .any(|sub| dir_contains_port_zero(&data_dir.join(sub)))
}

/// Recursively scan `dir` for the port-0 node-address signature. A missing or
/// unreadable directory reads as "no signature" — a dir that isn't there can't
/// be poisoned.
fn dir_contains_port_zero(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let matched = if path.is_dir() {
            dir_contains_port_zero(&path)
        } else {
            file_contains_port_zero(&path)
        };
        if matched {
            return true;
        }
    }
    false
}

/// True if `path`'s bytes contain `127.0.0.1:0` as a *complete* address — the
/// byte after the match must not be an ASCII digit, so a concrete port like
/// `127.0.0.1:52535` cannot false-positive (and `format!("127.0.0.1:{port}")`
/// never zero-pads, so `:0` is only ever produced by an actual port 0).
fn file_contains_port_zero(path: &Path) -> bool {
    const NEEDLE: &[u8] = b"127.0.0.1:0";
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    bytes.windows(NEEDLE.len()).enumerate().any(|(i, w)| {
        w == NEEDLE && bytes.get(i + NEEDLE.len()).is_none_or(|b| !b.is_ascii_digit())
    })
}

/// First non-existing `<data_dir>.<suffix>[.N]` sibling, so repeated heals on
/// successive launches never clobber a previously-preserved store.
fn next_available_sibling(data_dir: &Path, suffix: &str) -> PathBuf {
    let stem = data_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("data");
    let first = data_dir.with_file_name(format!("{stem}.{suffix}"));
    if !first.exists() {
        return first;
    }
    (1u32..)
        .map(|n| data_dir.with_file_name(format!("{stem}.{suffix}.{n}")))
        .find(|p| !p.exists())
        .expect("an unused sibling name exists")
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

    // ── Port-0 membership self-heal ──────────────────────────────────────

    /// The signature must match an *exact* port 0 and reject a concrete
    /// resolved port — the precise distinction between a poisoned and a
    /// healthy committed membership.
    #[test]
    fn file_contains_port_zero_matches_only_an_exact_port_0() {
        let dir = tempfile::tempdir().unwrap();

        let poisoned = dir.path().join("poisoned.wal");
        // Binary log content with the node address as a UTF-8 string, the way
        // hiqlite serialises `Node { addr_api: String, .. }`.
        std::fs::write(&poisoned, b"\x00\x01node\x011\x01addr_api=127.0.0.1:0\x00").unwrap();
        assert!(
            file_contains_port_zero(&poisoned),
            "an exact 127.0.0.1:0 address must be detected",
        );

        let healthy = dir.path().join("healthy.wal");
        std::fs::write(&healthy, b"\x00\x01node\x011\x01addr_api=127.0.0.1:52535\x00").unwrap();
        assert!(
            !file_contains_port_zero(&healthy),
            "a concrete resolved port must NOT be flagged as poisoned",
        );
    }

    #[test]
    fn heal_moves_a_poisoned_dir_aside_and_preserves_it() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        // A pre-fix layout: port-0 membership in the raft log, plus a
        // state-machine db that we must neither scan nor delete.
        std::fs::create_dir_all(data.join("logs")).unwrap();
        std::fs::create_dir_all(data.join("state_machine/db")).unwrap();
        std::fs::write(
            data.join("logs/0000000000000001.wal"),
            b"membership 127.0.0.1:0 node 1",
        )
        .unwrap();
        std::fs::write(data.join("state_machine/db/axiomregent.db"), b"checkpoint rows").unwrap();

        heal_port_zero_membership(&data).unwrap();

        assert!(!data.exists(), "the poisoned data dir is moved aside");
        let aside = root.path().join("data.port0-corrupt");
        assert!(aside.exists(), "the old store is preserved under a sibling");
        assert!(
            aside.join("state_machine/db/axiomregent.db").exists(),
            "the preserved store (incl. the state-machine db) is not deleted",
        );
    }

    #[test]
    fn heal_leaves_a_healthy_concrete_port_dir_untouched() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data");
        std::fs::create_dir_all(data.join("logs")).unwrap();
        std::fs::write(
            data.join("logs/0000000000000001.wal"),
            b"membership 127.0.0.1:52535 node 1",
        )
        .unwrap();

        heal_port_zero_membership(&data).unwrap();

        assert!(data.exists(), "a healthy concrete-port dir is left in place");
        assert!(!root.path().join("data.port0-corrupt").exists());
    }

    #[test]
    fn heal_is_a_noop_on_a_fresh_install() {
        let root = tempfile::tempdir().unwrap();
        let data = root.path().join("data"); // never created
        heal_port_zero_membership(&data).expect("a missing dir is not an error");
        assert!(!data.exists(), "the heal does not create the dir");
    }
}
