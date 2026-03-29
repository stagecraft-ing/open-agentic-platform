# Open questions (working notes)

> **Non-authoritative.** Resolved answers should move to `specs/...`, ADRs, or `execution/` artifacts — not linger only here.

## Purpose

Track **unresolved** questions that block or sharpen the next slice; keep each item tied to a **verification** path.

## Canonical references (read first)

- `specs/032-opc-inspect-governance-wiring-mvp/tasks.md` (remaining T010–T013)
- `specs/032-opc-inspect-governance-wiring-mvp/plan.md`

## Questions

| # | Question | Blocks | How to resolve |
|---|----------|--------|------------------|
| 1 | What is the smallest T010 action satisfying FR-005? | T010 | See recommendation below — **"Open spec file" button** from registry data is safest. Alternative: combine with "Check impact" secondary action. |
| 2 | Should T012/T013 verification document featuregraph degraded state as expected? | T012 | Yes — record `featuregraph: unavailable` as known bounded degradation in `verification.md`. Not a failure. |
| 3 | What verification commands complete the green baseline for T013? | T013 | See list below. |

### Q1 detail: T010 action recommendation

FR-005: "at least one actionable follow-up is available from inspect results."

**Recommended: "View spec" button.** When registry data shows features, each feature has a `specPath` field (e.g., `specs/032-opc-inspect-governance-wiring-mvp/spec.md`). A button that opens this file in a `claude-md` tab (or OS file viewer) satisfies FR-005 with:
- Zero new backend work
- Real data (from compiled registry)
- No dependency on `spec/features.yaml` (avoids featuregraph gap)
- Deterministic behavior (file path is known, existence is checkable)

**Implementation sketch:**
- `apps/desktop/src/features/inspect/actions.ts` — export function `openSpecAction(specPath: string)`
- `InspectSurface.tsx` — render "View spec" button when registry features are available
- Test: fixture-backed check that button renders when registry data includes specPath

### Q3 detail: verification commands for T013

```bash
# Frontend build (includes tsc type check)
pnpm -C apps/desktop build

# Tauri backend compile
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml

# Governance backend tests
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml commands::analysis::tests::

# Registry consumer contracts (must not regress)
cargo test --manifest-path tools/registry-consumer/Cargo.toml --all --quiet

# Spec compiler (registry emission)
cargo build --release --manifest-path tools/spec-compiler/Cargo.toml && ./tools/spec-compiler/target/release/spec-compiler compile

# Spec lint (non-blocking warnings)
cargo build --release --manifest-path tools/spec-lint/Cargo.toml && ./tools/spec-lint/target/release/spec-lint lint
```

## Answered (promote out)

- Q2 resolved: yes, document degraded featuregraph as expected state → promote to `execution/verification.md` when T012/T013 runs.

## Candidate promotions

- [ ] Q1 answer → T010 implementation plan for Cursor
- [ ] Q3 command list → `execution/verification.md` for T013
