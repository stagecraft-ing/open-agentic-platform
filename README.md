# open-agentic-platform

Authoritative architecture rules: human truth is **markdown** (with optional YAML **frontmatter inside** `.md` files); machine registries are **compiler-emitted JSON** only. See the constitutional bootstrap spec:

- [`specs/000-bootstrap-spec-system/spec.md`](specs/000-bootstrap-spec-system/spec.md)
- [`.specify/contract.md`](.specify/contract.md)

The **spec compiler MVP** (implements Feature 000’s contracts) is specified in [`specs/001-spec-compiler-mvp/spec.md`](specs/001-spec-compiler-mvp/spec.md). Build and run from the repo root:

```bash
cargo build --release --manifest-path tools/spec-compiler/Cargo.toml
./tools/spec-compiler/target/release/spec-compiler compile
```

Outputs: `build/spec-registry/registry.json` and `build-meta.json`. Details: [`tools/spec-compiler/README.md`](tools/spec-compiler/README.md).