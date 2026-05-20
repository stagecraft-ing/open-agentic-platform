# Spec-Spine `/init` Trace — Classification Pass

**Branch:** `cut-d/autonomous-run-20260519-025506`
**Date:** 2026-05-19
**Method:** Static read of protocol files (`.claude/commands/init.md`, `AGENTS.md` "New Sessions", `CLAUDE.md`) and the rule files they reference. No tool invocation, no `/init` execution, no patch.
**Scope:** Classification of every file, binary, and rule the `/init` protocol touches into `generic` / `OAP-specific` / `hybrid`, with confidence and evidence, plus drift surfaces and a standalone ship-list sketch.
**Decision rule:** When in doubt, lean generic. Spec-spine MUST be standalone.
**Halt check:** Premise holds. No coupling discovered that cannot be templated or decomposed; the OAP-specific bindings the trace surfaces are all addressable by template slots or by the surfaced render-path decision.

---

## S1 — Protocol source comparison

The three sources are not aligned. The table records the literal prescription from each.

| Protocol step | `.claude/commands/init.md` | `AGENTS.md` "New Sessions" | `CLAUDE.md` |
|---|---|---|---|
| Rules pre-load | — (Step 0 is `.specify/memory/`, not rules) | Step 0: load `.claude/rules/orchestrator-rules.md` AND `.claude/rules/governed-artifact-reads.md` (`AGENTS.md:9`) | "all orchestrated workflows load `.claude/rules/governed-artifact-reads.md` (spec 103) and `.claude/rules/adversarial-prompt-refusal.md` (CONST-005, spec 131) automatically" (`CLAUDE.md:23-28`) |
| Memory load | Step 0: read all files in `.specify/memory/` (`init.md:10-12`) | — (not mentioned) | — |
| Identity reads | Step 1 parallel: `AGENTS.md`, `CLAUDE.md`, `README.md` (`init.md:20-23`) | Step 1 parallel: `CLAUDE.md`, `README.md` (no `AGENTS.md` self-reference) (`AGENTS.md:12-13`) | — |
| Contract read | Step 1: `.specify/contract.md` (`init.md:26`) | — | — |
| Spec list | Step 1: `ls specs/` (`init.md:27`) | Step 1: `registry-consumer list --ids-only` (`AGENTS.md:16`) | — |
| Structural index | Step 1: `build/codebase-index/index.json` read directly (`init.md:30`) | Step 1: `codebase-indexer check` (staleness); `oap-code-index-enrich render` only if `CODEBASE-INDEX.md` missing; then read the markdown (`AGENTS.md:13-14`) | — |
| Lifecycle counts | — | Step 1: `registry-consumer status-report --json --nonzero-only` (`AGENTS.md:15`) | — |
| Tools / apps / docs listings | Step 1: `ls tools/`, `ls apps/`, `ls docs/` (`init.md:33-35`) | — | — |
| Git activity | Step 1: `git log --oneline -15`, `git diff --stat HEAD~1`, `git branch --show-current`, `git status --short` (`init.md:38-41`) | Step 1: `git log --oneline -10`, `git diff --stat HEAD~1` (`AGENTS.md:17-18`) | — |
| Self-extending hook | Step 1: parse AGENTS.md "New Sessions" for additional items (`init.md:43`) | (the source itself) | — |
| Read discipline note | — | "the init protocol MUST NOT parse `build/**/*.json` directly (no `python`, `jq`, `awk`, `sed` against compiled artifacts)" (`AGENTS.md:21`) | (spec 103 reference, `CLAUDE.md:23-26`) |
| Staleness handling | — | `codebase-indexer check` non-zero → surface "Structural index: stale" and continue (`AGENTS.md:23`) | — |
| Missing-binary handling | — | "instruct the user to `cargo build --release --manifest-path tools/<name>/Cargo.toml` and continue — do NOT fall back to ad-hoc parsing" (`AGENTS.md:25`) | — |
| Summary emission | Step 2: `## initialized: open-agentic-platform` structured block; specific template at `init.md:51-71` | Step 2: `## initialized: open-agentic-platform` summary block (`AGENTS.md:19`) — no template body | — |

**Self-description chain:**
- `init.md` self-describes as subordinate: "the 'New Sessions' section of AGENTS.md defines the init checklist" (`init.md:6`).
- `AGENTS.md` self-describes as the self-extending root: "Run `/init` as the mandatory first action of every new session. The command reads this section to derive its execution plan dynamically — any item added here is automatically picked up on the next init" (`AGENTS.md:5`).
- `CLAUDE.md` self-describes as a project convention reference, not the protocol root (`CLAUDE.md:1-32`).

Per design, `AGENTS.md` "New Sessions" is the authoritative protocol; `init.md` is the executor surface; `CLAUDE.md` is the convention layer. The drift section (S5) treats divergence between these three under that hierarchy.

---

## S2 — File classification

