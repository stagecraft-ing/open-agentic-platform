# Sync Protocol — Self-Reflection & Alignment Report

**Scope:** First-pass outbox/inbox sync substrate in `platform/services/stagecraft/api/sync/`.
**Date:** 2026-04-20
**Verdict:** **Structurally aligned, operationally incomplete.**

This document measures the implementation against the architectural intent laid
out in the task brief and spec 087. It is written to be honest about what is
*real* in the codebase versus what is *stubbed, best-effort, or deferred*.

---

## What exists after this change

### Files added

| File | Purpose |
|---|---|
| `types.ts` | Discriminated unions for `ClientEnvelope`, `ServerEnvelope`, handshake, meta. `isClientEnvelope` type guard. |
| `registry.ts` | In-memory `workspaceId → clientId → Session` registry with `sendTo`, `broadcastWorkspace`, auto-pruning of failed streams. |
| `store.ts` | `InboxStore` / `OutboxStore` / `CursorIssuer` interfaces with in-memory implementations. |
| `service.ts` | `handleInbound`, `publishAck`, `publishNack`, `dispatchServerEvent`. Only module that mints cursors. |
| `duplex.ts` | Authenticated `api.streamInOut` endpoint at `POST /api/sync/duplex` with handshake, `sync.hello`, heartbeat, cursor-gap detection. |
| `relay.ts` | PubSub subscriber that maps `FactoryEventTopic` events onto `dispatchServerEvent`, with `projectId → workspaceId` cache. |
| `types.test.ts`, `registry.test.ts`, `store.test.ts` | 21 unit tests — all passing. |

### Files touched

| File | Change |
|---|---|
| `vite.config.ts` | Aliased `encore.dev/log` to a test mock so pure units can run under bare `vitest`. |
| `test/__mocks__/encore-log.ts` | New no-op logger mock. |

### Files intentionally **not** modified

| File | Why |
|---|---|
| `sync.ts` (existing `streamOut` + HTTP ingest + old relay) | Actively consumed by `web/`, `apps/desktop`, and `packages/workspace-sdk`. Task says no gratuitous refactoring. Both paths coexist; the new duplex is additive. |

---

## A. Authority alignment

| Concern | Status |
|---|---|
| Stagecraft remains the audit authority | **Yes.** `audit.candidate` from the desktop is **never** written verbatim — `service.ts` normalises the action (`opc.*` prefix), stamps `actor_user_id` from the authenticated JWT, and injects server-side metadata (`clientId`, `workspaceId`, `clientEventId`). The desktop cannot forge `actor_user_id`. |
| Stagecraft remains authoritative for policy, grants, deploy state, workspace state | **Yes.** These are *outbound-only* envelope variants (`policy.updated`, `grant.updated`, `deploy.status`, `workspace.updated`, `project.updated`). There is no inbound client variant that would let the desktop mutate them through this channel. |
| Desktop/OPC authority for local execution/checkpoints/artifacts/runtime | **Yes.** These are *inbound-only* variants (`execution.status`, `checkpoint.created`, `artifact.emitted`, `runtime.observed`, `agent.invocation`). The server records them, but does not treat them as control-plane truth. |
| Authority split is explicit in the type system | **Yes.** `ClientEnvelope` and `ServerEnvelope` are disjoint unions; there is no shared "sync event" supertype blurring the directions. |

**Verdict: Fully aligned.**

---

## B. Sync model alignment

| Concern | Status |
|---|---|
| Application-layer sync, not DB replication | **Yes.** There is no replication of Postgres to Hiqlite. The transport is a typed event stream; each side persists whatever it is authoritative for. |
| Event directions and boundaries explicit | **Yes.** See disjoint envelope unions above. |
| No "generic sync everything" pipe | **Yes.** The union is intentionally small — seven inbound variants, ten outbound. Extending it requires adding a named variant, not a free-form field. |

**Verdict: Fully aligned.**

