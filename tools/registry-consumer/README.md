# registry-consumer

Implements **Feature 002**, **Feature 007**, **Feature 008**, **Feature 009**, **Feature 010**, **Feature 011**, **Feature 012**, **Feature 013**, **Feature 014**, **Feature 015**, **Feature 016**, **Feature 017**, **Feature 018**, **Feature 019**, **Feature 020**, **Feature 021**, **Feature 022**, **Feature 023**, **Feature 024**, **Feature 025**, and **Feature 026** ([`specs/002-registry-consumer-mvp/spec.md`](../../specs/002-registry-consumer-mvp/spec.md), [`specs/007-registry-consumer-status-report-mvp/spec.md`](../../specs/007-registry-consumer-status-report-mvp/spec.md), [`specs/008-registry-consumer-status-report-json-mvp/spec.md`](../../specs/008-registry-consumer-status-report-json-mvp/spec.md), [`specs/009-registry-consumer-status-report-nonzero-mvp/spec.md`](../../specs/009-registry-consumer-status-report-nonzero-mvp/spec.md), [`specs/010-registry-consumer-status-report-json-contract-mvp/spec.md`](../../specs/010-registry-consumer-status-report-json-contract-mvp/spec.md), [`specs/011-registry-consumer-status-report-status-filter-mvp/spec.md`](../../specs/011-registry-consumer-status-report-status-filter-mvp/spec.md), [`specs/012-registry-consumer-list-json-mvp/spec.md`](../../specs/012-registry-consumer-list-json-mvp/spec.md), [`specs/013-registry-consumer-show-json-mvp/spec.md`](../../specs/013-registry-consumer-show-json-mvp/spec.md), [`specs/014-registry-consumer-show-compact-json-mvp/spec.md`](../../specs/014-registry-consumer-show-compact-json-mvp/spec.md), [`specs/015-registry-consumer-list-compact-json-mvp/spec.md`](../../specs/015-registry-consumer-list-compact-json-mvp/spec.md), [`specs/016-registry-consumer-status-report-compact-json-mvp/spec.md`](../../specs/016-registry-consumer-status-report-compact-json-mvp/spec.md), [`specs/017-registry-consumer-shared-json-serialization-helper-mvp/spec.md`](../../specs/017-registry-consumer-shared-json-serialization-helper-mvp/spec.md), [`specs/018-registry-consumer-list-show-json-contract-mvp/spec.md`](../../specs/018-registry-consumer-list-show-json-contract-mvp/spec.md), [`specs/019-registry-consumer-readme-examples-contract-mvp/spec.md`](../../specs/019-registry-consumer-readme-examples-contract-mvp/spec.md), [`specs/020-registry-consumer-error-contract-mvp/spec.md`](../../specs/020-registry-consumer-error-contract-mvp/spec.md), [`specs/021-registry-consumer-field-shape-invariants-contract-mvp/spec.md`](../../specs/021-registry-consumer-field-shape-invariants-contract-mvp/spec.md), [`specs/022-registry-consumer-help-usage-contract-mvp/spec.md`](../../specs/022-registry-consumer-help-usage-contract-mvp/spec.md), [`specs/023-registry-consumer-flag-conflict-argument-validation-contract-mvp/spec.md`](../../specs/023-registry-consumer-flag-conflict-argument-validation-contract-mvp/spec.md), [`specs/024-registry-consumer-version-banner-contract-mvp/spec.md`](../../specs/024-registry-consumer-version-banner-contract-mvp/spec.md), [`specs/025-registry-consumer-default-path-contract-mvp/spec.md`](../../specs/025-registry-consumer-default-path-contract-mvp/spec.md), [`specs/026-registry-consumer-allow-invalid-contract-mvp/spec.md`](../../specs/026-registry-consumer-allow-invalid-contract-mvp/spec.md)): a **read-only CLI** over compiler-emitted **`registry.json`** (Feature **000** shape, produced by **`spec-compiler`**, Feature **001**), including lifecycle/status reporting UX. Feature **017** centralizes pretty vs compact JSON serialization in `serialize_json_compact_or_pretty` (`src/lib.rs`) without changing CLI output. Feature **018** adds fixture contract tests for **`list`/`show`** JSON and compact output (similar to Feature **010** for **`status-report --json`**). Feature **019** ties README examples to the same committed transcripts as integration tests (CLI + fenced blocks). Feature **020** locks key runtime failure-path stderr and exit-code contracts via deterministic fixtures. Feature **021** locks object field-shape invariants (required keys, omitted optionals, and stable key order) across list/show/status-report JSON paths. Feature **022** locks top-level and subcommand help/usage output transcripts for stable operator and agent ergonomics. Feature **023** locks argument-layer conflict and validation behavior (flag conflicts, missing required args, invalid enum value) with exact stderr and exit-code contracts. Feature **024** locks top-level `--version` banner output, exit code, and stderr behavior. Feature **025** locks omitted-`--registry-path` default-location behavior for both success and missing-path failure semantics from controlled working directories. Feature **026** locks `--allow-invalid` policy-override behavior, including boundaries where malformed registries still fail.

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

## Verified examples (Feature 019)

The transcripts below are **contract-tested**: `cargo test` checks that the CLI matches the files under `tests/fixtures/readme_examples/expected/`, and that these fenced blocks match those files. Registries are committed next to the tests.

