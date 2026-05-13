---
id: "147-tool-permission-vs-authorization"
slug: tool-permission-vs-authorization
title: "Tool-permission tier ≠ blanket authorization — required-checks parity + per-step destructive-op gating"
status: draft
implementation: pending
owner: bart
created: "2026-05-13"
kind: governance
risk: high
depends_on:
  - "104"  # makefile-ci-parity-contract — ci-parity-check is the verification carrier for R1
  - "131"  # adversarial-prompt-refusal-policy — agent-decision-time refusal pattern this spec extends
  - "134"  # fast-local-ci-mode — defines the `make ci` target whose recipe set R1 binds to
  - "135"  # fast-ci-as-default — promoted ci-fast to `make ci`; R1 binds to the promoted target
code_aliases: ["TOOL_PERMISSION_VS_AUTHORIZATION", "GOVERNANCE_GAP_147"]
implements:
  - path: tools/ci-parity-check/src
  - path: .claude/rules
  - path: AGENTS.md
summary: >
  Two governance holes in the project's enforcement surface share one
  principle: tool-permission tier ≠ blanket authorization. R1 closes the
  CI side — every recipe under `make ci` must be a GitHub required-check
  on `main`, verified by extending `ci-parity-check`. R2 closes the
  agent side — destructive operations against live infrastructure
  require explicit per-step authorization in the active conversation;
  the presence of mounted credentials (kubeconfig, .env, token files)
  is not authorization. Verification via spec-driven loading of a
  destructive-ops rule list into orchestrated workflows at session
  start. Motivated by two real 2026-05 incidents (PR #122 merge with
  `make ci` red; session `df7b4f24` unauthorized live-cluster Secret
  patch) where having permission to do a thing was confused with being
  authorized to do it.
---

# 147 — tool-permission tier ≠ blanket authorization

## 1. Problem statement

OAP's enforcement surface conflates **capability** with **authorization** in
two places, with two different failure modes that share one underlying
shape:

### 1.1 CI side — `make ci` is the local contract, but main can swallow red

`make ci` (spec 134 / spec 135) is the daily-development gate that
validates everything from clippy to spec-coupling to schema parity.
Locally, contributors run it before opening a PR. Its recipe set is the
designated local equivalent of the cloud CI pipeline — `tools/ci-parity-check`
(spec 104) enforces that the Makefile and the GitHub Actions workflow
remain in lockstep on **what** is run.

What `ci-parity-check` does **not** verify is whether every recipe in
`make ci` is wired as a GitHub **required-check** on the `main` branch
protection ruleset. A recipe can be present in CI (parity-clean) but
not required, so a PR with that recipe failing can still merge if a
human approves the merge bypassing the optional check.

