---
id: "000-bootstrap-spec-system"
title: "Bootstrap spec system (markdown → compiled JSON registry)"
feature_branch: "000-bootstrap-spec-system"
status: approved
implementation: complete
kind: constitutional-bootstrap
created: "2026-03-22"
ratified: "2026-03-22"
authors:
  - "open-agentic-platform"
language: en
summary: >
  Foundational contract: authored truth lives only in markdown (with YAML frontmatter
  blocks permitted inside .md); machine-consumable truth is compiler-emitted JSON only;
  full markdown → registry compilation from day one; no standalone authored YAML.
---

# Feature Specification: Bootstrap spec system

**Feature Branch**: `000-bootstrap-spec-system`  
**Created**: 2026-03-22  
**Ratified**: 2026-03-22 (ratification pass: dates, determinism split, frontmatter rules, V-005 scope)  
**Status**: Approved  
**Input**: Constitutional bootstrap — establish spec-first markdown → compiled JSON registry for the new repository.

## Purpose and charter

This feature is the **constitutional bootstrap** for `open-agentic-platform`. It defines how every later capability—including future **axiomregent** (governance interfaces), **xray** (repository analysis), and **featuregraph** (feature registry views)—must **attach to authored markdown** and **consume compiler-owned JSON**, rather than inventing parallel human-edited machine formats.

Scope of Feature 000:

- Normative rules for **what may be authored** (markdown only) and **what may be machine truth** (JSON emitted by the spec compiler only).
- A **minimum document grammar** for feature specs and related authored documents.
- A **minimum viable compiled registry** contract and **determinism** requirements.
- **Validation invariants** and **provenance** rules for reverse-engineered or legacy-informed work.
- The **initial `.specify/` layout contract** so Spec Kit workflows remain aligned with repository architecture.

Explicitly **out of scope** for Feature 000:

- Product runtime code (services, daemons, UI) except the **minimal compiler/scaffold** strictly required to compile specs to JSON.
- Full ontology for the agentic platform, policy engines, or graph schemas beyond what the MVP registry must carry.
- Preservation of repository boundaries or filenames from legacy projects (`opc`, `platform`) unless they serve this contract.

## Why this repository is spec-first

The platform is being rebuilt **from specifications**, not from incidental code structure. A spec-first workflow:

- Makes contracts **reviewable** before implementation exists.
- Lets humans reason in **natural language and structured markdown**, while tooling reasons over **stable JSON**.
- Prevents “truth drift” between informal docs and executable config.

Spec-first is non-negotiable: **implementation work is justified by specs**, not the reverse.

## Authored truth: markdown only (including frontmatter)

**All human-authored durable truth** in this repository MUST be expressed as **Markdown files** (`.md`), optionally using a **YAML frontmatter block** at the top of the file delimited by `---` lines.

**Frontmatter rules (normative):**

- Frontmatter is **metadata embedded in a `.md` file only**. It is **not** an independent authoring format and MUST NOT be extracted to standalone YAML or treated as a parallel schema.
- No tool or workflow may use frontmatter as a **free-form escape hatch** to park system semantics, large structured config, or “the other half” of the platform in YAML. If content does not belong in prose or in the **document grammar** for allowed keys, it belongs in a **new or existing feature spec**, not in unbounded frontmatter.
- Allowed frontmatter keys are those required or optional under **Markdown document grammar** and any **explicitly documented** extension per feature; ad-hoc keys MAY appear but MUST map into **`extraFrontmatter`** in the compiled registry (see data model) with **strict cardinality limits**—never a wholesale YAML blob in JSON.

Rationale:

- Markdown is diff-friendly, review-native, and suitable for narrative plus light structure.
- YAML **only** as frontmatter inside `.md` keeps a single reviewable artifact while avoiding “YAML by stealth.”

## Machine truth: compiler-owned JSON only

**All machine-oriented durable truth** (indices, registries, hashes, normalized metadata extracted from specs) MUST live in **JSON files** produced **only** by the **spec compiler** (or a tool explicitly designated as that compiler in a later feature). Humans MUST NOT hand-edit machine JSON except through automated regeneration.

Rationale:

- JSON maps cleanly to automation, schema validation, and language-agnostic consumers.
- Restricting machine truth to compiler output prevents competing “sources of truth” (e.g., hand-edited YAML registries alongside specs).

## Forbidden: standalone authored YAML

The following are **forbidden** in this repository’s **authored** surface area:

