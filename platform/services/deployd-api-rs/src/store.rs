use anyhow::{Context, Result};
use hiqlite::{Client, NodeConfig, Param};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::config::{BackupConfig, Config};

pub struct AppState {
    pub client: Client,
    pub config: Config,
}

/// Dummy 32-byte zero key encoded as base64 (cryptr standard alphabet).
/// Used only when the operator has not opted in to backup; satisfies
/// hiqlite's s3-feature validation (`enc_keys` must be non-empty when
/// the s3 feature is on, even if no S3Config is configured). See spec
/// 145 §2.3 "ENC_KEYS validation".
const DEV_FALLBACK_ENC_KEYS: &str =
    "dev-fallback/AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
const DEV_FALLBACK_ENC_KEY_ACTIVE: &str = "dev-fallback";

pub async fn init_db(data_dir: &str) -> Result<Client> {
    apply_hql_env(data_dir).context("failed to apply HQL_* env translation")?;
    let config = NodeConfig::from_env();
    let client = hiqlite::start_node(config).await?;
    migrate(&client).await?;
    Ok(client)
}

/// Translate deployd-facing env vars (and hardcoded single-replica defaults)
/// to the `HQL_*` env-var surface Hiqlite v0.13.1's `NodeConfig::from_env()`
/// consumes. Spec 145 §2.3 + §3.1 FR-005a.
///
/// This is the single env-mutation site in the binary. It is called once,
/// before any worker thread spawns; all `unsafe { set_var }` blocks are
/// safe under that lifecycle invariant.
fn apply_hql_env(data_dir: &str) -> Result<()> {
    let secret_raft = std::env::var("HIQLITE_SECRET_RAFT")
        .unwrap_or_else(|_| "deployd-raft-secret-key".to_string());
    let secret_api = std::env::var("HIQLITE_SECRET_API")
        .unwrap_or_else(|_| "deployd-api-secret-key0".to_string());

    // SAFETY: called once during startup before any worker thread spawns.
    unsafe {
        std::env::set_var("HQL_NODE_ID", "1");
        std::env::set_var("HQL_NODES", "1 127.0.0.1:7001 127.0.0.1:7002");
        std::env::set_var("HQL_DATA_DIR", data_dir);
        std::env::set_var("HQL_FILENAME_DB", "deployd.db");
        std::env::set_var("HQL_SECRET_RAFT", &secret_raft);
        std::env::set_var("HQL_SECRET_API", &secret_api);
        std::env::set_var("HQL_LOG_STATEMENTS", "false");
    }

    match BackupConfig::from_env()
        .map_err(|e| anyhow::anyhow!("invalid DEPLOYD_BACKUP_* config: {e}"))?
    {
        Some(bc) => {
            tracing::info!(
                bucket = %bc.s3_bucket,
                endpoint = %bc.s3_endpoint,
                cron = %bc.cron_schedule,
                keep_days = bc.keep_days,
                "backup configured — S3 snapshots enabled"
            );
            bc.apply_to_hql_env();
        }
        None => {
            tracing::info!(
                "backup not configured (no DEPLOYD_BACKUP_* env vars); \
                 using dev-fallback ENC_KEYS so hiqlite s3-feature validation passes"
            );
            // SAFETY: same as above — single-threaded init.
            unsafe {
                std::env::set_var("ENC_KEYS", DEV_FALLBACK_ENC_KEYS);
                std::env::set_var("ENC_KEY_ACTIVE", DEV_FALLBACK_ENC_KEY_ACTIVE);
            }
        }
    }
    Ok(())
}