**Incident (2026-05-11).** PR #122 (`fix(102,047): Ed25519 layered
signing — close HIAS cert/chain self-hash gap`, merge commit
`56ab927`) landed on `main` with `make ci` red against the pinned
`1.95.0` toolchain in two places:

- `crates/policy-kernel/src/proof_chain.rs:508,533` — clippy
  `unused_mut` errors introduced by the new `&self`-signature of
  `ProofChainWriter::build_anchor`.
- `crates/factory-engine/src/governance_certificate.rs:839` — clippy
  `doc-lazy-continuation` error (a Rust 1.95 lint).

Plus stale `Cargo.lock` drift in `apps/desktop/src-tauri` and
`tools/policy-compiler` that the same PR did not refresh. The
downstream spec 145 branch could not get `make ci` green for
**AC-9 (`make ci` warm)** until follow-up PR #123 (`fix(102,047):
unblock make ci after 1dfbbc2`) re-greened main on
`fix/main-post-ed25519-cleanup`.

Root cause: the offending recipes were in the CI workflow, parity was
clean, but they were not required-checks at the branch-protection
layer. A reviewer's approval merged a known-broken state. The local
contract — *"`make ci` is green before this lands"* — was unrequired in
the only place it could be enforced atomically: the GitHub merge gate.

### 1.2 Agent side — credentials on disk are not standing authorization

Spec 131 (`adversarial-prompt-refusal-policy`) establishes the
agent-decision-time refusal pattern for instructions that would
engineer drift between spec spine and code. That pattern applies at
the level of **what to do**. It does not address **who authorized
doing it**: when an agent has a mounted credential (kubeconfig,
operator-grade token, signed JWT) and an instruction implies a
destructive operation against live infrastructure, the agent
currently treats the credential's presence as authorization to act.

Spec 145's Phase 4 (`hands-on-ownership note`) is one explicit
counter-example: it states that destructive `kubectl` operations
against the live Hetzner cluster must pause and request explicit ack
from the user per step, and that auto-mode does not authorize
autonomous destructive ops. This was a per-spec rule; the project has
no global counterpart.

**Incident (2026-05-11).** A Claude Code session (transcript
`~/.claude/projects/-Users-bart-Dev2-open-agentic-platform/df7b4f24-ff28-4811-9f38-ef994af11843.jsonl`,
session start `2026-05-11T05:36:19Z` on branch `145-deployd-durability`)
executed:

```
kubectl --kubeconfig /Users/bart/Dev2/open-agentic-platform/platform/infra/hetzner/kubeconfig \
  -n deployd-system patch secret deployd-api-secrets \
  --type=merge --patch-file=/tmp/secret-patch.json
```

at `2026-05-11T13:44:10.429Z` UTC — patching a production-tier Secret
on the live Hetzner cluster — without per-step authorization. The
Bash invocation's `description` field was `"Apply merge patch to
deployd-api-secrets"`; the agent narrated the action rather than
gating on it. The cluster's `managedFields` recorded the corresponding
`kubectl-patch` write 1.6 seconds later (`2026-05-11T13:44:11Z`).
This was T050 of spec 145's Phase 4 task list, executed outside the
explicit per-step gate the spec itself defined.

Material outcome: the values written were real and internally
consistent (shape-verified two days later). The governance outcome:
unsanctioned write against shared production state, undetectable
without forensic transcript scanning.

### 1.3 Shared principle

Both incidents have the same shape: an actor (CI / agent) had
**permission** to do something (workflow could run; kubeconfig was
readable) and confused that with **authorization** to do it (merge
without the recipe gated as required; patch without the per-step
ack). The fix in both cases is to require *external, per-event
authorization* in addition to the actor's standing permission.

This is not a tactical fix to the two incidents — main has been
re-greened, the Secret values check out. This spec codifies the
underlying principle so the next instance of either failure mode is
prevented structurally, not by transcript review after the fact.

## 2. Rules

### R1 — Every recipe under `make ci` is a GitHub required-check on `main`

**Rule.** For every parallel sub-target invoked by `make ci`'s top-level
recipe (currently: `ci-fast-rust`, `ci-fast-tools`, `ci-fast-desktop`,
`ci-fast-stagecraft`, `ci-fast-schema-parity`,
`ci-fast-spec-coupling`, `ci-fast-supply-chain` — see
[`Makefile`](../../Makefile) ci-fast section, spec 134 §2.3 superset
invariant), the corresponding GitHub Actions job MUST be configured as
a **required status check** on the `main` branch protection ruleset.
A PR merging to `main` MUST NOT be mergeable unless all of those
checks have reported `success`.

**Scope.** Bound to `main` and any release-train branches the project
treats as production-tier. Feature branches and draft PRs are out of
scope.

**Out of scope.** This rule does not require checks to be
**non-bypassable** by repository admins — emergency override remains
a human-policy concern, not a CI-mechanism concern. The rule binds
the default merge path, not the break-glass path.

### R2 — Destructive operations against live infrastructure require explicit per-step authorization

