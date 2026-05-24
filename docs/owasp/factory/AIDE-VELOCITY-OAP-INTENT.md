# AIDE-VELOCITY → OAP — Intent Alignment

> **What this document is.** A captured-intent artifact from a refinement
> session between the OAP maintainer and an agent. It records the
> alignment reached on how OAP should match-and-surpass AIDE-VELOCITY +
> AIDE-VELOCITY-HARNESS, expressed in OAP-native primitives (spec-spine,
> stagecraft, OPC, factory-engine). It is the input to a subsequent
> decomposition pass that will produce real specs under
> `specs/NNN-slug/spec.md`.
>
> **What this document is not.** It is not a spec. It does not design
> solutions. It does not commit a roadmap. It does not duplicate the
> blueprints it depends on.
>
> **Reading dependencies.** This document presupposes:
>
> - [`AIDE-VELOCITY-blueprint-spec.md`](./AIDE-VELOCITY-blueprint-spec.md)
>   — what AIDE-VELOCITY does today.
> - [`AIDE-VELOCITY-HARNESS-blueprint-spec.md`](./AIDE-VELOCITY-HARNESS-blueprint-spec.md)
>   — what its cockpit does today.
> - [`AIDE-OAP-CONVERGENCE-blueprint-spec.md`](./AIDE-OAP-CONVERGENCE-blueprint-spec.md)
>   — the prior synthesis. Its L1/L2 framing is **superseded** by this
>   document's "project-as-unit-of-comparison" framing (see §2). The
>   convergence doc remains useful for the per-domain translation matrix
>   (its §2 A–K) and the ASI coverage matrix (its §3).
> - [`owasp_top_10_agentic_applications_summary.md`](../owasp_top_10_agentic_applications_summary.md)
>   — executive ASI 2026 doctrine. The only OWASP-side authority used here.
>   (A longer `_oap.md` companion previously sat alongside it; it was
>   retired during this intent alignment because it described a layered
>   OAP↔OPC separation that did not match the single-repo substrate
>   reality. Anything load-bearing in it has been absorbed here or
>   already lives in the spec spine.)
>
> **Authority.** The spec spine is the contract. Per CONST-005, when
> this document and the spec spine disagree, the spec spine wins.
> Forward-pointing references to "candidate specs" are intent, not
> commitment — they become commitments only when authored as
> `specs/NNN-slug/spec.md` under the constitution.

---

## 1. Purpose and non-goals

### Purpose

To capture, in a single faithful document, the aligned intent for
producing — and reverse-engineering into — fully OWASP ASI 2026
compliant agentic projects on the OAP substrate, matching and
surpassing the capability surface that AIDE-VELOCITY (platform side)
and AIDE-VELOCITY-HARNESS (cockpit side) demonstrate today inside the
Government of Alberta.

### Non-goals

- **Not a spec.** No spec frontmatter, no FR/NFR enumeration, no
  acceptance criteria.
- **Not a design.** Concrete shapes (schemas, endpoints, UI
  components) are mentioned only where the existing codebase already
  carries them.
- **Not a 1:1 AIDE port.** The convergence doc already established
  that. This document does not re-litigate it; it builds on it.
- **Not a roadmap.** No dates. Sequencing is implied by the
  decomposition guide in §9, not committed.
- **Not arcade.** AIDE's gamification surface (`user_points`,
  leaderboards, `cheating_violation`, the Reaper anti-cheat engine,
  `velocity_idempotency`, the chess-clock 8-step grammar) is not
  ported. OAP is a professional, governed platform for serious
  agentic SWE — agent behaviour governance lives in the spec-spine
  + governance certificate + policy kernel, not in points.

---

## 2. Frame

### 2.1 The single-substrate correction

OAP is a single-repo substrate. The spec spine, the Rust crates, the
desktop cockpit (OPC), and the platform services (stagecraft,
deployd-api-rs, Rauthy charts) all share one CI substrate and one
governance discipline. The convergence doc's L1 (substrate) / L2
(tenant) distinction was a useful early framing but it obscured the
load-bearing fact that **the unit of comparison between OAP and AIDE
is a project, not a substrate**.

### 2.2 The unit of comparison

A **stagecraft project** is the OAP equivalent of an
**AIDE-VELOCITY project**. The two systems compare at the project
level:

- An **AIDE-VELOCITY project** is a row in the `project` table with
  modules, a velocity board, a SharePoint folder, a GitHub
  repository, and an audit log.
- A **stagecraft project** (already wired today; see
  `platform/services/stagecraft/api/projects/`) is a more capable
  object: it carries the same lifecycle responsibilities and adds
  governed-import, scoped scaffolding, OPC-bundle generation,
  knowledge ingestion, factory-engine integration, and governance
  certificate emission.

