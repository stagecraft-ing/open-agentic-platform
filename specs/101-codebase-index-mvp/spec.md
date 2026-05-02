---
id: "101-codebase-index-mvp"
title: "Codebase Index MVP — Governed Structural Inventory"
status: approved
implementation: complete
owner: bart
created: "2026-04-14"
kind: tooling
summary: >
  A deterministic indexer tool that walks the repository tree, parses manifest files
  and spec frontmatter, and emits a governed build/codebase-index/index.json artifact.
  Provides four-layer structural inventory: crate/package inventory, spec-to-code
  traceability, factory adapter coverage, and tool/infrastructure catalog. Follows the
  same compiler-emits-artifact pattern established by spec-compiler (001).
depends_on:
  - "000"  # bootstrap-spec-system (artifact pattern)
  - "001"  # spec-compiler-mvp (registry pattern)
  - "003"  # feature-lifecycle-mvp (status vocabulary)
code_aliases: ["CODEBASE_INDEX"]
risk: low
---

# 101 — Codebase Index MVP

## 1. Problem Statement

OAP has grown to 100+ specs, 15+ Rust crates, multiple TypeScript packages, factory
adapters, platform services, and a deep `.claude/` agent/command infrastructure. No
single artifact provides a machine-readable, structurally accurate inventory of what
exists, what depends on what internally, and how code maps back to governing specs.

Contributors (human and agent) must manually explore the tree to orient themselves.
The `/init` command and explorer agent spend significant tokens re-discovering
structure that should be pre-computed. This is the exact kind of structural knowledge
that should be governed — compiled, verified, and kept current — not guessed or
re-derived per session.

### Why not just read the filesystem?

Reading the filesystem gives you file paths. It does not give you:

- Which crate depends on which other crate (internal dependency graph)
- Which specs govern which code (traceability)
- Which specs have no implementing code (orphaned specs)
- Which code has no governing spec (untraced code)
- Which factory adapters cover which pipeline stages
- What the `.claude/` agent/command inventory looks like

These require cross-referencing multiple manifest files and spec frontmatter — exactly
what a compiler does.

## 2. Solution

### 2.1 The Index as a Build Artifact

A new Rust tool `tools/codebase-indexer/` reads the repository tree and emits
`build/codebase-index/index.json`. This follows the identical pattern established by
the spec-compiler:

```
repo tree  →  codebase-indexer compile  →  build/codebase-index/index.json
                                        →  build/codebase-index/build-meta.json
```

The JSON schema lives at `schemas/codebase-index.schema.json` and is itself a
governed contract.

A markdown renderer mode emits `build/codebase-index/CODEBASE-INDEX.md` from the
JSON — this is the human-readable view. It is never hand-authored.

### 2.2 Four-Layer Schema

The index covers four structural layers:

**Layer 1 — Crate & Package Inventory**

Derived from `Cargo.toml` and `package.json` manifests. No LLM interpretation.

| Field | Source | Description |
|-------|--------|-------------|
| `name` | manifest `[package].name` or `"name"` | Canonical package name |
| `path` | directory relative to repo root | Location |
| `kind` | `lib`, `bin`, `npm-package`, `npm-workspace` | Package classification |
| `version` | manifest version field | Declared version |
| `entryPoint` | `src/lib.rs`, `src/main.rs`, `"main"` | Primary entry |
| `internalDeps` | `[dependencies]` matching known crate names | Internal dependency edges |
| `externalDeps` | remaining deps | External dependency list |

**Layer 2 — Spec-to-Code Traceability**

Cross-references spec frontmatter `implements` declarations against actual file paths.

| Field | Source | Description |
|-------|--------|-------------|
| `specId` | spec frontmatter `id` | Governing spec |
| `implementingPaths` | frontmatter `implements` list or `[package.metadata.oap].spec` | Code locations |
| `orphanedSpecs` | specs with no `implements` and no Cargo.toml back-reference | Unimplemented specs |
| `untracedCode` | crates/packages with no governing spec reference | Ungoverned code |

**Layer 3 — Factory Adapter Inventory**

Derived from `factory/adapters/*/manifest.yaml` files and the global pipeline stage
definitions in `factory/process/stages/`.

| Field | Source | Description |
|-------|--------|-------------|
| `name` | `adapter.name` from manifest.yaml | Adapter identifier |
| `path` | relative path | Location |
| `displayName` | `adapter.display_name` from manifest.yaml | Human-readable name |
| `targetStack` | adapter directory name | Technology stack |
| `stackLanguage` | `stack.language` from manifest.yaml | Primary language |
| `stackRuntime` | `stack.runtime` from manifest.yaml | Runtime environment |
| `phaseCoverage` | `factory/process/stages/*.md` | Global pipeline stages (shared across all adapters) |

