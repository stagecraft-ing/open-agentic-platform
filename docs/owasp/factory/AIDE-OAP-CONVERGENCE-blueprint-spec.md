# AIDE → OAP Convergence Blueprint (OWASP ASI 2026-aligned)

> **Purpose of this document.** The two preceding blueprints —
> [`AIDE-VELOCITY-blueprint-spec.md`](./AIDE-VELOCITY-blueprint-spec.md)
> and [`AIDE-VELOCITY-HARNESS-blueprint-spec.md`](./AIDE-VELOCITY-HARNESS-blueprint-spec.md)
> — describe two sibling Government-of-Alberta systems **as they are
> today**. This document translates every load-bearing AIDE capability
> onto OAP patterns and re-grounds each translation in the **OWASP
> Top 10 for Agentic Applications (ASI 2026)** — see
> [`owasp_top_10_agentic_applications_summary.md`](../owasp_top_10_agentic_applications_summary.md).
> That executive summary is the **only** OWASP-side doctrine this
> document relies on. (A longer `_oap.md` companion previously sat
> alongside it; it described an idealised OAP↔OPC architectural
> layering that did **not** match the codebase — for example, it
> treated deployd-api as external when in fact
> `platform/services/deployd-api-rs/` lives inside this repo and is
> wired through stagecraft per spec 136. It was retired during the
> intent-alignment work that produced
> [`AIDE-VELOCITY-OAP-INTENT.md`](./AIDE-VELOCITY-OAP-INTENT.md) and
> is **not** used as a source here.)
>
> **Two corrections to the original framing.**
>
> 1. **OAP is a single-repo substrate, not a layered OAP↔OPC pair.**
>    This repo holds the spec spine, the Rust crates, the OPC desktop,
>    *and* the platform services (stagecraft, deployd-api-rs, Rauthy
>    charts). Treating "OAP" and "OPC" as separate trust planes
>    obscures the load-bearing fact that they share one spec spine
>    and one CI substrate.
> 2. **stagecraft + OPC are not the AIDE replacements.** They are the
>    **substrate that produces (or reverse-engineers into) tenant
>    projects**, and a tenant project is the right unit-of-comparison
>    against AIDE-VELOCITY. The convergence question is therefore
>    *"what does an AIDE-VELOCITY-class tenant project look like
>    when it is born of OAP, or when an existing project is
>    reverse-engineered onto OAP's substrate?"* — not *"how do we
>    rebuild AIDE-VELOCITY inside stagecraft?"*. The
>    [`/Users/bart/Dev1/agent-builder-console`](file:///Users/bart/Dev1/agent-builder-console)
>    project illustrates the reverse-engineering case: it was built
>    without a spec spine, then a spec spine was established inside
>    `agent-builder-console/specs/` after the fact. (That repo's
>    spec-spine implementation predates the current OAP refinements
>    and is illustrative only — not normative for this document.)
>
> The goal is a single reference for OAP engineers asking three
> questions:
>
> 1. *"What is the **substrate-level** OAP equivalent of `<AIDE
>    capability X>`?"* — i.e. what stagecraft/OPC/spec-spine pattern
>    governs it.
> 2. *"What does a **tenant project** inherit from OAP for the same
>    capability, when it is produced by — or reverse-engineered onto
>    — the substrate?"*
> 3. *"Which ASI 2026 control(s) does the answer to (1) and (2)
>    satisfy, and where does it still fall short?"*
>
> **Not a spec.** This is a synthesis doc. Forward-looking moves
> below are written as *candidate spec sketches* — they are intended
> to inform spec authoring under `specs/NNN-slug/`, not to substitute
> for it. Per CONST-005 the spec spine remains the authority.

---

## 1. Convergence frame

OAP is a single substrate that operates on two levels at once:

| Level | What it governs | OAP components | OWASP ASI lens |
| --- | --- | --- | --- |
| **L1 — Substrate (meta-dev)** | OAP's own code — the agentic infrastructure itself | `specs/`, `tools/spec-spine/`, `tools/oap/`, `crates/*`, `platform/services/stagecraft`, `platform/services/deployd-api-rs`, `platform/charts/rauthy`, `product/apps/desktop`, `.claude/` | Internal hardening — does OAP itself satisfy ASI controls in how it builds and ships? |
| **L2 — Tenant project (produced or reverse-engineered)** | Individual projects the substrate produces (via factory-engine adapters such as `aim-vue-node`) or onto which spec-spine is retro-installed (e.g. agent-builder-console). AIDE-VELOCITY is an L2-shaped system built **without** the substrate; this is the level at which most AIDE-style features actually live | Spec spine ported into the tenant repo (`<tenant>/specs/`), factory-emitted scaffolds, OPC connecting *into* the tenant repo via the same cockpit pattern a developer uses against OAP itself, stagecraft's deployd-api governing the tenant's deployments | What the tenant inherits — does the tenant project, by virtue of being substrate-governed, satisfy ASI controls in its own runtime? |

The AIDE blueprints describe an L2-shaped pair (a tenant platform +
its cockpit) that was built outside any substrate of this kind. The
convergence work has two halves:

- **L1 (substrate hardening).** Where OAP's own internals fall
  short of ASI 2026, fix the substrate. The spec coupling gate
  (`127`/`130`/`133`), governance certificate (`102`), supply-chain
  policy gates (`116`), and release attestations (`117`) are the
  load-bearing L1 mechanisms today.
- **L2 (tenant inheritance).** Specify what an OAP-produced or
  OAP-governed tenant inherits — born-with-spec-spine factory
  output, factory-emitted Stop-hook drift gates, identity-bounded
  per-tenant scopes from Rauthy via deployd-api, sandboxed codegen
  for adapter execution. The AIDE-VELOCITY feature set then becomes
  a *shopping list of L2 capabilities* a tenant can require, not a
  rewrite target for stagecraft.

OAP's structural difference from AIDE is not incidental — it is
that the substrate exists at all. Where AIDE relies on disciplined
Express middleware + a `check-docs-sync.mjs` drift script local to
the repo, OAP's spec spine emits machine-truth JSON that fails CI
when authored truth and code diverge — **and that same discipline
can be ported into any tenant repo**. Where AIDE's harness gates
fire at Claude Code Stop-time inside one cloned repo, OAP's gates
fire at compile-time *and* PR-time *and* are reproducible across
every tenant by construction.

---

## 2. Translation matrix — domain by domain

Each subsection follows the same shape:

- **AIDE today** — what the source system does
- **OAP today** — what OAP currently provides (with spec / crate citations)
- **Forward gap** — what feature parity-but-more requires
- **OWASP ASI alignment** — which controls the gap closure answers

### A. Project lifecycle & state machine

**AIDE today.** 8-step chess-clock (`requirements → planning →
architecture → prototyping → development → user_testing →
user_acceptance → deployment`) with `current_actor ∈ {ai, human}`,
turn history, loopback counting, alignment scoring, per-step
revision counters maintained by DB triggers (`bump_step_revision`).
`If-Match: <revision>` + `Idempotency-Key: <uuid>` guard every
write. Gamified anti-cheat via the **Reaper** (single-transaction
CTE-based bulk detection).

**OAP today.** Factory pipeline is a **two-phase engine** (s0–s5
sequential, s6a–s6g fan-out) implemented in `crates/factory-engine`
under spec `075-factory-workflow-engine`. Project as a unit of
governance is spec `119-project-as-unit-of-governance`. Workflow
spec traceability is `118-workflow-spec-traceability`. Stagecraft's
`api/projects/{create,clone,import}.ts` materialise projects through
the substrate (`139-factory-artifact-substrate`).

**Forward gap.**

1. **Per-step actor attribution.** OAP's factory pipeline does not
   yet model `current_actor ∈ {ai, human}` as a first-class field;
   factory runs are agent-initiated. Adding an explicit actor field
   to the `governance-certificate.json` artifact chain (spec 102) +
   a `turn` table on factory_artifact_substrate would make per-step
   provenance auditable in the AIDE sense.
2. **Optimistic-concurrency primitives.** No `If-Match` /
   `Idempotency-Key` headers across stagecraft's mutation surface
   today. Encore.ts middleware can be added once; revision counters
   live naturally in Drizzle migrations.
3. **Alignment scoring + loopback penalty.** OAP's promotion-grade
   mirror (spec `097-promotion-grade-mirror`) is the closest
   analogue but does not score human/agent alignment.

**OWASP ASI alignment.**

- **ASI01 (Goal Hijack)** — Deterministic step grammar prevents an
  injected instruction from rewriting the plan. The spec spine's
  `establishes`/`extends`/`refines` relationship graph (constitution
  §"Spec Relationship Graph") is the strongest form of this — the
  plan is a hash-verifiable artifact, not a free-text prompt.
- **ASI08 (Cascading Failures)** — Bounded fan-out at s6a–s6g + the
  factory engine's two-phase contract gives an explicit ceiling on
  recursive reasoning. Forward gap: explicit `max_iterations` on
  agent loops within a stage.

### B. Identity, auth, and scope

**AIDE today.** RS256 JWT (15-min access cookie, 7-day refresh
rotated on every refresh), double-submit CSRF for browser sessions,
**`X-API-Key`** for agents — *same role enforcement as a browser
session*; the only differentiator at audit time is `auth_source`.
Cross-provider account linking by email. GitHub PAT encrypted
AES-256-GCM at rest.

**OAP today.** Rauthy as production-grade OIDC issuer (spec
`106-rauthy-native-oidc-and-membership`, `107-rauthy-client-redirect-convergence`).
`deployd-api-rs` enforces `DEPLOYD_REQUIRED_SCOPE` on every request.
Tenant environment access gates exist in draft (spec
`137-tenant-environment-access-gates`).

**Forward gap.**

1. **Ephemeral, downscoped tokens at every outbound boundary.**
   AIDE's shared X-API-Key pattern is exactly the "shared long-lived
   identity" ASI03 warns against. OAP should never adopt it. Agent
   tool calls must mint a fresh JWT scoped to the *specific*
   tool-invocation, not the agent session. (Reference architecture
   guide §ASI03 calls this the "Token Downscoping" proxy pattern.)
2. **Attribution parity.** AIDE's `auth_source` field is the right
   primitive — humans and agents share endpoints, attribution
   differentiates them. OAP needs the same field plumbed through
   `crates/agent` IDs and the governance-certificate `signer` block.
3. **Cross-provider account linking** is an unsolved OAP question;
   Rauthy supports multiple IdPs but the linking semantics are not
   yet specified.

**OWASP ASI alignment.**

- **ASI03 (Identity & Privilege Abuse)** — Rauthy + scoped JWTs +
  downscoped per-tool tokens is the textbook ASI03 mitigation.
  Spec 137 (draft) is the load-bearing piece; its implementation is
  next per the README status.
- **ASI09 (Human-Agent Trust Exploitation)** — Cryptographic
  attribution (signer field on the governance certificate, FR-007
  spec 102) means "the agent said it was safe" is independently
  verifiable, not anthropomorphically trusted.

### C. Inter-instance & inter-agent communication

**AIDE today.** **PostgreSQL `LISTEN/NOTIFY`** as a cross-instance
SSE bus (`server/src/sse/cross-instance-bus.ts`) — zero
infrastructure, dedicated connection, origin-ID dedup, capped
exponential backoff. **Three-layer SSE back-pressure**
(project-scoped subscriptions, per-client priority shedding,
memory-pressure eviction). Per-identity concurrency caps to stop
runaway agents. Live admin endpoint `GET /admin/sse-sessions`.

**OAP today.** Encore.ts `Topic<T>` PubSub (at-least-once delivery)
inside stagecraft. Slack + GitHub webhook handlers. No equivalent
of project-scoped SSE subscriptions or live-session introspection
endpoints yet. K8s baseline `network deny` policies in
`platform/k8s/`.

**Forward gap.**

1. **Signed inter-stage manifests in factory-engine.** Every hand-off
   between s0…s5 and s6a…s6g should be signed by the dispatching
   agent's ephemeral key. ASI07's core mitigation is mTLS +
   cryptographic identity assertions between cooperating agents.
2. **Project-scoped event subscription pattern in stagecraft.**
   When OPC subscribes to factory events, it should pass a
   `?projects=<id>` analogue and the server should fan-out only to
   interested clients. Reuse the three-layer back-pressure design
   from AIDE — it is genuinely production-grade.
3. **mTLS by default in-cluster.** Helm charts under
   `platform/charts/` should default to mTLS between services; the
   chart values can opt out for local dev but production reads
   should fail closed.
4. **Per-identity concurrency caps on stagecraft mutations.** The
   AIDE post-incident hardening (commit *"per-identity concurrency
   caps to stop abuse patterns"*) is forward-applicable; without it,
   a rogue agent can hold N stagecraft connections open.

**OWASP ASI alignment.**

- **ASI07 (Insecure Inter-Agent Communication)** — Signed manifests
  + mTLS = canonical ASI07 mitigation.
- **ASI08 (Cascading Failures)** — Project-scoped subscriptions +
  priority shedding bound the blast radius of any single misbehaving
  client.
- **ASI10 (Rogue Agents)** — Per-identity concurrency caps + admin
  live introspection (`/admin/sse-sessions` equivalent) are the
  operational primitives.

### D. Tool / agent action surface

**AIDE today.** Agents act through the same REST API as browsers
(`/api/v1/*`). Tool grammar is implicit in the OpenAPI spec.
**`harness/velo-listener.js` spawns Claude with
`--dangerously-skip-permissions` by default** — `CLAUDE_ALLOWED_TOOLS`
can replace it, but the documentation explicitly calls this out as
"a significant operational risk".

**OAP today.** `crates/tool-registry` (`067-tool-definition-registry`)
defines `ToolDef` trait with permission gates. `crates/agent`
contains the executor + verification + ID generation
(`035-agent-governed-execution`). Permission system + runtime in
specs `049`/`068`. Safety-tier governance: spec `036`.

**Forward gap.**

1. **Per-tool JSON-Schema parameter typing — non-permissive.**
   ASI02's core mitigation is "Schema validation, strong typing,
   and transactional write guardrails". The `ToolDef` trait has
   the shape today; the runtime should reject any tool registration
   whose schema contains `any` or unconstrained `object`. This
   constraint must apply at L1 (every OAP-registered tool) and L2
   (every tenant-side tool the substrate provides).
2. **Default-read-only tool semantics.** Any state-changing tool
   call should be classified as a critical transaction. The
   permission-runtime spec is the natural home for this rule.
3. **No `--dangerously-skip-permissions` default anywhere.** When
   OPC spawns a Claude session (or factory-engine spawns an agent
   subprocess), the **default** must be an explicit allow-list of
   tools derived from the spec the agent is implementing. The
   harness has the right idea — `CLAUDE_ALLOWED_TOOLS` — but the
   wrong default. OAP should invert it.
4. **No raw shell / eval.** ASI02 + ASI05 compound: any path from
   an LLM-generated string to a host `exec` is a structural risk.
   Audit `crates/agent` and `crates/axiomregent` to verify nothing
   bypasses the tool registry with a raw `Bash`/`exec` shortcut.
   Tenant projects must inherit the same audit through factory
   adapter output.

**OWASP ASI alignment.**

- **ASI02 (Tool Misuse & Exploitation)** — Strict JSON Schema +
  write-default-off + transactional isolation = the canonical ASI02
  mitigation stack.
- **ASI05 (Unexpected Code Execution / RCE)** — No raw shell + no
  generated-script execution outside sandboxes (see §G below).
- **ASI10 (Rogue Agents)** — Tool-registry-bound action surface
  means an agent that hallucinates a new tool just gets a
  "tool not found" error rather than ambient host access.

### E. Memory, RAG, and context

**AIDE today.** PostgreSQL for structured state + SharePoint for
artifact storage. AI processing queue (PostgreSQL-backed) processes
files into Markdown shadows. Per-project session UUIDs in
`.claude/state/listener-sessions.json` — Claude Code session
continuity across board events.

**OAP today.** **Unified artifact store** (spec
`094-unified-artifact-store`). **Checkpoint / branch-of-thought**
(spec `095-checkpoint-branch-of-thought`). Session memory
(spec `056-session-memory`). **Governance certificate** records
per-stage artifact SHA-256 hashes (spec 102 FR-007). Knowledge
extraction pipeline (spec `115-knowledge-extraction-pipeline`).
Codebase index (spec `101-codebase-index-mvp`).

**Forward gap.**

1. **Sanitization hooks on memory read/write boundaries.**
   ASI06 prescribes input/output sanitization on both the ingestion
   hook (write) and the retrieval hook (read) of episodic memory.
   Spec 094 + 056 should declare an explicit `pre_write` / `pre_read`
   hook contract. **L1**: applies to OAP's own session memory; **L2**:
   factory-emitted tenants inherit the contract for their own
   memory surfaces.
2. **Temporal decay + weighting.** Authenticated cryptographically-
   signed records (e.g. items whose hashes appear in a governance
   certificate) should outweigh raw ambient conversation history.
   The governance certificate already provides the signing — the
   gap is the *retrieval* logic that respects it.
3. **Stable per-project session continuity in the orchestrator.**
   The harness pattern (per-project session UUIDs persisted across
   listener restarts) is directly applicable to `crates/orchestrator`.
   Currently OAP dispatches per-task; per-project session continuity
   would let agents accumulate institutional memory.

**OWASP ASI alignment.**

- **ASI06 (Memory & Context Poisoning)** — Sanitization hooks +
  signed-record weighting is the doctrine prescription.
- **ASI04 (Agentic Supply Chain)** — Memory items resolved from
  external sources (e.g. RAG over public repos) must pass through
  content-addressed hash checks before ingestion.

### F. Supply chain

**AIDE today.** Dependabot on the harness repo. Redteam pipeline
runs `npm audit` + `secretlint` + ESLint security plugins at
**app-build time** inside `.claude/security/blueteam/`. Templates
ship intentionally vulnerable test fixtures (excluded from
dependabot) for blueteam regression testing.

**OAP today.** **Spec `116-supply-chain-policy-gates`** — `cargo-deny`
+ `pnpm audit` + `npm audit`, blocking from day 0. **Spec
`117-release-artifact-attestations`** — per-target CycloneDX SBOMs
(`sbom-desktop-<triple>.cdx.json`) and aggregate
`open-agentic-platform-release.cyclonedx.json` ship with every
release, plus SHA256 sidecars on every installer. **Spec
`121-claim-provenance-enforcement`** + the
`provenance-validator` crate. **Spec `074-factory-ingestion`**
defines factory contracts as Rust types.

**Forward gap.**

1. **Signed adapter manifests at runtime resolution.** The factory
   has four registered adapters (aim-vue-node, next-prisma, rust-axum,
   encore-react). Each adapter manifest at
   `factory/adapters/*/manifest.yaml` should be content-hash-signed,
   and `factory-engine` should refuse to load any adapter whose
   manifest hash does not match an entry in the spec spine. Today
   the codebase-indexer hashes manifests for the staleness check —
   that hash should become **enforced**, not just observed.
2. **Tool-plugin content addressing.** Same pattern, applied to MCP
   tool plugins resolved by `crates/axiomregent`. ASI04's
   prescription is "Content addressable storage (hashes) and signed
   manifest validation" — the spec spine is precisely the
   git-managed manifest surface that can carry the required hashes,
   and the same pattern ports into a tenant's own spec-spine for
   any plugins the tenant brings.

**OWASP ASI alignment.**

- **ASI04 (Agentic Supply Chain)** — Spec 116/117/121 + signed
  adapter manifests is the full doctrine answer.

### G. Execution isolation / sandboxing

**AIDE today.** Express server runs in a single Azure App Service
container; no ephemeral sandboxes for code generated by AI agents.
The redteam pipeline runs ad-hoc Bash commands. The harness's
`/v5-development` scaffold writes generated code into `./app/`
directly on the developer's machine.

**OAP today.** K8s baseline policies in `platform/k8s/`
(network deny, resource quotas). Helm charts under
`platform/charts/`. **No ephemeral micro-sandbox runtime for
code-gen execution.** The factory pipeline emits code; the code is
not currently executed inside a hardened isolation boundary by the
platform.

**Forward gap.**

1. **Ephemeral micro-sandbox runtime for factory-engine adapters.**
   When `aim-vue-node` (or any future adapter) generates code that
   needs to be exercised (lint, test, build), the execution must
   happen inside a network-isolated, TTL-capped pod (gVisor /
   Firecracker / strict-network-policy K8s pod). ASI05's core
   prescription is "Execution Isolation" — decouple all dynamic
   code blocks into ephemeral micro-sandboxes with hard TTL. This
   gap exists at both L1 (OAP's own factory-engine codegen runs)
   and L2 (a tenant project's own codegen agents).
2. **No host file-system / control-plane access from code-gen
   runtimes.** The pods that execute generated code must not mount
   anything that lets them touch the orchestrator state.

**OWASP ASI alignment.**

- **ASI05 (Unexpected Code Execution / RCE)** — Ephemeral
  micro-sandboxes is **the** ASI05 mitigation. This is currently
  OAP's largest open ASI gap.
- **ASI10 (Rogue Agents)** — `pids_limit`, memory ceilings, and CPU
  throttling on sandbox runtimes prevent a compromised codegen agent
  from spinning up parasite processes.

### H. Audit, governance, and self-verifying artifacts

**AIDE today.** **Immutable `audit_log`** (no DELETE endpoint),
JSONB diff of every mutation, `auth_source` attribution, `api_key_id`
on every row. The **Reaper** runs admin-triggered, detects
gamification abuse via bulk CTEs (`speed_run`, `no_collaboration`,
`burst_turns`, `self_approval`, …). **Redemption** mechanism
forgives without destroying violation records. Custom error
envelope (`{success, error: {code, message, …}}`).

**OAP today.** **Governance certificate** is the load-bearing
primitive — every factory run emits a self-authenticating
`governance-certificate.json` binding requirements hash, frozen
Build Spec hash, per-stage artifact hashes, and a SHA-256 over the
canonical JSON. `make verify-certificate` is the auditor's
independent verifier: **the verifier does not trust the system that
produced the certificate** (FR-007). Policy kernel (spec
`047-governance-control-plane`) provides 5-tier settings merge with
proof chains. Stop-the-line governance enforcement stitching
(spec `098-governance-enforcement-stitching`).

**Forward gap.**

1. **`auth_source` attribution on every certificate stage.** Today
   the certificate binds artifact hashes but does not always record
   whether the stage was driven by an agent or a human. Adding this
   field aligns OAP with AIDE's attribution-by-default model.
2. **Anti-abuse rule library, codified as spec spine entries.**
   The Reaper's eight detection rules
   (`speed_run`/`no_artifact`/`burst_turns`/…) are excellent
   *patterns*. OAP equivalents — applied to agent behaviour rather
   than user gamification — could live as `standards/anti-abuse/`
   specs and fire from a future `factory-engine` post-run hook.
3. **Redemption-style governance.** When a governance certificate
   is rejected, the rejection should not destroy the audit chain.
   AIDE's `cheating_violation` table pattern (idempotent insert +
   paired negative entry + Redemption forgiveness) is portable.

**OWASP ASI alignment.**

- **ASI09 (Human-Agent Trust Exploitation)** — The governance
  certificate's *machine-verifiability* is the structural defence
  against anthropomorphic trust. The auditor does not trust the
  agent's confident summary; they verify the certificate.
- **ASI10 (Rogue Agents)** — Centralized lifecycle orchestration
  (`crates/orchestrator`) + cryptographic per-run certificate =
  a rogue agent cannot self-replicate and erase its trail.

### I. Mechanical drift gates

**AIDE today.** `scripts/check-docs-sync.mjs` is wired both into
CI and into Claude Code `PostToolUse` + `Stop` hooks via
`.claude/settings.json`. Six **mandatory blocking** Stop hooks on
the harness: docs-sync, step-output validation, step-gates,
security-gate, harness-sync, **finding-codified** (the centrepiece
— a CRITICAL finding cannot leave the session without writing
itself into `02-security.md`).

**OAP today.** **Spec coupling gate**
(`127-spec-code-coupling-gate`, amended by `130-spec-coupling-primary-owner`
and `133-amends-aware-coupling-gate`) fails CI when code path
authority drifts. **Spec lint** default-fail-on-warn
(`128-spec-lint-default-fail-on-warn`). **Schema parity walker**
(`125-schema-parity-walker-rebuild`). `make pr-prep` runs the gates
locally. `.claude/rules/{orchestrator-rules,governed-artifact-reads,adversarial-prompt-refusal}.md`
loaded by every orchestrated workflow. **`.githooks/pre-commit`**
is opt-in.

**Forward gap.**

1. **Stop-hook chain inside Claude Code sessions, not just at PR
   time.** The harness's leverage is that gates fire **before any
   commit is made** — at conversation end. OAP can adopt the same
   pattern: wire `make pr-prep`-class checks as PostToolUse +
   Stop hooks in `.claude/settings.json`. The codebase-indexer's
   `check` subcommand is already shaped for this; spec coupling
   check binary already exists.
2. **Codification gate analogue.** For every security or
   correctness finding generated by `axiomregent` /
   `provenance-validator` / `policy-kernel`, require a spec spine
   entry under `standards/security/` (or under the implicated
   spec's `§Constraints`) **before the conversation closes.**
   This is the
   `check-security-finding-in-spec.mjs` equivalent — institutional
   memory at the harness level, mechanically enforced.
3. **Evidence-strength distinction in the coupling gate.** The
   harness's `check-evidence-strength.mjs` distinguishes "config
   file exists" from "config file is actually exercised."
   Translated to OAP: a spec being mentioned in a
   `[package.metadata.oap]` block is a *citation*; a spec's named
   FRs appearing in test assertions is *evidence*. The coupling
   gate could grade strength rather than just presence.

**OWASP ASI alignment.**

- **ASI01 (Goal Hijack)** — Drift gates at agent-decision time are
  the prompt-time defence against an injected instruction asking
  the agent to "just fix the spec to match what I did." This is
  already explicit in `.claude/rules/adversarial-prompt-refusal.md`
  (CONST-005, spec `131-adversarial-prompt-refusal-policy`).
- **ASI06 (Memory & Context Poisoning)** — A poisoned context
  cannot persist its effects if every code edit forces a matching
  spec edit and the spec edit fails the lint gate.

### J. Agent-runtime documents and live API contracts

**AIDE today.** `GET /velocity/claude-md` is served by the
**platform** — the agent's behavioural manual lives next to the
data. `GET /api/v1/docs` serves the live OpenAPI 3.0 spec. The
in-app `ApiPlaybook.vue` fetches the live spec at runtime and
renders a filterable endpoint browser embedded in `VelocityView.vue`
— a human and an agent reading the page get the same data, and
**neither can drift from the live server**.

**OAP today.** `CLAUDE.md` (root + `platform/services/stagecraft/`)
+ `AGENTS.md` + `.claude/rules/*` + the spec spine itself.
`registry-consumer` is the typed read surface
(`103-init-protocol-governed-reads`). The codebase-indexer renders
`.derived/codebase-index/CODEBASE-INDEX.md` as a human-shaped
governed view.

**Forward gap.**

1. **Stagecraft should serve a compiled "agent runtime manifest"
   at a well-known endpoint.** Generated by `spec-compiler`,
   containing the current tool grammar + policy rules + protocol
   excerpts the agent needs to behave correctly. OPC and external
   agents fetch it at startup. The spec spine is *authored* truth;
   the served manifest is its *runtime projection*.
2. **OPC should embed a "spec spine playbook" panel.** Analogous to
   `ApiPlaybook.vue` — a filterable, expandable browser over the
   compiled registry, fetching the runtime manifest above. The
   `crates/factory-engine` desktop panel (spec
   `076-factory-desktop-panel`) is the natural home.
3. **Living OpenAPI for stagecraft.** Encore.ts emits OpenAPI by
   default; surface it at a well-known endpoint and wire OPC's
   playbook to fetch it.

**OWASP ASI alignment.**

- **ASI01 (Goal Hijack)** — A served spec spine extract is harder
  to poison than a copy-pasted runtime doc — agents always read
  the freshest version, signed by the platform.
- **ASI06 (Memory & Context Poisoning)** — Same logic: pinned
  runtime manifest hash means a memory item cannot silently swap
  in a stale or hostile playbook.

### K. UI / OPC differentiation and "de-anthropomorphized" presentation

**AIDE today.** Vue SPA at one URL. Mixed agent + human surface —
the `/velocity` view is the agent's API target *and* the human's
status board. Agent plans render as conversational text in chat
panels. Velocity board uses cards + colour coding for status.

**OAP today.** Tauri v2 + React desktop cockpit (`product/apps/desktop`)
— a **separate trust boundary** from any web surface. Local
workspaces. Git context. Inspect / governance panels. Factory
panel (spec 076).

**Forward gap.**

1. **OPC should render agent plans as deterministic structural
   diffs**, not conversational summaries. ASI09's mitigation is
   "Out-of-band verification and rigid UX guardrails for critical
   actions" — concretely, render plans as structural diffs / action
   graphs (YAML/JSON), not as natural-language summaries that invite
   anthropomorphic trust. This is a load-bearing UX commitment —
   the cockpit must not let a confident-sounding agent talk a human
   into bypassing a gate.
2. **Dual-authorization UX for high-risk actions.** For any
   action that touches a tenant boundary (deployd-api scope,
   tenant-environment-access-gate per spec 137), OPC should require
   distinct M-of-N approvers — *not* the same human approving the
   action they prompted.
3. **Live-session introspection panel.** Analogue to AIDE's
   `GET /admin/sse-sessions` — a panel showing every connected
   agent / scope / event-rate, with a force-disconnect control.

**OWASP ASI alignment.**

- **ASI09 (Human-Agent Trust Exploitation)** — Structural diffs +
  dual auth = textbook ASI09 mitigation.
- **ASI10 (Rogue Agents)** — Live introspection + force-disconnect
  is the operator's defence against a runaway agent.

---

## 3. ASI 2026 × OAP spec spine — coverage matrix

The table reads top-down: each ASI control, the OAP specs that
*already* speak to it, and the load-bearing forward gap.

| ASI control | OAP specs already addressing it | Forward gap |
| --- | --- | --- |
| **ASI01 — Goal Hijack** | `127-spec-code-coupling-gate`, `131-adversarial-prompt-refusal-policy`, `132-constitutional-invariant-freeze`, `130-spec-coupling-primary-owner`, `133-amends-aware-coupling-gate` | Runtime planner-output validation — compile LLM-generated step proposals into a validated intermediate representation before execution. ASI01's "deterministic validation of the orchestration specification spine" maps directly to the spec spine; the missing piece is the *runtime* hook that gates plan execution on spec conformance, not just the build-time hook that gates code on spec conformance. |
| **ASI02 — Tool Misuse & Exploitation** | `067-tool-definition-registry`, `049-permission-system`, `068-permission-runtime`, `036-safety-tier-governance` | Per-tool JSON-Schema strictness (reject `any`/unconstrained `object`); default-read-only semantics; spawn-time tool allowlists everywhere. |
| **ASI03 — Identity & Privilege Abuse** | `106-rauthy-native-oidc-and-membership`, `107-rauthy-client-redirect-convergence`, `137-tenant-environment-access-gates` (draft) | Ephemeral downscoped tokens at every outbound boundary; `auth_source` attribution carried on every certificate stage. |
| **ASI04 — Agentic Supply Chain** | `116-supply-chain-policy-gates`, `117-release-artifact-attestations`, `121-claim-provenance-enforcement`, `074-factory-ingestion` | Signed adapter manifests enforced at runtime resolution (currently observed, not enforced). Tool-plugin content addressing. |
| **ASI05 — Unexpected Code Execution / RCE** | `072-multi-cloud-k8s-portability` (resource isolation primitives); K8s baseline `network deny`, resource quotas | **Primary gap.** No ephemeral micro-sandbox runtime for factory-engine adapter codegen execution. Largest open ASI item. |
| **ASI06 — Memory & Context Poisoning** | `094-unified-artifact-store`, `095-checkpoint-branch-of-thought`, `056-session-memory`, `115-knowledge-extraction-pipeline`, `102-governed-excellence` (hash-anchoring) | Pre-write/pre-read sanitization hooks; signed-record retrieval weighting; pinned runtime-manifest hash. |
| **ASI07 — Insecure Inter-Agent Communication** | `106`, `107`, K8s `network deny`, `075-factory-workflow-engine` (two-phase contract) | Signed inter-stage manifests inside factory-engine; mTLS default in `platform/charts/`. |
| **ASI08 — Cascading Failures** | `075-factory-workflow-engine` (bounded fan-out), `098-governance-enforcement-stitching`, `100-post-convergence-remediation` | Explicit `max_iterations` ceilings on agent reasoning loops; circuit breakers at the orchestrator stage boundary. |
| **ASI09 — Human-Agent Trust Exploitation** | `102-governed-excellence` (machine-verifiable certificate), `032-opc-inspect-governance-wiring-mvp`, `041-checkpoint-restore-ui` | Deterministic structural-diff UI for plans (cockpit-side); M-of-N dual-auth UX for tenant-boundary actions. |
| **ASI10 — Rogue Agents** | `052-state-persistence`, `067-tool-definition-registry`, `043-agent-organizer`, `057-notification-system` | Strict parent-child control tree with resource quotas enforced by the orchestrator; live-session introspection panel in OPC. |

The "**Primary gap**" tag on ASI05 is deliberate. Every other ASI
control has at least one spec doing structural work today; ASI05's
mitigation (ephemeral sandbox isolation) does not yet have a spec
home in OAP. Closing that gap is the highest-leverage forward move
the convergence implies.

---

## 4. Candidate spec sketches (forward-looking)

These are deliberately *sketches*, not full specs. Each becomes a
real spec under `specs/NNN-slug/spec.md` when work begins. Spec
authoring follows the constitution + spec 000.

### Sketch 1 — "Ephemeral codegen sandbox runtime"

- **Why.** Closes the ASI05 primary gap. Today factory-engine emits
  code; that code is exercised on host runtimes without a hardened
  isolation boundary.
- **Shape.** A new crate (or extension to `crates/factory-engine`)
  spawning gVisor / Firecracker / strict-network-policy K8s pods
  per s5/s6 execution with TTL, pids_limit, memory + CPU caps, and
  no host mounts. Pod templates live under `platform/k8s/`.
- **Coupling.** `establishes:` the sandbox pod spec under
  `platform/k8s/`; `co_authority:` with spec `072-multi-cloud-k8s-portability`
  on the K8s integration; `constrains:` `075-factory-workflow-engine`
  with an invariant that no s5/s6 codegen runs outside the sandbox
  contract.

### Sketch 2 — "Stop-hook drift gates"

- **Why.** Brings the harness's "gates fire at conversation end"
  leverage into OAP. Today gates fire at PR time; they should fire
  before any commit is even made.
- **Shape.** A `.claude/settings.json` schema extension (or a
  separate file the rules load) that wires PostToolUse (cheap) +
  Stop (full `make pr-prep`-class) hooks. Hooks invoke existing
  binaries — `codebase-indexer check`, spec-code-coupling-check,
  `spec-lint` — through the governed-read consumer pattern.
- **Coupling.** `extends:` spec `127-spec-code-coupling-gate` with
  a new Stop-time runtime; no `supersedes:` because PR-time gates
  remain.

### Sketch 3 — "Signed adapter manifests"

- **Why.** Closes the ASI04 enforcement gap. Today the
  codebase-indexer hashes adapter manifests for staleness; the
  factory engine does not yet refuse a hash-mismatched adapter.
- **Shape.** Extension to `factory-engine` that consults a
  `signed_manifests:` registry block (compiler-emitted) and refuses
  adapter load on hash mismatch. The block is populated by
  `spec-compiler` from each adapter's owning spec's
  `signed_artifacts:` frontmatter field.
- **Coupling.** `extends:` spec `074-factory-ingestion`,
  `establishes:` the `signed_manifests` registry path.

### Sketch 4 — "Stagecraft agent runtime manifest endpoint"

- **Why.** Brings AIDE's `GET /velocity/claude-md` pattern onto OAP
  without abandoning the spec spine. The endpoint serves a runtime
  projection of authored truth.
- **Shape.** New stagecraft service or a single endpoint within
  `platform/services/stagecraft/` that returns a compiler-emitted
  agent runtime manifest (tool grammar + active policy rules +
  protocol excerpts), versioned by spec registry hash. OPC + any
  external agent fetches it once at startup. Live OpenAPI surface
  served at the same time (Encore.ts emits it).
- **Coupling.** `establishes:` the endpoint path; `extends:` spec
  `103-init-protocol-governed-reads` with a network-facing variant
  of the governed-reads principle.

### Sketch 5 — "Deterministic structural-diff plan UI"

- **Why.** Closes the ASI09 cockpit-side gap. Agent plans rendered
  conversationally invite the trust-exploitation failure mode.
- **Shape.** OPC panel (likely a refresh to spec
  `076-factory-desktop-panel`) that renders any agent's proposed
  plan as a YAML/JSON action graph + a structural diff against the
  current spec spine state. Conversational summaries are *demoted*
  to a secondary panel, not the primary action surface.
- **Coupling.** `refines:` spec `076-factory-desktop-panel` on the
  rendering aspect; `co_authority:` with spec
  `032-opc-inspect-governance-wiring-mvp` on the governance UI
  contract.

### Sketch 6 — "Anti-abuse rule library for agent behaviour"

- **Why.** Ports the Reaper's pattern (`speed_run` /
  `no_collaboration` / `burst_turns` / `self_approval`) from
  user-gamification anti-cheat to agent-behaviour anomaly
  detection. Different domain, same idempotent-CTE-detection shape.
- **Shape.** A `standards/anti-abuse/` directory of small specs,
  each one a rule + idempotent SQL detection query, fired by a
  factory-engine post-run hook. Violations write to a governed
  `agent_anomaly` table; Redemption-style forgiveness preserves
  the audit chain.
- **Coupling.** `establishes:` the rule directory under
  `standards/anti-abuse/`; `co_authority:` with spec
  `047-governance-control-plane` on the proof-chain integration.

### Sketch 7 — "Per-tool JSON-Schema strictness enforcement"

- **Why.** ASI02 ("Schema validation, strong typing, and
  transactional write guardrails") requires non-permissive
  per-tool schemas. Today registration accepts whatever the
  `ToolDef` author writes; lint-time enforcement closes the gap.
- **Shape.** Extension to `crates/tool-registry`: `ToolDef`
  registration fails compile-time if the schema contains
  `additionalProperties: true`, `type: any`, or unconstrained
  `object` without `properties`. Existing tools migrate
  incrementally — a `permissive: true` opt-out exists during
  migration, lit on by `spec-lint` warning V-…
- **Coupling.** `refines:` spec `067-tool-definition-registry` on
  the schema-strictness aspect.

### Sketch 8 — "Tenant spec-spine: born-with + retrofit" (**L2, load-bearing**)

This is the load-bearing L2 sketch — the one that turns the
convergence from "stagecraft features" into "every produced /
governed tenant carries the substrate's discipline by construction."

- **Why.** OAP's spec-spine is not only the substrate's internal
  governance — it is a **portable governance kernel** that every
  tenant project should carry inside itself. Two channels:
  - **Born-with.** When `factory-engine` produces a new tenant
    project from an adapter, the resulting repo ships with a
    pre-populated `specs/000-bootstrap-spec-system/spec.md`,
    a `standards/spec/` directory cloned from OAP's current
    refinement, a compiled `.derived/spec-registry/registry.json`
    from the initial bootstrap, and tenant-side analogues of the
    `127`/`130`/`133` coupling gates wired into the tenant's CI.
  - **Retrofit.** For projects that pre-date OAP (the
    [agent-builder-console](file:///Users/bart/Dev1/agent-builder-console)
    case — its `specs/` was reverse-engineered onto an earlier
    spec-spine version) the same kernel can be installed after the
    fact, scanning the existing code to seed `establishes:`
    relationships and accepting that `origin: retroactive: true`
    will mark much of the initial corpus.
- **Shape.** Two pieces, one shared:
  - A new factory pipeline stage (or extension to spec
    `120-factory-extraction-stage`) that emits a tenant-side
    `specs/`, `standards/spec/`, and `.derived/` skeleton as part
    of any adapter's scaffold output.
  - A new OAP-side binary `tools/oap/spec-spine-port` that takes
    an existing repo and produces a retrofit PR establishing the
    spec-spine kernel inside it, seeded by `featuregraph`-style
    code scanning. The binary becomes a `/spec-spine port` slash
    command in OPC.
  - The **shared** piece: a versioned, content-hash-anchored
    "spec-spine kernel manifest" that both channels consume, so
    the OAP team's ongoing refinements (e.g. the relationship
    graph in spec `130` / `152`) propagate to tenants on a known
    cadence rather than forking silently.
- **Coupling.** `establishes:` the tenant scaffold output path
  contract under the adapter manifests; `extends:` spec
  `120-factory-extraction-stage`; `co_authority:` with spec
  `074-factory-ingestion` on factory-contract integration;
  `constrains:` adapter manifest specs with an invariant that the
  scaffold output must include a current-kernel-hash marker.
- **Why this is load-bearing.** Without it, every L2 capability in
  §2 (state-machine concurrency primitives, governance certificate,
  identity scopes, drift gates, sandboxing) has to be re-asserted
  per tenant by hand. With it, the substrate ships the discipline
  as a kernel and the tenant inherits it as a matter of birth — or
  retrofit, when the tenant predates the substrate.

### Sketch 9 — "Tenant-side governance certificate emission and verifier"

- **Why.** Spec 102's governance certificate proves OAP's own
  factory runs are tamper-evident. Tenant projects need the
  equivalent for *their* CI/CD pipelines — otherwise the trust
  chain breaks at the tenant boundary, and the auditor cannot
  independently verify what the tenant's agent has done.
- **Shape.** Ship `crates/governance-certificate` (factor out from
  spec 102 implementation if not already a standalone crate) as a
  reusable Rust crate + a Node port; emit certificates for tenant
  CI runs from a born-with template; ship `verify-certificate`
  alongside as a sibling binary (FR-007 of spec 102 — the auditor
  does not trust the producer).
- **Coupling.** `extends:` spec `102-governed-excellence` with a
  "tenant-emit" mode; `co_authority:` with Sketch 8 on the
  born-with scaffold output.

---

## 5. Convergence checklist (OWASP-shaped, release-readiness)

A pre-merge / pre-release checklist that absorbs both the AIDE
discipline and the ASI doctrine. Items are written so each maps to
a real binary, gate, or spec in OAP (or names the gap if absent).

- [ ] **No raw shell / `eval` in agent action paths** — audit
      `crates/agent`, `crates/axiomregent`, `crates/factory-engine`
      for direct `Bash`/`exec` bypassing `tool-registry`. (ASI02, ASI05)
- [ ] **Every tool defines a strict, non-permissive JSON schema** —
      Sketch 7 enforces this. Until landed, manual audit of
      `crates/tool-registry` registrations. (ASI02)
- [ ] **Bounded agent loops** — every agent invocation has a
      `max_iterations` ceiling that LLM context cannot override.
      Audit `crates/orchestrator` + `crates/factory-engine`. (ASI08)
- [ ] **External inputs wrapped in protected context boundaries** —
      raw user payloads inside agent prompts use explicit boundary
      markers; system-prompt overrides are rejected. (ASI01)
- [ ] **Audit trail to write-once log infrastructure** — governance
      certificate writes to disk under `<project>/.factory/runs/`;
      retention + tamper-evidence story is the governance certificate's
      SHA-256 chain. `make verify-certificate` exits non-zero on
      tamper. (ASI09, ASI10)
- [ ] **Stop-hook drift gates wired** — `.claude/settings.json` runs
      `codebase-indexer check` (PostToolUse) and `make pr-prep`-class
      checks (Stop). Sketch 2 is the spec home. (ASI01, ASI06)
- [ ] **Adapter manifests signed and verified** — Sketch 3. (ASI04)
- [ ] **Sandboxed codegen execution for factory-engine adapters** —
      Sketch 1. (ASI05)
- [ ] **Ephemeral, downscoped tokens at every outbound boundary** —
      Rauthy + spec 137 implementation. Today PATs (where they exist
      for upstream-template clone) are short-lived but not
      per-tool-call downscoped. (ASI03)
- [ ] **Signed inter-stage manifests** in factory-engine. (ASI07)
- [ ] **Deterministic structural-diff UI** for agent plans in OPC.
      Sketch 5. (ASI09)
- [ ] **Live agent-session introspection** in OPC (force-disconnect
      capability). (ASI10)
- [ ] **Codification gate** — every CRITICAL/HIGH security finding
      writes itself into the spec spine before a session can close.
      Adaptation of the harness pattern; needs a spec home
      (potentially extending spec `121-claim-provenance-enforcement`
      or a new `standards/security/` spec). (ASI04, ASI06)
- [ ] **L2: tenant born-with spec-spine** — every factory-emitted
      tenant ships with `specs/`, `standards/spec/`, `.derived/`,
      and a kernel-hash marker. Sketch 8 is the spec home. (ASI01)
- [ ] **L2: tenant retrofit path** — `tools/oap/spec-spine-port`
      can install the kernel into an existing repo (the
      agent-builder-console pattern, generalised). Sketch 8. (ASI01)
- [ ] **L2: tenant governance certificate** — tenant CI emits its
      own self-authenticating certificate. Sketch 9. (ASI04, ASI09)
- [ ] **L2: kernel-update propagation** — tenants opt in to a
      versioned spec-spine kernel; OAP-side refinements (e.g. the
      `130`/`152` relationship-graph work) propagate to tenants
      on a known cadence, not via silent fork. (ASI04)

The checklist is intentionally machine-translatable to a
`tools/oap/asi-readiness-check` binary in future. Today it lives
here as the human-readable form. Items prefixed **L2:** apply to
the produced/retrofitted tenant project, not to OAP itself.

---

## 6. What OAP already does *better* than AIDE

For symmetry — the convergence is not all one-way:

1. **Spec spine is *authored* truth, not derived truth.** AIDE
   relies on `check-docs-sync.mjs` to keep OpenAPI / README /
   CLAUDE.md aligned with code through a battery of mechanical
   checks. OAP's `spec-compiler` emits machine truth *from* authored
   markdown — the relationship is hierarchical, not horizontal.
   Authored truth cannot drift from itself.
2. **Encore.ts service boundaries** in `platform/services/` decompose
   stagecraft into independently-scaled services in a way AIDE's
   Express monolith cannot. This is structurally important for
   ASI08 (cascading failures) — a misbehaving service is bounded by
   its own service-account-scope.
3. **Rust crates with type safety** — `orchestrator`, `policy-kernel`,
   `tool-registry`, `factory-engine`, `agent` — give compile-time
   guarantees AIDE's `service → model` JS layers approximate
   informally. Type safety is a force multiplier for ASI02 (tool
   misuse: a typed `ToolDef` cannot accept the wrong shape).
4. **Governance certificate as an independently-verifiable artifact.**
   The auditor running `make verify-certificate` does not trust the
   producer. AIDE's `audit_log` is immutable inside its own
   database, but the integrity guarantee dissolves the moment you
   trust the DB. OAP's chain is verifiable from outside the system.
   This is the strongest single ASI09 / ASI10 mitigation in the
   codebase.
5. **Tauri-based OPC cockpit** establishes a separate trust boundary
   from any web surface. Local workspaces + native file-system
   access + persistent state outside the browser sandbox means an
   agent cannot reach the cockpit's host through a poisoned web
   asset.
6. **Policy kernel + 5-tier settings merge** (`047-governance-control-plane`)
   provides governed configuration that the harness handles
   informally via env vars (`CLAUDE_ALLOWED_TOOLS`, `VELO_LISTEN_ALL`,
   …). OAP's tiered merge with proof chains is the structural
   answer to ASI03.

These are the *current* OAP edges. The convergence work in §4 + §5
extends them — it does not replace them.

---

## 7. Open questions

1. **AIDE's "challenge" feature → OAP analogue?** AIDE supports
   multi-agent parallel attempts on the same project via cloning.
   The closest OAP analogue is the promotion-grade mirror (spec
   `097-promotion-grade-mirror`), but that does not yet model
   parallel competing attempts. Should this be a new spec, or an
   extension of 097?
2. **Should governance certificates carry self-critique alongside
   artifact hashes?** AIDE's `module-1-critical-audit.md` is a
   remarkable artifact — an AI-authored self-critique identifying
   real gaps in its own v1–v8 run. Embedding a structured
   self-critique field in the governance certificate (FR-007
   extension) would import that honesty discipline into OAP.
3. **GoA Design System adoption.** AIDE's GoA-public template uses
   `@abgov/web-components` + GoA-specific CSP rules. Does OPC need
   a GoA mode? Or is the cockpit's distinctive UX style a hard
   commitment? Likely out of scope for the convergence work.
4. **`If-Match` / `Idempotency-Key` plumbing across stagecraft.**
   Encore.ts middleware can do this once. Worth doing as a single
   small spec rather than per-endpoint?
5. **The harness's per-project session-UUID pattern** is a clean
   primitive (`listener-sessions.json`). Where does it live in OAP
   — `crates/orchestrator`'s state surface (spec 052), or a new
   per-project session spec? Likely a refinement to 052.
6. **Multi-tenant blast-radius bound.** Spec 137 is the load-bearing
   piece here. Until landed, stagecraft cannot bound per-tenant
   resource consumption — which is precisely ASI10's "central
   lifecycle orchestrators" prescription.
7. **MCP tool plugin content addressing.** `crates/axiomregent`
   resolves MCP plugins. Are plugins resolved by hash today, or by
   name? (ASI04 requires hash.) Worth a quick audit.
8. **How does the spec-spine kernel version-propagate to tenants?**
   When OAP refines its own spec-spine (e.g. the
   `130-spec-coupling-primary-owner` / `152-path-co-authority`
   relationship-graph work landed in May 2026), tenants born
   earlier carry an older kernel. The agent-builder-console
   `specs/` was reverse-engineered onto an earlier version and is
   now outdated relative to current OAP. We need a propagation
   model — versioned kernel hash + upgrade tool — so that the
   tenant fleet does not silently fork. Sketch 8's "shared piece"
   is the proposed mechanism; the open question is **cadence and
   compatibility policy**.
9. **L2-vs-L1 doctrine boundaries.** Some ASI controls
   (e.g. ASI05 sandboxed codegen) have to be satisfied at both
   levels — substrate and tenant — but with different
   implementations. Should the convergence checklist (§5) split
   into two parallel checklists, or is the L1/L2 prefix sufficient
   discipline? The current draft uses prefixes; experience with
   the first tenant born of the substrate will tell us whether
   that's enough.

---

## 8. Reading order for engineers picking this up

1. Skim **§1 Convergence frame** to align on which OAP component
   owns which AIDE responsibility.
2. Read the AIDE-side blueprint of the domain you are converging
   (e.g. §6 AIDE-VELOCITY for state machine + persistence; §10 of
   the HARNESS blueprint for Claude Code hooks).
3. Read the matching §2 sub-domain here (A through K).
4. Cross-check §3 to find the ASI control(s) and the OAP specs
   already addressing them.
5. If forward work is required, read the matching §4 candidate
   sketch — then promote it to a real spec under `specs/NNN-slug/`
   following the constitution.
6. The §5 checklist is the release-readiness pre-flight when you
   ship the convergence work.

The canonical OWASP doctrine source for this document is the
**executive** ASI summary at
[`owasp_top_10_agentic_applications_summary.md`](../owasp_top_10_agentic_applications_summary.md).
The longer "_oap.md" architectural sketch that previously sat
alongside it was retired during intent alignment (see Purpose block)
because it described a layered OAP↔OPC separation that doesn't match
the single-repo substrate reality.

When reading any §2 sub-domain, keep the L1/L2 distinction from §1
in mind: the same ASI control may be answered differently at
substrate level (hardening OAP itself) and at tenant level (what a
produced or reverse-engineered tenant project inherits from the
substrate). Most genuinely new convergence work is L2 work — the
substrate already does the heavy L1 lifting.