async fn migrate(client: &Client) -> Result<()> {
    client
        .execute(
            Cow::Borrowed(
                "CREATE TABLE IF NOT EXISTS deployments (
                    deployment_id TEXT PRIMARY KEY,
                    deployment_key TEXT NOT NULL UNIQUE,
                    tenant_id TEXT NOT NULL,
                    app_id TEXT NOT NULL,
                    env_id TEXT NOT NULL,
                    release_sha TEXT NOT NULL,
                    artifact_ref TEXT NOT NULL,
                    lane TEXT NOT NULL,
                    status TEXT NOT NULL,
                    app_slug TEXT,
                    env_slug TEXT,
                    desired_routes TEXT,
                    endpoints TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                )",
            ),
            vec![],
        )
        .await?;

    client
        .execute(
            Cow::Borrowed(
                "CREATE TABLE IF NOT EXISTS deployment_events (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    deployment_id TEXT NOT NULL,
                    event_type TEXT NOT NULL,
                    message TEXT,
                    timestamp TEXT NOT NULL
                )",
            ),
            vec![],
        )
        .await?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Deployment {
    pub deployment_id: String,
    pub deployment_key: String,
    pub tenant_id: String,
    pub app_id: String,
    pub env_id: String,
    pub release_sha: String,
    pub artifact_ref: String,
    pub lane: String,
    pub status: String,
    pub app_slug: Option<String>,
    pub env_slug: Option<String>,
    pub desired_routes: Option<String>,
    pub endpoints: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentEvent {
    pub id: i64,
    pub deployment_id: String,
    pub event_type: String,
    pub message: Option<String>,
    pub timestamp: String,
}

pub async fn get_by_key(client: &Client, key: &str) -> Result<Option<Deployment>> {
    let rows: Vec<Deployment> = client
        .query_as(
            "SELECT * FROM deployments WHERE deployment_key = $1",
            vec![Param::Text(key.to_string())],
        )
        .await?;
    Ok(rows.into_iter().next())
}

pub async fn get_by_release_id(client: &Client, release_id: &str) -> Result<Option<Deployment>> {
    let rows: Vec<Deployment> = client
        .query_as(
            "SELECT * FROM deployments WHERE deployment_id = $1",
            vec![Param::Text(release_id.to_string())],
        )
        .await?;
    Ok(rows.into_iter().next())
}

pub async fn insert_deployment(client: &Client, d: &Deployment) -> Result<()> {
    client
        .execute(
            Cow::Borrowed(
                "INSERT INTO deployments (deployment_id, deployment_key, tenant_id, app_id, env_id, \
                 release_sha, artifact_ref, lane, status, app_slug, env_slug, desired_routes, \
                 endpoints, created_at, updated_at) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
            ),
            vec![
                Param::Text(d.deployment_id.clone()),
                Param::Text(d.deployment_key.clone()),
                Param::Text(d.tenant_id.clone()),
                Param::Text(d.app_id.clone()),
                Param::Text(d.env_id.clone()),
                Param::Text(d.release_sha.clone()),
                Param::Text(d.artifact_ref.clone()),
                Param::Text(d.lane.clone()),
                Param::Text(d.status.clone()),
                d.app_slug.clone().map(Param::Text).unwrap_or(Param::Null),
                d.env_slug.clone().map(Param::Text).unwrap_or(Param::Null),
                d.desired_routes
                    .clone()
                    .map(Param::Text)
                    .unwrap_or(Param::Null),
                d.endpoints.clone().map(Param::Text).unwrap_or(Param::Null),
                Param::Text(d.created_at.clone()),
                Param::Text(d.updated_at.clone()),
            ],
        )
        .await?;
    Ok(())
}

pub async fn update_status(client: &Client, deployment_id: &str, status: &str) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    client
        .execute(
            Cow::Borrowed(
                "UPDATE deployments SET status = $1, updated_at = $2 WHERE deployment_id = $3",
            ),
            vec![
                Param::Text(status.to_string()),
                Param::Text(now),
                Param::Text(deployment_id.to_string()),
            ],
        )
        .await?;
    Ok(())
}

pub async fn add_event(
    client: &Client,
    deployment_id: &str,
    event_type: &str,
    message: Option<&str>,
) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339();
    client
        .execute(
            Cow::Borrowed(
                "INSERT INTO deployment_events (deployment_id, event_type, message, timestamp) \
                 VALUES ($1, $2, $3, $4)",
            ),
            vec![
                Param::Text(deployment_id.to_string()),
                Param::Text(event_type.to_string()),
                message
                    .map(|m| Param::Text(m.to_string()))
                    .unwrap_or(Param::Null),
                Param::Text(now),
            ],
        )
        .await?;
    Ok(())
}