Stagecraft's project surface already structurally exceeds AIDE's
project surface; the convergence work refines and completes that
surface rather than reinventing it.

### 2.3 The cockpit relationship

**OPC** (`product/apps/opc/`) is the OAP equivalent of the
**AIDE-VELOCITY-HARNESS**. OPC is where decomposition, execution,
and developer-side governance happen. Stagecraft governs; OPC drives.

OPC's capability surface (see §4) materially exceeds the harness
because it carries xray (structural fingerprinting + call graphs +
history), semantic search, checkpoint / branch-of-thought, the
governance certificate verifier, and the factory-engine driving UI
— none of which the harness has analogues for.

### 2.4 The universal representation

**Spec-spine is the universal representation that replaces AIDE's
`./velocity/v?-*/output/*.md` files**. Where AIDE has one fixed
`requirements.md` plus one `plan.md` plus one `architecture.md`
per module (rigid, one-shot, must-be-feature-complete-or-blocked),
OAP has many small specs that compose iteratively via the eight
relationship edges (spec 130), the logical-unit ownership grammar
(spec 154), named-anchor sectioning (spec 152), and the typed kind
grammar (spec 147).

Spec-spine is iterative-by-default. A baseline auth requirement is
one spec; an auth-driver registry is another that `extends` it; a
SAML driver is a third that `references` the registry. The
stakeholder dashboard composes "Authentication" as a derived view
over the relationship graph — not as a fixed enclosing entity.

### 2.5 The compliance posture

**Both the substrate and every project it produces or governs must
be 100% OWASP ASI 2026 compliant.** Not aspirationally — by
construction. The substrate's compliance is achieved by hardening
its own crates, services, and policies. A produced project's
compliance is achieved by being born of factory adapters that carry
the discipline into the scaffold, by living under stagecraft's
governance, by being driven through OPC's gates, and by emitting its
own governance certificate at every pipeline run.

This document treats ASI05 (sandboxed execution substrate) as a
non-negotiable in-scope gap. The substrate is not OWASP-compliant
until it lands.

### 2.6 The iterative discipline

A baseline-by-implication, refine-by-evidence model. A developer
ships a baseline spec that captures intent at today's information
level; further specs `extend` or `refine` it as the project evolves;
`amends` carries clarifications and corrections without superseding;
`supersedes` is reserved for genuine direction changes. The
coupling gate (spec 127/130/133) fires on logical-unit drift,
refactor-invariant by spec 154's grammar.

This is the structural opposite of AIDE's "feature-complete or
halt" model and is what makes the spec-spine fit for production
agentic SWE.

---

## 3. The stagecraft project surface

### 3.1 Current route inventory

Routes that already exist under
`platform/services/stagecraft/web/app/routes/`:

| Route | Status |
|---|---|
| `app.project.$projectId._index.tsx` | exists (the dashboard root) |
| `app.project.$projectId.knowledge.tsx` + `.$id.tsx` | exists |
| `app.project.$projectId.pipelines.tsx` | exists (rename candidate) |
| `app.project.$projectId.agents.tsx` + `._index.tsx` | exists |
| `app.project.$projectId.deploys.tsx` + `.$envId.tsx` | exists |
| `app.project.$projectId.settings.*` | exists (full hierarchy) |
| `app.project.$projectId.tsx` | exists (project layout root) |

API surface that already exists under
`platform/services/stagecraft/api/projects/`: `create.ts`, `clone.ts`,
`import.ts`, `importArtifacts.ts`, `importInstallations.ts`,
`importHelpers.ts`, `cloneCore.ts`, `cloneWorker.ts`, `cloneEvents.ts`,
`cloneRunStatus.ts`, `cloneAvailability.ts`, `scaffold/`,
`scaffoldReadiness.ts`, `scaffoldReadinessBlocker.ts`, `opcBundle.ts`,
`opcBundleHelpers.ts`, `projectKnowledge.ts`, `projectPat.ts`,
`tokenResolver.ts`.

This represents a project-lifecycle surface materially richer than
AIDE-VELOCITY's. The convergence work composes views on top of this
rather than building new project machinery.

### 3.2 Intended menu structure

```
/app/project/:id
├── Dashboard          (existing _index, refined)
├── Knowledge          (existing — AIDE SharePoint analogue)
├── Requirements       (NEW — spec-spine rendered)
├── Development        (RENAME of pipelines — lifecycle-state board)
├── Deploys            (existing — scope-gated deployment)
├── Agents             (existing — per-project agent catalogue)
└── Settings           (existing — full hierarchy)
```

### 3.3 Per-menu intent

**Dashboard (`_index`)** — refines the existing dashboard to fulfil
AIDE's `ProjectDetailView` role: project identity, current
lifecycle posture, recent governance certificate, recent factory
runs, risk banner, audit summary, links into Knowledge /
Requirements / Development. This is observability, not editing.
AIDE renders a full CRUD form here; OAP keeps editing in the
appropriate sub-view and the dashboard is the at-a-glance surface.

