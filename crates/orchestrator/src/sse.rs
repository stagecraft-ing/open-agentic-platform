// SPDX-License-Identifier: AGPL-3.0-or-later
// Feature 052 Phase 5: SSE event broadcaster with offset-based replay.
//
// This module provides the multi-subscriber broadcast layer that sits between
// the SQLite event store and HTTP SSE endpoints.  It satisfies:
//   FR-006  — live + replay streaming of workflow events
//   NF-002  — ≥ 50 concurrent subscribers per workflow
//   SC-004  — offset=0 yields all historical events then live

use crate::sqlite_state::{PersistedEvent, SqliteWorkflowStore};
use crate::OrchestratorError;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Default broadcast channel capacity — sized for NF-002 (50 concurrent
/// subscribers) with headroom for burst writes before slow readers catch up.
const DEFAULT_CHANNEL_CAPACITY: usize = 1024;

/// Multi-subscriber event broadcaster for a single workflow.
///
/// Internally wraps a `tokio::sync::broadcast` channel.  The broadcaster is
/// `Clone + Send + Sync` so it can be shared across an async HTTP server.
///
/// # Usage
///
/// 1. Create a broadcaster per active workflow.
/// 2. After each `SqliteWorkflowStore::append_event` call, call
///    `broadcaster.broadcast(event)` to push the event to all live subscribers.
/// 3. Clients call `subscribe_with_replay` to get a combined stream of
///    historical (from SQLite) + live events.
#[derive(Clone)]
pub struct EventBroadcaster {
    tx: broadcast::Sender<PersistedEvent>,
}

/// A handle for receiving live broadcast events.
///
/// Obtained via `EventBroadcaster::subscribe`.  Wraps a
/// `tokio::sync::broadcast::Receiver`.
pub struct EventSubscriber {
    rx: broadcast::Receiver<PersistedEvent>,
}

/// Combined replay + live event stream.
///
/// Contains the historical events (loaded eagerly from SQLite) and a live
/// subscriber.  The consumer should drain `replay` first, then `recv()` from
/// the subscriber for live events, skipping any whose `event_id` was already
/// covered by the replay window.
pub struct ReplaySubscription {
    /// Historical events loaded from SQLite, ordered by ascending event_id.
    pub replay: Vec<PersistedEvent>,
    /// The highest event_id in `replay`, or the requested `from_event_id` if
    /// replay is empty.  Live events with `event_id <= high_water_mark` should
    /// be deduplicated (skipped).
    pub high_water_mark: i64,
    /// Live event receiver — call `recv().await` after draining `replay`.
    pub subscriber: EventSubscriber,
}

impl EventBroadcaster {
    /// Creates a new broadcaster with the default channel capacity.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CHANNEL_CAPACITY)
    }

    /// Creates a new broadcaster with a specific channel capacity.
    ///
    /// Each subscriber can lag behind by at most `capacity` events before
    /// receiving a `Lagged` error on the next `recv`.
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Broadcasts an event to all current subscribers.
    ///
    /// Returns the number of subscribers that received the event.
    /// Returns 0 (not an error) when there are no active subscribers.
    pub fn broadcast(&self, event: PersistedEvent) -> usize {
        // `send` returns Err only when there are zero receivers, which is
        // normal (no subscribers yet).  We map that to 0.
        self.tx.send(event).unwrap_or(0)
    }

    /// Returns the number of currently active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Creates a live-only subscriber (no replay).
    pub fn subscribe(&self) -> EventSubscriber {
        EventSubscriber {
            rx: self.tx.subscribe(),
        }
    }

    /// Creates a combined replay + live subscription (SC-004).
    ///
    /// 1. Subscribes to the broadcast channel **first** (to avoid missing
    ///    events between the SQLite read and the subscription).
    /// 2. Loads historical events from SQLite where `event_id > from_event_id`.
    /// 3. Returns a `ReplaySubscription` containing the historical events and
    ///    the live receiver.
    ///
    /// The caller must deduplicate: after draining `replay`, skip any live
    /// events with `event_id <= high_water_mark`.
    pub fn subscribe_with_replay(
        &self,
        store: &SqliteWorkflowStore,
        workflow_id: Uuid,
        from_event_id: i64,
    ) -> Result<ReplaySubscription, OrchestratorError> {
        // Subscribe first so we don't miss events written between the SQLite
        // read and subscription.
        let subscriber = self.subscribe();

        // Load historical events from the SQLite store.
        let replay = store.load_events_since(workflow_id, from_event_id, None)?;

        let high_water_mark = replay
            .last()
            .map(|e| e.event_id)
            .unwrap_or(from_event_id);

        Ok(ReplaySubscription {
            replay,
            high_water_mark,
            subscriber,
        })
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSubscriber {
    /// Receives the next live event.
    ///
    /// Returns `Ok(event)` on success.  Returns `Err(RecvError::Lagged(n))`
    /// if this subscriber fell behind by `n` events (the missed events are
    /// lost and the subscriber resumes from the next available event).
    /// Returns `Err(RecvError::Closed)` when the broadcaster is dropped.
    pub async fn recv(&mut self) -> Result<PersistedEvent, broadcast::error::RecvError> {
        self.rx.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::WorkflowState;
    use serde_json::Value as JsonValue;

    /// Helper: open a temp SQLite store and seed a workflow row.
    fn setup_store_with_workflow() -> (tempfile::TempDir, SqliteWorkflowStore, Uuid) {
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("state.sqlite");
        let mut store = SqliteWorkflowStore::open(&db_path).expect("open store");
        let wf_id = Uuid::new_v4();
        let state = WorkflowState::new(
            wf_id,
            "sse-test",
            "2026-03-31T00:00:00Z".to_string(),
            Vec::<(String, String)>::new(),
            serde_json::Map::new(),
        );
        store.write_workflow_state(&state).expect("seed workflow");
        (tmp, store, wf_id)
    }

    #[test]
    fn broadcast_with_no_subscribers_returns_zero() {
        let broadcaster = EventBroadcaster::new();
        let event = PersistedEvent {
            event_id: 1,
            workflow_id: Uuid::new_v4(),
            timestamp: "1".to_string(),
            event_type: "step_started".to_string(),
            payload: JsonValue::String("test".to_string()),
        };
        assert_eq!(broadcaster.broadcast(event), 0);
    }

    #[tokio::test]
    async fn live_subscriber_receives_broadcast_events() {
        let broadcaster = EventBroadcaster::new();
        let mut sub = broadcaster.subscribe();
        let wf_id = Uuid::new_v4();

        let event = PersistedEvent {
            event_id: 42,
            workflow_id: wf_id,
            timestamp: "1".to_string(),
            event_type: "step_completed".to_string(),
            payload: JsonValue::String("done".to_string()),
        };
        let count = broadcaster.broadcast(event.clone());
        assert_eq!(count, 1);

        let received = sub.recv().await.expect("recv");
        assert_eq!(received.event_id, 42);
        assert_eq!(received.event_type, "step_completed");
    }

    #[tokio::test]
    async fn multiple_subscribers_all_receive_events() {
        let broadcaster = EventBroadcaster::new();
        let mut subs: Vec<_> = (0..50).map(|_| broadcaster.subscribe()).collect();
        assert_eq!(broadcaster.subscriber_count(), 50);

        let event = PersistedEvent {
            event_id: 1,
            workflow_id: Uuid::new_v4(),
            timestamp: "1".to_string(),
            event_type: "test".to_string(),
            payload: JsonValue::Null,
        };
        let count = broadcaster.broadcast(event);
        assert_eq!(count, 50);

        for sub in &mut subs {
            let e = sub.recv().await.expect("recv");
            assert_eq!(e.event_id, 1);
        }
    }

    #[test]
    fn subscribe_with_replay_returns_historical_events() {
        let (_tmp, mut store, wf_id) = setup_store_with_workflow();
        let broadcaster = EventBroadcaster::new();

        // Append 3 events to SQLite.
        for i in 1..=3 {
            store
                .append_event(
                    wf_id,
                    &format!("event_{i}"),
                    &JsonValue::from(i),
                    Some(format!("t{i}")),
                )
                .expect("append");
        }

        let sub = broadcaster
            .subscribe_with_replay(&store, wf_id, 0)
            .expect("replay");

        assert_eq!(sub.replay.len(), 3);
        assert_eq!(sub.replay[0].event_type, "event_1");
        assert_eq!(sub.replay[2].event_type, "event_3");
        assert_eq!(sub.high_water_mark, sub.replay[2].event_id);
    }

    #[test]
    fn subscribe_with_replay_partial_offset() {
        let (_tmp, mut store, wf_id) = setup_store_with_workflow();
        let broadcaster = EventBroadcaster::new();

        let e1 = store
            .append_event(wf_id, "a", &JsonValue::Null, Some("t1".to_string()))
            .expect("append");
        let _e2 = store
            .append_event(wf_id, "b", &JsonValue::Null, Some("t2".to_string()))
            .expect("append");
        let e3 = store
            .append_event(wf_id, "c", &JsonValue::Null, Some("t3".to_string()))
            .expect("append");

        // Replay from after e1 — should get e2 and e3.
        let sub = broadcaster
            .subscribe_with_replay(&store, wf_id, e1)
            .expect("replay");

        assert_eq!(sub.replay.len(), 2);
        assert_eq!(sub.replay[0].event_type, "b");
        assert_eq!(sub.replay[1].event_type, "c");
        assert_eq!(sub.high_water_mark, e3);
    }

    #[test]
    fn subscribe_with_replay_empty_history() {
        let (_tmp, store, wf_id) = setup_store_with_workflow();
        let broadcaster = EventBroadcaster::new();

        let sub = broadcaster
            .subscribe_with_replay(&store, wf_id, 0)
            .expect("replay");

        assert!(sub.replay.is_empty());
        assert_eq!(sub.high_water_mark, 0);
    }

    #[tokio::test]
    async fn replay_then_live_deduplication_pattern() {
        let (_tmp, mut store, wf_id) = setup_store_with_workflow();
        let broadcaster = EventBroadcaster::new();

        // Append 2 events before subscribing.
        store
            .append_event(wf_id, "old_1", &JsonValue::Null, Some("t1".to_string()))
            .expect("append");
        let e2 = store
            .append_event(wf_id, "old_2", &JsonValue::Null, Some("t2".to_string()))
            .expect("append");

        let mut sub = broadcaster
            .subscribe_with_replay(&store, wf_id, 0)
            .expect("replay");

        assert_eq!(sub.replay.len(), 2);
        let hwm = sub.high_water_mark;
        assert_eq!(hwm, e2);

        // Simulate: broadcaster sends a duplicate (e2) and a new event (e3).
        let dup_event = PersistedEvent {
            event_id: e2,
            workflow_id: wf_id,
            timestamp: "t2".to_string(),
            event_type: "old_2".to_string(),
            payload: JsonValue::Null,
        };
        broadcaster.broadcast(dup_event);

        let new_event = PersistedEvent {
            event_id: e2 + 1,
            workflow_id: wf_id,
            timestamp: "t3".to_string(),
            event_type: "new_3".to_string(),
            payload: JsonValue::Null,
        };
        broadcaster.broadcast(new_event);

        // Consumer pattern: skip events <= high_water_mark.
        let live1 = sub.subscriber.recv().await.expect("recv");
        assert!(live1.event_id <= hwm, "should be dedup'd");

        let live2 = sub.subscriber.recv().await.expect("recv");
        assert!(live2.event_id > hwm);
        assert_eq!(live2.event_type, "new_3");
    }

    #[tokio::test]
    async fn cross_workflow_isolation_in_replay() {
        let (_tmp, mut store, wf_a) = setup_store_with_workflow();
        let broadcaster = EventBroadcaster::new();

        // Create a second workflow.
        let wf_b = Uuid::new_v4();
        let state_b = WorkflowState::new(
            wf_b,
            "other",
            "2026-03-31T00:00:00Z".to_string(),
            Vec::<(String, String)>::new(),
            serde_json::Map::new(),
        );
        store.write_workflow_state(&state_b).expect("seed wf_b");

        // Append events to both workflows.
        store
            .append_event(wf_a, "a_event", &JsonValue::Null, Some("t1".to_string()))
            .expect("append a");
        store
            .append_event(wf_b, "b_event", &JsonValue::Null, Some("t2".to_string()))
            .expect("append b");

        // Replay for workflow A should only see A's events.
        let sub_a = broadcaster
            .subscribe_with_replay(&store, wf_a, 0)
            .expect("replay a");
        assert_eq!(sub_a.replay.len(), 1);
        assert_eq!(sub_a.replay[0].event_type, "a_event");

        // Replay for workflow B should only see B's events.
        let sub_b = broadcaster
            .subscribe_with_replay(&store, wf_b, 0)
            .expect("replay b");
        assert_eq!(sub_b.replay.len(), 1);
        assert_eq!(sub_b.replay[0].event_type, "b_event");
    }

    #[test]
    fn subscriber_count_tracks_active_receivers() {
        let broadcaster = EventBroadcaster::new();
        assert_eq!(broadcaster.subscriber_count(), 0);

        let _s1 = broadcaster.subscribe();
        assert_eq!(broadcaster.subscriber_count(), 1);

        let _s2 = broadcaster.subscribe();
        assert_eq!(broadcaster.subscriber_count(), 2);

        drop(_s1);
        assert_eq!(broadcaster.subscriber_count(), 1);
    }
}