| File | Identity | Use | Classification | Confidence | Evidence | Generic template / OAP slot (if hybrid) |
|---|---|---|---|---|---|---|
| `.claude/commands/init.md` | The `/init` slash-command definition. | Loaded as the command body when `/init` fires. | hybrid | high | Generic structure (Step 0 memory, Step 1 parallel reads, Step 2 summary); OAP-specific listings (`ls tools/`, `ls apps/`, `ls docs/`, README content) and the OAP project name in the summary template (`init.md:52`). | Generic: step layout + memory load + git verbs + summary emission. OAP slot: project name, supplementary `ls` targets, "Ready to help with" content. |
| `AGENTS.md` | The self-extending agent protocol. | Read whole; "New Sessions" section parsed by `init.md` self-extending hook. | hybrid | high | Generic mechanism (self-extending "New Sessions" pattern, governed reads via consumer binaries); OAP-specific protocol body (`oap-code-index-enrich`, "open-agentic-platform" project name, OAP agent list `AGENTS.md:33-37`, OAP commands `AGENTS.md:43-53`). | Generic: "New Sessions" stub + self-extending mechanism + read-discipline reminder. OAP slot: agent list, command list, OAP binaries. |
| `CLAUDE.md` | Project overview, conventions, build commands, policy rules. | Read whole. | hybrid | high | Generic structure (project overview / repository structure / conventions / build commands / policy rules); content is OAP-specific (OPC desktop, platform, axiomregent, factory adapters, CONST-001..005 names). | Generic: section skeleton + rule-loading convention sentence. OAP slot: all narrative content. |
| `README.md` | Project README, public-facing. | Read whole. | OAP-specific | high | Whole document is OAP product narrative (`README.md:1-358` — AGPL choice, OWASP ASI 2026, Rauthy, factory adapters, install paths). | n/a — README is intrinsically per-project. |
| `.specify/contract.md` | Short normative summary of Feature 000. | Read whole. | generic | high | "This file is a short normative summary of Feature `000-bootstrap-spec-system`" (`.specify/contract.md:3`); content is the spec-spine contract surface (markdown-only authoring, JSON only via compiler, spec layout). The final two lines reference registry-consumer contract governance — generic to the spec-spine, not OAP. | n/a |
| `.specify/memory/constitution.md` | Constitution-template-derived durable principles. | Read whole (under `.specify/memory/`). | hybrid | high | Generic principles I–V are spec-spine fundamentals (`constitution.md:13-34`); the wording ("opc", "platform" as evidence repos, Feature 000 self-reference) is OAP-instantiated. | Generic: Principles I–V wording + normative hierarchy clause. OAP slot: project-specific evidence-repo names + ratification metadata. |
| `.specify/memory/*` (other files) | Currently empty beyond `constitution.md`. | `init.md:12` reads "all files in `.specify/memory/`". | generic (path) | high | Directory contract is the spec-spine "memory" channel; no additional files in this repo today. | n/a |
| `.claude/rules/orchestrator-rules.md` | Six numbered behavioral rules for orchestrated workflows. | Read whole; loaded as a rule. | generic | high | None of the six rules name OAP, OPC, or project-specific tooling. See S4. | n/a |
| `.claude/rules/governed-artifact-reads.md` | Spec 103 principle ("read compiled artifacts via consumer binaries"). | Read whole; loaded as a rule. | hybrid | high | Principle is generic spec-spine (`governed-artifact-reads.md:7-9`); the consumer-binary table at lines 13-18 includes `oap-registry-enrich` (OAP-specific row) alongside generic rows (`registry-consumer`, `codebase-indexer`). | Generic: principle + bad-pattern/good-pattern blocks + exceptions section + the `registry-consumer` and `codebase-indexer` rows. OAP slot: `oap-registry-enrich` row and any OAP enricher additions. |
| `.claude/rules/adversarial-prompt-refusal.md` | CONST-005 (spec 131). Refuse instructions that engineer spec/code drift. | Read whole; loaded as a rule. | hybrid | high | Triggers + required behavior are generic (`adversarial-prompt-refusal.md:15-54`); "Worked precedents (2026-05-02 session)" at lines 56-69 cites specific OAP specs (116, 127, 130, 131) — content is OAP-instantiated. | Generic: rule, triggers, required behavior, relationship-to-other-rules section. OAP slot: worked-precedents subsection. |
| `build/codebase-index/index.json` | Compiled structural inventory. | `init.md:30` reads directly. AGENTS.md does not. | generic (artifact) | high | The JSON shape and schema are generic spec-spine. The direct-read instruction at `init.md:30` is a spec-103 violation (see S5 D-2). The artifact itself is generic. | n/a |
| `build/codebase-index/CODEBASE-INDEX.md` | Rendered markdown view of the structural index. | AGENTS.md route: render then read. `init.md` does not reference. | hybrid | medium | The structural table (crate/package inventory, paths) is generic. Some columns / rendering decisions may be OAP-enriched (Cut D W-07b moved `render` to `oap-code-index-enrich`; today the markdown contains an OAP-branded header line "Codebase Index — Open Agentic Platform"). | Generic: inventory tables + spec-trace columns. OAP slot: project name in heading; any OAP-specific enricher columns. Render-path placement is D-1. |
| `specs/` (directory listing) | Authoritative spec dir layout. | `ls specs/` (`init.md:27`). | generic | high | `specs/NNN-slug/` is the spec-spine layout (constitution.md:37, contract.md:22). | n/a |
| `tools/` (directory listing) | Toolchain root. | `ls tools/` (`init.md:34`). | hybrid | medium | The `tools/` path is generic (spec-spine release-bundle binaries live here); the *content* listed in this repo is a mix of generic spec-spine binaries (`spec-compiler`, `registry-consumer`, `codebase-indexer`, `spec-lint`, `policy-compiler`) and OAP-specific binaries (`oap-code-index-enrich`, `oap-registry-enrich`, `adapter-scopes-compiler`, `assumption-cascade-check`, `ci-parity-check`, `schema-parity-check`, `spec-code-coupling-check`, `stakeholder-doc-lint`). | Generic: convention of putting toolchain at `tools/`. OAP slot: which binaries are present. |
| `apps/` (directory listing) | App targets root. | `ls apps/` (`init.md:34`). | OAP-specific | high | `apps/desktop/` is OAP's OPC. The path convention is generic, but a spec-spine adopter may not have an `apps/` dir at all. | The convention "applications go in `apps/`" is generic. The presence and content are OAP-specific. |
| `docs/` (directory listing) | Documentation root. | `ls docs/` (`init.md:35`). | generic (path) | medium | `docs/` is a conventional documentation root; content (`ARCHITECTURE.md`, `registry-consumer-contract-governance.md`, `analysis/`, `runbooks/`) is project-specific but the listing instruction in `init.md` is path-generic. | n/a (content varies per project). |