**Registries**

- `tests/fixtures/readme_examples/registry_list_show.json` — **list** / **show**
- `tests/fixtures/readme_examples/registry_status_report.json` — **status-report**

From the repository root, using the **release** binary (paths below are relative to the repo root).

### Human-facing (terminal tables and readable JSON)

**`list` (text table)**

```bash
./tools/registry-consumer/target/release/registry-consumer --registry-path tools/registry-consumer/tests/fixtures/readme_examples/registry_list_show.json list
```

<!-- readme-contract:list-text -->
```text
id                                           status     title
001-a                                        active     First
002-b                                        draft      Second
```
<!-- /readme-contract:list-text -->

**`show` (default output; same object as `show --json`)**

```bash
./tools/registry-consumer/target/release/registry-consumer --registry-path tools/registry-consumer/tests/fixtures/readme_examples/registry_list_show.json show 001-a
```

<!-- readme-contract:show-text -->
```json
{
  "created": "2026-03-22",
  "id": "001-a",
  "sectionHeadings": [
    "H"
  ],
  "specPath": "specs/001-a/spec.md",
  "status": "active",
  "summary": "sum",
  "title": "First"
}
```
<!-- /readme-contract:show-text -->

**`status-report` (counts per status)**

```bash
./tools/registry-consumer/target/release/registry-consumer --registry-path tools/registry-consumer/tests/fixtures/readme_examples/registry_status_report.json status-report
```

<!-- readme-contract:status-report-text -->
```text
draft      1
active     1
superseded 1
retired    1
```
<!-- /readme-contract:status-report-text -->

### Automation-facing (JSON and compact)

**`list --json` / `list --compact`**

```bash
./tools/registry-consumer/target/release/registry-consumer --registry-path tools/registry-consumer/tests/fixtures/readme_examples/registry_list_show.json list --json
./tools/registry-consumer/target/release/registry-consumer --registry-path tools/registry-consumer/tests/fixtures/readme_examples/registry_list_show.json list --compact
```

<!-- readme-contract:list-json -->
```json
[
  {
    "created": "2026-03-22",
    "id": "001-a",
    "sectionHeadings": [
      "H"
    ],
    "specPath": "specs/001-a/spec.md",
    "status": "active",
    "summary": "sum",
    "title": "First"
  },
  {
    "created": "2026-03-22",
    "id": "002-b",
    "sectionHeadings": [
      "H"
    ],
    "specPath": "specs/002-b/spec.md",
    "status": "draft",
    "summary": "sum",
    "title": "Second"
  }
]
```
<!-- /readme-contract:list-json -->

<!-- readme-contract:list-compact -->
```json
[{"created":"2026-03-22","id":"001-a","sectionHeadings":["H"],"specPath":"specs/001-a/spec.md","status":"active","summary":"sum","title":"First"},{"created":"2026-03-22","id":"002-b","sectionHeadings":["H"],"specPath":"specs/002-b/spec.md","status":"draft","summary":"sum","title":"Second"}]
```
<!-- /readme-contract:list-compact -->

**`show --json` / `show --compact`**

```bash
./tools/registry-consumer/target/release/registry-consumer --registry-path tools/registry-consumer/tests/fixtures/readme_examples/registry_list_show.json show 001-a --json
./tools/registry-consumer/target/release/registry-consumer --registry-path tools/registry-consumer/tests/fixtures/readme_examples/registry_list_show.json show 001-a --compact
```

<!-- readme-contract:show-json -->
```json
{
  "created": "2026-03-22",
  "id": "001-a",
  "sectionHeadings": [
    "H"
  ],
  "specPath": "specs/001-a/spec.md",
  "status": "active",
  "summary": "sum",
  "title": "First"
}
```
<!-- /readme-contract:show-json -->

<!-- readme-contract:show-compact -->
```json
{"created":"2026-03-22","id":"001-a","sectionHeadings":["H"],"specPath":"specs/001-a/spec.md","status":"active","summary":"sum","title":"First"}
```
<!-- /readme-contract:show-compact -->

**`status-report --json` / `status-report --compact`**

```bash
./tools/registry-consumer/target/release/registry-consumer --registry-path tools/registry-consumer/tests/fixtures/readme_examples/registry_status_report.json status-report --json
./tools/registry-consumer/target/release/registry-consumer --registry-path tools/registry-consumer/tests/fixtures/readme_examples/registry_status_report.json status-report --compact
```

<!-- readme-contract:status-report-json -->
```json
[
  {
    "count": 1,
    "ids": [
      "004-d"
    ],
    "status": "draft"
  },
  {
    "count": 1,
    "ids": [
      "001-a"
    ],
    "status": "active"
  },
  {
    "count": 1,
    "ids": [
      "003-c"
    ],
    "status": "superseded"
  },
  {
    "count": 1,
    "ids": [
      "002-b"
    ],
    "status": "retired"
  }
]
```
<!-- /readme-contract:status-report-json -->

<!-- readme-contract:status-report-compact -->
```json
[{"count":1,"ids":["004-d"],"status":"draft"},{"count":1,"ids":["001-a"],"status":"active"},{"count":1,"ids":["003-c"],"status":"superseded"},{"count":1,"ids":["002-b"],"status":"retired"}]
```
<!-- /readme-contract:status-report-compact -->

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