> **Note:** Pipeline stages are global process definitions, not per-adapter stage
> files. Numbered stages run the canonical 7-stage build (`00-pre-flight` through
> `06-adapter-handoff`); conditional stages use a 2-letter prefix (e.g.
> `cd-client-documentation`) and run with NOW/SKIP/DEFERRED scheduling. All
> adapters share the same pipeline; individual adapter capabilities are
> expressed through the manifest's `capabilities`, `agents`, and `patterns`
> sections, not through stage presence.

**Layer 4 — Tool & Infrastructure Inventory**

Catalogs tools, agents, commands, and rules.

| Field | Source | Description |
|-------|--------|-------------|
| `tools` | `tools/*/Cargo.toml` | CLI tool inventory |
| `agents` | `.claude/agents/*.md` | Agent definitions |
| `commands` | `.claude/commands/**/*.md` | Command definitions |
| `rules` | `.claude/rules/*.md` | Rule files |
| `schemas` | `schemas/*.json` | JSON Schema contracts |

### 2.3 Traceability Convention

This spec introduces one new optional convention. Specs may declare which code
implements them via an `implements` key in frontmatter:

```yaml
---
id: "001-spec-compiler-mvp"
status: active
implements:
  - crate: spec-compiler
    path: tools/spec-compiler
  - crate: registry-consumer
    path: tools/registry-consumer
---
```

And `Cargo.toml` files may carry a back-reference:

```toml
[package.metadata.oap]
spec = "001-spec-compiler-mvp"
```

Both directions are optional. The indexer cross-references whatever is declared and
reports orphans in both directions. This is designed for incremental adoption — the
orphan report shows what's untraced and gaps close over time.

**Note:** The `implements` key is not in the spec-compiler's `KNOWN_KEYS` list. It
will land in `extraFrontmatter` as a structured value. A follow-up may promote it to
a first-class key if adoption warrants (requires spec-compiler change per the
`V-002` nested-mapping validation rule — the indexer would read it from
`extraFrontmatter` in the interim, or parse spec frontmatter directly).

### 2.4 Indexer Implementation

The indexer is a Rust binary at `tools/codebase-indexer/`. It follows the
spec-compiler's architecture:

```
src/
  main.rs          — CLI entry (clap): `compile`, `render`, `check`
  lib.rs           — Core indexing logic
  manifest.rs      — Cargo.toml and package.json parsers
  spec_scanner.rs  — Spec frontmatter reader (reuses spec-compiler patterns)
  factory.rs       — Factory adapter scanner
  infra.rs         — Tool/agent/command/rule scanner
  xref.rs          — Cross-reference engine (Layer 2)
  schema.rs        — JSON Schema validation of output
  render.rs        — Markdown renderer
```

Subcommands:

- `codebase-indexer compile` — full index, emits `index.json` + `build-meta.json`
- `codebase-indexer render` — emits `CODEBASE-INDEX.md` from existing `index.json`
- `codebase-indexer check` — exits non-zero if `index.json` is stale vs current tree

### 2.5 CI Integration

Same pattern as the spec registry:

- `codebase-indexer check` runs in CI and fails the build if the index is stale
- PRs that add/remove/move crates, packages, specs, adapters, or tools must update
  the index as part of the change

### 2.6 Agent Orientation

Once the index exists, any Claude Code agent can read `build/codebase-index/index.json`
on startup and immediately understand:

- What crates and packages exist and where
- What depends on what internally
- Which specs govern which code
- What's orphaned in either direction
- What factory adapters are available and their coverage

This replaces expensive per-session tree-walking with a single file read.

## 3. Functional Requirements

### FR-01: Manifest Parsing

The indexer MUST parse `Cargo.toml` files to extract: package name, version, edition,
`[[bin]]` targets, `[lib]` presence, `[dependencies]` (distinguishing workspace members
from external crates), and `[package.metadata.oap]` if present.

### FR-02: Package.json Parsing

The indexer MUST parse `package.json` files to extract: name, version, main/module
entry points, dependencies, devDependencies, and workspaces configuration.

### FR-03: Spec Frontmatter Scanning

The indexer MUST parse spec frontmatter from all `specs/*/spec.md` files, extracting
at minimum: `id`, `status`, `implementation`, `depends_on`, and `implements` (if present
in `extraFrontmatter` or as a direct field).