**Knowledge** (already wired) — the AIDE SharePoint document folder
analogue. Where a project's input artifacts live: uploaded
requirements documents, business analysis, existing-codebase
bundles for reverse-engineering, design docs, stakeholder writeups.
Already integrated with the spec 120 extraction stage
(`s-1-extract`), which produces typed `ExtractionOutput` from
knowledge objects via the deterministic Rust extractor
(`crates/artifact-extract`). Knowledge is the *input*; Requirements
is the *output*.

**Requirements (NEW)** — the spec-spine of the project, rendered.
This is the primary stakeholder-visible artifact. It is composed of
draft and approved specs that live inside the project's own repo
(under `<project>/specs/NNN-slug/spec.md`), produced either by OPC
from knowledge consumption, by OPC from reverse-engineering, or
authored directly. The view groups specs dynamically by walking
`references:` (and filtering on `category:` / `kind:`), exposing
optional cosmetic custom group names attached as decoration on the
derived groups. The view supports review / edit / approval flows;
approved specs become the contract OPC pulls back to drive
execution. See §6 for the production pipeline.

**Development (RENAME of pipelines)** — the AIDE `VelocityView`
analogue, but expressed in spec-spine lifecycle states, not the
rigid chess-clock 8-step grammar. Columns are spec lifecycle
states: `draft → approved → implementation:pending →
implementation:in-progress → implementation:complete`, with
separate visual lanes for `superseded` and `amended`. Cards are
individual specs (or grouped clusters when grouping is active);
governance-certificate emission and factory-engine activity are
overlaid as execution evidence. This is the **execution
observability** surface — the actual execution happens at OPC.

**Deploys** (already wired) — scope-gated deployment surface
backed by `deployd-api-rs`. Substrate-native; no AIDE analogue
(AIDE deploys to a single Azure App Service via GitHub Actions; OAP
governs deployments through Rauthy-scoped service identities).

**Agents** (already wired) — per-project agent catalogue. The
catalogue is org-rescoped per spec 123 and onboarded per
spec 080.

**Settings** (already wired) — repos, connectors, GitHub PAT, etc.

### 3.4 Knowledge → Requirements lineage

When a draft spec was derived from a particular knowledge item or
a reverse-engineering pass over a particular codebase snapshot, the
spec's frontmatter carries a provenance reference: the `references:`
edge with a sibling `provenance:` field (spec 156), kind `knowledge`
or `code-fingerprint`. The Requirements view renders this provenance
as a clickable link back to the originating Knowledge item or xray
fingerprint snapshot. This closes the loop AIDE leaves implicit: in
AIDE the SharePoint file produces `requirements.md` and the link
between them is operational convention; in OAP the link is a typed
relationship-graph edge.

> *Updated 2026-05-22.* Spec 156
> (`references-edge-provenance-grammar`) defines this as a
> **sibling `provenance:` field** on `references:` entries,
> mutually exclusive with `unit:` — the two kinds (`knowledge`,
> `code-fingerprint`) live in `provenance.kind`, not in spec 154's
> `unit.kind` (see spec 156 §3 for the rationale; the six in-tree
> unit kinds share refactor-invariance machinery the new external-
> pointer kinds do not).

---

## 4. The OPC project surface

### 4.1 ProjectToolbar tools

From `product/apps/opc/src/components/ProjectToolbar.tsx`, the
per-project surfaces are:

| Tool | Component | OAP capability |
|---|---|---|
| **CLAUDE.md** | `ClaudeMdTab` | Per-project agent guide |
| **Git Context** | `GitContextPanel.tsx` + `axiomregent/github` | History + PR/branch + remote context |
| **Xray Analysis** | `crates/xray/` | Structural fingerprinting, complexity, traversal, history, language detection |
| **Governance** | `GovernancePanel.tsx` + spec 102 | Live governance state, certificate browsing, verification |
| **Semantic Search** | `SemanticSearchPanel.tsx` + `axiomregent/search` | Vector-space code matching (the former blockoli surface) |
| **Call Graph** | `CallGraphPanel.tsx` + xray | Behavioural traversal — function relationships, not syntax |
| **Checkpoint** | `CheckpointPanel.tsx` + spec 095 | Branch-of-thought, alternate-history, agent-context snapshots |
| **Portfolio** | `PortfolioPanel.tsx` + spec 096 | Multi-project portfolio view |
| **Promotion** | `PromotionPanel.tsx` + spec 097 | Promotion-grade mirror — when is a project ready to graduate |

