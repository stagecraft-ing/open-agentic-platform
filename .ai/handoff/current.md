> **Non-authoritative.** This file coordinates temporary agent handoff only. Canonical feature and execution truth live under `specs/...` and in code — not here.

## Branch

`main`

## Objective

**Stagecraft-ing extraction complete.** 17 projects from `~/Dev2/stagecraft-ing/` analyzed exhaustively and all valuable content integrated into OAP or captured as draft specs. The platform now has a full governance foundation (CLAUDE.md, AGENTS.md, orchestrator rules), 9 slash commands, 4 agent definitions, 3 code modules, ast-grep enforcement rules, a devcontainer, and 22 new spec outlines (042–063) covering the next generation of platform capabilities.

## Agent pack

**Cursor** (implementation), **Claude** (review / deep analysis), **Antigravity** (wide exploration), **Claude Opus** (`claude-opus` — synthesis and next-slice prioritization). Details: `.ai/README.md`, `.ai/prompts/baton-rules.md`, role cards under `.ai/handoff/`.

## Lifecycle note

Registry **`status`** in frontmatter must be one of **`draft` | `active` | `superseded` | `retired`** (Feature **000** / **003**). Delivery completion is proven by checked tasks + verification artifacts, not by status changes.

## Canonical feature authority

- **032–041:** All `status: active`, all complete, all synthesized.
- **042–063:** All `status: draft`, newly created from stagecraft-ing extraction. See below for details.

## What was delivered in this session

### Phase 1: Extraction (17 projects → 18 reports)

All projects in `~/Dev2/stagecraft-ing/` were analyzed file-by-file. Extraction reports live in `docs/extractions/`. Master consolidation: `docs/extractions/CONSOLIDATION.md` (189 raw items deduplicated to 62: 14 P0, 28 P1, 20 P2).

**Source projects (all confirmed safe to delete):** agents, asterisk-mcp-server, claude-code, claude-code-by-agents, claude-code-single-binary, claude-code-sub-agents, claudecodeui, claudepal, crystal, deepreasoning, developer-claude-code-commands, equilateral-agents-open-core, gitctx (already byte-identical to OAP), product-manager-claude-code-commands, research (empty), ruflo, skills.

### Phase 2: Integration (65 new files)

**Governance foundation:**
- `CLAUDE.md` — project-level Claude Code configuration with architecture overview, conventions
- `AGENTS.md` — self-extending agent protocol with init checklist, available agents/commands
- `.claude/rules/orchestrator-rules.md` — 6-rule governance preamble for multi-step workflows

**Slash commands (`.claude/commands/`):**
| Command | What it does | Source projects |
|---------|-------------|-----------------|
| `code-review.md` | Parallel 6-agent review with adaptive selection | developer-cc-commands, skills |
| `validate-and-fix.md` | Phased quality-gate pipeline with rollback | developer-cc-commands, claudepal |
| `init.md` | Session init protocol (self-extending via AGENTS.md) | claudepal |
| `commit.md` | Governed conventional commits, impact-focused | skills, developer-cc-commands |
| `cleanup.md` | Delegated dead-code/duplicate analyzer | claudepal |
| `review-branch.md` | Exhaustive read-only branch review (10 cross-platform categories) | developer-cc-commands |
| `refactor-claude-md.md` | CLAUDE.md modularization with path-scoped rules | skills |
| `implement-plan.md` | Plan file executor with status state machine | skills |
| `research.md` | Parallel sub-agent research with query classification | skills, product-manager-cc-commands |

**Agent definitions (`.claude/agents/`):**
| Agent | Role | Model |
|-------|------|-------|
| `architect.md` | Read-only planning, task decomposition | sonnet |
| `explorer.md` | Codebase analysis, context gathering | sonnet |
| `implementer.md` | Focused code changes from plans | sonnet |
| `reviewer.md` | Post-change review (bugs, security, perf) | sonnet |

**Code modules (`apps/desktop/src/lib/`):**
- `censor.ts` — Secret censoring (Anthropic keys, OpenAI, GitHub, AWS, PEM, etc.)
- `shellPath.ts` — Cross-platform PATH resolution for Tauri (with Rust backend invoke fallback)
- `shellEscape.ts` — Safe shell argument escaping and git commit building

**ast-grep rules (`apps/desktop/.ast-grep/`):**
- `zustand/no-destructure.yml` — prevents render cascades
- `architecture/hooks-in-hooks-dir.yml` — enforces hooks boundary
- `architecture/no-store-in-lib.yml` — forces getState() in lib/

**DevContainer (`.devcontainer/`):**
- `Dockerfile` — Node 20 + Rust + pnpm + git-delta + Claude Code
- `devcontainer.json` — VS Code config with rust-analyzer
- `init-firewall.sh` — iptables sandbox (GitHub, npm, crates.io, Anthropic only)

### New specs (042–063)