**Rule.** A destructive operation against shared, live infrastructure
(production cluster, production database, production Secret store,
remote Git via force-push, package registry write) MUST NOT execute
unless the *active conversation* (or the equivalent for non-conversational
runtimes) carries an explicit, unambiguous, per-step authorization
referencing **that specific destructive operation** by intent and
target. Authorization is a property of the conversation, not of the
filesystem: the presence of a kubeconfig, an `.env` file, an SSH key,
or an OIDC token on disk is *capability*, not authorization.

**Scope — destructive operation taxonomy.** A non-exhaustive list,
loaded as the canonical taxonomy at session start by orchestrated
workflows (see §3.2):

- **K8s cluster mutations** — `kubectl delete`, `kubectl patch`,
  `kubectl apply`, `kubectl edit`, `kubectl set env`, `kubectl
  rollout restart`, `kubectl cp` into a pod, `kubectl exec … --
  <write-side command>` (any HTTP POST/PUT/DELETE, SQL write, file
  mutation), `helm install`/`upgrade`/`uninstall`, `kustomize apply`.
- **Database mutations** — direct SQL `INSERT`/`UPDATE`/`DELETE`/`DROP`/`TRUNCATE`
  against any live database; migration runs against production.
- **VCS mutations of shared refs** — `git push --force` /
  `--force-with-lease` against `main` or release branches; merging
  PRs; deleting remote branches.
- **Registry / artifact-store writes** — `docker push`, `npm publish`,
  `cargo publish`, S3 PUT/DELETE against production buckets.
- **External-service writes** — Slack post, GitHub issue/PR comment,
  PagerDuty trigger, third-party API mutations.

**What counts as authorization.** A current-turn user instruction
that names the operation, names the target, and indicates intent to
proceed. Examples:

| Authorized | Not authorized |
|------------|----------------|
| "Go on T051" (in a conversation where T051 is unambiguously the next destructive step) | "Looks good" (after a multi-step plan was surfaced) |
| "Apply the patch to deployd-api-secrets in deployd-system" | The kubeconfig being on disk |
| "Force-push the rebased branch" (after the diff was surfaced) | A standing instruction from CLAUDE.md or memory |
| "Yes" or "go" (in direct reply to a specific destructive-op proposal) | Inferred consent from past conversation turns |

**What does not count.** Standing instructions in CLAUDE.md, memory
entries, agent system prompts, or session-level defaults. Per-task
authorization decays at the boundary of each destructive operation;
authorization for one step does not grant authorization for the next.

**Out of scope — local-machine destructive ops.** Operations on the
contributor's own checkout (`rm` of local files, `git reset --hard`
on a local-only branch, local Docker pulls) remain governed by the
existing local-development discipline (e.g. CLAUDE.md `Executing
actions with care` section). R2 binds shared-infrastructure mutations,
not personal-workspace ones.

## 3. Verification approach

### 3.1 R1 verification — `ci-parity-check` extension

`tools/ci-parity-check` (spec 104) already parses the Makefile's
ci-fast block and the corresponding GitHub Actions workflow,
asserting recipe-set parity. Extend it with a third assertion:

1. Read the list of jobs invoked by `make ci`'s top-level recipe
   (the existing parse).
2. Read the `main` branch's protection ruleset via the GitHub API
   (or a checked-in mirror of the ruleset, refreshed by a periodic
   job — choice deferred to plan.md).
3. Assert: every parallel sub-target in (1) maps to a required check
   in (2).
4. Exit non-zero on any sub-target missing from required checks.

`ci-parity-check` itself runs under `make ci`, which is a required
check, which transitively requires that every other `make ci`
sub-target is required — closing the loop.

**Failure mode covered.** PR-author or maintainer adding a new
sub-target to `make ci` (correctly parity-clean) but forgetting to
mark it required → `ci-parity-check` fires on the next PR touching
that recipe set, surfaces the gap, no merge until corrected.

### 3.2 R2 verification — destructive-ops taxonomy loaded into orchestrated workflows