In `product/apps/opc/src/components/factory/`: the factory
driving UI — `ArtifactInspector`, `BuildSpecStructuredView`,
`FactoryPipelinePanel`, `PipelineDAG`, `PipelineHistory`,
`ProjectContextOverview`, `ProvenanceHealthPanel`, `ScaffoldMonitor`,
`StageCdReview`, `TokenDashboard`, `GateDialog`, `LiveAgentOutput`.

In `product/apps/opc/src/features/`: `checkpoint/`, `git/`,
`governance/`, `inspect/`, `portfolio/`, `promotion/`.

### 4.2 Sessions — multi by project path

OPC's session model is *multi-session per project path*. Each
session is independently persisted; logged-in user identity
disambiguates sessions across developers; checkpoint hooks (spec
095) capture branch-of-thought without forcing a single linear
session per project.

This materially exceeds AIDE-HARNESS's "one stable UUID per
project" pattern (`listener-sessions.json`). The OAP discipline:

- Many sessions per project (developer choice, no platform
  constraint)
- Sessions bound by project path (workspace identity, not project
  identifier — the same path may be a different stagecraft project
  in different orgs)
- Login granularity at the OPC level means two developers on the
  same project produce disambiguable session streams at the
  stagecraft level
- Checkpoint branches are the natural multi-session escape valve
  when a developer wants to explore alternative agent paths

### 4.3 Gates — mechanical, conversation-time

OAP today fires governance gates at PR time
(`make pr-prep`, `make ci`, the spec-code coupling gate, spec-lint
default-fail-on-warn). The AIDE-HARNESS leverage is that *gates
fire at conversation end, inside the Claude Code session*, before
any commit is made.

OPC should adopt the same pattern: wire `make pr-prep`-class checks
as `PostToolUse` (cheap, for staleness signal) and `Stop`
(blocking, for full validation) hooks in OPC's Claude Code surface.
The governed-artifact-reads rule already constrains how these
checks read the spec spine (via consumer binaries per spec 103);
the gates themselves run existing binaries
(`codebase-indexer check`, `spec-code-coupling-check`, `spec-lint`).

### 4.4 Why OPC's surface materially exceeds AIDE-HARNESS

| Capability | AIDE-HARNESS | OPC |
|---|---|---|
| Code understanding | LLM-only over heterogeneous text | xray structural fingerprint + semantic search + call graph + git history |
| Multi-session | One stable UUID per project | Many sessions per project, per-developer disambiguated |
| Audit | Append-only gate-state JSONL | Governance certificate (independently verifiable, spec 102) |
| Decomposition | `v1-requirements` skill, one LLM pass | Factory extraction stage (spec 120) + xray + semantic + LLM synthesis (see §6) |
| UI trust boundary | Browser tab | Tauri-based native cockpit — separate trust boundary from any web surface |
| Spec authority | Convention-derived from `./velocity/v?/output/*.md` | Spec-spine (authored markdown → compiler-emitted JSON) — drift-impossible by construction |

### 4.5 Project tab / Factory tab — awareness flag

OPC currently separates a "Project" tab and a "Factory" tab. A
near-term refactor is being considered to integrate the factory
surface as a sidebar within the project view, so that driving a
factory run is a project-scoped action rather than a context
switch. This document is written in tab-agnostic language; the
intent does not depend on the current split.

---

## 5. Spec-spine as universal representation

### 5.1 Why spec-spine replaces `./velocity`

AIDE's `./velocity/v?-*/output/*.md` files are rigid, one-shot,
must-be-feature-complete-or-block. The harness's gates fire when a
deliverable is incomplete or out of sync. This is *useful* discipline
but at the wrong seam: it forces upfront full specification of every
aspect of every feature before the project can advance.

Spec-spine inverts this:

- **Granular and iterative.** Many small specs, each one capturing
  one piece of intent at the level of fidelity available at the
  time it was written.
- **Refactor-invariant.** Per spec 154, ownership is expressed as
  logical units (crate / symbol / module / section / directory /
  file), not paths. A refactor that renames or moves code without
  changing behaviour does not trip the coupling gate.
- **Relationship-graph-aware.** Per spec 130, the eight relationship
  edges (`establishes` / `extends` / `refines` / `supersedes` /
  `amends` / `co_authority` / `constrains` / `origin`) plus the
  ninth non-owning edge (`references:` from spec 154) give the
  authoring grammar to evolve a project iteratively without losing
  governance.
- **Kind-typed.** Per spec 147, specs are typed (`capability` /
  `registry` / `profile` / `amendment` / `governance` / `platform`)
  with optional `shape:` and orthogonal `category:` for projection.

### 5.2 Grouping is derived

Stagecraft's Requirements view composes "Authentication" (or any
other stakeholder-visible cluster) by walking the relationship
graph at render time, filtering on `category:`, `kind:`, and
`references:` edges. **There is no `groups.yaml` config and no new
spec-spine field.** Everything needed is in the authored frontmatter.

