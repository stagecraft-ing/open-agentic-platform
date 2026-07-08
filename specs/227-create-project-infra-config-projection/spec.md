---
id: "227-create-project-infra-config-projection"
title: "Create Project as an Encore-infra-config projection: derived catalog, two-axis selector, dev-provisioned Redis"
feature_branch: "227-create-project-infra-config-projection"
status: draft
implementation: pending  # Design spec. No code lands in this PR; the contract is captured so the follow-on implementation PRs (stagecraft catalog derivation + form reframe; deployd previewRedis) can promote the referenced paths to authoritative relationships and satisfy the coupling gate one subsystem at a time. This spec establishes no code path yet: its only in-PR code change is the featuregraph golden node (extends 034), matching the 214/222/223/224/225/226 new-spec precedent.
kind: platform
domain: platform
created: "2026-07-06"
authors: ["open-agentic-platform"]
language: en
summary: >
  The stagecraft "Create New Project" surface hand-mirrors the factory
  adapter's module catalog in two independently-maintained hardcoded copies
  (the frontend route and a backend module) that have drifted: they still
  offer a "Redis" module whose factory-encore source is an inert marker with a
  false label (it claims a Redis rate-limit backend that does not exist; the
  baseline limiter is Postgres). The surface also conflates three different
  kinds of thing under one checkbox UI (inert markers, base-app config knobs,
  and the one real feature module) and fuses factory-encore's two orthogonal
  axes (auth profile x topology) into a single "Variant" selector that silently
  drops the default profile. This spec reframes the surface as a projection of
  vocabulary that already exists: feature modules derived from the adapter
  manifest, and infrastructure resources projected from the app's committed
  apps/api/infra.config.json (Encore's infra.schema.json). It splits the form
  into a two-axis selector (auth {public, internal}; topology {single, dual}),
  an Infrastructure section (read-only projection of the baked infra.config
  topology, Postgres default-on, Redis opt-in), an Application-features section
  (real modules only), and Base-app config fields (CORS, BFF knobs). Under
  Option A (topology parity), the infra.config.json is the app-level,
  build-time, read-only topology; per-environment hosts/credentials/secrets are
  runtime env resolved via GitHub Environments (dev owned by OAP/deployd,
  staging/prod by the tenant). It adds a deployd previewRedis dev-only
  provisioning path mirroring previewDatabase so Redis becomes a real
  provisionable resource. Companion to factory-encore spec 008 (data-redis
  promotion, dual module composition, CORS knob), which owns the adapter-side
  changes this surface projects.
code_aliases: ["previewRedis", "MODULE_CATALOG", "ENCORE_INFRA_CONFIG"]
depends_on:
  - "138-stagecraft-create-realised-scaffold"  # owns the create/scaffold flow (create.ts, scaffoldFromPrebuilt, pickProfileFromModules) and the moduleCatalog this spec derives instead of hand-mirrors
  - "199-factory-thin-consumer-sync"  # owns the stagecraft thin-consumer mirror of the factory manifest; the hand-mirrored MODULE_CATALOG is exactly the drift-prone copy this spec replaces with a derived catalog
  - "160-factory-adapter-stagecraft-relocation"  # owns the stagecraft-resident adapter surface (adapter-scopes.json) the derived infra/feature vocabulary reads from
  - "213-tenant-repo-image-build"  # owns the seeded oap-build.yml and `encore build docker --config ./infra.config.json`; Option A rides on this single build path
  - "214-tenant-app-chart-supersession"  # owns the acme-vue-encore chart and the previewDatabase dev-provisioning path previewRedis mirrors
  - "225-deployd-selfprovision-rbac"  # owns the current deployd provisioning/RBAC surface previewRedis extends
establishes:
  # Stage 3 (deployd previewRedis) landed this net-new chart template: the
  # dev-only preview-Redis workload (Deployment + Service + generated-password
  # Secret), the redis.yaml mirror of 214's postgres.yaml. Promoted here from
  # the design PR's implied surface as the code landed (spec 227 §6 staging;
  # the 214 `references:planned-establishes -> establishes` precedent).
  - unit: { kind: file, path: platform/charts/acme-vue-encore/templates/redis.yaml }
  # Stage 1 (catalog derivation) landed this net-new endpoint: the org-scoped
  # module-catalog loader + GET /api/factory/module-catalog that projects the
  # feature-module list from the adapter's substrate module manifests, replacing
  # the two hand-mirrored MODULE_CATALOG copies (FR-001).
  - unit: { kind: file, path: platform/services/stagecraft/api/factory/moduleCatalog.ts }
extends:
  # A new spec adds a node to the featuregraph golden (same precedent as specs
  # 214, 222, 223, 224, 225, 226); claimed additively against spec 034 so the
  # golden diff carries a 227 authority.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
  # Stage 3 (deployd previewRedis): promoted from references: as the
  # implementation landed. previewRedis mirrors 214's previewDatabase machinery
  # (helm.rs DeployExtras/build_values, routes.rs wire) and adds to 214's
  # acme-vue-encore chart the Redis provisioning path plus the previewDatabase
  # SQL_* env correction (the app resolves ${SQL_HOST}/${SQL_USERNAME}/
  # $env:SQL_PASSWORD, not the POSTGRES_* names the chart previously injected).
  # Additive against 214.
  - spec: "214-tenant-app-chart-supersession"
    nature: additive
    unit: { kind: file, path: platform/services/deployd-api-rs/src/helm.rs }
  - spec: "214-tenant-app-chart-supersession"
    nature: additive
    unit: { kind: file, path: platform/services/deployd-api-rs/src/routes.rs }
  - spec: "214-tenant-app-chart-supersession"
    nature: additive
    unit: { kind: file, path: platform/charts/acme-vue-encore/templates/deployment.yaml }
  - spec: "214-tenant-app-chart-supersession"
    nature: additive
    unit: { kind: file, path: platform/charts/acme-vue-encore/values.yaml }
  - spec: "214-tenant-app-chart-supersession"
    nature: additive
    unit: { kind: file, path: platform/charts/acme-vue-encore/templates/_helpers.tpl }
  - spec: "214-tenant-app-chart-supersession"
    nature: additive
    unit: { kind: file, path: platform/charts/acme-vue-encore/templates/networkpolicy.yaml }
  # Stage 1 (catalog derivation): promoted from references: as the
  # implementation landed. The create-project feature-module catalog is now
  # derived at request time from the adapter's substrate module manifests
  # instead of hand-mirrored in two copies (FR-001, SC-001). Additive against
  # 138 (the create/scaffold surface these paths belong to), 199 (the
  # thin-consumer factory browser whose admission gate the endpoint reuses), and
  # 112 (which owns the scaffold test files).
  - spec: "138-stagecraft-create-realised-scaffold"
    nature: additive
    unit: { kind: file, path: platform/services/stagecraft/api/projects/scaffold/moduleCatalog.ts }
  - spec: "138-stagecraft-create-realised-scaffold"
    nature: additive
    unit: { kind: file, path: platform/services/stagecraft/api/projects/create.ts }
  - spec: "138-stagecraft-create-realised-scaffold"
    nature: additive
    unit: { kind: file, path: platform/services/stagecraft/api/projects/scaffold/perRequestScaffold.ts }
  - spec: "138-stagecraft-create-realised-scaffold"
    nature: additive
    unit: { kind: file, path: platform/services/stagecraft/web/app/routes/app.projects.new.tsx }
  - spec: "138-stagecraft-create-realised-scaffold"
    nature: additive
    unit: { kind: file, path: platform/services/stagecraft/web/app/lib/projects-api.server.ts }
  - spec: "199-factory-thin-consumer-sync"
    nature: additive
    unit: { kind: file, path: platform/services/stagecraft/api/factory/browse.ts }
  - spec: "112-factory-project-lifecycle"
    nature: additive
    unit: { kind: file, path: platform/services/stagecraft/api/projects/scaffold/scaffold.test.ts }
  - spec: "112-factory-project-lifecycle"
    nature: additive
    unit: { kind: file, path: platform/services/stagecraft/api/projects/scaffold/perRequestScaffold.test.ts }
references:
  # Non-authoritative pointers to the code paths later stages govern. The
  # Stage 3 deployd/chart paths and the Stage 1 catalog-derivation paths were
  # promoted to establishes/extends above as that code landed; the pointer below
  # remains un-promoted until its own stage lands (claiming it now would
  # over-fire the coupling gate). The Stage 2 form reframe promotes the
  # dev-provisioning trigger when it wires the opt-in Redis selection.
  - role: dev-provisioning-trigger
    unit: { kind: file, path: platform/services/stagecraft/api/deploy/deploy.ts }
---

# 227. Create Project as an Encore-infra-config projection

> Provenance: this contract formalizes the agreed direction in
> [`docs/analysis/create-project-encore-config-reframe.md`](../../docs/analysis/create-project-encore-config-reframe.md).
> Companion: factory-encore spec `008-data-redis-promotion-dual-composition`
> owns the adapter-side changes (data-redis promotion, the dual
> module-composition gap, the CORS knob decision) this surface projects.

## 1. Purpose

The "Create New Project" surface is the front door of the factory: it is where
an operator turns a factory adapter + variant + module selection into a
scaffolded tenant repo, a seeded pipeline state, and a first deploy. Today that
surface is enigmatic for three mechanical reasons, and it hides the one
artifact that makes the produced app portable.

### 1.1 The catalog is hand-mirrored and has drifted

The module catalog (list, group labels, prose descriptions) is hardcoded inline
in two independently-maintained copies:

- frontend `platform/services/stagecraft/web/app/routes/app.projects.new.tsx`
  (the `MODULE_CATALOG` const)
- backend `platform/services/stagecraft/api/projects/scaffold/moduleCatalog.ts`

Nothing reads the adapter manifest at runtime, so the prose rots. It has: the
two copies already diverge in wording, and both still offer a **Redis** module.
In factory-encore that module is an inert marker (`files: {}`, `status: stable`)
whose description claims a Redis rate-limit backend that does not exist. The
baseline rate limiter is a Postgres `UNLOGGED` counter, and no baseline code
reads `REDIS_URL`. So the Redis checkbox scaffolds a real-but-dead module with a
false label. This is the drift-prone hand-copy the spine exists to eliminate
(constitution Principle II).

### 1.2 One checkbox UI conflates three kinds of thing

The "Modules" list presents, at identical visual weight:

- **Inert markers** (`security-core`, `data-postgres`) that ship nothing;
  toggling them is nearly a no-op. `security-core`'s only output is a
  `CORS_ORIGIN` env var that no baseline code reads (CORS is hardcoded static in
  the baseline `encore.app`).
- **A base-app config knob plus a diagnostic page** (`api-gateway`): the BFF
  proxy backend always ships in the baseline; the module contributes real,
  consumed env knobs (`PRIVATE_API_BASE_URL`, `GATEWAY_TIMEOUT_MS`) and one Vue
  `/connectivity` test page.
- **One real feature module** (`user-management`): a new Encore service,
  migration, admin CRUD, and `requireRole`.

Presenting a no-op marker and a real service as the same control is why the
form reads as opaque, and why "opt-in" sits next to "ships in the base app" in
the same description.

### 1.3 Two axes are fused, and the default profile is dropped

factory-encore has two orthogonal axes: an **auth profile** (`minimal` /
`public` / `internal`, the auth-driver axis) and a **topology** (`single` /
`dual`, where dual is explicitly "a topology, not a profile"). The form fuses
them into one three-way "Variant" (`single-public` / `single-internal` /
`dual`) and silently omits `minimal`. At the OAP surface `minimal` (mock auth)
has no product home and is deliberately not offered, but the fusion also means
a user cannot express the axes independently.

### 1.4 The Encore infra config is already there, and invisible

Every generated app already commits `apps/api/infra.config.json`, an Encore
self-host artifact (`sql_servers` + `secrets`) built via
`encore build docker --config ./infra.config.json` in the seeded oap-build.yml
(spec 213). The portability spine (dockerized Encore app, deployable in OAP
dev, handed to any cloud) is already wired end to end. It is simply not
surfaced, and it currently exercises only 2 of Encore's resource types.

## 2. The reframe

Stop authoring a bespoke prose catalog. Make the create-project surface a
**projection** of vocabulary that already exists:

- **Feature modules** are derived from the factory adapter manifest (the single
  source of truth), not hand-mirrored. Drift like the retired-Redis label
  becomes impossible by construction.
- **Infrastructure resources** are projected from Encore's `infra.schema.json`
  vocabulary and the app's committed `apps/api/infra.config.json` topology.

The surface splits into four honest regions:

```
Topology     ( ) Single      ( ) Dual
Auth         ( ) Public      ( ) Internal          [minimal not offered at OAP surface]

INFRASTRUCTURE  (read-only projection of the app's baked infra.config topology)
  [x] PostgreSQL     default-on, read-only          -> sql_servers  (OAP-provisioned in dev)
  [ ] Redis / cache  opt-in                          -> redis        (OAP-provisioned in dev)
  ( object_storage, pubsub, metrics: same pattern, later )
  > view generated apps/api/infra.config.json (read-only)

APPLICATION FEATURES  (real code modules, derived from the adapter manifest)
  [x] User / Role Management   (auto for Internal; opt-in for Public)

BASE-APP CONFIG  (knobs, as fields not toggles)
  CORS origin: [___]        BFF private-backend URL: [___]   timeout: [___]
  [ ] /connectivity diagnostic page
```

### 2.1 Option A: topology parity (the environment model)

`encore build docker --config` bakes the infra config file into the image at
build time; only the `{"$env": "VAR"}` markers resolve at container start from
runtime env (confirmed by the OAP chart at
`platform/charts/acme-vue-encore/templates/deployment.yaml`: "Encore resolves
the `$env` markers in its baked infra.config.json at start; no mounted Encore
config"). One image therefore serves dev/staging/prod **only when the resource
topology is identical** across them.

This spec adopts **Option A (topology parity)**, which the baseline already
lives on:

- "Which resources my app uses" is a **build-time, app-level** property, fixed
  by the Infrastructure resources selected at scaffold. The `infra.config.json`
  file (topology) is therefore **read-only across all environments**.
- "Where those resources live" is a **runtime, environment-level** property
  (hosts, credentials, secrets), authored per environment via GitHub
  Environments: OAP owns dev's values (deployd injects them), the tenant owns
  staging/prod values in their GitHub Environment or target cloud. This follows
  the existing seeded oap-build.yml + baseline `deploy-{dev,staging,prod}.yml`
  pattern (spec 213).

No per-environment config files and no per-environment image builds are
introduced. Option B (per-environment topology divergence, N images per commit)
is explicitly out of scope; if a future need requires dev to stay leaner than
prod, it is a separate spec.

## 3. Requirements

### Functional Requirements

- **FR-001**: The create-project feature-module catalog MUST be **derived from
  the factory adapter manifest** (the stagecraft-resident adapter surface,
  spec 160) rather than hardcoded. The two hand-mirrored `MODULE_CATALOG`
  copies (`app.projects.new.tsx`, `moduleCatalog.ts`) MUST be eliminated so a
  drift like the retired-Redis label cannot recur.
- **FR-002**: The form MUST present two orthogonal selectors: an **auth
  profile** {`public`, `internal`} and a **topology** {`single`, `dual`}.
  `minimal` MUST NOT be offered at the OAP surface (it remains a valid
  factory-encore generator profile for the factory-e2e harness only).
- **FR-003**: The **Infrastructure** section MUST be a projection of Encore's
  `infra.config.json` resource vocabulary. PostgreSQL MUST be default-on and
  read-only. The generated `apps/api/infra.config.json` topology MUST be
  surfaced **read-only** in the form (viewable, not editable), because under
  Option A the topology is fixed by the selected resources and is identical
  across environments.
- **FR-004**: **Redis** MUST be an opt-in Infrastructure resource. Selecting it
  MUST add a `redis` block to the app's `infra.config.json` topology, paired
  with the factory-encore 008 promotion of `data-redis` from inert marker to a
  real resource. It MUST NOT be presented as (or scaffold) a rate-limit backend
  claim, because the baseline limiter is Postgres.
- **FR-005**: deployd MUST provision Redis in **development only**, via a
  `previewRedis` flag mirroring the existing `previewDatabase` path (a
  `DeployExtras` flag, a `redis.yaml` StatefulSet template, and injected
  `REDIS_*` runtime env matching the baked `redis` block), gated by the same
  `envKind === "development"` check. Non-development environments MUST supply an
  external Redis endpoint as runtime env; deployd MUST NOT provision Redis for
  them.
- **FR-006**: Under Option A, the `infra.config.json` (topology) MUST be
  read-only across environments; per-environment hosts, credentials, and
  secrets MUST be runtime env resolved via GitHub Environments (dev by
  OAP/deployd; staging/prod by the tenant), following the existing oap-build.yml
  + baseline `deploy-*.yml` pattern. No per-environment config files and no
  per-environment image builds may be introduced by this spec.
- **FR-007**: The inert base-app knobs MUST leave the "Modules" checkbox list.
  `api-gateway`'s consumed env knobs (`PRIVATE_API_BASE_URL`,
  `GATEWAY_TIMEOUT_MS`) MUST be presented as **Base-app config fields**; its
  `/connectivity` diagnostic page MUST be an opt-in dev aid. `security-core`'s
  `CORS_ORIGIN` MUST be presented as a config field **only if factory-encore 008
  wires it**; if 008 drops the knob, the CORS field MUST be omitted rather than
  offered as an inert control. Security posture (CSP/HSTS, rate-limit, logging)
  MUST be shown as always-on, not as a toggle.
- **FR-008**: The OAP surface MUST reflect the composition capability the
  adapter actually exposes for the chosen topology. Because the `dual` topology
  composes no feature modules today, the form MUST NOT offer feature-module
  selection for `dual` that the generator cannot honor. If factory-encore 008
  closes the dual module-composition gap, the surface MUST project the newly
  supported composition rather than assume it.

### Key Entities

- **Feature module** (derived): an adapter-manifest module that composes real
  code (today only `user-management`). Displayed in the Application-features
  section.
- **Infrastructure resource** (projected): an Encore `infra.config.json` resource
  block (`sql_servers`, `redis`, and future `object_storage` / `pubsub` /
  `metrics`). Postgres default-on; Redis opt-in.
- **Base-app knob**: a consumed env var over always-present baseline behavior
  (`PRIVATE_API_BASE_URL`, `GATEWAY_TIMEOUT_MS`, and `CORS_ORIGIN` if wired).
- **previewRedis**: the deployd dev-only Redis provisioning flag mirroring
  `previewDatabase`.

## 4. Success Criteria

- **SC-001**: No hardcoded feature-module catalog remains in stagecraft; the
  catalog matches the adapter manifest by construction. A regression like the
  retired-Redis false label is structurally impossible (there is no hand-copy to
  drift).
- **SC-002**: A project created with Redis selected deploys in OAP dev with a
  live Redis instance and a matching `redis` block in its
  `apps/api/infra.config.json`.
- **SC-003**: The same built image deploys to dev and (given external endpoints
  supplied as runtime env) to staging/prod, with only runtime env differing and
  no rebuild for a topology change.
- **SC-004**: The create-project form presents auth and topology as independent
  selectors, offers only `public`/`internal` auth, and shows each option under a
  section whose meaning (infrastructure / feature / base-app knob) is
  unambiguous.

## 5. Relationships and coupling posture

This is a **design-only** contract PR. It establishes no code path. The paths it
governs are listed under `references:` and are promoted to authoritative
`establishes`/`extends`/`refines` edges by the follow-on implementation PRs, so
the coupling gate is satisfied one subsystem at a time (the 214/222/223/224/225/226
new-spec precedent). The only in-PR code change is the featuregraph golden node,
claimed additively against spec 034.

`depends_on` edges cite the specs that own the surfaces this spec reframes:
138 (create/scaffold + moduleCatalog), 199 (the thin-consumer mirror the
derived catalog replaces), 160 (the stagecraft-resident adapter surface),
213 (the single build path Option A rides on), 214 (the chart + previewDatabase
that previewRedis mirrors), and 225 (the deployd provisioning surface
previewRedis extends).

## 6. Implementation staging

Each stage is a separate PR that promotes the relevant `references:` paths to
authoritative relationships:

1. **Catalog derivation** (stagecraft): replace both `MODULE_CATALOG` copies
   with a derivation from the adapter manifest; refines 138/199. Satisfies
   FR-001 and removes the false Redis label at the source.
2. **Form reframe** (stagecraft): the two-axis selector, the Infrastructure
   projection (read-only infra.config view), the Application-features section,
   and Base-app config fields; refines 138. Satisfies FR-002, FR-003, FR-007,
   FR-008.
3. **deployd previewRedis** (deployd + chart): the `previewRedis` flag, the
   `redis.yaml` StatefulSet, and the `REDIS_*` injection; extends 214/225.
   Satisfies FR-005.
4. **Redis Infrastructure resource end to end**: wiring the opt-in Redis
   selection through to the baked `redis` block; depends on factory-encore 008
   landing the adapter-side promotion. Satisfies FR-004, and FR-006 falls out of
   Option A with no per-env files.

### Implementation log

**Stage 3 landed (2026-07-07).** deployd + chart preview-Redis provisioning:

- deployd `DeployExtras.preview_redis` + `build_values` `previewRedis.enabled`
  branch (`helm.rs`), the `preview_redis: Option<bool>` request wire field
  (`routes.rs`), the embedded `templates/redis.yaml` register, and unit +
  `helm template` render tests. Mirrors the `preview_database` path 1:1.
- Chart: net-new `templates/redis.yaml` (single-replica, no-persistence Redis
  Deployment + Service + generated-password Secret, gated on
  `previewRedis.enabled`); `previewRedis` values block; `redisName` helper;
  `REDIS_HOST`/`REDIS_PASSWORD` app-container env; NetworkPolicy egress 6379.
- **previewDatabase env correction (rode along).** A verification pass found
  the existing preview Postgres was unreachable by the app: the chart injected
  `POSTGRES_HOST`/`POSTGRES_USER`/`POSTGRES_PASSWORD`, but the generated app's
  baked `infra.config.json` resolves `${SQL_HOST}` (host:port), `${SQL_USERNAME}`,
  and `$env:SQL_PASSWORD`. The app container now receives the `SQL_*` names
  (the postgres pod/Secret keep the `POSTGRES_*` keys the image itself needs).
  Redis follows the corrected convention (`REDIS_HOST`/`REDIS_PASSWORD`).

The stagecraft trigger (`deploy.ts`) that sets `preview_redis` from a project's
opt-in Redis selection, gated by `envKind`, lands with the Stage 2 form reframe
and the Stage 4 end-to-end wiring (it has no honest opt-in source until the
Infrastructure section exists), so `deploy.ts` stays under `references:`.

**Stage 1 landed (2026-07-07).** Catalog derivation (FR-001, SC-001):

- The two hand-mirrored `MODULE_CATALOG` copies are gone. `api/projects/scaffold/moduleCatalog.ts`
  is now pure: `deriveModuleCatalog(rows)` maps the adapter's module
  `manifest.json` bodies to descriptors, `deriveInstallOrder` topo-sorts over
  `requires`, and `isKnownModule`/`extrasFor` take the derived catalog.
  `id`/`description`/`requires`/`conflicts`/`status` come from the manifest; a
  thin transitional `PRESENTATION` overlay supplies `displayName`/`category`
  (no upstream source), which Stage 2's re-sectioning reworks.
- New `api/factory/moduleCatalog.ts`: `loadModuleCatalogForOrg` + the
  admission-gated `GET /api/factory/module-catalog`, reusing `browse.ts`'s
  `loadOrgView`/`servableRows` (single admission gate). Module manifests land in
  the catch-all `reference-data` kind, so they are selected by path regex.
- `create.ts` loads the derived catalog per-org; the frontend route deletes its
  hand-copy const and consumes the endpoint via its loader (a rejected/empty
  fetch degrades to an empty picker, not a 500).
- Out of scope for Stage 1 (deferred to Stage 2): `PROFILE_MODULES`/`PRESETS`
  are left empty; the adapter manifest's `scaffold.profiles[].modules` declares
  per-profile defaults (`internal` ships `user-management`) that Stage 2's form
  reframe derives and surfaces as "auto for Internal". The runtime dedupe in
  `perRequestScaffold.readInstalledModules` keeps composition correct meanwhile,
  so behavior is unchanged. The false `data-redis` label is corrected by
  factory-encore 008 at the manifest source; Stage 1 removes the hand-copy so
  stagecraft cannot drift from it.

## 7. Out of scope

- Option B (per-environment topology divergence, per-environment builds).
- The adapter-side changes (data-redis promotion payload, dual
  module-composition, CORS knob wire-or-drop): owned by factory-encore spec 008.
- The additional Encore resource types (`object_storage`, `pubsub`, `metrics`):
  the Infrastructure section is designed to hold them, but they are separate
  future increments.
