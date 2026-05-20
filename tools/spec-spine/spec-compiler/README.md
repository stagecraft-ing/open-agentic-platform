# spec-compiler

Implements [`001-spec-compiler-mvp`](../../specs/001-spec-compiler-mvp/spec.md). Frontmatter strictness, heading-extraction normative behaviour, exit code mapping, and `contentHash` semantics live in §"Clarifications" of that spec. Crate-level inventory and dependencies: [`build/codebase-index/CODEBASE-INDEX.md`](../../build/codebase-index/CODEBASE-INDEX.md).

## Build & run

```bash
cargo build --release --manifest-path tools/spec-compiler/Cargo.toml
./tools/spec-compiler/target/release/spec-compiler compile
```

Run from the repository root. Output: `build/spec-registry/registry.json` (deterministic) and `build/spec-registry/build-meta.json` (ephemeral).
