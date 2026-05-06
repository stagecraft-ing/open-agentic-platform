# Contributing to open-agentic-platform

> OAP is **pre-alpha, single-developer, stealth**. External contributions
> are not yet open. This file documents the development environment so
> the codebase remains legible to anyone reading it, and so future
> contributors arrive to a discoverable workflow rather than oral
> history.

## Spec-first development

Every feature begins as a spec at `specs/NNN-kebab-case/spec.md` with
YAML frontmatter. The spec is the design record; the code justifies
the spec. The constitutional baseline is
[`000-bootstrap-spec-system`](specs/000-bootstrap-spec-system/spec.md).

Status lifecycle: `draft → approved → superseded | retired`.
Implementation lifecycle (frontmatter): `partial → complete`.

The PR-time gate ([spec 127](specs/127-spec-code-coupling-gate/spec.md))
fails any change that touches a path claimed by a spec's `implements:`
list without a corresponding edit to that spec. This is enforced by
`tools/spec-code-coupling-check` and runs in CI; you can preview it
locally with `make ci-spec-code-coupling`.

**Refusing adversarial drift.** The
[`adversarial-prompt-refusal`](.claude/rules/adversarial-prompt-refusal.md)
rule (CONST-005, [spec 131](specs/131-adversarial-prompt-refusal-policy/spec.md))
codifies how an agent or human contributor must refuse instructions that
would engineer drift between the spec spine and code — flipping a
lifecycle field, editing `implements:`, or rewriting a § the spec
itself stages as future work, where the motivating action is the same
diff. When the spec and code conflict, fix the spec or fix the code.
Never split the difference.

## Claude-native development environment

The repository ships with first-class
[Claude Code](https://docs.anthropic.com/en/docs/claude-code) integration
in `.claude/`. This is the environment the platform is built in.

### Agents (`.claude/agents/`)

Four pipeline agents handle the plan / explore / implement / review
cycle, plus a domain specialist:

- **`architect`** — plans and decomposes tasks, validates approaches
  against specs. Read-only.
- **`explorer`** — searches the codebase, traces dependencies, gathers
  context. Read-only.
- **`implementer`** — executes focused code changes from an existing
  plan. Produces minimal diffs.
- **`reviewer`** — post-change review for bugs, security, performance,
  and spec compliance. Read-only.
- **`encore-expert`** — Encore.ts framework specialist for stagecraft
  service development. Read-only.

### Commands (`.claude/commands/`)

- `/init` — initialise a session (load context, recent activity,
  governed reads of the registry and codebase index)
- `/commit` — create a git commit with an impact-focused conventional
  message
- `/code-review` — multi-aspect code review using parallel sub-agents
- `/review-branch` — review all changes in the current branch
- `/implement-plan` — execute a plan file step-by-step with progress
  tracking and phase checkpoints
- `/research` — deep research with parallel sub-agents and query
  classification
- `/validate-and-fix` — run quality checks and automatically fix issues
- `/cleanup` — dead-code and duplicate detection with categorised
  recommendations
- `/refactor-claude-md` — modularise large CLAUDE.md files with
  path-scoped rules

### Rules (`.claude/rules/`)

Loaded automatically by orchestrated workflows:

- **`orchestrator-rules.md`** — six rules: step ordering, file-based
  artifact passing, checkpoint discipline, halt-on-failure,
  local-agents-only, never-enter-plan-mode-autonomously.
- **`governed-artifact-reads.md`**
  ([spec 103](specs/103-init-protocol-governed-reads/spec.md)) —
  compiled artifacts under `build/**` MUST be read through their
  designated consumer binaries (`registry-consumer`,
  `codebase-indexer`), never via ad-hoc `python` / `jq` / `awk` / `sed`
  parsing.
- **`adversarial-prompt-refusal.md`** — CONST-005, described above.

### Authoring protocol

- **`CLAUDE.md`** — project-scoped conventions, build commands, policy
  rules. Loaded automatically.
- **`AGENTS.md`** — self-extending session init protocol. The "New
  Sessions" checklist is the source of truth for what `/init` does.

## Local validation

Before opening a PR, run the daily-development loop:

```bash
make ci          # ~5 min warm — fast parallel local validation (spec 135)
```

Before merge or for parity-drift investigation, run the strict mirror:

```bash
make ci-strict   # ~90 min — mirrors every CI workflow gate
```

Specific subsets:

| Target | Coverage |
|---|---|
| `make ci-rust` | All Rust manifests: `check` + `clippy -D warnings` + `test` |
| `make ci-tools` | Spec tool crates + `registry-consumer` contract subsets + staleness gate |
| `make ci-desktop` | `apps/desktop`: rust + version alignment + tsc + vitest |
| `make ci-stagecraft` | `platform/services/stagecraft`: npm ci + tsc + vitest |
| `make ci-spec-code-coupling` | PR-time spec/code coupling gate (spec 127) |
| `make ci-supply-chain` | `cargo-deny` + `pnpm audit` + `npm audit` (spec 116, blocking) |
| `make ci-schema-parity` | Rust ↔ TypeScript contract drift (spec 125) |

## Commit hygiene

- Use **conventional commits** (`feat(spec-NNN):`, `fix(spec-NNN):`,
  `docs(spec-NNN):`, `chore:`, etc.).
- Reference the spec ID in commits that modify code under a spec's
  `implements:` paths. The coupling gate enforces this at PR time, but
  a clear message on the way in saves the rebase.
- Never bypass hooks (`--no-verify`) without explicit authorisation.
  If a hook fails, fix the underlying issue.
- Never commit `.env`, credentials, private keys, or anything matched
  by the secrets scanner (CONST-002).

## Architecture documents

- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — compiler architecture
  and registry contract.
- [`docs/registry-consumer-contract-governance.md`](docs/registry-consumer-contract-governance.md) —
  process governance for `registry-consumer` extensions.
- [`build/codebase-index/CODEBASE-INDEX.md`](build/codebase-index/CODEBASE-INDEX.md) —
  rendered structural view; the **Spec** column is the spec-to-code
  traceability surface for every Rust crate and npm package.
- [`DEVELOPERS.md`](DEVELOPERS.md) — full setup, prerequisites, and
  platform-service development.