- Standalone `.yaml` / `.yml` files written or maintained by contributors (e.g. `meta.yaml`, `approvals.yaml`, `traceability.yaml`, CI-authored config exceptions).
- Any pattern that makes YAML the **authoritative** parallel channel for the same facts already expressed in specs.

**Not forbidden:** YAML **inside** markdown frontmatter, subject to **Frontmatter rules** above.

**Rejected legacy pattern:** Reverse-engineered flows that passed a **`features_yaml_path`** (or similar) as the registry input. In this repository, **features are defined in markdown**; any YAML-shaped interchange is **compiler output**, not an authoring format.

## Initial `.specify/` contract (future features)

The following **normative layout** applies to Spec Kit features in this repo:

| Path | Role |
|------|------|
| `.specify/memory/constitution.md` | Human-authored principles; subordinate to feature `000` where bootstrap rules are stricter. |
| `.specify/templates/*.md` | Authoring aids. Templates remain markdown. Must not require standalone YAML sidecars. |
| `.specify/scripts/bash/*.sh` | Workflow glue; may emit JSON **only** if a later feature explicitly designates those outputs as compiler products (otherwise scripts remain non-authoritative helpers). |
| `specs/<NNN>-<kebab-name>/spec.md` | **Authoritative feature specification** for numbered feature `NNN`. |
| `specs/<NNN>-<kebab-name>/plan.md` | Implementation plan (markdown). |
| `specs/<NNN>-<kebab-name>/tasks.md` | Task list (markdown; no YAML document header required). |
| `specs/<NNN>-<kebab-name>/contracts/` | **Optional** JSON Schema or example JSON **for machine contracts** tied to the feature; not a parallel authoring channel. |
| `build/spec-registry/` (or path fixed in a later implementation task) | **Compiler-emitted JSON**: deterministic `registry.json` and ephemeral `build-meta.json` (see below). |

**Repository location of feature specs (sticky decision):** Authoritative feature specs live under **`specs/<NNN>-<kebab-name>/` at the repository root**, not under `.specify/specifications/` or other tool-internal trees. This keeps specs **first-class repo content**, visible in review and branching like source code. `.specify/` holds **templates, scripts, and constitution**—not the canonical feature library. A future amendment may add mirrors or symlinks, but **must not** introduce a second authoritative path without superseding this spec.

**Feature ID rule:** Directory name MUST be `NNN-kebab-case` where `NNN` is three decimal digits, zero-padded. Feature `000` is reserved for this bootstrap contract.

## Markdown document grammar (normative minimum)

Every **feature spec** (`specs/*/spec.md`) MUST contain:

1. **YAML frontmatter** (between opening and closing `---` lines) with at least:
   - `id` — string, MUST equal `<NNN>-<kebab-name>` and match the parent directory name.
   - `title` — non-empty string.
   - `status` — enum string (`draft` \| `approved` \| `superseded` \| `retired`) — the design lifecycle status.
   - `implementation` — optional enum string (`pending` \| `in-progress` \| `complete` \| `n/a` \| `deferred`) — the implementation lifecycle status.
   - `created` — ISO 8601 date string.
   - `summary` — single-line or folded summary string (required; compiled into the registry as a **normalized** field, not only as raw YAML).

2. **Body** after frontmatter using level-1 heading for the document title, followed by structured sections. Required sections for feature specs follow the Spec Kit template **adapted as needed**, but MUST always include:
   - **User Scenarios & Testing** (or equivalently named mandatory section for testable journeys).
   - **Requirements** with uniquely identifiable requirements (e.g. `FR-NNN`).
   - **Success Criteria** with measurable outcomes.

Other authored markdown (e.g. plans, checklists) SHOULD include minimal frontmatter with `id`, `title`, `created` when they are tied to a feature directory.

## Minimum viable compiled JSON registry contract

The compiler MUST emit two related JSON artifacts under `build/spec-registry/`:

1. **`registry.json`** (deterministic) — conforms to `contracts/registry.schema.json`. Minimum semantic content:
   - **`specVersion`** — registry format version string.
   - **`build`** — `compilerId`, `compilerVersion`, `inputRoot`, and a **single deterministic `contentHash`** over canonical inputs. **No wall-clock timestamp** in this file (see determinism below).
   - **`features`** — ordered array of feature records: **normalized fields** from frontmatter (`id`, `title`, `status`, `created`, `summary`, optional `authors`, `kind`, `featureBranch`), **`specPath`**, **`sectionHeadings`**, and optionally **`extraFrontmatter`** (only for keys not mapped to normalized fields; max **8** keys; values constrained by schema **`extraFrontmatterValue`**; no dumping of full parsed YAML).
   - **`validation`** — aggregate pass/fail and a list of violations with stable error codes.

