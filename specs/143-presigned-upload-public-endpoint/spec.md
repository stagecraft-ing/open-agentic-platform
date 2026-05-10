---
id: "143-presigned-upload-public-endpoint"
slug: presigned-upload-public-endpoint
title: Presigned upload public endpoint — browser-reachable object store for direct uploads
status: draft
implementation: in-progress  # FR-001..006a + §4.4 + §4.7 green per §13 (historical) and validate/spec-143.sh CONTRACT (ongoing, post-FU-004); FU-001 Tier 1 closure landed 2026-05-09 (sweeper firing, FU-009/010/011-Finding-1 shipped); 2026-05-10 stability regression — stagecraft-api OOMKilled under FR-006 34-file batch load (§13 2026-05-10 ~01:34 UTC entry); FU-014 closed 2026-05-10 (listKnowledgeObjects asymmetric typing fix); FU-015 root-cause confirmed 2026-05-10 ~07:48 UTC — V8-heap-vs-cgroup distinction adds third fix leg (NODE_OPTIONS), Subscription literal-int maxConcurrency cap is the load-bearing leg (§13 2026-05-10 ~07:48 UTC entry); outstanding: FU-002, FU-003, FU-008, FU-011 Tier 2, FU-013 (cause #1 OOM confirmed; fix gates on FU-015), FU-015 (stagecraft-api OOM, mirrors FU-002 + V8-heap leg — next priority), FU-016 (mid-batch session-cookie loss → 401), FU-020 (optional load harness), FU-021 (conditional deployd-api retro check)
owner: bart
created: "2026-05-07"
kind: platform
risk: medium
depends_on:
  - "087"  # unified-workspace-architecture (knowledge intake design)
  - "115"  # knowledge-extraction-pipeline (downstream consumer of confirmed uploads)
amends:
  - "087"  # extends NF-002 with the browser-reachability requirement
  - "115"  # annotates FR-003 as load-bearing for spec 143 FR-010 race contract
code_aliases: ["PRESIGNED_UPLOAD_PUBLIC_ENDPOINT"]
implements:
  - path: platform/services/stagecraft/api/knowledge/storage.ts
  - path: platform/services/stagecraft/api/knowledge/storage.dualClient.test.ts
  - path: platform/services/stagecraft/api/knowledge/knowledge.ts
  - path: platform/services/stagecraft/api/knowledge/auditActions.ts
  - path: platform/services/stagecraft/api/knowledge/orphanSweeper.ts
  - path: platform/services/stagecraft/api/knowledge/orphanSweeper.integration.test.ts
  - path: platform/services/stagecraft/api/knowledge/scheduler.ts
  - path: platform/services/stagecraft/api/knowledge/uploadLimits.ts
  - path: platform/services/stagecraft/api/knowledge/uploadLimits.test.ts
  - path: platform/services/stagecraft/api/knowledge/requestUpload.integration.test.ts
  - path: platform/services/stagecraft/web/app/routes/app.project.$projectId.knowledge.tsx
  - path: platform/services/stagecraft/test/__mocks__/encore-config.ts
  - path: platform/services/stagecraft/vite.config.ts
  - path: platform/services/stagecraft/infra.config.hetzner.json
  - path: platform/services/stagecraft/infra.config.json
  - path: platform/infra/hetzner/setup.sh
  - path: platform/infra/hetzner/post-create.sh
  - path: platform/infra/hetzner/.env.example
  - path: platform/infra/hetzner/validate/spec-143.sh
  - path: platform/charts/stagecraft/values.yaml
  - path: platform/charts/stagecraft/values-hetzner.yaml
  - path: platform/charts/stagecraft/templates/deployment.yaml  # §12 L-003 — render imagePullPolicy from values; FU-015 — NODE_OPTIONS env from .Values.nodeOptions (§13 2026-05-10 ~07:48 UTC)
  - path: platform/charts/stagecraft/templates/cronjob-orphan-sweeper.yaml  # FR-010 self-hosted scheduler (FU-001 beat 4)
  - path: platform/charts/stagecraft/templates/external-secret-knowledge-sweeper.yaml  # FR-010 per-purpose-credential mount, ESO path (FU-001 beat 4)
  - path: platform/services/stagecraft/api/knowledge/extractionWorker.ts  # FU-015 — Subscription literal-int maxConcurrency cap (§13 2026-05-10 ~07:48 UTC). Primary owner is spec 115; spec 143 amends 115 (frontmatter `amends:`), so spec 133's amends-aware coupling gate accepts the touch — explicit `implements:` entry makes the relationship visible without amends-walking.
  - path: platform/services/stagecraft/test/spec143-fu015.config.test.ts  # FU-015 — three CI static assertions (Subscription maxConcurrency literal, chart memory limit ≥1Gi, chart NODE_OPTIONS sanity). §13 2026-05-10 ~07:48 UTC.
  # Note: platform/infra/terraform/envs/dev/core/{main,variables}.tf are owned
  # by spec 072 (multi-cloud-k8s-portability). FR-010 adds per-purpose
  # sweeper credential entries into 072's existing keyvault_secrets map —
  # an additive-only data-shape change, not a multi-cloud-portability
  # design amendment. The PR carries a Spec-Drift-Waiver per spec 127
  # FR-005 instead of pulling those paths into 143's implements: (which
  # would still require a 072 amendment under spec 130's primary-owner
  # heuristic, and the heuristic correctly assigns primary ownership to
  # 072 for the infrastructure layout).
summary: >
  Browser uploads via presigned PUT have never landed in MinIO on the
  Hetzner deployment because the server-issued presigned URL points at
  the cluster-internal MinIO service URL, which the browser cannot
  resolve and which mixed-content policy blocks anyway. This spec splits
  the storage client into internal and public-facing endpoints, adds a
  public ingress for MinIO with TLS, configures CORS for the stagecraft
  origin, and updates the storage module so presigned URLs use the
  public endpoint while server-side ops keep using the internal one.
  Amends spec 087 NF-002 to add the browser-reachability requirement
  for the object-store endpoint that backs presigned uploads.
---

# 143 — Presigned upload public endpoint

## 1. Problem Statement

Spec 087 §4.4 lists `upload` as a connector type with the description
"Direct browser/API upload" and NF-002 specifies "S3-compatible (MinIO
for local dev, any S3-compatible provider for production)." Neither
section specifies that the object-store endpoint must be reachable
**from the browser**.

On the Hetzner production deployment that assumption is silently
violated. Three observations from a 2026-05-07 hotfix session:

1. **Single endpoint configuration.** `platform/services/stagecraft/api/knowledge/storage.ts:26` reads
   one secret, `S3_ENDPOINT`, and uses it for every S3 operation —
   server-side `headObject`, `putObject`, `deleteObject`, and the
   browser-bound `getSignedUrl(...)` for presigned PUT/GET URLs.

2. **The configured endpoint is cluster-internal.** `platform/infra/hetzner/setup.sh:329`
   seeds `S3_ENDPOINT=http://minio.stagecraft-system.svc.cluster.local:9000`
   into the `stagecraft-secrets` Kubernetes Secret. That hostname only
   resolves from inside the cluster's DNS scope, and the URL is
   plaintext HTTP while the page origin is HTTPS.

3. **No browser PUT has ever succeeded.** `kubectl get ingress -n stagecraft-system`
   shows `stagecraft-api` only — no MinIO ingress. Direct inspection
   of the MinIO pod data dir
   (`/export/oap-stagecraft-ing-default/`,
   `/export/oap-stagecraft-ing-test-dual-project-1/`,
   `/export/oap-stagecraft-ing-test2/`) shows no `knowledge/` prefix
   in any project bucket. Server-side flows that bypass the browser
   (Import, sync connectors) work because they use the internal
   endpoint directly; only the browser path is broken.

The user-visible symptom is a per-file "Load failed" toast on the
Knowledge tab Upload control after every upload. The accompanying
`requestUpload` row appears in `state: imported` because the row is
inserted at request-upload time (`api/knowledge/knowledge.ts:365`,
before the browser PUT), giving the misleading impression that the
server accepted the file. It did not. `confirmUpload` is never called,
no bytes land in MinIO, no extraction is enqueued, and the row stays
in `imported` forever.

The architecture mismatch — single endpoint, internal hostname,
HTTPS origin — is the root cause. Patching the client cannot fix it.

### 1.1 Evidence ledger

| Source | Finding |
|---|---|
| `platform/infra/hetzner/setup.sh:329` | `S3_ENDPOINT=http://minio.stagecraft-system.svc.cluster.local:9000` |
| `kubectl -n stagecraft-system exec deploy/stagecraft-api -- env` | `APP_BASE_URL=https://stagecraft.ing` (HTTPS browser origin) |
| `kubectl -n stagecraft-system get svc minio` | `ClusterIP 10.43.29.62`; no external endpoint |
| `kubectl -n stagecraft-system get ingress` | `stagecraft-api` only; no MinIO ingress |
| MinIO pod `/export/<bucket>/knowledge/` | Does not exist in any of the three project buckets |
| `api/knowledge/knowledge.ts:356-371` | Row is inserted in state `imported` before the browser PUT |
| `app/routes/app.project.$projectId.knowledge.tsx:425-428` | Browser PUT is the throw site for the rejected fetch |

### 1.2 Why the cluster ended up like this

`platform/infra/hetzner/post-create.sh:198-199` carries a comment from
the original Hetzner deployment:

> "no ingress because presigned URLs are issued by stagecraft and the
> signing surface should not be reachable from the public internet."

That comment documents a **server-side-proxy** design (the option B
discussed at spec-review time) that the application code never
matched. `storage.ts` always returned presigned URLs to the browser;
the cluster was provisioned for a server-side flow that was never
wired. The code drifted toward A; the deployment stayed at "neither
A nor B fully." Spec 143 closes the gap on the A side after an
explicit security-tradeoff review (see §3 Non-Goals).

## 2. Goals

- **Browser uploads land in MinIO on Hetzner without further infrastructure
  changes outside this spec.** A user clicking "Upload Documents" on
  the Knowledge tab produces an object in the project bucket and a
  `knowledge.upload_confirmed` audit row.
- **The `S3_ENDPOINT` invariant is preserved.** Server-side flows
  (Import, sync connectors, extraction workers) keep using the
  cluster-internal endpoint — fastest data path, no traversal of the
  ingress controller.
- **Cloud targets unaffected.** Azure Blob and AWS S3 endpoints are
  already public; the dual-endpoint surface degenerates to "internal
  and public are the same value" with no behavioural change.
- **Spec 087 NF-002 is amended in place** to add the browser-
  reachability requirement, making the gap explicit for any future
  cloud target whose object store is private by default.

## 3. Non-Goals

- **Replacing the presigned-PUT model with a server-side upload proxy.**
  Option B (browser POSTs to `/api/.../upload-direct`; stagecraft
  streams the body to `putObject`) was reviewed at spec-draft time on
  2026-05-07 with explicit attention to security tradeoffs:
  fewer internet-reachable services, single auth layer, no CORS
  surface, simpler audit. It was rejected for **A with explicit
  hardening** because (a) presigned URLs are the canonical browser-
  to-storage pattern that the cloud targets (AWS S3, Azure Blob, GCS)
  already use natively, so A unifies behaviour across deployments;
  (b) the security delta is closeable through ingress hardening
  (path allowlist, method allowlist, strict CORS, `MINIO_BROWSER=off`,
  CVE monitoring); and (c) bytes-through-Encore is operationally
  costly for the larger knowledge documents the audience uploads.
  A future spec MAY add a proxy fallback for environments where
  presigned URLs are not viable; spec 143 does not.
- **Multipart upload presigning.** Single PUT presigning caps at 5 GiB
  by S3 protocol; this spec caps further at 1 GiB by policy (see
  FR-011). Larger uploads need multipart, which requires presigning
  N+1 URLs (initiate, N parts, complete) and a stateful client.
  Out of scope; covered by a future spec when the size envelope demands.
- **Exposing `Access-Control-Expose-Headers: ETag` from MinIO.** The
  browser client at `web/app/routes/app.project.$projectId.knowledge.tsx:425-428`
  reads only `s3Res.ok` from the PUT response. ETag is not consumed
  by the upload state machine. Exposing it is harmless and may be
  added defensively at chart-config time, but is not a requirement.
- **Public download URLs for already-imported objects.** The presigned
  GET flow (`getDownloadUrl` in `api/knowledge/knowledge.ts:501`) has
  the same architectural shape and will inherit the fix as a side
  effect — but the desktop OPC consumer is the documented user of
  presigned downloads (spec 110 §2.3), and it can resolve cluster-
  internal hostnames when run via the platform's mirrord dev path.
  The download surface is in scope for the storage.ts split but its
  user-visible enablement is out of scope.
- **Per-bucket CORS configuration.** MinIO's CORS surface is global
  (server-level) on the chart we use; the spec configures the global
  policy for the stagecraft origin. Per-bucket CORS would require
  an `mc admin` step on bucket creation and is out of scope.
- **Migrating the in-cluster MinIO to an external object store.**
  Moving Hetzner deployments off MinIO onto an external S3 provider
  would also fix the symptom, but is a larger operational change
  that this spec does not propose.

## 4. Architecture

### 4.1 Endpoint duality

```
                        ┌──────────────────────────────────┐
   Browser              │   Stagecraft Encore service       │
   (https://            │   (in-cluster pod)                │
   stagecraft.ing)      │                                   │
        │               │   storage.ts                      │
        │               │     ┌─ internalClient ─→ S3_ENDPOINT          (server ops)
        │               │     └─ publicClient   ─→ S3_PUBLIC_ENDPOINT   (presigning)
        │               │                                   │
        │  presigned    │                                   │
        │  PUT URL      │                                   │
        │ ◀─────────────┤   getPresignedUploadUrl()          │
        │               │     uses publicClient             │
        │               │                                   │
   PUT (presigned URL)  └──────────────────────────────────┘
        │
        ▼
   ┌────────────────────────────────┐    ┌────────────────────────────┐
   │ nginx ingress                   │    │  In-cluster ops            │
   │  https://minio.stagecraft.ing  │───→│  http://minio.stagecraft-  │
   │  TLS, CORS, body limits         │    │  system.svc.cluster.local  │
   └────────────────────────────────┘    └────────────────────────────┘
```

Two S3 clients in `storage.ts`:

- **Internal client** — endpoint = `S3_ENDPOINT` (cluster-internal).
  Used by every server-side helper: `headObject`, `putObject`,
  `getObject`, `getObjectRange`, `deleteObject`, `listAllObjects`,
  `ensureBucket`, `sniffMimeType`.
- **Public client** — endpoint = `S3_PUBLIC_ENDPOINT` (browser-
  reachable). Used **only** by the two helpers that produce URLs
  consumed by the browser: `getPresignedUploadUrl` and
  `getPresignedDownloadUrl`.

Both clients share the same access key, secret, region, AND
`forcePathStyle: true`. MinIO under a custom domain rejects virtual-
hosted addressing (`https://bucket.host/key`) and requires path-
style (`https://host/bucket/key`); AWS S3 accepts both. The setting
is non-negotiable on both clients — see FR-001.

The S3 signature is computed over the URL the **client** is signing
for, so the public client signs URLs that resolve to the public
hostname; the internal client never produces external URLs. The
signature also covers the `Host` header, so the ingress chain MUST
preserve the browser-sent `Host: minio.stagecraft.ing` end-to-end —
see §4.3a and FR-006a.

### 4.2 Configuration semantics

`S3_PUBLIC_ENDPOINT` is a new secret with two semantics:

1. **Set explicitly** — the value is used verbatim for the public
   client. Hetzner sets `https://minio.stagecraft.ing`. AWS S3
   deployments set `https://s3.amazonaws.com`. Azure deployments
   set the blob endpoint URL.
2. **Unset / empty string** — the public client falls back to
   `S3_ENDPOINT`. This preserves backward compatibility for local
   dev (where MinIO is on `http://localhost:9000` and the same
   value works for both server and browser) and for cloud targets
   that have not been migrated yet.

The fallback semantics are explicit in `storage.ts`:

```ts
const publicEndpoint = s3PublicEndpoint() || s3Endpoint();
```

### 4.3 MinIO ingress (Hetzner)

A Helm values addition to the existing MinIO release plus a chart
template. Two sub-options exist for routing:

**4.3a Subdomain (chosen).** A dedicated host
`https://minio.stagecraft.ing` resolves to the same cluster ingress
IP as `stagecraft.ing`. nginx routes the request to the MinIO service
in `stagecraft-system`. Two concrete settings are non-negotiable
because SigV4 covers the `Host` header — any rewrite mid-chain
breaks signatures with cryptic `SignatureDoesNotMatch` errors:

- **Ingress.** `proxy_set_header Host $host;` (preserves the
  browser-sent host through to MinIO; nginx-ingress default
  preserves it but state explicitly).
- **MinIO server URL.** `MINIO_SERVER_URL=https://minio.stagecraft.ing`
  on the MinIO container env. Required for SigV4 canonicalisation
  paths and console/redirect URLs. Without this, MinIO recomputes
  the canonical request against its in-cluster hostname and rejects.

Console security is a separate concern from SigV4 (see §4.4). The
console-disable knob (`MINIO_BROWSER`) is a defence-in-depth
recommendation, not a SigV4 prerequisite — earlier draft revisions
of this spec conflated the two. Setting `MINIO_BROWSER_REDIRECT_URL`
is also informational (the console is not exposed via public ingress
regardless), kept for config coherence in case the console is ever
re-enabled.

CORS allows the `https://stagecraft.ing` origin and the
`PUT, GET, HEAD, OPTIONS` methods. Body size limit at the ingress
matches the policy cap (1 GiB, FR-011); MinIO's 5 GiB protocol
ceiling is not the limit we expose.

**4.3b Path prefix on the same host (rejected).**
`https://stagecraft.ing/_minio/...` served by the same ingress,
rewriting the path prefix when forwarding to MinIO. Rejected for
three reasons:

1. **CORS becomes meaningless.** Same-origin uploads bypass the
   browser's preflight gate; the policy boundary between the
   application and the storage layer is implicit instead of
   explicit. Subdomain forces an origin contract you can name.
2. **Path rewrites are fragile around SigV4.** The S3 SDK signs
   against the URL it generates; nginx rewriting `/_minio/<bucket>/<key>`
   → `/<bucket>/<key>` invalidates the signature unless every
   rewrite rule is reflected in the signing path the SDK is told
   to use. One config drift breaks every upload.
3. **URL-namespace pollution.** Reserving `/_minio/` on the
   application origin steals a path prefix from `stagecraft.ing`
   forever. Subdomain compartmentalises naming.

Public exposure is equal in both options — both are public TLS
endpoints. The rejection is about the engineering hygiene of the
boundary, not its security posture.

### 4.4 CORS, console security, and MinIO chart wiring

