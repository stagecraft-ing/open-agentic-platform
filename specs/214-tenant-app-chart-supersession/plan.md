# Implementation Plan: 214-tenant-app-chart-supersession

> Companion to `spec.md`. Resolves the spec's plan-time Clarifications
> against the 2026-06-15 code-reality survey (explorer + encore-expert
> agents) and records the staging, file-by-file work, FR corrections, and
> verification split. Second deploy-path spec; independent of 213,
> prerequisite of 215. This is the heavy one: new chart, deployd-api
> contract changes, the destructive tenant-hello retirement, and 136/137
> amendments.

## Clarification resolutions

### Clarification 1: health endpoint (RESOLVED, FR-001 correction)

The template-encore scaffold serves `/health`, `/health/liveness`,
`/health/readiness` (`template-encore/apps/api/health/api.ts:18,31,39`).
It does **not** serve `/healthz`. `/healthz` exists in OAP only because
stagecraft declares it explicitly (`deploy.ts:356`). So FR-001's
"probe path defaults to `/healthz`" is corrected: the chart defaults to
`/health/liveness` (liveness) and `/health/readiness` (readiness), both
chart values. (Decision B below records the alternative: mandate the
scaffold add `/healthz` to honour spec 136 C-002 literally.)

### Clarification 2: Encore self-hosted runtime config (RESOLVED, FR-003 correction)

Self-hosted Encore does **not** read a mounted config file at runtime.
`infra.config.json` is **baked into the image** at `encore build docker
--config` time; it declares static DB topology (host/name/user) and marks
sensitive values as `{ "$env": "VAR" }`. At container start the runtime
resolves those `$env` markers against plain process env vars. Production
reference: stagecraft itself (`infra.config.hetzner.json:106-133` bakes
the DB host + `POSTGRES_PASSWORD` as `$env`; `charts/stagecraft/templates/
deployment.yaml:77-79` injects them via `envFrom: secretRef`). No
`ENCORE_*` config var, no infra-config mount.

**FR-003 correction:** the chart supplies runtime config **as env vars**
(secretRef / per-key `valueFrom`), NOT a mounted Encore ConfigMap. FR-004
(`config_refs` -> `extraEnv`) is already the right shape. The DB
host/name/user are baked into the tenant image's `infra.config.json` at
spec-213 build time, so the preview Postgres (FR-006) needs a
**deterministic in-namespace service name** the baked host can target
(the chart pins the Postgres Service name; the tenant's infra.config bakes
that host). Whether Encore's infra schema allows `$env` on the `host`
field (which would let the chart inject the host) is unconfirmed locally;
the deterministic-service-name approach avoids needing it.

### Clarification 3: `{base}` domain (to confirm at implementation)

The wildcard cert is `*.tenants.${DOMAIN}` /
`tenants.${DOMAIN}` (`infra/hetzner/manifests/
tenants-wildcard-certificate.yaml`), a **single** wildcard label.
`hostname.ts` reads the apex from a deploy-service env var (name to be
confirmed in implementation; candidate alongside `RAUTHY_ISSUER_URL` in
`deploy.ts`). The single-label wildcard is the binding constraint that
forces FR-007's single-label host.

### Clarification 4: fixture materialisation mode (RESOLVED 2026-06-15)

**CI-time SHA-pinned fetch.** `ci-tenant-app.yml` checks out template-encore
at the SHA `templateCache.ts` pins and materialises the profile at CI time,
so the fixture cannot drift from the shipped template (directly delivers the
SC-006 lockstep property). Cross-repo checkout uses a read token; the
template-encore `uses:`/ref is SHA-pinned per spec 158. Committed-snapshot
was rejected: it can go stale without a human regen discipline.

### Decision B: probe path (RESOLVED 2026-06-15)

Chart probe defaults are `/health/liveness` + `/health/readiness` (what the
scaffold serves). Spec 136 C-002's `/healthz` naming is reconciled by
folding a health-path clarification into the FR-012 amendment of spec 136
(Stage 2), rather than mandating a cross-repo scaffold change.

### Decision C: execution (RESOLVED 2026-06-15)

Stage 1 (additive) lands and is locally verified first; checkpoint for
review before the destructive Stage 2 (tenant-hello retirement + 136/137
amendments).

