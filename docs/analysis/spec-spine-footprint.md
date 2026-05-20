# Spec-Spine Extraction — Footprint Analysis & Seam Mapping

> Read-only inventory and dependency map produced 2026-05-17 against
> `main` @ `b1f1ba0b`. No code was moved or modified. Tools used to
> gather evidence: `ls`, `find`, `wc -l`, `grep -rn`, direct `Read` of
> Cargo.toml manifests and `*.rs` source files. Where claims rest on
> not-yet-verified detail, the text marks it as *inference*.

This document is structured by the phases of the analysis brief. Phase 1
and Phase 2 are complete; the remainder (Phases 3–8) are stubbed and
flagged so the user can redirect framing before they're filled in, per
the brief's "stop and check in after Phase 2" instruction.

---

## Executive summary

1. There are **six clean tool-crate candidates** in `tools/` plus **one
   crate** in `crates/featuregraph` that plausibly form Spec-Spine.
   Combined ≈11.3 kLoC of Rust source.
2. Inside that candidate set, dependency direction is **already
   one-directional**: `frontmatter → {spec-compiler, spec-lint,
   codebase-indexer, policy-compiler}`, and `codebase-indexer →
   spec-code-coupling-check`. No spec-spine candidate depends on
   `crates/*` OAP code.
3. **`registry-consumer` is the only candidate with zero internal-OAP
   dependencies in either direction** — it imports nothing from the
   shared frontmatter crate, parses `registry.json` via `serde_json::Value`
   directly, and is also the only candidate whose API is consumed only
   by its own binary. It's the "cleanest leaf" by every metric.
4. **`spec-compiler` carries OAP-specific scope inside its core lib.**
   `factoryProjects` (spec 074), `compliance.framework` (spec 102), and
   `adapter` parsing live in `tools/spec-compiler/src/lib.rs` lines
   796–1000, 1382–1416. These are not optional plugins; they execute on
   every `compile()` call. Extracting spec-compiler cleanly means deciding
   what to do with these.
5. **`codebase-indexer` is even more OAP-shaped.** `factory.rs` scans
   `factory/adapters/`, `infra.rs` scans `.claude/{agents,commands,rules}`,
   `workflows.rs` scans `.github/workflows/` for `# Spec:` headers.
   Roughly 670 of its 1838 src LoC scan OAP-specific directories.
6. **`featuregraph` parses `registry.json` directly** via its own
   `serde::Deserialize` struct (`CompiledRegistry` at
   `crates/featuregraph/src/registry_source.rs:14-46`), bypassing the
   `registry-consumer` "governed read" contract. This is a coupling
   *upstream* of Spec-Spine, not inside it.
7. **The only inbound import edges from OAP into spec-spine candidates
   are `crates/axiomregent` and `apps/desktop/src-tauri` → `featuregraph`.**
   No OAP crate imports any `tools/` library. `spec-code-coupling-check`
   imports `codebase-indexer::types`, but that's intra-candidate.
8. **The G-2 governance certificate pipeline does not import any
   spec-spine library.** `crates/factory-engine/src/governance_certificate.rs`
   has zero references to `registry.json`, `codebase-index`, or any
   `open_agentic_*` tool crate. It only stamps an `Option<String>`
   `spec_id` field. (Detail in Phase 8.)
9. The `release-tools.yml` workflow ships **five binaries** as Spec-Spine
   release artifacts: `spec-compiler`, `registry-consumer`, `spec-lint`,
   `policy-compiler`, `codebase-indexer`. `policy-compiler` is bundled
   in releases but is OAP-policy-specific (depends on
   `crates/policy-kernel`).
10. The biggest "format-as-contract" gap: **109/151 specs declare
    `implements:`** (≈72%); the validation rules around `kind`, `shape`,
    `category`, `implements: list-vs-scalar` are documented mostly inside
    spec-compiler's `lib.rs` constants and comments, not in a
    standalone grammar document.

**Confidence per phase**