MinIO's CORS surface is configured through environment variables on
the MinIO container. The required envs (SigV4 hard requirements +
CORS contract):

```
MINIO_SERVER_URL=https://minio.stagecraft.ing
MINIO_API_CORS_ALLOW_ORIGIN=https://stagecraft.ing
```

**Console security.** The MinIO web console runs as a separate
service inside the pod (`consoleService` in the chart). Spec 143's
console-security posture is **layered**:

1. **Primary control: no public ingress.** `consoleService.type =
   ClusterIP` (chart default for our deployment) means the console
   service is never reachable from the internet, regardless of the
   `MINIO_BROWSER` env value. Only the API service gets the public
   ingress (`minio.stagecraft.ing`); the console stays in-cluster.
   Operator access is via `kubectl port-forward` (or an in-cluster
   `mc` pod).
2. **Defence-in-depth: `MINIO_BROWSER=off`.** Disables the console
   daemon globally. Recommended for deployments where operators
   prefer `mc` CLI over the web console; harmless to set even if
   the team uses port-forward access (port-forward will return no
   connection, and operators fall through to `mc`).

The implementation MAY choose either layer or both. Spec 143
recommends both for the Hetzner production deployment; local-dev
deployments typically leave `MINIO_BROWSER` unset for operator
ergonomics.

The browser's preflight `OPTIONS` request needs `Access-Control-
Allow-*` headers for `PUT, GET, HEAD, OPTIONS`, `Content-Type`, and
the `x-amz-*` headers the SDK adds at signing time. The chart we
use (`minio/minio` from charts.min.io) sets container env via the
top-level `environment` key in values, NOT `extraEnv` (which is the
Bitnami-chart convention and was the working hypothesis in spec
draft v1; corrected here). Step 4 of §7 uses `--set
environment.MINIO_API_CORS_ALLOW_ORIGIN=...` form. Verify against
the chart version pinned at deployment time before the implementation
PR; if the knob has migrated, surface in the PR review rather than
silently switching.

`Access-Control-Expose-Headers: ETag` is **not** required by the
current client (the upload state machine reads only `s3Res.ok`
from the PUT response, see `web/app/routes/app.project.$projectId.knowledge.tsx:425-428`).
It is a low-risk defensive add and MAY be configured at deploy time;
this spec does not require it.

### 4.5 Orphan-row reconciliation

The current `requestUpload` flow inserts the `knowledge_objects` row
in state `imported` **before** the browser PUT (`knowledge.ts:365`).
Two distinct orphan classes accumulate from this insertion ordering:

- **Class A — PUT failed, no blob.** Row says `imported`, the bytes
  never landed, `headObject(storageKey)` returns 404. The PUT may
  have failed for the spec-143 endpoint bug, a network drop, a
  user navigating away, or any other browser-side abort. The row
  is unrecoverable: even a successful retry creates a fresh
  `objectId` and `storageKey`, so this row is dead.
- **Class B — PUT succeeded, confirmUpload never fired.** Row says
  `imported`, the blob IS present, but the browser closed between
  the PUT and the confirm POST. The row + blob are recoverable —
  `confirmUpload` is idempotent against a real blob; the only
  reason the row is stuck is a missing client-side completion
  signal.

`retryExtraction` (spec 115 FR-010) cannot help either class — it
keys on `lastExtractionError`, which is null for rows whose
extraction has never even been attempted. Spec 143 owns the
cleanup for both classes because the diagnosis surfaced both, and
the fix would not be complete without it.

Two design options were considered:

1. **New `pending` state distinct from `imported`.** Migration to
   add the enum value, state-machine surgery on `requestUpload` /
   `confirmUpload`, retroactive backfill for existing orphans.
   Larger blast radius; touches spec 087's lifecycle directly.
2. **Sweep that reconciles orphan rows past a grace window.** A
   cron job scans rows in `imported` whose `created_at` is older
   than the grace window, calls `headObject`, and either deletes
   the row (Class A) or invokes `confirmUpload`'s core logic to
   self-heal (Class B). Self-contained; mirrors the spec 115
   extraction-staleness sweeper shape.

Spec 143 commits to option 2 (sweep). FR-010 specifies the
behaviour for both classes. The `pending`-state surgery is left as
future work in case the orphan rate or class-B frequency is high
enough to warrant the migration; the sweep is correct in either
case.

Symmetry note: the `connectors/sync` path also inserts in `imported`
before bytes land (spec 087 §4.4 `upload` connector vs other
connectors). The sweep covers both insertion paths because it keys
on `state + headObject` rather than the row's origin.

### 4.6 Presigned URL TTLs

Single-PUT presigning timing is bounded by the URL TTL: if the
browser is still uploading when the signature expires, MinIO rejects
the in-flight PUT. The 1 GiB policy cap (FR-011) on a 10 Mbps
residential uplink is ~14 minutes wall-clock under ideal conditions
and longer with TLS overhead and packet loss.

Pinned values:

| Surface | TTL | Rationale |
|---|---|---|
| Upload PUT | 3600s (60 min) | 4× headroom over the 1 GiB / 10 Mbps worst-case wall-clock; covers slow-uplink / mobile-tether scenarios; matches the existing `getPresignedUploadUrl` default at `storage.ts:107`. |
| Download GET (knowledge UI) | 300s (5 min) | Browser fetches kick off immediately on link click; long TTLs are leak-amplifiers for shared/copied URLs. |
| Download GET (factory bundle, spec 110) | 900s (15 min) | Existing value at `knowledge.ts:1646`; preserved unchanged by this spec. |

The download UI value (300s) is shorter than the existing Encore
helper default (3600s in `storage.ts:132`); the spec adds an
explicit `expiresInSeconds` argument at every download call site
rather than mutating the default.

### 4.7 Cert issuance topology (amendment, 2026-05-08)

The original §4.3a / FR-008 design assumed cert issuance for
`minio.stagecraft.ing` would use DNS-01 via a Hetzner cert-manager
webhook. That assumption conflated *cluster host* with *authoritative
DNS host* — they are independently configurable, and on the current
deployment they diverge:

- **Cluster host** — Hetzner Cloud (k3s on HCloud servers), provisioned
  by `platform/infra/hetzner/post-create.sh`.
- **Authoritative DNS host** — Cloudflare, fronting `stagecraft.ing`
  and all subdomains. The Hetzner DNS API was never wired authoritatively
  for the domain; `HCLOUD_DNS_API_TOKEN` is unset by design, not by
  oversight. The DNS-01 ClusterIssuer block in `post-create.sh:106-188`
  is gated on that token and consequently never created
  `letsencrypt-dns01` in the cluster — the MinIO ingress annotation
  pointing at it would have failed to issue if the upgrade had ever
  been attempted before this amendment.

Resolution — use HTTP-01 via the cluster's existing `letsencrypt-prod`
ClusterIssuer:

- The cluster bootstrap already provisions `letsencrypt-prod` at
  `post-create.sh:87-104` (HTTP-01 challenge, nginx solver). It is
  `Ready: True` and has issued certs for `stagecraft.ing` /
  `auth.stagecraft.ing` / `deploy.stagecraft.ing` for ~29 days.
- HTTP-01's "first rollout" failure mode (the original FR-008
  rationale) is bootstrap-specific: it bites when the ingress class
  is not yet routing on the cluster. By the time spec 143's MinIO
  ingress is added, `nginx-ingress` is already serving the parent
  domain's other hosts; `minio.stagecraft.ing` joins the same routing
  fabric and HTTP-01 succeeds without ordering hazard.
- The DNS-01 + Hetzner-webhook code block in `post-create.sh` remains
  in place as a dormant fallback, gated on the same `HCLOUD_DNS_API_TOKEN`
  check. A future migration of authoritative DNS to a provider with a
  cert-manager webhook (Hetzner DNS, Cloudflare DNS-01, Route 53, etc.)
  re-activates it without resurrecting deleted code.

Wildcard certs (`*.stagecraft.ing`) and DNS-only-validatable certs
remain DNS-01 territory by ACME design; they are out of scope for
spec 143 and trigger the future migration above when needed.
FR-008 is amended to reflect this scoping.

## 5. Functional Requirements

- **FR-001** — `storage.ts` MUST expose two `S3Client` instances:
  an internal client for server-side ops and a public client for
  presigning. The internal client's endpoint is `S3_ENDPOINT`. The
  public client's endpoint is `S3_PUBLIC_ENDPOINT` if non-empty,
  else `S3_ENDPOINT`. Both clients MUST set `forcePathStyle: true`
  (path-style addressing is non-negotiable for MinIO under a custom
  domain; AWS S3 accepts both styles, so this is a safe universal
  default).
- **FR-002** — Only `getPresignedUploadUrl` and
  `getPresignedDownloadUrl` MAY use the public client. Every other
  helper MUST use the internal client. A regression test (FR-009b)
  pins this asymmetry.
- **FR-003** — `infra.config.hetzner.json` and `infra.config.json`
  MUST declare `S3_PUBLIC_ENDPOINT` as an env-mapped secret (the
  same shape as the existing `S3_ENDPOINT`).
- **FR-004** — `platform/infra/hetzner/setup.sh` MUST seed
  `S3_PUBLIC_ENDPOINT=https://minio.stagecraft.ing` (or the
  configured equivalent) into the `stagecraft-secrets` Kubernetes
  Secret.
- **FR-005** — The MinIO Helm release on Hetzner MUST be configured
  with an ingress at `https://minio.stagecraft.ing`, TLS via
  cert-manager, CORS allowing the `https://stagecraft.ing` origin
  and methods `PUT, GET, HEAD, OPTIONS`, and a body size limit at
  the policy cap (1 GiB, see FR-011).
- **FR-006** — Browser PUT against the presigned URL MUST succeed
  end-to-end: the file lands in MinIO, `confirmUpload` is called
  with a 2xx response, and `enqueueExtraction` fires.
- **FR-006a** — The ingress chain MUST preserve the browser-sent
  `Host: minio.stagecraft.ing` header end-to-end so SigV4
  canonicalisation matches between browser and MinIO. Required:
  (a) ingress config sets `proxy_set_header Host $host;`, (b) the
  MinIO container env sets `MINIO_SERVER_URL=https://minio.stagecraft.ing`.
  Failure to set either of (a)–(b) produces `SignatureDoesNotMatch`
  with no other diagnostic; the implementation PR MUST verify the
  chain with a real preflight + PUT against the deployed cluster.

  Recommended (not required) for config coherence:
  - `MINIO_BROWSER_REDIRECT_URL=https://minio.stagecraft.ing` —
    dead env while the console is not exposed via ingress; setting
    it now means a future operator who flips the console on
    inherits a correct redirect rather than a stale one.

  Console security is a separate concern handled by §4.4 (no
  public console ingress; optional `MINIO_BROWSER=off` for
  defence-in-depth). Earlier draft revisions of this FR
  incorrectly listed `MINIO_BROWSER=off` as a SigV4 requirement.
- **FR-007** — When `S3_PUBLIC_ENDPOINT` is unset, behaviour MUST
  be identical to the pre-spec-143 baseline (single-endpoint mode).
  This preserves AWS S3 / public-endpoint deployments without
  configuration churn and supports local dev where MinIO at
  `http://localhost:9000` is reachable from the browser directly.
- **FR-008** — DNS for `minio.stagecraft.ing` is a deployment-time
  concern documented in `platform/infra/hetzner/post-create.sh`.
  **Amended 2026-05-08, see §4.7.** The cert-manager flow MUST use
  **DNS-01** when the authoritative DNS provider supports a
  cert-manager webhook AND a wildcard or DNS-only-validatable cert
  is required. **HTTP-01** is acceptable for single-host
  non-wildcard certs once the cluster's nginx ingress is already
  routing the parent domain — at that point the ingress-bootstrap
  failure mode that motivated the original DNS-01 mandate no longer
  applies. Hetzner deployments today use HTTP-01 via the cluster's
  pre-existing `letsencrypt-prod` ClusterIssuer for
  `minio.stagecraft.ing`: authoritative DNS for `stagecraft.ing` is
  at Cloudflare (not Hetzner DNS), so the Hetzner DNS-01 webhook
  is not applicable to this deployment topology. The DNS-01
  ClusterIssuer block in `post-create.sh` is preserved as dormant
  (gated on `HCLOUD_DNS_API_TOKEN`, which remains unset) so a
  future authoritative-DNS migration can re-activate it without
  resurrecting deleted code. Future specs that need wildcard or
  DNS-only-validatable certs MUST first migrate to a DNS provider
  with a cert-manager webhook (Hetzner DNS, Cloudflare DNS-01
  solver, Route 53, etc.). Spec CI checks MUST NOT depend on live
  DNS.
- **FR-009** — A regression test in
  `platform/services/stagecraft/api/knowledge/storage.integration.test.ts`
  MUST stand up a fake S3-compatible server bound to a non-default
  hostname (e.g. via a localhost listener with a `Host`-aware
  matcher), point the public client at it, presign a PUT URL, and
  verify the fake server accepts the PUT — i.e. signature
  validates against the public-endpoint host, not the internal one.
  Pure URL-host matching is insufficient because it cannot catch
  Host / path-style / SigV4 canonicalisation drift, which is the
  actual production failure mode.
- **FR-009b** — A symmetric test MUST verify that
  `headObject(...)` against the same args goes through the
  **internal** client even when both endpoints are configured.
  Fixes the failure mode where someone wires the wrong client to
  a server-side helper during refactor.
- **FR-010** — Orphan-row sweeper. A new Encore CronJob
  (recommended ID: `knowledge-orphan-imported-sweeper`, schedule
  `every 30m`) reconciles rows in `state = 'imported'` whose
  `created_at < now() - INTERVAL <grace_window>` along **two**
  paths, since two distinct orphan classes exist:

  - **Class A — PUT failed, no blob.** `headObject(bucket, storageKey)`
    returns 404 (NotFound). The bytes never landed. The sweeper
    DELETEs the row in a single transaction with the audit insert.
    Audit shape: `action = knowledge.upload_orphaned`,
    `actorUserId = SYSTEM_USER_ID` (`00000000-0000-0000-0000-000000000000`,
    matching the convention used by the spec-115 staleness sweeper
    and other Encore CronJobs), `metadata = { filename, storageKey,
    class: "no_blob", projectId }`.
  - **Class B — PUT succeeded, confirmUpload never fired.** The
    blob is present (HEAD returns 200) but the row is still in
    `imported` because the browser tab closed (or the network
    flapped) between the PUT and the confirm POST. The sweeper
    invokes `confirmUploadCore` (the extracted core of the user-
    driven `confirmUpload` API handler — same pattern as
    `listKnowledgeObjectsCore` from `c1b5d51`) to update
    `sizeBytes`, write a `knowledge.upload_confirmed` audit row,
    and enqueue extraction. The row is **not** deleted — it
    self-heals into the normal pipeline.

    Audit semantics for Class B: the sweeper emits the **same
    action name** as the user-driven flow (`knowledge.upload_confirmed`),
    NOT a separate `knowledge.upload_self_healed` action. The
    rationale: the outcome is identical, dashboards keying on
    `action = knowledge.upload_confirmed` should see all confirms
    uniformly, and analytics that need to distinguish sweeper-
    driven confirms can filter on `actorUserId = SYSTEM_USER_ID`
    AND `metadata.source = "orphan_sweep_class_b"`. The user-
    driven confirm path keeps its existing audit shape (no
    `metadata.source` field, real `actorUserId`).

  The grace window is configurable via
  `STAGECRAFT_KNOWLEDGE_ORPHAN_AFTER_SEC` (default 3600s = upload
  TTL; a row whose URL has expired and has no blob is unrecoverable
  by the browser, and a row with a blob past the URL TTL has clearly
  lost its confirm signal).

  Cadence rationale: cleanup latency = grace + cadence. With grace
  3600s and cadence 1800s the worst-case latency is 90 min. Tighter
  cadence is wasted polling at current scale because no user-visible
  surface depends on sub-hour reconciliation (orphans are invisible
  to the UI until they accumulate at scale). Loosen further only if
  observability shows the sweep is meaningfully load-burning.

  **Self-hosted scheduler (amendment, 2026-05-08).** Encore's
  `CronJob` primitive is scheduled by Encore Cloud's platform
  scheduler, not by anything inside the application image. In
  self-hosted deployments (`encore build docker` + a K8s cluster
  with no Encore Cloud connection — exactly what
  `make deploy-hetzner` produces), the CronJob declaration is a
  no-op for scheduling purposes. The endpoint exists and is
  callable; nothing calls it. Empirically confirmed against the
  Hetzner cluster on 2026-05-08: 2 hours of stagecraft-api logs
  show zero `endpoint: runExtractionStalenessSweep` entries
  despite the spec-115 sweeper being declared at `every: "1m"`
  (which should have produced ~120 fires).

  Spec 143 therefore requires TWO scheduler entries for the orphan
  sweep:

  1. The Encore `CronJob` declaration stays in `scheduler.ts`. It
     is the local-dev entry point (Encore CLI does run cron
     handlers under `encore run`), the documentation form of the
     schedule, and the future Encore Cloud migration path.
  2. A Kubernetes `CronJob` resource provisioned in
     `platform/infra/hetzner/post-create.sh` is the **production
     scheduler for self-hosted deployments**. It curls the
     internal endpoint
     `http://stagecraft-api.stagecraft-system.svc.cluster.local:80/internal/knowledge/orphan-imported-sweep`
     on the same `every 30m` cadence. Both call into
     `runOrphanSweep()` via the same handler.

  > **Partially implemented (2026-05-08).** The K8s CronJob is
  > deployed and fires on schedule, but every run currently 404s.
  > Root cause: `scheduler.ts:194` declares the handler with
  > `expose: false`, so Encore returns 404 to any external HTTP
  > caller — including in-cluster service callers like the K8s
  > CronJob. The Encore `expose:false` design intends "callable
  > only from inside the same Encore service" via direct function
  > import, not "callable from anywhere on the cluster network".
  > Reconciliation does not actually run in production. FR-010 is
  > therefore not delivered against self-hosted deployments. The
  > Class A delete and Class B self-heal logic in `orphanSweeper.ts`
  > is correct and unit-tested; the integration is broken at the
  > scheduler-to-handler hop. Follow-up: see §12 L-004.

  This is a SYSTEMIC finding, not a spec-143-specific concern: the
  existing extraction-staleness sweeper (spec 115 FR-006), the
  connector sync scheduler (spec 087 §4.4), and the factory-runs
  staleness sweeper (spec 124) have all been silently no-ops in
  production since they were written. Spec 143 surfaces and fixes
  the pattern for its own sweeper; the upstream sweepers' fixes
  belong to follow-up amendments on their owner specs (see §12).

  Concurrency model: Encore CronJob does NOT guarantee single-flight
  at the platform level (the existing extraction-staleness sweeper
  relies on idempotence, not exclusion). Spec 143's sweep is safe
  under concurrent execution at three layers:

  1. **Sweeper-vs-sweeper, Class A.** Per-row `DELETE ... WHERE id
     = $1 AND state = 'imported' RETURNING id` plus same-transaction
     audit insert. Only the first concurrent DELETE returns rows;
     the second is a no-op and skips the audit. No duplicate
     `knowledge.upload_orphaned` rows.
  2. **Sweeper-vs-sweeper, Class B.** Per-row update of `sizeBytes`
     and `updatedAt` is naturally idempotent (same row, same
     value). The audit + enqueue calls inside `confirmUploadCore`
     can fire twice; the audit double is informational noise (a
     journal entry, not a deduplicated event), and the extraction
     enqueue is deduplicated by spec 115 FR-003 (`(projectId,
     contentHash, extractorVersion)` over the last 24h, see
     `extractionCore.ts:148-200`). Net result: at most two
     `knowledge.upload_confirmed` audit rows but exactly one
     extraction run.
  3. **Sweeper Class B vs returning user.** A user who reopens a
     tab and triggers `confirmUpload` while the sweeper is mid-
     flight produces:
     - **Two** `knowledge.upload_confirmed` audit rows (deterministic
       when both transactions commit; one with `metadata.source =
       "orphan_sweep_class_b"`, one without). The audit log has no
       uniqueness constraint and both INSERTs succeed
       unconditionally.
     - **At most two** extraction runs, **modulo spec 115 FR-003's
       dedup atomicity**. FR-003 (`extractionCore.ts:148-200`)
       implements dedup as SELECT-then-INSERT, which has a
       race window: two concurrent enqueues can both observe
       "no existing run" and both INSERT, producing two pending
       runs that the worker will then process serially. Under
       sequential timing FR-003 collapses the second enqueue to
       `outcome: "deduped"` and only one run exists; under
       contention up to two runs may exist. Both outcomes are
       acceptable — the worker is idempotent at the per-run level,
       and a doubled run is wasted-but-correct.

     **Load-bearing dependency on spec 115.** Spec 143's race
     contract assumes spec 115 FR-003's dedup window is non-zero
     and follows the SELECT-then-INSERT shape. Any future spec
     that relaxes FR-003's window or removes the dedup entirely
     weakens spec 143's race contract proportionately. Spec 115
     carries the symmetric `amendment_record: 143-...` annotation
     on FR-003 so the dependency is visible from both directions.
     The race test in §8 asserts the looser bounds (`toBe(2)` for
     audits, `toBeLessThanOrEqual(2)` for extraction runs)
     accordingly; tightening to `== 1` would silently break if
     FR-003's window were ever shortened to a sub-millisecond
     value where the race window dominates.

  The action constant `KNOWLEDGE_UPLOAD_ORPHANED = "knowledge.upload_orphaned"`
  MUST be exported from
  `platform/services/stagecraft/api/knowledge/auditActions.ts`
  alongside the existing spec-115 constants. Pattern matches
  `knowledge.<noun>_<verb_past>` per the existing naming
  convention.

  **Self-hosted scheduler requirement (amendment, 2026-05-08).** In
  addition to the Encore `CronJob` declaration, the deployment
  scripts MUST provision a Kubernetes `CronJob` resource (Helm-owned
  under `platform/charts/stagecraft/templates/cronjob-orphan-sweeper.yaml`,
  superseding the earlier `post-create.sh` heredoc bootstrap) that
  calls the internal sweep endpoint on the same cadence
  (`*/30 * * * *`). The K8s CronJob is the actual production
  scheduler for self-hosted deployments; the Encore CronJob
  declaration is local-dev and future-Encore-Cloud only. See §4.5
  self-hosted scheduler amendment for the rationale and empirical
  evidence.

  **Per-purpose credential mount discipline (amendment, 2026-05-09).**
  Each sweeper CronJob mounts only its purpose-specific M2M client
  credentials; cross-purpose mounts are forbidden. Concretely: the
  K8s CronJob authenticates to the internal sweep endpoint via a
  Rauthy-issued `client_credentials` JWT carrying the matching
  `platform:<service>:sweep` scope (here `platform:knowledge:sweep`);
  the JWT-fetch credentials live in a per-purpose K8s Secret (here
  `stagecraft-knowledge-sweeper-credentials`, materialised by
  `setup.sh` from `STAGECRAFT_KNOWLEDGE_SWEEPER_CLIENT_ID/_SECRET`
  on Hetzner, and from a dedicated ExternalSecret on ESO-backed
  clouds), and that Secret is the only credential surface the
  CronJob's pod sees. A leaked credential is bounded to that one
  sweeper's surface — defence in depth at the credential layer, not
  only at the validator. FU-003's K8s CronJobs for spec 115 FR-006
  (`extraction-staleness-sweeper`), spec 087 §4.4
  (`connector-sync-scheduler`), and spec 124
  (`factory-runs-staleness-sweeper`) inherit this discipline without
  re-deriving it. See §12 L-004 Option 1 + L-006 for the Rauthy 0.35
  *Default Scopes* nuance the discipline rests on.

