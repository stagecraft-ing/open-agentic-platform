# Platform Integration Seams — Deep Source Trace

> **Reviewer:** claude | **Date:** 2026-03-31 | **Scope:** Code-level verification of all 4 seams + cross-cutting concerns
> **Predecessor:** `.ai/findings/platform-integration-seams-readiness.md` (confirmed, extended)

## Purpose

This trace verifies the prior readiness review against the actual codebase, pinpoints exact injection points with line numbers, and refines the implementation order with concrete dependency chains.

---

## Seam B — Audit Streaming

### OPC Side (axiomregent)

| File | Line | Symbol | Finding |
|------|------|--------|---------|
| `crates/axiomregent/src/router/permissions.rs` | 88–104 | `audit_tool_dispatch` | Builds JSON with `"op": "axiomregent.tool_audit"` + `chrono::Utc::now()` timestamp, writes to **stderr only**. No network I/O. |
| `crates/axiomregent/src/router/mod.rs` | 141 | call site 1 | `decision: "allowed"` — lease present, permission passed |
| `crates/axiomregent/src/router/mod.rs` | 150 | call site 2 | `decision: "denied"` — lease present, permission failed |
| `crates/axiomregent/src/router/mod.rs` | 164 | call site 3 | `decision: "allowed_no_lease"` — fallback grants passed |
| `crates/axiomregent/src/router/mod.rs` | 173 | call site 4 | `decision: "denied_no_lease"` — fallback grants denied |
| `crates/axiomregent/src/router/mod.rs` | 199 | call site 5 | `decision: "policy_denied"` — OPA policy bundle denied |
| `crates/axiomregent/Cargo.toml` | — | — | **No HTTP client dependency** (no reqwest, ureq, or hyper). Confirmed blocker X-001. |

**Implementation path:** Add `reqwest` with `features = ["json"]` to Cargo.toml. Create a new `audit_http.rs` module with a fire-and-forget `tokio::spawn` POST. Read `PLATFORM_AUDIT_URL` env var at `Router::new()`. Thread the optional sender through to all 5 call sites. When URL is absent, stderr-only (current behavior preserved).

### Stagecraft Side

| File | Line | Symbol | Finding |
|------|------|--------|---------|
| `platform/services/stagecraft/api/db/schema.ts` | 39–49 | `auditLog` | `actor_user_id: uuid NOT NULL FK → users(id)`. **Blocker B-005 confirmed.** |
| `platform/services/stagecraft/api/db/migrations/1_create_auth_tables.up.sql` | — | — | No seed data. No system user. |
| `platform/services/stagecraft/api/admin/admin.ts` | 56 | `setRole` | Only write path into `audit_log` — internal Encore call, not HTTP-exposed for external ingest. |
| `platform/services/stagecraft/api/admin/admin.ts` | 68–78 | `GET /admin/audit` | Read-only, last 200 rows. Adequate for verification. |

**Required before implementation:**
1. New migration: seed a `system` user row (recommended: fixed UUID `00000000-0000-0000-0000-000000000000`, name "system", email "system@opc.local", role "admin")
2. New endpoint: `POST /api/audit-records` accepting `{ action, targetType, targetId, metadata }` — actor_user_id hardcoded to system user UUID for machine ingest
3. Auth: bearer token check (M2M pattern from deployd-api)

---

## Seam A — Policy Bundle Serving

### OPC Side (axiomregent)

| File | Line | Symbol | Finding |
|------|------|--------|---------|
| `crates/axiomregent/src/router/policy_bundle.rs` | 11 | `PolicyBundleCache` | `RwLock<HashMap<String, Option<Arc<PolicyBundle>>>>` keyed by `repo_root`. |
| `crates/axiomregent/src/router/policy_bundle.rs` | 23 | `bundle_for_repo_root` | On cache miss: reads `<repo_root>/build/policy-bundles/policy-bundle.json`. No invalidation — stale until restart. |
| `crates/axiomregent/src/router/mod.rs` | 209 | `handle_request` | **Synchronous** (`pub fn handle_request(&self, req: &JsonRpcRequest) -> JsonRpcResponse`). No async. |
| `crates/axiomregent/src/router/mod.rs` | 187 | `policy_preflight_response` | Calls `bundle_for_repo_root` inline. Blocking I/O path. |
| `crates/policy-kernel/src/lib.rs` | 44 | `PolicyBundle` | `{ constitution: Vec<PolicyRule>, shards: BTreeMap<String, Vec<PolicyRule>> }` — the deserialization target. |

**Design decision needed:** Since `handle_request` is sync and MCP runs on stdio, we can't `await` inside it. Two options:
- **(a) Background refresh thread** — spawn a thread at `Router::new()` that fetches from `PLATFORM_POLICY_URL` every N seconds and updates the cache via `RwLock`. `bundle_for_repo_root` reads from cache (already does). **Recommended.**
- **(b) `block_on()` inline** — blocks the MCP dispatch. Acceptable for a first pass since MCP is single-client, but fragile.

### Stagecraft Side

No policy modules, endpoints, or `PolicyBundle` references exist anywhere in `platform/services/stagecraft/`. This is fully green field. The policy bundle JSON format is already defined by `policy-kernel`.

---

## Seam C — Permission Grants

### OPC Side