| Phase | Confidence | What would raise it |
|-------|------------|---------------------|
| 1. Inventory | High | — |
| 2. Dependency direction | High | A `cargo tree`-based cross-check could surface transitive edges I missed. |
| 3. Public surface | *Not started.* | — |
| 4. Coupling classification | *Not started.* | — |
| 5. Test/fixture footprint | *Not started.* | — |
| 6. Build/CI footprint | Medium (partial evidence collected) | A pass over every `.github/workflows/*.yml` and per-target SBOM emission. |
| 7. Format-as-contract | *Not started.* | — |
| 8. G-2 question | Medium | Negative findings (no imports) are strong; the *positive* claim that the cert pipeline benefits/doesn't benefit from extraction needs the FR-002 / FR-010 schema and SSE work-streams to be re-read in light of Phase 3. |

---

## Phase 1 — Inventory

All Rust LoC counts from `wc -l` on `src/**/*.rs` (excluding tests and
target dirs).

### Core (format, parser, schema-defining)

| Path | Crate name | One-line purpose | src LoC |
|------|------------|------------------|---------|
| `tools/shared/frontmatter/` | `open_agentic_frontmatter` | Strips `---\nYAML\n---\n` from a markdown string. 40 lines, no Spec-Spine semantics — just the file-format boundary. | 40 |
| `tools/spec-compiler/` | `open_agentic_spec_compiler` | Compiles `specs/*/spec.md` → `build/spec-registry/registry.json` + `build-meta.json`. Defines KNOWN_KEYS, VALID_KINDS, VALID_RISK_LEVELS, SHAPE_TABLE, and all V-001..V-019 violation codes. | 1813 lib + 46 main |
| `schemas/codebase-index.schema.json` | (artifact) | JSON Schema 1.4.0 for the indexer's output. Owned by codebase-indexer (mirrors its `types::SCHEMA_VERSION`). | — |
| `schemas/agent-frontmatter.schema.json`, `schemas/skill-frontmatter.schema.json` | (artifact) | Not loaded by any tool I traced; consumed elsewhere in OAP. Listed here because spec-spine docs mention `schemas/`. | — |

### Index (codebase / spec-to-code traceability)

| Path | Crate name | One-line purpose | src LoC |
|------|------------|------------------|---------|
| `tools/codebase-indexer/` | `open_agentic_codebase_indexer` | Emits `build/codebase-index/index.json` (5 layers: inventory, traceability, factory, infrastructure, workflow-traceability). Carries the `// Spec: …` comment-header scanner and the spec/code coupling source-of-truth. | 1838 (12 files) |

### Validate (lint, coupling check)

| Path | Crate name | One-line purpose | src LoC |
|------|------------|------------------|---------|
| `tools/spec-lint/` | `open_agentic_spec_lint` | W-xxx warnings on the spec corpus. Strict mode (`--fail-on-warn`) gated by spec 128. | 358 lib + 55 main |
| `tools/spec-code-coupling-check/` | `open_agentic_spec_code_coupling_check` | PR-time gate: fail when an `implements:`-claimed path changes without the owning spec. Reads index via `codebase-indexer::types`. | 719 lib + 142 main |

### CLI surfaces (operate on the compiled artifacts)

| Path | Crate name | One-line purpose | src LoC |
|------|------------|------------------|---------|
| `tools/registry-consumer/` | `open_agentic_registry_consumer` | Read-only CLI over `registry.json`. `list`, `show`, `status-report`, `compliance-report`. Pure-`serde_json::Value` API; no domain types. | 304 lib + 317 main |

### Filesystem / corpus (specs/ live here)

| Path | Purpose | Count |
|------|---------|-------|
| `specs/000-bootstrap-spec-system/` through `specs/150-…/` | 151 spec directories, each `spec.md` + optional `plan.md`, `tasks.md`, `contracts/`, `research.md`, `quickstart.md`. 109/151 declare `implements:`. | 151 dirs |

### Fixtures

