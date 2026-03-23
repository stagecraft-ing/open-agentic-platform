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

## PR-1.1 — T000a baseline rerun (consolidation follow-up)

**Goal:** restore workspace + crate resolution so desktop install/build and Tauri compile succeed, without Feature 032 product behavior.

| Check | Command / how | Result (pass / fail / skip) | Notes |
|-------|----------------|-----------------------------|-------|
| Workspace root | `pnpm-workspace.yaml` lists `apps/*`, `packages/*` | pass | Added at repo root. |
| Desktop frontend install | `pnpm install --no-frozen-lockfile` (repo root) | pass | After importing `packages/types`, `packages/ui`. |
| Desktop frontend build | `pnpm -C apps/desktop build` | pass | `tsc && vite build` completed. |
| Tauri / backend compile | `cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml` | pass | After importing `crates/*`, `grammars/*`, and aligning `tauri` `macos-private-api` with `tauri.macos.conf.json`. |
| `packages/mcp-client` in tree | path check | pass | Unchanged from PR-1. |
| Baseline tests | `cargo test --manifest-path tools/registry-consumer/Cargo.toml --all --quiet` | pass | Still green. |

### Import-only changes (PR-1.1)

- `pnpm-workspace.yaml`, `pnpm-lock.yaml` (lockfile from root `pnpm install`)
- `packages/types/**`, `packages/ui/**` from OPC
- `crates/{agent,asterisk,axiomregent,blockoli,featuregraph,gitctx,run,stackwalk,titor,xray}/**` from OPC
- `grammars/tree-sitter-{c,javascript,python,rust,typescript}/**` from OPC (required by `asterisk` / `stackwalk` / `blockoli` build)
- `apps/desktop/src-tauri/Cargo.toml`: consolidation-only `tauri` feature alignment (`macos-private-api` on main `tauri` dependency; removed duplicate macOS-only `tauri` dep line) so `tauri-build` matches `tauri.macos.conf.json` allowlist

### Freeform: PR-1.1

- No T003+ inspect/git/governance/action wiring.

---

## PR-2+ — Feature 032 implementation commands

```bash
# Feature wiring: add package-specific test/build commands per PR.
```

## Results

- PR-1 baseline: preserved above (degraded truth at merge time).
- PR-1.1 baseline: **T000a green** for desktop install, desktop build, and Tauri compile on this host.
- Consolidation gate: **T000 complete**, **T000a complete** after PR-1.1 (full baseline checks green where applicable).