**Footnote on `.claude/agents/` and `.claude/commands/`.** `init.md` does not currently instruct reading these directories during init. They appear in the protocol surface only indirectly through the rule files. If an adopter extracts the spec-spine, they will need their own `.claude/agents/` and `.claude/commands/` directories; the templates ship the directory convention, not the populated contents.

---

## S3 — Binary classification

| Binary | Subcommand & flags | Identity | Classification | Confidence | Evidence |
|---|---|---|---|---|---|
| `codebase-indexer` | `check` | Staleness gate for `build/codebase-index/index.json`. Exits non-zero if the index is stale relative to inputs. | generic | high | Tool defined under spec 101 (codebase-index MVP); the consumer binary for the generic codebase-index artifact (`governed-artifact-reads.md:17`). |
| `oap-code-index-enrich` | `render` | Renders `CODEBASE-INDEX.md` from `index.json`. Moved here from `codebase-indexer` in Cut D W-07b. | OAP-specific | high | The `oap-` name prefix and the W-07b move are explicit project decisions (`CLAUDE.md:101-102`; `governed-artifact-reads.md:17` lists `render` under `codebase-indexer` but `AGENTS.md:14` calls `oap-code-index-enrich render`). See D-1 for the render-path decision. |
| `registry-consumer` | `status-report --json --nonzero-only` | Lifecycle counts per spec status, JSON output, skip zero-count statuses. | generic | high | Generic spec-spine consumer (`governed-artifact-reads.md:15`). |
| `registry-consumer` | `list --ids-only` | Spec id list for "latest-spec detection". | generic | high | Generic spec-spine consumer; spec 031 defines `list --ids-only` contract. |
| `git` | `log --oneline -15` (init.md) / `-10` (AGENTS.md) | Recent commit history. | generic | high | Git is universal. The verb count differs between sources (S5 D-7). |
| `git` | `diff --stat HEAD~1` | Last commit diff stat. | generic | high | Both sources agree. |
| `git` | `branch --show-current` | Current branch name. | generic | high | `init.md:40`. |
| `git` | `status --short` | Uncommitted changes. | generic | high | `init.md:41`. |

### D-1 — Render-path placement (load-bearing decision)

`oap-code-index-enrich render` is classified OAP-specific (the binary name and the W-07b move record both confirm), but `/init` per `AGENTS.md` depends on its output (`CODEBASE-INDEX.md`) for the structural summary. An adopter taking only the generic spec-spine cannot run this command. Three resolutions:

| Option | Mechanism | Trade-off |
|---|---|---|
| **1. Generic render returns to `codebase-indexer`.** | Partial undo of W-07b. The render template is structurally generic (crate + package + spec table); only certain columns with OAP-shaped data are OAP-side. Generic core, OAP overlay. | Restores generic `/init` summary path. Requires re-decomposing W-07b cleanly: which columns are generic vs OAP. The W-07b move was deliberate; revisit the original rationale before reversal. |
| **2. Spec-spine `/init` skips render.** | The summary is produced from `codebase-indexer check` + `registry-consumer status-report` + `registry-consumer list --ids-only`. The markdown view is OAP-side enrichment, not required for `/init`. | Smallest extraction surface. Adopters lose the human-readable markdown view in `/init`. The summary becomes thinner (counts but not a per-crate spec column). |
| **3. Generic render template, OAP-specific column adapters.** | Generic `codebase-indexer render` emits the structural core; an enricher binary (`oap-code-index-enrich`) overlays project-specific columns. | Largest design surface. Requires a generic enricher contract. May be the right place to land long-term, but the cost is a new contract. |

