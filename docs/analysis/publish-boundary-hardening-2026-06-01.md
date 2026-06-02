# Phase 1 — Publish-boundary inventory (post-#276, report only)

> Date: 2026-06-01 · Scope: enumerate **every** workflow step that can create
> a release, publish a release, upload/modify/delete release assets, move/delete
> tags, or trigger another workflow that does any of those. Classify each
> against the publish-boundary principle. **Report only — no edits in this
> phase.** Repo: `stagecraft-ing/open-agentic-platform`.
>
> Companion to the pre-#276 investigation
> [`release-publish-paths-2026-06-01.md`](release-publish-paths-2026-06-01.md),
> which this supersedes for *current state*: #276 landed the `--draft` fix on
> axiomregent, the `v*`→`opc-v*` tag rename, the `release-version-guard.sh`
> pre/post-build guards, and moved the release-tools `workflow_run` gate to
> `startsWith(head_branch, 'opc-v')`. This document audits the tree **after**
> those fixed the *version-mismatch* class — and isolates the residual
> *publish-boundary* class.

## The principle (this is the gate)

Autonomous workflows may operate **only on draft releases**. Crossing the
publish boundary — **publishing** a release, **uploading assets to an
already-published** release, or **creating a non-draft** release — requires a
**human action**. A reactive workflow that attaches assets to, or edits notes
on, a **draft** is compliant. Editing the **notes** of a published release is
legal under asset-immutability (notes are mutable metadata; downloadable
assets are not).

## Method

`grep -rEn` across all 28 `.github/workflows/*.yml` for `gh release`,
`gh api .*releases`, `softprops`/`create-release`/`upload-release-asset`,
`releaseDraft`/`--draft`/`draft: false`/`make_latest`/`--latest`/`prerelease`,
`git tag`/`git push .*tag`/`--cleanup-tag`/`gh release delete`, and
`workflow_run`/`repository_dispatch`. Six files matched on substance; the rest
matched only on comment/keyword and carry no release/tag/asset operation.

## Full inventory — every boundary-capable step

| # | Workflow → step (line) | Operation | Trigger | Acts on | Autonomous? | Class |
|---|------------------------|-----------|---------|---------|-------------|-------|
| 1 | **release-desktop** → tauri-action *create* (L284/317, `releaseDraft: true`) | create release | `push: tags ['opc-v*']` · `workflow_dispatch{tag}` | **draft** | No (human tag-push/dispatch) | ✅ COMPLIANT |
| 2 | **release-desktop** → *Upload SBOM/attest* (`gh release upload --clobber`, L434/439) | upload assets | same run as #1 | release just created (draft in happy path) | **runs without a draft check** | ⚠️ **VIOLATION** (can clobber a published release if a human publishes during the multi-OS build) |
| 3 | **release-desktop** → *Generate SHA-256 sidecars* (`gh release upload --clobber`, L472) | upload assets | same run as #1 | release (draft in happy path) | **runs without a draft check** | ⚠️ **VIOLATION** (same race as #2) |
| 4 | **release-desktop** → *post-build SBOM guard* (`gh release delete --yes --cleanup-tag`, L388, on guard failure) | **delete release + tag** | same run as #1 | release + tag (draft in happy path) | **runs without a draft check** | ⚠️ **VIOLATION** (most severe — can autonomously *delete a published release and its tag* if a human published before the post-build guard tripped) |
| 5 | **release-axiomregent** → *Create Release* (`gh release create --draft`, L268–271) | create release | `push: tags ['axiomregent-v*']` · `workflow_dispatch{tag}` | **draft** | No (human) | ✅ COMPLIANT (create-draft only; no upload-to-existing, no delete; `create` on an existing published tag would fail by construction) |
| 6 | **release-tools** → *Upload tool archives + SBOM + attest* (`gh release upload --clobber`, L264) | upload assets | **`workflow_run` ['Release Desktop' completed]** · `workflow_dispatch{tag}` | desktop release (draft in happy path) | **YES — reactive chain, runs without a draft check** | ⚠️ **VIOLATION** (primary — see race below) |
| 7 | **ai-changelog** → *Update release body* (`gh release edit --notes-file`, L81) | edit **notes** | **`release: [published]`** | published release, notes only | reactive | ✅ COMPLIANT (notes are mutable metadata; assets untouched — legal under immutability) |
| — | **ai-changelog** → *Get previous tag* (`git tag --sort`, L37) | **read** tags | `release: [published]` | — | — | ✅ not a mutation (lists tags to find the prior one) |
| — | **ci-supply-chain** (`release-version-consistency` job) | runs `release-version-guard.sh` (read-only lint) | `workflow_call`/`schedule`/`dispatch` | — (`permissions: contents: read`) | — | ✅ out of scope (no release/tag/asset op) |
| — | **ai-pr-review** | `gh pr comment` only | `workflow_call`/`dispatch` | PR comments | — | ✅ out of scope |