2. **`build-meta.json`** (ephemeral, compiler-owned) — conforms to `contracts/build-meta.schema.json`. Carries **`builtAt`** (UTC wall-clock) and optional duplicate `compilerId` / `compilerVersion` for log correlation. **Not** included in determinism or golden-file equality checks.

Downstream consumers that require **reproducible** output MUST depend only on **`registry.json`**. Ops, CI logs, and humans MAY read **`build-meta.json`**.

The MVP **may** omit large markdown bodies from JSON as long as **every feature record** references the authoritative `.md` path and normalized metadata is **complete for validation**.

## Deterministic compiler expectations

**Deterministic artifact:** `registry.json` only.

Given the **same committed input file tree** (markdown sources designated as compiler inputs) and the **same compiler version**:

- The compiler MUST produce **byte-identical `registry.json`** when run repeatedly (same OS/architecture normalization rules as declared in `research.md`).
- **`build-meta.json` is explicitly out of scope** for byte-identical comparison (its `builtAt` changes every run).
- Canonicalization MUST include: UTF-8 encoding; stable key ordering in emitted JSON; stable sorting of arrays derived from unordered sets; and a documented rule for newline normalization when hashing path contents.

If nondeterminism is discovered **in `registry.json`**, it is a **compiler bug**, not an authoring workaround.

## Minimum validation invariants

The compiler MUST reject or emit `validation.passed: false` when:

- **V-001**: Any `specs/<NNN>-*/spec.md` is missing or unreadable.
- **V-002**: Frontmatter is missing required keys or `id` does not match directory name.
- **V-003**: Duplicate feature `id` values exist across the tree.
- **V-004**: A standalone `.yaml` or `.yml` file appears under authored paths designated by policy (default: entire repo except `node_modules/`, `.git/`, and explicitly listed third-party vendored paths in a later amendment).
- **V-005** (**reserved; not MVP-enforced**): Hand-edited or non-compiler mutation of JSON under the compiler output directory SHOULD be detectable in a future phase (heuristic TBD: e.g. tamper-evident hash, read-only CI artifacts). The **intent** is workflow integrity, not a cryptographic guarantee in Feature 000. Until enforcement exists, implementations MUST NOT claim full **V-005** compliance; they MAY emit **warnings** only if explicitly implemented.

## Reverse-engineered provenance expectations

Legacy repositories `~/Dev2/opc` and `~/Dev2/platform` are **evidence only**. When a future feature is justified by reverse engineering:

- The feature spec MUST record a **Provenance** subsection: which legacy concept informed it, and **what was rejected or merged**.
- No feature may claim compliance solely by **porting** a legacy filename or crate boundary; alignment must be **contractual** (this registry and specs).

**Inference from `opc` (non-exhaustive):** A future **featuregraph** MUST consume **this registry JSON**, not a hand-maintained YAML feature list. **xray** remains an **analysis** concern and MUST NOT become a parallel registry format. **axiomregent** remains **governance MCP** semantics and MUST treat **spec violations** as first-class policy inputs in a later feature, not in Feature 000.

## Legacy assumptions explicitly rejected

- **Rejected:** “Feature registry lives in YAML on disk for tools to edit.”
- **Rejected:** “Each historical repo boundary gets a permanent top-level directory by default.”
- **Rejected:** “Markdown is informal; YAML/JSON is the real spec.” Here, **markdown is the real human spec**; JSON is **derived**.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Author a feature spec that compiles cleanly (Priority: P1)

A contributor authors `specs/001-example/spec.md` with valid frontmatter and required sections. They run the compiler. The emitted registry lists `001-example` with no validation violations.

**Why this priority:** Without this, the system fails its primary contract.

**Independent Test:** Compile a single minimal feature directory in isolation (fixtures in a later task).

**Acceptance Scenarios**:

1. **Given** valid spec markdown, **When** the compiler runs, **Then** `validation.passed` is true and the feature appears in `features[]` with correct `id`.
2. **Given** an `id` / directory mismatch, **When** the compiler runs, **Then** validation fails with a stable error code.

---

### User Story 2 — Reviewer verifies no forbidden authored YAML (Priority: P2)