No recommendation. The operator decides which trade-off matches the extraction goal.

---

## S4 — Rule classification

### `.claude/rules/orchestrator-rules.md`

| Sub-rule | Statement | Classification | Confidence | Evidence |
|---|---|---|---|---|
| File overall | Six numbered behavioral rules for any orchestrated workflow in this project. | generic | high | Header line: "These rules apply to any orchestrated, multi-step command or agent workflow in this project. Violating any rule is a failure." (`orchestrator-rules.md:3`). |
| 1 | Execute steps in order. | generic | high | Universal orchestration discipline (`orchestrator-rules.md:5`). |
| 2 | Write output files (file-based context passing). | generic | high | Universal orchestration discipline (`orchestrator-rules.md:6`). |
| 3 | Stop at checkpoints. | generic | high | Universal orchestration discipline (`orchestrator-rules.md:7`). |
| 4 | Halt on failure. | generic | high | Universal orchestration discipline (`orchestrator-rules.md:8`). |
| 5 | Use only local agents. | generic | high | The rule's wording ("agents bundled with this project. No cross-project dependencies") applies to any adopter project (`orchestrator-rules.md:9`); decision rule resolves to generic. |
| 6 | Never enter plan mode autonomously. | generic | high | Universal orchestration discipline (`orchestrator-rules.md:10`). |

### `.claude/rules/governed-artifact-reads.md`

| Sub-rule | Statement | Classification | Confidence | Evidence |
|---|---|---|---|---|
| File overall | Spec 103 principle: read compiled artifacts only through their designated consumer binaries. | hybrid | high | Principle is generic spec-spine; consumer-binary table mixes generic and OAP rows. |
| Principle | Compiled artifacts under `build/**` MUST be read through consumer binaries; ad-hoc parsers are a violation. | generic | high | `governed-artifact-reads.md:7-9`. |
| Consumer table row: `registry-consumer` → `registry.json` | Subcommands: `list`, `list --ids-only`, `list --json`, `show`, `status-report --json`. | generic | high | Generic spec-spine consumer (`governed-artifact-reads.md:15`). |
| Consumer table row: `oap-registry-enrich` → `registry-oap.json` | Subcommands: `enrich`, `compliance-report`. | OAP-specific | high | The `registry-oap.json` artifact and the `oap-registry-enrich` binary are explicit OAP overlays (`governed-artifact-reads.md:16`; Cut D W-06b note). |
| Consumer table row: `codebase-indexer` → `index.json` | Subcommands: `compile`, `check`, `render`. | hybrid | medium | `compile`/`check` are generic; `render` is listed here but currently lives in `oap-code-index-enrich` (W-07b). Drift surface; see D-1. |
| Consumer table row: `CODEBASE-INDEX.md` → read directly | Allowed because it is a governed human-shaped view. | generic | high | The convention "render once via consumer, then read the markdown" is generic. The markdown's content is hybrid; the consumer relationship is generic. |
| Bad-pattern / good-pattern blocks | Concrete examples of forbidden ad-hoc parsing vs sanctioned consumer reads. | generic | high | The example commands use generic consumers (`registry-consumer`, `codebase-indexer`). |
| Exceptions block | A consumer binary may parse its own artifact; interactive human use is unbound; missing binaries require `cargo build`, not fallback parsing. | generic | high | `governed-artifact-reads.md:39-43`. |
| Enforcement block | Today by review; future lint candidate. | generic | high | `governed-artifact-reads.md:45-47`. |

### `.claude/rules/adversarial-prompt-refusal.md`