---

## C. Auth alignment

| Concern | Status |
|---|---|
| Stream authenticated | **Yes.** `api.streamInOut` is opened with `auth: true`. The global Encore gateway runs the Rauthy JWT validator before the handler executes. No `auth: false` bootstrap compromise. |
| Workspace from token, not handshake | **Yes.** `duplex.ts` reads `workspaceId` from `getAuthData()`, NEVER from the handshake. The client cannot subscribe to a workspace it does not own, even if it sends a different workspaceId in the handshake. |
| Disabled-user enforcement | **Yes**, transitively — the Rauthy auth handler already rejects disabled users (FR-025). |

**Gaps:**
- WebSocket `auth: true` relies on Encore routing the upgrade request through the gateway authHandler. If that is bypassed in a particular deployment (e.g., direct pod access), the stream would be open. Mitigated by Helm/ingress policy, not by this file.
- The authenticated user's **role-in-workspace** (owner/admin/member) is not gated — any authenticated user whose JWT carries a workspaceId can open a stream in that workspace. For production we likely want a membership check.

**Verdict: Fully aligned for identity, partially aligned for fine-grained authorization.**

---

## D. Persistence alignment

| Concern | Status |
|---|---|
| Durable outbox | **No.** `OutboxStore` is an in-memory ring buffer capped at 500 events per workspace. Stagecraft restart wipes it. |
| Durable inbox | **No.** `InboxStore` is a 1,000-entry ring buffer. |
| Audit events for `audit.candidate` | **Yes, durable.** Persisted via Drizzle into the real `audit_log` table. This is the one piece of inbound data that survives a restart. |
| Persistent cursor | **No.** Cursors live in an in-memory `Map<string, bigint>`, reset on restart. |

**Failure/recovery limitations that remain:**
- A stagecraft restart drops all unacked server events. Clients reconnecting with an old `lastServerCursor` will get `sync.resync_required` because the new process peeks an empty cursor map (the existing hello logic already treats cursor mismatch as a gap).
- The desktop cannot reliably recover from a restart without either (a) re-requesting state via existing REST endpoints or (b) waiting for the next organic event.

**What's required to move forward:**
- A `sync_outbox` Postgres table with `(workspace_id, cursor, event_id, payload, created_at)`, one row per server event.
- A `sync_outbox_delivery` table with `(workspace_id, event_id, client_id, acked_at)` to persist ACKs and drive redelivery on reconnect.
- Replace `InMemoryOutbox`, `InMemoryInbox`, `MonotonicCursorIssuer` with Drizzle-backed implementations. The interface boundary in `store.ts` was designed precisely for this swap.

**Verdict: Structurally aligned, operationally incomplete.**

---

## E. Delivery semantics honesty

What actually exists right now:

| Property | Real? |
|---|---|
| At-most-once in-process, best-effort across reconnects | ✅ |
| Per-inbound-event server ACK/NACK | ✅ |
| Monotonic cursor per workspace within a single stagecraft process lifetime | ✅ |
| Cursor-gap detection at reconnect via `SyncHandshake.lastServerCursor` → `sync.resync_required` | ✅ |
| Replay of unacked server events on reconnect when cursor is known | ✅ (via `deliverResync` + outbox), **bounded by ring buffer** |
| At-least-once delivery across stagecraft restarts | ❌ (outbox wiped on restart) |
| Exactly-once delivery | ❌ — never claimed. |
| Backpressure / flow control | ❌ — `stream.send` is awaited but there is no explicit window/credit. |
| Ordering across workspaces | ❌ — per-workspace ordering only. |
| Cross-replica fan-out when stagecraft scales horizontally | ❌ — each replica's registry is local; a producer on replica A will not reach a client connected to replica B. Fixing this requires fronting `dispatchServerEvent` with PubSub/Redis. |

**Verdict: Honest. Claims match implementation.**

---

## F. Workspace isolation

