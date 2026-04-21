# Sync Protocol — Self-Reflection & Alignment Report

**Scope:** First-pass outbox/inbox sync substrate in `platform/services/stagecraft/api/sync/`.
**Spec binding:** [087 §5.3 Duplex Sync Substrate](../../../../../specs/087-unified-workspace-architecture/spec.md#53-duplex-sync-substrate) — FR-SYNC-001..010.
**Date:** 2026-04-20 (initial) · 2026-04-20 (post-review amendment)
**Verdict:** **Structurally aligned, operationally incomplete — with one authorization defect remaining as a blocker.**

This document measures the implementation against the architectural intent of
spec 087 §5.3. It is written to be honest about what is *real* in the codebase
versus what is *stubbed, best-effort, or deferred*.

## Post-review amendment (2026-04-20)

After the first pass of this document, the following changes landed in the same branch:

- **Schema versioning shipped.** `EnvelopeMeta.v: 1` is now required on every envelope; the runtime guard rejects mismatched or missing `v`. See FR-SYNC-003. Previously listed as follow-up #4 in the original version of this document — now in section D/G tables as ✅.
- **Reflection bound to a spec.** Spec 087 extended with §5.3 "Duplex Sync Substrate" codifying FR-SYNC-001..010, the authority invariant, the membership-gate design, and the retention calculus.
- **Membership gate (FR-SYNC-002) re-classified.** Originally listed alongside scale/ops items; elevated here and in §5.3 to **correctness blocker**. It is the only remaining item that gates "non-test deployment", not "production scale".

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
- WebSocket `auth: true` relies on Encore routing the upgrade request through the gateway authHandler. If that is bypassed in a particular deployment (e.g., direct pod access), the stream would be open. Mitigated by Helm/ingress policy, not by this file. This is a deployment-posture item, not a code item; tracked as an integration test + NetworkPolicy assertion rather than a code change here.
- **CORRECTNESS BLOCKER — FR-SYNC-002.** The authenticated user's membership in the workspace is NOT verified: any JWT with `oap_workspace_id = X` opens a stream in workspace X. If the claim drifts from the live membership tables (stale JWT after off-boarding, mis-issued token, claim tampering that passes signature check), the stream would grant workspace visibility the user no longer has. Unlike the other production gaps in §G — which break under *load* — this one is wrong on day one. Fix lands before any non-test use.

**Verdict: Fully aligned for identity; **one authorization defect remains** (FR-SYNC-002, blocker).**

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

**Retention calculus of the in-memory outbox (`MAX_OUTBOX_PER_WORKSPACE = 500`):**
- At target event rate **~10 events/s** (factory progress + audit + periodic deploy/grant changes) → **~50 seconds** of history retained.
- At burst rate **~50 events/s** (factory stage fan-out) → **~10 seconds** of history retained.
- A reconnect gap exceeding the retained window forces `sync.resync_required(cursor_gap)` and a REST-backed refetch. That is acceptable for single-digit-second deploys and for flows where state is refetchable; it is NOT acceptable for longer deploys or for state that only travels through this channel.
- `MAX_INBOX_HISTORY = 1000` is debug/inspection only; audit candidates are durably written to `audit_log` regardless.

**What's required to move forward (FR-SYNC-005 — spec 087 §5.3):**
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

**Is this sufficient for production?** **No.** Gaps ordered by severity, with spec 087 §5.3 FR-IDs:

| # | Gap | Class | FR | Status |
|---|-----|-------|----|--------|
| 1 | **Membership gate** — the stream should verify the authenticated user is an active member of `workspaceId` (join on `org_memberships` / `project_members`), not just that the JWT declares it. | **correctness — blocker** | FR-SYNC-002 | not shipped |
| 2 | Durable Postgres outbox/inbox (see §D). | durability | FR-SYNC-005 | not shipped |
| 3 | Cross-replica fan-out — events on replica A must reach clients on replica B via PubSub. | correctness (multi-replica) | FR-SYNC-006 | not shipped |
| 4 | Backpressure / slow-client handling — `await stream.send` is sequential per client; one slow client stalls the broadcast loop. | liveness | FR-SYNC-007 | not shipped |
| 5 | Observability — `sync_connections_total`, `sync_events_inbound_total`, `sync_events_outbound_total`, `sync_ack_latency_seconds`. | ops | FR-SYNC-008 | not shipped |
| 6 | Rate limiting per `clientId`. | abuse resistance | FR-SYNC-009 | not shipped |
| 7 | ~~Envelope schema versioning~~. | correctness | FR-SYNC-003 | **shipped** — `EnvelopeMeta.v: 1` required; runtime guard rejects `v !== 1`. |

**What IS production-safe:**
- Type guards reject unknown inbound kinds (no arbitrary-code-execution surface from client-supplied `kind`).
- Envelope schema version is required and strictly equal to 1; future v2 clients cannot silently fall through a best-effort decoder.
- `audit.candidate` is normalised, not trusted.
- Registry auto-prunes dead streams on send failure.
- All logging avoids emitting the raw payload.

**Verdict: Misaligned for production use as-is. Aligned as a foundation for subsequent phases; one item (FR-SYNC-002) must land before any non-test deployment.**

---

## H. Invariants (codified in spec 087 §5.3)

These are the rules future contributors MUST honour when touching this directory. Violating any of them is a spec violation, not a code-review preference.

1. **Authority is encoded in the envelope union.** `ClientEnvelope` variants MUST NOT carry control-plane authority (policy/grant/deploy/workspace/project state mutation). `ServerEnvelope` carries control-plane truth. Extending `ClientEnvelope` with a variant that asserts authoritative server state is a governance act requiring a spec amendment.
2. **`workspaceId` is taken from the authenticated JWT, never from the handshake.** Any code path that reads a workspace id from client-supplied data in `/api/sync/duplex` is a bug.
3. **`audit.candidate` is normalised, not trusted.** `actor_user_id` MUST be stamped from `auth.userID`; the `action` is prefixed `opc.`; `clientId` and `workspaceId` are server-stamped in `metadata`. Timestamps come from the server, not the desktop.
4. **Cursors are minted only in `service.ts#mintMeta`.** No other module mints a `workspaceCursor`. This is the single place that orders the outbound stream.
5. **`EnvelopeMeta.v` MUST equal the current `ENVELOPE_SCHEMA_VERSION` (currently `1`).** Bumping the version is a lock-step change to the TypeScript literal *and* the runtime guard in `isClientEnvelope`.
6. **The store interfaces are the swap boundary.** `OutboxStore`, `InboxStore`, and `CursorIssuer` exist so Drizzle-backed implementations can replace the in-memory ones without touching `duplex.ts` or `service.ts`. Do not leak storage concerns past those interfaces.

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

## Concrete follow-ups (ordered by severity)

1. **[FR-SYNC-002 — blocker]** Workspace-membership check inside `duplex.ts` before calling `registry.register`. Must land before any non-test deployment.
2. **[FR-SYNC-005]** Durable Postgres outbox/inbox tables + Drizzle-backed implementations of the `store.ts` interfaces.
3. **[FR-SYNC-006]** PubSub fan-out adapter so `dispatchServerEvent` reaches every replica's registry.
4. **[FR-SYNC-007]** Backpressure + slow-client handling in the broadcast loop.
5. **[FR-SYNC-008]** Prometheus metrics: `sync_connections_total`, `sync_events_inbound_total{kind, status}`, `sync_events_outbound_total{kind}`, `sync_ack_latency_seconds`.
6. **[FR-SYNC-009]** Rate limiting per `clientId`.
7. **[FR-SYNC-010]** Decommission the legacy `workspaceEventStream` streamOut + `ingestOpcEvent` HTTP endpoint. Blocked on migration of three consumers:
   - `platform/services/stagecraft/web/` — still imports the legacy stream.
   - `apps/desktop/` — `packages/workspace-sdk/` client used by the desktop.
   - `packages/workspace-sdk/` — publishes the consumer API to be swapped.
   Each migration is a separate PR; FR-SYNC-010 closes only when all three are done.

~~**[FR-SYNC-003 — schema versioning]**~~ Shipped in this branch as part of the post-review amendment (see top of document).