| Spec | Title | Priority | Action |
|------|-------|----------|--------|
| 042 | Multi-Provider Agent Registry | P0 | outline-spec |
| 043 | Agent Organizer / Meta-Orchestrator | P0 | outline-spec |
| 044 | Multi-Agent Orchestration (file-based artifacts) | P0 | outline-spec |
| 045 | Claude Code SDK Bridge | P0 | outline-spec |
| 046 | Context Compaction | P0 | outline-spec |
| 047 | Governance Control Plane (policy compiler) | P0 | outline-spec |
| 048 | Hookify Rule Engine | P1 | outline-spec |
| 049 | Permission System | P1 | outline-spec |
| 050 | Tool Renderer System | P1 | outline-spec |
| 051 | Worktree Agents | P1 | outline-spec |
| 052 | State Persistence | P1 | outline-spec |
| 053 | Verification Profiles | P1 | outline-spec |
| 054 | Agent Frontmatter Schema | P1 | outline-spec |
| 055 | YAML Standards Schema | P1 | outline-spec |
| 056 | Session Memory | P1 | outline-spec |
| 057 | Notification System | P1 | outline-spec |
| 058 | File Mention System | P1 | outline-spec |
| 059 | Git Panel | P1 | outline-spec |
| 060 | Panel Event Bus | P1 | outline-spec |
| 061 | Conductor Track Lifecycle | P1 | outline-spec |
| 062 | Multi-Model Chaining | P1 | outline-spec |
| 063 | Coherence Scoring | P1 | outline-spec |

## Baton

- Current owner: **claude** (047 Phase 2 review complete — approved)
- Next owner: **cursor** — implement Phase 3 (WASM policy kernel + gate enforcement: FR-006, FR-007, SC-003, SC-004, SC-005, SC-006).
- Last baton update: 2026-03-30 — **claude**: Phase 2 approved. FR-003 classification correct (constitution = `scope=global` + `mode=enforce`, shards = all other grouped by scope). FR-004 bundle emission structurally complete (constitution, shards, metadata with hash/timestamp/sources). FR-005 determinism satisfied (canonical JSON hash excluding `compiledAt`/`policyBundleHash`). SC-001/SC-002 directly tested. P1-001 resolved (WalkDir `filter_entry`). 6 findings: P2-001 MEDIUM (validation in hash payload — fragile if warning severities added), P2-002 LOW (compiledAt absent on failure), P2-003 LOW (no multi-scope shard test), P2-004–P2-006 INFO. 7/7 tests pass.
- Recommended files to read:
  - `.ai/findings/047-phase2-review.md` — Phase 2 review with findings
  - `.ai/plans/047-governance-control-plane-phased-plan.md` — Phase 3 spec
  - `specs/047-governance-control-plane/spec.md` — FR-006, FR-007, SC-003..SC-006
  - `tools/policy-compiler/src/lib.rs` — current implementation base

## Requested next agent output

**cursor**: Phase 3 — implement WASM policy kernel + gate enforcement (FR-006, FR-007, SC-003, SC-004, SC-005, SC-006). Consider P2-001 (exclude `validation` from hash payload) before proof chains in Phase 5.

Priority order for P0 specs (unchanged):

1. **045 — Claude Code SDK Bridge** — ✅ `status: active` — end-to-end complete
2. **042 — Multi-Provider Agent Registry** — ✅ complete (all phases + Phase 6 review)
3. **044 — Multi-Agent Orchestration** — Phase 1–5 ✅ (manifest, DAG, prompts, async dispatch, real executor); e2e validation follow
4. **046 — Context Compaction**
5. **047 — Governance Control Plane**

After each slice, **claude** reviews against `spec.md`.

## P2 items captured as ideas only (not yet specs)

20 P2 items from the consolidation (plugin marketplace, three-tier model strategy, encrypted keychain, WebSocket reconnection, i18n, quick pane, NDJSON streaming, Bun compilation, pain-to-pattern methodology, agent communication bus, FI/FDI guardrails, SQLite schema, AI session naming, security scanning MCP, markdown tool responses, VS Code extension, slash command fuzzy search, design tokens, scheduling system). See `docs/extractions/CONSOLIDATION.md` § P2 for details.

## Promotion candidates for canonical artifacts

- All 22 new specs already live in `specs/` (canonical)
- All commands/agents/rules already live in `.claude/` (canonical)
- Code modules already live in `apps/desktop/src/lib/` (canonical)
- Extraction reports in `docs/extractions/` are reference material (non-canonical, deletable after source projects are removed)

---

## Recent outputs

