# `make ci-fast` benchmark — SC-01 measurement record

> Spec **`134-fast-local-ci-mode`** §SC-01. This file MUST exist before
> spec 134 flips to `implementation: complete`. The aspirational targets
> in §SC-01 are warm ≤ 25 min and cold ≤ 50 min on reference hardware
> (M1 Pro, 10 cores, 64 GB RAM).

## Reference workstation

| | |
|---|---|
| Model | MacBookPro18,2 |
| CPU | Apple M1 Pro / Max (10 cores) |
| RAM | 64 GB |
| OS | macOS Darwin 25.3.0 |
| Rust toolchain | 1.95.0 (pinned via `rust-toolchain.toml`) |
| Node toolchain | per `apps/desktop/package.json` engine pin |
| Accelerators | sccache 0.15.0; cargo-nextest 0.9.133 |

## Warm-cache run (run #6, 2026-05-03)

**Wall time: `294s = 4m54s`. Exit: 0.**

Conditions:

- Branch: `main` at `2bae442` (post-PR #80 merge)
- Cargo target dirs: warm — populated by runs #1–#5 (over the prior ~5 hours of iteration)
- sccache: warm-cache populated; cargo's incremental build absorbed all but 2 of 245 compile requests, so sccache cache hit rate was 0% this run (nothing to fetch)
- nextest: enabled with `--no-tests=pass`
- `make -j$(CIFAST_JOBS)` outer concurrency: 4

### Per-sub-target breakdown (concurrent fan-out)

`ci-fast` runs all 7 sub-targets concurrently via `make -j4`, so the per-sub-target wall times overlap. The whole thing finishes when the slowest sub-target finishes. Approximate slowest: `ci-fast-rust` (the workspace clippy + nextest pass dominates), with `ci-fast-tools` close behind. `ci-fast-stagecraft` finishes in ~17 s standalone (dominated by `npm ci`); `ci-fast-supply-chain` is bounded by `cargo deny`. Test totals across all sub-targets:

| Surface | Tests passed |
|---|---|
| Rust (workspace) — `cargo nextest run --workspace` | 1003+ |
| Rust (deployd-api-rs) | bundled with workspace nextest |
| Tools — `cargo nextest run` per manifest | varies |
| Desktop vitest | 95/95 |
| Stagecraft vitest | 299/299 |
| Schema parity (Rust + bun walker) | 3/3 fingerprints |

### sccache snapshot at end of run

```
Compile requests              245
Compile requests executed       2
Cache hits                      0
Cache misses                    2
Cache hits rate              0.00 %
Non-cacheable calls           243
Compilations                    2
Compilation failures            0
```

The 0% sccache hit rate this run is **expected** — cargo's incremental build cache absorbed the recompile work first. sccache's value will arrive on later runs when target dirs are invalidated (toolchain bump, dep update, `cargo clean`).

### vs. SC-01 target

> Warm SHOULD be ≤ 25 min.

**Met by ~5×.** 4m54s vs 25-min target.

## Cold-cache run (PENDING)

A cold-cache measurement is **not yet captured**. It requires nuking the
following before running:

- `crates/target/` (workspace cargo target)
- `apps/desktop/src-tauri/target/`
- All `tools/*/target/` (covered by the shared `.target/cifast-tools/` for
  `ci-fast-tools`, but cold means starting from empty)
- `~/Library/Caches/Mozilla.sccache/` (or the platform sccache dir)
- `node_modules/` and `apps/desktop/node_modules/` and
  `platform/services/stagecraft/node_modules/`

Cold measurement is deferred to a follow-up commit. Spec 134 should not
flip to `implementation: complete` until cold is captured here.

## Trajectory

For context — what the optimisations bought:

| Run | Wall | State |
|---|---|---|
| #1 (estimate) | 90+ min | Original `make ci` (sequential, no accelerators) |
| #2 (warm) | 8m23s | Failed early — partial run, not a real comparison |
| #3 (warm, no accel) | 60m18s | xargs recipe bug + cargo lock contention |
| #4 (warm, +nextest, sccache cold) | 21m53s | Failed at nextest empty-tests |
| #5 (warm, all accel) | 4m53s | Failed at stagecraft cd-scope bug |
| **#6 (warm, all accel, all fixes)** | **4m54s** | **First end-to-end clean run** |

The 90+ min → 4m54s trajectory is roughly **18× faster** than the original
`make ci`. Headline drivers:

1. **Top-level parallel `make -j4` fan-out.** 7 sub-targets run concurrently rather than serial.
2. **`cargo nextest run`** instead of `cargo test`. Strict superset; ~2-3× faster on multi-binary test runs.
3. **`cargo clippy --workspace`** for `crates/Cargo.toml` instead of N per-member invocations. One typecheck of the whole graph beats 11.
4. **`cargo clippy --all-targets -- -D warnings`** subsumes the separate `cargo check` step.
5. **Shared `CARGO_TARGET_DIR`** for `tools/*` (deduplicates dep compilation across 7 isolated tool manifests).
6. **Concurrent tsc + vitest** in `ci-fast-desktop` and `ci-fast-stagecraft`.
7. **Background `pnpm audit` + `npm audit`** in `ci-fast-supply-chain`; parallel `cargo deny` via `xargs -P`.
8. **Dropped redundant `registry-consumer` 10× contract subset loop** (subsumed by the unfiltered run; prefix-existence side-channel preserved by an explicit `cargo test -- --list` post-pass).
9. **`sccache` and dropped `--jobs` flag** — neither contributed measurably to *this* run (cargo's incremental cache and the make jobserver respectively) but they're in place for future runs / rebuilds.

## Observations and follow-ups

- **Cargo package-cache lock contention.** The fan-out hits some serialisation on `~/.cargo/...` registry locks, but it doesn't dominate at this scale on a warm cache. Likely matters more on cold-cache runs.
- **sccache 0% hit rate.** Expected for warm-cache runs; the cache is positioned for the *next* full rebuild. Consider re-measuring after a deliberate `cargo clean` to validate sccache's actual win.
- **Concurrent-test flake** in `factory-platform-client::materialise_run_root_warm_cache_skips_fetches` is still open (carried from PR #76 description). It did not fire in run #6, but it's known.
- **Ghost-crate gap.** Several `crates/Cargo.toml` workspace members are not in `CI_RUST_MANIFESTS` and are validated only by `ci-fast`'s `--workspace` invocation, not by `make ci`. The structural amendment is a separate spec 104 §2.2 question (flagged on PRs #76, #77).
- **Cosmetic — redundant `--jobs $(CIFAST_CARGO_JOBS)`** in `ci-fast-rust` and `ci-fast-desktop`. Already cleaned up in `ci-fast-tools` (PR #78); same change for the others is queued.
