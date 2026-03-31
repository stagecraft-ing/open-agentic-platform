// SPDX-License-Identifier: AGPL-3.0-or-later
// Feature 052 Phase 6: HTTP surface for workflow state and events.
//
// This module exposes an Axum router with a single SSE endpoint:
//
//   GET /workflows/:id/events?offset=0
//
// It wraps `EventBroadcaster::subscribe_with_replay` and frames output as
// `text/event-stream` (SSE) so that clients can replay historical events from
// any offset and then receive live updates.

use crate::sse::{EventBroadcaster, ReplaySubscription};
use crate::sqlite_state::SqliteWorkflowStore;
use crate::OrchestratorError;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::Router;
use dashmap::DashMap;
use futures_util::stream::{self, StreamExt};
use futures_util::Stream;
use serde::Deserialize;
use std::convert::Infallible;
use std::time::Duration;
use std::{sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Shared state for the HTTP server: a SQLite store and a registry mapping
/// `workflow_id` → `EventBroadcaster`.
#[derive(Clone)]
pub struct HttpState {
    pub store: Arc<Mutex<SqliteWorkflowStore>>,
    pub broadcasters: Arc<DashMap<Uuid, EventBroadcaster>>,
}

/// Query parameters for the SSE endpoint.
#[derive(Debug, Deserialize)]
pub struct EventsQuery {
    /// Starting event offset (inclusive of `from_event_id + 1`).
    #[serde(default)]
    pub offset: i64,
}

/// Builds an Axum router with the workflow events SSE endpoint mounted.
pub fn router(state: HttpState) -> Router {
    Router::new()
        .route(
            "/workflows/:id/events",
            axum::routing::get(workflow_events_sse),
        )
        .with_state(state)
}

async fn workflow_events_sse(
    State(state): State<HttpState>,
    Path(workflow_id): Path<Uuid>,
    Query(query): Query<EventsQuery>,
) -> impl IntoResponse {
    // Look up the broadcaster for this workflow.
    let broadcaster = match state.broadcasters.get(&workflow_id) {
        Some(entry) => entry.clone(),
        None => return (StatusCode::NOT_FOUND, "workflow not found").into_response(),
    };

    // Load replay + live subscription from SQLite.
    let replay_subscription = {
        let mut guard = state.store.lock().await;
        match broadcaster.subscribe_with_replay(&mut *guard, workflow_id, query.offset) {
            Ok(sub) => sub,
            Err(OrchestratorError::StatePersistence { reason }) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("state persistence error: {reason}"),
                )
                    .into_response()
            }
            Err(other) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("error subscribing to events: {other}"),
                )
                    .into_response()
            }
        }
    };

    let stream = build_sse_stream(replay_subscription);

    Sse::new(stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response()
}

fn build_sse_stream(
    sub: ReplaySubscription,
) -> impl Stream<Item = Result<Event, Infallible>> + Send + 'static {
    let ReplaySubscription {
        replay,
        high_water_mark,
        mut subscriber,
    } = sub;

    // First, send all replay events in order.
    let replay_iter = replay.into_iter().map(|event| {
        let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
        Ok(Event::default()
            .id(event.event_id.to_string())
            .json_data(json)
            .unwrap_or_else(|_| Event::default().data("{}")))
    });

    // Then, stream live events, skipping those with event_id <= high_water_mark.
    let live_stream = async_stream::stream! {
        let mut current_hwm = high_water_mark;
        loop {
            match subscriber.recv().await {
                Ok(event) => {
                    if event.event_id <= current_hwm {
                        continue;
                    }
                    current_hwm = event.event_id;
                    let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
                    let event = Event::default()
                        .id(event.event_id.to_string())
                        .json_data(json)
                        .unwrap_or_else(|_| Event::default().data("{}"));
                    yield Ok(event);
                }
                Err(_lagged) => {
                    // On lagged subscribers, clients should reconnect with a higher offset.
                    continue;
                }
            }
        }
    };

    let replay_stream = stream::iter(replay_iter);
    replay_stream.chain(live_stream)
}