Confirmed-negative sweep: **no** `gh api .../releases` calls, **no** `softprops`/
`create-release` actions, **no** `git push --tags`, **no** `repository_dispatch`
chains anywhere in the tree. `release-desktop.yml:147 release:` is a **job name**,
not an `on:` trigger (its `on:` is tag-push + dispatch). `opc-e2e-nightly.yml`
uses `tauri-action` to **build** only — no `tagName`/`releaseDraft`/`gh release`.

## The reactive race (primary violation — #6)

`release-tools` triggers on **Release Desktop completion**, by which point the
desktop draft is fully populated and *looks done*. It then spends ~30 min
building the four CLI tools across three OSes before its single
`gh release upload --clobber` step (L264). There is **no assertion that the
target release is still a draft**. If the operator publishes the
complete-looking desktop draft during that ~30-min window, the upload lands on
an **already-published** release — an autonomous crossing of the publish
boundary, and a mutation of a release whose SHA-256 sidecars and SLSA
attestations were already public.

`release-desktop` (#2/#3/#4) carries the *same* missing-draft-check gap with a
shorter (but non-trivial: full multi-OS build) window — and its guard-failure
**delete** (#4) is strictly worse than an upload, because it would *destroy* a
published release + tag rather than append to it.

## Remediation direction (Phase 2 — constrain, do not delete)

Per the principle's "prefer constraining over deleting": none of these paths
should be removed — each has a compliant purpose (populate / repair / annotate
a **draft**). The fix is a uniform **fail-closed draft assertion** before every
`gh release upload` / `gh release delete` in #2, #3, #4, #6:

```
isDraft must be true for "$TAG" → else ::error:: and exit 1 (refuse to mutate a non-draft).
```

This converts the silent boundary-crossing into a loud, safe failure and
encodes the invariant directly: *automation only ever mutates drafts; a
published release is immutable to these workflows.* Happy path is unchanged
(the release is always a draft when these steps run), so the guard only bites
on the race. A human who genuinely wants to mutate a published release does so
manually at their own terminal — explicitly outside the governed automation,
which is exactly "requires a human action."

`release-tools`' `workflow_dispatch` backfill path keeps working for its
intended target (a draft that missed its tool archives); backfilling a
*published* release is, by this invariant, a deliberate manual human step.

**Untouched (verified draft-targeting / legal):** `release-axiomregent`
(create-draft-only), `ai-changelog` (notes-only edit, legal under immutability)
— both confirmed compliant and left as-is per the lane's explicit instruction.

## Governance note (coupling cascade)

Edits land in `release-desktop.yml` and `release-tools.yml`, both governed by
the spec spine (`# Spec: 037/086/117`; the draft/publish posture is **spec
193 — paired-release-cadence**, which already established "publishing the draft
is a separate, human-gated action"). The publish-boundary invariant is a
self-describing **refinement of spec 193**, so the coupling cascade is resolved
by **amending spec 193** (not a Spec-Drift-Waiver), per
`.claude/rules/adversarial-prompt-refusal.md` and the graph-truthful-resolution
posture.