| Path | Purpose | File count |
|------|---------|-----------|
| `tools/registry-consumer/tests/fixtures/` | Golden registries + expected stdout/stderr fixtures for all contract subsets (readme_, error_, shape_, help_, arg_, version_, default_path_, allow_invalid_, sorting_, channel_). | 41 files |
| `tools/ci-parity-check/tests/fixtures/` | CI parity walker fixtures. Not Spec-Spine. | a few |
| `tools/codebase-indexer/tests/` | Tests in-place: `golden.rs`, `exit_codes.rs`, `schema_conformance.rs`. No external fixture dir — golden expectations are inline strings. | 3 |
| `tools/spec-compiler/tests/` | 9 integration test files; corpus fixtures via `tempfile`. No persistent fixture dir. | 9 |

### Docs (spec format as contract)

| Path | Purpose |
|------|---------|
| `.specify/memory/constitution.md` | Constitution v1.0.2. Names Markdown + JSON layer split, names Feature 000 as the bootstrap. |
| `.specify/contract.md` | One-page summary of Feature 000 contract. |
| `.specify/templates/spec-template.md` | Template humans copy when authoring specs. |
| `docs/registry-consumer-contract-governance.md` | The contract-governance process for `registry-consumer`. |
| `specs/000-bootstrap-spec-system/spec.md` | Authoritative grammar source (referenced by both constitution and contract.md). Listed here, not summarised, per the brief. |

### Unclear (tools that *touch* spec-spine artifacts but may or may not belong)

| Path | Crate name | One-line purpose | src LoC | Why unclear |
|------|------------|------------------|---------|-------------|
| `tools/policy-compiler/` | `open_agentic_policy_compiler` | Compiles `policy:` frontmatter blocks (in CLAUDE.md and elsewhere) into policy artifacts. Uses `open_agentic_frontmatter`. | 701 lib + 71 main | Depends on `crates/policy-kernel` (OAP). Ships as a Spec-Spine release binary (release-tools.yml). |
| `crates/featuregraph/` | `featuregraph` | Reads `registry.json` via its own `serde` types; scans the codebase for feature aliases; computes blast radius / preflight info. | 2870 (9 files) | Parses `registry.json` directly (bypasses `registry-consumer`). Imported by `axiomregent` and the desktop app. Spec id `034-featuregraph-registry-scanner-fix` ties it to spec spine, but it's a *consumer*, not a *producer*. |
| `tools/stakeholder-doc-lint/` | `open_agentic_stakeholder_doc_lint` | Stakeholder-doc grammar lint, W-122-xxx codes. Depends on `crates/factory-contracts` + `crates/provenance-validator`. | 911 lib + 68 main | OAP-specific output but does *resemble* the spec-lint pattern. Almost certainly not Spec-Spine. |
| `tools/assumption-cascade-check/` | `open_agentic_assumption_cascade_check` | Spec 121 FR-034 cascade check. Depends on `crates/factory-engine`. | 184 lib + 126 main | Pure OAP; listed only because it's a `tools/` lint that might be confused for spec spine. |
| `tools/adapter-scopes-compiler/` | `open_agentic_adapter_scopes_compiler` | Compiles `factory/adapters/*/manifest.yaml` → `adapter-scopes.json`. | 263 lib + 86 main | Spec 105. Pure factory adapter concern. Not Spec-Spine. |
| `tools/ci-parity-check/` | `open_agentic_ci_parity_check` | Asserts Makefile↔`.github/workflows/` parity (spec 104). | 601 lib + 88 main | Not Spec-Spine. Listed because it sits in `tools/`. |

### Build-time codegen / macros tied to specs

None found. No proc-macros, no `build.rs` files that codegen against the
spec corpus. Everything is runtime parsing.

---

## Phase 2 — Dependency direction

### Internal-OAP dependency map (Cargo `path = …` edges + `use` imports)

Edges sourced from each candidate's `Cargo.toml` and the `grep` over
`use open_agentic_*` / `use featuregraph` shown earlier.

