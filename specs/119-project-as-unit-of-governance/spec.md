---
id: "119-project-as-unit-of-governance"
slug: project-as-unit-of-governance
title: Project as Unit of Governance — Workspace Collapse
status: draft
implementation: pending
owner: bart
created: "2026-04-29"
kind: governance
risk: high
depends_on:
  - "000"  # bootstrap-spec-system (frontmatter convention extension)
  - "087"  # unified-workspace-architecture (the namesake being collapsed)
  - "092"  # workspace-runtime-threading
  - "094"  # unified-artifact-store
  - "099"  # workspace-scoped-persistence
  - "113"  # stagecraft-projects-rename-and-clone (clone copies knowledge)
  - "114"  # async-project-clone-pipeline (clone runs as the displacing primitive)
  - "115"  # knowledge-extraction-pipeline (workspace-scoped today, project-scoped after)
amends:
  - "000"
  - "087"
  - "092"
  - "094"
  - "099"
code_aliases: ["PROJECT_AS_GOVERNANCE_UNIT"]
implements:
  - path: platform/services/stagecraft/api/db
  - path: platform/services/stagecraft/api/projects
  - path: platform/services/stagecraft/api/knowledge
  - path: platform/services/stagecraft/api/sync
  - path: platform/services/stagecraft/api/workspaces
  - path: platform/services/stagecraft/web
  - path: crates/orchestrator
  - path: crates/factory-contracts
  - path: crates/run
  - path: apps/desktop
  - path: .specify/contract.md
  - path: specs/000-bootstrap-spec-system
  - path: specs/087-unified-workspace-architecture
  - path: specs/092-workspace-runtime-threading
  - path: specs/094-unified-artifact-store
  - path: specs/099-workspace-scoped-persistence
summary: >
  Collapse the workspace abstraction into project. Workspace was introduced
  (spec 087) as a multi-project governance container with a shared knowledge
  corpus, but the cross-project knowledge-sharing case has been displaced by
  the clone pipeline (specs 113, 114) which copies knowledge into the
  destination project at clone time. The workspace layer no longer earns its
  keep. Promote project to be the top-level unit under organization, owning
  the S3 bucket, knowledge corpus, source connectors, sync runs, runtime
  threading, and permission grants. Amend specs 087, 092, 094, 099 in place
  using the `amends:` and `amendment_record:` frontmatter fields formally
  introduced by this spec into the bootstrap (spec 000).
---

# 119 — Project as Unit of Governance

## 1. Problem Statement

Spec 087 introduced a four-tier hierarchy: `organization → workspace → project → repo`. Workspace was the unit of identity, governance, collaboration, knowledge intake, and factory execution; project was the unit of work within it. The workspace layer was justified by one capability: cross-project knowledge sharing — a single uploaded document bound to multiple projects via `document_bindings`.

Two changes since 087 have eroded that justification:

- **Spec 113/114 (project clone)** copies the knowledge corpus into the destination project at clone time. The "share once, consume everywhere" path has been replaced by "clone duplicates everything," which matches how product teams actually work.
- **Spec 115 (knowledge extraction)** scopes extraction policy and run rows to workspace, but in practice each workspace today contains exactly one project; the "policy applies to many projects in this workspace" path is unexercised.

The current shape costs:

- Two RBAC layers (`workspace_grants` + `project_members`) with overlapping concerns and no clear precedence.
- Every storage query traverses `project → workspace → bucket` instead of `project → bucket`.
- Specs 087/092/094/099 frame their invariants around "workspace" because that was the unit at the time. Future spec authors have to remember which layer to attach new entity scope to.
- The OPC desktop, stagecraft web, and audit log all surface "workspace" as a first-class concept that maps 1:1 to a project in every demo and dev environment to date.

Spec 087's substantive contributions (knowledge intake domain, duplex sync substrate, identity wiring) all stand. The hierarchy choice is what changes.

## 2. Decision

Collapse `workspace` into `project`:

- `organization → project → repo` becomes the canonical entity hierarchy.
- Project is the unit of: storage (S3 bucket), knowledge corpus, source connectors, sync runs, runtime threading, permission grants, factory adapter binding, environments, members.
- Repo remains a project-owned 1-to-many child (a project can hold multiple repos in different GitHub orgs, unchanged).
- The `workspaces` table is dropped. The `document_bindings` table is dropped (knowledge_objects gain a direct `project_id`).
- Permission grants merge: `workspace_grants` becomes `project_grants` and merges by precedence with `project_members` per §6.4.