- **FR-011** — Upload size cap. The browser client MUST refuse
  files > 1 GiB before issuing `requestUpload` (UI-side fail-fast,
  user-visible toast). The `requestUpload` server handler MUST
  also refuse with `APIError.invalidArgument` for the same
  threshold. The ingress body size MUST also be capped at 1 GiB
  (FR-005), as a defence-in-depth backstop that should never fire
  in practice because the server check rejects first; if the
  ingress 413 ever reaches a user, that is a server-check
  regression, not an expected error path. Three layers, three
  distinct error surfaces, one shared limit.
- **FR-012** — Presigned URL TTLs are pinned per §4.6:
  3600s upload, 300s knowledge-UI download, 900s factory-bundle
  download (unchanged from existing `KNOWLEDGE_BUNDLE_URL_TTL_SECONDS`).
- **FR-013** — Multipart upload presigning is **out of scope**.
  Single-PUT presigning per AWS SigV4 v4 is the only browser-to-
  storage path this spec specifies. Files above the 1 GiB cap are
  rejected, not chunked.

## 6. Non-Functional Requirements

- **NF-001** — The fix MUST NOT introduce a server-side bytes-through-
  proxy upload path. The presigned-PUT contract is the only
  browser→storage data path.
- **NF-002** — TLS at the MinIO ingress is REQUIRED. HTTP-only
  endpoints fail mixed-content; this is not a temporary-by-design
  state.
- **NF-003** — The fix is deployable as a normal `make deploy-hetzner`
  Helm upgrade — no fast-build hotfix path. The accompanying code
  change ships through the supported `make docker-build-hetzner`
  builder, which since 2026-05-07 (`1e97ef2`) regenerates `web/build/`
  before `encore build docker` so the bundled main.mjs and the
  client tree share matching asset hashes.

## 7. Implementation Plan

Eight steps, intended to land as discrete PRs / commits for traceable
review:

1. **Storage client split.** Refactor `storage.ts` to maintain two
   `S3Client` instances (both with `forcePathStyle: true`).
   Introduce `s3PublicEndpoint` secret with the fallback semantics
   from §4.2. Route only `getPresignedUploadUrl` /
   `getPresignedDownloadUrl` through the public client. Pin the
   download TTLs at every call site per FR-012 / §4.6. Add
   integration tests for FR-009 and FR-009b.
2. **Encore secret declaration.** Add `S3_PUBLIC_ENDPOINT` to
   `infra.config.hetzner.json` and `infra.config.json` env mapping.
   _(Landed bundled into step 1's commit — the two-line config edit
   was on the critical path for the test fixture to resolve the new
   secret. The "eight steps" header is preserved as a planning
   surface; logically step 2 is complete via commit `14c5c56`.)_
3. **Orphan-row sweeper.** Implement the
   `knowledge-orphan-imported-sweeper` CronJob per FR-010 covering
   both Class A (delete) and Class B (self-heal via
   `confirmUpload` core logic). New module
   `platform/services/stagecraft/api/knowledge/orphanSweeper.ts`
   alongside `extractionCore.ts`. Mirrors the spec 115
   extraction-staleness sweeper shape. Adds
   `KNOWLEDGE_UPLOAD_ORPHANED` to `auditActions.ts`. Refactor
   `confirmUpload` to expose its idempotent core (`confirmUploadCore`)
   so the sweeper can reuse it without going through the auth/api
   surface — same pattern as `listKnowledgeObjectsCore` introduced
   by `ec84e76`.
4. **Upload size cap (browser + server).** Implement FR-011: client
   pre-check at `app.project.$projectId.knowledge.tsx` `handleFiles`
   (early reject with a user-visible toast); server-side check in
   `requestUpload` mirroring the same threshold.
5. **Hetzner setup script.** `platform/infra/hetzner/setup.sh` seeds
   `S3_PUBLIC_ENDPOINT=https://minio.stagecraft.ing` into the
   `stagecraft-secrets` Secret. The host is configurable via env in
   `.env.example` (default to the documented production host).
6. **MinIO chart wiring.** Update the MinIO Helm release in
   `platform/infra/hetzner/post-create.sh` to add ingress + env:

   Required (SigV4 + CORS):
   - `--set environment.MINIO_SERVER_URL=https://minio.stagecraft.ing`
   - `--set environment.MINIO_API_CORS_ALLOW_ORIGIN=https://stagecraft.ing`

   Required (ingress topology):
   - `--set ingress.enabled=true`,
     `--set ingress.hosts[0]=minio.stagecraft.ing`,
     `--set ingress.tls[0].secretName=minio-tls`,
     `--set ingress.tls[0].hosts[0]=minio.stagecraft.ing`
   - `--set ingress.annotations."nginx\.ingress\.kubernetes\.io/proxy-body-size"=1g`
     (value MUST match `KNOWLEDGE_UPLOAD_MAX_BYTES` from
     `api/knowledge/uploadLimits.ts`; the comment in
     `uploadLimits.ts` calls out the propagation requirement and
     the chart `--set` line points back to it)
   - `--set ingress.annotations."cert-manager\.io/cluster-issuer"=letsencrypt-prod`
     (was `letsencrypt-dns01` pre-amendment; see §4.7 / amended FR-008
     for the topology rationale.)

   Recommended (defence-in-depth + config coherence):
   - `--set environment.MINIO_BROWSER=off` — disables the console
     daemon. Console security is also achieved by
     `consoleService.type=ClusterIP` (existing config; no public
     ingress on the console service), so this is belt-and-
     suspenders, not a hard requirement.
   - `--set environment.MINIO_BROWSER_REDIRECT_URL=https://minio.stagecraft.ing`
     — informational; coherent state for any future operator who
     flips the console back on.

   Also update the stale comment block at `post-create.sh:198-199`
   (which currently documents the rejected server-side-proxy intent
   — see §1.2) to reflect the actual A-with-hardening design.

   Verify chart-knob names against the chart version pinned at
   deployment time before the PR (the `environment` vs `extraEnv`
   distinction; see §4.4).
7. **DNS / cert-manager.** **Amended 2026-05-08 (see §4.7 / FR-008).**
   Provision the DNS A record for `minio.stagecraft.ing` at the
   authoritative DNS provider (Cloudflare for the current deployment;
   Hetzner DNS would be the alternative if/when migrated) pointing at
   the cluster's nginx ingress IP. Use the existing `letsencrypt-prod`
   ClusterIssuer (HTTP-01 via the nginx solver) — already provisioned
   at `post-create.sh:87-104` and verified `Ready: True` against the
   deployed cluster. The DNS-01 + Hetzner-webhook block at
   `post-create.sh:106-188` stays in place as a dormant fallback;
   activating it requires migrating authoritative DNS to a provider
   with a cert-manager webhook, which is out of scope here.

7b. **Self-hosted scheduler (K8s CronJob).** Add a Kubernetes
    `CronJob` resource to `platform/infra/hetzner/post-create.sh`
    that calls the orphan-sweep endpoint on the same `every 30m`
    cadence as the Encore CronJob declaration. Required because
    Encore's CronJob primitive is scheduled by Encore Cloud's
    platform, not by the application image — see §4.5
    self-hosted scheduler amendment + §12 L-001 for the
    rationale and empirical evidence. The K8s CronJob curls
    `http://stagecraft-api.stagecraft-system.svc.cluster.local:80/internal/knowledge/orphan-imported-sweep`
    via a small image (e.g. `curlimages/curl`); idempotent on
    re-apply (kubectl apply -f -).
8. **End-to-end validation.** Land
   `platform/infra/hetzner/validate/spec-143.sh` as the executable
   form of the spec contract; run it after every deploy that
   touches the upload path. The script splits checks into two
   classes by exit code:

   - **exit 2** — prerequisite failure (deploy is incomplete: DNS
     missing, cert not issued, ingress unreachable, CORS
     misconfigured). Operator finishes the deploy and re-runs.
   - **exit 3** — contract failure (deploy is complete but the
     spec guarantee is broken: signature mismatch, blob did not
     land, sweeper not registered). Spec defect; integration
     tests should not have passed.
   - **exit 0** — all checks pass.

   The CORS preflight check uses a real `OPTIONS` request with
   `Origin: ${APP_BASE_URL}` and asserts the `Access-Control-Allow-Origin`
   response header — naked `curl -X PUT` would false-pass against
   broken CORS because curl does not preflight. The validation
   leaves the cluster in the same state it started: an EXIT trap
   removes the test blob, the synthetic `knowledge_objects` row,
   and the audit rows scoped to its `target_id`.

   Manual ad-hoc spot-check for first deploy (kept for reference;
   the script supersedes this for repeatable verification): run a
   real upload from the deployed stagecraft web UI; observe
   `knowledge.upload_confirmed` audit row; observe a non-empty
   `knowledge/` prefix in the project bucket on the MinIO pod.

Steps 1–4 are code-only and land first; the resulting deployment
falls back to single-endpoint behaviour because `S3_PUBLIC_ENDPOINT`
is unset until step 5 runs against the cluster. Steps 5–7 are
infrastructure changes. Step 8 closes the spec. There is no broken
intermediate state at any step boundary.

## 8. Testing

| Surface | Test | Where |
|---|---|---|
| Public client signs correctly against non-default host (FR-009) | Stand up a fake S3-compatible listener bound to a non-default hostname; point public client at it; presign + PUT; verify the listener accepts the signature | `storage.integration.test.ts` |
| Internal-client asymmetry (FR-009b) | With both endpoints configured, call `headObject` and assert the request hit the **internal** endpoint (not the public listener) | same |
| Backward compatibility (FR-007) | When `S3_PUBLIC_ENDPOINT` is unset, presigned URLs match `S3_ENDPOINT`; both clients are configured the same way | same |
| `forcePathStyle` invariant (FR-001) | Inspect both client configs and assert `forcePathStyle: true` | same |
| Upload size cap (FR-011) | Client: simulate `handleFiles` with a 1.1 GiB file; assert the pre-check rejects without calling `requestUpload`. Server: call `requestUpload` directly with `sizeBytes` > 1 GiB and assert `APIError.invalidArgument` | new client unit + existing server integration |
| Orphan sweeper Class A (FR-010, no blob) | Insert an `imported` row past grace with a `storageKey` that has no blob; run the sweeper; assert row deleted and `knowledge.upload_orphaned` audit row written with `metadata.class = "no_blob"` | `orphanSweeper.integration.test.ts` (new) |
| Orphan sweeper Class B (FR-010, blob present) | Insert an `imported` row past grace whose `storageKey` HAS a blob in the bucket; run the sweeper; assert row stays present, state remains `imported` (until extraction), `sizeBytes` updated to match S3, `knowledge.upload_confirmed` audit row written, extraction enqueued | same |
| Orphan sweeper concurrency, Class A | Run two sweeper invocations in parallel against a shared Class A row; assert exactly one DELETE returns rows, exactly one `knowledge.upload_orphaned` audit row written | same |
| Orphan sweeper Class B vs user confirm race | Insert an `imported` row past grace whose `storageKey` HAS a blob; spawn the sweeper's Class B handler and a `confirmUpload` API call concurrently; assert exactly one extraction run is created (FR-003 dedup carries through), and at most two `knowledge.upload_confirmed` audit rows exist (one user, one with `metadata.source="orphan_sweep_class_b"`) | same |
| Orphan sweeper does not touch fresh rows | Insert an `imported` row with `created_at = now()`; run the sweeper; assert untouched | same |
| Server-side ops unchanged | Existing `headObject` / `putObject` integration tests pass | existing |
| Browser PUT success (FR-006, deploy-time) | Real upload from the deployed Knowledge tab; observe `knowledge.upload_confirmed` audit row and non-empty `knowledge/` prefix in MinIO bucket | deploy-time |
| Host preservation (FR-006a, deploy-time) | `curl -X OPTIONS -H 'Origin: https://stagecraft.ing' -H 'Access-Control-Request-Method: PUT' https://minio.stagecraft.ing/<bucket>/<key>` returns 200 + ACAO header set to the stagecraft origin | deploy-time |
| Mixed-content regression (deploy-time) | DevTools Network panel shows the presigned URL is HTTPS on the HTTPS page | deploy-time |
| TTL bound (FR-012) | Inspect generated presigned URL's `X-Amz-Expires` query param: 3600 for upload, 300 for knowledge-UI download, 900 for factory-bundle download | unit-level inspection |

## 9. Migration

Pre-spec-143 deployments (Hetzner, today): browser uploads silently
fail.

Post-step-3 deployment: browser uploads still fail until step 4–5
land — but the server-side flows are unchanged so all other paths
keep working. There is no broken intermediate state.

Post-step-4-5 deployment: browser uploads succeed. Existing
knowledge objects are unaffected (the bucket layout is unchanged).
No backfill is required because no prior uploads exist to backfill.

## 10. Amendment record

This spec amends spec 087 (`unified-workspace-architecture`).

**Section affected:** §10 Non-Functional Requirements, NF-002.

**Pre-amendment:**
> NF-002: The object store must be S3-compatible (MinIO for local
> dev, any S3-compatible provider for production).

**Post-amendment (proposed):**
> NF-002: The object store must be S3-compatible (MinIO for local
> dev, any S3-compatible provider for production). The endpoint
> used for browser-issued presigned URLs MUST be reachable from
> the browser origin under TLS — separately configurable from the
> server-side endpoint where the two diverge. See spec 143 for the
> dual-endpoint storage client design and the Hetzner ingress
> implementation.

Spec 087's frontmatter SHOULD pick up `amended: 2026-05-07` and
`amendment_record: 143-presigned-upload-public-endpoint` when the
amendment is applied.

## 11. Open Questions

