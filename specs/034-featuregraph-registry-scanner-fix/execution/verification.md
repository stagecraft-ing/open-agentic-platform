# Verification: featuregraph registry scanner fix

**Feature**: `034-featuregraph-registry-scanner-fix`

## Commands (fill when implementing)

| Check | Command | Result |
|-------|---------|--------|
| Registry build | `cargo build --release --manifest-path tools/spec-compiler/Cargo.toml && ./tools/spec-compiler/target/release/spec-compiler compile` | |
| Featuregraph tests | `cargo test -p featuregraph` | |
| Desktop check | `pnpm -C apps/desktop check` | |

## Notes

- Governance manual smoke: load governance on a repo with `build/spec-registry/registry.json` present and no `spec/features.yaml`.