Clean break (no compatibility shims, no deprecation window) — pre-alpha posture, no external users.

## 3. Affected Specs

Each amended spec receives a minimal in-place edit (frontmatter `amended:` + `amendment_record: "119"`, narrative s/workspace/project, code-alias rename) and a callout near the top:

> **Amended by spec 119 (2026-04-29):** the unit of governance described in this spec is now `project`, not `workspace`. See spec 119 for the migration record.

| Spec | Title (after) | Edit weight | Code alias change |
|---|---|---|---|
| 000 | Bootstrap Spec System | minor — formally defines `amends:` and `amendment_record:` | none |
| 087 | Unified **Project** Architecture | heavy — title + ~all framing | `UNIFIED_WORKSPACE` → `UNIFIED_PROJECT` |
| 092 | **Project** Runtime Threading | heavy — title + 40 `workspace_id` mentions | `WORKSPACE_THREADING` → `PROJECT_THREADING` |
| 094 | Unified Artifact Store with Provenance | light — 6 incidental mentions | none |
| 099 | **Project**-Scoped Persistence | heavy — title + 24 mentions | `WORKSPACE_SCOPED_PERSISTENCE` → `PROJECT_SCOPED_PERSISTENCE` |

Spec 091 (registry-enrichment) was screened and dropped — zero workspace mentions, false positive in the original audit.

Spec 115 (knowledge-extraction-pipeline) is **not** amended in place; its narrative was correct for the workspace era and remains operative for the era between merge and migration. After this spec lands, 115's references to `workspace_id` are read as `project_id` per §4.3, and the §10 API-key amendment (now in flight) is authored at project scope from the start.

## 4. Schema Diff

Driven from `platform/services/stagecraft/api/db/schema.ts` and the migrations directory.

### 4.1 Tables dropped

- `workspaces`
- `workspace_grants` (merged into `project_grants` per §6.4)
- `document_bindings` (knowledge_objects gain direct project ownership)

### 4.2 Columns moved to `projects`

| From | Column | To |
|---|---|---|
| `workspaces.object_store_bucket` | `text NOT NULL` | `projects.object_store_bucket` |

### 4.3 `workspace_id` column renames

The following tables rename `workspace_id` → `project_id`. Type stays `uuid` everywhere except `workspace_grants` which today is `text` and is replaced by the new `project_grants` table per §6.4.

| Table | Before | After |
|---|---|---|
| `knowledge_objects` | `workspace_id uuid NOT NULL` | `project_id uuid NOT NULL` |
| `source_connectors` | `workspace_id uuid NOT NULL` | `project_id uuid NOT NULL` |
| `sync_runs` | `workspace_id uuid NOT NULL` | `project_id uuid NOT NULL` |
| `knowledge_extraction_runs` | `workspace_id uuid NOT NULL` | `project_id uuid NOT NULL` |
| `clone_runs` (spec 114) | `workspace_id uuid NOT NULL` | `project_id uuid NOT NULL` |
| `agent_policies` | `workspace_id uuid NOT NULL` | `project_id uuid NOT NULL` |
| `audit_log` (where `target_type IN ('workspace', ...)`) | `target_type='workspace'` rows | `target_type='project'` post-migration; rows remain for audit completeness |

The full list is generated by the migration tooling at implementation time; the table above is the design intent, not the canonical inventory.

### 4.4 Constraint changes

- `projects` unique constraint: `unique(workspace_id, slug)` → `unique(org_id, slug)`. Slug namespace flattens to org.
- `knowledge_objects` unique CAS key: `(workspace_id, content_hash, extractor_version)` → `(project_id, content_hash, extractor_version)`.

### 4.5 Compiled-policy paths

- `build/policy/workspaces/{workspace_id}.json` → `build/policy/projects/{project_id}.json`. Spec 091/115 readers update to the new path.

## 5. Code-Alias Migration

| Old alias | New alias | Notes |
|---|---|---|
| `UNIFIED_WORKSPACE` | `UNIFIED_PROJECT` | spec 087 |
| `WORKSPACE_THREADING` | `PROJECT_THREADING` | spec 092 |
| `WORKSPACE_SCOPED_PERSISTENCE` | `PROJECT_SCOPED_PERSISTENCE` | spec 099 |