1. **Q11.1 — Spec 087 amendment shape.** The amendment record above
   is in `spec.md` of 143. The convention from `.specify/contract.md`
   says spec 087 itself should also pick up the amendment marker
   (`amended: 2026-05-07`, `amendment_record: 143-...`, body callout
   on NF-002). Decide whether 087's frontmatter update lands in the
   same PR as the spec-143 implementation step 1 or in a separate
   amendment-hook commit. Per spec-127 coupling, mass-amend of
   owner specs is discouraged; a separate hook commit isolates the
   amendment's blast radius.
2. **Q11.2 — Local dev parity.** Local dev runs MinIO on
   `http://localhost:9000`, which is reachable from the browser
   directly. The fallback semantics in FR-007 cover this — but a
   local dev convention SHOULD be set: leave `S3_PUBLIC_ENDPOINT`
   unset locally; the docs MUST document the choice in
   `platform/services/stagecraft/CLAUDE.md` and `.env.example`.
3. **Q11.3 — Chart-knob verification at PR time.** §4.4 calls
   for `environment.MINIO_API_CORS_ALLOW_ORIGIN` per the official
   `minio/minio` chart's current values shape, vs the older
   `extraEnv` (Bitnami-chart convention). The implementation PR
   for §7 step 6 MUST verify the knob name against the chart
   version pinned at deploy time and surface any drift in PR review
   rather than silently switching.

## 12. Lessons (added by 2026-05-08 amendment)

These are findings the spec-143 implementation surfaced that
generalise beyond spec 143. Captured here so they propagate to
future specs and other consumers of the same patterns.

**L-001 — Encore platform primitives in self-hosted deployments.**
Encore's `CronJob` (and possibly other platform primitives —
PubSub workers, Object Storage, Caching) are scheduled or driven
by Encore Cloud's platform components, not by anything inside
the application image. In self-hosted deployments
(`encore build docker` + a K8s cluster with no Encore Cloud
connection), these primitives become no-ops or partial
implementations.

Empirical evidence (2026-05-08 Hetzner cluster):

- `audit_log` shows zero `knowledge.upload_confirmed` rows since
  the cluster started — confirms spec 143's diagnosis at the
  audit-trail level.
- 2 hours of stagecraft-api logs show zero
  `endpoint: runExtractionStalenessSweep` entries despite spec
  115's sweeper being declared at `every: "1m"` (~120 expected).
- `kubectl get cronjobs -n stagecraft-system` returns "No
  resources found" — no K8s-side scheduler exists for the
  Encore primitive.

Affected sweepers (each owner spec carries the bug
independently):

- Spec 115 FR-006 — `extraction-staleness-sweeper` (every 1m).
  Worker-crash recovery silently broken.
- Spec 087 §4.4 — `connector-sync-scheduler` (every 15m).
  Scheduled connector syncs silently broken.
- Spec 124 — `factory-runs-staleness-sweeper`. Stale factory
  run sweeping silently broken.
- Spec 143 FR-010 (this spec) — orphan-imported sweeper, fixed
  in step 7's K8s CronJob addition before it shipped broken.

The pattern: Encore's `CronJob` declaration is the local-dev
entry point; production self-hosted deployments must provision
a sibling K8s `CronJob` resource that curls the registered
endpoint on the same cadence. This requirement should be
reflected in any future spec that uses Encore's `CronJob` (or
the analogous self-hosted gap for other platform primitives).

A separate spec amending the affected owner specs (115, 087,
124) and codifying the rule generally is appropriate work for
a follow-up. Spec 143 fixes its own sweeper; the systemic
remediation is out of scope to keep 143's blast radius bounded.

**L-002 — Spec review must verify deployment-target alignment.**
The Encore CronJob misuse passed three review rounds without
being flagged because reviewers (including me) treated the
primitive as if it worked the same way across deployment
targets. The deployment target for this entire codebase has
been self-hosted Hetzner from page one — that should have been
the first question at FR-010 review, not a discovery during
post-completion validation. Add deployment-target verification
to the spec review checklist for any FR that depends on
platform primitives.

**L-003 — Cluster-state surfaces must have exactly one writer.**
On 2026-05-08 a `setup.sh` run during spec 143 infra provisioning
silently rolled the stagecraft pod back ~7 hours of code while the
DB stayed forward-migrated. Symptoms: `listProjects` /
`listAdapters` / `getUpstreams` all 500'd because the running
binary queried `workspace_id` (dropped by spec 119 migration 27),
`factory_adapters` (dropped by spec 139 migration 34), and
`factory_source` (dropped by spec 140 migration 36). The user
saw "no projects", "no factory adapters", and `/app/factory` 500.

Mechanism — dual writers on the same field:

- CD (`cd-stagecraft.yml`) ran on every push to main and called
  `helm upgrade --install stagecraft … --set image.tag=sha-${SHA}`,
  correctly pinning the deployment to an immutable tag.
- `setup.sh` ran on every operator infra change and called
  `helm upgrade --install stagecraft … -f values-hetzner.yaml`
  with no `--set image.tag` override. `values-hetzner.yaml`
  shipped `tag: latest`, which won the helm field-manager
  contention and rewrote the deployment to `:latest`.
- `imagePullPolicy: IfNotPresent` (K8s default that survived
  through helm's strategic merge) then resolved `:latest` to
  whatever digest happened to be cached on the node — which was
  the install-time digest from 28 days prior, not GHCR's actual
  current `:latest`.

The root failure mode was not the cache. The cache was the
tiebreaker. The root failure mode was that two systems claimed
ownership of `Deployment.spec.template.spec.containers[0].image`
without coordinating, and the loser silently regressed.

Fix — single-writer the surface:

- `values-hetzner.yaml` no longer ships `tag: …`. The field is
  intentionally unset; CD passes `--set image.tag=sha-${SHA}`
  on every deploy and is the sole authority.
- `setup.sh` no longer calls `helm upgrade` for stagecraft. It
  runs `kubectl rollout restart deploy/stagecraft-api` — the
  right verb for "secrets rotated, re-read" — guarded for the
  fresh-cluster case where the deployment doesn't exist yet.
- `imagePullPolicy: Always` is set in `values-hetzner.yaml` as
  defence-in-depth against any future scenario where a mutable
  tag gets reintroduced. With sha-pinned tags this is a no-op;
  with mutable tags it's correctness.
- The chart's `deployment.yaml` now renders `imagePullPolicy`
  from `.Values.image.pullPolicy` (default `IfNotPresent`).
  Previously the field was absent entirely, leaving it to K8s
  admission defaults — a non-deterministic surface.

Generalisation (the rule, not the instance):

> Any cluster-state surface — image tag, replica count,
> resource limits, ingress host, secret reference — must have
> **exactly one writer** by design, not by hope. If two systems
> can write the same field, document which is authoritative,
> remove the other's write path, and fail-fast at install time
> if both try. "Both writers happen to agree today" is not a
> design; it's a latent bug.

Same shape as L-001: a documented manual step (setup.sh's
helm-upgrade) silently undid the automation (CD's sha-pinned
tag). Same shape as the 2026-05-07 helm-deploy field-manager
incident: `kubectl set image` and `helm upgrade` both wrote
`Deployment.spec.template.spec.containers[0].image`, and the
field-manager metadata desynchronised. Three incidents, one
class. Worth treating "single-writer per cluster-state surface"
as a platform-wide invariant going forward, not a per-incident
fix.

Affected scope:

- Stagecraft chart + setup.sh (fixed in the same commit as this
  amendment).
- `deployd-api` chart + setup.sh helm-upgrade for deployd-api
  almost certainly carries the same shape (setup.sh:347-354).
  Not fixed here to keep blast radius bounded; tracked as
  follow-up. Same review for any future per-environment
  values-*.yaml that ships an `image.tag`.

**L-004 — `expose: false` is "internal to the Encore service",
not "internal to the cluster".** FR-010's K8s CronJob curls
`http://stagecraft-api.stagecraft-system.svc.cluster.local:80/internal/knowledge/orphan-imported-sweep`
expecting the in-cluster service hop to be the auth boundary.
But Encore's `expose: false` means "this route is not bound to
the public HTTP server at all" — Encore's API gateway returns
404 to any external HTTP caller, regardless of whether the
caller is inside the cluster or outside. The route is callable
only via direct function import inside the same Encore service.
A K8s CronJob is an external HTTP caller relative to the Encore
process boundary, not the K8s namespace boundary. Hence every
sweep run 404s; reconciliation never executes.

Three viable fixes (none chosen on this branch):

1. Flip the handler to `expose: true` and validate the request
   through the existing platform M2M auth surface
   (`api/auth/m2mAuth.ts::validateM2mRequest` — OIDC
   `client_credentials` JWT verified via Rauthy JWKS,
   scope-gated). This matches spec 087 Phase 5's M2M design,
   wired today on `policy.ts` (`platform:policy:read`),
   `audit.ts` (`platform:audit:write`), `grants.ts`
   (`platform:grants:read`), and `deployd-api-rs`
   (`DEPLOYD_REQUIRED_SCOPE`); the static-token fallback that
   would be the "smallest diff" form of this option was
   deliberately removed in Phase 5, so any "shared bearer
   token via Secret" framing regresses against settled M2M
   policy. **Rauthy 0.35 nuance (load-bearing, see L-006 and
   FU-006):** the `client_credentials` flow ignores the
   `scope=` request parameter — Rauthy mints whatever is in
   the client's *Default Scopes* (not *Allowed Scopes*).
   Required scopes for `client_credentials` callers MUST be
   added to the client's *Default Scopes*; placing them only
   in *Allowed Scopes* is silently inert under this flow.
   The K8s CronJob fetches a token from Rauthy
   `/auth/v1/oidc/token` (client_credentials grant) using a
   **per-purpose** Rauthy client (e.g.
   `stagecraft-knowledge-sweeper-m2m-app`) whose *Default
   Scopes* carry exactly `platform:knowledge:sweep`, with
   credentials mounted as a K8s Secret; the CronJob then
   POSTs to the handler with `Authorization: Bearer <jwt>`.
   One Rauthy client per sweeper purpose (FU-003 inherits
   the pattern: `stagecraft-factory-sweeper-m2m-app`,
   `stagecraft-audit-sweeper-m2m-app`, etc.) bounds a
   leaked credential to exactly that sweeper's surface —
   defense in depth at the credential layer, not only at
   the validator. Scope vocabulary follows the established
   `platform:<service>:<verb>` shape:
   `platform:knowledge:sweep`, `platform:factory:sweep`,
   `platform:audit:sweep`, etc.
2. Run the sweep work in-process via a tiny helper service that
   the K8s CronJob doesn't need to reach over HTTP — e.g. a
   sidecar container in the stagecraft pod that imports
   `runOrphanSweep` directly and is triggered by the K8s
   CronJob via `kubectl exec`. Awkward; introduces a sidecar
   pattern that has no current platform precedent — no service
   in `platform/charts/` ships a sidecar today, so this option
   sets a new infra primitive's precedent for one cron.
3. Bind a second HTTP listener inside Encore restricted to a
   private port, expose the internal routes there, and have
   the K8s CronJob curl that port. Closest in intent to the
   original `expose: false` framing. **Deferred pending
   framework-feature verification** — initial scoping assumed
   Encore exposes a second-listener primitive; reading the
   service's `encore.app` config and grepping the codebase
   surfaced no such primitive, and no service in this repo
   registers a second listener. Not eliminated on merit; the
   option becomes implementable if a future Encore release
   adds the primitive (or if verification of the current
   release surfaces one we missed).

Follow-up tracker (parking lot):

- **FU-001 — Orphan-sweeper expose:false fix.** Pick one of the
  three options above; amend FR-010 with the chosen approach;
  ship as a fast-follow against this spec. Validation script's
  job-exit-code check (added on this branch) will turn green
  the moment the chosen fix lands.
- **FU-002 — deployd-api dual-writer fix (same shape as L-003).**
  `setup.sh:347-354` calls `helm upgrade --install deployd-api`
  with `-f values-hetzner.yaml`; CD's `cd-deployd-api-rs.yml`
  separately deploys deployd-api with sha-pinned tags. Same
  pattern as L-003. Plus: `deployd-api-78ffc9b57-fg2th` is
  OOM-killing on a 512Mi memory limit during hiqlite WAL init
  (exit code 137, observed 2026-05-08). Fix needs both the
  single-writer cleanup AND a memory bump, and should ship in
  one commit so the recovery is verifiable end-to-end.
- **FU-003 — Generalised L-001 amendment for other affected
  sweepers.** Spec 115 FR-006 (`extraction-staleness-sweeper`,
  every 1m), spec 087 §4.4 (`connector-sync-scheduler`,
  every 15m), and spec 124 (`factory-runs-staleness-sweeper`)
  carry the same Encore-CronJob-self-hosted-no-op bug. Each
  owner spec needs its own K8s CronJob amendment. Best landed
  as a single sibling spec that amends all three rather than
  three separate amendments, to keep the systemic finding
  visible and the gates aligned.
- **FU-004 — `validate/spec-143.sh` fixture hardening.** The script
  passes all PREREQUISITE checks today (DNS, TLS, ingress
  reachability, CORS preflight) but its CONTRACT section has three
  independent fixture bugs that fire even when the deployable
  contract is green — see §13 for the authoritative evidence that
  the contract is green despite the script's red. Fix all three in
  one pass; until that lands, a CONTRACT-section red is **not** a
  regression signal on this spec's deployable promises.

  (a) **Project picker can hit an invalid bucket name.** The
      `ORDER BY created_at ASC LIMIT 1` query at lines 135-136
      picks the oldest project regardless of whether its
      `object_store_bucket` is valid for S3. On the Hetzner
      cluster (2026-05-08) the oldest project's bucket is 80 chars
      (`oap-stagecraft-ing-cfs-emergency-family-violence-services-funding-request-portal`),
      exceeding S3's 63-char ceiling; `mc share upload` errors with
      "Bucket name cannot be longer than 63 characters" and
      `contract_fail` fires. Fix at the script level: add
      `WHERE length(object_store_bucket) <= 63` to the SELECT (and
      ideally `AND name NOT ILIKE '%test%'` so the canary picks a
      stable non-fixture bucket). The underlying production bug —
      stagecraft creating projects with over-long buckets in the
      first place — is filed as FU-005 on spec 087.
  (b) **Presigned URL generator is the wrong shape.** Line 236-238
      uses `mc share upload`, which returns a multipart **POST**
      form (HTML form-upload contract); line 256-263 then runs
      `curl -X PUT --data-binary` against the host-substituted URL.
      POST and PUT signatures are not interconvertible — even with
      a working bucket the test would 403 `SignatureDoesNotMatch`,
      and the script would raise that as an FR-006a failure when
      the actual cause is fixture shape. Fix: drive presigning via
      a real SigV4 PUT — `aws s3 presign` (AWS CLI), boto3, or the
      Encore `requestUpload` endpoint with a real session. The
      current stand-in produces misleading diagnostics that point
      at FR-006a (Host preservation) when the real failure is the
      fixture itself.
  (c) **`BLOB_LANDED` check uses a brittle filesystem path.**
      Lines 278-280 test `[ -f /export/${TEST_BUCKET}/${TEST_KEY} ]`
      inside the MinIO pod. MinIO RELEASE.2024-12-18's on-disk
      layout does not expose objects at that simple path —
      manually confirmed during the 2026-05-08 validation: a
      successful upload (`HTTP 204` + server etag) lands in MinIO
      and is visible via `mc ls --recursive` at the expected key,
      yet the filesystem `[ -f ... ]` check returns absent. Fix:
      use `mc stat local/${TEST_BUCKET}/${TEST_KEY}` inside the
      pod (exit 0 = present, !=0 = absent) for the canonical
      answer. The current check would false-fail even on a
      successful upload, raising "blob did not persist" when the
      blob is in fact persisted.

  **Done when:** (a)(b)(c) above are fixed AND, in the same PR,
  §13 is re-titled "Evidence ledger (historical record)" with its
  trailing stage-out clause (the "When FU-004 lands…" paragraph)
  removed. The script's CONTRACT exit code becoming trustworthy
  is the event that retires §13's "manual trace is authoritative,
  script is known-broken" framing; landing the script fix without
  the §13 edit produces a spec that has both a green CONTRACT and
  a note saying don't trust it, and the next reader cannot tell
  which is current. The two edits are one atomic change.

- **FU-006 — Audit M2M Default Scopes across all platform
  clients.** L-006's discovery is platform-wide: every M2M
  `client_credentials` caller whose validator checks a
  non-default scope is latently broken under Rauthy 0.35.

  (a) **`stagecraft-m2m` carries `deployd:deploy` in *Allowed
      Scopes* but only `openid` in *Default Scopes*.**
      `deployd-api-rs::has_scope` (`auth.rs:70`) requires
      `deployd:deploy` exactly; under `client_credentials` the
      JWT will only carry `openid`, so deployd-api would 403
      every M2M call. Verified 2026-05-09 — no production
      traffic has exercised this path. Fix: extend
      `seed-rauthy.mjs` to converge `deployd:deploy` into
      `stagecraft-m2m`'s *Default Scopes*, not just *Allowed*.
  (b) **Same shape applies to `policy.ts`
      (`platform:policy:read`), `audit.ts`
      (`platform:audit:write`), `grants.ts`
      (`platform:grants:read`).** No production callers
      today, so latently broken rather than actively broken;
      these will fail at first use unless their respective
      M2M clients carry the required scope in *Default
      Scopes*.
  (c) **`deployd-api-rs`'s JWT validator is RSA-only**
      (`auth.rs` looks for the `n` field on the JWK).
      Rauthy 0.35 signs with EdDSA (Ed25519, JWK shape
      `{kty: OKP, crv: Ed25519, x: ...}` — no `n`). Verified
      2026-05-09 with a JWT carrying `scope: "openid"`
      (irrelevant to this layer): deployd-api returned
      `401 Missing n in JWK` at the JWKS layer, before scope
      check. Even after fixing (a), deployd-api will not
      accept the JWT without EdDSA support in `auth.rs`
      matching the EdDSA branch in stagecraft's
      `m2mAuth.ts:111-126`. Both fixes needed for the
      deployd-api M2M path to function end-to-end.

  **Done when:** (a)+(b) the affected M2M clients have their
  required scopes in *Default Scopes*; (c) `deployd-api-rs`
  handles EdDSA JWKs alongside RSA; AND a smoke test (one per
  affected validator) confirms an end-to-end M2M call returns
  200 for a valid scope and 403 for a missing scope — not 401
  at the JWK or scope-claim layer. Best landed as a single
  sibling spec covering the systemic finding rather than
  per-client fixes, per FU-003 precedent.

- **FU-008 — setup.sh secret-sync granularity.** Spec 143's
  FU-001 verification surfaced a recurring structural seam:
  on Hetzner-without-ESO, materialising any per-purpose M2M
  Secret requires running the full `setup.sh` monolith. This
  is the second of four hits on the same seam pattern across
  this spec's lifecycle:

  - *Hit #1 (Rauthy-seam, already filed)* — L-005/L-006:
    spec 143 originally inferred Rauthy 0.35's behaviour from
    OAuth2 protocol generality (FR-008 DNS-01 assumption,
    `scope=` param semantics). Resolved by L-005/L-006
    naming the empirical Rauthy 0.35 invariants verbatim.
  - *Hit #2 (this filing)* — setup.sh-monolith seam:
    `platform/infra/hetzner/setup.sh` interleaves pre-flight
    checks (lines 95-219), helm chart installs (Rauthy at
    201-217, infrastructure helm chain after 173 calling
    post-create.sh), `kubectl create secret` calls
    (rauthy-secrets at 184-191, deployd-api-secrets at
    194-198, stagecraft-api-secrets at 318-343,
    `stagecraft-knowledge-sweeper-credentials` at 354-359, S3
    creds at 339-343), stagecraft-api rollout (368-374),
    deployd-api helm upgrade (377-384), and GitHub Actions
    secret sync (389-408) into a single sequential script.
    To rotate or add **one** Secret — e.g. the new
    sweeper-credentials triple introduced by FU-001/FU-003 —
    operators must re-run the entire script, which (a) does
    far more than the secret rollout requires, (b) reliably
    triggers FU-009's cronjob deletion as a side effect (twice
    observed in the 2026-05-09 verification session: once on
    Beat 5→6 transition, once accidentally between Beat 6.4
    fire and Beat 6.4 pod-exec verify), (c) couples credential
    rollouts to chart-level state changes (stagecraft-api
    rollout) that are unrelated to the credential being
    rotated.
  - *Hits #3 and #4 (incoming with FU-003)* — when FU-003
    lands the spec 115 / 087 / 124 sweepers, each will need
    its own per-purpose `stagecraft-{factory,audit}-sweeper-
    credentials` Secret materialised the same way. Without
    a structural fix, each new sweeper inherits the same
    monolith-re-run requirement. The seam compounds; this is
    why hits #3 and #4 are not "more of the same kind" but
    "the same seam under load."

  Two candidate shapes — neither pre-committed:

  (a) **Idempotent setup.sh subcommand targets.** Refactor
      `setup.sh` into composable targets matching the
      operational unit of work — e.g. `setup.sh sync-secrets`
      (re-materialises every `kubectl create secret generic …`
      from the present `.env`, no helm work, no rollouts,
      no GitHub Actions sync), `setup.sh sync-rbac`,
      `setup.sh sync-app` (helm upgrades and rollouts),
      `setup.sh full` (the present monolith, calling all
      targets). Per-purpose credential rollouts then become
      a single narrow command. Shape inherits the existing
      script's `.env`-as-source-of-truth invariant; no
      cluster-side controller required.

  (b) **Deploy-time secret materialisation hook on the
      Hetzner-without-ESO branch.** Move per-purpose Secret
      materialisation out of `setup.sh` entirely and into a
      Helm chart hook (or sister Job) that reads from a
      cluster-side source of truth — e.g. a sealed-secret
      bundle, an SOPS-encrypted manifest, or a Hetzner-
      cluster-local `bootstrap-secrets` ConfigMap+Secret
      pair populated once by setup.sh. The Helm release
      then owns secret lifecycle the same way ESO would on
      cloud clusters — chart drift is the contract; setup.sh
      stops being the per-rotation rollout vehicle. Open
      question: source-of-truth durability — `.env` on the
      operator's laptop vs. cluster-internal store — is a
      separate trade-off worth naming.

  *Cross-FU constraint with FU-009.* The post-create.sh
  legacy-delete (FU-009) currently lives in the same script
  layer as the sweeper-Secret materialisation (this FU). Both
  candidate shapes here have implications for FU-009's
  resolution: shape (a) needs an answer for which target the
  legacy-delete belongs to (or whether it stays at the
  orchestrator level); shape (b) makes the legacy-delete
  homeless and forces FU-009 toward its candidate (b) —
  extraction to a one-time migration script. The two FUs
  should be resolved together, with FU-009's shape decision
  following FU-008's.

  **Decision needed:** which shape (a or b, or hybrid) the
  Hetzner-without-ESO path should adopt before FU-003 lands
  hits #3 and #4. The spec must be present before the next
  per-purpose M2M Secret rollout, otherwise the seam re-fires
  with each FU-003 sweeper. Companion feedback memory
  `feedback_setup_sh_secret_sync_granularity.md` cross-links
  this spec entry; that memory captures the agent-side
  recurrence pattern (the agent tried to materialise the
  missing Secret out-of-band on first encounter, was
  correctly halted by the auto-mode classifier; the
  structural fix removes the temptation entirely).

  **Done when:** the Hetzner-without-ESO secret rollout has
  a single composable command path (whatever shape (a)/(b)/
  hybrid resolves to); FU-003's incoming sweeper rollouts
  use that path; the `.env`/cluster-side source-of-truth
  durability question has its own resolution recorded.

