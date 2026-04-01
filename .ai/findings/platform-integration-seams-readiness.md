# Platform Integration Seams — Readiness Review

> **Reviewer:** claude | **Date:** 2026-03-31 | **Scope:** All 4 integration seams (A–D) from handoff Phase 2

## Executive Summary

All four seam endpoints described in the handoff are **gaps** — no HTTP bridge, no platform endpoint, and no workspace identity concept exists today. The OPC side has working local-only code at every specified call site. The stagecraft side has a solid DB schema and auth system but zero workspace/grant/policy/agent-authorization surface. Two cross-cutting blockers apply to all seams: (1) no HTTP client in axiomregent, and (2) no workspace identity mapping anywhere.

## Seam-by-Seam Findings

### Seam B — Audit Streaming (lowest risk, highest value)

**OPC side: READY with minor changes**

| Finding | Severity | Detail |
|---------|----------|--------|
| B-001 | INFO | `audit_tool_dispatch` exists at `crates/axiomregent/src/router/permissions.rs:87–103`. Currently writes JSON to stderr only. Hook point is clear. |
| B-002 | LOW | No HTTP client in `crates/axiomregent/Cargo.toml`. Must add `reqwest` (async fits existing tokio runtime) or `ureq` (sync, simpler). |
| B-003 | INFO | `PLATFORM_AUDIT_URL` env var does not exist anywhere. Must be read and threaded through to the audit call sites. |
| B-004 | INFO | Five call sites in `router/mod.rs` (`allowed`, `denied`, `allowed_no_lease`, `denied_no_lease`, `policy_denied`) — all need the fire-and-forget spawn. |

**Stagecraft side: SCHEMA BLOCKER**

| Finding | Severity | Detail |
|---------|----------|--------|
| B-005 | **HIGH** | `audit_log.actor_user_id` is `NOT NULL REFERENCES users(id)` (migration `1_create_auth_tables.up.sql`). Machine-ingest from axiomregent has no human user. Options: (a) pre-seed system user, (b) make column nullable, (c) add `actor_type` discriminator + nullable FK. Decision needed before implementation. |
| B-006 | LOW | No `POST /api/audit-records` endpoint exists. Must create new Encore.ts service or extend `admin.ts`. |
| B-007 | INFO | Existing `GET /admin/audit` reads last 200 rows — adequate for initial verification. |

**Recommendation:** Start with option (a) — pre-seed a `system` user row via migration. Simplest, no schema change, forward-compatible. The `metadata` JSONB column can carry the machine identity details.

### Seam A — Policy Bundle Serving

**OPC side: READY with design decision needed**

| Finding | Severity | Detail |
|---------|----------|--------|
| A-001 | INFO | `PolicyBundleCache` at `crates/axiomregent/src/router/policy_bundle.rs` is fully implemented for local-file loading from `build/policy-bundles/policy-bundle.json`. Single insertion point: `bundle_for_repo_root()`. |
| A-002 | **MEDIUM** | `Router::handle_request` is synchronous (`pub fn handle_request(&self, req: &JsonRpcRequest) -> JsonRpcResponse`). HTTP fetch must be either (a) background-cached at startup/refresh, or (b) `block_on()` inline. Option (a) strongly preferred — blocks the dispatch loop otherwise. |
| A-003 | LOW | No `workspace_id` concept in axiomregent. `repo_root` is the only key. Need env var `OPC_WORKSPACE_ID` or config file mapping. |
| A-004 | LOW | No HTTP client in axiomregent (same as B-002). |
| A-005 | INFO | `policy-kernel` crate defines `PolicyBundle` (public). HTTP response body must deserialize to this exact type. |

**Stagecraft side: GREEN FIELD**

| Finding | Severity | Detail |
|---------|----------|--------|
| A-006 | LOW | No policy service module exists in stagecraft. Must create `api/policy/policy.ts` + `encore.service.ts`. |
| A-007 | LOW | No `policy_bundles` storage in DB. Must decide: Postgres `jsonb`, object storage, or filesystem volume. |
| A-008 | INFO | Bundle JSON is already compiler output (`build/spec-registry/registry.json` pattern). Could serve it from a shared volume in K8s without needing DB storage. |

**Recommendation:** Cache-on-startup with background refresh (e.g., 5-minute TTL). This avoids blocking the synchronous dispatch loop and aligns with the existing cache pattern.

### Seam C — Platform-Sourced Permission Grants

**OPC side: READY, clear injection point**

