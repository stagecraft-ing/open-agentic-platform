---
paths:
  - "**/Cargo.toml"
  - "Makefile"
  - "Makefile.*"
---

# Raw build invocations

These are the underlying cargo commands behind the Makefile entry points
(`make setup`, `make ci`, `make registry`, `make pr-prep`). Prefer the
Makefile targets in normal use; reach for these only when investigating
a specific tool or driving a build outside the Makefile.

## Spec-spine tools

```bash
# Spec compiler
cargo build --release --manifest-path tools/spec-spine/spec-compiler/Cargo.toml
./tools/spec-spine/spec-compiler/target/release/spec-compiler compile

# Registry consumer
cargo build --release --manifest-path tools/spec-spine/registry-consumer/Cargo.toml
./tools/spec-spine/registry-consumer/target/release/registry-consumer list
./tools/spec-spine/registry-consumer/target/release/registry-consumer show <feature-id>

# Spec lint
cargo build --release --manifest-path tools/spec-spine/spec-lint/Cargo.toml

# Codebase indexer
cargo build --release --manifest-path tools/spec-spine/codebase-indexer/Cargo.toml
./tools/spec-spine/codebase-indexer/target/release/codebase-indexer compile
./tools/spec-spine/codebase-indexer/target/release/codebase-indexer check
./tools/oap/oap-code-index-enrich/target/release/oap-code-index-enrich render
```

## OAP-specific tools

```bash
# Policy compiler
cargo build --release --manifest-path tools/oap/policy-compiler/Cargo.toml
```

## Per-crate target dirs

The Makefile passes `--target-dir <crate>/target` overrides per issue #46
to avoid workspace-wide rebuilds. Direct `cargo build` without
`--target-dir` lands in the workspace `target/` and may not match what
the Makefile expects. Prefer `make <target>` for routine work.