A *custom cosmetic name* may be attached to a derived group as
presentation metadata — a label the project owner chooses to show
stakeholders. The custom name has no semantic relationship to the
spec-spine; it does not change ownership, authority, or any gate
behaviour. It is decoration on a derived view.

### 5.3 Lifecycle states drive the Development board

Spec-spine lifecycle states (from spec 147's grammar):

- `draft` — captured but not yet approved
- `approved` — accepted as intent
- `implementation: pending` — approved, work not yet started
- `implementation: in-progress` — actively being implemented
- `implementation: complete` — implemented and merged
- `superseded` (with `superseded_by:`) — direction change
- `amended` — patched without supersession

The Development view's columns are these states. Cards are individual
specs or (when grouping is active) derived clusters. Governance
certificate emissions, factory run completions, and coupling gate
fires are overlaid as execution evidence on the cards. This is the
**honest** AIDE-VelocityView analogue — it surfaces the same
lifecycle visibility AIDE provides via its 8 chess-clock steps,
without imposing AIDE's "all features go through exactly these 8
states in this order or stall" rigidity.

### 5.4 No chess-clock steps, no actor turns, no points

AIDE's distinctive primitives that **OAP does not adopt**:

- The 8-step chess-clock state machine (`requirements → planning →
  architecture → prototyping → development → user_testing →
  user_acceptance → deployment`). Replaced by spec-spine lifecycle
  states (§5.3).
- `current_actor ∈ {ai, human}` turn tracking. Replaced by Rauthy
  identity + governance certificate signer block + git commit
  attribution.
- `velocity_turn` immutable ledger. Replaced by spec-spine git
  history + governance certificate chain + audit log via
  `crates/run`.
- Gamification (`user_points`, leaderboards, `cheating_violation`,
  the Reaper anti-cheat engine). Not ported. Agent behaviour
  governance lives in the policy kernel + governance certificate,
  not in points.
- `If-Match` + `Idempotency-Key` HTTP header primitives. These are
  *useful patterns* and may land per-endpoint where needed in
  stagecraft (optimistic concurrency + safe retries), but they are
  endpoint mechanics, not project-lifecycle primitives.

---

## 6. The reverse-engineering pipeline

### 6.1 Trigger

The trigger sits in stagecraft. When a developer imports an existing
project (the agent-builder-console case — a GitHub repository
without a pre-existing spec-spine), stagecraft completes the import
via its existing `api/projects/import.ts` pipeline and then emits
two affordances:

- **"Ready for decomposition"** — a state event indicating the
  project is materialised, knowledge bundles exist, and the
  decomposition pipeline can run.
- **"Open in OPC"** — an action that hands the project off to OPC
  for the actual decomposition work.

The decomposition itself happens at OPC, not at stagecraft.

### 6.2 OPC decomposition pipeline

Inputs:

- The imported project's working tree (locally accessible via OPC's
  workspace integration)
- Knowledge items associated with the project in stagecraft
- The OAP spec spine kernel (versioned)

Stages:

1. **Extraction** — spec 120's `s-1-extract` stage. Deterministic
   Rust extractor (`crates/artifact-extract`) emits typed
   `ExtractionOutput` per knowledge object. No LLM at this stage —
   provenance-bearing, page-bounded structured text.
2. **Structural fingerprint** — `crates/xray` traverses the working
   tree, emitting fingerprints, complexity scores, language map,
   loc, history.
3. **Semantic clustering** — `axiomregent/search` produces
   conceptual clusters over the codebase via vector matching.
4. **Behavioural traversal** — call graph + cross-reference data,
   exposing functional implications that pure syntactic inspection
   would miss.
5. **Temporal lineage** — git history mapped to logical units
   (per spec 154 grammar) so that *when* a piece of behaviour was
   introduced is captured alongside *what* it does.
6. **LLM synthesis** — the synthesiser consumes the structured
   evidence from stages 1–5 and emits **draft specs**. Each spec:
   - Carries `status: draft`
   - Carries `origin: retroactive: true` (honestly: it was
     reverse-engineered, not authored from intent first)
   - Declares `kind:` per spec 147 grammar
   - Declares appropriate `category:` for grouping projection
   - Declares logical units (per spec 154) for any
     `establishes:` / `extends:` / `refines:` / `references:` it
     claims
   - Carries a `references:` edge with `kind: knowledge` or
     `kind: code-fingerprint` pointing at the originating knowledge
     item or xray fingerprint snapshot (the Knowledge→Requirements
     lineage from §3.4)

### 6.3 Review, edit, approve

The draft specs land in the project's repo under `<project>/specs/`
and surface in stagecraft's Requirements view. The developer
reviews them there: edits, refinements, rejection of bad
derivations, splitting of over-broad specs, grouping into named
clusters. When a draft spec is approved, its `status:` is promoted
from `draft` to `approved`.