```mermaid
flowchart LR
    %% Spec-Spine candidates (boxed)
    subgraph CORE[Spec-Spine candidates]
        FM[open_agentic_frontmatter<br/>40 LoC]
        SC[spec-compiler<br/>1.8k LoC]
        SL[spec-lint<br/>0.4k LoC]
        CI[codebase-indexer<br/>1.8k LoC]
        SCC[spec-code-coupling-check<br/>0.7k LoC]
        RC[registry-consumer<br/>0.3k LoC]
    end

    subgraph UNCLEAR[Unclear / borderline]
        PC[policy-compiler]
        FG[featuregraph]
    end

    subgraph OAP[OAP-proper]
        PK[policy-kernel]
        AX[axiomregent]
        APP[apps/desktop/src-tauri]
        XR[xray]
        FCG[factory-contracts]
        FE[factory-engine]
    end

    FM --> SC
    FM --> SL
    FM --> CI
    FM --> PC

    CI --> SCC

    PC --> PK

    FG -.reads registry.json.-> RC_OUTPUT[(build/spec-registry/<br/>registry.json)]
    SC --> RC_OUTPUT
    CI --> CI_OUTPUT[(build/codebase-index/<br/>index.json)]
    SCC --> CI_OUTPUT

    AX --> FG
    APP --> FG
    FG --> XR
```

### Per-edge inventory

**Edge legend:** `lift` = belongs in spec-spine; `keep` = legitimate
intra-candidate edge; `leak` = extraction blocker pointing into OAP;
`bypass` = an OAP consumer reaches past a candidate's public CLI to read
its raw artifact.

| # | Source (file:line) | Target | Used for | Class |
|---|---|---|---|---|
| 1 | `tools/spec-compiler/src/lib.rs:3` | `open_agentic_frontmatter::{FrontmatterError, split_frontmatter_required}` | Splitting `---\nYAML\n---\n` from spec body | keep |
| 2 | `tools/spec-lint/src/lib.rs:3` | `open_agentic_frontmatter::split_frontmatter_optional` | Same as above, optional shape | keep |
| 3 | `tools/codebase-indexer/src/infra.rs:4` | `open_agentic_frontmatter::split_frontmatter_optional` | Parsing agent / command / rule frontmatter | keep |
| 4 | `tools/codebase-indexer/src/spec_scanner.rs:3` | `open_agentic_frontmatter::split_frontmatter_required` | Parsing spec frontmatter for `implements:` | keep |
| 5 | `tools/policy-compiler/src/lib.rs:2` | `open_agentic_frontmatter::split_frontmatter_optional` | Parsing `policy:` blocks | keep (within unclear bucket) |
| 6 | `tools/spec-code-coupling-check/src/lib.rs:9` | `open_agentic_codebase_indexer::types::{CodebaseIndex, SCHEMA_VERSION}` | Typed read of `build/codebase-index/index.json` | keep |
| 7 | `tools/spec-code-coupling-check/Cargo.toml:19` | `open_agentic_codebase_indexer = { path = "../codebase-indexer" }` | Library dependency for #6 | keep |
| 8 | `tools/policy-compiler/Cargo.toml:22` | `open_agentic_policy_kernel = { path = "../../crates/policy-kernel" }` | Imports `Policy`, proof-chain types | **leak** (policy-compiler→OAP) |
| 9 | `crates/featuregraph/src/registry_source.rs:14-46` | local-redeclared `CompiledRegistry`, `RegistryFeatureRecord`, `ImplementsField` | Re-parses `registry.json` outside `registry-consumer` | **bypass** |
| 10 | `apps/desktop/src-tauri/src/commands/analysis.rs:28` | `repo_root.join("build/spec-registry/registry.json")` and `featuregraph::scanner::Scanner::scan` | Desktop analysis panel | leak (desktop → featuregraph + raw artifact) |
| 11 | `apps/desktop/src-tauri/Cargo.toml:88` | `featuregraph = { path = "../../../crates/featuregraph" }` | Desktop imports featuregraph | leak |
| 12 | `crates/axiomregent/src/feature_tools.rs:8-11` | `featuregraph::{graph, locate, preflight, scanner}` | MCP agent uses featuregraph for feature-graph tools | leak |
| 13 | `crates/axiomregent/src/lib.rs:18` | `pub use featuregraph;` | Featuregraph re-exported from axiomregent's public surface | leak (and **widens** featuregraph's public surface) |
| 14 | `crates/axiomregent/src/router/legacy_provider.rs:12` | `featuregraph::tools::FeatureGraphTools` | Provider routing | leak |
| 15 | `crates/featuregraph/Cargo.toml:20` | `xray = { path = "../xray" }` | featuregraph uses xray's complexity scoring | **leak** (featuregraph → OAP xray) |
| 16 | `tools/spec-compiler/src/lib.rs:796-1000` | `factoryProjects` / `compliance` / `adapter` emission **internal** to the compiler core | Bakes OAP concepts into the spec-spine producer | leak (within-crate, no import — see "Hidden leaks" below) |
| 17 | `tools/codebase-indexer/src/factory.rs` (entire 166 LoC) | Scans `factory/adapters/*/manifest.yaml` | Layer 3 of the index | leak (within-crate) |
| 18 | `tools/codebase-indexer/src/infra.rs:104-112` | Scans `.claude/{agents,commands,rules}` | Layer 4 of the index | leak (within-crate, OAP-shaped paths) |
| 19 | `tools/codebase-indexer/src/workflows.rs` (entire 243 LoC) | Scans `.github/workflows/*.yml` for `# Spec:` headers | Layer 5 of the index | leak (within-crate, OAP-shaped artifacts) |
| 20 | `tools/spec-compiler/tests/factory_projects.rs` (213 LoC) | Whole test file uses `.factory/build-spec.yaml` fixtures | OAP-shaped test corpus | leak (in test code, not lib) |