pub async fn get_events(client: &Client, deployment_id: &str) -> Result<Vec<DeploymentEvent>> {
    client
        .query_as(
            "SELECT id, deployment_id, event_type, message, timestamp \
             FROM deployment_events WHERE deployment_id = $1 ORDER BY id",
            vec![Param::Text(deployment_id.to_string())],
        )
        .await
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T011 — `apply_hql_env` writes the required HQL_* keys + dev-fallback
    /// ENC_KEYS when no `DEPLOYD_BACKUP_*` is set (steady-state, no opt-in).
    /// This is the single env-mutation test in this binary; no other test
    /// in the same module touches HQL_* env vars (config.rs tests use the
    /// closure-based `from_var_lookup` path, not the env path).
    #[test]
    fn apply_hql_env_dev_fallback_path() {
        // SAFETY: single-threaded test path; no concurrent env access.
        unsafe {
            for k in [
                "DEPLOYD_BACKUP_S3_ENDPOINT",
                "DEPLOYD_BACKUP_S3_BUCKET",
                "DEPLOYD_BACKUP_S3_REGION",
                "DEPLOYD_BACKUP_S3_ACCESS_KEY",
                "DEPLOYD_BACKUP_S3_SECRET_KEY",
                "DEPLOYD_BACKUP_CRYPTR_KEYRING",
                "DEPLOYD_BACKUP_CRYPTR_ACTIVE_KEY",
            ] {
                std::env::remove_var(k);
            }
        }

        apply_hql_env("/tmp/deployd-test-data-dir").expect("apply_hql_env should succeed");

        assert_eq!(std::env::var("HQL_NODE_ID").unwrap(), "1");
        assert_eq!(
            std::env::var("HQL_NODES").unwrap(),
            "1 127.0.0.1:7001 127.0.0.1:7002"
        );
        assert_eq!(
            std::env::var("HQL_DATA_DIR").unwrap(),
            "/tmp/deployd-test-data-dir"
        );
        assert_eq!(std::env::var("HQL_FILENAME_DB").unwrap(), "deployd.db");
        assert!(std::env::var("HQL_SECRET_RAFT").unwrap().len() >= 16);
        assert!(std::env::var("HQL_SECRET_API").unwrap().len() >= 16);
        assert_eq!(std::env::var("HQL_LOG_STATEMENTS").unwrap(), "false");

        // Dev-fallback ENC_KEYS path (no DEPLOYD_BACKUP_* opt-in).
        assert_eq!(
            std::env::var("ENC_KEY_ACTIVE").unwrap(),
            DEV_FALLBACK_ENC_KEY_ACTIVE
        );
        assert_eq!(std::env::var("ENC_KEYS").unwrap(), DEV_FALLBACK_ENC_KEYS);
    }

    /// T012 — End-to-end restore validation against a real S3-compatible
    /// endpoint (localstack / minio). `#[ignore]`-gated; runs only when
    /// the operator has explicitly configured an S3 endpoint AND
    /// `HQL_BACKUP_RESTORE=s3:<known-key>` AND the supporting `HQL_S3_*` /
    /// `ENC_KEYS` / `ENC_KEY_ACTIVE` envs.
    ///
    /// Procedure:
    /// 1. Stand up a localstack/minio bucket; populate with one snapshot
    ///    (e.g. via a manual `curl PUT` of a known sqlite file named
    ///    `backup_node_1_<unix-ts>.sqlite`).
    /// 2. Export DEPLOYD_BACKUP_* env vars + HQL_BACKUP_RESTORE.
    /// 3. `cargo test --manifest-path platform/services/deployd-api-rs/Cargo.toml
    ///    --test-threads=1 -- --ignored restore_from_env_var`.
    ///
    /// Asserts: the data dir contents include `state_machine/db/deployd.db`
    /// after init_db returns Ok — implying hiqlite's auto-restore ran.
    #[tokio::test]
    #[ignore = "requires real S3-compatible endpoint + populated bucket; run manually"]
    async fn restore_from_env_var() {
        let tmp = std::env::var("DEPLOYD_TEST_DATA_DIR")
            .expect("set DEPLOYD_TEST_DATA_DIR to a writable empty path");
        std::fs::create_dir_all(&tmp).expect("test data dir");

        // Caller is responsible for setting HQL_BACKUP_RESTORE=s3:<key>
        // and DEPLOYD_BACKUP_* env vars before invoking the test.
        let _client = init_db(&tmp).await.expect("init_db with HQL_BACKUP_RESTORE should restore + init");

        let db_path = std::path::Path::new(&tmp).join("state_machine/db/deployd.db");
        assert!(
            db_path.exists(),
            "expected restored sqlite db at {db_path:?}"
        );
    }
}