### FR-04: Internal Dependency Graph

The indexer MUST compute the internal dependency graph by resolving `[dependencies]`
entries that reference other workspace members (by name or path).

### FR-05: Cross-Reference Engine

The indexer MUST cross-reference spec `implements` declarations against actual
filesystem paths and `[package.metadata.oap].spec` back-references. Mismatches
(declared path doesn't exist, back-reference points to non-existent spec) MUST be
reported as warnings.

### FR-06: Orphan Detection

The indexer MUST identify:
- **Orphaned specs**: specs with `implementation != n/a` that have no `implements`
  declaration and no `Cargo.toml` back-reference pointing to them
- **Untraced code**: crates/packages with no governing spec (no `implements` reference
  from any spec and no `[package.metadata.oap].spec`)

### FR-07: Factory Adapter Scanning

The indexer MUST scan `factory/adapters/*/manifest.yaml` and report: adapter name,
display name, path, target stack, language, runtime, and version. The indexer MUST
also scan `factory/process/stages/*.md` to report the global pipeline phase list
(shared across all adapters).

### FR-08: Infrastructure Scanning

The indexer MUST inventory:
- `tools/*/` entries (name, path, binary targets)
- `.claude/agents/*.md` (name, description from frontmatter)
- `.claude/commands/**/*.md` (name, path)
- `.claude/rules/*.md` (name, path)
- `schemas/*.json` (name, path)

### FR-09: JSON Schema Validation

The emitted `index.json` MUST validate against `schemas/codebase-index.schema.json`.
The indexer MUST validate its own output before writing.

### FR-10: Staleness Check

`codebase-indexer check` MUST compare the current repo state against the existing
`index.json` content hash and exit non-zero if they differ.

### FR-11: Markdown Rendering

`codebase-indexer render` MUST produce a human-readable markdown document from
`index.json` that presents all four layers in a structured format.

## 4. Success Criteria

### SC-01: Deterministic Output

Running `codebase-indexer compile` twice on the same repo state MUST produce
byte-identical `index.json` output (same content hash).

### SC-02: Complete Inventory

The index MUST include every Rust crate in `crates/` and `tools/`, every
`package.json` in `apps/` and `platform/services/`, and every spec in `specs/`.

### SC-03: Accurate Dependencies

Internal dependency edges MUST match actual `Cargo.toml` `[dependencies]` entries.
No false positives (edges that don't exist in manifests), no false negatives
(manifest deps on workspace members that are missing from the graph).

### SC-04: Orphan Coverage

The orphan report MUST correctly identify at least the known untraced crates and
unimplemented specs that exist as of spec creation date.

### SC-05: CI Enforcement

A PR that adds a new crate without updating `index.json` MUST fail the CI check.

### SC-06: Agent Startup Acceleration

After index exists, the `/init` command MUST be able to load structural context from
`index.json` instead of walking the tree, reducing init token cost.

## 5. Out of Scope (MVP)

- **Runtime/deployment topology** (Layer 5) — future spec
- **Call graph or symbol-level indexing** — xray crate handles this separately
- **Automatic `implements` inference** — MVP requires explicit declaration
- **Spec-compiler modification** — MVP reads `implements` from `extraFrontmatter`;
  promoting it to a first-class key is a follow-up
- **Cross-repo indexing** — this indexes only the OAP monorepo

## 6. Clarifications

- The indexer is a **deterministic Rust binary**, not an LLM-driven agent. The
  explorer/architect/implementer agent workflow described in the design discussion is
  the bootstrapping approach for the first pass. The long-term path is the compiled tool
  running in CI without any LLM in the loop.
- The `implements` convention is **opt-in and incremental**. Specs and crates without
  declarations simply appear in the orphan report. There is no enforcement gate in MVP.
- The markdown output is a **derived view**, not a source of truth. `index.json` is
  canonical. The markdown is for human consumption and PR review diffs.

## 7. Cross-references

- Spec 118 (`workflow-spec-traceability`) added `workflowTraceability` (Layer 5)
  and bumped `schemaVersion` to `1.1.0`.
- Spec 129 (`granular-package-oap-metadata`) bumps `schemaVersion` to `1.2.0`,
  extends `TraceSource` with `cargo-metadata-crate` (renamed from
  `cargo-metadata`), `cargo-metadata-module` (reserved), `comment-header`
  (new), and `multiple` (replaces `both`); adds the `comment_scanner`
  module and merges file-level claims via xref. The mechanism is additive;
  the index's existing layers are unchanged.