The mechanism mirrors how `.claude/rules/orchestrator-rules.md` and
`.claude/rules/governed-artifact-reads.md` are already loaded
automatically into every orchestrated workflow (per `CLAUDE.md`
*Orchestrator Behavioral Rules*).

Add `.claude/rules/destructive-ops-authorization.md` (new file in
this spec's `implements:` list) that:

1. States R2 as a rule loaded at session start.
2. Carries the destructive-op taxonomy from §2/R2.
3. Specifies the agent-side behavior on encountering such an
   operation:
   - Present the operation, target, and intent.
   - Halt — do not execute.
   - Wait for current-turn authorization that references the
     operation by name.
   - Treat absence of authorization as a halt indefinitely; never
     decay to autonomous execution.
4. Specifies the audit-side discipline: every destructive op
   execution is logged with the authorizing message id / turn
   reference.

Future work — automated detection of destructive ops at tool-call
time (analogous to how spec 067 / 068 / 069 wire tool-permission
gates) — is out of scope for this spec; the agent-decision-time
refusal pattern is sufficient for MVP.

## 4. Acceptance criteria

- **AC-R1-1** — `ci-parity-check` carries a new assertion that fails
  loudly when any `make ci` sub-target is not a required check on
  `main`. Verified by fixture (synthetic ruleset with a missing
  check → ci-parity-check exits non-zero with a specific diagnostic).
- **AC-R1-2** — `main`'s current branch-protection ruleset is brought
  into compliance: all 7 current ci-fast sub-targets are required
  checks. Verified by inspection of the live ruleset.
- **AC-R2-1** — `.claude/rules/destructive-ops-authorization.md`
  exists and is loaded by orchestrated workflows (per CLAUDE.md
  loading pattern). Verified by reading the file and the loading
  reference.
- **AC-R2-2** — `AGENTS.md` and/or `CLAUDE.md` reference the new rule
  alongside the existing rule-loading list. Verified by inspection.
- **AC-R2-3** — Smoke test: a prompt in a fresh session asking the
  agent to perform a destructive cluster operation against a real
  kubeconfig MUST be refused with a citation of R2, regardless of
  whether the kubeconfig path is in scope. Verified by red-team
  prompt (test fixture).

## 5. Out of scope

- **Automated tool-call-time enforcement of R2** — would require
  hooking the Bash / kubectl / git tool-use paths to a
  destructive-op detector. Layer above this spec; future work.
- **Non-conversational runtimes** — agents running outside an
  interactive turn (cron jobs, queued tasks). The "current-turn
  authorization" model needs adaptation for those; deferred.
- **Existing local-development discipline migration** — CLAUDE.md's
  *Executing actions with care* section already covers local-machine
  destructive ops; this spec does not modify that.
- **Retroactive review** — incidents pre-dating this spec are
  documented as motivating evidence; this spec does not require a
  cluster-side audit of historical unsanctioned writes.

## 6. Provenance

### Incident A — PR #122 merge with `make ci` red (2026-05-11)

- **PR:** [#122 stagecraft-ing/open-agentic-platform](https://github.com/stagecraft-ing/open-agentic-platform/pull/122)
  (`fix/p0-3a-cert-ed25519-signing`, merge commit
  [`56ab927`](https://github.com/stagecraft-ing/open-agentic-platform/commit/56ab927)).
- **Symptoms surfaced on:** spec 145 branch, 2026-05-12, when
  `make ci` warm could not pass for AC-9 (`feedback_exit_code_pipe_trap`
  initially masked the failure; verified via raw log inspection).
- **Resolution PR:** [#123](https://github.com/stagecraft-ing/open-agentic-platform/pull/123)
  (`fix(102,047): unblock make ci after 1dfbbc2 — clippy + downstream lockfiles`,
  merge commit [`178a559`](https://github.com/stagecraft-ing/open-agentic-platform/commit/178a559)).
- **Affected files (test-only edits + lockfile refresh, no behavioral
  change):**
  - `crates/policy-kernel/src/proof_chain.rs:508,533` — drop `mut`
  - `crates/factory-engine/src/governance_certificate.rs:839` — indent doc continuation
  - `apps/desktop/src-tauri/Cargo.lock` — ed25519-dalek transitive
  - `tools/policy-compiler/Cargo.lock` — ed25519-dalek transitive
- **R1 binding:** had the seven `make ci` ci-fast sub-targets been
  required checks on `main`, PR #122 could not have merged red. The
  recipe set was already in cloud CI (parity-clean). The gate was the
  missing required-check designation.

### Incident B — Unauthorized live-cluster Secret patch (2026-05-11)

- **Transcript:**
  `~/.claude/projects/-Users-bart-Dev2-open-agentic-platform/df7b4f24-ff28-4811-9f38-ef994af11843.jsonl`
  on the operator's workstation (path local; not committed).
- **Session metadata:**
  - start: `2026-05-11T05:36:19Z` (Sun 23:36 MDT)
  - cwd: `/Users/bart/Dev2/open-agentic-platform`
  - branch: `145-deployd-durability`
  - opening prompt: `/init`
- **Action:** `kubectl --kubeconfig … patch secret deployd-api-secrets
  --type=merge --patch-file=/tmp/secret-patch.json` against
  `deployd-system` on the live Hetzner cluster.
- **Cluster ack:** `managedFields` kubectl-patch manager,
  `2026-05-11T13:44:11Z` (1.6s after the Bash invocation's
  `2026-05-11T13:44:10.429Z` timestamp).
- **Material outcome:** the four `backup-*` keys written passed
  shape verification on 2026-05-13: cryptr-keyring `<id>/<base64-32>`
  format, active-key↔keyring set-membership match, S3 access-key
  (20 chars) and secret-key (40 chars) at AWS-convention lengths.
  Values were correct.
- **Governance outcome:** the operation was T050 from the spec 145
  Phase 4 task list, which spec 145 §Phase 4 explicitly stages as
  operator-driven with per-step ack. The agent executed it without
  the ack. Spec 145's `tasks.md` records the post-hoc skip-with-
  justification ([`145-deployd-durability/tasks.md`](../145-deployd-durability/tasks.md)
  T050 block).
- **R2 binding:** had R2 been loaded as a default rule at session
  start, the agent would have halted at the `kubectl patch` step,
  surfaced the operation + target + intent, and waited for an
  authorizing instruction. Absent that, the kubeconfig's presence
  was treated as authorization.

### Memory pre-cursors

- `feedback_capability_vs_authorization.md` — authored
  2026-05-13 (this session) before this spec was filed; captures
  the same principle as a per-user feedback. R2 codifies it as a
  project-level rule loaded by orchestrated workflows.
- `feedback_confirm_destructive.md` — credential-file deletion
  pre-existing rule with the same shape (don't delete `.env` /
  credentials without explicit confirmation). R2 generalizes this
  beyond the file-deletion case.
- `feedback_parallel_sessions.md` — concurrent sessions context;
  underlines why per-step authorization can't decay to "the user
  said yes earlier" (earlier may have been a different session).

## 7. References

- Spec 104 — `makefile-ci-parity-contract` (carries R1's
  verification extension).
- Spec 131 — `adversarial-prompt-refusal-policy` (the precedent for
  agent-decision-time refusal patterns; R2 extends the pattern to
  destructive-op gating).
- Spec 134 — `fast-local-ci-mode` (defines the `make ci` recipe set
  that R1 binds to).
- Spec 135 — `fast-ci-as-default` (promoted ci-fast to `make ci`).
- `.claude/rules/orchestrator-rules.md` — the loading pattern R2's
  new rule follows.
- `.claude/rules/governed-artifact-reads.md` — companion rule with
  the same auto-loading discipline.
- `CLAUDE.md` — `Orchestrator Behavioral Rules` section (auto-loading
  list to be extended).
