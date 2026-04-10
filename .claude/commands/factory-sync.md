---
name: factory-sync
description: Detect, map, and translate upstream changes from the_factory and AIM-vue-node-template into OAP factory/
allowed-tools: Bash, Agent, Read, Edit, Write, Glob, Grep
argument-hint: "[upstream-name] — optional: sync only one upstream (the_factory or aim-vue-node-template)"
---

# Factory Sync

Translate upstream repository changes into OAP's factory/ three-layer architecture using the mapping manifest at `factory/upstream-map.yaml`.

See: `specs/088-factory-upstream-sync/spec.md`

## Phase 1: Discover Changes

For each upstream in `factory/upstream-map.yaml` (or only the one named in `$ARGUMENTS` if provided):

1. **Read the manifest**: parse `factory/upstream-map.yaml` to get the upstream's `path`, `last_synced_sha`, `exclude_patterns`, and `mappings`.

2. **Verify upstream exists**: check the path resolves and is a git repo.
   ```bash
   git -C {path} rev-parse HEAD
   ```
   If the upstream is unavailable, warn and skip it.

3. **Diff from last sync**: get the commit range and changed files.
   ```bash
   git -C {path} log --oneline {last_synced_sha}..HEAD
   git -C {path} diff --stat {last_synced_sha}..HEAD
   git -C {path} diff --name-only {last_synced_sha}..HEAD
   ```

4. **Map changes**: for each changed upstream file:
   - Check if it matches any `exclude_patterns` → skip if excluded
   - Look up its entry in `mappings` → record the OAP targets, relationship type, and layer
   - If no mapping exists → flag as **unmapped** (may need a manifest update)

5. **Write Change Report** to `.factory/sync-report.md`:
   ```markdown
   # Factory Sync Report — {date}

   ## {upstream-name}
   Commit range: {last_synced_sha}..{current_sha}
   Commits: {count}
   Date range: {first_date} to {last_date}

   ### Mapped Changes
   | Upstream File | Commits | OAP Targets | Relationship | Layer |
   |---|---|---|---|---|

   ### Unmapped Changes
   | Upstream File | Commits | Action Needed |
   |---|---|---|

   ### Excluded Files
   {list of skipped files and which exclude pattern matched}
   ```

## Phase 2: Analyze Impact

For each mapped change, dispatch an Agent to analyze it:

**CRITICAL: Launch multiple analysis agents in parallel when they are independent (different upstream files). Each agent should:**

1. Read the upstream diff for its file:
   ```bash
   git -C {path} diff {last_synced_sha}..HEAD -- "{source_file}"
   ```

2. Read the current content of all OAP target files listed in the mapping.

3. Read the mapping's `notes` field for translation guidance.

4. Classify the change as: **bug-fix**, **enhancement**, **refactor**, or **goa-specific**.

5. For `diffable` relationships: produce a proposed edit (old_string → new_string) for each OAP target.

6. For `restructured` relationships: describe what changed upstream, identify which sections of the OAP targets are affected, and propose what to change — but note that human judgment is required.

7. For `extract-only` relationships: extract the relevant sections from the upstream diff and compare against OAP targets.

8. Write per-file analysis to `.factory/sync-analysis/{upstream}/{filename}.md`.

After all agents complete, update `.factory/sync-report.md` with the analysis results and a recommended action for each change:
- **APPLY** — clear translation, propose specific edits
- **REVIEW** — restructured mapping, human judgment needed
- **SKIP** — GoA-specific or no OAP impact
- **MANIFEST-UPDATE** — unmapped file that should be added to the manifest

## Phase 3: Apply Changes

**CHECKPOINT: Present the sync report and analysis to the user. Wait for explicit approval before modifying any OAP files.**

The user may:
- Approve all changes → apply everything
- Approve selectively → apply only approved changes
- Request modifications → adjust proposed edits before applying
- Reject → skip this sync entirely

For each approved change:

1. Apply the proposed edit(s) to the OAP target file(s).

2. After modifying each file, verify internal consistency:
   - If a stage file was modified: check that gate IDs referenced in the file match entries in `contract/schemas/verification.schema.yaml`
   - If the verification schema was modified: check that all referenced check IDs exist
   - If a pattern file was modified: check that `factory/adapters/aim-vue-node/manifest.yaml` lists it
   - If an invariant was added: check that the ID follows the sequence (no gaps, no duplicates)
   - If the reviewer was modified: check that any new deficiency tags are documented in the tag table

3. Report each applied change as it completes.

**CHECKPOINT: Present all modified files for review before updating the manifest SHA.**

## Phase 4: Finalize

After user approval of the applied changes:

1. Update `last_synced_sha` and `last_synced_date` in `factory/upstream-map.yaml` for each synced upstream.

2. Run a quick validation:
   ```bash
   # Verify spec compiler still works
   cargo build --release --manifest-path tools/spec-compiler/Cargo.toml 2>&1 | tail -5
   ./tools/spec-compiler/target/release/spec-compiler compile 2>&1 | tail -10
   ```

3. Report summary:
   ```
   ## Sync Complete
   - Upstream: {name}
   - Commit range: {old_sha}..{new_sha} ({count} commits)
   - Changes applied: {N}
   - Changes skipped: {M} (GoA-specific or rejected)
   - Unmapped files: {K} (manifest update recommended)
   - Files modified: {list}
   ```

4. Clean up: remove `.factory/sync-analysis/` working directory.

## Important Rules

1. **Never modify upstream repos.** This command is read-only with respect to the_factory and AIM-vue-node-template.
2. **Never skip checkpoints.** Both checkpoint gates (before apply, before SHA update) require explicit user approval.
3. **Preserve OAP architecture.** Upstream changes to monolithic skill files must be decomposed into the correct OAP layers (process, contract, adapter). Never copy upstream content verbatim into OAP.
4. **Strip GoA content.** Government of Alberta-specific references (ministry names, Entra ID specifics, Protected B, ASVS chapters) must not appear in OAP files. Translate the underlying pattern, not the GoA implementation.
5. **Idempotent.** Running this command twice with no new upstream commits should produce no changes.
6. **One upstream at a time when $ARGUMENTS is provided.** If the user specifies an upstream name, only sync that one.
