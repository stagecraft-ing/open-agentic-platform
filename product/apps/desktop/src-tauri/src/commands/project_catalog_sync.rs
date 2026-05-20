//! Desktop side of the workspace project catalog (spec 112 §7 / Phase 8).
//!
//! Stagecraft broadcasts `project.catalog.upsert` envelopes whenever a
//! project is created, imported, deleted, or replayed during a handshake
//! snapshot. This module registers a dispatch-table handler that
//! projects each frame onto a Tauri event the frontend listens for, so
//! the OPC Projects panel updates without a restart and without polling.
//!
//! Authority invariant (spec 087 §5.3 / 112 §7): the desktop never
//! originates the upsert — those are stagecraft-owned mutations. We
//! mirror state into an in-memory frontend store. Restart or
//! reconnect re-runs the handshake snapshot, so a missed upsert never
//! leaves the panel permanently stale.
//!
//! Like the agent catalog sync (`agent_catalog_sync.rs`), the path is
//! best-effort: malformed envelopes log a warning and drop the frame
//! rather than killing the consumer.

use std::sync::Arc;

use log::{info, warn};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::sync_client::{
    FnHandler, ProjectCatalogRepo, ServerEnvelopeWire, SyncClientState,
};

/// Tauri event emitted when a project upsert (or tombstone) arrives.
pub const EVENT_PROJECT_CATALOG_UPSERT: &str = "project-catalog-upsert";

/// Flat projection of a `project.catalog.upsert` envelope. The
/// frontend store maintains entries keyed on `project_id`; tombstones
/// drop the row.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCatalogUpsertEvent {
    pub project_id: String,
    pub org_id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub factory_adapter_id: Option<String>,
    pub detection_level: Option<String>,
    pub repo: Option<ProjectCatalogRepo>,
    pub opc_deep_link: String,
    pub tombstone: bool,
    pub updated_at: String,
}

/// Pull a [`ProjectCatalogUpsertEvent`] out of a server envelope.
/// Returns `None` when a required field is missing — a drifted server
/// that emits an incomplete frame should not crash the dispatcher.
pub fn extract_upsert(env: &ServerEnvelopeWire) -> Option<ProjectCatalogUpsertEvent> {
    let project_id = env.project_id.clone()?;
    let org_id = env
        .org_id
        .clone()
        .or_else(|| {
            // Fall back to the meta org id if the payload omits the
            // explicit field. Both stagecraft snapshots and live broadcasts
            // populate the payload form, but accepting the meta form keeps
            // the parser tolerant.
            let m = &env.meta.org_id;
            if m.is_empty() { None } else { Some(m.clone()) }
        })?;
    let name = env.name.clone()?;
    let slug = env.slug.clone().unwrap_or_default();
    let description = env.description.clone().unwrap_or_default();
    let factory_adapter_id = env.factory_adapter_id.clone();
    let detection_level = env.detection_level.clone();
    let repo = env.repo.clone();
    let opc_deep_link = env.opc_deep_link.clone().unwrap_or_default();
    let tombstone = env.tombstone.unwrap_or(false);
    let updated_at = env.updated_at.clone().unwrap_or_default();

    Some(ProjectCatalogUpsertEvent {
        project_id,
        org_id,
        name,
        slug,
        description,
        factory_adapter_id,
        detection_level,
        repo,
        opc_deep_link,
        tombstone,
        updated_at,
    })
}

/// Install the Phase 8 handler on the shared dispatch table. No-op
/// (plus a single info log) when the duplex `SyncClientState` has not
/// been managed yet — the consumer wires it up at app startup, so this
/// is a defensive guard rather than a normal path.
pub fn register_project_catalog_handlers(app: AppHandle) {
    if app.try_state::<SyncClientState>().is_none() {
        warn!("project_catalog_sync: SyncClientState not managed — cannot register handlers");
        return;
    }

    let dispatch = app.state::<SyncClientState>().dispatch_table();

    let app_handle = app.clone();
    let handler = FnHandler(move |env: &ServerEnvelopeWire| {
        on_project_upsert(app_handle.clone(), env);
    });
    dispatch.register("project.catalog.upsert", Arc::new(handler));

    info!("project_catalog_sync: dispatch handler registered");
}

