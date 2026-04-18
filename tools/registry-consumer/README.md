# registry-consumer

A **read-only CLI** over compiler-emitted **`registry.json`** (Feature **000** shape, produced by **`spec-compiler`**, Feature **001**). Provides `list`, `show`, and `status-report` subcommands with pretty / compact / ids-only JSON variants, status and id-prefix filters, and a `--allow-invalid` policy override.

Traceability for the per-surface contracts lives in the compiled registry — query it rather than enumerating specs here:

```bash
./tools/registry-consumer/target/release/registry-consumer list --id-prefix 0 --status approved
```

Governed contract surfaces (normative; fixture-backed under `tests/fixtures/`):

- **JSON shape / content / ordering** — `list`, `show`, `status-report` pretty and compact outputs
- **Field-shape invariants** — required keys, omitted optionals, stable key order
- **README examples** — fenced blocks in this file are fixture-tested verbatim
- **Error / exit contracts** — runtime failure stderr and exit codes
- **Help / version contracts** — top-level and subcommand usage banners
- **Argument validation** — flag conflicts, missing required args, invalid enums
- **Default-path and `--allow-invalid`** — `registry-path` resolution and override semantics
- **Sorting order** — list order, status-report rows, ids within rows
- **stdout/stderr channel discipline** — across every representative scenario
- **`list --ids-only`** — line-oriented id stream for automation

Governance doctrine (change classification, extension acceptance rubric, release gate): [`docs/registry-consumer-contract-governance.md`](../../docs/registry-consumer-contract-governance.md).

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

## Verified examples

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
001-a                                        approved   First
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
  "status": "approved",
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
approved   1
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
    "status": "approved",
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
[{"created":"2026-03-22","id":"001-a","sectionHeadings":["H"],"specPath":"specs/001-a/spec.md","status":"approved","summary":"sum","title":"First"},{"created":"2026-03-22","id":"002-b","sectionHeadings":["H"],"specPath":"specs/002-b/spec.md","status":"draft","summary":"sum","title":"Second"}]
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
  "status": "approved",
  "summary": "sum",
  "title": "First"
}
```
<!-- /readme-contract:show-json -->

<!-- readme-contract:show-compact -->
```json
{"created":"2026-03-22","id":"001-a","sectionHeadings":["H"],"specPath":"specs/001-a/spec.md","status":"approved","summary":"sum","title":"First"}
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
    "status": "approved"
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
[{"count":1,"ids":["004-d"],"status":"draft"},{"count":1,"ids":["001-a"],"status":"approved"},{"count":1,"ids":["003-c"],"status":"superseded"},{"count":1,"ids":["002-b"],"status":"retired"}]
```
<!-- /readme-contract:status-report-compact -->

## Usage (repository root)

Default registry path: **`build/spec-registry/registry.json`** relative to the **current working directory** (same convention as **`spec-compiler`**).

```bash
./tools/registry-consumer/target/release/registry-consumer list
./tools/registry-consumer/target/release/registry-consumer list --json
./tools/registry-consumer/target/release/registry-consumer list --compact
./tools/registry-consumer/target/release/registry-consumer list --ids-only
./tools/registry-consumer/target/release/registry-consumer list --status draft --id-prefix 002
./tools/registry-consumer/target/release/registry-consumer list --json --status draft --id-prefix 002
./tools/registry-consumer/target/release/registry-consumer list --compact
./tools/registry-consumer/target/release/registry-consumer list --ids-only --status draft --id-prefix 002
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
./tools/registry-consumer/target/release/registry-consumer status-report --status approved
./tools/registry-consumer/target/release/registry-consumer status-report --json --status approved
./tools/registry-consumer/target/release/registry-consumer status-report --compact --status approved
```

**`--status`** filters on the Feature **000** enum (`draft`, `approved`, `superseded`, `retired`). Normative meanings and recommended transitions: [`specs/003-feature-lifecycle-mvp/spec.md`](../../specs/003-feature-lifecycle-mvp/spec.md).

Override path:

```bash
./tools/registry-consumer/target/release/registry-consumer --registry-path /path/to/registry.json list
```

If **`validation.passed`** is **false** in **`registry.json`**, commands fail with exit code **1** unless **`--allow-invalid`** is set (diagnostics only).

`status-report --json` is treated as a stable automation-facing contract and is guarded by fixture-based integration tests.

`status-report --compact` emits the same row array as **`status-report --json`** on **one line**; **`--json`** and **`--compact`** cannot be used together. **`--show-ids`** applies only to **text** mode (not JSON).

`list --json` emits a pretty-printed JSON array of feature objects from **`features[]`**, in the same lexicographic **`id`** order as text **`list`**, with the same **`--status`** and **`--id-prefix`** filters.

`list --compact` emits the same array as **`list --json`** in a **single line** (`serde_json::to_string`); **`--json`** and **`--compact`** cannot be used together.

`list --ids-only` emits the same filtered, id-sorted feature set as text/json list modes, but as one feature id per line; `--ids-only` is mutually exclusive with `--json` and `--compact`.

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