| File | Line | Symbol | Finding |
|------|------|--------|---------|
| `apps/desktop/src-tauri/src/governed_claude.rs` | 45–53 | `grants_json_for_agent` | Reads `agent.enable_file_read/write/network`, hardcodes `max_tier: 3`. |
| `apps/desktop/src-tauri/src/governed_claude.rs` | 55–63 | `grants_json_claude_default` | All booleans `true`, `max_tier: 2`. |
| `apps/desktop/src-tauri/src/governed_claude.rs` | 82–93 | `plan_governed` | Returns `Bypass` when axiomregent port absent or binary missing. |
| `apps/desktop/src-tauri/src/governed_claude.rs` | 104 | `append_claude_governance_args` | `Bypass` → pushes `--dangerously-skip-permissions`. |
| `apps/desktop/src-tauri/src/web_server.rs` | 488, 605, 699 | — | All three Claude launch paths use `grants_json_claude_default()`. No per-agent customization. |

**Auth gap:** The desktop Rust backend has **zero** HTTP auth infrastructure. No JWT, no bearer tokens, no session management. Every `token` reference is LLM token counts; every `session` reference is a Claude JSONL session UUID. The web server has `CORS: Any` (line 794) — local-only by design.

### Stagecraft Side

- No `workspace_grants` table in any migration or schema file.
- Auth is cookie-session based (`__session`). Not compatible with desktop → server calls.

**M2M pattern available:** `platform/services/stagecraft/api/deploy/logtoM2m.ts` provides `getCachedDeploydAuthHeader()` using Logto `client_credentials` grant. `platform/services/deployd-api/src/auth/logtoJwt.ts` provides `verifyLogtoJwt()` for the receiving side. This exact pattern should be extracted into a shared auth utility.

---

## Seam D — Agent Identity Validation

### OPC Side

| File | Line | Symbol | Finding |
|------|------|--------|---------|
| `apps/desktop/src-tauri/src/commands/agents.rs` | 38–52 | `Agent` struct | No `slug` field. Fields: id, name, icon, system_prompt, default_task, model, enable_file_read/write/network, hooks, created_at, updated_at. |
| `apps/desktop/src-tauri/src/commands/agents.rs` | 9 | `use reqwest` | Import present (used elsewhere in file, not in `execute_agent`). |
| `apps/desktop/src-tauri/src/commands/agents.rs` | 899–1014 | `execute_agent` | Governance wiring at 982–992: reads sidecar port, builds grants, calls `plan_governed`. |

**Implementation path:** Derive slug from `agent.name` via kebab-case transform. Add optional pre-flight `GET /api/agents/{slug}/authorized`. Failure must NOT block execution (spec says optional).

### Stagecraft Side

No agent-authorization endpoint exists. Fully green field.

---

## Cross-Cutting Concerns (Updated)

| ID | Severity | Concern | Verified | Notes |
|----|----------|---------|----------|-------|
| X-001 | **HIGH** | No HTTP client in axiomregent | ✅ Confirmed — Cargo.toml has no reqwest/ureq/hyper | Blocks Seams A, B |
| X-002 | **MEDIUM** | No workspace_id concept | ✅ Confirmed — "workspace" only means filesystem in axiomregent | Blocks Seams A, C |
| X-003 | **MEDIUM** | No M2M auth for OPC → stagecraft | ✅ Confirmed — zero auth code in desktop Rust | Blocks Seams B, C, D. Pattern exists in deployd-api (Logto JWT) |
| X-004 | LOW | No `PLATFORM_*` env vars | ✅ Confirmed — none defined | All seams |
| X-005 | INFO | Env-var gating for offline mode | ✅ Design confirmed — absent URL = local-only | All seams |
| **X-006** | **NEW — MEDIUM** | `PolicyBundleCache` has no invalidation | Discovered in trace — stale bundles persist until restart | Seam A |
| **X-007** | **NEW — LOW** | `max_tier` hardcoded (3 for agents, 2 for default) | No runtime override path | Seam C (if tiers become platform-managed) |

---

## Refined Implementation Order

### Foundation (prerequisite for all seams)

1. Add `reqwest = { version = "0.12", features = ["json"] }` to `crates/axiomregent/Cargo.toml`
2. Define `PLATFORM_AUDIT_URL`, `PLATFORM_POLICY_URL`, `PLATFORM_API_URL` env vars in a shared config module
3. Extract M2M auth pattern from `logtoM2m.ts` / `logtoJwt.ts` into reusable utilities
4. Create system user migration for stagecraft

### Seam B → A → C → D (unchanged order, confirmed)

- **B first:** simplest HTTP path (fire-and-forget POST), proves the transport + auth pattern
- **A second:** requires background refresh thread design, builds on reqwest + auth
- **C third:** most complex — needs workspace_grants table, new Drizzle schema, desktop auth flow
- **D last:** optional pre-flight, lowest value, depends on C's auth pattern

---

## Delta from Prior Review

The prior readiness review (`.ai/findings/platform-integration-seams-readiness.md`) was **accurate on all 28 findings (B-001 through X-005)**. This deep trace adds:

- Exact line numbers for all injection points
- Confirmation that the desktop Rust backend has zero HTTP auth infrastructure (not just "no M2M pattern" — no auth of any kind)
- New finding X-006: `PolicyBundleCache` lacks invalidation, which matters for Seam A's background refresh design
- New finding X-007: `max_tier` hardcoding, which is a minor concern for Seam C
- Concrete implementation paths for each seam's OPC-side changes
- Confirmation that stagecraft's `setRole` in `admin.ts:56` is the only `audit_log` write path — the POST endpoint is truly new work
