# Next slice (working synthesis)

> **Non-authoritative.** This is a **staging** view for the smallest next increment — not a replacement for `specs/.../tasks.md` or `plan.md`. Promote agreed work into canonical tasks.

## Context

- Branch: `main`
- **Features 032–038: COMPLETE** — all delivered 2026-03-29, verification green
- **Slice A (post-035 hardening): COMPLETE** — no-lease bypass fixed, NF-001 benchmark, max_tier rationale documented
- Synthesis by: **claude-opus** (2026-03-29)

## Platform state after Feature 038

The governed execution thesis is **live, spec-governed, enforcement-complete on macOS arm64, partially extended to Windows, and temporally recoverable**:

| Milestone | Feature | Status |
|-----------|---------|--------|
| Inspect + governance wiring | 032 | Active, complete |
| axiomregent sidecar alive | 033 | Active, complete |
| featuregraph reads registry | 034 | Active, complete |
| Agent execution governed | 035 | Active, complete |
| Safety tier governance | 036 | Active, complete |
| Cross-platform axiomregent | 037 | Active, complete (T003/T004 deferred to CI) |
| Titor Tauri command wiring | 038 | Active, complete |

**All authority-map items from 032 through 038 are RESOLVED.** The only remaining items are **Feature ID duality** (MEDIUM — 13 UPPERCASE vs 38+ kebab IDs, no bridge) and **Blockoli semantic search** (LOW — heavy lift, stubbed).

The platform's critical gap has shifted from **"temporal safety net not accessible from desktop"** (post-037) to **"no identifier bridge between code attribution and spec registry"** (post-038). This is a data architecture debt, not a capability gap.

## Residuals inventory

### Feature ID duality (MEDIUM — authority-map)

13 UPPERCASE code IDs (`// Feature: FEATUREGRAPH_REGISTRY` etc.) vs 38+ kebab spec IDs (`032-opc-inspect-governance-wiring-mvp` etc.), no mapping. The scanner's alias system works in legacy YAML mode but not in the compiled registry. This grows worse with every new feature.

**Fix:** Needs an ADR to choose the canonical format and mapping strategy before implementation. Options: (a) add `aliases` field to compiled registry JSON, (b) derive UPPERCASE from kebab via convention, (c) adopt kebab everywhere and migrate code headers.

### Cross-platform axiomregent CI residuals (LOW)

T003 (macOS x86_64) and T004 (Linux x86_64/arm64) deferred to CI runners. CI workflow exists (`.github/workflows/build-axiomregent.yml`) but hasn't run yet. Will resolve automatically when CI runners are available.

### Blockoli semantic search (LOW — heavy lift)

Desktop UI stub exists but backend is not wired. The `crates/blockoli/` library exists but Tauri command integration has not been scoped.

## Ordered next-slice priority

### Slice E: Feature ID reconciliation (ADR + Feature 039)

**Why first:** Only remaining MEDIUM item. Growing urgency (38+ features, every new feature adds entries in both systems with no cross-reference). Purely a data architecture concern — no runtime impact, but increasingly confusing for governance panel consumers.

Scope:
1. **ADR** — ~~choose canonical ID format, define mapping strategy~~ **Done:** `docs/adr/0001-feature-id-reconciliation.md` (kebab `id` + `codeAliases`). Three options considered:
   - (a) Add `aliases` field to compiled registry JSON — scanner emits both forms, consumers match either.
   - (b) Convention-derived: `032-opc-inspect-governance-wiring-mvp` → `OPC_INSPECT_GOVERNANCE_WIRING_MVP` or similar. Zero-config but noisy.
   - (c) Adopt kebab everywhere and migrate all `// Feature:` headers in code. Clean but large change surface.
2. **Implement chosen strategy** in `Scanner` and `spec-compiler`.
3. **Verify** — governance panel shows unified view, featuregraph cross-references resolve.

Estimated effort: ADR is 1 session (claude-opus or claude). Implementation is 1-2 cursor sessions depending on chosen option.

### Slice F: Blockoli semantic search (future)

**Why second:** Lowest urgency. Heavy lift. Desktop UI stub exists but backend (embedding, indexing, query) is not wired. Requires scoping the `crates/blockoli/` library API and determining Tauri command signatures.

Not ready to scaffold — needs discovery pass first.

### Slice G: Desktop UI for checkpoint/restore (future)

**Why after Slice E:** Feature 038 wired the backend commands. The next product-visible step is a UI for checkpoint/restore in the desktop app. Depends on design decisions about where checkpoint controls appear (per-project? per-agent-session?).

Not ready to scaffold — needs design input.

## Fork resolution

**Chosen path: reconcile identifiers → expand product UI.**

Features 032–038 established, broadened, and completed the governed execution thesis including the temporal safety net. The next priority is reconciling the dual identity system (Slice E), then expanding product-visible capabilities (Slices F/G). The platform is now capability-complete for the governed execution story; remaining work is polish, coverage, and product surface.

## Recommended promotion set

### Promote now

- **Feature 039 ADR** — feature ID reconciliation (needs design decision before spec scaffolding)

### Promote next

- **Blockoli semantic search discovery** — scoping pass on `crates/blockoli/` API and Tauri command signatures
