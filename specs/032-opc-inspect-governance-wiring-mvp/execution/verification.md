# Verification: OPC inspect + governance wiring

Date: 2026-03-23  
Feature: `032-opc-inspect-governance-wiring-mvp`

## PR-1 — T000a baseline evidence (fill on import PR)

Record **before** any Feature 032 product wiring (T003+). Goal: prove imported trees are present and minimally healthy; behavior-neutral except required shims.

| Check | Command / how | Result (pass / fail / skip) | Notes |
|-------|----------------|-----------------------------|-------|
| Desktop frontend install | `pnpm -C apps/desktop install --no-frozen-lockfile` | fail | Missing workspace packages (`@opc/types@workspace:*` not present in this repo yet). |
| Desktop frontend build | `pnpm -C apps/desktop build` | skip | Build blocked because install step fails from unresolved workspace dependencies. |
| Tauri / backend compile | `cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml` | fail | Desktop backend depends on crates not yet imported (`crates/agent` and peers missing). |
| `packages/mcp-client` in workspace | `test -f packages/mcp-client/package.json && test -f packages/mcp-client/src/index.ts` | pass | Package path present after import; workspace resolution remains degraded until workspace files/deps are consolidated. |
| Baseline tests | `cargo test --manifest-path tools/registry-consumer/Cargo.toml --all --quiet` | pass | Existing pre-import repo baseline remains green for current toolchain surface (non-desktop path). |
| Temporary shims / path fixes | N/A — prose | pass | None applied in this PR slice yet. |
| Known non-032 breakages | N/A — prose | pass | Degraded baseline is bounded to missing consolidated workspace dependencies/crates required by imported desktop trees. |

### Freeform: import-only fixes

- Imported trees only:
  - `apps/desktop/**`
  - `packages/mcp-client/**`
- No inspect/git/governance feature behavior changes in this baseline capture step.

---

## PR-2+ — Feature 032 implementation commands

```bash
# T000a baseline checks (pre-032 wiring) — superseded after PR-1 fills the table above.
# Feature wiring: add package-specific test/build commands per PR.
```

## Results

- PR-1 baseline captured with bounded degraded state.
- Consolidation gate status: **T000 complete**, **T000a complete (degraded, documented)**.