- **FU-009 — post-create.sh legacy-delete ordering safety.**
  `platform/infra/hetzner/post-create.sh:419-422` runs
  `kubectl delete cronjob knowledge-orphan-imported-sweeper
  --ignore-not-found=true` unconditionally on every invocation.
  The comment at lines 388-417 claims this "runs BEFORE the
  helm release lands so a cluster carrying the legacy
  un-Helm-owned CronJob has it cleared in time for the
  Helm-owned successor to be created cleanly." That ordering
  premise holds only on the first cluster bootstrap. On
  subsequent setup.sh re-runs against a cluster where CD has
  already deployed the Helm-owned successor (the FR-010
  cronjob), the delete fires AFTER the Helm-owned cronjob
  exists and removes it — leaving the cluster in a degraded
  state until the next CD run rebuilds it.

  Validated empirically twice in the 2026-05-09 verification
  session for FU-001:

  - First firing: setup.sh re-run between Beat 5
    (cronjob present, verified) and Beat 6 (manual fire
    expected) deleted the cronjob; CD re-run
    [25586826114](https://github.com/stagecraft-ing/open-agentic-platform/actions/runs/25586826114)
    (workflow_dispatch, 01:01:09Z helm upgrade,
    01:01:38Z cronjob recreated) restored it.
  - Second firing: setup.sh accidentally re-run between
    Beat 6.4 manual fire (01:02:32Z) and Beat 6.4 pod-exec
    verify (01:27:29Z) deleted the cronjob again.
    Cluster-truth and chart-truth diverged: helm release
    151 still carried the manifest (verified via `helm get
    manifest stagecraft --revision 151 | grep CronJob`),
    cluster API returned NotFound. No further restoration
    attempt was made — the §13 2026-05-09 deferred-evidence
    gap captures the close conditions.

  Two candidate shapes — pick at fix time:

  (a) **Label-gate the delete.** Wrap the kubectl delete
      in a check on `app.kubernetes.io/managed-by`: delete
      only if absent or != "Helm". A cluster carrying the
      Helm-owned successor will see the delete skip; a
      cluster carrying the legacy un-Helm-owned cronjob
      will see the delete fire. Cheapest fix; adds one
      jq/grep wrapper. Drawback: relies on the legacy
      cronjob never having that label (it didn't, by
      construction — the legacy was raw kubectl-applied,
      not Helm-managed).

  (b) **Extract the legacy-bootstrap retirement to a
      one-shot migration script.** Move the delete out of
      post-create.sh into a single-use `migrations/2026-05-08-retire-legacy-orphan-sweeper.sh`
      (or equivalent), run once per pre-FR-010 cluster,
      then removed from post-create.sh entirely once every
      live cluster has crossed the cutover. Cleaner
      long-term — the legacy-bootstrap retirement is a
      one-time historical concern, not a perpetual hook.
      Drawback: requires tracking which clusters have
      completed the migration (manual today; would warrant
      cluster-side state if formalised).

  **Done when:** the post-create.sh legacy delete is either
  label-gated (per (a)) OR extracted to a one-time migration
  script and removed from post-create.sh entirely (per (b));
  AND a setup.sh re-run against a cluster carrying the
  Helm-owned cronjob no longer destroys it. Smoke-test:
  re-run setup.sh; `kubectl get cronjob knowledge-orphan-imported-sweeper
  -n stagecraft-system` continues to return the same
  `metadata.uid` as before the re-run.

- **FU-010 — Chart cronjob template uses `client_secret_post`
  body params + §12 L-007.** The cronjob template at
  `platform/charts/stagecraft/templates/cronjob-orphan-sweeper.yaml:35-41`
  authenticates to Rauthy's token endpoint with
  `curl --user "${CLIENT_ID}:${CLIENT_SECRET}"` (HTTP Basic
  Auth, RFC 6749 §2.3.1 `client_secret_basic`). Rauthy 0.35
  rejects this for confidential `client_credentials` clients
  with **HTTP 400 BadRequest `'client_secret' is missing`** —
  the same rejection class as L-006's "Default Scopes vs
  Allowed Scopes" finding: where OAuth2 RFC 6749 permits
  multiple shapes, Rauthy 0.35 has narrowed to one and rejects
  the others.

  Verified 2026-05-09 against the live Rauthy instance:

  - HTTP Basic via `curl --user "${CID}:${CSEC}"` against
    `${RAUTHY_URL}/auth/v1/oidc/token` (chart's deployed
    shape) → **HTTP 400** `'client_secret' is missing`.
  - Body-form via `curl --data-urlencode "client_id=${CID}"
    --data-urlencode "client_secret=${CSEC}"` (same endpoint,
    same credentials) → **HTTP 200** with a 536-char Bearer,
    `expires_in: 1800` (Beat 6.4 evidence line A in §13
    2026-05-09 entry).

  **Fix:** Replace `--user "${CLIENT_ID}:${CLIENT_SECRET}"`
  in the cronjob template with body-form params:
  `--data-urlencode "client_id=${CLIENT_ID}"` plus
  `--data-urlencode "client_secret=${CLIENT_SECRET}"`.
  FU-003's incoming spec 115/087/124 sweepers inherit the
  corrected shape from day one. The existing
  `--data-urlencode scope=platform:knowledge:sweep` line
  stays — its body-inert semantics under L-006 are documented
  in the chart comment and the comment stays current.

  **§12 L-007 to land alongside the chart fix** (verbatim,
  not pre-committed in this filing):

  > **L-007 — Rauthy 0.35 client-auth-method invariant.**
  > OAuth2 RFC 6749 §2.3.1 permits confidential clients to
  > authenticate via either HTTP Basic (`client_secret_basic`)
  > or request-body params (`client_secret_post`). Rauthy
  > 0.35 has chosen `client_secret_post` for confidential
  > `client_credentials` flows and rejects Basic with
  > `400 BadRequest "'client_secret' is missing"`. Same class
  > as L-006: when the OAuth2 spec permits multiple shapes,
  > Rauthy picks one and rejects the others — verify Rauthy's
  > choice empirically rather than reading the OAuth2 spec
  > for permission. The verify-don't-infer discipline applies
  > to any protocol surface where the implementation has
  > narrowed the spec's permissiveness.

  **Done when:** (a) `cronjob-orphan-sweeper.yaml:35-41`
  uses body-form `client_secret_post`; (b) §12 L-007 lands
  verbatim alongside the chart fix; (c) CD stagecraft
  redeploys; (d) smoke-test: a manual fire (`kubectl create
  job --from=cronjob/knowledge-orphan-imported-sweeper`)
  reproduces Beat 6.4 evidence line A's HTTP 200 token
  fetch. Sweep-endpoint closure (Beat 6.4 evidence line B's
  401) is **not** in this FU's done-when — that is FU-011's
  scope and tracked there.

- **FU-011 — M2M validator correctness across platform
  services + §12 L-008.** Spec 143 FU-001 verification
  surfaced a third Rauthy-0.35-vs-hand-rolled-validator
  finding, structurally adjacent to FU-006c (deployd-api
  RSA-only validator). The seam is platform-wide:
  hand-rolled M2M JWT validators across stagecraft-api,
  deployd-api-rs, and any future sibling Rust/TypeScript
  service each carry their own independent reimplementation
  of "what is the issuer / how do I match it / what alg
  do I support." Subtle divergence is the recurring failure
  mode; this FU files the seam with a concrete trigger and
  three numbered findings, only one of which is the spec
  143 closure gate.

  **Concrete trigger (Finding 1 — stagecraft-api issuer
  derivation):** `validateM2mJwt` at
  `platform/services/stagecraft/api/auth/m2mAuth.ts:85-89`
  derives `expectedIssuer` by string concatenation:
  `` `${rauthyUrl()}/auth/v1` ``. With
  `RAUTHY_URL=https://auth.stagecraft.ing` (the cluster's
  mounted secret), this evaluates to
  `https://auth.stagecraft.ing/auth/v1` — no trailing slash.
  Rauthy 0.35's OIDC discovery doc publishes
  `issuer: "https://auth.stagecraft.ing/auth/v1/"` *with*
  a trailing slash, and `client_credentials` access tokens
  carry that exact string in the `iss` claim (verified
  2026-05-09 by base64url-decoding the Beat 6.4 evidence
  line B token: `iss: "https://auth.stagecraft.ing/auth/v1/"`).
  Strict-equality compare → false → `return null` →
  `m2mAuth.ts:53` throws `"invalid or expired M2M JWT"`.
  This is the 401 in §13 2026-05-09 Beat 6.4 evidence
  line B.

  The same service has a sibling validator that does it
  right: `validateJwt` in
  `platform/services/stagecraft/api/auth/rauthy.ts:201`
  fetches the canonical issuer via OIDC discovery
  (`getJwksAndIssuer()`, defined at `rauthy.ts:152`) and
  compares against the discovery-doc-published `issuer`
  string. Two validators five lines apart in the same
  directory derive issuer two different ways. The bug *is*
  the divergence.

  **Fix (named, not leaned).** Replace `m2mAuth.ts:85-89`'s
  string concatenation with a call to `getJwksAndIssuer()`
  (already exported from `rauthy.ts:152`) and compare against
  the returned `issuer`. Mirror the pattern
  `rauthy.ts::validateJwt:201` already uses; m2mAuth.ts
  catches up to rauthy.ts. The decode evidence makes this
  the only defensible fix: trailing-slash normalisation
  would normalise the symptom while preserving the
  divergence that produced it, and any future Rauthy-version
  issuer-shape change re-fires this exact class of bug. The
  authoritative-by-deference-to-the-discovery-doc shape is
  the actual contract; making the validator match the
  contract is the structural fix.

  *JWKS-failure behaviour discipline.* `getJwksAndIssuer()`
  has its own error modes (network, cache miss, discovery-
  doc shape change). The M2M path must explicitly decide
  JWKS-failure behavior; do not inherit by default. Audit
  `rauthy.ts::validateJwt`'s current swallow/throw choice
  on JWKS error and either mirror it explicitly or improve
  on it in `m2mAuth.ts` — the explicit decision is the
  point.

  *Latent — `M2mClaims.sub` null handling.* The decoded
  Beat 6.4 evidence line B token carried `sub: null`
  (Rauthy's standard for `client_credentials`, where there
  is no end-user principal). The TS interface at
  `m2mAuth.ts:19-25` declares `sub: string`. Today the
  validator returns null on issuer mismatch before any sub
  consumer runs, so the type/runtime mismatch is harmless.
  Once Finding 1 lands and execution flows past the issuer
  check, audit downstream consumers of `M2mClaims.sub` for
  null-handling. This is a one-line audit-note, not a spec
  gate; called out here so a "fix-Finding-1-then-immediately-FU-012"
  sequence is avoided.

  **Finding 2 — deployd-api-rs RSA-only JWK validator
  (cross-reference to FU-006c).** `deployd-api-rs::has_scope`
  via `auth.rs` accepts only RSA JWKs (looks for the `n`
  field). Rauthy 0.35 signs with EdDSA (Ed25519, JWK shape
  `{kty: OKP, crv: Ed25519, x: …}` — no `n`). FU-006c's
  existing text and verification (2026-05-09, JWT carrying
  `scope: "openid"` returned `401 Missing n in JWK` at the
  JWKS layer before scope check) stays where it is in
  FU-006; this is now part of the FU-011 seam framing as
  cross-reference, not duplicated text.

  **Finding 3 — audit step (seam-closure work).** Walk every
  M2M JWT validator in the platform — at minimum
  `stagecraft-api`'s two validators (`m2mAuth.ts` +
  `rauthy.ts`), `deployd-api-rs`'s `auth.rs`, and any
  sibling service that receives M2M-authenticated traffic —
  and surface every place that derives issuer, fetches JWKS,
  or selects signature algorithm by hand-rolled code rather
  than by deference to the OIDC discovery doc. Findings 1
  and 2 are two examples of this surface; the audit closes
  whether there are more. Without the audit step, the seam
  framing is just text; the audit is the work that makes
  "platform-wide M2M validator correctness" a finite,
  closeable contract.

  **§12 L-008 to land alongside Finding 1's fix** (verbatim,
  not pre-committed in this filing):

  > **L-008 — Rauthy 0.35 issuer-claim invariant (use OIDC
  > discovery, not string concatenation).** Rauthy 0.35's
  > `client_credentials` access tokens carry `iss` exactly
  > as published in the OIDC discovery doc at
  > `/auth/v1/.well-known/openid-configuration` — currently
  > `https://<host>/auth/v1/` with trailing slash.
  > Hand-rolled validators that derive `expectedIssuer` by
  > string concatenation (e.g. `` `${rauthyUrl()}/auth/v1` ``
  > without trailing slash) will mismatch the token's `iss`
  > and reject otherwise-valid M2M tokens. Same class as
  > L-006 (Default vs Allowed Scopes) and L-007
  > (client-auth-method invariant): when Rauthy publishes a
  > contract through OIDC discovery, defer to the discovery
  > doc as the source of truth — do not infer the contract
  > from protocol generality or from the values you used in
  > configuration.

  **Done-when, tiered:**

  - *Tier 1 — spec 143 FU-001 closure gate.* Finding 1
    fixed (m2mAuth.ts uses `getJwksAndIssuer()`'s
    discovery-canonical issuer); Beat 6.4 evidence line B
    flips from HTTP 401 `"invalid or expired M2M JWT"` to
    HTTP 200; FU-010 has also landed (so the chart's token
    fetch produces a Rauthy-valid token in the first
    place); CD stagecraft redeploys; ≥ 1 full scheduled
    tick fires successfully under the schedule
    `*/30 * * * *`. Spec 143 closes when Tier 1 lands.
  - *Tier 2 — FU-011 closure gate.* Finding 1 cleared (per
    Tier 1); Finding 2 cleared (deployd-api-rs `auth.rs`
    handles EdDSA JWKs alongside RSA, per FU-006c's
    existing done-when); Finding 3 audit completed and any
    additional latent validators it surfaces are filed and
    resolved; §12 L-008 lands verbatim. FU-011 stays open
    until Tier 2 lands. The audit is the seam-closure work
    — if it is not in FU-011's done-when, the seam framing
    is just text.

