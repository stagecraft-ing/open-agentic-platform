---
feature: "037-cross-platform-axiomregent"
---

# Tasks: cross-platform axiomregent binaries

## Implementation

- [ ] **T001** — Create build script for axiomregent cross-compilation
  - `scripts/build-axiomregent.sh` (or `.ps1` for Windows)
  - Must produce `axiomregent-{triple}[.exe]` for all 5 targets
  - Use `cargo build --release --target <triple>` with appropriate toolchains
  - Strip debug symbols (`strip` on Unix, default MSVC release on Windows)

- [ ] **T002** — Build Windows x86_64 binary locally
  - `cargo build --release --target x86_64-pc-windows-msvc -p axiomregent`
  - Copy to `apps/desktop/src-tauri/binaries/axiomregent-x86_64-pc-windows-msvc.exe`
  - Verify binary runs: `./axiomregent-x86_64-pc-windows-msvc.exe --help` or stdio MCP handshake

- [ ] **T003** — Build macOS x86_64 binary (CI or cross-compile)
  - Target: `x86_64-apple-darwin`
  - Copy to `apps/desktop/src-tauri/binaries/axiomregent-x86_64-apple-darwin`

- [ ] **T004** — Build Linux x86_64 and arm64 binaries (CI or cross-compile)
  - Targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`
  - Copy to `apps/desktop/src-tauri/binaries/`

- [ ] **T005** — Verify sidecar spawn on Windows
  - Run desktop app on Windows
  - Confirm `spawn_axiomregent` succeeds (check `SidecarState.axiomregent_port` is `Some`)
  - Confirm governance UI shows connected state (not degraded/bypass)
  - Confirm agent execution uses governed dispatch (audit log shows `allowed`/`denied`, not bypass badge)

- [ ] **T006** — Create CI workflow for cross-platform builds
  - `.github/workflows/build-axiomregent.yml`
  - Matrix: `[macos-latest, ubuntu-latest, windows-latest]` × appropriate targets
  - Trigger: push to `crates/axiomregent/**`
  - Artifact: upload binaries for manual download or auto-commit

- [ ] **T007** — Document binary sizes and verification results
  - Record size per target in `execution/verification.md`
  - Verify NF-001 (< 30 MB per target)

## Closure

- [ ] **T008** — Update `execution/verification.md` with test commands and results
- [ ] **T009** — Run `spec-compiler compile` to update registry