### Hidden leaks (no `use` statement, but the *purpose* of the code is OAP-specific)

These don't show as cross-crate `use` edges because they are *inside*
spec-spine candidates. They are nevertheless extraction blockers — the
"abstract" spec-spine crate today contains concrete OAP semantics.

| Site | What's OAP-specific | Why it can't just "move with spec-spine" |
|------|---------------------|------------------------------------------|
| `spec-compiler/src/lib.rs:796-803, 999-1000, 1212-1340, 1418-1431` | `factoryProjects` array emission, `.factory/build-spec.yaml` discovery, `parse_factory_project`, `is_valid_code_alias` shape aligned to featuregraph | Bakes spec 074 (Factory Ingestion) into the compiler. If Spec-Spine is to be a generic spec compiler, this whole code path is the wrong layer. |
| `spec-compiler/src/lib.rs:1382-1416` | `parse_compliance` reads `compliance: [{framework, controls}]` blocks | OAP-specific frontmatter (spec 102 FR-023). The generic compiler would not know about "owasp-asi-2026". |
| `spec-compiler/src/lib.rs:46-83` (KNOWN_KEYS) | `compliance`, `feature_branch`, `code_aliases`, `implementation` | Some of these (e.g. `feature_branch`) are OAP-workflow assumptions, not generic spec-format keys. |
| `codebase-indexer/src/factory.rs` | Whole file scans `factory/adapters/` and `factory/process/stages/` | These directories are an OAP concept. |
| `codebase-indexer/src/infra.rs:104-115` | Hardcoded paths `.claude/agents`, `.claude/commands`, `.claude/rules` | `.claude/` is Claude Code agentic infrastructure, not generic. |
| `codebase-indexer/src/workflows.rs` | Spec 118 `# Spec:` header convention in `.github/workflows/*.yml` | OAP-specific. |
| `spec-lint/src/lib.rs:23-63` | `CONVENTIONAL_CATEGORIES`, `SHAPE_TABLE` (mirrored across spec-compiler + spec-lint) | The vocabularies are OAP-shaped, and the two crates carry duplicate copies. |
| `spec-code-coupling-check/src/lib.rs:23-34` (`BYPASS_PREFIXES`) | Path list refers to OAP-specific directories (`crates/`, `tools/`, `apps/`, `packages/`, `platform/services/`) | The bypass list itself encodes OAP layout. |