- 2026-03-30 (claude): **047 Phase 2 review** — Phase 2 approved. FR-003 classification (constitution = global+enforce, shards = all else by scope), FR-004 bundle emission (5 sections), FR-005 determinism (canonical JSON SHA-256 excluding compiledAt/policyBundleHash) — all spec-faithful. SC-001/SC-002 directly tested. P1-001 resolved. 6 findings: P2-001 MEDIUM (validation in hash payload), P2-002/P2-003 LOW, P2-004–P2-006 INFO. 7/7 tests pass. Review: `.ai/findings/047-phase2-review.md`.
- 2026-03-30 (cursor): **047 Phase 2** — P1-001 WalkDir `filter_entry` prune; constitution/shard classification; deterministic `policy-bundle.json` + `policyBundleHash` (payload excludes timestamp/hash fields); SC-001/SC-002 tests. Output: `build/policy-bundles/policy-bundle.json`
- 2026-03-30 (claude): **047 Phase 1 review** — Phase 1 approved. FR-001 discovery precedence correct (3-tier with exclusions). FR-002 fenced `policy` block parser faithful (YAML → PolicyRule with id/description/mode/scope/gate). FR-011 V-101..V-106 all implemented. F-005 resolved (shared frontmatter crate, 3 tools wired). F-006 resolved (fenced `policy` syntax committed). G-006 resolved (dual-target native+WASM strategy). 7 findings: P1-001 MEDIUM (WalkDir skip doesn't prevent descent — use `filter_entry`), P1-002 LOW (dead replacement branch), P1-003 LOW (V-103 message misleading for same-precedence), P1-004/P1-005 LOW (missing V-101/V-102 tests), P1-006/P1-007 INFO. 4/4 tests pass. Review: `.ai/findings/047-phase1-review.md`.
- 2026-03-30 (cursor): **047 Phase 1 + decision resolution** — Implemented `tools/policy-compiler/` scaffold with `compile`/`validate` commands and Phase 1 outputs: policy source discovery precedence (`CLAUDE.md` root > `.claude/policies/*.md` > subdirectory `CLAUDE.md`), fenced `policy` block parsing, structured rule extraction (`id`, `description`, `mode`, `scope`, optional `gate`), and V-series validations (`V-101`..`V-106`, including duplicate IDs as `V-103`). Added tests for discovery precedence, valid/invalid parsing, and duplicate resolution. Resolved F-005 by introducing shared crate `tools/shared/frontmatter/` and reusing it in `tools/spec-compiler` and `tools/spec-lint`. Resolved F-006 by committing to fenced `policy` blocks in plan G-002. Added G-006 dual-target native+WASM host-runtime strategy in `.ai/plans/047-governance-control-plane-phased-plan.md`.
- 2026-03-30 (claude): **047 plan review** — All 26 requirements (11 FR + 4 NF + 11 SC) covered across 6 phases. Phase ordering sound. 7 findings: F-001 HIGH (WASM host runtime unspecified — wasmtime vs dual-target native+WASM must be decided before Phase 3), F-003 MEDIUM (constitution append-only session invariant from contract notes missing from plan), F-004 MEDIUM (.claude/governance.toml config for coherence thresholds not in Phase 4), F-005 MEDIUM (no shared frontmatter parser crate exists — split_frontmatter duplicated in spec-compiler and spec-lint), F-006 LOW (rule annotation syntax not committed — recommend fenced `policy` blocks), F-007 LOW (bundle format JSON vs msgpack undecided), F-008 LOW (policyBundleHash field in registry.json not tracked). Plan approved for Phase 1 start. Review: `.ai/findings/047-plan-review.md`.
- 2026-03-30 (cursor): **047 planning pass** — Responded to baton by reading `specs/047-governance-control-plane/spec.md` and drafting `.ai/plans/047-governance-control-plane-phased-plan.md`. Plan defines G-001..G-005 pre-implementation decisions and six implementation phases: compiler discovery/parsing, deterministic bundle emission, WASM gate enforcement, coherence scheduler, proof-chain verification, and axiomregent integration with perf/verification evidence targets.
- 2026-03-30 (claude): **046 Phase 6 review** — Phase 6 approved. All 17 spec requirements (8 FR + 3 NF + 6 SC) satisfied. P5-001 resolved (recordIndex alignment in normalizeRuntimeMessages — regression test confirms). P5-002 resolved (fetchGitSnapshotFromRepo calls 4 Tauri git commands with graceful fallback — FR-006(b) interruption signal now live). P5-003 resolved (getContextWindowTokensForModel dispatches on model string — 1M for extended-context, 200k for standard Claude). NF-001 benchmark verified (400-message 100k-token-equiv compaction < 2s). SC-005 50-turn round-trip verified (completed steps, pending steps, file paths all preserved in session_context). git_last_commit Tauri command correct (git2 crate, returns OID + summary). 29/29 tests pass. Findings: P6-001 (no unit test for fetchGitSnapshotFromRepo, LOW), P6-002 (substring model matching, LOW), P6-003–P6-006 (INFO). **046 feature-complete.** Review: `.ai/findings/046-phase6-review.md`.
- 2026-03-30 (cursor): **046 Phase 6** — `git_last_commit` + `fetchGitSnapshotFromRepo` (Tauri git status / diff stats / branch / last commit), `getContextWindowTokensForModel`, P5-001 `recordIndex` alignment, NF-001 & SC-005 & P5-001 tests, `ClaudeCodeSession` async snapshot load. Validation: `pnpm vitest run src/lib/contextCompaction.test.ts` (29/29), `cargo check` (src-tauri).
- 2026-03-30 (claude): **046 Phase 5 review** — Phase 5 approved. All 7 deliverables implemented: `rewriteSessionHistoryForCompaction()` integrated into `ClaudeCodeSession.tsx`, FR-008 40% ceiling with cascading collapse (file summary counts → XML minification), FR-007 session init elevation of `<session_context>` after system prompt, interruption init hint ("Prioritize interruption resumption first"), P2-001 `resetTo()` on TokenBudgetMonitor, R-004 deterministic runtime ID synthesis via FNV-1a hash, P4-001 `<!-- pin -->` regex parsing. SC-001 validated (trigger at >75%, compacted output ≤40%). SC-002 validated (session restart produces system→hint→context ordering with structured sections). 24/24 tests pass. Findings: P5-001 (index mismatch in normalizeRuntimeMessages vs composeCompactedHistory for non-record entries, LOW), P5-002 (buildRuntimeGitSnapshot returns dummy data — disables FR-006(b) uncommitted changes interruption signal, MEDIUM), P5-003 (getContextWindowTokens hardcoded 200000, LOW), P5-004–P5-006 (INFO). No blockers for Phase 6. Review: `.ai/findings/046-phase5-review.md`.
- 2026-03-30 (cursor): **046 Phase 5** — Implemented message rewrite integration + session init hydration: `rewriteSessionHistoryForCompaction()` as the main entry point normalizing runtime messages, triggering compaction via TokenBudgetMonitor, composing compacted history with session_context block, enforcing 40% ceiling via `collapseFileModificationSection()` + `minifySessionContextXml()`. `elevateSessionContextForInit()` detects `<session_context>` blocks and reorders after system prompt. `addInterruptionInitHint()` inserts priority hint when interruption detected. `normalizeRuntimeMessages()` bridges runtime `{type, message}` format to CompactionMessage. `resolveRuntimeMessageId()` synthesizes deterministic FNV-1a IDs for messages without `id`. `resolvePinned()` scans `<!-- pin -->` annotations. `ClaudeCodeSession.tsx` wired: `loadSessionHistory` → `rewriteSessionHistoryForCompaction` → display. Validation: 24/24 tests passing.
- 2026-03-30 (claude): **046 Phase 4 review** — Phase 4 approved. P3-001 resolved (compacted_at parameterized via optional Date argument). P3-002 resolved (active tool preservation scoped to tool_use.id/tool_result.tool_use_id/tool_call_id match only). SC-004 satisfied (byte-identical preservation for system/pinned/recent turns including non-alternating patterns). SC-003 satisfied (2-signal threshold with true-positive multi-signal and false-positive single-signal fixtures). Non-alternating turn handling confirmed improved (user-turn counting replaces * 2 heuristic). 6 findings: P4-001 (<!-- pin --> annotation deferred to Phase 5, LOW), P4-002 (interruption type not asserted in test, LOW), P4-003 (only latest unresolved tool tracked, INFO), P4-004 (role guard on tool result matcher, INFO), P4-005 (P2-001 reset still open, INFO), P4-006 (R-004 message IDs still open, INFO). No blockers for Phase 5. Review: `.ai/findings/046-phase4-review.md`.
- 2026-03-30 (cursor): **046 Phase 4** — Implemented preserve-vs-compress + interruption heuristic refinements in `apps/desktop/src/lib/contextCompaction.ts`: (1) `compact(..., compactedAt = new Date())` now parameterizes `compacted_at` for production correctness and deterministic testing, (2) active tool preservation now scopes to active operation messages only (`tool_use.id`, `tool_result.tool_use_id`, `tool_call_id`) and no longer preserves all `role=tool` messages, (3) recent-turn preservation now tracks recent user turns for non-alternating histories, (4) interruption summary now requires 2+ active signals to reduce false positives. Expanded `apps/desktop/src/lib/contextCompaction.test.ts` with SC-004 preservation tests, active-tool scoping test, and SC-003 true-positive/false-positive interruption fixtures.
- 2026-03-30 (claude): **046 Phase 3 review** — Phase 3 approved. All 7 FR-004 sections present and deterministic. R-001 resolved (content block union). 6 findings: P3-001 (`compacted_at` epoch placeholder, MEDIUM), P3-002 (active tool preservation overly broad — preserves all `role=tool` messages instead of just active call, MEDIUM), P3-003 (`inferFileAction` per-message not per-mention, LOW), P3-004 (test coverage below plan — no golden snapshot or regex parse test, LOW), P3-005 (step extraction only on compacted messages, INFO), P3-006 (file path regex false positives, INFO). P2-001 reset strategy documented. R-004 message IDs still open. Review: `.ai/findings/046-phase3-review.md`.
- 2026-03-30 (cursor): **046 Phase 3** — Implemented `ProgrammaticCompactor` and deterministic `<session_context>` builder in `apps/desktop/src/lib/contextCompaction.ts` with structured extraction for task summary, completed/pending steps, file modification entries, git snapshot serialization, key decisions, and optional interruption summaries. Expanded `CompactionMessage.content` to `string | ContentBlock[]` for tool-use/result aware compaction. Added two Phase 3 tests in `apps/desktop/src/lib/contextCompaction.test.ts` for deterministic XML output and interruption omission path. Validation: `pnpm vitest src/lib/contextCompaction.test.ts` (17/17 passing).
- 2026-03-30 (claude): **046 Phase 2 review** — Phase 2 approved. FR-001 cumulative token accounting correct (+=, sanitized, >= threshold comparison). SC-006 verified for 0.5/0.95 overrides and boundary equality. Observability reason string deterministic. R-003 resolved. 3 findings: P2-001 (no reset mechanism for post-compaction, MEDIUM), P2-002 (contextWindowTokens per-call, INFO), P2-003 (getTotals untested directly, INFO). No Phase 3 blockers. Review: `.ai/findings/046-phase2-review.md`.
- 2026-03-30 (cursor): **046 Phase 2** — Implemented `TokenBudgetMonitor` in `apps/desktop/src/lib/contextCompaction.ts` with passive cumulative usage accounting (`reportUsage`) and trigger decision API (`shouldCompact`) that returns `shouldCompact`, ratio fields, token counts, and an observability `reason` string. Expanded `apps/desktop/src/lib/contextCompaction.test.ts` with below/at/above threshold tests plus explicit SC-006 override tests for `0.5` and `0.95`; added positive env threshold parse coverage.
- 2026-03-30 (claude): **046 Phase 1 review** — F-001..F-004 resolutions validated (TypeScript architecture, deterministic template, caller-provided git snapshot, passive token accumulator — all sound). Phase 1 code approved: config, threshold validation [0.5–0.95], env > config > default precedence, deterministic `stableSerializeHistory()` with key-sorted meta normalization. 5 findings: R-001 (`content: string` needs block union for Phase 3, LOW), R-002 (shallow meta sort, INFO), R-003 (missing positive env test, INFO), R-004 (required `id` field — verify runtime IDs exist before Phase 5, LOW), R-005 (F-005/F-006 unresolved for later phases, INFO). No blockers. Phase 1 approved → cursor for Phase 2. Review: `.ai/findings/046-phase1-review.md`.
- 2026-03-30 (claude): **046 plan review** — All FR/NF/SC requirements covered in plan. 7 findings: F-001 HIGH (compactor targets Rust `crates/orchestrator/` but conversation history lives in TypeScript — architectural mismatch), F-002 MEDIUM (deterministic `<task_summary>` construction undefined), F-003 MEDIUM (git state extraction source unspecified — `gitctx` crate?), F-004 MEDIUM (token count sourcing unspecified), F-005 LOW (Phase 3/4 ordering rework risk — add PreservePolicy trait), F-006 LOW (40% ceiling cascading-collapse strategy needed), F-007 LOW (SC-005 round-trip test should be structural, not LLM-dependent). No contract misses. F-001 must be resolved before coding. Review: `.ai/findings/046-plan-review.md`.
- 2026-03-30 (cursor): **046 Phase 1 start + design-resolution pass** — Updated `.ai/plans/046-context-compaction-phased-plan.md` to resolve F-001..F-004 (TypeScript architecture, deterministic summary template, caller-provided git snapshot, passive usage-fed token monitor). Added `apps/desktop/src/lib/contextCompaction.ts` and `apps/desktop/src/lib/contextCompaction.test.ts` with config parsing/validation (`OAP_COMPACTION_THRESHOLD`, `compaction.threshold`), deterministic history model serialization, and boundary/precedence tests.
- 2026-03-30 (cursor): **046 planning pass** — Responded to baton by reading `specs/046-context-compaction/spec.md` and drafting `.ai/plans/046-context-compaction-phased-plan.md`. Plan includes 6 phases: config + trigger, deterministic compactor + XML schema, preserve/compress policy, interruption heuristics, session-init integration, and perf/round-trip validation aligned to FR/NF/SC.
- 2026-03-30 (claude): **044 Phase 6 review** — Phase 6 approved. P5-002 verified (project_path → working_directory). P5-001 verified (systemPrompt sidecar field on provider path). E2e 3-step test validates SC-001 (artifact chaining via filesystem), SC-002 (630 tokens < 800 = 80% below 4000 baseline), SC-005 (quick < investigate < deep output sizes). 15/15 tests pass. 4 findings: P6-001 (test double not real agent, INFO), P6-002 (double system prompt in governed path, INFO), P6-003 (hardcoded baseline, INFO), P6-004 (step prompt in user slot on provider path, LOW — acceptable). All FRs done except FR-001 NL decomposition (deferred). SC-001/002/003/004/005/006 done. SC-007 deferred. 044 feature-complete. Review: `.ai/findings/044-phase6-review.md`.
- 2026-03-30 (cursor): **044 Phase 6** — P5-002 fixed by adding `project_path` to `orchestrate_manifest` and wiring executor `working_directory` to it. P5-001 addressed on provider path by sending sidecar `systemPrompt` separately from `prompt`. Added e2e 3-step YAML manifest test (`research -> draft -> review`) validating SC-001 artifact chaining, SC-002 token accounting, and SC-005 effort-size differentiation. Validation: `cargo test --manifest-path crates/orchestrator/Cargo.toml` (15/15), `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` pass. Details: `.ai/findings/044-phase6-validation.md`.
- 2026-03-30 (claude): **044 Phase 5 review** — `RealGovernedExecutor` dual-path dispatch (provider-registry sidecar for `providerId:apiModel`, governed Claude CLI with 035 MCP config for native models). Per-agent profiles (model, system_prompt, permissions) loaded from SQLite. Token tracking from JSONL result events. Sidecar protocol extended with `systemPrompt` (both provider + bridge paths). 14/14 tests pass. Findings: P5-001 (system prompt in user slot, LOW), P5-002 (working_directory CWD not project path, LOW — fix before e2e), P5-003 (model routing heuristic, LOW), P5-004–P5-006 (INFO). Review: `.ai/findings/044-phase5-review.md`. Phase 5 approved → cursor for Phase 6.
- 2026-03-30 (cursor): **044 Phase 5** — `RealGovernedExecutor` replaces `FileBackedGovernedExecutor`. `AgentExecutionProfile` with model/system_prompt/permissions from SQLite. `dispatch_via_provider_registry()` spawns Node sidecar with JSONL protocol. `dispatch_via_governed_claude()` spawns `claude` CLI with 035 axiomregent MCP config + permission grants (file_read/write/network, max_tier 3). Bypass fallback when axiomregent unavailable.
- 2026-03-30 (claude): **044 Phase 4 review** — All 4 spec contract operations implemented (orchestrate_manifest, get_run_status, cancel_run, cleanup_artifacts). All 4 error semantics present (StepFailed, DependencyMissing, AgentNotFound, CycleDetected). P3-001 resolved (output paths in prompt), P3-002 resolved ("no sub-agent calls"), P3-003 resolved (zero-input test). Async `dispatch_manifest()` with `AgentRegistry` + `GovernedExecutor` traits — correct trait-based design for swappable implementations. 14/14 tests pass. Findings: P4-001 (cancel_run in-memory only, LOW), P4-002–P4-006 (INFO). Review: `.ai/findings/044-phase4-review.md`. Phase 4 approved → cursor for Phase 5.
- 2026-03-30 (cursor): **044 Phase 4** — Async `dispatch_manifest()` with `AgentRegistry` + `GovernedExecutor` traits, `DispatchRequest`/`DispatchResult` types, output artifact verification post-dispatch, `FileBackedGovernedExecutor` scaffold. Tauri commands: `orchestrate_manifest`, `get_run_status`, `cancel_run`, `cleanup_artifacts`. P3-001/P3-002/P3-003 resolved. Tests: 3 new (async success, agent-not-found, zero-input prompt).
- 2026-03-30 (claude): **044 Phase 3 review** — FR-004 satisfied. `build_step_system_prompt()` includes absolute input artifact paths, filesystem-read directive, and effort level text matching FR-005. 11/11 tests pass. Findings: P3-001 (output paths not in prompt, LOW — actionable before real dispatch), P3-002 (Quick text omits "no sub-agent calls", COSMETIC), P3-003 (no zero-input test, INFO), P3-004 (to_string_lossy, INFO). Full wiring plan for governed dispatch + registry + Tauri commands provided. Review: `.ai/findings/044-phase3-review.md`. Phase 3 approved → cursor for Phase 4.
- 2026-03-30 (claude): **044 Phase 2 review** — All Phase 2 deliverables spec-faithful. FR-002 (input gating), FR-006 (summary persistence), FR-008 (failure cascade) all fully implemented. `dispatch_manifest_noop()` checks artifact existence, marks Failure/Skipped, writes summary.json before error return. 10/10 tests pass. Findings: P2-001 (unused var, COSMETIC — fixed), P2-002 (summary code dup, LOW), P2-003 (no transitive cascade test, INFO), P2-004 (linear producer scan, INFO), P2-005 (root noop success, INFO). Review: `.ai/findings/044-phase2-review.md`. Phase 2 approved → cursor for Phase 3.
- 2026-03-30 (cursor): **044 Phase 2** — `dispatch_manifest_noop()`: no-op executor with FR-002 input gating, FR-008 Skipped cascade (eager + lazy), `RunSummary::write_to_disk()`, summary.json persistence on success and failure paths. Tests: 2 new unit tests (success path + failure cascade).
- 2026-03-30 (claude): **044 Phase 1 review** — All Phase 1 deliverables spec-faithful. FR-002/003/005/006/007 covered; FR-001/008 partial (expected — dispatch-phase). Error variants match spec (4/4 + 1 extension). DAG validation: cycle detection, duplicate outputs, input reference checks all correct. 8/8 tests pass. Findings: P1-001 (CycleDetected generic message, LOW), P1-002 (greedy "quick" match, INFO), P1-003 (no investigate keyword test, INFO), P1-004 (no empty-manifest test, INFO). Review: `.ai/findings/044-phase1-review.md`. Phase 1 approved → cursor for Phase 2.
- 2026-03-29 (cursor): **044 Phase 1** — `crates/orchestrator`: `WorkflowManifest` YAML, DAG validation + topological order, `ArtifactManager`, `EffortLevel` + `classify_from_task`, `materialize_run_directory`, `resolve_input_paths`, `OrchestratorError` variants. Tests: 8 unit tests in crate.
- 2026-03-29 (claude): **042 Phase 6 review** — All 6 phases spec-faithful. Sidecar dual-path verified (registry for `providerId:apiModel`, legacy fallback). P5-003 resolved. AgentEventBridgeEncoder → BridgeEvent JSONL correct. 20/20 tests pass. Findings: P6-001 (encoder test thin, LOW), P6-002 (mergeAbortSignals dup, COSMETIC), P6-004 (no permission broker on registry path, LOW), P6-005 (hard-coded provider IDs, INFO). Review: `.ai/findings/042-phase6-review.md`. 042 complete. Baton → cursor for 044.
- 2026-03-29 (cursor): **042 Phase 6** — `ProviderRegistry` wired into Tauri Node sidecar (`packages/provider-registry/dist/node-sidecar.js`); `registerBuiltInProvidersFromEnv`; model prefix `providerId:apiModel`; `AgentEventBridgeEncoder`; Gemini/Bedrock tool-result round-trip. Legacy `packages/claude-code-bridge/src/sidecar.ts` removed (superseded).
- 2026-03-29 (claude): **042 Phase 5 review** — All 5 adapters spec-faithful (Anthropic, OpenAI, ClaudeCodeSDK, Gemini, Bedrock). 16/16 tests pass. SC-001–SC-006 satisfied. Findings: P5-002 (mergeAbortSignals 4× dup, COSMETIC), P5-003 (tool role unmapped in Gemini/Bedrock, LOW — Phase 6 prerequisite), P5-005 (Bedrock test thin, LOW), P5-006 (vision content blocks stringify, LOW). Review: `.ai/findings/042-phase5-review.md`. Baton → cursor for Phase 6.
- 2026-03-29 (cursor): **042 Phase 5** — Gemini (`@google/generative-ai`) + Bedrock Converse/ConverseStream adapters, stream normalizers, unit tests; exports on `@opc/provider-registry`.
- 2026-03-29 (claude): **042 Phase 4 review** — OpenAI adapter mirrors Anthropic pattern. 13/13 tests pass. SC-001 satisfied (3 adapters). P4-002 (vision content stringified, LOW), P4-003 (mergeAbortSignals dup, COSMETIC). Review: `.ai/findings/042-phase4-review.md`. Baton → cursor for Phase 5/6.
- 2026-03-29 (cursor): **042 Phase 4** — OpenAI Chat Completions: `createOpenAIProvider`, `OpenAIStreamNormalizer`, `completionToAgentEvents`, vitest. SC-001 (two adapters) satisfied by Anthropic + OpenAI.
- 2026-03-29 (review): **042 Phase 3 review** — `.ai/findings/042-phase3-review.md`. FR-002/FR-003/FR-007 satisfied; FR-006/SC-006 vs optional API key documented LOW; P3-002/003 INFO. Next: Phase 4 OpenAI adapter.
- 2026-03-29 (cursor): **042 Phase 3** — Claude Code SDK provider: `queryClaudeCode()` + `ClaudeCodeBridgeNormalizer`, session resume map, abort forwarding, `ambient-claude-code-sdk.d.ts` for tsc with linked bridge. Tests: `claude-code-events.test.ts`. 10/10 vitest pass.
- 2026-03-29 (claude): **042 Phase 2 review** — Adapter implements all 4 Provider ops, normalizer handles text/tool/thinking. 7/7 tests pass. P2-001 (input tokens 0 in streaming) + P2-002 (no text_complete in streaming), both LOW. Review: `.ai/findings/042-phase2-review.md`. Baton → cursor for Phase 3.
- 2026-03-29 (cursor): **042 Phase 2** — Anthropic Messages provider (`@anthropic-ai/sdk`), `AnthropicStreamNormalizer`, `messageToAgentEvents`, exports + tests.
- 2026-03-29 (claude): **042 Phase 1 review** — All types match spec byte-for-byte. FR-001–FR-007 covered. 6/6 tests pass. No issues. Review: `.ai/findings/042-phase1-review.md`. Baton → cursor for Phase 2 (Anthropic adapter).
- 2026-03-29 (cursor): **042 Phase 1** — New package `@opc/provider-registry` (`InMemoryProviderRegistry`, types, vitest).
- 2026-03-29 (claude): **045 → active** — Confirmed `ClaudeCodeSession` uses `executeClaudeBridge` for new + resumed prompts, no legacy CLI paths remain. Promoted spec to `status: active`. 045 complete. Baton → cursor for 042.
- 2026-03-29 (cursor): **045 session UI → bridge** — `ClaudeCodeSession` calls `executeClaudeBridge` for new and resumed prompts (replaces CLI `execute` / `resume`).
- 2026-03-29 (claude): **045 final confirmation** — F-011 verified (abort JSON before stdin drop). API bindings verified (`executeClaudeBridge`, `respondToBridgePermission`, web adapter routes). All 9 FRs pass, all HIGH/MEDIUM findings resolved. 045 feature-complete, ready for `status: active` on UI wiring. Full tracker: `.ai/findings/045-final-status.md`. Baton → cursor for session UI + 042.
- 2026-03-29 (cursor): **F-011 + api.ts** — Abort JSON on bridge stdin before close; `executeClaudeBridge` / `respondToBridgePermission` on desktop API; `apiAdapter` REST mappings for web fallback.
- 2026-03-29 (claude): **045 sidecar review** — All 9 FRs verified ✅. FR-004 PermissionBroker now fully wired (F-003 resolved). New finding F-011: `cancel_claude_execution` drops stdin without sending abort JSON (MEDIUM). F-012–F-014 cosmetic. Architecture matches IPC plan. Full review: `.ai/findings/045-sidecar-review.md`. Baton → cursor for F-011 + frontend api.ts.
- 2026-03-29 (cursor): **045 Tauri ↔ Node sidecar** — `sidecar.ts`, `claude-output-lines.ts`, `execute_claude_bridge` / `respond_to_bridge_permission`, `ClaudeBridgeIpcState`, `spawn_claude_bridge_process`; desktop re-exports mapper from `@opc/claude-code-bridge/claude-output-lines`. Requires `dist/sidecar.js` (run `tsc` in bridge package). Next: claude review; frontend API wiring.
- 2026-03-29 (claude): **045 post-F001/F002 review** — Verified F-001, F-002, F-005, F-006 all resolved. F-003 (PermissionBroker) confirmed still dead code but deferred to sidecar slice. New findings: F-008 (start event drop OK by design), F-009 (stale closure risk in error branch, LOW), F-010 (index signature masks type errors, blocked on consumer audit). Spec parity: 9/9 FR done, FR-004 partial. Full review: `.ai/findings/045-post-f001f002-review.md`. Next: cursor implements sidecar.
- 2026-03-29 (cursor): **045 F-001 + F-002** — `useClaudeMessages` parity with SDK/stream-json; `cli-adapter` duplicate `session-complete` eliminated. Types consolidated via `AgentExecution` `ClaudeStreamMessage`. Next: Tauri/Node sidecar + F-003 optional.
- 2026-03-29 (claude): **045 review** — 7 findings in `.ai/findings/045-bridge-parity-gaps.md`. Critical: useClaudeMessages checks wrong message types (F-001), CLI adapter double session-complete (F-002), PermissionBroker dead code (F-003). Produced sidecar IPC plan in `.ai/findings/045-tauri-node-ipc-plan.md` recommending Node sidecar with stdin/stdout JSONL protocol. Next: fix F-001/F-002, then implement sidecar.
- 2026-03-29 (cursor): 045 integration slice — `@opc/claude-code-bridge` wired into `apps/desktop` (workspace dep); `bridgeEventToClaudeOutputLines()` maps bridge events to JSONL strings for existing `claude-output` consumers; `packages/claude-code-bridge` exports `./types`, ambient SDK declaration, `cli-adapter` import cleanup. Tests: `apps/desktop/src/lib/bridgeEventToClaudeOutput.test.ts`. Next: Tauri/Node bridge process + permission round-trip.
- 2026-03-29 (claude): First vertical slice for 045 landed — `packages/claude-code-bridge/` (5 files). Implements: typed `queryClaudeCode()` async generator (FR-001), `BridgeQueryOptions` (FR-002), session resumption (FR-003), `canUseTool` permission broker (FR-004), permission mode fallback (FR-005), AbortController cancellation (FR-006), cost tracking from SDKResultMessage (FR-007), CLI fallback when SDK absent (FR-008), discriminated union BridgeEvent (FR-009). Next: Tauri backend + frontend integration.
- 2026-03-29 (claude-opus): Stagecraft-ing full extraction + integration. 17 projects analyzed, 189 items extracted, 62 consolidated, 65 files created (9 commands, 4 agents, 1 rule, 3 code modules, 3 ast-grep rules, 3 devcontainer files, 22 specs, CLAUDE.md, AGENTS.md). All source projects confirmed safe to delete.
- 2026-03-30 (cursor): 044 Phase 3 helper — `crates/orchestrator::build_step_system_prompt()` added to construct FR-004-compliant system prompts (absolute artifact paths + effort directive) for each `WorkflowStep`, with unit test coverage. Next: claude reviews prompt contract and recommends integration points into governed agent dispatch + provider registry path in Tauri.
- 2026-03-29 (claude-opus): Slice H complete. V-005 message wording fixed. All residuals cleared.
- 2026-03-29 (claude-opus): Post-041 synthesis complete. Authority-map, next-slice, integration-debt all updated for 032–041.
