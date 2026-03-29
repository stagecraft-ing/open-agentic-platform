# Next slice (working synthesis)

> **Non-authoritative.** This is a **staging** view for the smallest next increment — not a replacement for `specs/.../tasks.md` or `plan.md`. Promote agreed work into canonical tasks.

## Context

- Branch: `main`
- **Features 032–035: COMPLETE** — all delivered 2026-03-28/29, verification green
- Synthesis by: **claude-opus** (2026-03-29)

## Platform state after Feature 035

The governed execution thesis is now **live end-to-end**:

| Milestone | Feature | Status |
|-----------|---------|--------|
| Inspect + governance wiring | 032 | Active, complete |
| axiomregent sidecar alive | 033 | Active, complete |
| featuregraph reads registry | 034 | Active, complete |
| Agent execution governed | 035 | Active, complete |

The platform has moved from "Claude wrapper with governance aspirations" to "governed execution environment with known gaps." What remains is **hardening, cross-platform, and capability expansion**.

## Residuals inventory

### From Feature 035 review (Risk 1 — MEDIUM)

**No-lease tool calls bypass permission checks.** `preflight_tool_permission` in `router/mod.rs:118-119` returns `None` (no denial) when `lease_id` is absent or unknown. Practical risk is low (Claude CLI sends structured calls; read-only tools are Tier 1), but the gap should be closed.

**Fix:** Default to session-level grants from `PermissionGrants::from_env_or_default()` when no lease is found. Audit-log the fallback.

### From Feature 035 review (Risk 2 — LOW)

**Agent max_tier=3 vs Claude max_tier=2.** Defensible (agents have per-permission flags as primary gate), but should be documented as a contract note in `spec.md`.

### From Feature 033 (cross-platform)

**Only macOS arm64 axiomregent binary bundled.** Windows/Linux degrade gracefully but cannot run governed dispatch. Needs cross-compilation pipeline.

### From Feature 034 review (minor)

**Error message wording** at `scanner.rs:275-276` should lead with "Re-run `spec-compiler compile`" now that registry is primary source.

### Ongoing debt

- **Feature ID duality** — kebab spec IDs vs UPPERCASE code IDs remain unbridged.
- **Titor command stubs** — 5 of 6 Tauri commands are `todo!()`. Temporal safety net blocked.
- **Safety tier model** — code-only in `safety.rs`, not spec-governed.
- **NF-001 latency gate** — no automated measurement artifact.

## Ordered next-slice priority

### Slice A: Post-035 hardening (Feature 036)

**Why first:** Closes the most critical gap in the just-delivered feature before adding new surface area. Small scope, high confidence.

Scope:
1. **Fix no-lease bypass** — default to session grants when lease_id absent; audit-log the fallback path. Touches `router/mod.rs:112-141`.
2. **Document max_tier rationale** — add contract note to `specs/035-agent-governed-execution/spec.md`.
3. **Add NF-001 benchmark** — integration test asserting < 50ms overhead per tool call.
4. **Minor wording fix** — `scanner.rs:275-276` message update.

Estimated effort: 1 Cursor session. No spec scaffolding needed — these are hardening tasks on existing features.

### Slice B: Safety tier governance (Feature 037)

**Why second:** Safety tiers are now enforcement-critical (Feature 035 consults `get_tool_tier()` on every tool call). Tier definitions must be spec-governed before adding more tools or changing tier assignments.

Scope:
1. Formalize tier definitions (Tier1=autonomous, Tier2=gated, Tier3=manual) in a spec.
2. Define tier assignment rules (which tool category → which tier).
3. Add verification that `safety.rs` matches spec.
4. Surface tier assignments in governance UI (beyond current hardcoded labels).

### Slice C: Cross-platform axiomregent binaries (Feature 033 residual)

**Why third:** Broadens governed execution from macOS-only to all development platforms. Prerequisite for any production deployment.

Scope:
1. Add cross-compilation targets (x86_64-apple-darwin, x86_64-pc-windows-msvc, x86_64-unknown-linux-gnu).
2. CI pipeline for binary builds (follow `gitctx-mcp` fetch-and-build pattern).
3. Update `tauri.conf.json` externalBin list.
4. Verification on at least one non-macOS target.

### Slice D: Titor Tauri command wiring (Feature 038)

**Why fourth:** Titor is ~17k LOC of production-grade checkpoint/restore. Desktop access enables the temporal safety net — critical for governed agent execution to be recoverable.

Scope:
1. Wire the 5 stubbed Tauri commands to titor library.
2. Expose checkpoint/restore in agent execution UI.
3. Add verification for round-trip checkpoint→execute→restore.

### Slice E: Feature ID reconciliation (ADR)

**Why fifth:** Grows in urgency with each new feature, but has no enforcement impact today. Needs an architecture decision record (ADR) before implementation.

Scope:
1. ADR: choose canonical ID format, define mapping strategy.
2. Either generate UPPERCASE→kebab mapping from registry, or adopt one system.
3. Update `Scanner` and code attribution headers to reconcile.

## Fork resolution

**Chosen path: hardening-first, then widen.**

Features 032–035 established the governed execution thesis. The next priority is to harden what exists (Slice A), then formalize the tier model that enforcement depends on (Slice B), then broaden platform support (Slice C). Capability expansion (D, E) follows after the foundation is solid.

## Recommended promotion set

### Promote now

- **Slice A tasks** — no new spec needed; add tasks to `specs/035-agent-governed-execution/tasks.md` as T014–T017 (hardening), or scaffold a lightweight `specs/036-post-035-hardening/`.

### Promote next

- **Slice B spec** — `specs/037-safety-tier-governance/spec.md`
- **Slice C plan** — extension of `specs/033-axiomregent-activation/` or standalone `specs/038-cross-platform-axiomregent/`
