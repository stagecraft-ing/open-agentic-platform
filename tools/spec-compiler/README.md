# spec-compiler

Implements **Feature 001** ([`specs/001-spec-compiler-mvp/spec.md`](../../specs/001-spec-compiler-mvp/spec.md)), emitting Feature **000** JSON contracts:

- `build/spec-registry/registry.json` (deterministic)
- `build/spec-registry/build-meta.json` (ephemeral `builtAt`)

## Build

```bash
cd tools/spec-compiler
cargo build --release
```

## Run (from repository root)

```bash
./tools/spec-compiler/target/release/spec-compiler compile
```

Exit codes: `0` = success and validation passed; `1` = validation failed; `3` = I/O or parse error.

## Frontmatter (strict MVP)

The parser expects a line that starts exactly with `---` (no leading spaces, no trailing content on that line), then YAML, then a closing `---` delimiter before the markdown body. Variants such as `--- ` with trailing spaces on the opening line are **not** accepted—document this if editors emit them.

## Heading extraction (normative for this binary)

`sectionHeadings` lists ATX headings:

- Only `#` (H1) and `##` (H2); deeper levels ignored.
- Document order preserved.
- If the **first** heading text equals the frontmatter `title` (trimmed), that heading is **dropped** so the title line is not duplicated in the TOC.

This rule is implementation-specific; Feature 000 only requires stable `sectionHeadings` in the registry.

## contentHash

SHA-256 (hex) per Feature 000 research **D2** over discovered `specs/<NNN>-*/spec.md` files only (see Feature 001 **FR-007**).
