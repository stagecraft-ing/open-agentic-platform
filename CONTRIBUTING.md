# Contributing to Open Agentic Platform

Open Agentic Platform is an AI-native governed delivery system. Contributions of all kinds are welcome: bug reports, spec proposals, code, documentation, and tooling improvements.

This is a spec-first project built by a single developer using AI-assisted development. The `.claude/` directory ships Claude Code agents, commands, and rules — these are first-class development tools, not optional extras.

---

## Getting Started

See `DEVELOPERS.md` for prerequisites (Rust, Node, pnpm, Docker, Encore CLI) and environment setup.

```bash
make setup       # install deps, build tools, verify environment
make check-deps  # verify core tools are installed
```

---

## Spec-First Development

Every feature starts as a spec. Before writing any code, write a spec in `specs/NNN-slug/spec.md` with YAML frontmatter.

Specs are the source of truth. They drive the registry, the compiler, and the agents. The constitutional bootstrap spec is at `specs/000-bootstrap-spec-system/spec.md` — read it first to understand the system's own design contract.

```yaml
# Minimal spec frontmatter
id: "NNN-short-feature-name"
title: "Human-readable feature title"
status: "draft"          # draft | active | superseded | retired
created: "YYYY-MM-DD"
summary: "One-line description of the feature"
```

Run the spec compiler after any spec change:

```bash
cargo build --release --manifest-path tools/spec-compiler/Cargo.toml
./tools/spec-compiler/target/release/spec-compiler compile
```

---

## Development Workflow

1. Fork the repository and create a branch from `main`.
2. Write or update the spec for your change if it introduces new behavior.
3. Implement the change following the conventions below.
4. Open a pull request with one logical change. Split unrelated concerns across separate PRs.

### Commit conventions

Use conventional commits:

```
feat(scope):     new capability
fix(scope):      bug fix
refactor:        code restructure, no behavior change
docs:            documentation only
chore:           build, deps, tooling
test:            tests only
```

The `scope` is the crate or subsystem name (e.g., `orchestrator`, `factory-engine`, `spec-compiler`, `desktop`).

---

## Claude Code Workflows

Contributors using Claude Code get first-class tooling via the `.claude/` directory:

**Slash commands** (`.claude/commands/`):
- `/init` — orient to the codebase and load project context
- `/commit` — structured commit with conventional message
- `/code-review` — review staged or branch changes
- `/validate-and-fix` — run checks, surface failures, apply fixes

**Agents** (`.claude/agents/`):
- `architect` — design, spec review, technical decisions
- `explorer` — codebase research and impact analysis
- `implementer` — focused code changes following a spec or plan
- `reviewer` — code review and conformance checks

**Rules** (`.claude/rules/`):
- `orchestrator-rules.md` — governs multi-step agent workflows (order, checkpoints, halt-on-failure)

These agents and commands are designed to work with the spec-first methodology. The implementer agent, for example, will read the relevant spec before making any change.

---

## Code Conventions

**Languages:**
- Rust for all CLI tools (`tools/`) and library crates (`crates/`)
- TypeScript + React for the desktop app (`apps/desktop/`, Tauri v2)
- TypeScript + Encore.ts for platform services (`platform/services/stagecraft/`)

**Rust:**
- Zero clippy warnings — `cargo clippy -- -D warnings` must pass
- Use `thiserror` for error types
- Keep `pub` surface minimal
- Build: `cargo build --manifest-path <path>/Cargo.toml`

**TypeScript:**
- Follow existing component and hook patterns in `apps/desktop/`
- `platform/services/stagecraft/` uses `npm`, not `pnpm`

**License headers:**
All source files must include the AGPL-3.0 license header. Check existing files for the format.

---

## CI and Review

CI checks that must pass before merge:
- `cargo clippy -- -D warnings` (no warnings)
- `cargo test` (all tests pass)
- Spec compiler produces a valid `build/spec-registry/registry.json`
- Spec lint (`tools/spec-lint/`) reports no errors

At least one maintainer review is required before any PR is merged.

---

## Reporting Issues

Open a GitHub issue for bugs, feature requests, or spec proposals. For security issues, see `SECURITY.md` — do not open a public issue.
