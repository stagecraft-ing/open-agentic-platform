# Render-Path Decomposition Design (D-1 Option 3)

**Branch:** `cut-d/autonomous-run-20260519-025506`
**Date:** 2026-05-19
**Method:** Read of `tools/oap-code-index-enrich/src/render.rs`, `tools/codebase-indexer/src/types.rs`, `tools/oap-code-index-enrich/src/types.rs`, `AGENTS.md`.

## Current state (post-W-07b)

Cut D W-07b lifted the markdown renderer out of `codebase-indexer` and into `oap-code-index-enrich`. Today:

- **`codebase-indexer compile`** → emits `build/codebase-index/index.json` (Layers 1 + 2 + Diagnostics).
- **`codebase-indexer check`** → staleness gate against the committed `index.json`.
- **`codebase-indexer render`** → **does not exist** (lifted out).
- **`oap-code-index-enrich`** → reads `index.json` + scans factory adapters / infra / workflow traceability → emits `build/codebase-index/index-oap.json` (Layers 1 + 2 + 3 + 4 + 5 + Diagnostics).
- **`oap-code-index-enrich render`** → reads `index-oap.json` → emits `build/codebase-index/CODEBASE-INDEX.md`.

The generic indexer cannot produce a markdown view today; the only render path is OAP-flavoured and requires the OAP enricher to have run.

## Why we got here

W-07b's motivation was decoupling: the generic indexer (spec-spine adoptable as a standalone) must not depend on OAP-specific types like `AdapterRecord` / `Infrastructure` / `WorkflowTrace`. The renderer at the time of W-07b assumed the enriched `EnrichedView` shape (Layers 1-5); moving the renderer with the enriched shape kept the generic indexer's type surface clean.

The trade-off: an adopter extracting only the spec-spine cannot run `<some-binary> render` to get a markdown view of their codebase. The `/init` protocol in AGENTS.md only knows about `oap-code-index-enrich render` (line 14).

D-1 from the `/init` trace asks: **how does an adopter get a generic structural-summary view without the OAP overlay binary?**

## Option 3 (the locked resolution)

Restore a `render` subcommand to `codebase-indexer` that produces the generic-core markdown view, and treat the OAP overlay as an enrichment on top. The contract preserves the W-07b decoupling: `codebase-indexer render` knows nothing about OAP-specific types.

## Column classification

Every section of today's `render_markdown` (`tools/oap-code-index-enrich/src/render.rs`) classified by data source:

