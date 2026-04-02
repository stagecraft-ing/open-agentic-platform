# Xray Evolution Plan

## Status: Draft
## Author: Bartek Kus
## Date: 2026-04-02

---

## Executive Summary

Xray is a deterministic repository scanner producing cryptographically-signed JSON indexes. Its architecture — canonical serialization, per-file hashing, invariant validation — is overbuilt for "list files and count lines." It is the foundation for a **verifiable context layer** that AI agents can trust.

This plan addresses xray's 7 latent potential vectors, architectural hardening, and the absorption of blockoli + stackwalk into a unified `xray` crate that becomes the single repo-truth oracle.

---

## Part 1: Absorption Analysis — blockoli & stackwalk

### Evidence for Absorption

| Signal | xray | stackwalk | blockoli | Verdict |
|--------|------|-----------|----------|---------|
| **Directory walking** | walkdir (deterministic) | jwalk (parallel) | delegates to stackwalk | Duplicate — xray's walker is superior (deterministic, ignore-aware) |
| **File discovery** | Full inventory with hash/size/loc | Extension filter (4 langs) | None (delegates) | xray already does this better |
| **Language detection** | 26+ extensions, policy-driven | 4 extensions (phf_map) | None | xray subsumes stackwalk's detection |
| **Per-file analysis** | LOC + hash + size | AST parsing + call graph | Vector embeddings | Complementary layers, not conflicting |
| **Output format** | Canonical JSON with digest | JSON + DOT + Mermaid | SQLite + REST | xray's format is the anchor |
| **Quality** | 17 tests, invariant validation, atomic writes | 0 tests, debug printlns, panics, hardcoded paths | 0 unit tests, unused functions | xray is production-grade; others need rescue |
| **Integration** | MCP via axiomregent | Only consumed by blockoli + desktop | Desktop app Tauri commands | All converge at axiomregent/desktop |
| **Determinism** | Core design principle | Not considered | Not considered | xray's contract must govern |

### The Case

**stackwalk** provides AST parsing (tree-sitter) and call graph generation — capabilities xray lacks. But stackwalk has zero tests, hardcoded debug paths (`test-code-base/`), debug printlns in production, `panic!` on unsupported languages, and broken cross-module call resolution. It's a liability as a standalone crate.

**blockoli** provides vector embeddings (fastembed) and semantic search (KD-tree + SQLite). It's a consumer of stackwalk that adds ML-powered search. But it duplicates directory walking, has no tests, carries dead code (`_generate_embeddings`, `_search_embeddings`), and its REST API duplicates what axiomregent's MCP router already provides.

**The absorption thesis**: xray already walks every file, detects every language, and produces a cryptographically-signed inventory. stackwalk's AST parsing and blockoli's embeddings are *analysis layers that operate on files xray has already discovered*. Rather than three crates independently walking the same directory, xray should be the single scanner that feeds downstream analyzers.

### Absorption Strategy: Federation, Not Monolith

Do NOT merge 3,000 lines of AST parsing and ML embeddings into xray's core. Instead:

```
xray (orchestrator + file inventory)
 ├── xray::analysis::structure  (absorbs stackwalk's AST parsing, cleaned up)
 ├── xray::analysis::embeddings (absorbs blockoli's vector generation)
 └── xray::analysis::complexity (new — uses structure for complexity scoring)
```

xray remains the scanner. Analysis modules are **feature-gated** so the core scan pipeline stays lean. The `XrayIndex` gains new optional fields populated by analysis passes.

---

## Part 2: Implementation Phases

### Phase 0 — Architectural Hardening (prerequisite)

**Goal**: Fix the foundation before building on it.

#### 0.1 — stackwalk Production Cleanup
Before absorption, stackwalk needs decontamination:

- [ ] Remove all `println!` debug statements (parser.rs:435-442, 481-485, 524-529)
- [ ] Remove hardcoded `test-code-base/` path (parser.rs:264, 270) — use actual module paths
- [ ] Remove duplicate match arm (parser.rs:600-603)
- [ ] Replace `panic!("Unsupported language")` with `Result::Err` (parser.rs:91)
- [ ] Replace `fs::read_to_string().unwrap()` with `?` propagation (parser.rs:30)
- [ ] Add basic test coverage: parse a known Rust file, verify block count and call graph
- [ ] Remove dead code from blockoli: `_generate_embeddings()`, `_search_embeddings()`

