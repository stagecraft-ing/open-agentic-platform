---
id: "141-aim-vue-node-source-id-template-name-alignment"
slug: aim-vue-node-source-id-template-name-alignment
title: "Align aim-vue-node scaffold_source_id with template.json::templateName"
status: approved
implementation: in-progress
owner: bart
created: "2026-05-06"
approved: "2026-05-06"
kind: amendment
risk: low
amends: ["140"]
depends_on:
  - "140"  # aim-vue-node manifest cutover (introduces scaffold_source_id and migration 36)
code_aliases: ["AIM_VUE_NODE_SOURCE_ID_TEMPLATE_NAME_ALIGNMENT"]
implements:
  # Single source of truth for the canonical aim-vue-node source-id literal.
  - path: platform/services/stagecraft/api/factory/oapNativeAdapters.ts
  # jsdoc references to the canonical literal — kept in sync with the constant.
  - path: platform/services/stagecraft/api/factory/translator.ts
  - path: platform/services/stagecraft/api/factory/substrateBrowser.ts
  # Forward-migration of migration-36 substrate row + sibling factory_upstreams row.
  - path: platform/services/stagecraft/api/db/migrations/37_aim_vue_node_canonical_source_id.up.sql
  - path: platform/services/stagecraft/api/db/migrations/37_aim_vue_node_canonical_source_id.down.sql
  - path: platform/services/stagecraft/api/db/migrations/37_aim_vue_node_canonical_source_id.test.ts
  # Test fixtures + assertions updated to the canonical literal in lockstep.
  - path: platform/services/stagecraft/api/factory/translator.test.ts
  - path: platform/services/stagecraft/api/factory/projection.test.ts
  - path: platform/services/stagecraft/api/factory/artifacts.test.ts
  - path: platform/services/stagecraft/api/projects/scaffold/scheduler.test.ts
  - path: platform/services/stagecraft/api/projects/scaffold/scaffold.test.ts
  # Vite exclude list extended for the migration-37 isolated test (mirrors mig 36).
  - path: platform/services/stagecraft/vite.config.ts
summary: >
  Spec 140 §2.1 fixed the canonical scaffold_source_id for aim-vue-node
  as `aim-vue-node-template`. The upstream's own
  `template.json::templateName` is `aim-vue-node`. This amendment aligns
  the code constant, the migration-36 substrate row body, and the
  factory_upstreams source-id key with the upstream's self-declared
  name (one canonical id end-to-end), so the readiness gate resolves
  for orgs whose template upstream is registered via the legacy
  two-row form without re-entering the same repo URL.
---

# 141 — aim-vue-node source-id alignment with `template.json::templateName`

> **Amendment** of [`140-aim-vue-node-scaffold-source-id-cutover`](../140-aim-vue-node-scaffold-source-id-cutover/spec.md).
> Spec 140's design (one canonical scaffold_source_id; source-id-keyed
> lookup against `factory_upstreams`) is unchanged. Only the literal
> value of that id is refined.

## 1. Background

Spec 140 §2.1 picked `aim-vue-node-template` as the canonical
`scaffold_source_id` for the aim-vue-node adapter, with the implicit
rationale that the orchestration source-id namespace shares the
`factory_upstreams.source_id` table and a `-template` suffix
disambiguates "the scaffold for aim-vue-node" from "the orchestration
for aim-vue-node".

Two pieces of evidence post-dating §2.1's authoring make
`aim-vue-node-template` the wrong choice:

1. **The upstream declares its own name as `aim-vue-node`.** The
   `template.json` at the root of `GovAlta-Pronghorn/template`
   carries `templateName: "aim-vue-node"`. The upstream is the
   authoritative source of its own identity; the inventoried
   `-template` suffix is a downstream invention.

2. **No collision exists to defend against.** The orchestration
   source-id for aim-vue-node is `goa-software-factory` (named after
   the orchestration upstream repo, not after the adapter). The
   scaffold source-id `aim-vue-node` does not collide with any other
   row in `factory_upstreams`.

Concrete failure mode that motivated the amendment: existing orgs
register the template upstream through the legacy two-row
`POST /api/factory/upstreams` form, which writes
`factory_upstreams.source_id = 'legacy-template-mixed'` (per spec 139
Phase 4b — `upstreams.ts:278`). The readiness gate at
`scaffoldReadiness.ts:131` queries `factory_upstreams WHERE source_id
IN (declaredSourceIds)` with `declaredSourceIds = {'aim-vue-node-template'}`
and finds nothing → `blocker='no-scaffold-source-resolved'` → the
"aim-vue-node — needs scaffold source registered" banner on
`/app/projects/new`. Aligning the canonical id with the upstream's
`templateName` lets a one-shot migration promote
`legacy-template-mixed` to a sibling row keyed `aim-vue-node` cleanly,
without inventing alias-table machinery the spec 140 cutover set out
to retire.

## 2. Resolution

### 2.1 Code constant