## FR-007 hostname (214 amends spec 137)

Spec 137 §12's earlier sketch flattened to `<env>-<project>-<org>` with
single hyphens. Spec 214 FR-007 is the authority and supersedes it:
`{orgSlug}--{projectSlug}--{envSlug}` (double-hyphen separators so
single-hyphen slugs stay unambiguous), `--int` appended for the internal
variant, single label under `tenants.{base}`, 63-char truncate-plus-hash.
Implemented in `hostname.ts` with a property test (SC-004). Landing it
amends spec 137 (clarification): `amended:` frontmatter + callout on 137.

## Code-reality deltas the survey surfaced (beyond the spec)

1. **`selectChart` is not wired into `deploy.ts`**, and `chart` /
   `chart_version` are never forwarded to deployd (`deploy.ts:305-317`);
   deployd defaults to `tenant-hello` (`routes.rs:119`). FR-002 therefore
   needs deploy.ts plumbing (derive shape from `factoryAdapterId`, forward
   `chart`), not just a registry entry.
2. **`config_refs` is validated then silently dropped** (no field on
   deployd's `DeploymentRequest`). FR-004 adds the field + `extraEnv`
   rendering through `routes.rs`/`helm.rs`.
3. **TLS is never activated**: `helm.rs::build_values` never sets
   `ingress.tls.enabled`. The new chart + build_values must wire the
   `tenants-wildcard-tls` secret for User Story 1 (serves over TLS).
4. **Namespace** isn't persisted; deployd recomputes `{app_id}-{env_id}`
   (`routes.rs:175`), ignoring `environments.k8sNamespace`. FR-008 adds
   `namespace` forwarding + a `store.rs` column.
5. **Gate seam location**: the `oauth2-proxy-gate` chart is embedded
   separately in `helm.rs` (region `gate-overlay`, lines 51-68) and
   installed alongside via `install_with_gate`; the **tenant chart's
   Ingress** renders the `auth-url`/`auth-signin` annotations when
   `gate.enabled` (`tenant-hello/templates/ingress.yaml:8-23`). That block
   is the FR-011 render-parity surface to port verbatim into
   `aim-vue-encore`.

## Staging (FR-011 mandates additive-first, prove parity, then retire)

**Stage 1 (additive, non-destructive):**
- `platform/charts/aim-vue-encore/` chart (FR-001/003/006): Deployment
  (port 4000, non-root, RO-rootfs + writable /tmp, probes
  `/health/liveness` + `/health/readiness`, resources), Service, per-release
  ServiceAccount, Ingress with the spec-137 gate annotation block ported
  verbatim + TLS wired, optional preview Postgres StatefulSet+Service+Secret
  (deterministic service name), `extraEnv` from `config_refs`,
  `imagePullSecrets`.
- `helm.rs`: embed the new chart (`include_str!`) + `write_chart` arm;
  `build_values` sets `ingress.tls.*`, `extraEnv`, `imagePullSecretName`,
  forwarded `namespace`.
- `routes.rs`: add `config_refs`, `image_pull_secret_name`, `namespace`
  to `DeploymentRequest`; use forwarded namespace, persist it.
- `store.rs`: `namespace` column + migration.
- `deploy.ts`: forward `chart`/`chart_version` (from `selectChart` keyed
  on `factoryAdapterId`), `config_refs` (reject reserved `ENCORE_`/
  `KUBERNETES_` prefixes), `image_pull_secret_name`, `namespace`; derive
  `desired_routes` from `hostname.ts` when caller supplies none.
- `chartSelector.ts`: ADD `aim-vue-encore` (keep `tenant-hello` for now);
  update test.
- `hostname.ts` + `hostname.test.ts` (FR-007, property test SC-004).
- `ci-tenant-app.yml` (helm lint + template renders, fixture build).
- **FR-011 render-parity check**: `helm template` tenant-hello vs
  aim-vue-encore against the spec-137 gate fixture values; assert
  equivalent gate-relevant objects.
- `image_pull_secret` reflector: a `ghcr-pull` dockerconfigjson secret
  replicated like the wildcard TLS secret (manifest under
  `infra/hetzner/manifests/`).

**Stage 2 (destructive, after Stage 1 parity is green):**
- Delete `platform/services/tenant-hello/`, `platform/charts/tenant-hello/`,
  `ci-tenant-hello.yml`, `cd-tenant-hello.yml`; remove the tenant-hello
  `include_str!` block + `write_chart` arm from `helm.rs`; drop
  `tenant-hello` from `CHART_REGISTRY` (sole shape becomes `aim-vue-encore`).
- `cd-tenant-app.yml` (build+push the reference image).
- Amend spec 136 (supersession callout + FR-012 statelessness refinement)
  and spec 137 (gate-anchor `co_authority` units move from tenant-hello
  chart files to aim-vue-encore; hostname clarification). Both via the
  amendment convention (`amended:` + callout) in the same PR.
- Supersession frontmatter on 214 already declares the partial
  supersedes of 136's directories/workflows.

## Stage 1 progress (2026-06-15, session handoff)

**DONE + locally verified:**

- `api/deploy/hostname.ts` + `hostname.test.ts` (FR-007): 8 tests incl. the
  SC-004 100-triple property. Green.
- `api/deploy/chartSelector.ts` + test: `aim-vue-encore` added to the union +
  registry (tenant-hello kept for Stage 1); `listShapes` test updated. Green.
- `platform/charts/aim-vue-encore/` (8 files, FR-001/003/006): `helm lint`
  clean; renders clean at default / gate+ingress+tls / preview-db. **FR-011
  gate-seam render-parity vs tenant-hello PROVEN** (4 gate-relevant lines
  byte-identical: 3 `auth-*` annotations + TLS `secretName`).

**PENDING (resume here):**

- `helm.rs`: `include_str!` embed the 8 chart files + `write_chart` arm +
  `build_values` (set `ingress.tls.*`, `extraEnv`, `imagePullSecretName`,
  forwarded `namespace`). The TLS wiring is net-new (build_values never set
  `ingress.tls.*` before, per the survey).
- `routes.rs`/`store.rs`: add `config_refs`, `image_pull_secret_name`
  (default `ghcr-pull`), `namespace` to `DeploymentRequest`; use + persist
  the forwarded namespace + migration. `cargo check` + `cargo test`.
- `deploy.ts`: forward `chart`/`chart_version` from `selectChart` (keyed on
  `factoryAdapterId`); reject reserved `ENCORE_`/`KUBERNETES_` `config_refs`
  prefixes; `image_pull_secret_name`; `namespace`; derive `desired_routes`
  from `hostname.ts` when the caller omits them. Confirm the `{base}` domain
  env var name in `deploy.ts` (Clarification 3).
- `ci-tenant-app.yml` (CI-time SHA-pinned template-encore fetch + `helm lint`
  + template renders + the FR-011 parity check) and a `ghcr-pull` reflector
  secret manifest under `infra/hetzner/manifests/`.
- Spec 214 frontmatter: convert `references: planned-establishes` to
  `establishes:` for `platform/charts/aim-vue-encore` (directory),
  `hostname.ts`/`.test.ts`, `ci-tenant-app.yml`; run coupling / spec-lint /
  featuregraph golden / codebase-index gates.

Then **checkpoint before Stage 2** (destructive: delete tenant-hello, flip the
registry to sole-shape, amend 136/137).

**Branch note:** all 214 Stage 1 changes are currently on
`feat/213-tenant-repo-image-build` (uncommitted), intermingled with the
completed spec 213 work. On resume, decide whether to commit 213 first and
split 214 onto `feat/214-tenant-app-chart-supersession`.

## Verification split

**Locally verifiable:** `helm lint` + `helm template` (default + gate),
FR-011 render-parity diff, `hostname.test.ts` property test (100 random
triples, SC-004), `chartSelector.test.ts`, stagecraft `tsc`, `cargo check`
+ `cargo test` on deployd-api-rs, coupling gate, spec-lint, featuregraph
golden.

**Deploy-time only (NOT here):** SC-001 live helm release + TLS endpoint,
SC-003 DB-backed write round-trip in a preview env, SC-005 private-image
pull in a fresh namespace, gate auth flow against live Rauthy.

## Out of this PR (per spec §Out of scope)

Production-grade DB provisioning (HA/backups), the deploy trigger + UI
(215), tenant-repo image production (213), per-adapter chart generation,
multi-cluster/non-nginx/non-GHCR, migrating historical tenant-hello
deployment records.