#### 0.2 — xray Architectural Fixes
From the architectural observations:

- [ ] **Schema version guard**: `scan_target()` should set and validate schema version; reject unknown versions on deserialization
- [ ] **`.xrayignore` support**: Read optional `.xrayignore` file (gitignore syntax) and merge with `IGNORED_DIRS`. Determinism preserved because the ignore file itself is hashed into the digest
- [ ] **`spawn_blocking` wrapper**: Add `pub async fn scan_target_async()` that wraps the sync scan in `tokio::task::spawn_blocking` for MCP router integration
- [ ] **`Commands::All` implementation**: Wire up scan → docs pipeline in main.rs

---

### Phase 1 — Complexity Scoring (Vector 1)

**Goal**: Fill the `complexity: 0` placeholder with real values.

#### Schema Change (v1.1.0)
```rust
pub struct FileNode {
    // ... existing fields ...
    pub complexity: u64,        // was: always 0
    pub functions: Option<u32>, // NEW: function count (None if unparsable)
    pub max_depth: Option<u32>, // NEW: max nesting depth (None if unparsable)
}
```

#### Implementation
- [ ] Create `xray::analysis::structure` module (feature-gated: `analysis-structure`)
- [ ] Move cleaned-up stackwalk parser into `xray::analysis::structure::parser`
- [ ] Keep tree-sitter FFI bindings and `build.rs` grammar compilation
- [ ] Implement `analyze_file(path: &Path) -> Option<StructureMetrics>`:
  - Parse with tree-sitter
  - Count function definitions → `functions`
  - Measure max AST nesting depth → `max_depth`
  - Compute complexity = `functions * avg_depth` (simple heuristic, refineable later)
- [ ] Integrate into `traversal.rs`: after LOC computation, if `analysis-structure` feature enabled, run `analyze_file()` and populate `complexity`, `functions`, `max_depth`
- [ ] Update `validate_invariants()` to check complexity consistency when `functions` is `Some`
- [ ] Add golden test: scan fixture repo, verify non-zero complexity for `.rs` and `.py` files

**Why functions + max_depth instead of cyclomatic complexity**: Cyclomatic complexity requires full control-flow graph analysis. Function count + nesting depth is computable from the AST in a single pass and correlates strongly with maintainability. It's the 80/20.

---

### Phase 2 — Incremental Scanning (Vector 2)

**Goal**: Compare previous index hashes against current files, re-hash only changes.

#### Schema Addition
```rust
pub struct XrayIndex {
    // ... existing fields ...
    pub prev_digest: Option<String>, // NEW: digest of previous index (None if first scan)
    pub changed_files: Option<Vec<String>>, // NEW: paths that changed since prev_digest
}
```

#### Implementation
- [ ] Add `--previous <path>` CLI flag to `scan` command
- [ ] In `scan_target()`, if previous index provided:
  1. Deserialize previous `index.json`
  2. Build `HashMap<path, hash>` from previous files
  3. During walk, check if file exists in previous map AND current hash matches → skip re-analysis (reuse FileNode)
  4. Track which files are new/changed/deleted → populate `changed_files`
  5. Set `prev_digest` to previous index's digest
- [ ] Determinism contract: incremental scan of unchanged repo MUST produce identical digest to full scan
- [ ] Add test: scan fixture, modify one file, incremental scan, verify only that file re-hashed

**Performance target**: For a 10K-file repo where 5 files changed, incremental scan should complete in <500ms (vs ~5s full scan).

---

### Phase 3 — Temporal Intelligence (Vector 3)

**Goal**: Track index evolution over time for churn detection.

#### New Module: `xray::history`
```rust
pub struct IndexHistory {
    pub entries: Vec<HistoryEntry>,
}

pub struct HistoryEntry {
    pub digest: String,
    pub timestamp: String, // ISO 8601
    pub file_count: usize,
    pub changed_count: usize,
    pub changed_files: Vec<String>,
}

impl IndexHistory {
    pub fn append(&mut self, index: &XrayIndex) -> Result<()>;
    pub fn churn_report(&self, top_n: usize) -> Vec<(String, usize)>; // path → change count
    pub fn growth_report(&self) -> GrowthStats; // files/size over time
}
```