The `origin: retroactive: true` marker is intentional and
permanent. It records the honest provenance of the spec — it was
not authored from intent first, and the spec-spine treats that
distinction as load-bearing (per spec 130). A future commit may
elaborate the spec, refine its claims, or extend it; the retroactive
marker stays until and unless a successor spec replaces it under
the normal `supersedes:` mechanism.

### 6.4 Approved specs become the execution contract

When OPC subsequently drives factory runs against the project, it
pulls the approved spec set as the execution contract. The same
coupling discipline that applies to OAP's own internals (spec
127/130/133) applies to the project: code edits must be matched by
spec edits, refactor-invariant per spec 154's logical units.

### 6.5 Born-with vs. retrofit — one pipeline

The same decomposition pipeline supports the born-with case (when
factory-engine produces a new project from an adapter) and the
retrofit case (agent-builder-console). In the born-with case the
input is the adapter's output bundle rather than an imported
codebase, and the draft specs are richer (the adapter knows what
it scaffolded), but the synthesis stage and the review/approve flow
are identical.

---

## 7. OWASP ASI 2026 compliance posture

### 7.1 Both substrate and produced/governed projects

The substrate (stagecraft + OPC + spec-spine + factory-engine +
deployd-api-rs + Rauthy) must be ASI 2026 compliant for its own
operations. Every project it produces or governs must inherit
compliance by construction.

These are two compliance targets with two evidence chains. They
share the spec spine, the governance certificate format, the
identity model, and the gate machinery — but they apply to
different artifact sets.

### 7.2 Per-control posture

| Control | Substrate posture | Project posture | Status |
|---|---|---|---|
| **ASI01** Goal Hijack | Spec-spine + coupling gate (127/130/133) + adversarial-prompt refusal (131) | Inherits spec-spine kernel; project-scoped coupling gate runs against the project's own spine | Strong, with forward gap on **runtime** planner-output validation (not just build-time) |
| **ASI02** Tool Misuse | tool-registry (067) + permission system (049/068) + safety-tier governance (036) | Inherits the same tool-registry contract via factory scaffolds | Strong, with forward gap on per-tool JSON-Schema strictness enforcement and default-read-only semantics |
| **ASI03** Identity Abuse | Rauthy (106/107) + deployd-api scope gate + tenant-environment-access-gates (137, draft) | Inherits per-project Rauthy-issued scopes; downscoped tokens at every outbound boundary | Draft (spec 137) — implementation next |
| **ASI04** Supply Chain | Supply-chain gates (116) + release attestations (117) + claim provenance (121) | Inherits the same gates via factory scaffolds + tenant-side governance certificate | Strong, with forward gap on **runtime** signed-adapter-manifest enforcement (currently observed, not enforced) |
| **ASI05** Code Execution / RCE | **NON-NEGOTIABLE GAP** — no ephemeral micro-sandbox runtime for factory-engine adapter codegen execution | Same gap at project level | **Critical** — must be specced as part of this convergence |
| **ASI06** Memory Poisoning | Unified artifact store (094) + checkpoint (095) + session memory (056) + extraction (120) + governance certificate hash anchoring | Inherits the same hooks via factory scaffolds | Strong, with forward gap on pre-write / pre-read sanitization hooks declared as a contract |
| **ASI07** Inter-Agent Comm | K8s baseline network deny + Encore `Topic<T>` pubsub | Inherits via deployd-api scope discipline | Gap on signed inter-stage manifests inside factory-engine and mTLS-by-default in Helm charts |
| **ASI08** Cascading Failures | factory-engine bounded fan-out (075) + governance enforcement stitching (098) + post-convergence remediation (100) | Inherits via the same factory-engine contract | Gap on explicit `max_iterations` ceilings on agent loops |
| **ASI09** Human-Agent Trust | Governance certificate (102) is independently verifiable; OPC is a separate trust boundary from web surfaces | Tenant emits its own governance certificate per pipeline run | Gap on deterministic structural-diff plan UI in OPC and M-of-N dual-auth UX for tenant-boundary actions |
| **ASI10** Rogue Agents | tool-registry-bound action surface; state persistence (052); centralized orchestrator | Inherits via tenant-side governance certificate + tool-registry contract | Gap on live-session introspection panel in OPC and parent-child control tree with resource quotas |

### 7.3 ASI05 — the non-negotiable

The single largest open compliance gap is the absence of a sandboxed
execution substrate for code that the factory-engine emits and then
exercises. Today, when an adapter generates code that needs to be
linted, tested, or built, the execution happens on host runtimes
without a hardened isolation boundary. ASI05's core prescription is
**ephemeral micro-sandbox isolation** — gVisor / Firecracker /
strict-network-policy K8s pods, with TTL ceilings, `pids_limit`,
memory + CPU caps, and no host file-system or control-plane access.

