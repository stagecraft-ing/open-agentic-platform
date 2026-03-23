# registry-consumer

Implements **Feature 002** ([`specs/002-registry-consumer-mvp/spec.md`](../../specs/002-registry-consumer-mvp/spec.md)): a **read-only CLI** over compiler-emitted **`registry.json`** (Feature **000** shape, produced by **`spec-compiler`**, Feature **001**).

## Prerequisite

Produce **`build/spec-registry/registry.json`** first (from the repository root):

```bash
cargo build --release --manifest-path tools/spec-compiler/Cargo.toml
./tools/spec-compiler/target/release/spec-compiler compile
```

## Build

```bash
cargo build --release --manifest-path tools/registry-consumer/Cargo.toml
```

## Usage (repository root)

Default registry path: **`build/spec-registry/registry.json`** relative to the **current working directory** (same convention as **`spec-compiler`**).

```bash
./tools/registry-consumer/target/release/registry-consumer list
./tools/registry-consumer/target/release/registry-consumer list --status draft --id-prefix 002
./tools/registry-consumer/target/release/registry-consumer show 002-registry-consumer-mvp
```

Override path:

```bash
./tools/registry-consumer/target/release/registry-consumer --registry-path /path/to/registry.json list
```

If **`validation.passed`** is **false** in **`registry.json`**, commands fail with exit code **1** unless **`--allow-invalid`** is set (diagnostics only).

## Exit codes

| Code | Meaning |
|------|---------|
| **0** | Success |
| **1** | Feature not found; or registry not authoritative (`validation.passed` false without `--allow-invalid`) |
| **3** | Missing/unreadable file, JSON parse error, or malformed registry for the requested operation |

## Trust model

The tool **does not** re-validate against **`registry.schema.json`**. It parses JSON, enforces **`validation.passed`** (unless **`--allow-invalid`**), and reads **`features[]`**. Feature **001** remains the schema gate.
