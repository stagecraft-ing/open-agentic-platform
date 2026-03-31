# 057 Notification Orchestrator — Pre-Implementation Readiness Review

**Reviewer**: claude
**Date**: 2026-03-31
**Verdict**: **READY** — no blockers, 9 findings (0 HIGH, 3 LOW, 6 INFO)

## Dependencies

| Dep | Status | Compatible? |
|-----|--------|-------------|
| 042 Multi-Provider Agent Registry | feature-complete | Yes — `ProviderId` is `string`, maps directly to `NotificationEvent.provider` |
| 035 Agent Governed Execution | feature-complete | Partial — spec 035 emits tool dispatch logs, not high-level lifecycle events; notification system must synthesize `task_complete`/`task_error`/`permission_request` from lower-level signals |

## Architecture Decisions

**Package scaffold**: New `packages/notification-orchestrator` (`@opc/notification-orchestrator`), following the established ESM pattern (TypeScript, vitest, `tsc` build, subpath exports per module).

**6 phases scoped** (per spec):
- P1: Types + orchestrator core dispatch loop
- P2: Sliding-window deduplication index
- P3: Preference engine + persistence
- P4: Channel adapters (native, web-push, toast)
- P5: Event log + retention pruning
- P6: Integration (wire `notify()` into agent session lifecycle)

**Existing toast**: `packages/ui/src/toast.tsx` has a simple `Toast` component with `success | error | info` types. The notification-orchestrator toast adapter (Phase 4) should emit events that the existing UI consumes — NOT replace the UI component.

**No SQLite needed for event log**: The spec says "persistent event log" (FR-007) but doesn't mandate SQLite. Given session-memory already uses better-sqlite3, the same approach is viable. Alternatively, a simple NDJSON append log would be lighter. Decision deferred to Phase 5 implementation.

## Findings

### R-001 — Lifecycle event synthesis gap (LOW)

Spec 035 emits structured tool dispatch logs (`{tool_name, tier, permission_decision, timestamp}`), not high-level lifecycle events. Spec 057 needs `task_complete`, `task_error`, `permission_request` kinds. Phase 6 integration will need to define the mapping from lower-level execution signals to notification kinds. This is a design gap in the spec — the implementer should define synthesis rules in Phase 6.

### R-002 — Deduplication window reset semantics (INFO)

FR-003 says "the window resets on each new duplicate." This means the dedup window is a **sliding window** — each duplicate extends the suppression period. This is correct for bursty progress updates but could indefinitely suppress events that arrive in a steady stream (e.g., one event every 19 seconds with the same key would never deliver after the first). Spec-as-written is clear; this is a documented behavior, not a bug.

### R-003 — Event log storage backend unspecified (LOW)

FR-007 requires persistent event log queryable by session, kind, severity, and time range. NF-003 requires 30-day retention. The spec doesn't specify the storage backend. Options: SQLite (consistent with session-memory), NDJSON file (simpler), or in-memory with periodic flush. Recommendation: SQLite for query flexibility and 30-day retention management.

### R-004 — Channel adapter availability vs. package scope (INFO)

Spec defines three channel adapters (native, web-push, toast) but the package is a pure TypeScript library. Native OS notifications require Tauri/Electron APIs. Web push requires a service worker. These adapters will need platform-specific imports or be stub/no-op in the library, with real implementations provided by the desktop app. The `isAvailable()` method on `ChannelAdapter` handles this gracefully — adapters can return `false` when running outside their target environment.

### R-005 — Preference persistence location unspecified (LOW)

FR-005 defines the preference data model but doesn't specify where preferences are stored. Options: file-based YAML (consistent with verification-profiles), SQLite (consistent with session-memory), or in-memory with file backup. The `preferences/store.ts` in the spec's package structure suggests a dedicated persistence module. Recommendation: YAML file at a well-known location (e.g., `.notification-preferences.yaml` in project root or XDG config dir).

### R-006 — Toast adapter is UI-agnostic (INFO)

The `toast` channel adapter in the library cannot directly render React components. It should emit events (e.g., via EventEmitter or callback) that the `packages/ui` Toast component subscribes to. This is consistent with the spec's out-of-scope note: "Notification UI components are separate front-end concerns."

### R-007 — No rate limiting in initial spec (INFO)

R-003 in the spec acknowledges high event volume risk and says "rate limiting can be added per channel as a follow-on." The deduplication + preference system provides some protection, but a flood of distinct-keyed events would still overwhelm channels. Acceptable for initial implementation.

### R-008 — UUID generation dependency (INFO)

FR-002 requires `id: string` as UUID. The `crypto.randomUUID()` API is available in Node 19+ and all modern browsers. No external dependency needed for ESM packages targeting ES2022.

### R-009 — Metadata field typing (INFO)

`metadata: Record<string, unknown>` is maximally flexible but provides no type safety for consumers. Consumers will need to cast or validate metadata fields. This is acceptable for a generic notification system — specific metadata shapes can be defined per-kind in a follow-on if needed.

## Recommendation

Proceed with implementation. Phase 1 should define types, the orchestrator dispatch loop, and the `notify()` public API. The deduplication index (Phase 2) is the most algorithmically interesting piece — use a `Map<string, number>` with timestamps and periodic cleanup via `setInterval` (similar to session-memory's `ExpirySweeper` pattern).
