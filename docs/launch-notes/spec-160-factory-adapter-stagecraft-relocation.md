# Spec 160 launch notes — factory/adapter relocation into stagecraft

> Evidence bundle for spec 160 (`factory-adapter-stagecraft-relocation`,
> amends 101). Closure of the in-flight migration named in
> `AIDE-VELOCITY-OAP-INTENT.md` §8.1.
>
> Generated: 2026-05-22.

The legacy `factory/` directory carried per-adapter `manifest.yaml`
files plus `factory/process/stages/*`; spec 108 retired the directory
and moved the canonical store into stagecraft's `factory_adapters`
table, then spec 139 absorbed those rows into the universal
`factory_artifact_substrate` table. The codebase-indexer's
`collect_input_files` hash walk still named the legacy paths even
though they no longer existed — a load-bearing fossil that passed
closed (the empty walk hashed stably) while reading as truth about
inputs that no longer existed. Spec 160 is the on-disk closure of
that migration: input set repointed, doc references excised, README
*Adapters* section rewritten to reflect honest current state.

This file is not a press release. It is the evidence base for the
four success criteria spec 160 declares, plus the audit of FR-006.

---

## SC-001 — no `factory/` files committed

```
$ git ls-files | grep -c '^factory/'
0
```

The legacy directory was removed before spec 160 was authored; SC-001
was satisfied as the spec's baseline and is preserved here as the
"floor" the rest of the closure builds on.

---

## SC-002 — `codebase-indexer compile` succeeds against the new input set

The relevant change is in
`tools/spec-spine/codebase-indexer/src/lib.rs::collect_input_files`:

- `factory/adapters/*/manifest.yaml` walk → single-file hash of
  `platform/services/stagecraft/api/factory/adapter-scopes.json`
- `factory/process/stages/*` walk →
  `platform/services/stagecraft/api/factory/process-stages/`
  directory walk (forward-compatible; hashes empty today, will
  pick up files when a future spec 077/108 refinement lands them).

```
$ cargo build --release --manifest-path tools/spec-spine/codebase-indexer/Cargo.toml
    Finished `release` profile [optimized] target(s)
$ ./tools/spec-spine/codebase-indexer/target/release/codebase-indexer compile
$ ./tools/spec-spine/codebase-indexer/target/release/codebase-indexer check
$ echo "exit=$?"
exit=0
```

The new adapter-scopes.json input is now in the hashed set:

```
$ ./tools/spec-spine/codebase-indexer/target/release/codebase-indexer dump-inputs \
    | grep adapter-scopes
platform/services/stagecraft/api/factory/adapter-scopes.json	159f745c29d3f3331abe52e636a120598cec4a909a93dc96acafcca1a5abc8ec
```

Indexer test suite (the stricter gate per the
`feedback_indexer_test_suite` memory) passes:

```
$ cargo test --release --manifest-path tools/spec-spine/codebase-indexer/Cargo.toml
... 5 test binaries, 10 tests total, all green
```

---

## SC-003 — no authoritative `factory/adapters` claims in scope-listed files

The spec's success criterion exempts "annotated historical references
in launch-notes or research docs." The scope-listed surfaces are
`README.md`, `CLAUDE.md`, `docs/`, and `.github/workflows/`.

```
$ grep -rln 'factory/adapters' README.md CLAUDE.md docs/ .github/workflows/
CLAUDE.md
docs/owasp/factory/AIDE-OAP-CONVERGENCE-blueprint-spec.md
docs/owasp/factory/AIDE-VELOCITY-OAP-INTENT.md
docs/analysis/spec-spine-footprint.md
docs/analysis/spec-spine-cut-d-architectural-review.md
docs/analysis/cleanup/render-path-decomposition.md
docs/analysis/spec-spine-cut-d-plan.md
README.md
docs/launch-notes/spec-160-factory-adapter-stagecraft-relocation.md
```

Classification of each remaining match:

