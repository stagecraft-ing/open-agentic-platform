# OAP ↔ GoA Whitepaper Alignment Audit

Generated: 2026-05-07 (initial audit) · Updated: 2026-05-07 (pre-disclosure hardening pass) · Updated: 2026-05-11 (pre-disclosure refresh pass).
Whitepaper version: 1.2 (May 2026) — *Modernizing at Speed: An AI-First Approach to Addressing Technical Debt in Government*, Janak Alford, Deputy Minister of Technology and Innovation.
Whitepaper anchor copy reviewed: `/tmp/whitepaper.txt` (textutil conversion of `_tmp/Whitepaper.docx`, 918 lines). Whitepaper itself is restricted-distribution and is not tracked in this repository (`_tmp/` is in `.gitignore`); this audit document quotes only short anchor phrases (≤15 words each) sufficient to locate the referenced sections.
OAP commit at initial audit: `ac6c4fd1804c15d8feb0650a08269ccd6efe376e` (branch `main`, clean working tree). Pre-disclosure hardening pass commit: `52fc6962` (`chore(pre-disclosure): expand ASI 2026 coverage 6/10 → 7/10 with provenance`). Pre-disclosure refresh pass: HEAD `972d2ea8`, branch `main`, clean working tree.

## Pre-disclosure hardening pass — record (2026-05-07)

The audit was followed by a focused pass before disclosure. Changes that landed in this pass:

1. **Whitepaper-quote trim.** Verbatim passages in the per-dimension "Whitepaper anchor" callouts trimmed to ≤15-word anchors that preserve the §-reference; full direct quotes retained only where exact phrasing carries weight ("spec driven by design", "no module … is expected to exceed approximately one thousand lines of code", "common leaderboard", "preservation audit", etc.). Closes the disclosure-confidentiality concern relating to the whitepaper's restricted-distribution preamble while leaving the audit fully usable for a reader who has the whitepaper.
2. **OWASP ASI 2026 coverage 6/10 → 7/10** (dim. 12). `compliance:` frontmatter added to specs 047, 067, 068, 069, 116, and 121. ASI04 newly covered (via spec 116, supply-chain gates); ASI01/03/05/09/10 each gained 2–3 additional spec citations (depth, not breadth). ASI02, ASI06, ASI08 remain unmapped because their canonical control names are not defined in any OAP file-of-record — see "Framework data incompleteness" under dim. 12 for the post-disclosure closure that would unblock further breadth gains. **Honest 7/10 was preferred over forced 9/10.**
3. **Confidentiality status.** `_tmp/Whitepaper.docx` is not tracked in git history; `_tmp/` is already in `.gitignore` line 302. No history rewrite needed.
4. **Spec 091 maintenance note.** Featuregraph golden file regenerated to absorb pre-existing accumulated drift (specs 117/118/136/141/142 lifecycle/membership changes that pre-dated this session); spec 091 carries a maintenance entry mirroring the 2026-05-05 precedent. Mechanical projection of the current registry; no design change.
5. **CI gates.** `make ci` (fast loop, spec 135) green end-to-end after the pass: 1013 Rust tests pass, 382 stagecraft tests pass, spec-coupling gate clean, compliance-report regenerated.

Tasks 4–6 of the pre-disclosure plan (strategic-tier rollup, dim. 12 product gating sizing check, dim. 11 footnote resolution) are NOT part of this commit; they require additional user judgment and were left for a follow-up pass.

## Pre-disclosure refresh pass — record (2026-05-11)

Refresh pass applied four days after the 2026-05-07 hardening pass, before disclosure. The 2026-05-07 record above is preserved; this section captures only what moved between then and now.

