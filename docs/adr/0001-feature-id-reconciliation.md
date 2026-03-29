# ADR 0001: Feature ID reconciliation (code ↔ spec registry)

| Field | Value |
|-------|--------|
| Status | **Accepted** (design intent; schema/compiler work tracked under Feature 039) |
| Date | 2026-03-29 |
| Context | Slice E — `.ai/plans/next-slice.md` |

## Context

Two parallel identifier systems exist:

1. **Spec registry (canonical)** — Kebab-case IDs with a three-digit numeric prefix, matching `specs/<id>/` and `FeatureRecord.id` in `registry.json` (see `specs/000-bootstrap-spec-system/contracts/registry.schema.json`).
2. **Code attribution** — `// Feature: UPPERCASE_TOKEN` (and `#` variants) scanned by `featuregraph` (`crates/featuregraph/src/scanner.rs`), using semantic names such as `FEATUREGRAPH_REGISTRY` that do not encode the spec folder slug.

There is no stable bridge in compiled registry JSON between these systems, so governance consumers cannot join spec metadata to scanned code ownership. Legacy YAML alias plumbing does not apply to the deterministic compiled registry path.

## Decision

- **Canonical feature identity** remains the **kebab-case `id`** in `registry.json` (and spec frontmatter). No change to directory layout, compiler primary key, or `specPath` rules.
- **Code-side tokens** are treated as **aliases** of that canonical id. The compiled registry gains an optional, deterministic field on each feature record — working name **`codeAliases`**: a JSON array of unique strings matching the existing scanner token shape (`[A-Z][A-Z0-9_]{2,63}`), sorted lexicographically for stable output.
- **Population strategy (for implementation):** merge alias sets from (a) optional spec frontmatter approved in Feature 039, (b) scanner-derived attribution grouped by resolved feature, with validation that **each alias appears under at most one feature** and CI/spec-compiler violations if ambiguous or orphaned mappings are forbidden by policy.

This corresponds to **option (a)** in the next-slice plan: extend the compiled registry so both human/spec ids and code tokens are first-class for matching.

## Rationale

| Option | Verdict |
|--------|---------|
| **(a) Aliases in compiled registry** | **Chosen.** Preserves kebab ids as single source of truth for specs; avoids mass header churn; handles many-to-one (several code tokens → one feature) explicitly; aligns with deterministic-registry goals if arrays are sorted and merge rules are fixed. |
| **(b) Convention-derived UPPERCASE from kebab slug** | Rejected. Slugs are long and not isomorphic to semantic bundle names (`032-opc-…` vs `OPC_INSPECT_…`); automatic derivation is noisy and breaks when one feature legitimately uses multiple distinct code tokens. |
| **(c) Kebab everywhere in code headers** | Rejected. Very large change surface; `scanner` today encodes UPPERCASE in `FEATURE_REGEX`; widespread churn for limited gain when explicit aliases solve linking without renaming every file’s header. |

## Consequences

### Positive

- Governance UI and tools can resolve `// Feature: FOO` → kebab `id` via `codeAliases` without ambiguous heuristics.
- Spec workflow remains anchored on `specs/<id>/`; no second “primary” id system.

### Negative / follow-up

- **Schema and compiler** must be extended (`registry.schema.json`, `tools/spec-compiler`, validation rules) — scoped to Feature 039.
- **Content maintenance:** new or renamed code tokens may require frontmatter or alias tables until scanner grouping is fully automated; document in Feature 039 tasks.
- **Consumers** (`registry-consumer`, featuregraph, desktop) should treat **`id` as canonical** and use `codeAliases` only for lookup; optional field preserves backward compatibility until consumers opt in.

## References

- `specs/000-bootstrap-spec-system/contracts/registry.schema.json` — `featureRecord.id` pattern.
- `crates/featuregraph/src/scanner.rs` — `FEATURE_REGEX`, attribution model.
- `.ai/plans/next-slice.md` — Slice E scope and options (a)–(c).