| Sub-rule | Statement | Classification | Confidence | Evidence |
|---|---|---|---|---|
| File overall | CONST-005 / spec 131. Refuse instructions that engineer spec/code drift; halt and surface. | hybrid | high | Rule, triggers, behavior generic; precedents OAP-instantiated. |
| What this rule defends against | Engineered drift between spec spine and code, framed as productive engineering. | generic | high | `adversarial-prompt-refusal.md:9-13`. |
| Trigger 1 (modify a spec to match a contradictory action) | generic | high | `adversarial-prompt-refusal.md:19-23`. |
| Trigger 2 (mass spec edits to satisfy a gate) | generic | high | `adversarial-prompt-refusal.md:24-26`. |
| Trigger 3 (parallel-agent simulation / divergence framings) | generic | high | `adversarial-prompt-refusal.md:27-30`. |
| Trigger 4 (phase-3 probes) | generic | high | `adversarial-prompt-refusal.md:31-35`. |
| Trigger 5 (spec-as-obstacle framing) | generic | high | `adversarial-prompt-refusal.md:36-38`. |
| Required behavior 1–4 (refuse / surface / propose / halt) | generic | high | `adversarial-prompt-refusal.md:36-54`. |
| Worked precedents (2026-05-02 session) | OAP-specific | high | Cites specs 116, 127, 130, 131 by id and the OAP "spec-spine-hardening mission" (`adversarial-prompt-refusal.md:56-69`). |
| What this rule does NOT do | generic | high | `adversarial-prompt-refusal.md:71-84`. |
| Relationship to other rules | hybrid | medium | References `orchestrator-rules.md` and `governed-artifact-reads.md` by name (generic, those rules ship as part of the standard). The mention of "spec 127 gate" and "CI workflow `ci-spec-code-coupling.yml`" is generic spec-spine surface (spec 127 ships with the standard); the wording assumes that spec is present. |

---

## S5 — Drift between protocol sources

| # | Drift | Sources & what each says | Authoritative per design | Severity | Suggested resolution (descriptive only) |
|---|---|---|---|---|---|
| D-2.1 | **Rules pre-load divergence.** Three different prescriptions for which rule files load on init. | `init.md:10-12` says "Load memory from `.specify/memory/`" (no rule pre-load mentioned). `AGENTS.md:9` says "Load rules — read `orchestrator-rules.md` AND `governed-artifact-reads.md`." `CLAUDE.md:23-28` says "all orchestrated workflows load `governed-artifact-reads.md` (spec 103) and `adversarial-prompt-refusal.md` (CONST-005, spec 131) automatically." Note: `AGENTS.md` excludes `adversarial-prompt-refusal.md`; `CLAUDE.md` excludes `orchestrator-rules.md`; `init.md` excludes all three. | `AGENTS.md` "New Sessions" is the self-extending root per `init.md:6`. | contract-drift | Align the three sources on a single list of rule pre-loads. Whichever set is canonical, name it in `AGENTS.md` "New Sessions" Step 0 and have the others defer. |
| D-2.2 | **Structural-index read path is a spec-103 violation in `init.md`.** | `init.md:30` instructs reading `build/codebase-index/index.json` directly. `AGENTS.md:13-14` routes the read via `codebase-indexer check` + `oap-code-index-enrich render` + the rendered markdown. `governed-artifact-reads.md:7-9` forbids ad-hoc parsing of `build/**/*.json` in orchestrated workflows. | `AGENTS.md` + spec 103. | spec-103-violation | Replace the `build/codebase-index/index.json` line in `init.md` with the governed-read path. (Surface only — not patched here.) |
| D-2.3 | **Identity reads diverge.** | `init.md:20-23` reads `AGENTS.md`, `CLAUDE.md`, `README.md`. `AGENTS.md:12-13` reads only `CLAUDE.md` and `README.md` (no `AGENTS.md` self-read, which is sensible — the protocol cannot read itself in a step it defines, the executor reads it implicitly). | `AGENTS.md` is canonical; `init.md` adds the self-read explicitly because the executor needs the protocol to parse the "New Sessions" hook. Both can be correct under different framing. | contract-drift | Document the asymmetry: `AGENTS.md` is implicitly read by `/init` as the protocol source itself, separately from the identity-read list. The two sources can be aligned by stating this explicitly in one place. |
| D-2.4 | **Contract.md read missing from AGENTS.md.** | `init.md:26` reads `.specify/contract.md`. `AGENTS.md` does not list it. | `AGENTS.md` is canonical, but `init.md` adds it for orientation. | contract-drift | Decide whether `.specify/contract.md` belongs in the canonical `AGENTS.md` list. Per S2 it is generic and worth loading. |
| D-2.5 | **Spec-list path: `ls` vs governed consumer.** | `init.md:27` uses `ls specs/`. `AGENTS.md:16` uses `registry-consumer list --ids-only`. | `AGENTS.md` + spec 103 spirit (governed read). `ls` of source directories is not strictly a `build/**` violation, but the registry-consumer path is the typed surface. | minor (not a 103 violation in the strict sense, since `specs/` is authored, not compiled) | Align on `registry-consumer list --ids-only` so the count surface is typed and matches lifecycle counts. |
| D-2.6 | **Lifecycle counts missing from `init.md`.** | `init.md` does not invoke `registry-consumer status-report`. `AGENTS.md:15` does. | `AGENTS.md`. | contract-drift | Add the call to `init.md` Step 1 or remove from `AGENTS.md` if the summary template does not need it. The summary template in `init.md:51-71` does not currently surface lifecycle counts. |
| D-2.7 | **Git log verb count.** | `init.md:38` uses `-15`; `AGENTS.md:18` uses `-10`. | Neither is structurally authoritative; cosmetic. | minor | Pick one. The choice has no semantic consequence. |
| D-2.8 | **Memory load only in `init.md`.** | `init.md:10-12` reads `.specify/memory/`. `AGENTS.md` does not. | `init.md` adds the read. `AGENTS.md` does not exclude it. | contract-drift | Add an explicit "Step 0: memory" line to `AGENTS.md` "New Sessions", or document that the executor handles it before Step 0 of the listed protocol. |
| D-2.9 | **`ls tools/`, `ls apps/`, `ls docs/` only in `init.md`.** | `init.md:33-35` lists project-specific directories. `AGENTS.md` does not. | These are OAP-specific orientation reads. | minor | Move OAP-specific listings into the `AGENTS.md` "New Sessions" hook so the generic `init.md` template stays generic. (See S2: `init.md` is hybrid today partly because of these lines.) |
| D-2.10 | **Render binary identity.** | `AGENTS.md:14` calls `oap-code-index-enrich render` (OAP-specific binary, classified OAP-specific in S3). `init.md` is silent on render. `governed-artifact-reads.md:17` lists `render` under `codebase-indexer` (generic), reflecting the pre-W-07b name. | The post-W-07b reality is `oap-code-index-enrich`. The rule-file table is stale. | contract-drift | Reconcile the rule-file table with the actual binary. Tied to D-1 (render-path decision). |
| D-2.11 | **Summary template only in `init.md`.** | `init.md:51-71` contains the structured `## initialized:` template. `AGENTS.md:19` references the emission step but does not specify the template body. | `init.md` is the executor; the template body lives where the executor is. | minor | No change needed unless the template needs to be re-shared between the two sources. |

