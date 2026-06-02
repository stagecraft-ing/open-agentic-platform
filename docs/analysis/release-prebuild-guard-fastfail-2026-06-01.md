# Pre-build version guard — why the opc-v0.4.1 mismatch didn't fail fast

**Date:** 2026-06-01 (run UTC 2026-06-02)
**Lane:** Pre-build guard fast-fail (cost/correctness; safety already intact)
**Subject runs:** `release-desktop.yml` dispatch `26793968624` (committed=0.4.0, tag=opc-v0.4.1)
**Scope:** `.github/workflows/release-desktop.yml`, `.github/workflows/release-axiomregent.yml`
**Status:** Phase 1 — diagnosis only

---

## TL;DR — the premise was half-wrong, but there *is* a real cost problem

The prompt's premise was: *"the guard did not fire there; the build ran; the
refusal happened post-build."* The run log falsifies the first two clauses and
re-frames the third:

1. **The pre-build guard DID fire.** It is the failing step (`X Guard —
   committed version == tag (pre-build)`), and it produced exactly the
   `release-version-guard FAILED for opc (reference 0.4.1)` message the
   operator saw. That message is the **pre-build** guard's output (no SBOM
   arg), *not* the post-build SBOM guard (which emits a different
   `version guard failed post-build` string that never appears in this log).
2. **The desktop build (tauri-action) never ran.** Every step after the guard
   is `-` (skipped): "Build and release with Tauri (signed/unsigned)", SBOM,
   attestations, uploads. The two `release` matrix jobs died in **1m0s / 51s** —
   that is checkout + toolchain + pnpm install + sidecar *download* overhead,
   not a build. No draft was created. No version burned. Protection held.
3. **What actually ran wastefully was the SIDECAR matrix (~16.5 min), not the
   build.** The `release` job declares `needs: [sidecar, sidecar-linux-arm64]`,
   so GitHub Actions cannot start `release` — and therefore cannot run the
   guard — until the entire sidecar matrix completes. The slowest sidecar
   (windows) took **15m41s**; the guard executed at **02:24:33Z** against a run
   created at **02:07:58Z** → **~16m35s of axiomregent cross-compiles burned
   before a 0-cost committed-source check could reject the dispatch.**

**Root cause = candidate #1 from the prompt, confirmed by run-log evidence:**
the guard is correctly placed *within its job* (after tag-resolution, before
tauri-action) but the *job itself* is gated behind the full sidecar matrix via
`needs:`. The guard cannot pre-empt the sidecar builds. This is **not** a
non-problem — the fast-fail is genuinely missing; every mismatched dispatch
costs the full sidecar matrix to reject.

---

## Evidence

### Run-log excerpt (job `78987719094`, `release / x86_64-unknown-linux-gnu`, via GitHub API)

```
2026-06-02T02:24:33.3348773Z ##[group]Run tools/lint/release-version-guard.sh opc "0.4.1"
2026-06-02T02:24:33.3595772Z release-version-guard: product=opc reference=0.4.1 (from tag/expected)
2026-06-02T02:24:33.3615287Z ##[error]version mismatch [tauri.conf.json]: expected '0.4.1', got '0.4.0'
2026-06-02T02:24:33.3622431Z ##[error]version mismatch [package.json]: expected '0.4.1', got '0.4.0'
2026-06-02T02:24:33.3623823Z ##[error]version mismatch [Cargo.toml]: expected '0.4.1', got '0.4.0'
2026-06-02T02:24:33.3625065Z ##[error]version mismatch [Cargo.lock]: expected '0.4.1', got '0.4.0'
2026-06-02T02:24:33.3626782Z ##[error]release-version-guard FAILED for opc (reference 0.4.1) — release NOT eligible; discard the draft.
```

This is the step at `release-desktop.yml:262-264` — `release-version-guard.sh opc
"<version>"` with **no SBOM argument** → the pre-build call. The post-build
guard (`release-desktop.yml:377-404`) calls the script *with* an SBOM arg and
prints `::error::version guard failed post-build for '<tag>'` on failure; that
string is absent from the log. The refusal was unambiguously the pre-build guard.

### Job timeline (run `26793968624`)

| Job | Result | Duration |
|---|---|---|
| `sidecar / x86_64-unknown-linux-gnu` | ✓ | 8m56s |
| `sidecar / aarch64-unknown-linux-gnu` (the `sidecar-linux-arm64` job) | ✓ | 9m44s |
| `sidecar / aarch64-apple-darwin` | ✓ | 12m18s |
| `sidecar / x86_64-pc-windows-msvc` | ✓ | **15m41s** |
| `release / aarch64-apple-darwin` | **X** (failed at pre-build guard) | 1m0s |
| `release / x86_64-unknown-linux-gnu` | **X** (failed at pre-build guard) | 51s |

- Run created: `02:07:58Z`
- Guard executed: `02:24:33Z` → **Δ ≈ 16m35s** (the sidecar matrix wall-clock)
- Run finished: `02:25:35Z`

The guard itself is instant; the cost is entirely the sidecar matrix that
`release` waits on.

### Committed versions at the time (all 0.4.0, confirming the mismatch was real)

```
product/apps/opc/package.json            0.4.0
product/apps/opc/src-tauri/tauri.conf.json 0.4.0
product/apps/opc/src-tauri/Cargo.toml    version = "0.4.0"
crates/axiomregent/Cargo.toml            version = "0.4.0"
```

---

## Job dependency graph — `release-desktop.yml`

