# Tooling inventory — README rewrite (Phase 0)

> Captured 2026-05-06 during the May 12 launch README rewrite. Records what
> agentic-first tooling existed and what gaps the platform's own tooling
> exposed when an agent dogfooded it for an authoring task.

## What was found

### CLIs (`tools/`)

| Tool | Use |
|---|---|
| `tools/spec-compiler` | Compiles `specs/**/spec.md` → `build/spec-registry/registry.json`. Used via `make setup`/`make spec-compile`. |
| `tools/registry-consumer` | Read-only CLI over the compiled registry. **Used heavily**: `list`, `show <id>`, `status-report --json --nonzero-only`, `compliance-report --framework owasp-asi-2026 --json`. |
| `tools/spec-lint` | Conformance linter, runs in CI (`--fail-on-warn` since spec 128). |
| `tools/codebase-indexer` | Compiles `build/codebase-index/index.json` and renders `CODEBASE-INDEX.md`. **Used**: `check`, `render`. The rendered markdown was the structural source of truth for the rewrite. |
| `tools/spec-code-coupling-check` | PR-time gate (spec 127): code paths claimed by `implements:` must change with the spec. |
| `tools/policy-compiler` | Compiles policy rules. Not exercised by the rewrite. |
| `tools/stakeholder-doc-lint`, `tools/assumption-cascade-check`, `tools/ci-parity-check`, `tools/schema-parity-check`, `tools/adapter-scopes-compiler` | CI-only or specialised; not used for this task. |

### Claude commands (`.claude/commands/`)

`/init`, `/commit`, `/code-review`, `/review-branch`, `/implement-plan`,
`/research`, `/validate-and-fix`, `/cleanup`, `/refactor-claude-md`. **Used:**
`/init` (session bootstrap). `/research` would have been a candidate for the
Phase 1 investigation but the structured investigation surface here was small
enough (5 named questions) that a single `Explore` sub-agent was a tighter fit
than the multi-agent research orchestration `/research` invokes.

### Claude agents (`.claude/agents/`)

`architect`, `explorer`, `implementer`, `reviewer`, `encore-expert`. **Used:**
`Explore` (built-in; the project's `explorer` agent is path-equivalent for the
purposes of this task).

### Makefile

`make setup`, `make ci`, `make ci-strict`, `make registry`, `make index`,
`make deploy-{azure,aws,hetzner}`, `make dev`, `make dev-platform`. **Used:**
none directly during this task — the relevant artifacts (`registry.json`,
`CODEBASE-INDEX.md`) were already built. `make registry` is the canonical
recompile entry point and is referenced in the README quickstart.

### Authoring protocol

`CLAUDE.md` (project conventions), `AGENTS.md` ("New Sessions" init protocol),
`.claude/rules/orchestrator-rules.md`, `.claude/rules/governed-artifact-reads.md`,
`.claude/rules/adversarial-prompt-refusal.md` (CONST-005). The
governed-artifact-reads rule is the reason this task did not parse
`build/**/*.json` directly — every machine-truth read went through
`registry-consumer` or the rendered codebase index.

`scripts/` does not exist (deliberately — spec 105 migrated scripts to binaries).

## What worked well for an agentic README rewrite

1. **`registry-consumer compliance-report --json`** is the load-bearing demo.
   It emits a real, structured OWASP-ASI-2026-to-spec mapping today. The
   README quickstart section uses it as-is.
2. **`registry-consumer status-report --json --nonzero-only`** gave the spec
   lifecycle counts (137 approved, 1 draft, 4 superseded) without a single
   spec read. The spec corpus is 142 specs and not one was opened to count.
3. **`build/codebase-index/CODEBASE-INDEX.md`** answered "which crate
   implements which spec" for every claim in the README. The Spec column is
   the spec/code traceability surface a reader actually wants.
4. **`registry-consumer show <id>`** answered "what does spec 102 actually
   contract?" without opening the spec file — frontmatter only, structured.

## What did not work / where the agent had to fall back

### Gap T-1 — Adapter status is not in the spec spine

The README needs to position **`aim-vue-node` as the production-supported
adapter** and the other three (`next-prisma`, `rust-axum`, `encore-react`)
as factory-contract validators. **No metadata in `registry.json`,
`CODEBASE-INDEX.md`, or any spec carries this distinction.** The Explorer
agent confirmed: `oapNativeAdapters.ts` treats all four equally.

The agent fell back to merge-history concentration — specs 138, 140, 141 are
all aim-vue-node-specific, indicating it's where active work lands — but
this is inference, not a governed claim. Recorded in `gaps.md` as G-1 and
flagged in the README's Adapters section honestly.

**Roadmap implication:** add an `adapter_status: production | validator |
roadmap` field to whatever spec owns the adapter manifest (currently spec
108 / spec 139), so the README and any future stakeholder-facing surface
can read it from machine truth.

### Gap T-2 — Governance certificate emission has no run command

The README's "Try it" section was supposed to demo a real governance
certificate. Spec 102's central deliverable is "a single JSON artifact
produced at the end of every successful factory pipeline run." The schema,
the verifier binary (`crates/factory-engine/src/bin/verify_certificate.rs`),
and 5 unit tests exist. **`factory_run.rs` does not call certificate
emission.** No `make` target produces one. No `cargo run` invocation from a
fresh checkout produces one.

The agent fell back to demoing the **traceability/compliance report**
instead — which is real and works — and explicitly flagged the certificate
emission gap in `gaps.md` as G-2 (severity: critical for May 12 launch).

**Roadmap implication:** the spec carries `implementation: complete`, but
the load-bearing demo it promises is not wired. Either the wiring lands
before launch or spec 102's `implementation` field needs to flip back to
`partial` (CONST-005: spec must match code).

### Gap T-3 — Multi-cloud framing has no governed surface

The README needs to claim multi-cloud as procurement optionality. The
Explorer agent found Azure is fully wired, AWS/GCP/DO are
modules-without-environments, Hetzner is operational on a separate path.
This is true today, and the README states it honestly. But there's no
governed artifact that asserts which clouds have working `envs/`. A reader
has to inspect `platform/infra/terraform/envs/` and the `platform/Makefile`
to know.

**Roadmap implication:** a `make deploy-status` target (or
`registry-consumer cloud-status`) would let the README link to the
authoritative list rather than asserting it inline.

### Gap T-4 — `compliance-report` coverage is shallow

`compliance-report --framework owasp-asi-2026 --json` returns mappings for
ASI01, ASI03, ASI05, ASI07, ASI09, ASI10 — all to a single spec
(`102-governed-excellence`). Even-numbered ASI controls (ASI02, ASI04,
ASI06, ASI08) are unmapped. The CLI works; the corpus it queries is thin.

**Roadmap implication:** add `compliance:` frontmatter to specs that
materially address ASI02/04/06/08 (e.g., supply-chain → ASI04 already
covered by spec 116; this is content authoring, not tooling).

## Net assessment

The platform's authoring tooling was sufficient to write this README without
opening any spec file or parsing any build artifact directly. That itself is
a finding — agentic dogfooding works. The four gaps above are content/wiring
gaps, not tooling gaps. The README rewrite proceeded under the
governed-artifact-reads rule (spec 103) without exception.
