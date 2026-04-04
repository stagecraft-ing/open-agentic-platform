use anyhow::Result;
use hiqlite::{Client, Node, NodeConfig, Param};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use crate::config::Config;

pub struct AppState {
    pub client: Client,
    pub config: Config,
}

pub async fn init_db(data_dir: &str) -> Result<Client> {
    let config = NodeConfig {
        node_id: 1,
        nodes: vec![Node {
            id: 1,
            addr_raft: "127.0.0.1:7001".to_string(),
            addr_api: "127.0.0.1:7002".to_string(),
        }],
        data_dir: Cow::Owned(data_dir.to_string()),
        filename_db: Cow::Borrowed("deployd.db"),
        secret_raft: std::env::var("HIQLITE_SECRET_RAFT")
            .unwrap_or_else(|_| "deployd-raft-secret-key".to_string()),
        secret_api: std::env::var("HIQLITE_SECRET_API")
            .unwrap_or_else(|_| "deployd-api-secret-key0".to_string()),
        log_statements: false,
        ..NodeConfig::default()
    };
    let client = hiqlite::start_node(config).await?;
    migrate(&client).await?;
    Ok(client)
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
