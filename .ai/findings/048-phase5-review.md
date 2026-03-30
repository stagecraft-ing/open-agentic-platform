# 048 Phase 5 Review — Loader, Snapshots, and Hot-Reload

> **Reviewer:** claude | **Date:** 2026-03-30 | **Verdict:** ✅ Approved

## Scope

Phase 5 deliverables per `.ai/plans/048-hookify-rule-engine-phased-plan.md`:

- `loader.ts`: configurable directory input (default `.claude/hooks/rules/`), recursive markdown rule discovery, parse + validate to ruleset snapshot
- File-watch reload loop: add/remove/change triggers rebuild of immutable ruleset snapshot, diagnostics retained for skipped invalid files
- API entrypoints: `loadRules(config)`, `getRulesSnapshot()`, `startHotReload()` / `stopHotReload()`

Reviewed against: FR-008, FR-009, SC-004, H-004, H-006.

## Requirement Satisfaction

### FR-008 — Configurable directory, hot-reload without restart

**Satisfied.**

- `DEFAULT_RULES_DIR` is `.claude/hooks/rules/` (`loader.ts:7`).
- `LoaderConfig.rulesDir` accepts an override (`loader.ts:11`).
- `resolveRulesDir()` resolves the path via `path.resolve()` (`loader.ts:28-30`).
- `loadRules()` performs one-shot synchronous load (`loader.ts:127-130`).
- `createRuleRuntime()` provides long-lived `loadRules()` for manual reload and `startHotReload()`/`stopHotReload()` for file-watch-driven reload (`loader.ts:146-219`).
- Hot-reload rebuilds the snapshot on any file change without requiring a new runtime instance — confirmed by test `loader.test.ts:101-111`.

### FR-009 — Invalid rules produce diagnostics, skip gracefully

**Satisfied.**

- `buildSnapshotFromDir()` calls `parseRuleSet()` which returns per-rule diagnostics for malformed YAML, missing fields, etc. (`loader.ts:112-122`).
- Invalid files are included in `diagnostics` while valid rules continue loading.
- Test `loader.test.ts:150-157`: file with no YAML frontmatter yields `HKY_MISSING_FRONTMATTER` diagnostic while `good.md` loads successfully. Consistent with H-006.

### SC-004 — New rule file takes effect on next event without restart

**Satisfied.**

- Test `loader.test.ts:83-99`: writes `a.md`, creates runtime, writes `b.md`, calls `runtime.loadRules()`, verifies both rules present, then evaluates event against new rule and confirms `warn` fires.
- Hot-reload path also tested (`loader.test.ts:101-111`): writes `b.md` after `startHotReload()`, waits 250ms (> 75ms debounce), confirms snapshot includes both rules.

### H-004 — Atomic ruleset snapshots (copy-on-write replacement)

**Satisfied.**

- `RulesetSnapshot` is declared with `readonly` properties (`loader.ts:18-25`).
- `createRuleRuntime` replaces `snapshot` variable atomically in `rebuild()` (`loader.ts:153-155`).
- Prior snapshot holders retain their reference — test `loader.test.ts:132-148` captures `captured = runtime.getRulesSnapshot()`, then reloads, and confirms the captured snapshot still has 1 rule while the runtime now has 2. Evaluation against the captured snapshot correctly sees no match for the new rule.
- Monotonic `version` field via `globalVersionSeq` (`loader.ts:105-109`) ensures callers can detect snapshot staleness.

## Implementation Quality

### Discovery — `discoverMarkdownRuleFiles()`

- Recursive walk with `readdirSync({ withFileTypes: true })` (`loader.ts:35-63`).
- Gracefully handles missing directory (returns `[]`), non-directory path (returns `[]`), and read errors (try-catch, skip).
- Sorted output (`localeCompare`) ensures deterministic rule ordering regardless of filesystem enumeration order.
- Test `loader.test.ts:62-71` confirms nested discovery (`nested/deep/r.md`).

### Hot-reload — per-directory `fs.watch`

- `listDirectoriesRecursive()` enumerates all subdirectories (`loader.ts:65-90`).
- Each directory gets its own `fs.watch()` watcher (`loader.ts:182-195`) — avoids reliance on the `recursive` option which is not universally supported on Linux.
- Debounced via 75ms timer (`RELOAD_DEBOUNCE_MS`, `loader.ts:140`) to coalesce rapid file changes.
- After each reload, watchers are torn down and re-attached (`loader.ts:174-179`) to pick up newly created subdirectories.
- `stopHotReload()` clears the debounce timer and closes all watchers (`loader.ts:210-217`).