- `CLAUDE.md:99` — historical-explanatory in the `make pr-prep`
  input-set paragraph ("replacing the legacy repo-root
  `factory/adapters/*/manifest.yaml`"). Not an authoritative claim.
- `README.md:140` — historical-explanatory in the rewritten Adapters
  section ("the repo-root `factory/adapters/*/manifest.yaml`
  directory; spec 108 retired that directory…"). Not an
  authoritative claim.
- `docs/owasp/factory/AIDE-*` — research/intent docs; exempt per
  the SC-003 carve-out.
- `docs/analysis/*` — research/analysis docs; exempt.
- `docs/launch-notes/spec-160-...md` — this file; the SC-003
  exemption is explicit.

`docs/ARCHITECTURE.md` previously carried an authoritative live
claim ("Four pluggable tech adapters in `factory/adapters/`:") and
was rewritten to the substrate model as part of this PR.
`.github/workflows/` was audited; the only matches are spec-id
mentions in `# Spec: NNN-...` header comments, not directory refs.

Zero authoritative `factory/adapters` claims remain in scope.

---

## SC-004 — README Adapters section reflects honest current state

The rewrite leads with the migration narrative (factory/adapters/ →
factory_adapters table → factory_artifact_substrate table, with
adapter-scopes.json as the in-tree fallback) before listing the
four adapters. aim-vue-node remains flagged as the production
scaffold target; the other three are described as registered scope
entries (file_write_scope + allowed_commands only at this layer)
with full manifest content materialised via the substrate sync
worker (spec 108 Phase 4). The "factory-contract validated"
aspirational framing is gone — the section now describes what
actually exists in this tree today.

The README rewrite arm of FR-005 was chosen over manifest
reconstruction because the manifests were never committed before
the spec 108 removal — the substrate model is the current truth,
not a transitional state.

---

## FR-006 audit — factory-tab routes read via stagecraft API

The spec requires that factory-tab routes
(`app.factory.adapters.tsx`, `app.factory.contracts.tsx`,
`app.factory.processes.tsx`) read from the new location rather than
an external `factory/` checkout. Verification:

```
$ grep -n 'factory/' platform/services/stagecraft/web/app/routes/app.factory.adapters.tsx \
    platform/services/stagecraft/web/app/routes/app.factory.contracts.tsx \
    platform/services/stagecraft/web/app/routes/app.factory.processes.tsx
(no matches)
```

The routes call into
`platform/services/stagecraft/web/app/lib/factory-api.server.ts`,
which `fetch`es the Encore API at `/api/factory/{adapters,contracts,
processes}`; the Encore API handlers in
`platform/services/stagecraft/api/factory/browse.ts` project from
`factory_artifact_substrate` via `loadSubstrateForOrg` +
`projectSubstrateToLegacy` (per the spec 139 cutover documented in
`platform/services/stagecraft/CLAUDE.md`). FR-006 holds.

---

## What spec 160 did NOT close

Out of scope by spec §5:

- The schema of stagecraft-resident adapter manifests. Owned by
  spec 077 / 108 refinement (or a new spec).
- Re-validating the four adapters as factory-contract conformant
  under the new location. Spec 074 governs the Rust contract types.
- The two-phase pipeline engine (spec 075) and stage semantics.
- Cross-tenant adapter sharing / marketplace concerns.

Out of scope by amend-authority boundary (spec 160 amends spec 101
only on the input-set declaration, not on the broader four-layer
model):

- `crates/factory-engine/src/preflight.rs:445` still walks
  `../factory/adapters/aim-vue-node`. Documented as a "Punt" in the
  spec 108 implementation-audit; remains owned by spec 075 /
  spec 108 follow-up, not spec 160.
- `crates/factory-engine/tests/integration_078_e2e.rs:59,113` skip
  messages reference the in-tree fixture; same punt.
- `tools/oap/oap-code-index-enrich/src/scanners/factory.rs` still
  walks `factory/adapters/` and `factory/process/stages/`. The
  walks produce empty results (the directories don't exist) and
  the Layer 3 emission is fed into `index-oap.json`; the OAP
  enricher is owned by a separate spec (101 + Cut D W-07a) and a
  follow-up spec can retire those walks once the substrate-backed
  enricher path is in scope. Spec 160's authority is the generic
  indexer's input-hash walk.