---

## S6 — Standalone-shape ship list

What spec-spine must provide for an adopter repo to get `/init` working standalone, sorted into the four categories.

### Template files

| Item | Adopter action | S2/S4 cross-ref | Open question |
|---|---|---|---|
| `.claude/commands/init.md` | Copy verbatim **after** D-2.* drift is resolved. Today the file ships OAP-specific `ls tools/`, `ls apps/`, `ls docs/` lines that an adopter would have to scrub or re-target. | S2 (hybrid) | D-2.2 (spec-103 violation), D-2.9 (OAP-specific listings) must be resolved before this file is extraction-ready. |
| `AGENTS.md` (with "New Sessions" stub only) | Copy as template; populate "Available Agents" / "Available Commands" / "Conventions" with project content. Adopter fills the "New Sessions" hook with project-specific reads. | S2 (hybrid) | D-1 (render placement) determines whether `AGENTS.md` ships `codebase-indexer render` or an OAP-specific enricher hook. |
| `CLAUDE.md` (skeleton) | Copy as template; specialize all project narrative. The five policy rules CONST-001..005 are generic spec-spine constants (block destructive ops, secrets scanner, tool allowlist, diff-size, spec-code coherence) and ship verbatim. | S2 (hybrid) | None — but adopters need to know which CONST policies are required vs optional. |
| `.specify/memory/constitution.md` | Copy as template; specialize ratification date and project name. Principles I–V ship verbatim. | S2 (hybrid) | None. Generic principles I–V are the load-bearing content. |
| `.specify/contract.md` | Copy verbatim. Already a short normative summary of Feature 000. | S2 (generic) | None. |
| `.specify/templates/*.md` (`agent-file-template.md`, `checklist-template.md`, `constitution-template.md`, `plan-template.md`, `spec-template.md`, `tasks-template.md`) | Copy verbatim. These are the authoring templates for spec-spine adopters. | (not in S2 explicitly; supports `init.md:6` self-extending mechanism) | None. |
| `.claude/rules/orchestrator-rules.md` | Copy verbatim. Six rules are entirely generic. | S4 (generic) | None. |
| `.claude/rules/governed-artifact-reads.md` | Copy as template; consumer-binary table specialized: drop `oap-registry-enrich` row, drop OAP-specific subcommands. | S4 (hybrid) | D-1 (render placement) and D-2.10 (rule-table staleness) shape the table. |
| `.claude/rules/adversarial-prompt-refusal.md` | Copy as template; "Worked precedents" section specialized per adopter, or omitted in the template version. | S4 (hybrid) | None — generic rule + project-filled precedents is a clean templating boundary. |
| `specs/000-bootstrap-spec-system/spec.md` | Copy verbatim as the constitutional baseline. | (out of S2 scope but implicit) | None. Required by precedence (`constitution.md:7`). |
| `specs/000-bootstrap-spec-system/contracts/registry.schema.json` | Copy verbatim. Defines `specVersion` and the deterministic registry shape. | S6 spec-format | None. |
| `specs/000-bootstrap-spec-system/contracts/build-meta.schema.json` | Copy verbatim. Defines the ephemeral compiler metadata shape. | S6 spec-format | None. |
| `schemas/codebase-index.schema.json` | Copy verbatim. Generic spec-spine codebase-index shape. | S6 spec-format | D-3 (schema location: today schemas are split between `schemas/` and `specs/000-.../contracts/`; an adopter benefits from one canonical location). |
| `schemas/codebase-index-oap.schema.json` | Do **not** copy; this is the OAP-overlay schema. | — | D-3. |
| `schemas/agent-frontmatter.schema.json`, `schemas/skill-frontmatter.schema.json` | Generic or hybrid — neither is examined deeply in this trace. | — | D-4 (frontmatter schemas: generic spec-spine concept or OAP-specific?). Surface for decision. |

