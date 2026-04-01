# Platform Seams — Implementation Brief

> **Author:** claude | **Date:** 2026-03-31 | **For:** cursor (implementer)
> **Sources:** `.ai/findings/platform-integration-seams-deep-trace.md`, `.ai/findings/platform-integration-seams-readiness.md`

## Overview

Wire OPC ↔ Platform integration across 4 seams (B → A → C → D). All injection points verified against current codebase — line numbers confirmed accurate as of 2026-03-31.

---

## Phase 0: Foundation (prerequisite for all seams)

### 0a. Add reqwest to axiomregent

**File:** `crates/axiomregent/Cargo.toml`
```toml
reqwest = { version = "0.12", features = ["json"] }
```
Currently absent (blocker X-001). Tokio is already a dependency.

### 0b. Platform config module

**New file:** `crates/axiomregent/src/platform_config.rs`

Read from environment:
- `PLATFORM_AUDIT_URL` — Seam B (e.g., `http://localhost:4000/api/audit-records`)
- `PLATFORM_POLICY_URL` — Seam A (e.g., `http://localhost:4000/api/policy-bundle`)
- `PLATFORM_API_URL` — Seams C, D (e.g., `http://localhost:4000/api`)
- `PLATFORM_M2M_TOKEN` — Bearer token for all platform calls

All optional. When absent → local-only mode (current behavior preserved, finding X-005).

### 0c. System user migration (stagecraft)

**New file:** `platform/services/stagecraft/api/db/migrations/2_seed_system_user.up.sql`
```sql
INSERT INTO users (id, name, email, role)
VALUES ('00000000-0000-0000-0000-000000000000', 'system', 'system@opc.local', 'admin')
ON CONFLICT (id) DO NOTHING;
```
Required because `audit_log.actor_user_id` is NOT NULL FK → users(id) (finding B-005).

### 0d. M2M auth pattern

Extract from existing code:
- **Client side:** `platform/services/stagecraft/api/deploy/logtoM2m.ts` — `getCachedDeploydAuthHeader()`
- **Server side:** `platform/services/deployd-api/src/auth/logtoJwt.ts` — `verifyLogtoJwt()`

For first pass: simple bearer token check (`PLATFORM_M2M_TOKEN` env var on both sides). Logto M2M JWT can replace this later.

---

## Phase 1: Seam B — Audit Streaming

**Lowest risk, highest value. Proves the transport + auth pattern.**

### OPC side

1. **New file:** `crates/axiomregent/src/router/audit_http.rs`
   - `pub struct AuditForwarder { client: reqwest::Client, url: String, token: String }`
   - `pub fn forward(&self, payload: serde_json::Value)` — `tokio::spawn` fire-and-forget POST
   - On HTTP error: log to stderr, do NOT block or retry

2. **Modify:** `crates/axiomregent/src/router/mod.rs`
   - Add `audit_forwarder: Option<AuditForwarder>` field to `Router`
   - Initialize from `PLATFORM_AUDIT_URL` in `Router::new()`
   - At all 5 call sites (lines 141, 150, 164, 173, 199): after `audit_tool_dispatch()`, call `self.audit_forwarder.as_ref().map(|f| f.forward(payload))`

3. **Modify:** `crates/axiomregent/src/router/permissions.rs`
   - Refactor `audit_tool_dispatch` (line 88) to return the JSON value in addition to writing stderr, so it can be forwarded

### Stagecraft side

4. **New endpoint:** `POST /api/audit-records` in `platform/services/stagecraft/api/admin/admin.ts`
   - Accept `{ action: string, targetType: string, targetId: string, metadata?: object }`
   - Hardcode `actor_user_id` to system user UUID (`00000000-...`)
   - Bearer token auth check
   - Insert into `audit_log` table

### Verification

- Start stagecraft locally (`npm run start` in `platform/services/stagecraft/`)
- Set `PLATFORM_AUDIT_URL=http://localhost:4000/api/audit-records` + `PLATFORM_M2M_TOKEN=test`
- Run axiomregent, trigger a tool permission check
- Verify row appears via `GET /admin/audit`

---

## Phase 2: Seam A — Policy Bundle Serving

### OPC side

1. **New file:** `crates/axiomregent/src/router/policy_http.rs`
   - Background refresh thread: spawn at `Router::new()` when `PLATFORM_POLICY_URL` is set
   - Fetch `GET {PLATFORM_POLICY_URL}/{repo_root_hash}` every 60s
   - Deserialize to `PolicyBundle` (from `policy-kernel` crate)
   - Update `PolicyBundleCache` via existing `RwLock`
   - On failure: keep stale cache, log warning (addresses X-006 — adds TTL-based invalidation)

2. **Modify:** `crates/axiomregent/src/router/policy_bundle.rs`
   - Add `pub fn update_bundle(&self, repo_root: &str, bundle: PolicyBundle)` method
   - Existing `bundle_for_repo_root` (line 23) continues to work unchanged

### Stagecraft side

3. **New service:** `platform/services/stagecraft/api/policy/policy.ts`
   - `GET /api/policy-bundle/:workspace_id` — serve policy bundle JSON
   - Storage: filesystem volume or Postgres JSONB (simplest: read from a mounted config volume)
   - Bearer token auth

### Verification

- Upload a policy bundle to stagecraft
- Confirm axiomregent picks it up within refresh interval
- Confirm local-file fallback still works when URL is absent

---

## Phase 3: Seam C — Permission Grants (complex)

### OPC side

1. **Modify:** `apps/desktop/src-tauri/src/governed_claude.rs`
   - In `grants_json_for_agent` (line 45): add optional HTTP fetch from `PLATFORM_API_URL/grants/{user_id}/{workspace_id}`
   - Fall back to current local grants on failure
   - `user_id` and `workspace_id` from new env vars `OPC_USER_ID`, `OPC_WORKSPACE_ID`

2. **Modify:** `apps/desktop/src-tauri/src/web_server.rs`
   - Lines 488, 605, 699: replace `grants_json_claude_default()` with platform-aware variant

### Stagecraft side

3. **New migration:** `workspace_grants` table
4. **New endpoint:** `GET /api/grants/:user_id/:workspace_id`

---

## Phase 4: Seam D — Agent Identity (optional)

### OPC side

1. **Modify:** `apps/desktop/src-tauri/src/commands/agents.rs`
   - Derive slug from `agent.name` (kebab-case)
   - Optional pre-flight `GET /api/agents/{slug}/authorized` before `execute_agent` (line 899)
   - On failure: proceed with execution (non-blocking)

### Stagecraft side

2. **New endpoint:** `GET /api/agents/:slug/authorized` — 200/403/404

---

## Risk Summary

| Risk | Mitigation |
|------|-----------|
| Breaking offline mode | All gated on env vars — absent = current behavior |
| Blocking MCP dispatch | Seam B: fire-and-forget. Seam A: background thread. Neither blocks `handle_request` |
| Auth complexity | Start with bearer token, upgrade to Logto M2M later |
| Schema migration | Only change: seed row insert (no ALTER TABLE) |