The honest-state principle: when an FR's contract is broken in
production but the implementation is structurally close to
working, mark it partially-implemented in the spec body rather
than carrying a clean "implemented" status that misrepresents
the cluster's actual behaviour. A spec that lies about its own
state corrodes the audit trail — the value of the spec spine is
the trust that markdown matches truth.

- **FU-013 — `confirmUpload` 502 / `requestUpload` 503 under
  concurrent batch upload load — leading cause: stagecraft-api
  OOMKill (FU-002 pattern).** Spec 143 FU-001 deploy-time
  verification surfaced a partial-failure mode in the user-driven
  upload flow under FR-006's 34-file concurrent batch shape.
  Three observation windows captured:

  - 2026-05-09 ~07:30 MDT (image 2): subset of files in a
    34-file batch returned `Upload landed but confirm failed
    (502)` with HTML body. Initial framing: 502 + HTML
    upstream-proxy signature suggests ingress timeout.
  - 2026-05-10 01:21:47 UTC (image 6): subset of files in a
    fresh batch returned `Failed to request upload (503)` with
    HTML `<title>503 Service Temporarily Unavailable</title>`
    body. 503 from nginx = no ready upstream.
  - 2026-05-10 01:31:54 UTC (image 9): subset of files in a
    fresh project batch returned `Failed to request upload
    (401)`. Distinct cause — see FU-016, not FU-013.

  *Cause #1 confirmed by post-restart cluster-state check
  (2026-05-10 ~01:24 UTC).* `kubectl get pods -l
  app=stagecraft-api`: `RESTARTS=1`, `Last State: Terminated,
  Reason: OOMKilled, Exit Code: 137`. The previous container
  died at 2026-05-10T01:20:35Z under a memory limit of
  **512Mi**. Pre-OOM logs (`kubectl logs --previous`) show the
  34-file batch storm: ~12 concurrent `requestUpload` + ~12
  concurrent `confirmUpload` + extraction-enqueue events firing
  in a 4-second window (01:20:26 → 01:20:33), with auth handler
  durations climbing 13.9ms → 491ms → 904ms → 1321ms as memory
  pressure built. A `healthz` probe at 01:20:32 returned
  `code: unknown` after 1115ms; container died ~2s later.
  **This is the L-003 / FU-002 pattern, applied to
  stagecraft-api: same 512Mi limit, same exit 137, same
  load-pressure trigger.** Image 6's 503s land ~70s into the
  recovery window before the readiness gate re-flipped —
  symptoms of the OOM, not a separate cause. Image 2's earlier
  502s were not log-correlated at the time but have the same
  shape and likely belong to the same pattern.

  *§13 closure entry record discipline.* The §13 2026-05-09
  ~07:35 UTC Tier 1 closure entry's "stagecraft-api pod stable:
  0 restarts in 18h" bullet was honest-as-of-07:35-UTC. The
  freshness clock ticked: ~18h later (01:20:35 UTC) the OOM
  happened. Per §12 L-004 honest-state principle, that prior
  entry stays unamended — the new evidence lands as the §13
  2026-05-10 ~01:34 UTC entry below.

  *Hypothesis ranking, revised.*

  (1) **stagecraft-api OOMKilled under FR-006 batch load**
      *(confirmed leading cause)* — concrete evidence above.
      Fix gated on **FU-015** (memory bump + concurrency review).

  (2) **Ingress `proxy_read_timeout` on synchronous MinIO
      `headObject` under concurrent load** *(secondary, may
      also contribute under load short of OOM)*. Even with the
      memory bump in (1), if `confirmUpload`'s server-side path
      runs `headObject` synchronously, it could exceed the 60s
      `proxy_read_timeout` under MinIO load. To verify or rule
      out: post-FU-015, repeat the 34-file batch and inspect
      whether residual 502s remain. If they do, FU-013 stays
      open on (2).

  (3) **File-type-specific failure path in `confirmUpload`**
      *(tertiary)*. Image 2's type pattern is plausibly hiding
      the OOM-recovery pattern (larger files = longer pipeline
      = more memory in flight = first-killed under OOM). Likely
      not a real distinct cause; if (1) and (2) both clear and
      type-correlated 502s persist, revisit.

  *Self-healing absorbs the user-visible failure today.* The
  Class B orphan-sweeper is firing on schedule per the §13
  2026-05-09 Tier 1 closure entry. Rows whose `confirmUpload`
  502'd land in `imported` state and the sweeper picks them up
  on the next tick. FU-013 stays **investigation-priority** at
  the user-visible level, but the underlying cause (FU-015 OOM)
  is **service-stability priority** and should be tackled first.

  *Done when:* (i) FU-015 has landed and the 34-file batch
  repro produces no OOMKill; (ii) any residual 502s post-FU-015
  are diagnosed against hypothesis (2) and either fixed or
  determined absent; (iii) a 34-file concurrent web-UI upload
  batch produces zero `confirm failed` AND zero `Failed to
  request upload (503)` rows.

- **FU-014 — Knowledge-route refresh-500: `r.completed_at.toISOString
  is not a function` in `listKnowledgeObjects`.** *Closed 2026-05-10
  — see Resolution sub-section below.* Spec 143 FU-001
  deploy-time verification surfaced a 500 on hard-refresh of the
  project knowledge route (`app/project/<uuid>/knowledge`). First
  navigation rendered cleanly (image 1); hard-refresh returned
  "An unexpected error occurred" — the Remix / React Router
  error-boundary catch shape on a loader throw (images 3, 7, 10).

  *Cause confirmed by log diagnosis (2026-05-10 01:23:13.439Z).*

  ```
  {"code":"internal","endpoint":"listKnowledgeObjects",
   "error":"an internal error occurred:
            r.completed_at.toISOString is not a function",
   ...}
  Error: {"code":"internal","message":"an internal error occurred",...}
      at apiFetch (.../web/build/server/index.js:7475:9)
      at async loader$16 (.../web/build/server/index.js:7607:27)
      at async callRouteHandler (.../react-router/...)
  ```

  Server-side bug in `listKnowledgeObjects`: a row's
  `completed_at` column is not a Date object — handler calls
  `.toISOString()` on a non-Date and throws. Reproduces across
  three independent refreshes on three different project URLs
  (images 3, 7, 10).

  *Reframe vs prior FU-014 hypotheses.* The prior FU-014 stub
  ranked three possible causes — (a) loader auth-cold-cache,
  (b) recent-migration column shape, (c) SSR URL building.
  **All three were wrong.** Actual cause is a server-side
  type-mapping bug in the API handler, not an SSR-layer issue.
  Hypothesis (b) was closest in spirit (column-shape mismatch)
  but specifically about a missing column, not a
  timestamp-typed-as-string mismatch.

  *FU-014 is in spec 143 scope.* `listKnowledgeObjects` is the
  user-facing list endpoint for FR-006's knowledge-objects
  table — directly touched by the upload → confirmUpload →
  state-transition flow this spec owns. The "may close as not
  spec 143" framing in the prior stub is retracted.

  *Suspect concrete root causes (one of the three).*

  (a) **`completed_at` column type drift.** A migration may
      have changed the column from `timestamptz` to `text`
      (compare against spec 142's factory-id-columns-text-cutover
      precedent). Handler's `r.completed_at.toISOString()`
      assumes a Date; if the column is text, `.toISOString` is
      undefined.

  (b) **Sweeper or `confirmUpload` writing `completed_at` as a
      string.** If either path sets `completed_at` via raw SQL
      with a string literal or an ORM path that doesn't preserve
      Date typing, subsequent reads return string. The Class B
      sweeper writes state on every tick; if it's the writer,
      every tick produces more poisoned rows.

  (c) **Encore.ts driver returning `timestamptz` as string under
      certain conditions.** Less likely — Encore's PG driver
      typically maps `timestamptz` to `Date` — but worth
      verifying with a one-row check against the live cluster.

  *Diagnostic moves (root-cause phase).*

  - Inspect column type:
    `kubectl exec postgresql-0 -- psql -U stagecraft -d
    knowledge -c "\\d knowledge_objects"` for column types;
    `SELECT id, completed_at, pg_typeof(completed_at) FROM
    knowledge_objects LIMIT 3` for runtime type.
  - Grep `listKnowledgeObjects` handler for `.toISOString()`
    callsites in `platform/services/stagecraft/api/knowledge/`.
  - Grep `orphanSweeper.ts` and `confirmUpload` for
    `completed_at` writes — if either uses raw SQL with a
    string literal, that's the writer.

  *Done when:* the cause is identified, fixed in spec 143
  scope (likely a one-line guard in the handler — e.g.
  `r.completed_at ? new Date(r.completed_at).toISOString() : null`
  — paired with a column-type or writer-path correction), and
  a refresh of the knowledge route after a populated upload
  batch returns the list cleanly.

  *Resolution (2026-05-10).* Root cause was none of (a)/(b)/(c)
  above. The actual cause is a **fourth shape: asymmetric typing
  between `db.execute<>(sql\`raw\`)` and Drizzle's typed
  `db.select(...)` in the same file.** `knowledge.ts:191-217`
  (the list path, broken) used `db.execute<{ ..., completed_at:
  Date | null, ... }>(sql\`...\`)` — but `db.execute<>()` returns
  `timestamptz` columns as **string** at runtime; the TypeScript
  generic is a compile-time-only assertion that does not invoke
  Drizzle's Date mapping. `r.completed_at` was a string;
  `.toISOString()` is undefined on string; handler threw
  TypeError; the Remix loader at
  `web/app/routes/app.project.$projectId.knowledge.tsx` (no
  catch wrapper) propagated the error to the route boundary
  and rendered "Oops! An unexpected error occurred." The
  parallel single-row path at `knowledge.ts:282-303` uses
  `db.select(...)` with Drizzle's typed select, which does
  invoke the Date mapping — that path returned Date and worked.
  The asymmetry is the source of the bug.

  *Why the existing test did not catch it.* The fixtures in
  `listKnowledgeObjects.integration.test.ts` already seeded a
  populated `completed_at` value for `OBJ_A` (line 88,
  `'2026-05-07T11:00:02Z'`), but the assertions at lines
  132-134 only checked `.status` and `.durationMs`. The
  `.completedAt` field was asserted only for the **null** case
  (OBJ_B, line 138). The populated-completedAt path was data-seeded
  but not assertion-covered — so the bug shipped while the
  test was green.

  *Fix.* `knowledge.ts:191-217` — change the `db.execute<>`
  generic to declare `completed_at: string | null` (matching
  runtime), wrap the consumer in `new Date(r.completed_at).toISOString()`,
  and add a load-bearing comment at the type declaration
  naming the asymmetry between `db.execute()` and `db.select()`
  for future readers. The comment is the discipline — the
  next person who writes `db.execute<{ … : Date }>` in this
  codebase has a fighting chance of remembering the conversion.

  *Audit step.* Grep across `platform/services/stagecraft/api/`
  for other `db.execute<{ ... Date ... }>` callsites: **no
  other instances**. Bug isolated to `knowledge.ts:191-217`.
  This is the same shape as FU-011 Finding 3's "audit step"
  pattern — when a class of bug is found, the audit *is* the
  close, not a separate follow-up. Audit result lands here so
  the next reader knows the surface was checked.

  *Test added.* `listKnowledgeObjects.integration.test.ts`
  gains a `completedAt` ISO 8601 assertion on `OBJ_A`'s
  populated path
  (`expect(...completedAt).toBe("2026-05-07T11:00:02.000Z")`)
  with a comment naming the prior coverage gap. The
  null-completedAt assertion on OBJ_B is preserved.

- **FU-015 — stagecraft-api OOMKilled under FR-006 34-file
  batch load (mirrors FU-002 for deployd-api).** Spec 143
  FU-001 deploy-time verification (§13 2026-05-10 ~01:34 UTC
  entry) surfaced an OOMKill on stagecraft-api at
  2026-05-10T01:20:35Z, exit 137, under a memory limit of
  **512Mi**. Trigger was a user-driven 34-file concurrent
  upload batch through the stagecraft web UI — the FR-006
  happy path under realistic load.

  *This is the L-003 / FU-002 pattern, on stagecraft-api.*
  FU-002 records the same shape on deployd-api: 512Mi memory
  limit insufficient under hiqlite WAL init pressure, OOMKill
  exit 137. stagecraft-api carries the same 512Mi limit and
  exhibits the same OOM behaviour under a different load shape
  (concurrent uploads + concurrent extraction enqueues rather
  than WAL init).

  *Concrete evidence pinned.* `kubectl get pods` shows
  `RESTARTS=1, LAST_REASON: OOMKilled, LAST_EXIT: 137`.
  Pre-OOM logs (`kubectl logs --previous`) show ~12 concurrent
  `requestUpload` + ~12 concurrent `confirmUpload` + multiple
  `enqueueExtraction` events in a 4-second window (01:20:26 →
  01:20:33), with auth handler latency climbing 13.9ms → 491ms
  → 904ms → 1321ms as memory pressure built. Liveness probe
  returned `code: unknown` after 1115ms at 01:20:32.637;
  OOMKill ~2s later.

  *Three fixes needed (mirrors FU-002 plus a V8-heap leg the
  initial stub did not anticipate; root-cause investigation
  documented in §13 2026-05-10 ~07:48 UTC entry).*

  (a) **Cgroup memory limit raised on the Deployment.** 512Mi
      is empirically insufficient. Raise to 1Gi. FU-002 named
      "single-writer cleanup AND a memory bump, in one commit
      so recovery is verifiable end-to-end" — apply the same
      shape here.

  (b) **`NODE_OPTIONS=--max-old-space-size=896` set on the
      container.** The 2026-05-10 06:58:55Z restart was exit
      139 (V8 "Reached heap limit"), NOT exit 137 (cgroup
      OOM-killer): V8 hit its ~256MB default
      `max-old-space-size` ceiling before the cgroup engaged.
      Raising cgroup alone does not move the V8 ceiling — Node
      20's auto-detection from cgroup is not active on this
      image. Set `NODE_OPTIONS` explicitly so V8 uses the new
      headroom. Budget math in §13 2026-05-10 ~07:48 UTC.

  (c) **Literal-integer `maxConcurrency: 4` on the extraction
      Subscription.** `extractionWorker.ts:35-46` explicitly
      omits `maxConcurrency` (Encore parses Subscription config
      at build time and accepts only literal integers, not
      constants). Combined with `extractionCore.ts:796`
      `getObject` buffering the full file body into a Node
      `Buffer`, parallel extraction workers stack file bodies
      in V8 heap unbounded under FR-006 batch fan-out. Set the
      literal (justification in §13). This is the only leg
      that bounds memory regardless of batch size — survives
      64- or 128-file batches that the cgroup + V8 bump alone
      would not.

  *Cross-references.* FU-013's leading cause (cause #1) is
  this OOM. FU-013 done-when (i) gates on this FU landing.
  L-003 (cluster-state surfaces with one writer) and FU-002
  (deployd-api) carry the same pattern; the lesson is no
  longer a one-off — it's a platform pattern. Worth a §12
  lesson update on "512Mi memory-limit pattern in platform
  services" once FU-015 and FU-002 are both closed. FU-021 is
  conditional on a post-FU-015-deploy retro check on
  deployd-api (see §13 2026-05-10 ~07:48 UTC); FU-020 is an
  optional reusable load harness.

  *Done when:* (a) cgroup memory limit raised on stagecraft-api
  Deployment; (b) `NODE_OPTIONS=--max-old-space-size=...` set
  on the container; (c) extraction Subscription has a
  literal-integer `maxConcurrency`; (d) a 34-file concurrent
  web-UI upload batch completes without OOMKill (cluster
  validation); (e) three CI static assertions land —
  Subscription `maxConcurrency` literal, chart memory limit
  ≥ 1Gi, chart `nodeOptions` `--max-old-space-size` sane
  against cgroup. A real load harness is filed as FU-020
  (optional follow-up); not part of FU-015's done-when.

- **FU-016 — Mid-batch session-cookie loss → 401 on
  `requestUpload`.** Spec 143 FU-001 deploy-time verification
  (image 9, 2026-05-10 01:31:34 → 01:31:48 UTC) surfaced a
  distinct failure mode: in a fresh browser-session upload
  batch that had earlier `done` rows, mid-batch requests
  started returning `Failed to request upload (401):
  {"code":"unauthenticated","message":"No authentication
  token provided"}`. **Not** the OOM-recovery pattern —
  `RESTARTS=1` since 01:20:35Z and unchanged through 01:34:25Z.

  *Concrete evidence (auth handler debug log).*

  ```
  {"cookieHasSession":false,"endpoint":"auth",
   "hasAuthorization":false,"hasCookie":true,
   "level":"warn",
   "message":"auth handler: no token in Authorization header
              or __session cookie",
   ...}
  ```

  Translation: the request carries a Cookie header, but among
  the cookies the `__session` cookie is **absent**. No
  Authorization header either. Auth handler returns
  `unauthenticated`; `requestUpload` returns 401. Repeated
  across at least four trace IDs in the 01:31:54 → 01:31:58 UTC
  window.

  *Suspect causes.*

  (a) **Session cookie TTL hit mid-batch.** If the `__session`
      cookie has a short TTL (5–15 min) and there's no refresh
      mechanism on the browser side during a long upload run,
      the browser drops it mid-stream. A 34-file batch of
      large transcripts can take several minutes.

  (b) **Browser tab/session state divergence.** The user may
      have closed/reopened the browser between batches or
      navigated through a logout-flow page. Same browser
      `uid: 5dcf6f54-…` appears in successful and failed
      windows, so this is plausible only if the cookie was
      cleared client-side.

  (c) **Server-side session store wiped by the OOM restart.**
      If stagecraft-api maintains an in-memory session store
      rather than stateless JWT-only sessions, the OOM at
      01:20:35Z would have wiped it — but the log message says
      the cookie itself isn't present in the request, not that
      it's present-but-invalid. So (c) is less likely.

  *Diagnostic moves.* Inspect `__session` cookie in the browser
  DevTools at the moment of failure (present? expiry?); inspect
  the cookie set on initial login for its TTL; grep auth handler
  in `platform/services/stagecraft/api/auth/` for session-refresh
  logic and the cookie-issuing path's `Max-Age` / `Expires`.

  *Done when:* cause identified; long-running upload batches no
  longer 401 mid-stream; a 34-file batch that takes ≥ N minutes
  (where N exceeds the prior cookie TTL) completes without 401s.

