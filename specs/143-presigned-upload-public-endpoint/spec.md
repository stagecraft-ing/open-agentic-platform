---
id: "143-presigned-upload-public-endpoint"
slug: presigned-upload-public-endpoint
title: Presigned upload public endpoint — browser-reachable object store for direct uploads
status: draft
implementation: pending
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
  - path: platform/charts/stagecraft/templates/deployment.yaml  # §12 L-003 — render imagePullPolicy from values
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
  scripts MUST provision a Kubernetes `CronJob` resource in
  `platform/infra/hetzner/post-create.sh` that calls the internal
  sweep endpoint on the same cadence (`*/30 * * * *`). The K8s
  CronJob is the actual production scheduler for self-hosted
  deployments; the Encore CronJob declaration is local-dev and
  future-Encore-Cloud only. See §4.5 self-hosted scheduler
  amendment for the rationale and empirical evidence.

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

1. Flip the handler to `expose: true` and add an Encore
   middleware-style auth gate (header bearer token shared with
   the K8s CronJob via a Secret). Smallest diff; keeps the
   route exposed to the public surface but rejects callers
   without the secret.
2. Run the sweep work in-process via a tiny helper service that
   the K8s CronJob doesn't need to reach over HTTP — e.g. a
   sidecar container in the stagecraft pod that imports
   `runOrphanSweep` directly and is triggered by the K8s
   CronJob via `kubectl exec`. Awkward; introduces a sidecar
   for one cron.
3. Bind a second HTTP listener inside Encore restricted to a
   private port, expose the internal routes there, and have
   the K8s CronJob curl that port. Requires Encore-runtime
   plumbing that may not exist; closest to the original
   `expose: false` intent.

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

The honest-state principle: when an FR's contract is broken in
production but the implementation is structurally close to
working, mark it partially-implemented in the spec body rather
than carrying a clean "implemented" status that misrepresents
the cluster's actual behaviour. A spec that lies about its own
state corrodes the audit trail — the value of the spec spine is
the trust that markdown matches truth.

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