#### Implementation
- [ ] Store history in `.axiomregent/data/history.jsonl` (append-only, one JSON object per line)
- [ ] Each scan appends a `HistoryEntry` with digest + changed files from Phase 2
- [ ] `churn_report()` counts how many times each file appears in `changed_files` across entries
- [ ] Add CLI command: `xray history --top 20` — shows highest-churn files
- [ ] Add MCP tool: `xray.churn` — returns churn report as JSON for agent consumption
- [ ] **Determinism note**: history is append-only, never modifies the index itself. The index digest remains pure.

**Agent value**: "These 10 files change in every commit — they're the hot zone. Read them first."

---

### Phase 4 — Call Graph Integration (Vector 1 + stackwalk absorption)

**Goal**: Absorb stackwalk's call graph into xray's analysis layer.

#### New Module: `xray::analysis::call_graph`
Migrated from stackwalk, with fixes:

- [ ] Move `call_stack.rs`, `call_graph.rs`, `block.rs` into `xray::analysis::call_graph`
- [ ] Refactor `parser.rs` into `xray::analysis::structure::parser` (already done in Phase 1)
- [ ] Feature-gate under `analysis-call-graph` (implies `analysis-structure`)
- [ ] Add to `XrayIndex`:
  ```rust
  pub call_graph: Option<CallGraphSummary>, // NEW
  ```
  ```rust
  pub struct CallGraphSummary {
      pub total_functions: usize,
      pub total_edges: usize,
      pub entry_points: Vec<String>,
      pub max_fan_out: (String, usize), // function with most outgoing calls
      pub max_fan_in: (String, usize),  // function with most incoming calls
  }
  ```
- [ ] Full call graph data written to separate file: `.axiomregent/data/call-graph.json` (too large for index)
- [ ] Export formats preserved: DOT, Mermaid, JSON flowchart — via `xray docs --call-graph`
- [ ] Entry point detection feeds into complexity scoring: entry points with high fan-out get higher complexity

**Why summary in index, detail in separate file**: The index must stay compact for MCP tool responses. The full call graph (every edge) can be megabytes. Summary stats in the index, detail on disk.

---

### Phase 5 — Dependency Graph Extraction (Vector 4)

**Goal**: Parse module files to build lightweight SBOM.

#### New Module: `xray::analysis::deps`
```rust
pub struct DependencyInventory {
    pub ecosystems: BTreeMap<String, Vec<Dependency>>, // "cargo" → [...], "npm" → [...]
    pub total_direct: usize,
    pub total_dev: usize,
}

pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub dev_only: bool,
    pub source_file: String, // which module file declared it
}
```

#### Implementation
- [ ] Feature-gate: `analysis-deps`
- [ ] Parse `Cargo.toml` → extract `[dependencies]` and `[dev-dependencies]`
- [ ] Parse `package.json` → extract `dependencies` and `devDependencies`
- [ ] Parse `go.mod` → extract `require` block
- [ ] Use xray's existing `module_files` to know which files to parse (no extra traversal)
- [ ] Add to `XrayIndex`: `pub dependencies: Option<DependencyInventory>`
- [ ] Add MCP tool: `xray.deps` — returns dependency inventory
- [ ] **No full resolution**: This is NOT a lockfile parser. It reads declared dependencies only. Good enough for agent context ("this project uses React, Tokio, and gRPC").

---

### Phase 6 — Semantic Embeddings (Vector 7 + blockoli absorption)

**Goal**: Absorb blockoli's vector embedding capability as an xray analysis pass.

#### New Module: `xray::analysis::embeddings`
Feature-gated: `analysis-embeddings` (heavy dependency: fastembed ~30MB model)

- [ ] Move fastembed integration from blockoli into `xray::analysis::embeddings::encoder`
- [ ] Move KD-tree search into `xray::analysis::embeddings::search`
- [ ] **Drop blockoli's SQLite store** — replace with xray's file-based approach:
  - Embeddings stored in `.axiomregent/data/embeddings.bin` (binary format, not JSON — vectors are 384 floats per block)
  - Index file: `.axiomregent/data/embeddings-index.json` maps block IDs to file paths + function names
