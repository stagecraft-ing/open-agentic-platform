# Investigation — GitHub release publish paths (report only)

> Date: 2026-06-01 · Scope: enumerate every path that can publish or mutate
> a GitHub Release; determine what published `axiomregent-v0.3.1`
> (immutable); flag non-human paths. **Report only — no remediation in this
> document.** Repo: `stagecraft-ing/open-agentic-platform`.

## TL;DR

`axiomregent-v0.3.1` was published by the **Release axiomregent** workflow's
`gh release create` step, which **omits `--draft`** and therefore publishes a
**public, immutable** release the instant the workflow runs. The run was a
**manual `workflow_dispatch`** by human **`bartekus`** at `2026-06-01T21:54:34Z`;
the release's nominal `author` is `github-actions[bot]` only because the
workflow's `GITHUB_TOKEN` performs the creation. The tag `axiomregent-v0.3.1`
was created by that publish, pointing at `main` HEAD (`5bb5528d`). The
axiomregent crate is still at `0.2.0`, so the published `v0.3.1` ships a binary
that reports `0.2.0` — the same version-mismatch class as the OPC drafts, but
already burned into an immutable public release.

## Paths that can CREATE / PUBLISH a release

| # | Workflow | Mechanism | Draft? | Triggers | Human? |
|---|----------|-----------|--------|----------|--------|
| 1 | **release-axiomregent.yml** | `gh release create "$TAG" dist/*` (L239) — **no `--draft`** | ❌ **publishes live, immutable** | `push: tags ['axiomregent-v*']` · `workflow_dispatch{tag}` | dispatch = human; tag-push = human; **bot owns the created release object** |
| 2 | **release-desktop.yml** | `tauri-action` `releaseDraft: true` (L286/313) | ✅ draft only | `push: tags ['v*']` · `workflow_dispatch{tag}` | human; **publish step still requires a human to click Publish** |

**Path 1 is the uncontrolled live-publish path.** No draft, no version guard —
a single `workflow_dispatch` burns a public, immutable tag + release.
**Path 2** already gates the *publish* behind a human (drafts), but bakes the
stale internal version (`0.3.4`) into the assets regardless.

## Paths that MUTATE an existing release (cannot create a public release alone)

| # | Workflow | Mechanism | Triggers | Non-human? |
|---|----------|-----------|----------|------------|
| 3 | **release-tools.yml** | `gh release upload "$TAG" --clobber` (L264) | **`workflow_run` on "Release Desktop" completed** · `workflow_dispatch{tag}` | ⚠️ **YES — automated chain** off Desktop completion |
| 4 | **ai-changelog.yml** | `gh release edit "$TAG" --notes-file` (L81) | **`release: published`** | ⚠️ **YES — automated/reactive** to any publish |

- **release-tools.yml** auto-runs after every Release Desktop completion and
  uploads the CLI-tool archives to the release tag. Its `workflow_run` gate is
  `startsWith(github.event.workflow_run.head_branch, 'v')` (L48). On a
  *tag-push* desktop release, `head_branch` is the tag (`v0.3.4`) → fires. On a
  *workflow_dispatch* desktop release, `head_branch` is `main` → **does not
  fire** (this is why the `v0.3.5/6/7` dispatched drafts have no tool archives).
- **ai-changelog.yml** fires on `release: published`. Drafts are not "published",
  so it fires only for **axiomregent** live releases (and any manually-published
  desktop release). It ran against `axiomregent-v0.3.1` and `-v0.3.0`. It
  **edits notes only** — it does not create or publish.

## Paths that BUILD via tauri-action but do NOT release

- **opc-e2e-nightly.yml** — uses `tauri-action` (L15) for the E2E harness only;
  no `tagName`/`releaseName`/`releaseDraft`/`gh release` keys. Builds, never
  releases. Triggers: `schedule` (nightly) + `workflow_dispatch`.

## Current release inventory (observed 2026-06-01)

| Tag | State | Created | Note |
|-----|-------|---------|------|
| `v0.3.7` | **Draft** | 22:24Z | OPC — assets `0.3.4` (mismatch) |
| `axiomregent-v0.3.1` | **Latest (live)** | 21:30Z / pub 22:11Z | binary `0.2.0` (mismatch); **immutable** |
| `v0.3.6` | **Draft** | 15:48Z | OPC — assets `0.3.4` (the original screenshot) |
| `axiomregent-v0.3.0` | live | 08:30Z | binary `0.2.0` (mismatch) |
| `v0.3.5` | **Draft** | 2026-05-29 | OPC — assets `0.3.4` |
| `v0.3.4` | live | 2026-05-25 | last honest bump (`#231`) — internal `0.3.4` matched |

Three stale OPC drafts (`v0.3.5/6/7`) all carry `0.3.4` assets. Two live
axiomregent releases (`v0.3.0`, `v0.3.1`) carry `0.2.0` binaries.

## Non-human paths — flagged

1. **release-tools.yml** `workflow_run` chain (auto-runs after Release Desktop). Upload-only.
2. **ai-changelog.yml** `release: published` reactive notes edit. Mutate-only.
3. **All workflow-created release objects** are authored by `github-actions[bot]`
   (the `GITHUB_TOKEN`), independent of which human dispatched the run. The human
   actor is recoverable only from the workflow-run `triggering_actor`
   (here: `bartekus`).

## Implications for the rename (`v*` → `opc-v*`) — forward note, not remediation

If Release Desktop's trigger becomes `opc-v*`, the **release-tools.yml**
`workflow_run` gate `startsWith(head_branch, 'v')` will evaluate
`startsWith('opc-v0.4.0', 'v')` = **false** → the tool-archive chain silently
stops firing for renamed desktop releases. The gate must move to `'opc-v'`.
(Recorded here so the IMPLEMENT phase accounts for it; not changed in this report.)