### Leaf / hub classification

| Module | Inbound (callers) | Outbound (deps) | Class |
|--------|-------------------|-----------------|-------|
| `open_agentic_frontmatter` | spec-compiler, spec-lint, codebase-indexer, policy-compiler | `serde_yaml` only | **Leaf-producer.** Pure utility. Trivially extracts. |
| `registry-consumer` | None (binary-only; no lib consumers) | `serde_json`, `clap` | **Leaf-consumer.** Standalone CLI; doesn't link any spec-spine library. |
| `spec-compiler` | None (binary-only) | frontmatter | **Producer hub** for the compiled artifact; no library consumers. |
| `spec-lint` | None (binary-only) | frontmatter | **Independent lint.** |
| `codebase-indexer` | spec-code-coupling-check (lib consumer) | frontmatter | **Hub.** The only candidate with a library consumer inside the candidate set; carries the largest OAP-shaped surface area. |
| `spec-code-coupling-check` | None (binary-only) | codebase-indexer::types | **Tip.** |
| `policy-compiler` | None | frontmatter + **policy-kernel (OAP)** | **Leak.** Bundled in releases but logically OAP. |
| `featuregraph` | axiomregent + desktop | xray (OAP) | **Bidirectional entanglement.** Reads spec-spine artifact + depends on OAP `xray` + is imported by two OAP consumers. |

### OAP→Spec-Spine inbound edges (what would need to be replumbed)

| OAP source | What it reaches into | Replumb option |
|-----------|----------------------|----------------|
| `apps/desktop/src-tauri/src/commands/analysis.rs` | reads `build/spec-registry/registry.json` raw + uses `featuregraph` | Could read via `registry-consumer --json` subprocess, or via featuregraph staying in OAP. |
| `crates/axiomregent/{feature_tools.rs, lib.rs, router/legacy_provider.rs}` | `featuregraph::*` | Same — featuregraph could either go with Spec-Spine or stay in OAP. |
| `tools/policy-compiler` (already inside `tools/`) | `crates/policy-kernel` | Inverted edge: policy-compiler should logically be in OAP (it depends on OAP). The fact that it's in `tools/` and ships in `release-tools.yml` is misleading. |
| `tools/stakeholder-doc-lint`, `tools/assumption-cascade-check` | `crates/factory-{contracts,engine}`, `crates/provenance-validator` | These are OAP lints living in `tools/`. Not spec-spine. |
| (Indirect) `crates/factory-contracts/src/provenance.rs` references the registry.json path (grep hit on filename) — needs Phase 3 to confirm. | TBD | TBD |

### Findings that surprised the working model

1. **`registry-consumer` does not depend on `open_agentic_frontmatter`.**
   It does not parse markdown; it parses `registry.json` produced by
   spec-compiler. The "spec-spine library" mental model where
   `registry-consumer` shares parsing code with `spec-compiler` is
   wrong. They share *no* code today.
2. **`spec-compiler` and `spec-lint` carry duplicate `SHAPE_TABLE`**
   (`tools/spec-compiler/src/lib.rs:115-132` vs.
   `tools/spec-lint/src/lib.rs:46-63`). The spec-lint comment says
   "Mirrors `SHAPE_TABLE` in `tools/spec-compiler/src/lib.rs`" —
   acknowledged duplication.
3. **`featuregraph` redeclares the registry shape** in its own
   `CompiledRegistry`/`RegistryFeatureRecord` types
   (`registry_source.rs:14-70`) and `load_registry_records` reads
   `registry.json` directly. The "consumer binaries are the only
   readers" rule in spec 103 is violated by featuregraph — but it's a
   crate, not an orchestrated workflow, so the rule may not apply.
   Worth confirming in Phase 4.