Rename
`OAP_NATIVE_ADAPTERS["aim-vue-node"].scaffoldSourceId` from
`"aim-vue-node-template"` to `"aim-vue-node"`. All four spec 140
implementation paths (`projection.ts`, `translator.ts`, `scheduler.ts`,
`scaffoldReadiness.ts`) read `scaffold_source_id` through this
constant or through the manifest field — no other production literal
needs editing.

### 2.2 Migration 37

`37_aim_vue_node_canonical_source_id.up.sql` — idempotent, two effects:

1. **UPDATE** the migration-36 synthetic substrate row(s)
   (`origin = 'oap-self'`, `path = 'adapters/aim-vue-node/manifest.yaml'`)
   replacing `aim-vue-node-template` with `aim-vue-node` in
   `upstream_body` and `frontmatter->>'scaffold_source_id'`.
   Filter `WHERE frontmatter->>'scaffold_source_id' =
   'aim-vue-node-template'` so re-runs are no-ops.

2. **INSERT** a sibling `factory_upstreams` row per org keyed
   `(org_id, source_id='aim-vue-node')`, role `'scaffold'`, copying
   `repo_url` / `ref` / `subpath` from the existing
   `legacy-template-mixed` row. `ON CONFLICT (org_id, source_id) DO
   NOTHING` so the migration is safe to re-run.

The legacy `legacy-template-mixed` row stays in place — the legacy
two-row UI form continues to read/write it for the singleton compat
path (`upstreams.ts:152-153`). Only the source-id-keyed lookup uses
the new sibling.

### 2.3 Test + jsdoc updates

Five test files assert on the literal `aim-vue-node-template`:

- `platform/services/stagecraft/api/factory/translator.test.ts`
- `platform/services/stagecraft/api/factory/projection.test.ts`
- `platform/services/stagecraft/api/factory/artifacts.test.ts`
- `platform/services/stagecraft/api/projects/scaffold/scheduler.test.ts`
- `platform/services/stagecraft/api/projects/scaffold/scaffold.test.ts`

Updated to assert `aim-vue-node`. Two jsdoc comment references
(`translator.ts:701`, `translator.ts:725`, `substrateBrowser.ts:27`)
updated in lockstep.

The migration-36 idempotence test
(`36_aim_vue_node_manifest_cutover.test.ts`) is **not** updated. It
runs migration 36 in isolation against an existing schema and asserts
the immediate post-migration-36 state, which still contains
`aim-vue-node-template` from the immutable migration-36 SQL.
Migration 37 has its own isolated test
(`37_aim_vue_node_canonical_source_id.test.ts`) covering the
post-amendment state.

### 2.4 Migration 36 immutability

Migration 36 SQL is **not** edited. Per repo convention and the
runner's version-only tracking (`scripts/migrate.mjs`), an applied
migration's body must not change. Migration 36 continues to insert
`aim-vue-node-template`; migration 37 forward-migrates that value to
`aim-vue-node` in the same transaction sequence on every cluster.

## 3. Acceptance criteria

- **AC-1** — `OAP_NATIVE_ADAPTERS["aim-vue-node"].scaffoldSourceId`
  is the literal `"aim-vue-node"`.
- **AC-2** — Migration 37 applied: each migration-36 synthetic
  substrate row carries `frontmatter.scaffold_source_id =
  "aim-vue-node"` and the same string in its `upstream_body` YAML.
- **AC-3** — Migration 37 applied: every org with a
  `legacy-template-mixed` row also has a sibling row keyed
  `(org_id, 'aim-vue-node')` with `role='scaffold'` and matching
  `repo_url` / `ref` / `subpath`.
- **AC-4** — The Create Project page no longer shows the
  `aim-vue-node — needs scaffold source registered` banner for orgs
  whose template upstream is registered via the legacy two-row form.
- **AC-5** — No production code path or non-migration-36 test
  asserts on the literal `"aim-vue-node-template"`. Migration 36
  SQL and its isolated test remain frozen at the pre-amendment
  state.

## 4. Out of scope

- A source-id input on the upstream-config UI form — the legacy
  two-row form continues to be the only writer, and migration 37
  bridges its output to the canonical key. A source-id-aware UI is
  tracked separately under spec 139's "N-per-org source endpoints"
  surface.
- Renaming the orchestration source-id for aim-vue-node from
  `goa-software-factory` — orthogonal, not motivated by any current
  failure.

## 5. Provenance

- **2026-05-06** — Cluster failure observed:
  `Scaffold source not resolved. aim-vue-node — needs scaffold
  source registered` banner on `/app/projects/new` immediately
  after migration 36 deployed.
- **Upstream evidence** — `template.json::templateName =
  "aim-vue-node"` in `GovAlta-Pronghorn/template@main` (predates
  spec 140's authoring).
- **CONST-005 framing** — Spec 140 was authored without sight of
  the upstream's `template.json`. Per
  `.claude/rules/adversarial-prompt-refusal.md` "What this rule
  does NOT do": "It does not block legitimate amendments —
  refining a spec's narrative to clarify or extend is welcome."
  The §2.1 literal is refined here, not retroactively justified;
  spec 140's design (one canonical id, source-id-keyed lookup) is
  preserved verbatim.
