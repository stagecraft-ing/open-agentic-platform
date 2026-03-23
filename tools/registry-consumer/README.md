# registry-consumer

Implements **Feature 002**, **Feature 007**, **Feature 008**, **Feature 009**, **Feature 010**, **Feature 011**, **Feature 012**, **Feature 013**, **Feature 014**, **Feature 015**, **Feature 016**, and **Feature 017** ([`specs/002-registry-consumer-mvp/spec.md`](../../specs/002-registry-consumer-mvp/spec.md), [`specs/007-registry-consumer-status-report-mvp/spec.md`](../../specs/007-registry-consumer-status-report-mvp/spec.md), [`specs/008-registry-consumer-status-report-json-mvp/spec.md`](../../specs/008-registry-consumer-status-report-json-mvp/spec.md), [`specs/009-registry-consumer-status-report-nonzero-mvp/spec.md`](../../specs/009-registry-consumer-status-report-nonzero-mvp/spec.md), [`specs/010-registry-consumer-status-report-json-contract-mvp/spec.md`](../../specs/010-registry-consumer-status-report-json-contract-mvp/spec.md), [`specs/011-registry-consumer-status-report-status-filter-mvp/spec.md`](../../specs/011-registry-consumer-status-report-status-filter-mvp/spec.md), [`specs/012-registry-consumer-list-json-mvp/spec.md`](../../specs/012-registry-consumer-list-json-mvp/spec.md), [`specs/013-registry-consumer-show-json-mvp/spec.md`](../../specs/013-registry-consumer-show-json-mvp/spec.md), [`specs/014-registry-consumer-show-compact-json-mvp/spec.md`](../../specs/014-registry-consumer-show-compact-json-mvp/spec.md), [`specs/015-registry-consumer-list-compact-json-mvp/spec.md`](../../specs/015-registry-consumer-list-compact-json-mvp/spec.md), [`specs/016-registry-consumer-status-report-compact-json-mvp/spec.md`](../../specs/016-registry-consumer-status-report-compact-json-mvp/spec.md), [`specs/017-registry-consumer-shared-json-serialization-helper-mvp/spec.md`](../../specs/017-registry-consumer-shared-json-serialization-helper-mvp/spec.md)): a **read-only CLI** over compiler-emitted **`registry.json`** (Feature **000** shape, produced by **`spec-compiler`**, Feature **001**), including lifecycle/status reporting UX. Feature **017** centralizes pretty vs compact JSON serialization in `serialize_json_compact_or_pretty` (`src/lib.rs`) without changing CLI output.

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
./tools/registry-consumer/target/release/registry-consumer list --json
./tools/registry-consumer/target/release/registry-consumer list --compact
./tools/registry-consumer/target/release/registry-consumer list --status draft --id-prefix 002
./tools/registry-consumer/target/release/registry-consumer list --json --status draft --id-prefix 002
./tools/registry-consumer/target/release/registry-consumer list --compact --status draft --id-prefix 002
./tools/registry-consumer/target/release/registry-consumer show 002-registry-consumer-mvp
./tools/registry-consumer/target/release/registry-consumer show 002-registry-consumer-mvp --json
./tools/registry-consumer/target/release/registry-consumer show 002-registry-consumer-mvp --compact
./tools/registry-consumer/target/release/registry-consumer status-report
./tools/registry-consumer/target/release/registry-consumer status-report --show-ids
./tools/registry-consumer/target/release/registry-consumer status-report --json
./tools/registry-consumer/target/release/registry-consumer status-report --compact
./tools/registry-consumer/target/release/registry-consumer status-report --nonzero-only
./tools/registry-consumer/target/release/registry-consumer status-report --json --nonzero-only
./tools/registry-consumer/target/release/registry-consumer status-report --compact --nonzero-only
./tools/registry-consumer/target/release/registry-consumer status-report --status active
./tools/registry-consumer/target/release/registry-consumer status-report --json --status active
./tools/registry-consumer/target/release/registry-consumer status-report --compact --status active
```

**`--status`** filters on the Feature **000** enum (`draft`, `active`, `superseded`, `retired`). Normative meanings and recommended transitions: [`specs/003-feature-lifecycle-mvp/spec.md`](../../specs/003-feature-lifecycle-mvp/spec.md).

Override path:

```bash
./tools/registry-consumer/target/release/registry-consumer --registry-path /path/to/registry.json list
```

If **`validation.passed`** is **false** in **`registry.json`**, commands fail with exit code **1** unless **`--allow-invalid`** is set (diagnostics only).

`status-report --json` is treated as a stable automation-facing contract and is guarded by fixture-based integration tests.

`status-report --compact` emits the same row array as **`status-report --json`** on **one line**; **`--json`** and **`--compact`** cannot be used together. **`--show-ids`** applies only to **text** mode (not JSON).

`list --json` emits a pretty-printed JSON array of feature objects from **`features[]`**, in the same lexicographic **`id`** order as text **`list`**, with the same **`--status`** and **`--id-prefix`** filters.

`list --compact` emits the same array as **`list --json`** in a **single line** (`serde_json::to_string`); **`--json`** and **`--compact`** cannot be used together.

`show <id> --json` is the explicit automation-facing single-feature JSON path; output matches default **`show <id>`** (pretty-printed feature object) today.

`show <id> --compact` emits a **single-line** compact JSON object (no pretty-print); **`--json`** and **`--compact`** cannot be used together.

## Exit codes

| Code | Meaning |
|------|---------|
| **0** | Success |
| **1** | Feature not found; or registry not authoritative (`validation.passed` false without `--allow-invalid`) |
| **3** | Missing/unreadable file, JSON parse error, or malformed registry for the requested operation |

## Trust model

The tool **does not** re-validate against **`registry.schema.json`**. It parses JSON, enforces **`validation.passed`** (unless **`--allow-invalid`**), and reads **`features[]`**. Feature **001** remains the schema gate.
