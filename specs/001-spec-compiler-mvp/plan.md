# Implementation Plan: Spec compiler MVP

**Branch**: `001-spec-compiler-mvp` | **Date**: 2026-03-22 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-spec-compiler-mvp/spec.md`

## Summary

Implement a **Rust** workspace crate at **`tools/spec-spine/spec-compiler/`** exposing a **`spec-compiler`** CLI that compiles repo feature specs into **`build/spec-registry/registry.json`** and **`build-meta.json`**, following Feature **000** schemas and validation codes **V-001**–**V-004**.

## Technical Context

**Language/Version**: Rust (edition 2021), stable toolchain pinned via `rust-toolchain.toml` or `Cargo.toml` `rust-version`.

**Primary Dependencies**: `serde`, `serde_json` (with deterministic serialization strategy), YAML frontmatter parser (`gray_matter`-equivalent Rust ecosystem, e.g. `matter` + `serde_yaml` scoped to frontmatter blocks), `sha2` for `contentHash`, `walkdir` or `ignore` for filesystem walks, `clap` for CLI.

**Storage**: Read-only inputs from `specs/`; write outputs to `build/spec-registry/`. Create parent directories if missing.

**Testing**: `cargo test` including integration tests with golden `registry.json` fixtures; subprocess tests for CLI exit codes.

**Target Platform**: macOS and Linux CI (Windows optional, not required for MVP).

**Project Type**: CLI binary + internal library modules.

**Performance Goals**: Compile default `specs/` tree in &lt; 5 s on a typical dev machine for &lt; 200 features.

**Constraints**: No standalone authored YAML; determinism for `registry.json`; subordinate to Feature 000.

**Scale/Scope**: Single repository; one compiler version line until Feature 000 `specVersion` bumps.

## Constitution Check

| Gate | Status |
|------|--------|
| Markdown-only authoring | Pass — compiler reads `.md` only |
| Compiler-owned JSON | Pass |
| Feature 000 precedence | Pass — schemas imported by path |

## Project Structure

### Documentation (this feature)

```text
specs/001-spec-compiler-mvp/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── tasks.md
├── quickstart.md
├── clarify.md
├── contracts/
│   └── README.md
└── checklists/
    └── requirements.md
```

### Source Code (repository root)

```text
tools/spec-spine/spec-compiler/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── scan.rs
│   ├── parse.rs
│   ├── emit.rs
│   ├── hash.rs
│   └── validate.rs
└── tests/
    ├── golden.rs
    └── fixtures/
```

**Structure Decision**: Rust crate under `tools/spec-spine/spec-compiler/`; binary name **`spec-compiler`** (package `spec-compiler` or `open_agentic_spec_compiler` — crate name must be snake_case; binary can still be `spec-compiler` via `[[bin]]` name).

## Complexity Tracking

None.
