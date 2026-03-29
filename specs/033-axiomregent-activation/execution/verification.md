# Verification: axiomregent activation

**Feature**: `033-axiomregent-activation`

## Commands (fill when implementing)

| Check | Command | Result |
|-------|---------|--------|
| Desktop build | `pnpm -C apps/desktop build` | |
| Tauri check | `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` | |
| axiomregent crate tests | `cargo test --manifest-path crates/axiomregent/Cargo.toml` | |

## Notes

- Record platform-specific sidecar spawn evidence (logs or UI capture) when FR-001 is satisfied.
