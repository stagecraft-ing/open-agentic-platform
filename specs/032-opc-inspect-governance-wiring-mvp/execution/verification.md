# Verification: OPC inspect + governance wiring

Date: 2026-03-23  
Feature: `032-opc-inspect-governance-wiring-mvp`

## PR-1 — T000a baseline evidence (fill on import PR)

Record **before** any Feature 032 product wiring (T003+). Goal: prove imported trees are present and minimally healthy; behavior-neutral except required shims.

| Check | Command / how | Result (pass / fail / skip) | Notes |
|-------|----------------|-----------------------------|-------|
| Desktop frontend install | _(e.g. pnpm/npm install in workspace)_ | | |
| Desktop frontend build | _(e.g. vite/turbo build for `apps/desktop`)_ | | |
| Tauri / backend compile | _(e.g. `cargo build` for desktop crate)_ | | |
| `packages/mcp-client` in workspace | _(workspace file lists package; resolves from desktop)_ | | |
| Baseline tests | _(list commands: unit/e2e as applicable)_ | | |
| Temporary shims / path fixes | N/A — prose | | Only what PR-1 needed for baseline; link commits or files |
| Known non-032 breakages | N/A — prose | | Documented degraded baseline if not fully green |

### Freeform: import-only fixes

- List any files changed **only** for consolidation (paths, workspace, CI), not for 032 behavior.

---

## PR-2+ — Feature 032 implementation commands

```bash
# T000a baseline checks (pre-032 wiring) — superseded after PR-1 fills the table above.
# Feature wiring: add package-specific test/build commands per PR.
```

## Results

- Pending implementation.
- Baseline verification results (T000a): pending — complete table in **PR-1**.