| Concern | Status |
|---|---|
| Registry keyed by workspaceId | **Yes.** `broadcastWorkspace` cannot leak across workspaces — verified by `registry.test.ts` → *"broadcastWorkspace does not leak across workspaces"*. |
| Workspace taken from authenticated claims, not client input | **Yes.** See section C. |
| Outbox cursor scoped per-workspace | **Yes.** Verified by `store.test.ts` → *"cursors are independent per workspace"* and *"pending events do not cross workspaces"*. |
| `ClientAuditCandidate` forcibly stamped with server-side `workspaceId` | **Yes.** |

**Residual risks:**
- A single authenticated user with claims for workspace A *cannot* subscribe to workspace B through this endpoint.
- If the Rauthy JWT itself issues the wrong `oap_workspace_id`, the isolation breaks — but that is an auth-layer bug, not a sync-layer bug.

**Verdict: Fully aligned.**

---

## G. Runtime readiness

**Is this sufficient for production?** **No.** Specifically missing:

1. **Durable outbox/inbox** (see section D).
2. **Cross-replica fan-out** — stagecraft runs >1 replica in staging/prod. Events dispatched on replica A will not reach clients on replica B. Fix: wrap `dispatchServerEvent` in a PubSub publish, subscribe on each replica, then call into the local registry.
3. **Membership gate** — stream should verify the authenticated user is an active member of `workspaceId` (`org_memberships` / `project_members`), not just that the JWT declares the workspace.
4. **Backpressure / slow-client handling** — an unresponsive client currently backs up the broadcast loop (`await stream.send` is per-client and sequential).
5. **Observability** — no metrics (connections, messages/sec, ACK latency). Structured logs exist but are not enough for alerting.
6. **Rate limiting** — nothing caps how fast a client can flood inbound events.
7. **Envelope schema versioning** — the envelope types have no `schemaVersion` field; introducing breaking changes later will be harder than necessary.

**What IS production-safe:**
- Type guards reject unknown inbound kinds (no arbitrary-code-execution surface from client-supplied `kind`).
- `audit.candidate` is normalised, not trusted.
- Registry auto-prunes dead streams on send failure.
- All logging avoids emitting the raw payload.

**Verdict: Misaligned for production use as-is. Aligned as a foundation for subsequent phases.**

---

## Honest summary

> **Structurally aligned, operationally incomplete.**

The shape is right:
- authority boundaries are encoded in the type system,
- auth is real,
- workspace isolation is real,
- the interface seams for a durable store exist and are exercised by tests,
- the reflection document matches the code.

What is **not** production-grade and must land before any customer traffic:
- durable Postgres-backed outbox/inbox,
- cross-replica fan-out,
- workspace membership check (not just JWT claim check),
- backpressure + rate limiting,
- metrics.

None of the above are stubbed in a way that *pretends* to work. The in-memory
stores are documented as in-memory; the `MAX_INBOX_HISTORY` / `MAX_OUTBOX_PER_WORKSPACE`
constants are visible in `store.ts`; this document names every gap.

## Concrete follow-ups (ordered)

1. Durable Postgres outbox/inbox tables + Drizzle-backed implementations of the `store.ts` interfaces.
2. Workspace-membership check inside `duplex.ts` before calling `registry.register`.
3. PubSub fan-out adapter so `dispatchServerEvent` reaches every replica's registry.
4. Schema-versioning field on `EnvelopeMeta` (`v: 1`), with a migration-friendly guard in `isClientEnvelope`.
5. Prometheus metrics: `sync_connections_total`, `sync_events_inbound_total{kind, status}`, `sync_events_outbound_total{kind}`, `sync_ack_latency_seconds`.
6. Decommission the legacy `workspaceEventStream` streamOut + `ingestOpcEvent` HTTP endpoint once `web/`, `apps/desktop`, and `packages/workspace-sdk` are migrated to the duplex.