**L-005 — Spec assumptions about deployment topology must be
verified, not inferred from the cloud-platform name.** Spec 143's
original FR-008 mandated DNS-01 via a Hetzner cert-manager webhook on
the assumption that "Hetzner cluster" implied "Hetzner DNS as the
authoritative provider for the cluster's domain." It does not. The
authoritative DNS for `stagecraft.ing` is at Cloudflare and was never
migrated to Hetzner DNS during cluster bootstrap. The DNS-01 webhook
block at `post-create.sh:106-188` was added but never fired — gated
on `HCLOUD_DNS_API_TOKEN`, which was never set — and the MinIO
ingress annotation pointed at a `letsencrypt-dns01` ClusterIssuer
that consequently never existed. Implementation surfaced this only
when the helm upgrade was actually attempted on 2026-05-08:
`kubectl get clusterissuer` showed `letsencrypt-prod` (HTTP-01) only.

Generalisation: cluster-bootstrap topology — authoritative DNS
provider, ingress controller class, cert-manager solver chain,
identity provider — is *separately* configurable from the cloud
platform that hosts the cluster. Spec FRs that pin behaviour against
one of these surfaces MUST verify the surface against the running
cluster (or against the bootstrap script that provisions it), not
infer it from the cloud-platform name. Same shape as L-002:
deployment-target alignment is a spec-review checklist item, not an
inference. Add to the §6.4-style review checklist used at FR-review
time: "for every infra-touching FR, name the authoritative provider
of every external surface the FR depends on (DNS, identity, object
storage, secrets), and cross-check against the bootstrap script."

Affected: FR-008 relaxed from "MUST DNS-01" to "MUST DNS-01 when the
authoritative DNS provider supports a cert-manager webhook AND
wildcard or DNS-only validation is required; HTTP-01 acceptable for
single-host non-wildcard certs once the parent domain's ingress is
already routing." See §4.7 for the topology rationale and §7 step 7
for the implementation path. Future authoritative-DNS migrations
re-activate the dormant DNS-01 path without code resurrection.

**L-006 — External-service behavior must be verified against the
service's own documentation, not inferred from spec/protocol
generality.** OAuth 2.0 generally permits `client_credentials`
callers to request specific scopes via the `scope=` request
parameter. Rauthy 0.35's documented behavior is the opposite:
the `client_credentials` flow *cannot* request specific scopes —
only `authorization_code` can — and Rauthy mints whatever is in
the client's *Default Scopes* regardless of what was requested or
what is in the client's *Allowed Scopes*. This is a documented
divergence from protocol generality, not a bug.

Implementation surfaced this only when token claims were inspected
on the live cluster (2026-05-09 re-investigation, prompted by an
earlier inference that "policy/audit/grants are in production use,
validating M2M JWTs successfully" — a claim the live cluster
falsified): every `client_credentials` token-mint returned
`scope: "openid"` regardless of the `scope=` parameter, scope-object
registration state, or attribute-mapping presence. Adding
`platform:knowledge:sweep` to the sweeper client's *Default Scopes*
caused the JWT to immediately include it; no other intervention
moved the needle. The Rauthy admin doc names this behavior
explicitly: "Default Scopes are the ones that Rauthy will simply
always add … only the `authorization_code` flow can request
specific scopes while all others can't." We did not read that
sentence before designing against the OAuth2 generality of
`scope=`.

Same shape as L-005 (don't infer authoritative DNS from
cloud-platform name): don't infer service-specific behavior from
protocol generality. Add to the §6.4-style review checklist used
at FR-review time: "for every external service the FR depends on,
name the service's specific behavior for the integration mode in
use, not the protocol's general behavior — and confirm the
configuration the FR will rely on matches that documented
behavior."

Generalisation: any FR that asserts "the client requests scope X
via the OAuth2 token endpoint" against an unspecified IDP MUST
cite the IDP's documentation for the flow being used, not OAuth2
generally. Rauthy is one data point; other IDPs differ in
different ways. Concretely for this codebase: the assumption that
`oidcM2m.ts::fetchClientCredentialsToken` could request a scope
via the `scope=` URL parameter and have it land in the issued
JWT was empirically wrong under Rauthy 0.35; the
required-scope-into-Default-Scopes pattern is the platform
convention going forward. See FU-006 for the audit of currently
mis-configured M2M clients (`stagecraft-m2m`'s `deployd:deploy`
chief among them) and the `deployd-api-rs` EdDSA-vs-RSA JWK
shape mismatch surfaced in the same investigation.

Affected: §12 L-004 Option 1 (refined 2026-05-09 to specify
*Default Scopes, not Allowed Scopes*); FU-006 (new follow-up
filing the platform-wide audit).

**L-007 — Rauthy 0.35 client-auth-method invariant.**
OAuth2 RFC 6749 §2.3.1 permits confidential clients to
authenticate via either HTTP Basic (`client_secret_basic`)
or request-body params (`client_secret_post`). Rauthy
0.35 has chosen `client_secret_post` for confidential
`client_credentials` flows and rejects Basic with
`400 BadRequest "'client_secret' is missing"`. Same class
as L-006: when the OAuth2 spec permits multiple shapes,
Rauthy picks one and rejects the others — verify Rauthy's
choice empirically rather than reading the OAuth2 spec
for permission. The verify-don't-infer discipline applies
to any protocol surface where the implementation has
narrowed the spec's permissiveness.

Affected: `platform/charts/stagecraft/templates/cronjob-orphan-sweeper.yaml`
(FU-010 fix, body-form `client_id`/`client_secret`); FU-003's
incoming spec 115/087/124 sweepers inherit the corrected
shape from day one. Cross-reference to L-006 (scope inertia)
and L-008 (issuer-claim derivation) — three Rauthy-0.35
invariants in one family.

**L-008 — Rauthy 0.35 issuer-claim invariant (use OIDC
discovery, not string concatenation).** Rauthy 0.35's
`client_credentials` access tokens carry `iss` exactly
as published in the OIDC discovery doc at
`/auth/v1/.well-known/openid-configuration` — currently
`https://<host>/auth/v1/` with trailing slash.
Hand-rolled validators that derive `expectedIssuer` by
string concatenation (e.g. `` `${rauthyUrl()}/auth/v1` ``
without trailing slash) will mismatch the token's `iss`
and reject otherwise-valid M2M tokens. Same class as
L-006 (Default vs Allowed Scopes) and L-007
(client-auth-method invariant): when Rauthy publishes a
contract through OIDC discovery, defer to the discovery
doc as the source of truth — do not infer the contract
from protocol generality or from the values you used in
configuration.

Affected: `platform/services/stagecraft/api/auth/m2mAuth.ts`
(FU-011 Finding 1 fix, `getJwksAndIssuer()` call mirrors
the sibling validator at `rauthy.ts::validateJwt:201`).
FU-011 Finding 2 (`deployd-api-rs::auth.rs` RSA-only JWK)
and Finding 3 (platform-wide M2M validator audit) remain
open under FU-011's Tier 2 closure gate.

## 13. Evidence ledger (historical record)

This section is the historical evidence ledger for spec 143's
deployable contract on 2026-05-08, the date the manual end-to-end
trace was captured. At that date, `platform/infra/hetzner/validate/spec-143.sh`
had known fixture bugs in its CONTRACT section that produced
false-negative regression signals — see §12 FU-004 — so this
ledger was the load-bearing artefact for "is the contract met". With
FU-004 closed (2026-05-09), the script's CONTRACT exit code is now
the authoritative ongoing answer; this ledger remains as the dated
record of the first deployable proof and as a template for future
per-deploy evidence appended below.

**Manual end-to-end POST trace, 2026-05-08 18:02:13 UTC.** A
multipart POST against `https://minio.stagecraft.ing/oap-stagecraft-ing-default/`
(presigned via `mc share upload` inside the MinIO pod, host-substituted
to the public ingress) returned:

- `HTTP/2 204` (No Content — S3-protocol success for form upload).
- `etag: "4ec131f28888de0e8592ae2c27884a3b"` (server-computed
  content hash; matches the body the client uploaded).
- `location: https://minio.stagecraft.ing/oap-stagecraft-ing-default/knowledge/spec-143-validate-trace-1778263332/payload.bin`
  (server-confirmed object key, addressable through the public
  ingress).
- `x-amz-id-2: dd9025bab4ad464b049177c95eb6ebf374d3b3fd1af9251148b658df7ac2e3e8`,
  `x-amz-request-id: 18ADA91496DC886C` — MinIO request acceptance
  proof.
- `vary: Origin`, `vary: Accept-Encoding` — CORS-aware response
  surface present on the data path (not just preflight).
- `strict-transport-security: max-age=31536000; includeSubDomains` —
  HSTS enforced at MinIO behind the public ingress.

The blob was independently verified present via `mc ls --recursive
local/oap-stagecraft-ing-default/`: 22 B at `knowledge/spec-143-validate-trace-1778263332/payload.bin`,
written 18:02:13 UTC. ingress-nginx access log confirmed upstream
success at `10.244.1.197:9000` (the MinIO pod IP) with `0.010s`
upstream latency. The TLS chain on `:443` is `letsencrypt-prod`
issued via HTTP-01, valid `2026-05-08 → 2026-08-06`.

**Promise-to-evidence map.** Spec 143's deployable contract is
green against the following per-FR check:

| Promise | Evidence |
|---|---|
| FR-005 — public TLS ingress at `minio.${DOMAIN}` | TLS handshake on `:443`; `Certificate/minio-tls Ready=True`; cert valid through 2026-08-06 |
| FR-005 — body-size cap at the ingress (FR-011 backstop) | `Ingress/minio` annotation `nginx.ingress.kubernetes.io/proxy-body-size: 1g` set; verified via `kubectl get ingress minio -n stagecraft-system -o yaml` |
| FR-006 — browser PUT lands a blob in MinIO | POST returned 204 + etag; `mc ls --recursive` confirms the object at the expected key |
| FR-006a — Host preserved end-to-end (SigV4) | The signature was signed for the public host and the server validated it (a Host rewrite mid-chain would have produced a 403 `SignatureDoesNotMatch`); MinIO `Server` header shows `MinIO` from the in-cluster pod |
| §4.4 — CORS via MinIO env (`MINIO_API_CORS_ALLOW_ORIGIN`) | OPTIONS preflight returns `Access-Control-Allow-Origin: https://stagecraft.ing` (verified by `validate/spec-143.sh` PREREQUISITE 4 — that step is not affected by FU-004) |
| §4.7 — HTTP-01 via `letsencrypt-prod` | `kubectl get clusterissuer letsencrypt-prod` shows `Ready: True`; `Certificate/minio-tls` lists this issuer; no `letsencrypt-dns01` issuer exists in the cluster (confirms dormant fallback) |

**Pre-checks that ran clean via the validate script.** PREREQUISITE
1-4 of `validate/spec-143.sh` all returned PASS on this run: DNS A
record resolves to `178.104.146.181` (a worker node IP), TLS cert is
Ready, public ingress responds with HTTP-non-000 (proves
reachability), and CORS preflight returns the expected ACAO header.
These four are not affected by FU-004 and remain authoritative
through the script's exit code.

**What is NOT yet evidence-of-record at this date.**

- A real upload through the stagecraft web UI (FR-006 deploy-time
  test). The data path is up; the user-driven flow is the next
  layer of confirmation. The orphan-sweeper 404 (FU-001) is
  orthogonal — uploads succeed regardless; only the sweep step is
  broken.
- A genuine SigV4 **PUT** (not the POST form used above). The
  signature shapes differ; FU-004(b)'s fix produces this. The
  spec's storage.ts code path emits PUT, so a real-traffic PUT
  flowing through stagecraft's `requestUpload` endpoint will be
  the next confirmation; the manual POST trace above proves the
  surrounding chain (DNS, TLS, host preservation, body-size,
  blob persistence) is sound.

**FU-001 cutover deploy-artefact-landed-but-not-end-to-end-verified,
2026-05-09 01:30 UTC.** This entry records the honest state of the
FU-001 self-hosted-scheduler cutover at the close of the verification
session. The deploy-artefact level landed cleanly; the runtime path
is blocked on three new follow-ups (FU-008/009/010) plus FU-011
(the M2M validator correctness seam). Beat 6.6 has no available
destructive action — see below.

**What landed cleanly (deploy-artefact level).**