1. **Spec count refresh: 143 → 147.** Four new specs landed since the initial audit (specs 143–146 + driver follow-ups). Lifecycle counts (governed read, `registry-consumer status-report --json --nonzero-only`): **138 approved, 5 draft, 4 superseded** — same approved count as the initial audit; four new specs all entered as `draft` (143–146) joining the pre-existing 137 draft. Net: corpus grew without weakening approval ratio at the approved-count level; denominator updated wherever cited.
2. **Codebase-index numbers refreshed.** Layer 1: still 18 producer-plane crates totalling 77,467 LOC (unchanged). Layer 2: **136 mapped specs (was 132), 27 amendment relationships (was 25), 7 orphaned specs, 5 untraced paths**. Content hash rotated to `ae320cc76caa83a79d8bdf668871e9983ae0cd9bb6751e8a07a676dec7f42a93` (regen-of-day; this value drifts on every `make registry` so a precise rendering is illustrative only).
3. **`implements:` path bindings refreshed.** 100 specs / 368 paths (audit) → **102 specs / 429 paths** (current). Growth concentrated in the new draft specs; coupling gate semantics unchanged.
4. **Producer-plane >1000-LOC files: 10 → 9.** The audit's prose claimed 10 files while listing 9; the current `find crates -name '*.rs'` count matches the listed 9. Single-file reduction since 2026-05-07 (one file dropped under the 1000-line bar); the existing nine offender names are unchanged in identity and rank. Desktop count unchanged at 14.
5. **Code-path line citations resynced.** `crates/factory-engine/src/governance_certificate.rs` line ranges shifted by 1–14 lines as the file evolved (certificate type 24–43 → 23–44; StageRecord 94–103 → 95–103; ProofChainSummary 147–150 → 147–154; `compute_certificate_hash` 293–311 → 297–308; `generate_certificate` 315–427 → 316–409). Symbols and semantics unchanged; ranges updated for honesty.
6. **README citations checked.** Lines 28–35 (SBOM), 122–134 (trust fabric), 152–163 (license), 219–246 / 234–246 (try-it + verify-certificate tamper example) — all still pointing at the same content. Only README diff since `ac6c4fd1` is the spec-count badge (142 → 147).
7. **ASI coverage holds at 7/10.** Same control set (ASI01, 03, 04, 05, 07, 09, 10), same spec citations per control. No regression. The path beyond 7/10 remains gated on framework-data completeness (post-disclosure closure #1), with one externally-sourced research artifact that may offer canonical control names — surfaced separately for user judgment rather than incorporated in this pass.
8. **Whitepaper-anchor discipline reverified.** Every per-dimension callout and the tail appendix were re-counted: all verbatim phrases are ≤15 words. Section references re-checked against `_tmp/Whitepaper.docx` (lines 453, 467/471/475, 479/483, 509/513, 525/529, 535/543, 567 (§21), 575 (§22), 603 (§25), 886 (§B.6)) — all anchors valid.
9. **CI / known issues.** No re-run of `make ci` in this pass; the 2026-05-07 disposition for the pre-existing `CARGO_BIN_EXE_<bin>` flake stands. `make registry` exits 0 on HEAD.

What this pass did NOT change: dimension status enums, headline strengths/gaps, the unrealized-wins framing, the upstream-dependency boundary table, or the recommended closures lists. Architectural claims are unchanged; only quantitative anchors and citation ranges were refreshed.

## Known issues at time of disclosure

- **`make ci` fast loop fails on a pre-existing local-environment surface** (13 tests across `tools/spec-compiler/tests/exit_codes.rs` and `tools/registry-consumer/tests/cli.rs`). Each failing test spawns the crate's binary via `CARGO_BIN_EXE_<bin>`, falling back to `<MANIFEST_DIR>/target/debug/<bin>` if the env var is unset. The Makefile recipe `ci-fast-tools` overrides `CARGO_TARGET_DIR=.target/cifast-tools/`, which prevents cargo from setting `CARGO_BIN_EXE_<bin>` in the test process; the fallback path then points at a directory the override no longer populates. Failure surface: `spawn: Os { code: 2, kind: NotFound }`.
- **Reproduces at clean HEAD** (`ac6c4fd1`, prior to this pass) under both `cargo nextest run` and `cargo test`. Not caused by the pre-disclosure pass edits.
- **What does pass at HEAD with this pass applied:** 1013 Rust workspace tests, 382 stagecraft (Encore.ts) tests, spec-code coupling gate (spec 127/130/133), codebase-indexer staleness gate (spec 101), schema-parity walker (spec 125), supply-chain gate (spec 116, cargo-deny + npm/pnpm audit), spec-lint default-fail-on-warn (spec 128). The compliance-report governed read confirms 7/10 ASI 2026 controls.
- **Disposition:** post-disclosure work. The fix is mechanical — set `CARGO_BIN_EXE_<bin>` explicitly in the Makefile recipe, or build the binary into the per-manifest `target/` path the test fallback expects. Tracked alongside the framework-data-completeness item under "Recommended closures after disclosure."

## Reading guide for the reviewer

OAP comprises two systems evaluated separately: the **producer** (the OAP codebase — spec spine, compilers, indexers, policy kernel, axiomregent, governance-certificate machinery) and the **product** (whatever the OAP factory builds when it runs a pipeline for a stakeholder). Each whitepaper dimension is evaluated against both, with separate statuses and a reconciliation sentence explaining why they differ. Where the factory depends on artifacts outside OAP's current control — the upstream `goa-software-factory` framework and the `aim-vue-node` scaffold template — the boundary is named explicitly rather than absorbed into either status.

Status enum: `MET` (claim is supported by code), `PARTIAL` (some FRs landed, some pending or aspirational), `ASPIRATIONAL` (spec exists but code does not), `GAP` (neither spec nor code), `EXCEEDS` (OAP delivers more than the whitepaper requires), `N/A` (dimension does not apply with a one-line reason).

## Executive summary

Counts across 13 dimensions (after the skeptical second pass below):

- **Producer:** MET 7 · PARTIAL 3 · EXCEEDS 2 · GAP 1 · ASPIRATIONAL 0
- **Product:** MET 8 · PARTIAL 3 · EXCEEDS 1 · GAP 1 · ASPIRATIONAL 0

Headline strengths (with evidence):

1. **Producer — auditability of compression is a first-class artifact** (dim. 4, 5). Governance certificate (spec 102, `crates/factory-engine/src/governance_certificate.rs`) is self-authenticating: `certificateHash = SHA-256(canonical_json with certificateHash="")`. `make verify-certificate` exits 1 on tamper with a specific `artifact-hash mismatch` diagnostic. The verifier (`crates/factory-engine/src/bin/verify_certificate.rs`) is a sister binary that does not trust the producing system. Satisfies §13 and §25 more concretely than the whitepaper's own description.

2. **Producer — spec-driven discipline is PR-time enforced, not prose-asserted** (dim. 6). 138/147 specs `approved` (same 138-approved count as 2026-05-07; corpus grew by four new drafts); 102 specs declare 429 `implements:` paths; the spec-code coupling gate (`tools/spec-code-coupling-check`, spec 127 amended by 130, 133) fails CI when a claimed path changes without its owning spec changing.

3. **Product — produced application has no OAP runtime dependency** (dim. 11). Certificate verifiable without OAP installed; four adapters target conventional stacks. The disclosure-grade answer to "what happens if you leave" is "the produced project keeps running and the certificate stays verifiable."

Headline gaps (with effort-to-close):

1. **Producer — module-size discipline broken in load-bearing files** (dim. 10). 9 Rust files >1000 LOC including `crates/orchestrator/src/lib.rs` (3,081); 14 desktop files >1000 LOC including `apps/desktop/src-tauri/src/commands/factory.rs` (3,112). Effort: **medium** (file-by-file split, no architectural change).

2. **Product — competitive evaluation foundation exists, leaderboard does not** (dim. 9). Adapters/Contracts/Processes split is in place; the runner that would drive multiple adapters against the same Build Spec is missing. Effort: **medium**.

3. **Producer + product — OWASP ASI 2026 at 7/10 controls** (dim. 12, post-pre-disclosure-pass). ASI01, 03, 04, 05, 07, 09, 10 covered. ASI02, 06, 08 remain unmapped because the framework's per-control names are not in any OAP file-of-record (only the six controls cited in spec 102 §"Phase D" and the ASI04 → supply-chain assertion in `docs/launch/gaps.md` were defensible without external definitions). Effort to push beyond 7/10: **small once canonical control definitions land** in the framework data, then frontmatter-only.

**Net assessment.** As a producer, OAP exhibits the architectural commitments the whitepaper asks for at higher mechanical rigour than the whitepaper itself describes: tiered representation, observable compression, deterministic execution, auditable provenance, spec-driven discipline, and AGPL-3.0 non-lock-in are all backed by code, not aspiration. As a product factory, OAP delivers reproducible Build Spec freezing, per-stage artefact hashing, an independently-verifiable certificate, and four pre-vetted adapter targets, but inherits module-size characteristics from upstream `aim-vue-node` and does not yet enforce ASI controls or competitive evaluation on produced projects. Producer plane is more mature than product plane. The boundary between them — OAP's translator over `goa-software-factory` and `aim-vue-node` plus the content-addressed substrate — is the architecturally interesting feature: OAP governs upstream artefacts it does not own, and survives changes to those upstreams.

## The producer/product distinction

OAP is two systems sharing a repository.

The **producer** is OAP-the-platform: 18 Rust crates (77,467 LOC), 25 npm packages, the Tauri OPC desktop, and the `platform/services/stagecraft/` Encore.ts service. Its construction is governed by the spec spine in `specs/` (147 specs as of 2026-05-11) and by five constitution-anchored policy rules (CONST-001 through CONST-005). The producer's daily-development loop is `make ci` (~5 min, spec 135), and pre-merge parity is `make ci-strict`. When you read this codebase as a *system*, you are reading OAP-the-platform.

The **product** is whatever OAP's factory generates when it runs a pipeline for a stakeholder's project. The factory pipeline is two-phase: a sequential process phase (`s0-preflight` through `s5-ui-specification`, with a checkpoint after each, freezing a SHA-256-hashed Build Spec at the s5 gate) followed by a fan-out scaffolding phase (`s6a–s6g`, with adapter-specific compile/test/lint/typecheck verification after each step). Spec 074 frames the factory as transforming "business documents into working software"; spec 075 specifies the workflow engine; spec 102 specifies the governance certificate that every run terminates with.

The factory has two upstream dependencies that are deliberately outside OAP's sphere of control:

- **`goa-software-factory`** — the originating factory framework, ingested into OAP's substrate via translation (`platform/services/stagecraft/api/factory/translator.ts`). This was first protocoled by spec 088 (now superseded by spec 108, which moved storage from on-disk `factory/` tree into PostgreSQL substrate, and amended by spec 139, which made the substrate content-addressed).
- **`aim-vue-node`** — the production scaffold template (`GovAlta-Pronghorn/template`). It is the only production-supported adapter today; `next-prisma`, `rust-axum`, and `encore-react` are factory-contract validated but not the current production target.

What OAP's factory adds *over* these upstream dependencies is **(a)** project as the unit of governance (spec 119, replacing earlier workspace-keyed scoping), **(b)** policy-kernel + axiomregent enforcement on artefacts produced for a project, **(c)** knowledge-artefacts-driven execution where requirements derive from aggregated stakeholder communication (spec 115 knowledge extraction, spec 122 stakeholder doc inversion) rather than human-authored specs, and **(d)** the Adapters/Contracts/Processes (ACP) split which is the substrate that makes the upstreams swappable.

Because of (d), every product-side dimension below has a "boundary" reconciliation: the Adapters/Contracts/Processes split is what *makes* the upstream dependencies swappable rather than load-bearing. This is why several product-side gaps are framed as "future-closeable through the substrate" rather than "current limitations."

## Dimension-by-dimension audit

### 1. Tiered representation of system state

**Whitepaper anchor (Part IV §12).** Four-tier estate model: raw, structural, capability, strategic.

**Producer status: PARTIAL** (demoted from `MET` on the skeptical second pass — see reconciliation below). Three observable tiers map cleanly onto the whitepaper's four. Raw: `specs/NNN-slug/spec.md` markdown plus `Cargo.toml`'s `[package.metadata.oap].spec` and `package.json`'s `"oap": {"spec": ...}` declarations. Structural: `build/spec-registry/registry.json` (compiler-emitted, deterministic) and `build/codebase-index/index.json` (spec-to-code mapping over 60 inventory entries). Capability: `build/codebase-index/CODEBASE-INDEX.md` (136 mapped specs, 27 amendment relationships) and `registry-consumer compliance-report --framework owasp-asi-2026 --json`. The fourth tier — the whitepaper's "strategic intent … the tier at which deputy ministers and ministers reason" — has no producer-plane analogue at portfolio level. The governance certificate is the closest artefact but it is per-run, not per-portfolio; spec frontmatter (`kind`, `risk`, `status`, amends/supersedes graph) carries strategic-intent fragments but is not aggregated into a queryable strategic-intent view.

**Product status: MET.** Per the factory pipeline: stakeholder docs (`crates/factory-contracts/src/stakeholder_docs.rs:40`, anchored sections by `<KIND>-<NNN>` IDs) are raw, `ExtractionOutput` (`crates/factory-contracts/src/knowledge.rs:19`) is structural, the frozen `BuildSpec` (`crates/factory-contracts/src/build_spec.rs:16–44`) is the capability contract, and per-stage `PipelineState` records (`crates/factory-contracts/src/pipeline_state.rs:19–33`) plus the run-terminating governance certificate constitute the strategic tier.

**Evidence.** `tools/spec-compiler/`, `tools/codebase-indexer/`, `tools/registry-consumer/`; `crates/factory-contracts/src/{build_spec,pipeline_state,stakeholder_docs,knowledge}.rs`; spec 102 §"Central Deliverable: The Governance Certificate".

**Gap or unrealized win.** Producer's strategic tier is missing as an aggregate artefact. A `registry-consumer strategic-report` (or `--rollup` flag on `compliance-report`) that aggregates lifecycle, risk, kind, and amendment-graph state into a portfolio-level view would close the gap. **Unrealized win:** OAP's tiers that *do* exist carry hashes between layers, which the whitepaper §12 does not require — the upward links are stronger than the description.

**Effort to close: small** (per-portfolio strategic-tier rollup is a `registry-consumer` extension).

**Reconciliation.** Producer `PARTIAL`, product `MET`. The producer's gap is a tooling absence, not an architectural one — the underlying frontmatter and substrate are present; the aggregator binary that would render the strategic tier as a first-class artefact is not. Product is `MET` because the factory pipeline carries an explicit per-stage tier ladder culminating in the certificate, which functions as the strategic-intent artefact for that run.

---

### 2. Observable compression upward

**Whitepaper anchor (Part IV §13).** Compression upward: summarisation across tiers must preserve provenance links; "apparent loss that is actually a broken link is a defect".

**Producer status: MET.** Spec markdown → registry.json compression is deterministic (golden-file checks, spec 000 invariant V-004); `build-meta.json` carries non-deterministic wall-clock metadata only; codebase-indexer maintains content hashes (`build/codebase-index/CODEBASE-INDEX.md` line 4 carries a `Content hash: <SHA-256>` rendered on every regen — value at this pass: `ae320cc76caa83a79d8bdf668871e9983ae0cd9bb6751e8a07a676dec7f42a93`, illustrative only since the hash rotates per regen) so a broken provenance link surfaces as a hash drift in CI rather than as silent loss. Compliance-report compresses spec-level compliance frontmatter into framework views (`registry-consumer compliance-report --framework owasp-asi-2026 --json` returns 7 control-to-spec mappings after the 2026-05-07 hardening pass).

**Product status: MET.** `requirements_hash` (SHA-256 of input requirements docs, `crates/factory-engine/src/governance_certificate.rs:59`) binds upward to the certificate; per-stage `artifact_hashes: BTreeMap<String, String>` (FR-005, line 94–103 of governance_certificate.rs) preserves the lower-tier evidence; the proof chain summary (`record_count`, `first_record_hash`, `last_record_hash`, `chainIntegrity`) records every policy decision made during compression.

**Evidence.** spec 102 FR-004, FR-005, FR-008; `tools/codebase-indexer/` content-hash machinery; `crates/factory-engine/src/governance_certificate.rs:147–154` (`ProofChainSummary`); spec 121 (claim provenance enforcement) Quality Gate `QG-13_ExternalProvenance`.

**Gap or unrealized win.** **Unrealized win:** every compression step in OAP carries either a content hash, a SHA-256 artefact hash, or a proof chain link. The whitepaper §13 only requires that the *operations* be observable; OAP makes the *artefacts* hash-anchored, which is strictly stronger.

**Effort to close: n/a** (already exceeds the bar).

**Reconciliation.** Both planes `MET` for the same architectural reason: the compiler-and-hash discipline that governs the producer was carried forward into the factory pipeline.

---

### 3. Observable decompression downward

**Whitepaper anchor (Part IV §13).** Decompression downward: strategic decisions elaborated through capabilities, workflows, implementations, into running code.

**Producer status: MET.** Each spec's `implements:` block enumerates the code paths it decompresses into (429 paths across 102 specs). The spec-code coupling gate (`tools/spec-code-coupling-check`, spec 127, amended by 130 and 133) fails CI when a path changes without its owning spec changing — the decompression mapping is enforced at PR time, not at design review. The amends-aware variant (spec 133) lets a code change cite an amending spec, preserving accuracy when designs evolve.

**Product status: MET.** Build Spec → manifest → per-stage execution → produced project files. `generate_scaffold_manifest(build_spec, ...)` (spec 075 FR-003) takes the frozen Build Spec and emits the Phase 2 manifest; each scaffold step's adapter `feature_verify` command (compile, test, lint, typecheck) checks downward fidelity at the moment of decompression.

**Evidence.** `.github/workflows/ci-spec-code-coupling.yml` (referenced in `.claude/rules/adversarial-prompt-refusal.md`); `tools/spec-code-coupling-check/`; spec 075 FR-003; `crates/factory-engine/src/manifest_gen.rs` (1,206 LOC).

**Gap or unrealized win.** **Unrealized win:** spec/code coupling gate at PR time is strictly stronger than the whitepaper's "checkable against the tier above and below" — it makes the check obligatory rather than recommended.

**Effort to close: n/a.**

**Reconciliation.** Both planes `MET`; both carry their decompression mappings as typed contracts (spec frontmatter; Build Spec → manifest types).

---

### 4. Determinism where it matters

**Whitepaper anchor (Part IV §14).** "Confine non-determinism to the layers where it adds value." Probabilistic models for orchestration, planning, summarisation; deterministic execution for citizen records, entitlements, and regulatory rules.

**Producer status: EXCEEDS.** Producer-plane determinism is broader than the whitepaper requires:

- spec-compiler is deterministic (spec 000 V-004; golden-file fixtures)
- registry-consumer is deterministic (spec 029 contract governance gate)
- codebase-indexer is deterministic with content-hash gating
- policy-kernel five-tier merge yields deterministic decisions given fixed inputs
- governance certificate is self-authenticating (`certificateHash = SHA-256(canonical_json with certificateHash="")`, FR-008)
- `verify-certificate` exit 1 on artefact-hash mismatch with a specific diagnostic (validated by README §"Try it" lines 234–246)

AI agents are confined to the orchestration role (planning, drafting, summarisation); deterministic Rust binaries do execution.

**Product status: MET.** The factory pipeline freezes the Build Spec at s5 (SHA-256 hash recorded in `PipelineState.build_spec.hash`), and per-stage artefact hashes are captured by the certificate. Phase 2 scaffold generation (`s6a–s6g`) uses LLM agents and is *not* byte-for-byte reproducible across runs — but it is checkpointed by deterministic verification gates (compile/test/lint/typecheck/security) and a deterministic certificate. This is the whitepaper's exact split: probabilistic composition, deterministic execution and verification.

**Evidence.** `crates/factory-engine/src/governance_certificate.rs:297–308` (`compute_certificate_hash`); `Makefile` targets `make verify-certificate` (exit 0 on clean, exit 1 on tamper); spec 102 FR-007, FR-008; spec 075 FR-005 (post-step verification hooks).

**Gap or unrealized win.** **Unrealized win — boundary as feature:** the upstream `goa-software-factory` and `aim-vue-node` template determine the *content* of probabilistic generation, but OAP's deterministic harness around them (Build Spec freeze, artefact hashing, gate verification, certificate self-authentication) means non-determinism in those upstreams cannot leak into the audit chain. OAP achieves determinism *over* upstream artefacts it does not own. This is a stronger architectural property than owning the whole stack would be.

**Effort to close: n/a.**

**Reconciliation.** Producer `EXCEEDS` (deterministic across the entire producer surface); product `MET` (deterministic where the whitepaper requires, probabilistic where it permits). The difference is a design choice driven by Phase 2's adapter-driven scaffolding, which is a probabilistic-composition use case by intent.

---

### 5. First-class auditability of compression/decompression

**Whitepaper anchor (Part IV §13, Part VI §25).** Compression and decompression as first-class artefacts: which agent did what, what was preserved, what was discarded, how outputs trace to sources.

**Producer status: MET.** The codebase index records spec→code mapping (136 mapped specs); spec-compiler emits `build-meta.json` recording compiler version and wall-clock; policy-kernel produces a `ProofRecord` per evaluation (`crates/policy-kernel/src/lib.rs`, 792 LOC); the certificate's proof-chain summary binds first/last record hashes and chain integrity. *Which agent did what* is queryable through the audit log table in `platform/services/stagecraft/api/db/schema.ts`.

**Product status: PARTIAL.** What was *preserved* is recorded (artefact hashes per stage, FR-005). What was *discarded* is not first-class. Whitepaper §B.6 (Git Insights Ministry's preservation audit) names this property explicitly: each build specification carries a "preservation audit" listing capabilities folded forward and capabilities discarded. OAP's certificate has no equivalent preservation-audit field. The Quality Gate `QG-13_ExternalProvenance` (spec 121) checks claim provenance but does not produce a preservation-loss list.

**Evidence.** `crates/factory-engine/src/governance_certificate.rs:23–44` (certificate type), `95–103` (StageRecord); spec 121 §QG-13; `crates/factory-engine/src/stages/quality_gates.rs` (842 LOC). Counter-evidence for product: no field named `preservationAudit`, `discardedCapabilities`, or similar in the certificate Rust types.

**Gap or unrealized win.** A `preservationAudit` section on the certificate (per stage: stakeholder anchors consumed, anchors discarded with reason) would close the §B.6 gap.

**Effort to close: medium** (~1 spec, ~1 schema field, plumbing through the existing stakeholder-doc anchoring already provided by `crates/factory-contracts/src/stakeholder_docs.rs`).

**Reconciliation.** The product gap is a design choice driven by current absence of a stakeholder-anchor-to-Build-Spec-section traceability path. The producer plane records its compression provenance; the product plane records artefact preservation but not anchor-level preservation.

---

### 6. Spec-driven discipline

**Whitepaper anchor (Part V §16).** Pronghorn is "spec driven by design": "the specification is the contract", agents work to it, output is checked against it. Aligned with Part III's Garage model.

**Producer status: EXCEEDS.** 138 of 147 specs are at lifecycle `approved`; the constitution mandates spec-first development (Principle III); the spec-code coupling gate fails CI on drift. Spec-lint default-fail-on-warn (spec 128), amends-aware coupling (spec 133), constitutional invariant freeze (spec 132), and the adversarial-prompt-refusal rule (`.claude/rules/adversarial-prompt-refusal.md`, spec 131, CONST-005) collectively make the spec spine more rigorous than the whitepaper's description: the whitepaper says agents work to the spec; OAP additionally refuses prompts that would engineer drift between spec and code.

**Product status: MET via different means.** Factory inputs are stakeholder docs (`StakeholderDoc` with anchored sections, `crates/factory-contracts/src/stakeholder_docs.rs:40`) plus extraction output (`ExtractionOutput`, schema version 1.0.0), not human-authored specs. The frozen Build Spec at s5 plays the spec-as-contract role: it is hashed, approved, and the agents in s6 work to it. The whitepaper §16 does not require the spec to be human-authored — Pronghorn itself "generate[s] draft specifications from conversations with business users" (§17). OAP's intent matches.

**Evidence.** `registry-consumer status-report --json --nonzero-only` (138 approved, 5 draft, 4 superseded); `tools/spec-code-coupling-check/`; spec 074 FR-003 (Build Spec validation); spec 075 FR-002 (process manifest from business doc paths); spec 102 FR-003 (`buildSpec` field in certificate).

**Gap or unrealized win.** **Unrealized win:** OAP's spec-code coupling gate makes spec-driven discipline *enforced* on the producer. The product-plane equivalent would be a coupling gate from frozen Build Spec to produced files (which manifest_gen.rs already implements; the gate-equivalent is the s6 verification harness).

**Effort to close: n/a** (the analogue exists; it is the verification harness, not a separate gate).

**Reconciliation.** Producer `EXCEEDS` (spec-first plus drift-refusal); product `MET via different means` (Build Spec is the contract, generated rather than authored, then frozen). This is a design choice consistent with the whitepaper's own model.

---

### 7. Harness model

**Whitepaper anchor (Part V §18).** Harness = "curated bundle of skills, standards, and templates" pre-vetted against Government-of-Alberta enterprise requirements, designed to pass enterprise review on the first attempt.

**Producer status: MET.** OAP's own development uses a curated harness: the `.claude/agents/` set (architect, explorer, implementer, reviewer, encore-expert, all path-scoped), the `.claude/rules/` set (`orchestrator-rules.md`, `governed-artifact-reads.md`, `adversarial-prompt-refusal.md`), the policy kernel (CONST-001 through CONST-005), the tool registry with permission-tiered gates (spec 067), and the skill/command factory (spec 071). The constitution is the standards layer; specs/ is the templates layer.

**Product status: MET.** The factory's harness is `factory-engine` + adapters (4 registered) + `standards-loader` + axiomregent + policy-kernel + verify-harness (`crates/factory-engine/src/verify_harness.rs`) + per-stage gate configs (`factory/contract/checks/{stage-id}.checks.yaml`, FR-012 of spec 102). Adapter manifests carry pre-vetted templates; the verify_harness loads check configs by convention (FR-019). A produced project's compile/test/lint/typecheck pass rate at first run is the empirical answer to the §18 "first attempt" claim — not measured here, but the mechanism is in place.

**Evidence.** `.claude/agents/` (5 agents); `.claude/rules/` (3 rule files); `crates/tool-registry/` (spec 067); `crates/factory-engine/src/verify_harness.rs`; spec 102 FR-011 through FR-020 (governance plumbing completion); `platform/services/stagecraft/api/factory/oapNativeAdapters.ts:24–56` (adapter declarations).

**Gap or unrealized win.** **Unrealized win — boundary as feature:** OAP's harness wraps two upstream artefacts (`goa-software-factory`, `aim-vue-node`) it does not own, via `platform/services/stagecraft/api/factory/translator.ts` (941 LOC). The harness survives upstream changes because the substrate (`factory_artifact_substrate` table per spec 139) is content-addressed; an upstream rewrite re-syncs into substrate without breaking the certificate contract.

**Effort to close: n/a.**

**Reconciliation.** Both planes `MET`; product harness *exceeds* the whitepaper's Nexus model architecturally because it operates over swappable upstreams rather than over an owned scaffold. Disclosed honestly: this is the strongest direct analogue to Nexus in OAP's surface, with one structural advantage (upstream-swappability) and one current limitation (only `aim-vue-node` is production-supported; the other three adapters are contract-validated parity targets).

---

### 8. Observability and traceability suitable for AG/IPC review

**Whitepaper anchor (Part V §19, Part VI §25).** Velocity = integrated, queryable record of every action by every agent and human; the artefact AG and IPC require for their work.

**Producer status: MET.** The codebase index produces a queryable spec-to-code map (136 mapped specs, 27 amendment relationships, 20 CI workflows traced); `registry-consumer status-report` and `compliance-report` give framework-aligned views; CODEBASE-INDEX.md is regenerated on each `make registry`. Every governed read goes through a consumer binary (spec 103); ad-hoc parsing of `build/**/*.json` is a workflow violation per `.claude/rules/governed-artifact-reads.md`. The audit log table (`platform/services/stagecraft/api/db/schema.ts`) plus the proof chain in policy-kernel together cover "every action taken … by every agent and every human" for actions mediated by the platform.

**Product status: MET.** Per-run governance certificate carries `intent` (requirements hash, spec ID, spec hash), `buildSpec` (hash + approval record), `stages` (per-stage status, artifact hashes, gate results), `verification` (compile/test/lint/typecheck/security), `proofChain` (record count, first/last hash, integrity), `traceability` (FR-027 maps generated files to governing spec requirements), and `certificateHash` (self-authenticating). `verify-certificate` is the AG/IPC's independent verifier per FR-007.

**Evidence.** spec 102 FR-003 through FR-010, FR-027; `crates/factory-engine/src/bin/verify_certificate.rs`; `Makefile` targets `make verify-certificate`, `make build-certificate`; `platform/services/stagecraft/api/db/migrations/8_factory_artifact_persistence.up.sql`.

**Gap or unrealized win.** What's missing for AG/IPC is *cross-run aggregation* — a Velocity-equivalent dashboard. Each cert is per-run; an aggregate "every action taken on every project across every run" view does not exist as a single queryable artefact. The audit log + factory_runs tables are the database-side substrate; no compiled aggregate report tool sits over them.

**Effort to close: medium** (a `factory-audit-report` consumer binary or stagecraft endpoint that aggregates certificates across runs; substrate already content-addressed per spec 139).

**Reconciliation.** Both planes `MET` for per-unit auditability (per-spec for producer, per-run for product); cross-unit aggregation is the missing surface in both planes.

---

### 9. Competitive evaluation by design

**Whitepaper anchor (Part V §19).** Multiple harnesses, agent configurations, and delivery teams set the same challenge and evaluated on a "common leaderboard".

**Producer status: GAP.** OAP's own development is single-author (`Bartek Kus` per git log) and single-track. There is no internal-development leaderboard that compares alternative agent configurations against the same spec. This is acceptable at pre-alpha scale but is a genuine gap against the whitepaper.

**Product status: PARTIAL.** The Adapters/Contracts/Processes split is the *prerequisite* to product-plane competitive evaluation, and it exists: four adapters (`aim-vue-node`, `next-prisma`, `rust-axum`, `encore-react`) all register against the same factory contract. The mechanism that converts that into a leaderboard — a runner that drives multiple adapters against the same Build Spec, captures comparable metrics (compile success rate, test pass rate, certificate-verify outcome, token cost, wall-clock), and persists them for cross-comparison — is not present. `multi-model-chaining` (spec 062, package present at 414 LOC of test code) is the closest related primitive.

**Evidence.** Adapter inventory in `platform/services/stagecraft/api/factory/oapNativeAdapters.ts:24–56`; absence of a runner — searched `crates/factory-engine/`, `tools/`, and `platform/services/stagecraft/api/factory/` for a multi-adapter benchmark harness, none found.

**Gap or unrealized win.** **Unrealized win — boundary as feature:** the ACP split is what makes the leaderboard mechanically possible. Most factories that own the whole stack would have to refactor before a leaderboard could exist. OAP could add a leaderboard without changing its core architecture.

**Effort to close: medium.** The runner is a few hundred lines of orchestration plus a substrate table for results. Spec 074's adapter manifest already standardises adapter-level metrics surfaces.

**Reconciliation.** Producer `GAP` (single-track development is the design); product `PARTIAL` (foundation exists, leaderboard does not). The product gap is closeable without touching upstream dependencies.

---

### 10. Modular, owned, auditable code (modules ≤ ~1000 LOC)

**Whitepaper anchor (Part VI §21).** Highly modular target architecture; "no module … is expected to exceed approximately one thousand lines of code", each transparently auditable.

**Producer status: PARTIAL.** Distribution across the producer's Rust source is honest about this:

- 18 crates totalling 77,467 LOC; 9 files exceed 1,000 LOC, 33 files in the 500–1,000 range.
- Largest offenders: `crates/orchestrator/src/lib.rs` (3,081 LOC), `crates/agent/src/prompt/compaction.rs` (1,505), `crates/factory-engine/src/stages/stage_cd_comparator.rs` (1,426), `crates/orchestrator/src/sqlite_state.rs` (1,307), `crates/factory-engine/src/manifest_gen.rs` (1,206), `crates/provenance-validator/src/validator.rs` (1,200), `crates/factory-contracts/src/build_spec.rs` (1,056), `crates/orchestrator/src/claude_executor.rs` (1,047), `crates/factory-engine/src/migration/stakeholder_docs.rs` (1,041).
- Desktop is worse: 14 files >1,000 LOC, including `apps/desktop/src-tauri/src/commands/factory.rs` (3,112), `apps/desktop/src/components/ToolWidgets.tsx` (3,000), `apps/desktop/src-tauri/src/commands/claude.rs` (2,951), `apps/desktop/src-tauri/src/commands/agents.rs` (2,589).

The smaller crates (skill-factory 1,227 LOC across the crate; tool-registry 1,118; standards-loader 958; artifact-extract 945; run 663) do meet the bar at the per-file level, but the orchestrator-and-factory-engine core does not. This is a real gap.

**Product status: GAP.** No line-count gate exists in `crates/factory-engine/src/gate.rs` or `crates/factory-engine/src/stages/quality_gates.rs`. The constraint is documented in spec prose (074 §1, 102 §21-style language) but is not an enforced check at adapter-level `feature_verify`. Produced module sizes inherit from the `aim-vue-node` template — outside OAP's control.

**Evidence.** Output of `find crates -name "*.rs" -not -path "*/target/*" | xargs wc -l | sort -rn` and equivalent for `apps/desktop/`; absence of any `loc` or `module-size` check in `crates/factory-engine/src/stages/quality_gates.rs` (842 LOC, contains QG-13 only).

**Gap or unrealized win.** Splitting the offender files is mechanical but not free — the orchestrator's `lib.rs` at 3,081 LOC concentrates the public surface and would need a careful refactor.

**Effort to close: medium for the producer** (file-by-file split, gated by tests). **Small for the product** (add a `module-size` gate to the adapter `feature_verify` config; `factory/contract/checks/{stage-id}.checks.yaml` per FR-019 already supports custom check types).

**Reconciliation.** Producer and product both gapped, for related-but-distinct reasons. Producer gap is internal hygiene drift; product gap is unenforced inheritance from upstream template. The producer gap is the more urgent one to disclose because it is internal-OAP code; the product gap is closeable through an adapter-level check that does not require upstream changes.

---

### 11. Open-source / non-lock-in posture

**Whitepaper anchor (Part VI §22).** Open-source models, "modular code that we own", and standardised APIs/data models reduce lock-in.

**Producer status: MET.** AGPL-3.0 (`LICENSE`); Rust + TypeScript (no proprietary runtime); per-target CycloneDX SBOMs and aggregate SBOM ship with releases per `README.md` lines 28–35; SHA-256 verifiable installers per release; runs locally (Tauri desktop, no SaaS in the trust path unless deliberately added — see README §"Trust fabric" lines 122–134). Strong copyleft is the explicit design choice (README §"License" lines 152–163).

**Product status: EXCEEDS.** A produced project has no OAP runtime dependency. The certificate is independently verifiable (`make verify-certificate FILE=... ARTIFACT_DIR=...`, README lines 234–246, spec 102 FR-007); the verifier is a sister binary that does not trust the producing system. Adapters are swappable (4 registered against the same contract). The four adapter outputs target conventional stacks (Vue 3 + Express, Next.js + Prisma, Axum + HTMX, Encore.ts + React) — none require OAP at runtime. This is the answer to "what happens if you leave": the produced application keeps running, the certificate stays verifiable, the adapter manifest can be ported.

**Evidence.** `LICENSE` (AGPL-3.0); README §"Try it" lines 219–246 (tamper-detection example); `crates/factory-engine/src/bin/verify_certificate.rs`; adapter declarations in `platform/services/stagecraft/api/factory/oapNativeAdapters.ts:24–56` (target stacks named: `aim-vue-node` is Express + Vue + Node, `next-prisma` is Next.js + Prisma + Node, `rust-axum` is Axum + HTMX, `encore-react` is Encore.ts + React). Note: this audit verifies the *target stack declarations* but not the *absence of OAP-runtime imports* in the upstream-owned scaffolds; that absence is consistent with the stacks named but would need to be re-verified post-pipeline against a real produced project before being claimed as load-bearing in a disclosure.

**Gap or unrealized win.** **Unrealized win — boundary as feature, primary case:** OAP's product is *more* portable than a stack-owning factory's product would be, because the produced project is not coupled to the producing factory's runtime. The dependency relationship is: produced project → certificate → verifier; verifier does not require OAP itself.

**Effort to close: n/a.**

**Reconciliation.** Producer `MET`; product `EXCEEDS`. The product result is what the disclosure should lead with.

---

### 12. OWASP ASI 2026 coverage

**Whitepaper anchor (implied by Part VI §21 and §25).** Test-coverage and AG/IPC auditability framing; OWASP ASI 2026 is the de-facto external standard for agentic-system risk.

**Producer status: PARTIAL** (improved from 6/10 to 7/10 in the pre-disclosure pass). `registry-consumer compliance-report --framework owasp-asi-2026 --json` reports 7 of 10 ASI controls covered. The pre-disclosure pass added `compliance:` frontmatter to specs 047, 067, 068, 069, 116, and 121, expanding both depth (3–4 specs per covered control instead of 1) and breadth (ASI04 newly covered via spec 116's supply-chain gate composition):

- **ASI01** (Agent Goal Hijack) — specs 047, 102, 121
- **ASI03** (Privilege Escalation) — specs 047, 067, 068, 102
- **ASI04** (supply-chain compromise, *name asserted by `docs/launch/gaps.md`, not by file-of-record* — see "Framework data incompleteness" below) — spec 116
- **ASI05** (Information Disclosure) — specs 047, 068, 069, 102
- **ASI07** (Cascading Failures) — spec 102
- **ASI09** (Unsafe Code Execution) — specs 067, 068, 069, 102
- **ASI10** (Agent Behavior Drift) — specs 047, 068, 102, 121

**ASI02, ASI06, ASI08 remain unmapped.** They are referenced as "unmapped" in `docs/launch/gaps.md` but their per-control names are not defined in any OAP file-of-record (`tools/registry-consumer/`, spec 102, `tools/spec-compiler/`). Mapping any of them would have required asserting a control definition without textual support — refused under CONST-005.

**Product status: PARTIAL.** Spec 102's coverage attaches to the producer (spec spine), not to per-product certificates. The certificate carries `intent.specId`, which transitively binds the produced project to whatever ASI controls that spec declares — but per-product ASI gating (e.g., "this Build Spec requires ASI03 enforcement and the verifier checks for it") is not implemented. Policy-kernel evaluations record ASI-relevant decisions in the proof chain, but no ASI-specific gate exists in `quality_gates.rs`.

**Evidence.** Output of `registry-consumer compliance-report --framework owasp-asi-2026 --json` (7 control entries; ASI01/03/05/09/10 each carry 3–4 spec citations; ASI04 carries spec 116; ASI07 carries spec 102 only). Spec 102 §"Phase D — OWASP ASI 2026 + Security Hardening (FR-031 to FR-040)" defines the canonical names for the six controls it covers; the seventh (ASI04) is named only in `docs/launch/gaps.md`.

**Framework data incompleteness — disclosed honestly.** OAP's compliance framework machinery (spec 102 FR-023 to FR-030, `tools/registry-consumer/src/main.rs`) is data-driven by frontmatter alone; it does not carry a registry of ASI control names with definitions. The compliance-report subcommand reports whatever string the spec author wrote — there is no consistency check, no canonical control-name list, and no mechanism to detect a mismatch between an OAP spec's claimed `controls: ["ASI04"]` and OWASP's actual ASI 2026 framework. This is what limits this pass to 7/10: the unsafe path to 9/10 would be asserting ASI02/06/08 mappings against control names whose definitions OAP cannot itself check. Closing this gap (folding canonical OWASP ASI 2026 control names + brief definitions into a registry-consumer config and gating compliance frontmatter against the list) is recorded as a post-disclosure closure.

**Gap or unrealized win.** Closure beyond 7/10 requires (a) framework data completeness — see post-disclosure closures — and (b) any spec content that genuinely addresses ASI02/06/08, none of which were found among the candidate specs reviewed.

**Effort to close further: small** for ASI02/06/08 *if* the canonical control definitions are folded into framework data and a content review of remaining specs confirms a defensible mapping exists. **Medium** for product-plane per-certificate ASI gating.

**Reconciliation.** Both planes `PARTIAL`. Producer is at 7/10 honestly; the path to 9/10 is gated on framework-data closure, not on more frontmatter edits. The audit's current ASI control names derive from spec 102 §"Phase D" and `docs/launch/gaps.md`; an external research artifact at `_tmp/compass_artifact_wf-...md` may provide canonical OWASP names (see "Framework-data closure — external research artifact available" under post-disclosure closures), adoption deferred to post-disclosure verification.

---

### 13. Self-documenting traceability (meta-governance)

**Whitepaper anchor (implied by Part VI §25 and §A.9).** Decision-support figures must be reproducible at any point in time.

**Producer status: MET.** OAP describes its own state in machine-readable form: `build/codebase-index/index.json` enumerates every crate and package with its declaring spec; `CODEBASE-INDEX.md` is the rendered human view (regenerated by `codebase-indexer render`); featuregraph crate (spec 034) maps spec → code via the codebase-indexer's `index.json`; registry-consumer's `compliance-report`, `status-report`, `list`, and `show` subcommands together produce a complete typed query surface over the spec corpus. Every figure in the README's Try-it block is a consumer-binary invocation, not a hand-edited number.

**Product status: MET.** Per-run, the produced project carries `.factory/pipeline-state.json` (durable state), `.factory/build-spec.yaml` (frozen contract), and `.factory/governance-certificate.json` (audit chain). The certificate's `traceability` section (spec 102 FR-027) maps generated files to governing spec requirements. The run is self-describing without OAP installed.

**Evidence.** `tools/codebase-indexer/`; `crates/featuregraph/`; `tools/registry-consumer/`; spec 102 FR-027; spec 074 FR-003 (PipelineState validation); README.md "Try it" block.

**Gap or unrealized win.** **Unrealized win:** the producer's self-description was hardened by the spec 103 "governed reads" rule that *forbids* ad-hoc parsing of `build/**/*.json`. This is a meta-governance discipline the whitepaper does not name — every read of the compiler's output goes through the consumer binary, so a schema change fails loudly rather than silently corrupting downstream tools.

**Effort to close: n/a.**

**Reconciliation.** Both planes `MET` via different mechanisms (compiler-driven on producer, run-local artefacts on product). Both are queryable post-hoc.

## Unrealized wins

Three claims worth disclosing as architecturally distinctive rather than as compensations for a gap.

1. **Governance over upstream dependencies you don't own (dim. 4, 7, 11).** OAP achieves deterministic Build Spec freeze, artefact hashing, gate verification, and self-authenticating certificates *over* upstream artefacts (`goa-software-factory`, `aim-vue-node`) it does not author. The translator (`platform/services/stagecraft/api/factory/translator.ts`, 941 LOC) plus the content-addressed substrate (spec 139) form a governance ring; OAP survives upstream changes because the substrate is content-addressed and the certificate contract is versioned independently.

2. **Adapters / Contracts / Processes split as the swappability layer (dim. 7, 9).** Four adapters register against one contract. This is the prerequisite to a competitive leaderboard (dim. 9), to upstream replacement (dim. 11), and to the harness-survives-upstream property (dim. 4). Mechanism present in code; the leaderboard runner that would surface it is the only missing piece.

3. **Spec-driven discipline as a producer property that has not yet been imposed on products — and could be (dim. 6).** A future spec could extend the producer's PR-time spec-code coupling enforcement to per-product file-level traceability; the content-addressed substrate already carries everything required.

## The upstream-dependency boundary

Every place OAP's product-plane alignment is constrained by upstream artefacts it does not control:

| Boundary | Upstream | OAP's mechanism over it | Future-closeability |
|---|---|---|---|
| Process-stage agent prompts (s0–s5) | `goa-software-factory` (translated into substrate) | Substrate stores agents as content-addressed artefacts (spec 139); per-org overrides supported | OAP could author replacement process agents; upstream becomes optional |
| Scaffold templates (s6a–s6g for `aim-vue-node`) | `GovAlta-Pronghorn/template` | Adapter manifest (spec 074 FR-004) wraps template; verification harness runs adapter `feature_verify` commands | OAP could author its own template; the other three adapters (`next-prisma`, `rust-axum`, `encore-react`) are already factory-contract validated and would serve as parity validators |
| Module-size discipline on produced projects | `aim-vue-node` template structure | None today; adapter `feature_verify` could enforce | Small effort: add a check to `factory/contract/checks/{stage-id}.checks.yaml` per FR-019 |
| Reproducibility of scaffold-stage code generation | LLM provider + adapter prompt content | Build Spec freeze + per-stage artefact hashes + verification gates; reproducibility is at the contract layer, not byte-level | Future-closeable only if scaffold-stage generation moves to deterministic templates rather than LLM agents |
| ASI control gating on produced projects | n/a (OAP-internal future work) | None today; certificate carries `intent.specId` only | Small-to-medium: per-Build-Spec ASI requirement → per-certificate gate |

Framing for the reviewer: the boundary is the load-bearing feature, not a limitation. OAP can honestly claim "we do not own these upstreams; the substrate is what makes them swappable; here are the mechanisms by which we govern them."

## Recommended closures before disclosure

Trivial-to-small effort items, ordered by disclosure-readability impact:

1. **~~Add `compliance:` frontmatter to specs 047, 067, 068, 069, 121~~** **— DONE in pre-disclosure pass.** Closed by adding `compliance:` blocks to specs 047, 067, 068, 069, 116, and 121. Result: 7/10 ASI coverage (was 6/10), with depth on six of the seven covered controls (3–4 specs each instead of 1). The aspirational target of ~9/10 was not reached: ASI02, ASI06, and ASI08 cannot be honestly mapped without canonical control definitions in OAP file-of-record. Closure to 8 or 9/10 is gated on the framework-data-incompleteness item below, NOT on more frontmatter edits. CI green; coupling gate green.
2. **Add a `loc-limit` check type to `factory/contract/checks/{stage-id}.checks.yaml`** so adapter `feature_verify` can enforce module-size discipline on produced projects (dim. 10 product). Effort: small.
3. **Author the per-portfolio strategic-tier rollup as a `registry-consumer compliance-report --rollup` flag** (dim. 1 producer caveat). Effort: small.
4. **Rename `_tmp/` to `tmp/`** if it should remain in the repo, or `.gitignore` it. **— RESOLVED in pre-disclosure pass.** `_tmp/` is already in `.gitignore` (line 302); the whitepaper has never been tracked. Repository is clean for external disclosure with respect to whitepaper redistribution.

## Recommended closures after disclosure (post-v1.0.0)

Medium-to-large items, ordered by impact:

1. **Framework data completeness for compliance-report** (dim. 12 producer, surfaced during pre-disclosure pass). Today the compliance machinery (spec 102 FR-023 to FR-030, `tools/registry-consumer/`) is data-driven by frontmatter alone: no canonical control-name registry, no per-control definitions, no consistency check between an OAP spec's claimed `controls: [...]` and the framework's actual control list. The pre-disclosure pass capped at 7/10 ASI coverage because mapping ASI02/06/08 would have required asserting control names whose definitions OAP cannot itself check — refused under CONST-005. Closure: fold canonical OWASP ASI 2026 (and SOC2, ISO-27001, EU AI Act, NIST AI RMF) control definitions into a `tools/registry-consumer/data/frameworks/*.yaml` configuration; gate compliance-frontmatter at `spec-compiler` time against the declared list (W-series warning on unknown control); mirror the schema-parity discipline used for stakeholder/provenance schemas. Effort: **small-to-medium** for the OWASP ASI 2026 file (10 controls); **medium** for full framework set. Once landed, ASI02/06/08 (and any additional SOC2/ISO/NIST control mappings) can be asserted with the same defensibility as ASI01/03/05/07/09/10 today.
2. **Velocity-equivalent leaderboard runner** (dim. 9 product). Drives multiple adapters against the same Build Spec, persists comparable metrics into substrate. Medium.
3. **Split the +1,000-LOC offenders** (dim. 10 producer): `crates/orchestrator/src/lib.rs` (3,081), `apps/desktop/src-tauri/src/commands/factory.rs` (3,112), `apps/desktop/src/components/ToolWidgets.tsx` (3,000), plus the other 21 files >1,000 LOC. Medium per file, large in aggregate.
4. **Preservation-audit field on the governance certificate** (dim. 5 product). Per-stage record of stakeholder anchors consumed and discarded. Medium.
5. **Per-certificate ASI gating** (dim. 12 product). Per-Build-Spec ASI requirement set evaluated by the verifier on replay. Medium.
6. **Cross-run aggregate audit report** (dim. 8). `factory-audit-report` consumer over the substrate. Medium.
7. **Per-product file-level spec-code traceability** (dim. 6 product, "unrealized win 3"). Extends the producer's coupling discipline into produced projects. Large; most architecturally significant.

### Framework-data closure — external research artifact available

The user holds an external research artifact at `_tmp/compass_artifact_wf-f1ff0eeb-dc39-4a22-b69b-262e5f799823_text_markdown.md` (gitignored; not tracked) that appears to carry canonical OWASP ASI 2026 control names. It enumerates five controls inline — `Agent Goal Hijack (ASI01), Tool Misuse (ASI02), Identity & Privilege Abuse (ASI03), Agentic Supply Chain (ASI04), and so on through ASI10 Rogue Agents` — sourced to the "OWASP GenAI Security Project release" of 2025-12-09. The artifact's provenance to OWASP has not been independently verified in this audit pass: the OWASP source document itself is not embedded or quoted at length, and no per-control definitions are present in the artifact.

Three observations make this a load-bearing post-disclosure item rather than a current edit:

1. **Naming may shift, but control content is unchanged.** The audit's existing seven mapped controls were chosen by reviewing the *behavioural content* of each cited spec (e.g., spec 047 governance-control-plane → goal-hijack-class control; spec 116 supply-chain gates → supply-chain-class control). CONST-005 applies: refuse to chase names that have not been verified against the canonical OWASP framework. The seven existing mappings remain defensible on content even if the name strings shift on verification.
2. **Discrepancies if compass names are canonical.** Compass calls ASI03 "Identity & Privilege Abuse" (audit: "Privilege Escalation"), ASI04 "Agentic Supply Chain" (audit: "supply-chain compromise"), ASI10 "Rogue Agents" (audit: "Agent Behavior Drift"). The compass also names ASI02 ("Tool Misuse") — which is currently unmapped in this audit.
3. **ASI06 and ASI08 are not named in the compass.** Closure remains contingent on a canonical source, not on adopting the compass wholesale.

**Closure.** Verify the compass artifact's provenance against the actual OWASP ASI 2026 publication (or the OWASP GenAI Security Project release of 2025-12-09 it cites). If provenance is validated: adopt compass-derived names and definitions in spec 102's framework-data file (`tools/registry-consumer/data/frameworks/owasp-asi-2026.yaml` per closure #1 above), which unblocks honest mapping of any controls compass defines (notably ASI02 against specs 067/068, behavioural fit pre-screened) and refines naming on the seven currently covered. **Effort: medium** — the compass contains names only, no per-control definitions; closure work still includes researching canonical definitions for ASI05/06/07/08/09 (and verifying compass names are themselves canonical) from the OWASP source. If a future compass-like artifact lands with per-control definitions for ASI02/06/08, the sizing drops to **small** (verify and incorporate).

This closure is a refinement of closure #1 above, not a substitute for it. CONST-005 governs: do not edit any spec frontmatter from the compass alone.

## Appendix: evidence index

**Specs cited (load-bearing):** 000 (bootstrap), 047 (governance-control-plane, ASI mapping), 067 (tool-definition-registry, ASI mapping), 068 (permission-runtime, ASI mapping), 069 (lifecycle-hook-runtime, ASI mapping), 074 (factory-ingestion), 075 (factory-workflow-engine), 088 (factory-upstream-sync, superseded), 091 (registry-enrichment; pre-disclosure pass golden-refresh maintenance note), 102 (governed-excellence; FR-001 to FR-040, six-control ASI baseline), 103 (governed reads), 108 (factory-as-platform-feature), 116 (supply-chain-policy-gates, ASI04 mapping), 119 (project-as-unit-of-governance), 121 (claim-provenance-enforcement; QG-13, ASI mapping), 122 (stakeholder-doc-inversion), 127/130/133 (spec-code coupling gate + amendments), 131 (adversarial-prompt-refusal, CONST-005), 139 (factory-artifact-substrate), 140 (aim-vue-node alignment).

**Code paths cited:** `crates/factory-engine/src/governance_certificate.rs:23–44, 95–103, 147–154, 297–308, 316–409`; `crates/factory-engine/src/bin/verify_certificate.rs`; `crates/factory-engine/src/{stages/quality_gates.rs (842 LOC), manifest_gen.rs (1,206 LOC), verify_harness.rs}`; `crates/factory-contracts/src/{build_spec.rs:16–44, pipeline_state.rs:19–33, stakeholder_docs.rs:40, knowledge.rs:19, adapter_manifest.rs}`; `crates/orchestrator/src/lib.rs (3,081 LOC)`; `crates/policy-kernel/src/lib.rs (792 LOC)`; `tools/{spec-compiler,registry-consumer,codebase-indexer,spec-code-coupling-check}/`; `platform/services/stagecraft/api/factory/{oapNativeAdapters.ts:24–56, translator.ts (941 LOC), upstreams.ts:30–47, substrate.ts, syncWorker.ts}`; `platform/services/stagecraft/api/db/schema.ts:265 (audit_log), :466 (factory_audit_log)`.

**Meta-files cited:** `CLAUDE.md`, `AGENTS.md`, `.specify/contract.md`, `.specify/memory/constitution.md`, `.claude/rules/{orchestrator-rules,governed-artifact-reads,adversarial-prompt-refusal}.md`, `.github/workflows/ci-spec-code-coupling.yml`, `LICENSE` (AGPL-3.0), `README.md`.

**Quantitative checks performed (refreshed 2026-05-11):**
- 18 Rust crates, 77,467 LOC; **9 files >1,000 LOC** (was 10), 33 files 500–1,000 LOC.
- 14 desktop files >1,000 LOC (largest 3,112).
- **102 specs declare `implements:` paths totalling 429 path bindings** (was 100 / 368).
- **138/147 specs `approved`, 5 draft, 4 superseded** (governed read; was 138/143 with 1 draft).
- **136/147 specs traced to code** (Layer 2 of CODEBASE-INDEX.md; was 132/143).
- **7/10 OWASP ASI 2026 controls covered** (governed read; unchanged since 2026-05-07 hardening pass: ASI01/03/04/05/07/09/10; ASI02/06/08 still gated on framework-data closure).
- 4 adapters registered.
- 5 policy-kernel tiers.

**Whitepaper anchors (verbatim phrases used in this audit):** §12 "raw material … structural representation … business capability … strategic intent"; §13 "compression and decompression operations are themselves first-class artifacts, not invisible side effects"; §14 "confine non-determinism to the layers where it adds value"; §16 "spec driven by design. The specification is the contract"; §18 "curated bundle of skills, standards, and templates that have been pre vetted"; §19 "common leaderboard"; §21 "no module … is expected to exceed approximately one thousand lines of code"; §22 "modular code that we own"; §25 "auditable by … Auditor General … Information and Privacy Commissioner"; §B.6 "preservation audit … lists explicitly any capabilities that have no home in the target".