- [ ] **Drop blockoli's REST API** — axiomregent's MCP router is the integration point
- [ ] Add MCP tools:
  - `xray.search` — semantic code search (replaces blockoli's standalone search)
  - `xray.index-embeddings` — generate embeddings for a project
- [ ] **Incremental embedding**: Use Phase 2's `changed_files` to only re-embed changed files

**Why drop SQLite**: blockoli used SQLite for project isolation and persistence. But xray already has project-scoped output (`.axiomregent/data/`), atomic writes, and file-based determinism. Adding SQLite as a dependency contradicts xray's design. Binary files + JSON index achieve the same persistence without the runtime dependency.

**Why drop REST API**: blockoli's actix-web server duplicates axiomregent's MCP router. One integration surface, not two.

---

### Phase 7 — Context Budget Optimizer (Vector 7)

**Goal**: Given a task description and token budget, recommend which files to read.

#### New Module: `xray::context`
```rust
pub struct ContextBudget {
    pub max_tokens: usize, // approximate, using 1 LOC ≈ 10 tokens heuristic
    pub task: String,       // natural language task description
}

pub struct ContextPlan {
    pub files: Vec<ContextFile>,
    pub total_loc: u64,
    pub estimated_tokens: usize,
    pub coverage: f64, // fraction of relevant files included
}

pub struct ContextFile {
    pub path: String,
    pub loc: u64,
    pub relevance: f64,  // 0.0–1.0
    pub reason: String,  // why this file was selected
}
```

#### Implementation
- [ ] **Without embeddings** (no `analysis-embeddings` feature): Rank by heuristics:
  - Files matching task keywords in path → high relevance
  - Entry point files (from call graph) → medium relevance
  - High-churn files (from history) → medium relevance
  - Recently changed files → boost
  - Sort by relevance, greedily fill budget by LOC
- [ ] **With embeddings** (feature enabled): Use semantic search:
  - Embed task description
  - Find nearest code blocks
  - Map blocks back to files
  - Merge with heuristic ranking
- [ ] Add MCP tool: `xray.context` — takes task + budget, returns file list
- [ ] **This is the capstone**: it combines index (files, LOC), call graph (entry points), history (churn), and optionally embeddings into a single "what should the agent read?" answer.

---

### Phase 8 — Policy Engine (Vector 6)

**Goal**: Parameterize `validate_invariants()` into a configurable policy engine.

#### New Module: `xray::policy`
```rust
pub struct PolicySet {
    pub rules: Vec<PolicyRule>,
}

pub enum PolicyRule {
    MaxFileSize(u64),                    // no file larger than N bytes
    MaxFileLoc(u64),                     // no file with more than N LOC
    MaxComplexity(u64),                  // no file with complexity > N
    RequireLanguage(String),             // at least one file of this language
    ForbidLanguage(String),              // no files of this language in scan
    MaxUnknownRatio(f64),               // fraction of "Unknown" lang files
    DependencyDenyList(Vec<String>),     // forbidden dependencies
    Custom(String, Box<dyn Fn(&XrayIndex) -> Result<()>>), // extensible
}

pub struct PolicyReport {
    pub passed: Vec<String>,   // rule descriptions that passed
    pub violated: Vec<PolicyViolation>,
}

pub struct PolicyViolation {
    pub rule: String,
    pub message: String,
    pub files: Vec<String>, // offending files
    pub severity: Severity,
}
```

#### Implementation
- [ ] Load policy from `.xray-policy.toml` at repo root
- [ ] Run policy evaluation after scan + analysis passes
- [ ] Add CLI command: `xray policy` — evaluates and reports
- [ ] Add MCP tool: `xray.policy` — returns violations as JSON
- [ ] **Non-blocking by default**: Policy violations are warnings, not scan failures. CI can choose to fail on violations.
- [ ] Connect to existing `validate_invariants()` — structural invariants are non-negotiable; policy rules are configurable overlays.

---

### Phase 9 — Structural Fingerprinting (Vector 5)

**Goal**: Classify repos by structure for cross-repo comparison.

#### Addition to `XrayIndex`
```rust
pub struct XrayIndex {
    // ... existing fields ...
    pub fingerprint: Option<String>, // NEW: structural fingerprint hash
}
```

#### Implementation
- [ ] Compute fingerprint from: sorted language ratios + top_dir structure + dependency ecosystems + file count bucket (small/medium/large)
- [ ] Fingerprint is a short hash (e.g., first 8 chars of SHA-256 of the above)
- [ ] Two repos with the same fingerprint have similar structure (not identical content)
- [ ] Add MCP tool: `xray.fingerprint` — returns fingerprint + classification label
- [ ] Classification labels: "rust-cli", "node-webapp", "go-microservice", "monorepo", "terraform-infra", etc. — heuristic mapping from language + structure signals

---

## Part 3: Dependency Graph (Build Order)

```
Phase 0: Hardening (prerequisite for all)
    │
    ├── Phase 1: Complexity Scoring (stackwalk parser absorbed)
    │       │
    │       ├── Phase 4: Call Graph (builds on structure analysis)
    │       │       │
    │       │       └── Phase 7: Context Budget (uses call graph entry points)
    │       │
    │       └── Phase 8: Policy Engine (uses complexity scores)
    │
    ├── Phase 2: Incremental Scanning (independent)
    │       │
    │       └── Phase 3: Temporal Intelligence (depends on changed_files from Phase 2)
    │               │
    │               └── Phase 7: Context Budget (uses churn data)
    │
    ├── Phase 5: Dependency Extraction (independent, uses module_files)
    │       │
    │       └── Phase 8: Policy Engine (uses dependency data for deny lists)
    │
    ├── Phase 6: Semantic Embeddings (blockoli absorbed, independent)
    │       │
    │       └── Phase 7: Context Budget (uses embeddings for ranking)
    │
    └── Phase 9: Fingerprinting (uses languages + deps + structure)
```

**Parallelizable work streams**:
- Stream A: Phase 0 → 1 → 4 → 7
- Stream B: Phase 0 → 2 → 3
- Stream C: Phase 0 → 5 → 9
- Stream D: Phase 0 → 6

Phase 7 (Context Budget) and Phase 8 (Policy) are convergence points that integrate outputs from multiple streams.

---

## Part 4: Schema Evolution

### v1.0.0 (current)
```
XrayIndex { schema_version, root, target, files[], languages{}, top_dirs{}, module_files[], stats, digest }
FileNode  { path, size, hash, lang, loc, complexity }
```

### v1.1.0 (Phase 1)
```
FileNode  += { functions?, max_depth? }
```

### v1.2.0 (Phase 2-3)
```
XrayIndex += { prev_digest?, changed_files? }
```

### v2.0.0 (Phase 4-9, breaking: optional sections become first-class)
```
XrayIndex += { call_graph?, dependencies?, fingerprint? }
```

Each version bump preserves backward compatibility within minor versions. v2.0.0 is breaking because it changes the digest computation to include new optional fields.

---

## Part 5: Feature Flag Matrix

| Feature Flag | Adds | Dependencies Added | Binary Size Impact |
|---|---|---|---|
| (default) | File scan, LOC, hash, digest | walkdir, sha2, serde_json | ~2MB |
| `analysis-structure` | AST parsing, complexity, function count | tree-sitter, cc (build) | +5MB (grammars) |
| `analysis-call-graph` | Call graph, entry points | (implies analysis-structure) | +0.5MB |
| `analysis-deps` | Dependency extraction | toml (already dep) | +0.1MB |
| `analysis-embeddings` | Vector embeddings, semantic search | fastembed, kd-tree | +30MB (model) |
| `policy` | Policy evaluation | (no new deps) | +0.1MB |
| `history` | Temporal tracking | (no new deps) | +0.1MB |
| `context` | Context budget optimizer | (no new deps, uses other features) | +0.1MB |
| `full` | All of the above | All | ~38MB |

**Default compilation** (`cargo build -p xray`) stays lean. Desktop app and axiomregent opt into the features they need.

---

## Part 6: Crate Consolidation Plan

### Before (3 crates)
```
crates/stackwalk/  — 1,195 LOC, 0 tests, 6 bugs, MIT license
crates/blockoli/   — 1,300 LOC, 0 tests, dead code, MIT + AGPL conflict
crates/xray/       — 1,600 LOC, 17 tests, production-grade
```

### After (1 crate, feature-gated)
```
crates/xray/
  src/
    lib.rs                  — orchestrator
    schema.rs               — XrayIndex, FileNode (extended)
    traversal.rs            — file discovery
    hash.rs, digest.rs      — cryptographic integrity
    canonical.rs            — deterministic serialization
    language.rs, loc.rs     — basic file analysis
    write.rs, docs.rs       — output generation
    tools.rs                — MCP integration (extended)
    policy.rs               — policy engine (new)
    history.rs              — temporal tracking (new)
    context.rs              — context budget optimizer (new)
    analysis/
      mod.rs
      structure/
        mod.rs              — from stackwalk: parser, block extraction
        parser.rs           — tree-sitter AST parsing (cleaned up)
        block.rs            — Block, BlockType
      call_graph/
        mod.rs              — from stackwalk: call graph + call stack
        graph.rs            — CallGraph with DOT/Mermaid/JSON export
        stack.rs            — CallStack tree
      deps/
        mod.rs              — dependency extraction
        cargo.rs            — Cargo.toml parser
        npm.rs              — package.json parser
        gomod.rs            — go.mod parser
      embeddings/
        mod.rs              — from blockoli: vector generation + search
        encoder.rs          — fastembed wrapper
        search.rs           — KD-tree similarity search
        store.rs            — binary file storage (replaces SQLite)
      complexity.rs         — complexity scoring (uses structure)
  tests/
    golden_scan.rs          — existing
    index_format_test.rs    — existing
    invariant_tests.rs      — existing
    structure_tests.rs      — new: AST parsing tests
    call_graph_tests.rs     — new: call graph tests
    incremental_tests.rs    — new: incremental scan tests
```

### Migration Path
1. Phase 0: Fix stackwalk bugs in-place
2. Phase 1: Copy cleaned stackwalk modules into `xray::analysis::structure`, feature-gate
3. Phase 4: Copy call graph modules into `xray::analysis::call_graph`
4. Phase 6: Copy cleaned blockoli modules into `xray::analysis::embeddings`
5. Update axiomregent imports: `stackwalk::*` → `xray::analysis::structure::*`
6. Update desktop app imports: `blockoli::*` → `xray::analysis::embeddings::*`
7. Deprecate standalone `crates/stackwalk/` and `crates/blockoli/` (keep as empty re-export crates for one release, then remove)

---

## Part 7: MCP Tool Surface (Final State)

| Tool | Phase | Description |
|---|---|---|
| `xray.scan` | existing | Full repo scan → XrayIndex |
| `xray.scan-incremental` | 2 | Delta scan against previous index |
| `xray.churn` | 3 | Top-N highest-churn files |
| `xray.call-graph` | 4 | Call graph summary + entry points |
| `xray.deps` | 5 | Dependency inventory |
| `xray.search` | 6 | Semantic code search |
| `xray.context` | 7 | Context budget optimization |
| `xray.policy` | 8 | Policy evaluation report |
| `xray.fingerprint` | 9 | Structural fingerprint + classification |

Each tool returns JSON. Each tool is independently callable. The `xray.context` tool is the crown jewel — it orchestrates scan + call graph + churn + embeddings into a single "here's what to read" answer.

---

## Risk Register

| Risk | Mitigation |
|---|---|
| fastembed model download (30MB) on first use | Feature-gated; model path configurable; pre-download in CI |
| tree-sitter grammar compilation in `build.rs` | Feature-gated; grammars only compiled when `analysis-structure` enabled |
| Schema v2.0.0 breaking change | Staged: v1.1 and v1.2 are additive. v2.0 only when all optional fields stabilize |
| Determinism broken by analysis features | Analysis results are optional fields. Core digest computed from v1.0 fields only. Analysis fields have separate validation |
| License conflict (blockoli MIT vs xray AGPL) | AGPL is more restrictive; MIT code can be relicensed under AGPL. No conflict in this direction |
| Performance regression from analysis passes | Feature-gated. Default scan path unchanged. Analysis passes run after core scan, can be parallelized with rayon |