Featuregraph readers consume code aliases; the rename is a clean break per §2. Any downstream consumer that hard-codes the old alias must update in the same PR.

## 6. Migration Order and Invariants

The migration is a single Postgres migration paired with a code-and-spec PR. Pre-alpha posture allows a stop-the-world cut.

### 6.1 Pre-flight

- Snapshot the database (existing backup tooling).
- Assert no inflight `clone_runs` or `knowledge_extraction_runs` (status NOT IN ('queued', 'running')). If present, drain or fail.
- Assert every `project.workspace_id` resolves to an existing workspace (referential integrity precondition).

### 6.2 Backfill (per-workspace, may be skipped if zero rows)

1. **Bucket promotion.** Add `projects.object_store_bucket text` (nullable). Backfill `projects.object_store_bucket = workspaces.object_store_bucket` via the `workspace_id` join. Mark NOT NULL.
2. **Knowledge object project ownership.** Add `knowledge_objects.project_id uuid` (nullable).
   - For each `knowledge_object`, compute its bound projects via `document_bindings`.
     - **One binding** → set `project_id` to that project. Common case.
     - **Multiple bindings** → policy: pick the most recent binding by `bound_at`, log a `migration_warning` row. (Pre-alpha: confirmed to be empty in dev DBs by inspection during pre-flight.)
     - **Zero bindings** → orphan; either delete or assign to the workspace's first project. Decision deferred to migration tooling time when dev-DB inventory is taken; spec invariant: zero orphans remain after migration.
   - Mark `project_id` NOT NULL.
3. **Connector/sync/extraction-run/clone-run/policy projection.** For tables that today hold `workspace_id`, add `project_id`, backfill via the workspace's first/only project (asserted single during pre-flight), drop `workspace_id`.

### 6.3 Cutover

4. Drop `document_bindings`.
5. Drop `workspaces`.
6. Drop `projects.workspace_id`.
7. Apply the new uniqueness constraints (§4.4).
8. Update `audit_log` index for `target_type='project'` queries.

### 6.4 Permission-grant merge

Today: `workspace_grants` (per-user, workspace-scoped, governs file/network/max_tier) and `project_members` (per-user, project-scoped, role enum: viewer/developer/deployer/admin) coexist with overlapping but distinct semantics.

After: a single `project_grants` table replaces `workspace_grants` with `workspace_id` renamed to `project_id`. `project_members` is unchanged. Precedence:

- `project_members.role` controls coarse access (read/write/deploy/admin).
- `project_grants` (existing fields: enable_file_read/write/network, max_tier) governs runtime tool permissions for OPC governance.
- A user MAY have a `project_members` row without a `project_grants` row (default tool permissions apply); a `project_grants` row without `project_members` is a configuration error and rejected at write time.

Migration: rename `workspace_grants` → `project_grants`, rename `workspace_id` → `project_id`, type-fix `text` → `uuid` (existing column type is `text`; backfill via lookup against `workspaces.id::text → projects.id` for projects under that workspace, expand to one row per project).

### 6.5 Invariants

- I-1: At any point in time during migration, exactly one of `(workspace_id, project_id)` is populated on a moving table; never both, never neither.
- I-2: No knowledge_object exists post-cutover without a `project_id`. (No orphans.)
- I-3: No code path post-cutover may reference the symbol `workspace`, `workspace_id`, or `workspaceId` outside of (a) the changelog and migration scripts, (b) Cargo/pnpm-workspace senses (build tooling, not the entity), (c) the migration log records that retain the historical `target_type='workspace'` audit rows.
- I-4: No spec under `specs/**` may reference the OAP workspace entity post-merge except (a) this spec, (b) the four amended specs' "Amended by 119" callouts, (c) historical mentions in superseded specs (which remain frozen).

## 7. Frontmatter Convention Introduced

This spec formally introduces two frontmatter fields, with the canonical definitions landed in spec 000 as part of this spec's amendment.

### 7.1 `amends`

```yaml
amends:
  - "000"
  - "087"
```

A list of spec ids that this spec amends in place (i.e. modifies their narrative or invariants without superseding them). Each listed spec MUST receive corresponding frontmatter `amended: <date>` and `amendment_record: <this-spec-id>` fields, and a callout in its body pointing at the amender.