```
                ┌──────────────────────────────┐
  (dispatch /   │ sidecar (matrix × 3)          │  8–16 min  ── axiomregent cross-compiles
   tag push) ──▶│ sidecar-linux-arm64           │  ~10 min   ── (no version check anywhere here)
                └──────────────┬───────────────┘
                               │ needs: [sidecar, sidecar-linux-arm64]
                               ▼
                ┌──────────────────────────────┐
                │ release (matrix × 3)          │
                │   checkout                    │
                │   toolchain / pnpm install    │
                │   download sidecar artifacts  │
                │   Resolve release tag         │ ◀── needs only checkout
                │   ▶ Guard (pre-build) ◀       │ ◀── needs only checkout  ← FIRST version check, but gated behind sidecar
                │   tauri-action (build)        │     (skipped on mismatch)
                │   SBOM → Guard (post-build)   │     (defense-in-depth, build-time SBOM drift)
                │   attest / upload             │
                └──────────────────────────────┘
```

**Earliest possible version-vs-tag check that needs no build output:** right
after `checkout` + tag-resolution. The guard reads only committed sources
(`tauri.conf.json`, `package.json`, `Cargo.toml`, `Cargo.lock`) — all present
in a fresh checkout. It needs **neither the sidecar artifacts nor any build
product.** Today the earliest it *can* run is post-sidecar only because it
lives inside `release`, which `needs:` the sidecar jobs. Nothing technical
forces that ordering — it is an artifact of where the step was placed.

## Job dependency graph — `release-axiomregent.yml` (same shape)

```
  build (matrix × 3) ──┐
  build-linux-arm64  ──┴─ needs ─▶ publish
                                     checkout
                                     download artifacts
                                     Resolve tag           ◀── needs only checkout
                                     SBOM (scans crates/ source + generated lock)
                                     attest × 4             ◀── needs built binaries
                                     ▶ Guard (pre-publish, +SBOM) ◀  ← only version check, gated behind build
                                     gh release create --draft
```

Identical structural issue: the guard sits in `publish`, which `needs: [build,
build-linux-arm64]`. A mismatched axiomregent dispatch pays the full build
matrix before the guard rejects it. The axiomregent guard additionally asserts
the **SBOM** component version (3rd arg), but that SBOM is generated by scanning
`crates/` *source* + a freshly generated lockfile — **not** the built binaries —
so the committed-source half of the check is just as build-independent as the
desktop case.

---

## Root cause statement

The pre-build guard is **present, correctly authored, and fires before any
build** within its job. The defect is purely **job ordering**: the guard is a
step inside a job (`release` / `publish`) that `needs:` the build/sidecar
matrix, so ~9–16 min of compilation runs before the guard — a zero-cost,
checkout-only check — can execute. Candidate cause #1 (job-ordering, not a
missing guard) is confirmed; all other candidates (guard after build in-job,
`if:`-gated out, dispatch-only-missing, displaced by #277) are ruled out by
the workflow source and the run log.

#277 (spec 194, publish-boundary guard) did **not** move or displace the
pre-build guard; `git log` shows #276 (spec 193) introduced it and #277 only
added the three `gh release view` publish-boundary checks around the
post-build / upload steps. The pre-build guard is exactly where #276 placed it.

---

## Recommended fix (Phase 2)

Hoist the existing committed-source check into a **standalone early job** that
needs nothing, and make the build/sidecar matrix `needs:` it. Gate the whole
matrix on it rather than duplicating the check into every matrix leg.

- **`release-desktop.yml`**: add job `version-guard` (checkout → resolve tag →
  `release-version-guard.sh opc <version>`). Make `sidecar`,
  `sidecar-linux-arm64`, and `release` all `needs: [version-guard]` (release
  keeps its existing sidecar needs). On mismatch: fail in seconds, **zero**
  sidecar/build minutes, no draft, no tag. Keep the in-`release` pre-build
  guard *and* the post-build SBOM guard as defense-in-depth (they cost nothing
  on the happy path and catch build-time drift the early job can't see).
- **`release-axiomregent.yml`**: same — add a `version-guard` job (committed
  sources only, no SBOM); make `build`, `build-linux-arm64`, `publish` need it.

### axiomregent cost/benefit (the prompt asked us to weigh, not assume)

Worth adding. The axiomregent build matrix is 4 Rust cross-compiles of the same
order as the sidecar matrix (it builds the *same crate*). A mismatched
axiomregent dispatch today burns that full matrix before the guard rejects —
identical waste profile to desktop. An early committed-source-only guard job is
cheap (one checkout, ~20–30s) and removes that waste. The post-build
`+SBOM` guard stays as-is for build-time SBOM drift. Net: same fix, same
justification; no reason to treat axiomregent differently.

### What NOT to touch

- The post-build SBOM guard (`release-desktop.yml:377`) — different failure
  mode (build-time SBOM drift), defense-in-depth, keep as-is.
- The spec-194 publish-boundary guards — unrelated to fast-fail.
- `release-tools.yml` — separate lane (the checkout-by-SHA / v0.4.0 publish
  blocker); out of scope here.

### Governance

Both workflows are claimed by spec 193 via `refines: { aspect:
release-version-alignment }` (and co-claimed by 117/178/194). The fix is a
refinement of spec 193's version-guard mechanism, so it lands as a
**self-describing spec 193 refinement** (document the standalone early-guard
job in 193's body), **not** a drift waiver. `make pr-prep` before commit;
no new spec → no golden refresh expected. Halt at PR-open with ci-gate green;
operator merges.