| Finding | Severity | Detail |
|---------|----------|--------|
| C-001 | INFO | `grants_json_for_agent` at `governed_claude.rs:45` reads from local SQLite `Agent` struct. The injection point is clear: fetch from platform, fall back to local. |
| C-002 | LOW | `plan_governed` returns `GovernedPlan::Bypass` when axiomregent is not running. Platform-sourced grants must handle this bypass path explicitly — otherwise they become a no-op in offline mode. |
| C-003 | INFO | `web_server.rs` (lines 488, 605, 699) uses `grants_json_claude_default` with hardcoded defaults. These paths also need the platform-sourced grant check if consistency is required. |
| C-004 | LOW | No `user_id` or `workspace_id` concept in the desktop app's Rust governance path. Both must be introduced via env vars or IPC. |

**Stagecraft side: GREEN FIELD**

| Finding | Severity | Detail |
|---------|----------|--------|
| C-005 | **MEDIUM** | No `workspace_grants` table. Requires new migration (`2_create_workspace_grants.up.sql`), Drizzle schema update, and endpoint. |
| C-006 | LOW | Auth is cookie-based sessions (`__session`). Desktop app has no session management — a credential-passing mechanism (stored token, env var, or service-to-service JWT) must be defined for Rust → stagecraft HTTP calls. |
| C-007 | INFO | deployd-api already uses Logto M2M (machine-to-machine) JWT auth for service-to-service calls — this pattern could be reused for OPC → stagecraft auth. |

**Recommendation:** Implement Seam C after Seam B (audit) since it shares the auth challenge (C-006/C-007). Resolve M2M auth pattern once, reuse across seams.

### Seam D — Agent Identity Validation (lowest priority)

**OPC side: PARTIALLY READY**

| Finding | Severity | Detail |
|---------|----------|--------|
| D-001 | LOW | `Agent` struct has no `slug` field (only `name` and `id`). Either add a `slug` column to the SQLite agents table, or derive from `name` (kebab-case). |
| D-002 | INFO | `execute_agent` at `agents.rs:898` accepts `agent_id: i64`. Pre-flight check needs slug resolution before the HTTP call. |
| D-003 | INFO | `reqwest` is already imported in `agents.rs` (line 9) and used for GitHub API calls. No new dependency needed for this file. |

**Stagecraft side: GREEN FIELD**

| Finding | Severity | Detail |
|---------|----------|--------|
| D-004 | LOW | No agent-authorization endpoint exists. Must create `/api/agents/:slug/authorized`. |
| D-005 | INFO | Since this is optional pre-flight, the endpoint can return 200 (authorized) / 403 (denied) / 404 (unknown agent). Failure should NOT block local execution. |

## Cross-Cutting Concerns

| ID | Severity | Concern | Affects |
|----|----------|---------|---------|
| X-001 | **HIGH** | No HTTP client in axiomregent Cargo.toml | Seams A, B |
| X-002 | **MEDIUM** | No workspace identity concept (workspace_id) anywhere | Seams A, C |
| X-003 | **MEDIUM** | No service-to-service auth pattern for OPC → stagecraft | Seams B, C, D |
| X-004 | LOW | `PLATFORM_*` env vars not defined or documented | All seams |
| X-005 | INFO | All seams must be env-var gated (offline mode = local-only) | All seams |

## Recommended Implementation Order

1. **Foundation:** Add `reqwest` to axiomregent, define `PLATFORM_*` env vars, establish M2M auth pattern
2. **Seam B (Audit Streaming):** Lowest risk, highest value, minimal schema impact (pre-seed system user)
3. **Seam A (Policy Bundles):** Extends existing cache pattern, requires background refresh design
4. **Seam C (Permission Grants):** Requires new DB migration and grant resolution logic
5. **Seam D (Agent Identity):** Optional pre-flight, lowest priority, depends on slug concept

## Files Referenced

- `crates/axiomregent/src/router/permissions.rs` — audit dispatch, grant checking
- `crates/axiomregent/src/router/policy_bundle.rs` — policy cache
- `crates/axiomregent/src/router/mod.rs` — Router dispatch, preflight
- `crates/axiomregent/Cargo.toml` — dependency list
- `crates/policy-kernel/src/lib.rs` — PolicyBundle types
- `apps/desktop/src-tauri/src/governed_claude.rs` — grant JSON construction
- `apps/desktop/src-tauri/src/commands/agents.rs` — agent execution
- `apps/desktop/src-tauri/src/web_server.rs` — web mode governance
- `platform/services/stagecraft/api/db/schema.ts` — Drizzle ORM schema
- `platform/services/stagecraft/api/db/migrations/1_create_auth_tables.up.sql` — SQL DDL
- `platform/services/stagecraft/api/admin/admin.ts` — existing admin endpoints
- `platform/services/stagecraft/api/auth/auth.ts` — auth endpoints
- `platform/services/stagecraft/api/deploy/deploy.ts` — M2M auth pattern reference
