---
id: "212-factory-schema-lockstep-ci"
title: "Factory Schema Lockstep CI (cross-repo contract parity, automated)"
feature_branch: "feat/212-factory-schema-lockstep-ci"
status: draft
implementation: pending
kind: governance
domain: tooling
created: "2026-06-11"
authors: ["open-agentic-platform"]
language: en
summary: >
  Spec 197 declares the factory Build Spec an open standard that the owned
  factory source (factory-encore) mirrors, and spec 197 AC-8 / spec 198
  both verified that mirror BY HAND ("verified directly against
  factory-encore origin/main @ cc1139f"). A hand diff is not a gate: the
  next factory-encore edit, or the next OAP schema bump, can silently
  diverge the two contract surfaces and nothing fails. This spec automates
  the lockstep: an OAP-side enforcing CI job that fetches factory-encore's
  contract/schemas/** at a committed pin and asserts FIELD-LEVEL structural
  parity against standards/schemas/factory/** (byte-equality is impossible —
  the surfaces carry divergent comments and org-specific example values by
  design), plus an FR-005 guard that no GoA-specific concept entered either
  contract surface. Two lanes: a PR-time check against the committed pin,
  and a scheduled check against factory-encore@main that catches upstream
  drift the pin hasn't yet absorbed. factory-encore stays gate-free
  (spec_spine: false); enforcement is unidirectional, OAP-side.
code_aliases: ["FACTORY_SCHEMA_LOCKSTEP_CI"]
compliance:
  - framework: "owasp-asi-2026"
    controls: ["ASI07"]
extends:
  # The spec-177 orchestrator gains a route dispatching the new job, the
  # same shape spec 191 used for ci-schema-parity.
  - spec: "177-ci-orchestrator-pr-gate"
    nature: additive
    unit: { kind: file, path: .github/workflows/ci.yml }
  # The new workflow is classified enforcing in ci-parity-check's
  # ENFORCING_WORKFLOWS so the Makefile↔CI run-mirror covers it (spec 104).
  - spec: "104-makefile-ci-parity-contract"
    nature: additive
    unit: { kind: file, path: tools/oap/ci-parity-check/src/lib.rs }
  # Same precedent as specs 196, 194, 193, 187, 183 and the 202–211 batch:
  # a new spec adds a row to the featuregraph golden.
  - spec: "034-featuregraph-registry-scanner-fix"
    nature: additive
    unit: { kind: file, path: crates/featuregraph/tests/golden/features_graph.json }
references:
  # The open-standard contract whose cross-repo mirror this gate makes
  # falsifiable. Spec 197 AC-8 is the manual check this automates; FR-005 is
  # the GoA-concept rejection this asserts mechanically. This spec does not
  # reshape 197's contract surface — it watches it — so this is a reference,
  # not an extends.
  - role: gate-declaration
    unit: { kind: file, path: specs/197-factory-contract-open-standard-extensions/spec.md }
  # The canonical schema set under lockstep. This gate reads these files; it
  # does not author or own them (specs 074/197/198 do).
  - role: context
    unit: { kind: directory, path: standards/schemas/factory }
  # The sibling cross-Rust↔TS parity tool whose CI wiring shape (spec 191)
  # this job mirrors — but whose fingerprint walker it does NOT reuse (that
  # tool never parses standards/schemas/factory/*.yaml; see §4).
  - role: context
    unit: { kind: directory, path: tools/oap/schema-parity-check }
  # The owned factory source whose contract/schemas/** are the far side of
  # the lockstep. spec_spine: false by declaration (upstream-map.yaml) — no
  # gates run there; this check is the OAP-side enforcement.
  - role: historical
    unit: { kind: directory, path: factory }
---

# Feature Specification: Factory Schema Lockstep CI

**Feature Branch**: `212-factory-schema-lockstep-ci`
**Created**: 2026-06-11
**Status**: Draft (WS4 of the OAP gap-closure pass)
**Input**: Spec 197's FR-006 promises the canonical YAML schema and the
owned factory source (`factory-encore`) stay "in lockstep", and AC-8
records that lockstep as *verified* — but the verification note reads
"verified directly against factory-encore `origin/main` @ `cc1139f`",
i.e. a one-time manual diff. Spec 198 §AC closes the first sealed
admission for `GovAlta-Pronghorn/factory-encore` and likewise leans on a
hand check. The lockstep that two approved specs depend on has never been
gated. This spec is that gate.

## Purpose

The factory Build Spec contract is an open standard (spec 197 FR-001):
OAP authors it under `standards/schemas/factory/**`, and `factory-encore`
mirrors it under `contract/schemas/**` because the owned factory consumes
the same contract OAP defines. Two approved specs assert the two surfaces
are identical, and both verified it by eye on a specific SHA.

A hand diff decays the instant either side moves. `factory-encore` is
`spec_spine: false` by declaration (`upstream-map.yaml`) — it runs **no**
spec-spine gates, by design, because it is plain OAP-conformant content,
not a governed spine. So the only place this lockstep can be enforced is
OAP's CI, looking outward. Without that, the failure mode is exactly the
one OAP's gates exist to prevent: a contract claim (spec 197 AC-8) that
was true at one commit and quietly stops being true one edit later, with
no signal. The same covenant-without-a-gate decay specs 209/211 name for
OAP's own verification loop, applied to the cross-repo contract surface.

This is **ASI07 (Insecure Inter-Agent / Inter-Component Communications)**
applied across the repo boundary: the factory and OAP exchange a typed
contract, and a silent skew on either side breaks that contract the same
way a duplex envelope-version skew breaks the desktop↔server stream
(spec 189). Spec 198 already credits schema-parity (125/191) for ASI07 on
the Rust↔TS axis; this closes the third axis — OAP↔factory-encore.

It is scoped to a **gate over existing artifacts**. It adds no new contract
field and changes no schema; it asserts that two already-authored surfaces
agree, and that neither has acquired a GoA-specific concept FR-005 forbids.

## Why not extend the existing schema-parity-check (§4 of the design)

The instinct is to extend `tools/oap/schema-parity-check` (specs 125/191).
It does not fit. That tool compares a **stagecraft TypeScript** descriptor
(`SchemaNode`, walked by `walk-descriptor.mjs`) against a **Rust**
fingerprint emitted by `crates/factory-contracts` during `cargo test`. It
never reads `standards/schemas/factory/*.yaml` — its inputs are `.ts`
modules and `.derived/schema-parity/*.json`, and it requires a TS runtime
(bun) precisely to import those modules. Cross-repo YAML↔YAML parity shares
none of that machinery: no Rust fingerprint, no TS import, no descriptor
export. Bolting a YAML-vs-YAML mode onto it would mean a second,
unrelated comparison engine wearing the same name, and would entangle the
spec-191 `schema_parity` route (scoped to `factory-contracts` + stagecraft
`knowledge/governance/sync`) with a route that has nothing to do with
those paths. A **separate, focused check** with its own route and its own
`ENFORCING_WORKFLOWS` entry is cleaner and keeps spec 191's gate
single-purpose. (Decision; the alternative — generalize the existing tool —
is recorded and rejected here.)

## Key design positions

### Pin source — two lanes (committed pin + cron against main)

**Decision: a committed pin file lives in OAP, read at PR-time; a second
scheduled lane checks `factory-encore@main` directly.**

The admitted `factory_sha` (spec 198) lives in the deployed stagecraft DB,
which CI cannot reach — rejected as the pin source. Three viable homes for
a committed pin were weighed:

1. **A standalone pin file** (e.g. `factory/factory-encore.pin`) — rejected:
   a bare ref in a non-markdown file is exactly the standalone-data channel
   the constitution Principle I forbids for authored truth.
2. **This spec's own frontmatter** — a single `pinned_ref` field carrying
   the SHA, read by the check. Honors Principle I (truth is markdown
   frontmatter) and Principle II is untouched (the check reads source, not
   a compiler artifact). **Chosen.** Bumping the pin is a spec edit, which
   the coupling gate already governs, so a pin move is reviewable and
   attributable by construction.
3. **The committed factory-encore checkout** — OAP does not vendor
   factory-encore; rejected (no such checkout exists, and adding one
   duplicates the contract OAP already authors).

A pin alone catches OAP-side drift but goes stale silently: if
factory-encore moves ahead, the PR lane keeps passing against the old pin
while the real surfaces diverge. So a **second lane** runs on a schedule
(cron) and on `workflow_dispatch`, fetching `factory-encore@main` and
running the same parity assertion. A red cron lane is the signal "upstream
moved; bump the pin and reconcile". This is the two-lane design: PR-time is
**pinned and blocking** (deterministic, no network flake gates a PR on
content the PR didn't touch); the cron is **against-main and advisory-to-a-
human** (opens/annotates an issue rather than failing a PR that has nothing
to do with the drift).

### Fetch auth — authenticated cross-org fetch, PAT secret

`gh api repos/GovAlta-Pronghorn/factory-encore` returns **404** for the
authenticated CI identity at spec time — the repo is private or org-gated
to the stagecraft-ing CI token's scope. The check therefore cannot assume
anonymous fetch. **Decision:** the workflow fetches via an authenticated
sparse checkout / `gh api` using a repo-scoped PAT stored as an Actions
secret (working name `FACTORY_ENCORE_RO_TOKEN`, read-only contents scope on
`GovAlta-Pronghorn/factory-encore`). If org policy later makes the repo
public, the token becomes optional and the workflow falls back to anonymous
fetch — but the spec assumes private until verified otherwise.
**Fail-visible:** a missing or unauthorized token fails the job with
"cannot fetch factory-encore at <ref>; check FACTORY_ENCORE_RO_TOKEN" — it
is **never** skipped-green (the spec 200 FR-004 / spec 209 FR-005 posture).
Only the cron lane needs the network every run; the PR lane needs it only
when a lockstep-set file actually changed (the route below), bounding the
secret's blast radius and the flake surface.

### Semantic diff — field-level structural parity, not byte-equality

A live diff of the two surfaces at spec time shows they are **not**
byte-equal and were never meant to be: comment blocks differ, and
adapter-manifest example values are deliberately org-specific
(`aim-vue-node`/`node-22`/`express-5` in OAP's stack-agnostic illustration
vs `aim-vue-encore`/`node-24`/`encore-ts` in factory-encore's reference
adapter). Byte-equality would fail on text that FR-001 *wants* to differ.

**Decision:** parse both YAML/JSON files and compare their **structural
field set** — field names, requiredness, enum value sets, nesting — while
ignoring comments and the values of free-form example/illustration scalars.
This mirrors the existing tool's structural-only philosophy
(`walk-descriptor.mjs`: "value-shape constraints are NOT carried through")
applied to YAML. The comparison is the contract shape, not the prose around
it. Where a real structural divergence exists (see the lockstep-set note
below), the gate names the field path and the side that carries it, in the
spec-191 diff-reporting style (`<path>: present in OAP only`).

### The lockstep set (and a finding the gate must encode)

The contract files under lockstep are the **org-agnostic contract surface**
spec 197 governs:

- `build-spec.schema.yaml` — the open-standard contract (spec 197 FR-006).
- `adapter-manifest.schema.yaml` — the adapter contract **shape**; its
  example *values* are org-specific (above) and are excluded from
  structural compare by the value-ignoring rule.
- `pipeline-state.schema.yaml`, `verification.schema.yaml` — currently
  byte-identical across both repos (verified at spec time); cheap to keep
  pinned.
- `stage-outputs/**` (`audiences`, `business-rules`, `entity-model`,
  `sitemap`, `use-cases`) — structurally identical at spec time
  (`audiences` differs only in a description string).

**Finding the gate encodes:** OAP carries
`governance-envelope.schema.yaml`, and `factory-encore` does **not** mirror
it yet, while spec 198 §AC states factory-encore "must file a conformant
envelope". The lockstep set therefore distinguishes **two tiers**:

- **Tier A — must match if present on both sides** (the contract surfaces
  above). Divergence is a hard failure.
- **Tier B — present-on-OAP, expected-but-absent on factory-encore**
  (`governance-envelope.schema.yaml` today). The gate reports this as a
  **named, expected gap** with the spec-198 obligation cited — advisory in
  the PR lane (it is a known authoring debt, not a regression a PR
  introduced), and the cron lane's signal that the gap has been closed when
  the file appears and must then graduate to Tier A. The gate must not let
  Tier B silently mask a *real* Tier-A divergence — gap classification is
  per-file and explicit, never a catch-all.

### FR-005 GoA-concept guard — denylist token scan, both surfaces

Spec 197 FR-005 rejects two GoA concepts from the contract: **security
classification labels** (`Public`/`Protected A`/`Protected B`/`Protected
C`) and the **external service catalogue** (the GoA OpenAPI/capability
taxonomy). Spec 197 AC-6 already asserts no such token appears in
`standards/schemas/factory/` or `crates/factory-contracts/src/` — but only
for the OAP side, and only by intent. **Decision:** a denylist token scan
(case-insensitive, word-boundary) over **both** the OAP and the fetched
factory-encore contract surfaces, with the denylist defined by citation to
spec 197 FR-005 (`Protected\s+[ABC]`, classification-label and
service-catalogue identifiers). A match on either side fails the lane
naming the file, line, and the FR-005 clause it violates. This is a guard,
not a parser — it does not understand YAML; it asserts the rejected
vocabulary never entered the open standard on *either* repo, which is the
mechanizable reading of "no GoA-specific concepts in the contract layer".
The denylist is the gate's single source of forbidden vocabulary and cites
FR-005 as its authority, so widening it is a spec-coupled edit.

### Free-disk-space composite — not needed (recorded)

The spec-135 FR-05a `free-disk-space` composite exists
(`.github/actions/free-disk-space`) for disk-heavy Rust/Docker jobs. This
job is a sparse fetch plus a small Node/bun YAML compare — no large
toolchain, no Docker image, no `target/`. It does **not** use the composite,
and recording that here pre-empts the reflexive "add free-disk-space"
review note.

## Functional requirements (sketch — refine before implementation)

- **FR-001 — Cross-repo lockstep check (the tool).** A small OAP-side
  checker (working name `factory-schema-lockstep`, sibling to
  `tools/oap/schema-parity-check`, **not** an extension of it per §4) that,
  given a local factory-encore contract tree, parses each Tier-A file on
  both sides and asserts field-level structural parity, ignoring comments
  and free-form example values. Divergence exits non-zero naming the file
  and field path; Tier-B gaps are reported with their spec-198 citation and
  do not fail the PR lane.
- **FR-002 — FR-005 GoA-concept guard.** The same tool scans both contract
  surfaces for the spec-197-FR-005 denylist vocabulary; a match fails
  naming file:line and the FR-005 clause. The denylist cites FR-005 as its
  authority.
- **FR-003 — PR lane (pinned, blocking).** A reusable workflow dispatched
  from the spec-177 orchestrator on the lockstep route (below) fetches
  factory-encore at the pin in this spec's frontmatter (`pinned_ref`),
  runs FR-001/FR-002, and blocks the PR on a Tier-A divergence or an
  FR-005 hit. Runs identically in `merge_group`. SHA-pinned action refs
  (spec 158). Fail-visible on fetch/auth failure (never skipped-green).
- **FR-004 — Cron lane (against main, human-routed).** A scheduled
  (+ `workflow_dispatch`) lane fetches `factory-encore@main`, runs the same
  assertions, and on divergence opens/annotates a tracking issue ("upstream
  drifted from pin `<ref>`; reconcile and bump") rather than failing an
  unrelated PR. Catches the stale-pin failure mode the PR lane structurally
  cannot.
- **FR-005 — Makefile mirror + parity classification (spec 104).** A
  `make factory-schema-lockstep` target mirrors the PR-lane recipe; the
  workflow is added to `ci-parity-check`'s `ENFORCING_WORKFLOWS` with
  aligned/divergent fixtures proving drift detection; the target joins
  `make ci-strict`. Whether it also joins fast `make ci` is a measured
  spec-135 decision — it needs a network fetch, so default to strict-only
  unless the fetch is cheap enough and reliable enough for the ~5-minute
  budget, with the measurement recorded (the spec 211 FR-002 rule).
- **FR-006 — Pin lives in frontmatter (Principle I).** The checked
  factory-encore ref is a `pinned_ref` field in this spec's frontmatter, not
  a standalone data file. Bumping it is a coupling-gated spec edit, so a pin
  move is attributable and reviewable.

## Acceptance criteria (sketch)

- **AC-1.** A PR that edits `standards/schemas/factory/build-spec.schema.yaml`
  to add/remove/rename a field, or change an enum set, without the matching
  edit landing in factory-encore at the pinned ref, fails the PR lane naming
  the field path and the side that carries it.
- **AC-2.** Comment-only or example-value-only differences between the two
  surfaces (the `aim-vue-node` vs `aim-vue-encore` class of divergence) do
  **not** fail — structural parity ignores prose and free-form example
  scalars.
- **AC-3.** A GoA-specific token (e.g. `Protected B`, or a service-catalogue
  identifier from FR-005) introduced into either contract surface fails the
  guard naming file:line and the FR-005 clause.
- **AC-4.** `governance-envelope.schema.yaml` present on OAP and absent on
  factory-encore is reported as a named Tier-B expected gap citing spec 198
  — advisory in the PR lane — and does not mask a Tier-A divergence in the
  same run (gap classification is per-file, proven by a fixture that pairs a
  Tier-B gap with a Tier-A break and asserts the run still fails on the
  break).
- **AC-5.** The cron lane, run against a `factory-encore@main` that has
  drifted ahead of the pin, opens/annotates a tracking issue and does not
  fail an unrelated PR; bumping `pinned_ref` to the new ref turns the PR
  lane green again.
- **AC-6.** A missing/unauthorized `FACTORY_ENCORE_RO_TOKEN` fails the job
  with the fetch diagnostic — never skipped-green (spec 200 FR-004 posture).
- **AC-7.** `ci-parity-check` is green with the new workflow classified
  enforcing: the Makefile mirror exists token-for-token, and the divergent
  fixture proves drift detection (spec 104).
- **AC-8.** The job runs in `merge_group` with the same blocking semantics
  as on PRs (spec 177 gate composition, no PR-only carve-out); action refs
  are SHA-pinned (spec 158).

## Out of scope

- **Contract field changes.** This gate watches the contract; specs
  074/197/198/210 own its shape. It adds and removes no field.
- **factory-encore-side CI.** factory-encore is `spec_spine: false` by
  declaration; enforcement is unidirectional, OAP-side. This spec does not
  add gates to factory-encore.
- **The Rust↔TS structural-parity gate** (specs 125/191) — a different axis
  on different files; this spec adds the third (OAP↔factory-encore) axis and
  does not touch the first.
- **Admitted-`factory_sha` reconciliation** (spec 198). The deployed
  admission SHA is a runtime fact in the stagecraft DB; this gate's pin is a
  build-time, CI-reachable surrogate. Aligning the two is a separate concern.
- **Generalizing `schema-parity-check`** — recorded and rejected in §4.

## Sequencing

Implementable now — every dependency is landed machinery: the contract
surfaces exist on both sides at a known SHA (spec 197 AC-8 verified
`cc1139f`), the spec-177 orchestrator already routes added jobs (the
spec-191 precedent), `ci-parity-check` already classifies enforcing
workflows, and a read-only cross-org fetch is a solved CI pattern given the
PAT secret. Per the gap-batch convention, the relationship edges above point
only at verified existing paths; the `establishes:` edges for the new tool,
the workflow file, and the parity fixtures ride the implementation PR that
creates them (the spec 191 precedent). The `pinned_ref` field is added to
this spec's frontmatter when the implementation lands, pinned to the then-
current factory-encore contract SHA.