fn on_project_upsert(app: AppHandle, env: &ServerEnvelopeWire) {
    let Some(payload) = extract_upsert(env) else {
        warn!("project.catalog.upsert missing required fields — ignored");
        return;
    };
    if let Err(e) = app.emit(EVENT_PROJECT_CATALOG_UPSERT, &payload) {
        warn!("project.catalog.upsert: failed to emit frontend event: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::sync_client::ServerMeta;
    use serde_json::{Value as JsonValue, json};

    fn server_envelope(kind: &str, payload: JsonValue) -> ServerEnvelopeWire {
        let mut wrapped = json!({
            "kind": kind,
            "meta": {
                "v": 1,
                "eventId": "e1",
                "sentAt": "2026-04-27T00:00:00Z",
                "orgCursor": "cur-1",
                "orgId": "org-1"
            }
        })
        .as_object()
        .unwrap()
        .clone();
        if let Some(obj) = payload.as_object() {
            for (k, v) in obj {
                wrapped.insert(k.clone(), v.clone());
            }
        }
        serde_json::from_value(JsonValue::Object(wrapped)).expect("envelope parse")
    }

    #[test]
    fn extract_upsert_projects_required_fields() {
        let env = server_envelope(
            "project.catalog.upsert",
            json!({
                "projectId": "p-1",
                "orgId": "org-1",
                "name": "Alpha",
                "slug": "alpha",
                "description": "first",
                "factoryAdapterId": "ad-1",
                "detectionLevel": "scaffold_only",
                "repo": {
                    "githubOrg": "acme",
                    "repoName": "alpha",
                    "defaultBranch": "main",
                    "cloneUrl": "https://github.com/acme/alpha.git",
                    "htmlUrl": "https://github.com/acme/alpha"
                },
                "opcDeepLink": "opc://project/open?project_id=p-1",
                "tombstone": false,
                "updatedAt": "2026-04-27T00:00:00Z"
            }),
        );
        let u = extract_upsert(&env).expect("parses");
        assert_eq!(u.project_id, "p-1");
        assert_eq!(u.org_id, "org-1");
        assert_eq!(u.name, "Alpha");
        assert_eq!(u.slug, "alpha");
        assert_eq!(u.factory_adapter_id.as_deref(), Some("ad-1"));
        assert_eq!(u.detection_level.as_deref(), Some("scaffold_only"));
        let repo = u.repo.expect("repo present");
        assert_eq!(repo.github_org, "acme");
        assert_eq!(repo.repo_name, "alpha");
        assert!(!u.tombstone);
    }

    #[test]
    fn extract_upsert_falls_back_to_meta_org_id() {
        let mut env = server_envelope(
            "project.catalog.upsert",
            json!({
                "projectId": "p-1",
                "name": "Alpha",
                "tombstone": false
            }),
        );
        // Drop the payload orgId so the meta fallback kicks in.
        env.org_id = None;
        let u = extract_upsert(&env).expect("parses with meta fallback");
        assert_eq!(u.org_id, "org-1");
    }

    #[test]
    fn extract_upsert_returns_none_on_missing_project_id() {
        let env = server_envelope(
            "project.catalog.upsert",
            json!({
                "name": "Alpha",
                "orgId": "org-1"
            }),
        );
        assert!(extract_upsert(&env).is_none());
    }

    #[test]
    fn extract_upsert_rejects_empty_org_in_meta_and_payload() {
        let mut env = server_envelope(
            "project.catalog.upsert",
            json!({
                "projectId": "p-1",
                "name": "Alpha"
            }),
        );
        env.org_id = None;
        env.meta = ServerMeta {
            v: 1,
            event_id: "e1".into(),
            sent_at: "2026-04-27T00:00:00Z".into(),
            correlation_id: None,
            causation_id: None,
            org_cursor: "cur-1".into(),
            org_id: "".into(),
        };
        assert!(extract_upsert(&env).is_none());
    }

    #[test]
    fn extract_upsert_carries_tombstone_flag() {
        let env = server_envelope(
            "project.catalog.upsert",
            json!({
                "projectId": "p-1",
                "orgId": "org-1",
                "name": "Alpha",
                "tombstone": true
            }),
        );
        let u = extract_upsert(&env).expect("parses");
        assert!(u.tombstone);
    }
}