| Section | Render lines | Classification | Data source | Rationale |
|---|---|---|---|---|
| Header (title + build version + content hash) | 56–66 | **generic** | `index.json: build.{indexer_version, content_hash}` | Comes from generic Layer 0. |
| Layer 1: Inventory — Rust crates (Name, Path, Kind, Version, Spec, Internal Deps) | 73–116 | **generic** | `index.json: inventory[]` (PackageRecord) | All fields are in the generic shape. The "Spec" column renders `spec_ref` which already lives in Layer 1. |
| Layer 1: Inventory — NPM packages | 119–135 | **generic** | `index.json: inventory[]` (PackageRecord with NpmPackage/NpmWorkspace kinds) | Same. |
| Layer 2: Traceability summary line (`N mapped, M orphans, K untraced paths`) | 137–144 | **generic** | `index.json: traceability` | Already typed in generic. |
| Layer 2: Implements table | 155–185 | **generic** | `index.json: traceability.mappings[]` with `implementing_paths[]` | All fields (`spec_id`, `spec_status`, `implementing_paths`, `source` enum, `primary` boolean) are in the generic types. |
| Layer 2: Amends table | 187–212 | **generic** | `index.json: traceability.mappings[]` with `amends` / `amendment_record` | Spec 133 surfaced these into the generic Layer 2. |
| Layer 3: Factory adapters | 215–237 | **oap-specific** | `index-oap.json: factory[]` (AdapterRecord) | Type is OAP-only (`tools/oap-code-index-enrich/src/types.rs:14`). |
| Layer 4: Infrastructure — Tools | 240–250 | **oap-specific** | `index-oap.json: infrastructure.tools[]` (ToolEntry — lists the OAP-specific tool roster) | The roster is OAP-specific: `tools/codebase-indexer`, `tools/oap-code-index-enrich`, `tools/policy-compiler`, etc. — these names are not the spec-spine baseline. |
| Layer 4: Infrastructure — Agents | 251–260 | **oap-specific** | `index-oap.json: infrastructure.agents[]` | `.claude/agents/` is OAP-adoption-of-Claude-Code; not core spec-spine. |
| Layer 4: Infrastructure — Commands | 261–270 | **oap-specific** | `index-oap.json: infrastructure.commands[]` | Same. |
| Layer 4: Infrastructure — Rules | 271–280 | **oap-specific** | `index-oap.json: infrastructure.rules[]` | Same. |
| Layer 4: Infrastructure — Schemas | 281–290 | **oap-specific** | `index-oap.json: infrastructure.schemas[]` | Could be generic in principle (the spec-spine has its own schemas), but the entry shape is OAP — sourced from the OAP enricher's schema-scan. |
| Layer 5: Workflow Traceability | 293–314 | **oap-specific** | `index-oap.json: workflowTraceability[]` (WorkflowTrace) | OAP-specific because the trace conventions (workflow→spec) are OAP. |
| Diagnostics — Errors | 319–325 | **generic** | `index.json: diagnostics.errors[]` | Diagnostics are emitted by both the generic indexer and the OAP enricher; the *generic* errors render here. |
| Diagnostics — Warnings | 326–331 | **generic** | `index.json: diagnostics.warnings[]` | Same. |

### Summary

- **8 sections** are generic (header, Layer 1 Rust, Layer 1 NPM, Layer 2 summary, Layer 2 implements, Layer 2 amends, Diagnostics errors, Diagnostics warnings).
- **7 sections** are OAP-specific (Layer 3 factory, Layer 4 ×5 sub-sections, Layer 5 workflow).

## Decomposition

### `codebase-indexer render` (new subcommand, generic)

**Reads:** `index.json` (Layer 1 + Layer 2 + Diagnostics shape; SCHEMA_VERSION 2.0.0).

**Produces:** `build/codebase-index/CODEBASE-INDEX.md` (after I9: `.derived/codebase-index/CODEBASE-INDEX.md`).

**Schema of output (markdown sections, in order):**

1. `# Codebase Index — <project-name>` header
2. `> Auto-generated by codebase-indexer v<version>.\n> Content hash: <hash>` provenance line
3. `## Layer 1: Crate & Package Inventory (N total)` with Rust crates table (Name / Path / Kind / Version / Spec / Internal Deps) and NPM packages table (Name / Path / Kind / Version / Spec)
4. `## Layer 2: Spec → Code Traceability (N mapped, M orphans, K untraced paths)` with Implements + Amends tables
5. `## Diagnostics` with Errors + Warnings tables (if any present)

**Adopter affordance:** running `codebase-indexer render` alone produces a self-contained generic structural summary without OAP-specific knowledge.

### `oap-code-index-enrich render` (existing; reframed)

