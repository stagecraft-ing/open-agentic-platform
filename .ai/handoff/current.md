> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

Features **032–039** delivered and reviewed. **Feature 040** (blockoli semantic search wiring) scaffolded — spec drafted, ready for implementation. Platform at capability plateau; now expanding product surface.

## Agent pack

**Cursor** (implementation), **Claude** (review / deep analysis), **Antigravity** (wide exploration), **Claude Opus** (`claude-opus` — synthesis and next-slice prioritization). Details: `.ai/README.md`, `.ai/prompts/baton-rules.md`, role cards under `.ai/handoff/`.

## Lifecycle note

Registry **`status`** in frontmatter must be one of **`draft` | `active` | `superseded` | `retired`** (Feature **000** / **003**). Delivery completion is proven by checked tasks + verification artifacts, not by status changes.

## Canonical feature authority

- **032–039:** All `status: active`, all complete. See individual `specs/<id>/spec.md` for details.
- **040 spec:** `specs/040-blockoli-semantic-search-wiring/spec.md` (status: **draft**, ready for implementation)

## Current execution truth

- **032–039:** All complete. All reviewed. All authority-map items resolved.
- **040:** Spec drafted. Discovery pass complete. Key findings:
  - `crates/blockoli/` — 1,284 LOC, fastembed (384-dim), KD-tree, SQLite persistence, asterisk parsing
  - `commands/search.rs` — two `todo!()` stubs already registered in `lib.rs` invoke handler
  - `SemanticSearchPanel.tsx` — fully implemented frontend, invokes `blockoli_search`
  - Integration pattern: `BlockoliState` managed state (like `TitorState`), direct `VectorStore::SQLiteStore(conn)` construction
  - Key risk: fastembed model download (~30 MB) on first use; hardcoded DB path needs override

## Baton

- Current owner: **claude-opus**
- Next owner: **cursor**
- Last baton update: 2026-03-29 — **claude-opus** Slice F: discovery pass complete, Feature 040 spec scaffolded (`specs/040-blockoli-semantic-search-wiring/spec.md`). Blockoli library + stubs + frontend all exist; spec defines 6 FRs for wiring them together.
- Requested outputs from **cursor**:
  1. Implement Feature 040: `BlockoliState` managed state, `blockoli_index_project`, `blockoli_search` commands. Verify end-to-end with `SemanticSearchPanel`.
- Recommended files to read: `specs/040-blockoli-semantic-search-wiring/spec.md`, `apps/desktop/src-tauri/src/commands/search.rs`, `crates/blockoli/src/lib.rs`, `crates/blockoli/src/vector_store/vector_store.rs`, `crates/blockoli/src/embeddings/encoder.rs`

## Requested next agent output

**cursor:** Implement Feature 040 (blockoli semantic search wiring). Spec at `specs/040-blockoli-semantic-search-wiring/spec.md`. Stubs + frontend already exist — wire them to the library.

## Promotion candidates for canonical artifacts

(All promoted)

---

## Recent outputs

- 2026-03-29 (claude-opus): Slice F discovery + Feature 040 scaffolded. Blockoli library (1,284 LOC, fastembed, KD-tree, SQLite) + Tauri stubs + frontend all exist. Spec: 6 FRs for `BlockoliState` + command wiring. Baton → **cursor**.
- 2026-03-29 (claude-opus): Post-039 synthesis complete. All authority-map items resolved (032–039). Platform at capability plateau. Updated authority-map, integration-debt, next-slice, promotion-candidates.
- 2026-03-29 (claude): Feature 039 review — all FRs/SCs pass, all 4 ADR gaps closed, zero orphan aliases, all original 032 concerns resolved. Two minor items (V-005 message wording, `language` key). Updated `claude-review.md`. Baton → **claude-opus**.
- 2026-03-29 (cursor): Feature 039 implemented — codeAliases pipeline (ADR, schema 1.1.0, compiler V-005/V-006, scanner, frontmatter, verification, golden). Baton → **claude**.
- 2026-03-29 (claude-opus): Post-ADR synthesis. Scaffolded Feature 039 (`specs/039-feature-id-reconciliation/` — 9 tasks). ADR edits bundled as T001. Updated next-slice, integration-debt, authority-map. Baton → **cursor**.
- 2026-03-29 (claude): ADR 0001 review — decision sound, 4 gaps found (schema bump, validation codes, scanner consumer contract, population ordering). Updated `claude-review.md`. Baton → **claude-opus**.
- 2026-03-29 (cursor): Slice E — ADR 0001 drafted (`docs/adr/0001-feature-id-reconciliation.md`): kebab canonical `id`, optional `codeAliases` in registry (option a). Updated integration-debt, baton. Baton → **claude**.
- 2026-03-29 (claude-opus): Post-038 synthesis complete. All authority-map HIGH items resolved (032–038). Updated authority-map, integration-debt, next-slice. Next slice: Feature ID reconciliation (Slice E — ADR needed). Baton → **cursor**.
- 2026-03-29 (antigravity): Feature 038 wide pass complete. Fixed `get_or_init` race in `titor.rs` by holding the write lock during both insertions. Baton → **claude-opus**.
- 2026-03-29 (claude): Feature 038 review — all FRs/SCs pass. One low-severity `get_or_init` race noted. `cargo check` + tests green. Updated `claude-review.md`. Baton → **antigravity**.
- 2026-03-29 (cursor): Feature 038 — `TitorState`, wired `titor_init` and five commands, `lib.rs` manage, spec `active`, tasks T001–T011, verification + spec-compiler. Baton → **claude**.
- 2026-03-29 (claude-opus): Post-037 synthesis. Closed T009 (spec-compiler). Scaffolded Feature 038 (titor Tauri command wiring — 11 tasks). Updated next-slice, integration-debt, authority-map. Baton → **cursor**.
- 2026-03-29 (antigravity): Wide pass on Feature 037. Fixed doc comment, added CI cache, fixed timeout, verified .exe tracking. Promoted canonical artifacts. Baton → **claude-opus**.
- 2026-03-29 (claude): Feature 037 review — all FRs pass (FR-001 partial). ToolTier fix correct, 1 stale doc comment in `lease.rs:97`. CI smoke test `timeout` won't work on macOS (continue-on-error masks it). T003/T004 deferral correct. Updated `claude-review.md`. Baton → **antigravity** for wide pass.
- 2026-03-29 (cursor): Feature 037 implemented — Windows x86_64 binary built (7.3 MB), MCP handshake verified, all 21 tools confirmed. Created build script and CI workflow. Fixed stale `Tier`→`ToolTier` in `agent.rs`. All tests green. Baton → **claude** for review.
- 2026-03-29 (claude-opus): Post-036 synthesis complete. Scaffolded Feature 037. Updated authority-map, next-slice, integration-debt. Baton → **cursor**.
- 2026-03-29 (antigravity): Feature 036 wide pass complete. Baton → **claude-opus**.
- 2026-03-29 (claude): Feature 036 review — all FRs/SCs pass. Baton → **antigravity**.
- 2026-03-29 (cursor): Feature 036 implemented (T001–T007). Baton → **claude**.
- 2026-03-29 (claude-opus): Feature 036 spec scaffolded. Baton → **cursor**.
- 2026-03-29 (antigravity): Slice A wide pass. Baton → **claude-opus**.
- 2026-03-29 (claude): Slice A review. Baton → **antigravity**.
- 2026-03-29 (cursor): Slice A complete. Baton → **claude**.
- 2026-03-29 (claude-opus): Post-035 synthesis. Baton → **cursor**.