This must be specced and built. It is in scope for this convergence
because the substrate and every project it produces are otherwise
structurally non-compliant on ASI05. The dockerised execution
substrate is the canonical mitigation; the spec home is the
candidate in §9.

### 7.4 The auditor's verifier — independent verification

The governance certificate (spec 102) emits a self-authenticating
`governance-certificate.json` per factory run, binding requirements
hash, frozen Build Spec hash, per-stage artifact hashes, and a
SHA-256 over the canonical JSON. The companion verifier
(`make verify-certificate`) **does not trust the system that
produced the certificate** (FR-007). This is the structural ASI09
mitigation: the auditor does not depend on the agent's confident
narrative summary; they verify cryptographically.

Every project the substrate produces or governs must inherit this
emission discipline. Per §6.4, when OPC drives a factory run against
a project, the project itself emits its own governance certificate
under its `.factory/runs/<run-id>/` directory.

---

## 8. Known gaps and follow-ups

These are surfaced for spec-spine decomposition and are not
designed here.

### 8.1 Factory / adapter machinery is in flight

The four registered adapters referenced in `README.md` and
`CLAUDE.md` (`aim-vue-node`, `next-prisma`, `rust-axum`,
`encore-react`) had their canonical `manifest.yaml` files in a
`factory/` directory that has been **removed from this repo**. The
removal is part of a refactor that relocates the factory / adapter
machinery into stagecraft as a first-class feature. The old
directory (`/Users/bart/Dev1/factory/`) is referenced here only as
a fossil for structural inspiration, not as a normative source.

Consequences:

- The README's "Adapters" section is partially aspirational against
  the on-disk state.
- The codebase-indexer's `collect_input_files` list still names
  `factory/adapters/*/manifest.yaml` — this will either get
  re-routed to the new stagecraft-resident location or removed in
  step with the migration.
- Any spec sketches in §9 that depend on adapter manifests need to
  account for the in-flight relocation; they should be written
  against the stagecraft-resident form, not the legacy form.

### 8.2 ASI05 dockerised execution substrate

See §7.3. Non-negotiable. Candidate spec in §9.

### 8.3 Requirements view as net-new stagecraft route

`app.project.$projectId.requirements.tsx` does not exist. Per §3.2
+ §3.3 it needs to be authored, with the spec-spine read surface
backing it (the project's local `specs/` + `.derived/` plus the
relationship-graph projection logic for grouping).

### 8.4 Pipelines → Development rename

Existing route `app.project.$projectId.pipelines.tsx` is renamed to
Development. The rename carries the lifecycle-state board model
(§5.3), not AIDE's chess-clock 8 steps. The existing pipelines
backing (factory-engine activity, gate state, run history) becomes
the execution-evidence overlay on the new view.

### 8.5 Knowledge → Requirements lineage

Per §3.4 the lineage is encoded via the existing `references:` edge
(spec 154) with typed `kind: knowledge` or `kind: code-fingerprint`.
The Requirements view renders the link. No new edge type is
required; this is a rendering and provenance-emission concern.

### 8.6 OPC project / factory tab integration

Out of scope for this intent doc but flagged here so the
decomposition pass does not assume the current split is permanent.

### 8.7 OPC Stop-hook gate chain

OPC should adopt the harness's leverage point — gates fire at
conversation end, inside the Claude Code session, before any commit.
The mechanics already exist (`make pr-prep` runs the same gates);
the wiring is new. Candidate spec in §9.

---

## 9. Decomposition guide — candidate specs

When this intent document is decomposed into spec-spine specs, the
following are the candidates this work implies. Each entry names a
single concern; the full design of each lives in its own
`specs/NNN-slug/spec.md` written under the constitution. Numbers are
suggestive only — the spec-compiler assigns final IDs.

1. **Spec-spine Requirements view in stagecraft.** The net-new
   `app.project.$projectId.requirements.tsx` route. Renders the
   project's local spec-spine. Groups specs dynamically via the
   relationship-graph projection. Supports custom cosmetic group
   names. Surfaces Knowledge → Requirements lineage. Provides
   review / edit / approve flows that promote draft specs to
   approved. Kind: `platform`.

2. **Pipelines → Development rename + lifecycle-state board.** The
   rename of `app.project.$projectId.pipelines.tsx` to Development
   plus the lifecycle-state board model from §5.3. Columns are spec
   lifecycle states; cards are specs or grouped clusters; overlays
   are governance-certificate emissions and factory run activity.
   Kind: `platform`.

3. **OPC decomposition pipeline.** The §6.2 stages 1–6 (extraction
   → xray fingerprint → semantic clustering → call graph → temporal
   lineage → LLM synthesis), the draft-spec emission contract, and
   the integration with stagecraft's Requirements view for
   review/approve. Kind: `capability` (with `references:` to the
   relevant registry specs).