A reviewer searches the repo for standalone `.yml`/`.yaml` and finds none in authored areas; CI enforces the same.

**Why this priority:** Prevents silent reintroduction of a competing authoring channel.

**Independent Test:** Static policy check independent of product features.

**Acceptance Scenarios**:

1. **Given** a new `foo.yaml` added under `docs/`, **When** validation runs, **Then** the build fails with **V-004**.

---

### User Story 3 — Consumer reads stable JSON (Priority: P3)

A downstream tool (placeholder for future featuregraph) reads **`build/spec-registry/registry.json`** for deterministic feature metadata and needs no markdown parsing. It does **not** require `build-meta.json`.

**Why this priority:** Proves the separation of human and machine layers.

**Independent Test:** JSON Schema validation of `registry.json` against `contracts/registry.schema.json`.

**Acceptance Scenarios**:

1. **Given** emitted `registry.json`, **When** validated against the schema, **Then** validation succeeds.

---

### Edge Cases

- **Empty repository (only Feature 000):** Compiler still emits a valid registry with `features` possibly empty or containing only bootstrap metadata — behavior fixed in implementation but MUST remain schema-valid.
- **Editor-inserted BOM:** Compiler either strips UTF-8 BOM for hashing or fails deterministically; behavior must be documented once chosen.
- **Concurrent edits:** Determinism applies to committed snapshots, not uncommitted editor buffers.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Repository MUST define a single **normative bootstrap spec** (this document) that future features extend rather than contradict without explicit supersession.
- **FR-002**: Authoring MUST be **markdown-centric**; machine registries MUST be **compiler-emitted JSON** only.
- **FR-003**: Standalone authored YAML MUST be forbidden subject to invariant **V-004**.
- **FR-004**: The spec compiler MUST implement **full compilation** from designated markdown inputs to the **MVP outputs**: deterministic **`registry.json`** plus ephemeral **`build-meta.json`** (full here means “all spec inputs → normalized registry,” not “include full markdown bodies in JSON”).
- **FR-005**: The compiler MUST be **deterministic** per rules in this spec.
- **FR-006**: The compiler MUST implement **minimum validation** rules **V-001** through **V-004** for MVP. **V-005** is **reserved** (see invariants); full enforcement is explicitly **post-MVP** unless a later feature promotes it.
- **FR-007**: Feature directories MUST follow `specs/NNN-kebab-case/` with matching `id` frontmatter.
- **FR-008**: Provenance from legacy repos MUST be **declared in spec text** when used, per “Reverse-engineered provenance expectations.”

### Key Entities

- **Feature Spec Document**: A markdown file with frontmatter + body; authoritative human record for a numbered feature.
- **Spec Compiler**: The tool that reads markdown inputs and writes **`registry.json`** and **`build-meta.json`**; only component allowed to author those machine JSON artifacts.
- **Registry Record**: A JSON object describing one feature’s **normalized** metadata (`id`, `title`, `status`, `created`, `summary`, …), optional **`extraFrontmatter`** (capped), and references to its markdown source—**not** a dump of raw parsed frontmatter.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A reviewer can state, without inspecting implementation, whether a change violates markdown-only / JSON-only rules by reading Feature 000 alone.
- **SC-002**: For a golden fixture tree, two compiler runs produce **byte-identical `registry.json`** (ephemeral `build-meta.json` excluded from this criterion).
- **SC-003**: `registry.json` emitted for a sample feature passes **`contracts/registry.schema.json`** validation with no errors.
- **SC-004**: Adding a standalone `test.yaml` under a covered path causes validation to fail with **V-004** in CI (once CI exists).

## Clarifications

### Session 2026-03-22 (initial)

- Interactive `/speckit.clarify` loop was **not required**: bootstrap rules were drafted as a complete constitutional contract. See `clarify.md` for the handoff note.

### Session 2026-03-22 (ratification pass)

- Normalized documentation dates to **2026**.
- Confirmed **repo-root `specs/`** as the sole authoritative feature-spec tree (vs `.specify/specifications/`).
- Tightened **frontmatter** rules to forbid YAML-as-escape-hatch semantics.
- Resolved **determinism vs `builtAt`**: split **`registry.json`** (deterministic) and **`build-meta.json`** (ephemeral); updated schema and success criteria.
- Replaced bulk **`frontmatter`** blob in registry with **normalized fields** + optional **`extraFrontmatter`** (capped).
- Marked **V-005** as **reserved**, not MVP-mandatory enforcement.