4. **`spec-code-coupling-check` consumes the `codebase-indexer` library
   for its `types::CodebaseIndex` struct** — i.e., the gate doesn't read
   `index.json` via JSON parsing; it pulls the strongly-typed
   deserializer from the producer crate. This is a spec 103 sanctioned
   exception ("the consumer binary IS allowed to parse its own
   artifact") but it locks the two crates to the same schema version.
5. **The release-tools workflow bundles `policy-compiler`** alongside
   the four canonical Spec-Spine binaries. That's a packaging fact
   that contradicts a clean Spec-Spine boundary — see Phase 6.
6. The constitution names `specs/000-…/spec.md` as the highest-
   precedence document but the actual grammar (KNOWN_KEYS, VALID_KINDS,
   SHAPE_TABLE, V-xxx codes) lives in `spec-compiler/src/lib.rs`
   constants and inline comments. Phase 7 will quantify the
   docs/implementation gap.

---

## Phases 3–8 — stubs (not yet executed)

Per the brief, I am stopping after Phase 2 to let the dependency map
reshape the framing of the rest. The remaining phases are scoped below
so you can confirm or redirect.

### Phase 3 — Public surface as it stands
*Planned:* enumerate every `pub` item across the six candidate crates,
flag OAP-type leaks in signatures, and identify `pub(crate)`/private
items that would need to become `pub` after extraction. Initial grep
shows ≈40 public items across the six candidates; the hidden-API surface
will mostly be in `spec-compiler` (1.8k LoC, all crammed into one
`lib.rs`).

### Phase 4 — Coupling classification (lift / invert / trait-ify / duplicate / unclear)
*Planned:* per Phase 2's edge table. The high-value buckets I currently
expect: **invert** for the OAP-shaped scanners inside codebase-indexer;
**trait-ify** for the directory-scanning passes (Layer 3/4/5 want to
become trait-based plugins); **duplicate** for the small `SHAPE_TABLE`
double; **unclear** for `featuregraph` and for the `factoryProjects`
emission in spec-compiler.

### Phase 5 — Test & fixture footprint
*Planned:* a tabular pass. Initial counts: registry-consumer has 41
fixture files (all schema-of-registry, all generic-shaped); spec-compiler
has 9 integration test files of which `factory_projects.rs` (213 LoC)
and parts of `code_aliases.rs` carry OAP-shaped fixtures. codebase-
indexer tests are inline-string golden tests with OAP-shaped expectations.

### Phase 6 — Build, tooling, CI footprint
*Partial evidence:* `.github/workflows/spec-conformance.yml` runs
spec-compiler + registry-consumer (+ 10 contract subsets) + spec-lint
(`--fail-on-warn`) + codebase-indexer + policy-compiler.
`ci-codebase-index.yml`, `ci-spec-code-coupling.yml`, `release-tools.yml`,
`ci-supply-chain.yml` all touch the candidate set. The Makefile has
~30 lines explicitly building/running these tools. Full mapping pending.

### Phase 7 — Format-as-contract assessment
*Planned:* compare what's documented in
`specs/000-bootstrap-spec-system/spec.md`,
`.specify/contract.md`, and `docs/registry-consumer-contract-governance.md`
against what the implementation actually does (KNOWN_KEYS, VALID_KINDS,
the V-xxx code registry, the W-xxx code registry, the spec 103
governed-read rule). My initial impression: identifier rules and link
semantics are documented; resolution order is partially documented;
the "what counts as a known frontmatter key" question is *only* in
spec-compiler source. Quantification pending.

### Phase 8 — The G-2 question
*Initial answer (medium confidence):* the G-2 governance-certificate
pipeline does not import any spec-spine library.
`crates/factory-engine/src/governance_certificate.rs` (936 LoC) carries
zero grep hits for `registry.json`, `codebase-index`, `open_agentic_*`
or `featuregraph`. The only spec-spine touch point is an
`Option<String> spec_id` field on `StageRecord`. So:
- Extracting Spec-Spine today does **not** simplify G-2's hot path.
- Extracting Spec-Spine today does **not** complicate G-2's hot path
  either.
- The version of extraction that *would* affect G-2 is one that forces
  `registry-consumer` (or a successor crate) to publish a typed Rust
  client used by the certificate emitter — closing the gap where
  factory-engine stamps `spec_id` but no one verifies it resolves.
  That's worth doing on its own merits; whether it requires extraction
  is a Phase 4 call.

This phase will be filled in fully after you confirm the framing.

---

## Open Questions

1. **Does `featuregraph` go with Spec-Spine or stay in OAP?** It
   bridges both worlds. It is the only `crates/*` consumer of
   `registry.json`. It also depends on `xray` (OAP). If it goes with
   Spec-Spine, two OAP crates (`axiomregent`, `apps/desktop`) flip from
   internal imports to external-crate consumers. If it stays in OAP,
   it remains the only sanctioned-but-unblessed parser of
   `registry.json` outside `registry-consumer`.
2. **What's the Spec-Spine stance on `factoryProjects` / `compliance`
   in the registry?** spec-compiler today emits both as registry fields.
   A "generic" Spec-Spine compiler wouldn't know about either. Options:
   keep them (Spec-Spine ships with OAP-specific extensions baked in),
   add a plugin/extension mechanism (real surface), or drop them from
   the compiler core and let an OAP-side enricher append them.
3. **What's the Spec-Spine stance on the codebase-indexer's
   `factory/`, `.claude/`, `.github/workflows/` scanners?** They're
   structurally inside the candidate but semantically OAP.
4. **Does `policy-compiler` ship with Spec-Spine?** It bundles in
   `release-tools.yml` alongside the four canonical Spec-Spine
   binaries but depends on `crates/policy-kernel` (an OAP crate). It
   should logically be in OAP, but the current release contract
   already promises it to consumers.
5. **Do you want `frontmatter` extracted as its own crate, or inlined
   into `spec-compiler`?** It's 40 lines and the only out-of-band shared
   utility. Either is defensible.
6. **Is the "spec spine" you're considering extracting (a) the four
   binaries that release-tools.yml ships, (b) the candidate set I
   identified, or (c) a smaller set like just spec-compiler +
   registry-consumer?** The answer rewrites the rest of the analysis.

## Surprises

1. **G-2's pipeline doesn't touch spec spine** — at all. The brief
   framed G-2 as "the launch blocker" related to extraction; the code
   says G-2 and spec-spine are orthogonal today. If anything, the link
   is via `spec_id: Option<String>` which is never validated against
   the registry. Extraction won't make G-2 easier; *closing this
   validation gap* will, and it can be done with or without extraction.
2. **The "consumer binaries are the only readers" rule has one
   internal violator.** `crates/featuregraph` redeclares
   `CompiledRegistry` and parses `registry.json` itself. This isn't a
   problem at workflow level (it's a crate, not a workflow), but it
   contradicts the architectural narrative.
3. **`registry-consumer` has zero library consumers** (no `use
   open_agentic_registry_consumer` outside its own binary). The
   "registry-consumer is the library used to read registry.json" idea
   is aspirational, not load-bearing — featuregraph rolls its own,
   stagecraft TS reads spec.md paths directly via Tauri file IO, and
   the codebase-indexer reads `spec.md` files directly (not the
   compiled registry) via its `spec_scanner.rs`.
4. **The codebase-indexer reads `spec.md` files directly**
   (`spec_scanner.rs:3`), not the compiled `registry.json`. So
   spec-compiler's output and codebase-indexer's output are computed
   from the *same source* (`specs/*/spec.md`) in parallel rather than
   one consuming the other. There's no chain `spec.md → spec-compiler →
   registry.json → codebase-indexer → index.json`. The two pipelines
   are independent reads of the corpus.
5. **OAP concepts leak into the spec-spine producer (spec-compiler),
   not just the index.** It's not "the indexer is OAP-shaped but the
   compiler is generic" — both have OAP concepts hardcoded.
6. **Test coupling is small but specific.** The single OAP-shaped test
   file in spec-compiler is `factory_projects.rs`; everything else is
   format-generic. spec-lint and registry-consumer tests are
   format-generic. codebase-indexer tests are inline-string-golden and
   carry OAP-shaped expectations throughout.

---

*End of Phase 2 checkpoint. Awaiting direction before continuing to
Phase 3.*