`amends:` is mutually exclusive with `supersedes:` (proposed for a future amendment if needed; not introduced here). A spec that fully replaces another uses `superseded_by:` on the old spec, not `amends:`.

### 7.2 `amendment_record`

```yaml
amendment_record: "119"
```

Set on amended specs (087/092/094/099 in this case). Points to the spec that records the amendment's rationale, schema diff, and migration plan. The amended spec stays operative on the parts not covered by the amendment record; the amendment record is consulted for the parts that are.

### 7.3 Compiler treatment

The spec compiler (tools/spec-compiler) recognises both fields without treating them as semantic gates: amendments do not change the amended spec's `status` or `implementation` fields. The compiler emits both fields into the registry verbatim. Spec lint MAY warn on dangling references (e.g. `amends: ["087"]` without 087 carrying matching `amendment_record:`), but this is non-blocking in the introductory release.

## 8. Out of Scope

- `teams` / `groups` layer between organization and project — explicitly deferred. If shared governance across multiple projects becomes a real need post-collapse, a follow-up spec re-introduces a grouping layer with the lessons learned.
- Cross-project knowledge sharing as a first-class primitive — explicitly removed. Cross-project knowledge use happens via clone (which copies) or, if needed later, an explicit "import knowledge from project X" action that creates a fresh `knowledge_object` row in the destination project.
- Renaming the user-facing UI label. The codename collapse is from `workspace` to `project`; any further UX naming review (e.g. "should it be called engagement, workbench, lab") is deferred to a UX spec post-collapse.

## 9. Acceptance Criteria

A-1. Spec 000 carries the `amends:` and `amendment_record:` field definitions in its body and is itself amended (frontmatter `amended:` set, `amendment_record: "119"`).

A-2. `.specify/contract.md` carries one new bullet describing `amends:` / `amendment_record:` as the in-place amendment convention.

A-3. Specs 087, 092, 094, 099 each carry `amended: "2026-04-29"`, `amendment_record: "119"`, an "Amended by 119" callout in the body, narrative updates s/workspace/project, and (for 087/092/099) updated titles and code aliases.

A-4. The Postgres migration applies cleanly against a representative dev database, satisfying invariants I-1..I-3 of §6.5, with the migration warning log empty.

A-5. `grep -r "workspace_id\|workspaceId" platform/services/stagecraft/api crates apps/desktop` returns zero hits outside (a) historical migration files, (b) the migration script for this spec, (c) frozen superseded specs, (d) Cargo/pnpm-workspace mentions.

A-6. The compiled spec registry (`build/spec-registry/registry.json`) carries `amends`/`amendment_record` fields on this spec and the four amended specs, with no schema-validation errors.

A-7. The codebase index (`build/codebase-index/index.json`) re-renders cleanly after schema changes; spec-to-code traceability for the four amended specs continues to resolve to active code.

A-8. `make ci` passes on the post-migration branch.

## 10. Open Questions

OQ-1. **Orphan-knowledge-object policy.** §6.2.2 leaves the zero-binding case ("delete or assign to first project") unresolved pending dev-DB inventory at migration time. The chosen rule is recorded in the migration changelog.

OQ-2. **Audit-log retention.** §4.3 keeps `target_type='workspace'` historical rows as-is. Confirm that downstream audit consumers tolerate the mixed `workspace`/`project` namespace, or migrate the historical rows in a separate non-blocking pass.

OQ-3. **`amends:` lint posture.** §7.3 leaves spec-lint enforcement non-blocking for the introductory release. Promote to blocking once the convention is exercised across at least one further amendment.

OQ-4. **OPC desktop URL paths.** Today: `/workspaces/{id}/projects/{id}`. After: `/projects/{id}`. Confirm no external bookmarks rely on the old path (pre-alpha posture suggests no, but worth a quick scan of any committed docs/screenshots).

## 11. References

- spec 087 (`unified-workspace-architecture`) — origin of the workspace concept.
- spec 113 (`stagecraft-projects-rename-and-clone`) — clone-as-knowledge-copy primitive.
- spec 114 (`async-project-clone-pipeline`) — async clone runs.
- spec 115 (`knowledge-extraction-pipeline`) — extraction worker, currently workspace-scoped.
- platform/CLAUDE.md — current schema documentation.
- platform/services/stagecraft/api/db/schema.ts — source of truth for the schema diff.