4. **Dockerised execution substrate (ASI05).** The non-negotiable
   ephemeral micro-sandbox runtime for factory-engine adapter
   codegen execution. Pod spec under `platform/k8s/` with gVisor /
   Firecracker / strict-network-policy boundaries, TTL ceilings,
   `pids_limit`, memory + CPU caps, and no host mounts. Kind:
   `platform`; constrains the factory-engine codegen contract.

5. **Knowledge → Requirements provenance edge contract.** The
   `references:` edge typing for `kind: knowledge` and
   `kind: code-fingerprint`, the emission contract during
   decomposition (§6.2), the rendering contract in the Requirements
   view (§3.4). Kind: `governance` (refines spec 154's unit grammar).

6. **OPC Stop-hook gate chain.** Wiring the existing `make pr-prep`
   class checks as `PostToolUse` (cheap, staleness signal) and
   `Stop` (blocking, full validation) hooks in OPC's Claude Code
   surface. Reuses existing binaries via the governed-reads
   discipline. Kind: `platform`; refines spec 127.

7. **Factory / adapter relocation into stagecraft.** Closing out
   the in-flight migration named in §8.1. Establishes the
   stagecraft-resident location for adapter manifests, repoints the
   codebase-indexer input list, and excises legacy references.
   Kind: `platform`; co-authority with the relevant existing
   factory specs.

8. **Born-with spec-spine kernel emission.** Per §6.5, the
   factory-engine emits a spec-spine kernel (`specs/`,
   `standards/spec/`, `.derived/`, kernel-hash marker) into every
   produced project. Kind: `capability`; extends spec 120.

9. **Per-project governance certificate emission.** Per §7.4, every
   project the substrate produces or governs emits its own
   governance certificate under `.factory/runs/<run-id>/`. Sister
   verifier `verify-certificate` ships with it (FR-007 of spec 102).
   Kind: `capability`; extends spec 102.

10. **Per-tool JSON-Schema strictness enforcement.** Per §7.2 ASI02
    gap. `crates/tool-registry`-side validation that rejects any
    `ToolDef` registration whose schema carries `additionalProperties:
    true`, `type: any`, or unconstrained `object`. Kind: `governance`;
    refines spec 067.

11. **Signed inter-stage manifests in factory-engine.** Per §7.2
    ASI07 gap. Every hand-off between factory-engine stages is
    signed by the dispatching agent's ephemeral key. Kind:
    `capability`; refines spec 075.

12. **Deterministic structural-diff plan UI in OPC.** Per §7.2 ASI09
    gap. OPC renders agent plans as YAML/JSON action graphs and
    structural diffs against the current spine state, not as
    conversational summaries. Kind: `platform`; refines spec 076.

13. **Live agent-session introspection in OPC.** Per §7.2 ASI10 gap.
    Panel showing every connected agent session by scope, with
    force-disconnect capability. Kind: `platform`.

14. **OPC multi-session-by-project-path session model (formalised).**
    Per §4.2. Captures the discipline that's already implicit in OPC
    and binds it to the spec spine. Kind: `governance`; refines
    spec 052.

15. **Codification gate analogue.** Per the harness's
    `check-finding-codified.mjs` pattern. For every CRITICAL/HIGH
    finding generated by `axiomregent` / `provenance-validator` /
    `policy-kernel`, require a spec-spine entry before the
    conversation closes. Kind: `governance`.

The above list is intentionally **flat** — no priorities, no
sequencing. Sequencing is a separate exercise. The list is also
**non-exhaustive** in the sense that some refinements that surface
during real spec authoring will spawn additional small specs; that
is the iterative discipline §2.6 commits to.

---

## 10. Final dispositions

- This document supersedes the convergence doc's L1/L2 framing in
  §1 only. The convergence doc's translation matrix (§2 A–K) and
  ASI coverage matrix (§3) remain useful references for the
  decomposition pass.
- `owasp_top_10_agentic_applications_summary_oap.md` has been retired
  as part of this work. Anything load-bearing it carried has been
  absorbed here or already lives in the spec spine.
- The two AIDE blueprint documents
  (`AIDE-VELOCITY-blueprint-spec.md` and
  `AIDE-VELOCITY-HARNESS-blueprint-spec.md`) remain as faithful
  blueprints of the source systems and are not modified by this
  work.
- The short executive summary
  (`owasp_top_10_agentic_applications_summary.md`) is the only
  OWASP-side doctrine source. It remains.
- This document is the input to a decomposition pass that produces
  real specs under `specs/NNN-slug/spec.md`. No spec authored from
  this document inherits this document's authority — each new
  spec stands on its own under the constitution and Feature 000.