**Reads:** `index-oap.json` (the OAP enricher's output, which already contains Layers 1-5).

**Produces:** `build/codebase-index/CODEBASE-INDEX.md` (after I9: `.derived/...`) — **same path as the generic render, but enriched**.

**Behavior:**

- Renders the full enriched markdown: header + Layer 1 + Layer 2 + Layer 3 + Layer 4 (×5 sub-sections) + Layer 5 + Diagnostics.
- Overwrites the generic render if it has already been emitted.
- The header still says "Auto-generated by codebase-indexer v<version>" with the suffix "(rendered by oap-code-index-enrich)" preserved from W-07b.

**Implementation hint:** the enricher can either (a) keep its current self-contained renderer (just renders all 5 layers from `EnrichedView`), or (b) delegate the Layer 1+2+Diagnostics blocks to a function exposed by `codebase-indexer` as a library export and append the Layer 3/4/5 blocks itself. Option (b) is cleaner (single canonical generic renderer) but introduces a build-time dep from `oap-code-index-enrich` on `codebase-indexer`'s lib — which it already has (`open_agentic_codebase_indexer = { path = "../codebase-indexer" }`, see `tools/oap-code-index-enrich/Cargo.toml`). Option (b) is recommended.

## Contract between generic core and OAP overlay

```
┌──────────────────────────┐
│ specs/*/spec.md          │
│ + Cargo.toml             │
│ + package.json           │
│ + .claude/{a,c,r}/**     │ ← raw inputs
│ + factory/adapters/**    │
│ + .github/workflows/*.yml│
└────────────┬─────────────┘
             │
             ▼
┌───────────────────────────────┐
│  codebase-indexer compile     │
│  emits index.json             │
│  (Layer 1 + 2 + Diagnostics)  │
└────────────┬──────────────────┘
             │
       ┌─────┴─────┐
       │           │
       ▼           ▼
┌──────────────┐  ┌──────────────────────────────┐
│ codebase-    │  │ oap-code-index-enrich        │
│ indexer      │  │ reads index.json,            │
│ render       │  │ scans factory/.claude/       │
│              │  │ workflows;                   │
│ writes:      │  │ emits index-oap.json         │
│ CODEBASE-    │  │ (L1+L2+L3+L4+L5+Diag)        │
│ INDEX.md     │  └──────────────┬───────────────┘
│ (generic     │                 │
│  view)       │                 ▼
└──────────────┘  ┌──────────────────────────────┐
                  │ oap-code-index-enrich render │
                  │ writes:                      │
                  │ CODEBASE-INDEX.md            │
                  │ (enriched view, overwrites)  │
                  └──────────────────────────────┘
```

The contract:

1. **Output-path identity.** Both renderers write to the same path. The last writer wins. The full OAP-context flow runs generic render, then OAP overlay render — the overlay version is what the user sees. The spec-spine adopter (no OAP enricher present) runs only generic render and gets the generic view.
2. **Schema-version compatibility.** Both renderers consume the same Layer 1 + Layer 2 shape (SCHEMA_VERSION = 2.0.0 from `tools/codebase-indexer/src/types.rs`). When the schema bumps, both renderers update lockstep.
3. **No type coupling.** `codebase-indexer render` knows only generic types (PackageRecord, TraceMapping, Diagnostics). The OAP enricher's `render.rs` continues to read `EnrichedView` which extends the generic shape with Layer 3/4/5 fields. The generic indexer's library exports a `render_generic(&Index) -> String` function the enricher can reuse for the L1+L2+Diagnostics part of its output (recommended path).
4. **Diagnostics namespace.** Both `index.json: diagnostics` (generic) and `index-oap.json: diagnostics` (enriched, includes overlay-side warnings). Generic render shows only generic diagnostics. OAP render shows the union.

## Why this doesn't reintroduce the W-07b cycle

W-07b's cycle concern was: generic indexer should not depend on OAP-specific overlay types. Option 3 preserves that:

- `codebase-indexer render` operates on the generic types it already owns (PackageRecord, TraceMapping, Diagnostic).
- `oap-code-index-enrich render` reads the OAP overlay types (AdapterRecord, Infrastructure, WorkflowTrace) which live in `tools/oap-code-index-enrich/src/types.rs` — those types stay where they are.
- The dependency direction is downstream-only: `oap-code-index-enrich` depends on `codebase-indexer` (already true); `codebase-indexer` never depends on `oap-code-index-enrich`.

The decomposition splits *the renderer* across two binaries by responsibility surface; it does not re-introduce a type-graph cycle.

## /init protocol change required

Current AGENTS.md "New Sessions" Step 0 (line 14):

> `oap-code-index-enrich render` → `build/codebase-index/CODEBASE-INDEX.md` — rendered structural summary (Cut D W-07b moved this from `codebase-indexer render`; run only if the markdown is missing)

After the decomposition, AGENTS.md should read (proposed text for I10):

> Render the structural summary:
> 1. `codebase-indexer render` → emits the generic-core view to `<repo-root>/.derived/codebase-index/CODEBASE-INDEX.md`. Always safe to run.
> 2. (OAP context only) `oap-code-index-enrich render` → overlays Layer 3/4/5 onto the same path. Run only if the OAP enricher is installed (adopters extracting just the spec-spine skip step 2).

This is the D-2.10 (render binary identity) resolution. D9 ratifies it as one of the 11 drift resolutions; D8 is the design.

`/init`'s read of the rendered markdown is unaffected — same file path, same human-shaped view, with optional enrichment.

## I11 readiness summary

- **Code changes in `codebase-indexer`:**
  - Add `render` subcommand to `tools/codebase-indexer/src/main.rs` (or `lib.rs`'s CLI dispatch).
  - Implement `render_generic(&Index) -> String` in `tools/codebase-indexer/src/render.rs` (new file).
  - Wire the subcommand to write `<repo-root>/.derived/codebase-index/CODEBASE-INDEX.md` after I9 (or `build/...` if I11 lands before I9).
  - Files: ~2 (new `render.rs`, edited `main.rs` or `lib.rs`).
- **Code changes in `oap-code-index-enrich`:**
  - Replace the current self-contained `render_markdown` with delegation: call `open_agentic_codebase_indexer::render::render_generic` for the L1+L2+Diagnostics block, then append L3/L4/L5 OAP-specific blocks itself.
  - The current `render.rs` shrinks; the diff is moderate.
  - Files: ~1 (`tools/oap-code-index-enrich/src/render.rs` edited).
- **`AGENTS.md` change:** "New Sessions" Step 0 line 14 → split into two ordered steps as above. ~5 lines edit.
- **`.claude/rules/governed-artifact-reads.md` change** (`docs/analysis/cleanup/reference-audit.md` Group O lists this rule file's role): the consumer table at lines 15–18 lists `build/codebase-index/CODEBASE-INDEX.md` as "read directly". After Option 3 lands, both renderers produce it; the rule file should add a sentence to that effect. ~2 lines edit.
- **Test surface:** add a `codebase-indexer render` golden test (snapshot the generic output for a fixture index.json). Update `oap-code-index-enrich`'s existing render test if it asserts on the full enriched output.
- **Estimated complexity:** **medium** — the architectural design is settled here; the implementation is a moderate refactor of `oap-code-index-enrich/src/render.rs` plus a new file in `codebase-indexer`. The risk is the delegation contract (option b): ensure `render_generic` is byte-identical to the L1+L2 block of the OAP renderer to avoid double-rendering drift.

## Open questions (surface for operator triage)

1. **Delegation vs. duplication.** Recommendation is option (b) — `oap-code-index-enrich` delegates the L1+L2 block to `codebase-indexer::render::render_generic`. Alternative (a) — each renderer keeps its own L1+L2 rendering code, accepting near-duplication. Recommendation: delegation; one source of truth for the generic block.
2. **Path naming.** Render output stays at `CODEBASE-INDEX.md` (post-I9 under `.derived/codebase-index/`). Confirm vs. introducing `CODEBASE-INDEX-CORE.md` (generic) and `CODEBASE-INDEX.md` (enriched) — recommended **no**: single canonical filename with last-writer-wins semantics is cleaner.
3. **Schema version coupling.** Both renderers consume `index.json` with `SCHEMA_VERSION = "2.0.0"`. When that bumps, both renderers update. Document this coupling in `tools/codebase-indexer/src/types.rs` near `SCHEMA_VERSION` so a future schema-bump author knows to touch both renderers.
4. **Adopter affordance.** Confirm the spec-spine adopter flow: after extraction, the adopter runs `make registry` which should produce both `index.json` and the generic `CODEBASE-INDEX.md`. The Makefile changes accordingly in I11 (the `make index` and `make index-render` recipes split per binary).