### Binary toolchain

| Binary | Adopter action | Classification cross-ref | Open question |
|---|---|---|---|
| `spec-compiler` | Install (Cargo build from extracted source, or distribute prebuilt). Required to produce `build/spec-registry/registry.json`. | implied by S2 reads of `build/spec-registry/` | None. |
| `registry-consumer` | Install. Required by `/init` Step 1 lifecycle/id reads. | S3 (generic, high) | None. |
| `codebase-indexer` | Install. Required by `/init` Step 1 staleness check. | S3 (generic, high) | D-1 — does `codebase-indexer render` come with it (Option 1 or 3) or not (Option 2)? |
| `spec-lint` | Install. Conformance lint. Not invoked by `/init` directly but ships with the spec-spine bundle and is referenced in convention guidance. | not in S3 (not on `/init` path) | None. |
| `policy-compiler` | Install. Compiles governance policies (CONST-001..005). Not invoked by `/init` but referenced by `CLAUDE.md` policy block. | not in S3 | Whether it ships in the minimum bundle or as an optional add-on. |
| `oap-code-index-enrich` | Do **not** ship in the generic bundle. OAP-specific overlay. | S3 (OAP-specific, high) | D-1 resolution determines the generic substitute. |
| `oap-registry-enrich` | Do **not** ship in the generic bundle. OAP-specific overlay. | implied by `governed-artifact-reads.md:16` | None. |
| `git` | Pre-existing on any developer host. | S3 (generic) | None. |

### Directory layout conventions

| Convention | Adopter action | Cross-ref | Notes |
|---|---|---|---|
| `specs/NNN-kebab-case/spec.md` | Follow convention. The `NNN` must match `id` in frontmatter. | `constitution.md:37`, `.specify/contract.md:22` | Generic spec-spine. |
| `.specify/memory/` | Follow convention. At least `constitution.md`; other files are adopter choice. | `init.md:11-12`, S2 | Generic. |
| `.specify/templates/` | Follow convention. Authoring templates live here. | `AGENTS.md:n/a` (referenced by spec 000 implicitly) | Generic. |
| `.specify/contract.md` | Follow convention. Short normative summary of the adopter's Feature 000 instance. | S2 (generic) | Generic. |
| `build/spec-registry/` (output of `spec-compiler`) | Follow convention. Holds `registry.json` and `build-meta.json`. | `constitution.md:38` | Generic. |
| `build/codebase-index/` (output of `codebase-indexer`) | Follow convention. Holds `index.json` and (per D-1) optionally `CODEBASE-INDEX.md`. | `governed-artifact-reads.md:13-18` | Generic core, possible D-1 wrinkle. |
| `.claude/commands/`, `.claude/rules/`, `.claude/agents/` | Follow convention. Hold the slash-command, rule, and agent definitions. | `AGENTS.md`, `init.md:6` | Generic convention; OAP-specific population. |
| `tools/`, `apps/`, `docs/` | Optional. `tools/` is the conventional toolchain location; `apps/` and `docs/` are project-shape choices. | S2 | `tools/` is conventional for the spec-spine; `apps/` and `docs/` are not enforced. |

### Spec format definition reference

| Item | Adopter action | Notes |
|---|---|---|
| `specs/000-bootstrap-spec-system/spec.md` | Copy verbatim. Constitutional baseline; overrides constitution.md per precedence. | `constitution.md:7`. |
| Grammar locus: markdown body + YAML frontmatter only; no standalone YAML | Follow convention (invariant V-004 of Feature 000). | `constitution.md:17`, `.specify/contract.md:8-13`. |
| Schema: `registry.schema.json` | Defines deterministic `registry.json` shape and `specVersion`. | `specs/000-bootstrap-spec-system/contracts/registry.schema.json`. |
| Schema: `build-meta.schema.json` | Defines ephemeral `build-meta.json` shape. Not part of golden-file determinism checks. | `specs/000-bootstrap-spec-system/contracts/build-meta.schema.json`. |
| Schema: `codebase-index.schema.json` | Defines the generic codebase-index shape. | `schemas/codebase-index.schema.json`. |
| SemVer policy | `specVersion` is the version field on the registry; bumps follow the policy described in Feature 000. | `.specify/contract.md` (final paragraph references registry-consumer contract governance). |
| Amendment / supersession convention | `amends:`/`amended:`/`amendment_record:` vs `status: superseded`/`superseded_by:` | `.specify/contract.md:28`. |

---

## Open decisions