### File read race handling

- `readRuleFiles()` wraps `readFileSync` in try-catch (`loader.ts:92-103`) to handle race between discovery and deletion. Clean approach.

### Exports

- All public types and functions exported from `index.ts:26-31`: `createRuleRuntime`, `DEFAULT_RULES_DIR`, `discoverMarkdownRuleFiles`, `loadRules`, plus type exports for `LoaderConfig`, `RuleRuntime`, `RulesetSnapshot`.
- Subpath export `./loader` in `package.json:18`.

## Test Coverage (8 tests in `loader.test.ts`)

| Test | Requirement |
|------|-------------|
| Default path constant | FR-008 |
| Missing directory → empty | FR-008 edge |
| Nested discovery | FR-008 |
| SC-004 manual reload | SC-004 |
| Hot-reload add (debounced) | SC-004, FR-008 |
| Change + delete update snapshot | H-004, FR-008 |
| In-flight snapshot isolation | H-004 |
| Invalid file diagnostic | FR-009, H-006 |

All 39/39 package tests pass. `tsc` clean.

## Findings

### P5-001 — `globalVersionSeq` is module-level mutable state (LOW)

`globalVersionSeq` (`loader.ts:105`) is a module-level counter shared across all `RuleRuntime` instances. If multiple runtimes are created in the same process, version numbers interleave — e.g., runtime A might see versions 1, 3, 5 while runtime B sees 2, 4, 6. This doesn't break monotonicity within a single runtime but may confuse callers comparing versions across runtimes. Acceptable for current usage but worth noting.

**Impact:** LOW — monotonicity within a single runtime is preserved; cross-runtime comparison is unlikely in practice.

### P5-002 — No test for hot-reload detecting new subdirectory creation (LOW)

Hot-reload re-attaches watchers after each rebuild (`loader.ts:177-178`), which should pick up newly created subdirectories. However, no test verifies this — all hot-reload tests write files to the root temp directory. A test that creates a new subdirectory and writes a rule into it would strengthen SC-004 coverage.

**Impact:** LOW — the code path is sound (teardown + re-attach), just untested for this specific scenario.

### P5-003 — Hot-reload debounce timer not cleared on `loadRules()` (LOW)

If `startHotReload()` is active and a file change triggers `scheduleReload()`, then the user calls `runtime.loadRules()` manually before the debounce fires, the timer still fires and rebuilds again redundantly. The manual `loadRules()` in `rebuild()` (`loader.ts:153-155`) doesn't cancel a pending debounce timer. This is harmless (just a wasted rebuild) but slightly wasteful.

**Impact:** LOW — correctness unaffected; rebuild is idempotent.

### P5-004 — `fs.watch` does not detect file renames on all platforms (INFO)

`fs.watch` behavior for rename events varies across platforms. On macOS, renaming a `.md` file within the watched directory fires a change event; on some Linux filesystems it may not. Since the plan explicitly chose per-directory watches for Linux safety, this is an accepted trade-off.

### P5-005 — No configurable debounce interval (INFO)

`RELOAD_DEBOUNCE_MS` is a constant (75ms). A future enhancement could expose this in `LoaderConfig` for environments with high filesystem churn. Not required by spec.

### P5-006 — `readRuleFiles` catch swallows all errors (INFO)

The empty catch in `readRuleFiles()` (`loader.ts:98`) silently drops permission errors, encoding errors, etc. — not just race-with-delete. A diagnostic for read failures would improve observability but is not required by FR-009 (which specifies syntax/validation diagnostics).

## Verdict

**Phase 5 approved.** FR-008 (configurable directory + hot-reload), FR-009 (diagnostic skip for invalid rules), SC-004 (new rule takes effect without restart), and H-004 (atomic snapshot replacement) are all satisfied. Implementation is clean, Linux-safe (per-directory watchers), and well-tested (8 tests covering the core scenarios). No blockers for Phase 6.

## Recommendation

Proceed to **Phase 6** (hooks.json manifest + built-in starter rules + verification artifact). Baton to **cursor**.