- Helm-owned `CronJob/knowledge-orphan-imported-sweeper` deployed
  via CD stagecraft run [25586025148](https://github.com/stagecraft-ing/open-agentic-platform/actions/runs/25586025148)
  (push event, headSha `22ce2695`, deploy 2026-05-09 00:29:19Z).
  Restored via re-run [25586826114](https://github.com/stagecraft-ing/open-agentic-platform/actions/runs/25586826114)
  (workflow_dispatch, headSha `cc02ee0e`, deploy 01:01:38Z) after
  setup.sh's post-create.sh hook deleted it (FU-009).
  Manifest verified: `app.kubernetes.io/managed-by: Helm`,
  `spec.oap/id: 143-presigned-upload-public-endpoint`,
  `spec.oap/fr: FR-010`, schedule `*/30 * * * *`,
  `envFrom: [secretRef: stagecraft-knowledge-sweeper-credentials]`
  (sole credential mount, FR-010 per-purpose discipline).
- Rauthy client `stagecraft-knowledge-sweeper-m2m-app` configured
  correctly: `confidential: true`,
  `flows_enabled: [client_credentials]`,
  `default_scopes: [openid, platform:knowledge:sweep]`. L-006
  (Default vs Allowed Scopes) is satisfied for this client.
  Sibling clients `stagecraft-factory-sweeper-m2m-app` and
  `stagecraft-audit-sweeper-m2m-app` carry the same shape for
  FU-003's incoming spec 115 / 087 / 124 sweepers.

**What did not work end-to-end (runtime level).**

- *Beat 6.4 manual fire, evidence line A — Rauthy token fetch with
  corrected `client_secret_post` body-form auth.* `POST
  /auth/v1/oidc/token` with `client_id` + `client_secret` as form
  params returned **HTTP 200** with a 536-char Bearer
  (`expires_in: 1800`). The chart's deployed shape uses HTTP Basic
  Auth via `curl --user`, which Rauthy 0.35 rejects as **HTTP 400**
  `'client_secret' is missing`. The corrected pod-exec
  (`Pod/sweeper-verify-1778290048`, stagecraft-system,
  2026-05-09T01:27:29Z–01:27:30Z, exit 0) used body-form auth and
  confirmed the auth invariant is satisfied at the token-mint layer.
  Filed as FU-010 (chart cronjob template + §12 L-007).
- *Beat 6.4 manual fire, evidence line B — sweep endpoint with the
  line-A Bearer.* `POST stagecraft-api …/internal/knowledge/orphan-imported-sweep`
  with `Authorization: Bearer <token>` returned **HTTP 401**
  `{"code":"unauthenticated","message":"invalid or expired M2M JWT","details":null}`.
  Cause diagnosed by base64url-decoding the token: `iss:
  "https://auth.stagecraft.ing/auth/v1/"` (with trailing slash, from
  Rauthy's OIDC discovery doc) versus stagecraft's `m2mAuth.ts:86`
  hard-coded expectedIssuer `${rauthyUrl()}/auth/v1` (no trailing
  slash). Strict-equality compare → false → `m2mAuth.ts:53` throws
  `"invalid or expired M2M JWT"`. Sibling validator `rauthy.ts::validateJwt`
  uses the discovery-doc canonical issuer correctly via
  `getJwksAndIssuer()`. Filed as FU-011 (M2M validator correctness
  across platform services + §12 L-008), absorbing FU-006c by
  cross-reference.
- *Beat 6.5 scheduled tick.* One real scheduled tick fired at
  2026-05-09T00:30:00Z (cronjob controller event
  `SuccessfulCreate :: Created job
  knowledge-orphan-imported-sweeper-29638110`), running the same
  broken-Basic-Auth chart path as the manual Beat 6.4 fire. Job
  since cascaded-deleted with the cronjob (FU-009 fired). No
  further scheduled-tick evidence is available on this cluster
  until (a) FU-009 lands so setup.sh re-runs stop destroying the
  Helm-owned successor, (b) FU-010 lands so the chart's curl call
  can mint a token, (c) FU-011 Tier 1 lands so the sweep endpoint
  accepts that token, (d) CD stagecraft redeploys, (e) ≥ 1 full
  scheduled tick fires successfully post-(d). Recorded as a
  deferred-evidence gap with named close conditions — these are
  the *Tier 1* close conditions; Tier 2 (FU-011 audit step) does
  not block spec 143 closure.
- *Beat 6.6 legacy-client deletion.* No destructive action is
  available — the legacy `stagecraft-sweeper-m2m-app` Rauthy
  client was never created in this Rauthy instance (verified
  2026-05-09 01:28 UTC against the live admin API: 7 clients
  total, none with that id; only the new per-purpose triple
  `stagecraft-{knowledge,factory,audit}-sweeper-m2m-app` plus
  `stagecraft-m2m`, `stagecraft-server`, `stagecraft-spa`, and
  `rauthy`), meaning there is no rollback path to a pre-FR-010
  sweeper auth flow. Recorded as "no-op — legacy client absent"
  rather than "deferred pending FU-010." Implication for FU-010
  cutover: smoke-test must be fully green before redeploy; there
  is no fallback configuration to revert to.

**Follow-ups filed in this session (§12 above).**

- **FU-008** — setup.sh secret-sync granularity. Second of four
  hits on a recurring structural seam (Rauthy-seam L-005/L-006
  was hit #1; FU-003 factory + audit will be hits #3 and #4).
  Decision needed; two candidate shapes named, pre-committed to
  neither.
- **FU-009** — post-create.sh legacy-delete ordering safety.
  Validated empirically twice in this session (once on the first
  setup.sh re-run between Beat 5 and Beat 6, once on an
  accidental re-run between Beat 6.4 manual fire and Beat 6.4
  pod-exec). Operational fix, two candidate shapes.
- **FU-010** — chart cronjob template `client_secret_post` body
  params + §12 L-007 (Rauthy 0.35 client-auth-method invariant,
  same class as L-006). Smoke-test references Beat 6.4 evidence
  line A on landing.
- **FU-011** — M2M validator correctness across platform
  services + §12 L-008 (Rauthy 0.35 issuer-claim invariant).
  Three numbered findings with tiered done-when: Tier 1
  (Finding 1 — stagecraft-api `m2mAuth.ts` issuer derivation)
  is the spec 143 closure gate; Tier 2 (Finding 2 — FU-006c
  cross-reference; Finding 3 — platform-wide audit) is the
  seam-closure gate. Spec 143 closes when Tier 1 lands; FU-011
  stays open until Tier 2 lands.

**Lesson family — the N-shaped tax of hand-rolled OIDC against
Rauthy's wire shape.** This session surfaced three Rauthy-0.35-vs-
hand-rolled-validator findings inside one spec — L-006 (Default
Scopes vs Allowed Scopes, already filed), L-007 (client-auth-method,
filed with FU-010), L-008 (issuer-claim derivation, filed with
FU-011). The pattern: where the OAuth2/OIDC spec permits multiple
shapes, Rauthy 0.35 has narrowed to one and rejects the others;
hand-rolled validators that infer the contract from protocol
generality, or from the values configured locally, accumulate an
N-shaped tax against Rauthy's published wire shape. The remediation
shape is also recurring: defer to the OIDC discovery doc as the
single source of truth (issuer, JWKS, supported algs, supported
auth methods) rather than re-implementing the contract by hand.
The L-006/L-007/L-008 family is worth its own §12 cross-reference
once a fourth instance lands, so future M2M work inherits the
discipline by default.

This is an *honest-state* §13 entry per the §12 L-004 honest-state
principle: the deploy-artefact and the runtime path are separately
credible, and the spec spine carries that separation verbatim until
the runtime path is end-to-end verified.

**FU-001 Tier 1 closure, 2026-05-09 ~07:35 UTC — FR-006
deploy-time green; sweeper observed firing; Class B self-heal
absorbing observed partial failures.** This entry crystallises
the green gates while the cluster-state evidence is fresh, and
separates what *is* now evidence-of-record from what was newly
observed during the same verification session and filed as
follow-ups. The four bullet groups below are deliberately
distinct surfaces — the next reader should be able to tell
green gates from open ones at a glance.

*Verified post-deploy (this entry).*

- **FR-006 user-driven upload path green.** A 34-file batch
  uploaded through the stagecraft web UI at the project
  knowledge route (`app/project/<uuid>/knowledge`) showed
  live "Uploading N/34" with `done` rows accumulating —
  browser → presigned PUT → MinIO blob → `confirmUpload` →
  row state transition. This is the user-visible confirmation
  §13's 2026-05-08 "What is NOT yet evidence-of-record" entry
  was waiting on. Evidence: image 1 of the verification session.
- **Helm-owned `CronJob/knowledge-orphan-imported-sweeper`
  firing on schedule.** `kubectl get cronjob -n stagecraft-system`:
  `SCHEDULE */30 * * * *`, `AGE 18h`, `LAST SCHEDULE` within
  the trailing 30-min window. `kubectl get jobs -n
  stagecraft-system`: three Completed jobs (`Complete 1/1`,
  5–6s each) at +5m / +35m / +65m before this entry —
  `knowledge-orphan-imported-sweeper-{29639520,29639550,29639580}`.
  This satisfies the "≥ 1 full scheduled tick fires successfully
  post-deploy" Tier 1 close condition step (e) named in §13's
  2026-05-09 01:30 UTC entry.
- **stagecraft-api pod stable.** `kubectl get pods -l
  app=stagecraft-api`: 1 replica, READY=True, **RESTARTS=0**,
  uptime 18h, no `lastState.terminated` reason. The 18h matches
  the CronJob age — the deploy that landed FU-009 / FU-010 /
  FU-011 Finding 1 also brought up the sweeper, and neither
  has destabilised the API pod since.
- **Class B self-heal mechanism live.** §4.5's orphan-sweeper
  contract is now exercised continuously by the every-30-min
  schedule; rows the user-driven flow leaves in `imported`
  state are absorbed on the next tick. The mechanism is no
  longer "deploy-artefact landed" — it is observed-firing.

*Observed but tolerated (filed as FU-013 — see §12).* A subset
of files in the 34-file batch returned `Upload landed but
confirm failed (502)` with an HTML error body. The blob landed
in MinIO; the `confirmUpload` POST died upstream of the Encore
handler (502 + HTML, not Encore JSON). Cause #1 (pod restart /
OOMKilled mid-batch, same shape as L-003 / FU-002) is **ruled
out** by the 0-restart, 18h-uptime check above. Two surviving
hypotheses ranked in FU-013: (a) ingress `proxy_read_timeout`
on synchronous MinIO `headObject` under concurrent load (higher
likelihood — the 502 + HTML signature is upstream-proxy, not
an Encore-handler error shape); (b) file-type-specific path in
`confirmUpload`, with the type-pattern in image 2 plausibly
being a size-pattern under hypothesis (a). The Class B sweeper
is absorbing the failure rows, so FU-013 is investigation-priority,
not a Tier 1 blocker. Evidence: image 2.

*Observed, diagnosis pending (filed as FU-014 — see §12).*
The project knowledge route returned an "unexpected error
occurred" Remix-error-boundary 500 on hard refresh — first
navigation worked (image 1 proves it), refresh threw. Held
pending a retry-stability check; if persistent, log-driven
diagnosis follows. Likely unrelated to spec 143 — the knowledge
route's SSR path is upstream of the upload contract this spec
owns. FU-014 may close as "not spec 143" with a pointer to the
owning spec. Evidence: image 3.

*Tier 2 still open (unchanged by this entry).*

- **FU-011 Findings 2 and 3** — `deployd-api-rs::auth.rs`
  EdDSA-vs-RSA JWK shape mismatch (Finding 2) and the
  platform-wide M2M validator audit (Finding 3). FU-011's
  Tier 2 close gate; spec 143 closes on Tier 1 — Finding 1's
  `m2mAuth.ts` issuer-via-OIDC-discovery fix shipped at
  f08a941 / cc02ee0.
- **FU-002** — deployd-api dual-writer + memory bump (same
  shape as L-003). Unchanged by this entry.
- **FU-003** — Sibling spec for spec 115 / 087 / 124
  Encore-CronJob-self-hosted-no-op corrections. Unchanged.
- **FU-008** — setup.sh secret-sync granularity. Decision
  pending; two candidate shapes named in §12.

*Frontmatter.* The `implementation:` field stays `in-progress`
— Tier 2 work remains. The frontmatter comment is updated in
the same commit to reflect that FU-009 / FU-010 / FU-011
Finding 1 have landed, FR-006 deploy-time confirmation is
verified, and the outstanding-list now reads
FU-002 / FU-003 / FU-008 / FU-011 Tier 2 / FU-013 / FU-014.

The closure narrative for spec 143's FU-001 Tier 1 contract
is complete; spec-level closure waits on Tier 2.

**FU-001 Tier 1 contract holds; stagecraft-api stability
regression observed under FR-006 34-file batch load —
2026-05-10 ~01:34 UTC.** This entry records new evidence that
postdates the §13 2026-05-09 ~07:35 UTC Tier 1 closure entry
above. The Tier 1 contract (sweeper firing on schedule,
deploy-artefacts landed, FR-006 user-driven path green at the
data-path level) is **not retracted**. What is new: the
user-visible upload flow has surfaced four distinct failure
modes under realistic batch load, three of which root in
service-stability or handler-quality regressions that the
Tier 1 closure did not exercise. Per §12 L-004 honest-state
principle, the prior entry stays unamended; this entry is the
evidence-of-record for the post-deploy regression surface.

*Trigger.* User-driven 34-file concurrent upload batches
through the stagecraft web UI on two projects (`test-4-dual`
and `test-5-dual`), 2026-05-09 19:21 → 19:32 MDT
(2026-05-10 01:21 → 01:32 UTC).

*Finding A — stagecraft-api OOMKilled at 2026-05-10T01:20:35Z,
exit 137.* `kubectl get pods -l app=stagecraft-api`:
`RESTARTS=1, LAST_REASON: OOMKilled, LAST_EXIT: 137,
STARTED: 2026-05-10T01:20:35Z`. Memory limit on Deployment:
**512Mi** (same as deployd-api per FU-002 — same OOM-prone
configuration). Pre-OOM logs (`kubectl logs --previous`):
~12 concurrent `requestUpload` + ~12 concurrent
`confirmUpload` + multiple `enqueueExtraction` events firing
in a 4-second window (01:20:26 → 01:20:33); auth handler
latency climbing 13.9ms → 491ms → 904ms → 1321ms; `healthz`
returning `code: unknown` after 1115ms at 01:20:32; container
died ~2s later. **L-003 / FU-002 pattern, on stagecraft-api.**
Filed as **FU-015**. Reframes FU-013: cause #1 (originally
ruled out as of the 07:35 UTC check) is now confirmed leading
cause.

*Finding B — image-6 503s on `requestUpload` and image-2 / 5
502s on `confirmUpload` are symptoms of A.* The 503 (`503
Service Temporarily Unavailable`, nginx) means no ready
upstream. User screenshots at 01:21:47 UTC = ~70s into the
recovery window before the readiness gate re-flipped.
Endpoints currently show 1 ready upstream
(`10.244.1.229:4000`) — recovery is complete. No separate
root cause.

*Finding C — refresh-500 = `r.completed_at.toISOString is not
a function` in `listKnowledgeObjects`.* Server-side bug in the
knowledge-list handler. Log evidence at 2026-05-10
01:23:13.439Z:

```
{"code":"internal","endpoint":"listKnowledgeObjects",
 "error":"...r.completed_at.toISOString is not a function",
 ...}
Error: ... at apiFetch (.../web/build/server/index.js:7475:9)
       at async loader$16 (.../web/build/server/index.js:7607:27)
       at async callRouteHandler (.../react-router/...)
```

Reproduces across three independent refreshes on three
different project URLs (images 3, 7, 10). **FU-014's prior
three hypotheses (a/b/c — auth-cold-cache, missing column,
SSR URL building) were all wrong.** Actual cause is a
type-mapping bug: `completed_at` is not a Date when the
handler expects one. Reframes FU-014: cause confirmed, in
spec 143 scope (the prior "may close as not spec 143" framing
is retracted), root-cause investigation needed (column-type
drift vs sweeper-write vs driver-mapping). FU-014 stays open
at spec 143; root-cause moves named in the amended stub.

*Finding D — mid-batch session-cookie loss → 401 on
`requestUpload`.* Distinct from A/B (no pod restart at the
401 timestamps — `RESTARTS=1` unchanged from 01:20:35Z
through 01:34:25Z). Auth handler debug log at 01:31:54 UTC:
`{"cookieHasSession":false,"hasAuthorization":false,"hasCookie":true,...}`
— the request carries a Cookie header but among the cookies
the `__session` cookie is absent. Reproduces across at least
four trace IDs in a 4-second window (01:31:54 → 01:31:58).
Likely session-cookie TTL hit mid-batch, but cause not yet
diagnosed. Filed as **FU-016**.

*Frontmatter implication.* The `implementation:` field stays
`in-progress`. The frontmatter comment is updated in this
commit to surface the regression and the four FU IDs that
pin it (FU-013 amended, FU-014 amended, FU-015 new, FU-016
new). The Tier 1 closure narrative remains valid for the
sweeper / deploy-artefact / Class-B-self-heal contract; what
this entry adds is "the FR-006 batch-load envelope exposed
service-quality issues that Tier 1 did not exercise." The
spec spine carries that distinction verbatim.

*Cross-reference to §12 lessons.* L-003 (cluster-state
surfaces with one writer) is now hit twice (FU-002
deployd-api, FU-015 stagecraft-api); worth a §12 lesson
update on "512Mi memory-limit pattern in platform services"
once both FUs are closed. Not done in this commit — let the
FU closes drive the lesson.

This is an *honest-state* §13 entry per §12 L-004: the
Tier 1 contract and the post-deploy stability regression are
separately credible, and the spec spine carries that
separation verbatim.

**FU-015 root-cause investigation, 2026-05-10 ~07:48 UTC —
V8-heap-vs-cgroup distinction; extraction Subscription
no-`maxConcurrency`; three-leg fix shape.** Same pod
(`stagecraft-api-5c67dd4544-s9jnm`, `sha-a7f6693`) as the §13
2026-05-10 ~01:34 UTC entry; fresh diagnostic pass surfaced a
second crash AND a handler-level root cause that reframes the
FU-015 fix shape from two legs to three.

*Second crash, exit 139 (V8 heap exhaustion) at
2026-05-10T06:58:55Z.* `kubectl get pod` lastState.terminated:
`exitCode: 139, reason: "Error", finishedAt: 06:58:55Z,
startedAt: 02:06:19Z` (same pod, container two restarts on from
the §13 2026-05-10 ~01:34 UTC entry). `kubectl logs --previous`
ends with V8's pre-abort heap diagnostic:

```
<--- Last few GCs --->

[1:0x1224a000] Mark-Compact 251.5 (258.6) -> 250.9 (258.8) MB,
  pooled: 0 MB, 989.73 / 0.00 ms (average mu = 0.206,
  current mu = 0.005) allocation failure; scavenge might not succeed

<--- JS stacktrace --->

FATAL ERROR: Reached heap limit Allocation failed - JavaScript
heap out of memory
```

Same FR-006 batch shape as Finding A
(`oap-stagecraft-ing-test-6-dual`, parallel `requestUpload` /
`confirmUpload` / `enqueueExtraction`, auth latency climbing
113ms → 1750ms). Different exit code: V8 hit its default
`--max-old-space-size` ceiling (~256MB old-space) BEFORE the
cgroup OOM-killer engaged at 512Mi RSS. Node 20's automatic
heap-from-cgroup detection is not active on this image — V8 was
allocating against its built-in default, not the cgroup limit.

*Handler-level root cause.* `extractionWorker.ts:35-46`
explicitly omits `maxConcurrency` (the comment names Encore's
build-time literal-integer parser as the friction). Combined
with `extractionCore.ts:796` calling `getObject(bucket, key)` —
which buffers the full file body into a Node `Buffer` via
`storage.ts:316-326`'s `chunks.push(Buffer.from(chunk))` loop,
materialising the complete body — parallel extraction workers
stack file bodies in the same V8 heap. Per-run CAS dedupes work
for the same row; the day-aggregate cost gate guards
agent-extractor spend. Neither bounds parallel-batch fan-out
memory.

Memory math under §13 2026-05-10 ~01:34 UTC's observed load
(~12 concurrent extraction workers, 1-4MB blobs):
~44MB worst-case per worker (4MB eager buffer + ~10MB AWS SDK
SigV4 staging + ~30MB extractor work-set: deterministic-pdf
parses pages in-memory; deterministic-docx unzips XML) × 12 =
~528MB extraction-side. Plus ~150MB base process
(Encore.ts runtime, drizzle pg pool, MinIO clients, prompt-cache
singletons). Total ~678MB working set. Greatly exceeds V8's
256MB default — abort is deterministic at ~256MB regardless of
cgroup limit.

*Three-leg fix.*

(1) **Cgroup memory limit raised** —
    `platform/charts/stagecraft/values.yaml`
    `resources.limits.memory: 512Mi` → `1Gi`. Without this, V8
    bump alone gets killed by the kernel exit 137.

(2) **`NODE_OPTIONS=--max-old-space-size=896` set on the
    container** — `templates/deployment.yaml` sources from
    `.Values.nodeOptions`. Budget: cgroup 1024 MiB − V8
    old-space 896 MiB = 128 MiB reserve for V8 new-space
    (~32 MiB), code-space (~10 MiB), Node runtime / libuv /
    native modules (~30 MiB), off-heap library buffers
    (~30 MiB), OS / page-cache headroom (~20 MiB). Standard
    75%-old-space-of-cgroup shape; raise reserve if observed
    RSS approaches the cap, lower the old-space cap before
    raising cgroup further.

(3) **Literal-integer `maxConcurrency: 4` on the extraction
    Subscription** — `extractionWorker.ts` adds the literal.
    Justification: 4 × ~44MB worst-case = 176MB
    extraction-side ceiling. Plus 150MB base + 50MB
    HTTP-handler concurrency = ~376MB working set, ~520MB
    headroom in 896MB old-space. 34-file batch at 4-worker
    fan-out completes in ~9 × 5s = ~45s wall-clock —
    user-tolerable for the async extraction-after-upload path.
    Conservative starting point; raisable to 6-8 once
    empirical headroom is observed (cf. FU-020 below). The
    literal lives in source because Encore's build-time
    parser rejects constant references — justification
    documented here, not via "Encore needs a literal."

*CI regression — three static assertions (not a load harness).*

(i) `extractionWorker.ts` MUST declare a literal-integer
    `maxConcurrency` in the Subscription config. Test reads
    the source and asserts the regex
    `/maxConcurrency:\s*(\d+)\b/` matches with `1 ≤ N ≤ 8`.

(ii) `platform/charts/stagecraft/values.yaml`
     `resources.limits.memory` MUST be ≥ 1Gi (parsed via YAML
     load + Mi/Gi normalization).

(iii) `platform/charts/stagecraft/values.yaml` MUST set
      `nodeOptions` containing `--max-old-space-size=N` where
      `256 ≤ N ≤ (cgroup memory limit in MiB - 64)` so the
      reserve is non-negative.

Pinned in `platform/services/stagecraft/test/spec143-fu015.config.test.ts`.
Done-when (e) of the amended FU-015 stub.

*Optional follow-up — FU-020 — stagecraft-api batch-load
harness for memory-ceiling regression.* Reusable load-test
for FR-006 fan-out (local Encore + MinIO + 34 fixture files +
concurrent batch driver + heap profile capture). Useful for
raising `maxConcurrency` past 4 with empirical evidence rather
than budget math, and for catching memory-ceiling regressions
under future Subscription tuning. Not part of FU-015's
done-when. Optional stub; file when there is concrete demand
to raise the cap or when a second memory-ceiling regression
surfaces.

*Conditional follow-up — FU-021 — deployd-api retroactive
check.* FU-002 documented the same OOM shape on deployd-api
but predates this V8-heap-vs-cgroup distinction. deployd-api
is Rust (no V8 heap leg applies — its OOM was hiqlite WAL
pressure under cgroup), so a retroactive check is likely a
one-line confirmation that the language-appropriate fix shape
is sufficient. Run after FU-015 deploys cleanly:

```bash
kubectl get deployment deployd-api-rs -o yaml \
  | grep -E "(memory:|NODE_)"
```

If the deployment carries an adequate memory bump and (Rust
being Rust) no `NODE_OPTIONS` gap, no FU-021 needed. If a gap
exists — e.g. memory still 512Mi, or a new Rust-side
allocator-tuning seam emerges — file FU-021 against spec 143
§13 with a cross-reference to FU-002 (which lives on its own
spec surface and cannot be amended from spec 143). This check
is the discipline FU-002 deserves before the §12 lesson family
("512Mi memory-limit pattern in platform services")
generalises across two languages.

*Frontmatter implication.* `extractionWorker.ts` joins spec
143's `implements:` list. Primary owner is spec 115; spec 143
amends 115 (frontmatter `amends:`), so spec 133's
amends-aware coupling gate accepts the touch — the explicit
`implements:` entry makes the relationship visible to spec 127
without relying on amends-walking. The `implementation:`
comment is updated to reflect the V8-heap-vs-cgroup discovery
and the three-leg fix shape.

This is an *honest-state* §13 entry per §12 L-004: the FU-015
stub's two-leg framing was not wrong, but incomplete. V8 heap
was the missing leg.