Consolidated. Every low-confidence item and every surfaced decision in one place. Each entry: one-line decision + options + one-line trade-offs. No recommendations.

| ID | Decision | Options | Trade-offs |
|---|---|---|---|
| D-1 | Render-path placement for `codebase-indexer` vs `oap-code-index-enrich`. | (1) Return `render` to `codebase-indexer` (partial undo of W-07b). (2) Spec-spine `/init` skips render; markdown is OAP-side enrichment. (3) Generic render template + OAP column adapters. | (1) restores generic summary; needs decomposition of W-07b. (2) thinnest extraction; adopters lose markdown view in `/init`. (3) largest design surface; new enricher contract. |
| D-2.1 | Rules pre-load canonical list. | (a) `AGENTS.md` Step 0 lists all three rule files. (b) `AGENTS.md` lists two, `CLAUDE.md` mentions the third as "automatic." (c) `init.md` lists rules explicitly. | (a) single source of truth; verbose Step 0. (b) preserves current split; perpetuates drift. (c) couples the executor to the rule list; loses self-extending discipline. |
| D-2.2 | Structural-index read path in `init.md`. | (a) Rewrite `init.md:30` to use governed consumers. (b) Treat the line as deliberate and amend spec 103 to permit `/init` direct read. (c) Remove the line; rely on AGENTS.md hook. | (a) restores spec 103 compliance. (b) weakens spec 103. (c) cleanest; loses redundancy. |
| D-2.3 | Identity-read asymmetry (`init.md` reads AGENTS.md; AGENTS.md does not). | (a) Add `AGENTS.md` to its own "New Sessions" identity list. (b) Document the implicit-protocol-source read in a note. (c) Drop the read from `init.md`. | (a) self-referential. (b) clarifies without changing reads. (c) loses orientation context. |
| D-2.4 | `.specify/contract.md` membership in canonical identity-read list. | (a) Add to AGENTS.md. (b) Drop from `init.md`. (c) Keep asymmetry. | (a) aligns sources; one more file to load. (b) loses orientation. (c) preserves drift. |
| D-2.5 | Spec-list path: `ls specs/` vs `registry-consumer list --ids-only`. | (a) Use consumer. (b) Use `ls`. (c) Use both. | (a) typed surface, aligns with spec 103 spirit. (b) cheaper, less governed. (c) redundant. |
| D-2.6 | Lifecycle counts in `/init` summary. | (a) Add `registry-consumer status-report` call to `init.md` and surface counts in the summary template. (b) Drop from `AGENTS.md`. | (a) richer summary; more tool calls. (b) matches current `init.md` template; loses lifecycle visibility. |
| D-2.7 | Git log verb count (`-15` vs `-10`). | (a) `-15`. (b) `-10`. (c) Configurable. | Cosmetic. (c) over-engineered. |
| D-2.8 | Memory load: place in `AGENTS.md` "New Sessions" or implicit. | (a) Add Step 0' to `AGENTS.md`. (b) Document as implicit pre-step. | (a) single source. (b) preserves AGENTS.md focus on additive hooks. |
| D-2.9 | OAP-specific listings (`ls tools/`, `ls apps/`, `ls docs/`) in `init.md`. | (a) Move to AGENTS.md "New Sessions" hook. (b) Drop entirely. (c) Keep. | (a) cleanest extraction; uses the self-extending mechanism. (b) loses orientation. (c) perpetuates hybrid `init.md`. |
| D-2.10 | Consumer-binary table in `governed-artifact-reads.md` stale on render. | (a) Update table to reflect post-W-07b state. (b) Restore W-07b. | (a) honest. (b) reverses a deliberate move. Tied to D-1. |
| D-3 | Schema location: `schemas/` vs `specs/000-.../contracts/`. | (a) Unified `schemas/` directory. (b) Keep split; document the boundary. | (a) one canonical place; migration cost. (b) zero migration; perpetuates discoverability split. |
| D-4 | `agent-frontmatter.schema.json` and `skill-frontmatter.schema.json` classification. | (a) Generic (ship in standard). (b) OAP-specific (do not ship). (c) Hybrid (ship a thin generic core; OAP overlay). | (a) more bundled. (b) smaller standard. (c) needs an explicit contract decomposition. |
| D-5 | `CODEBASE-INDEX.md` content header. | (a) Adopt a project-name-templated header. (b) Strip the project name. (c) Leave the OAP-branded header in the generic render. | (a) clean adoption story; needs render template change. (b) faceless. (c) generic-in-name-only. |

---

## Halt-check (per prompt directive)

The trace did not encounter a structural coupling that cannot be templated cleanly. Every OAP-specific element on the `/init` path is either (a) addressable by a template slot (`AGENTS.md`, `CLAUDE.md`, the worked-precedents subsection of `adversarial-prompt-refusal.md`, the constitution amendment metadata), or (b) decomposable through D-1 (render placement). The standalone premise holds. Deliverable is complete.
