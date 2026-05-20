# V-code Emission Audit

**Branch:** `cut-d/autonomous-run-20260519-025506`
**Date:** 2026-05-19
**Method:** `grep` for `"V-NNN"` constants in `tools/spec-compiler/src/`, `tools/shared/spec-types/src/`. Read of every emission site. Cross-reference with `specs/000-bootstrap-spec-system/contracts/registry.schema.json`.

## V-code constant declarations

`tools/shared/spec-types/src/lib.rs:208-225` declares 18 V-codes — V-001 through V-019 with V-009 intentionally absent. No additional V-codes exist outside this constant list.

| code | constant | declared-at |
|---|---|---|
| V-001 | `V_001` | `tools/shared/spec-types/src/lib.rs:208` |
| V-002 | `V_002` | `:209` |
| V-003 | `V_003` | `:210` |
| V-004 | `V_004` | `:211` |
| V-005 | `V_005` | `:212` |
| V-006 | `V_006` | `:213` |
| V-007 | `V_007` | `:214` |
| V-008 | `V_008` | `:215` |
| (V-009 gap) | — | (not declared) |
| V-010 | `V_010` | `:216` |
| V-011 | `V_011` | `:217` |
| V-012 | `V_012` | `:218` |
| V-013 | `V_013` | `:219` |
| V-014 | `V_014` | `:220` |
| V-015 | `V_015` | `:221` |
| V-016 | `V_016` | `:222` |
| V-017 | `V_017` | `:223` |
| V-018 | `V_018` | `:224` |
| V-019 | `V_019` | `:225` |

V-010 is declared in spec-types but **not emitted** by spec-compiler (no `"V-010"` literal in `tools/spec-compiler/src/lib.rs`). Possible dormant code or a spec-lint-side emitter; surface for D7 open question.

## All V-codes — emission classification

| V-NNN | description (compiler message excerpt) | site (`tools/spec-compiler/src/lib.rs`) | trigger field/value | emission classification | schema constraint | I12 disposition |
|---|---|---|---|---|---|---|
| V-001 | `spec.md missing for feature directory` | line 128 | n/a — file presence | **structural** | n/a | no fix |
| V-002 (a) | wrong-typed extra-frontmatter value | line 1267 | key value not scalar | **dropping** — key NOT inserted into `extra` map | `extraFrontmatter.additionalProperties = $ref` | no fix; matches V-007 pattern |
| V-002 (b) | extraFrontmatter exceeds maxProperties (8) | line 1280 | `extra.len() > 8` | **permissive** — entire `extra` map still returned with all entries | `extraFrontmatter.maxProperties = 8` | **fix candidate** — registry violates schema's `maxProperties` if extra has > 8 keys |
| V-002 (c) | duplicate `code_aliases` shape error (must be list of strings) | lines 1171, 1188 | `code_aliases` not a list | **dropping** — function returns Ok(None) for the whole list at line 1176 | `codeAliases.items.pattern` | no fix |
| V-003 | `duplicate feature id` | lines 160, 166 | second spec carrying same id | **structural** — second spec skipped via `continue` at line 171; never appended to features | n/a | no fix |
| V-004 | standalone authored YAML forbidden | line 1068 | repo-walk file with `.yaml`/`.yml` extension not exempt | **structural** — no field is emitted; entire file is the violation | n/a | no fix |
| V-005 | code alias already claimed by other feature | lines 1212, 1218 | alias already in `alias_owner` for different feature_id | **dropping** — `continue` at line 1225 skips inserting the alias | `codeAliases.uniqueItems = true` | no fix; matches V-007 pattern |
| V-006 | code_aliases entry doesn't match pattern | line 1197 | `is_valid_code_alias(s)` returns false | **dropping** — `continue` at line 1204 skips the bad entry | `codeAliases.items.pattern = ^[A-Z][A-Z0-9_]{2,63}$` | no fix; matches V-007 pattern |
| V-007 | invalid `risk` value | line 186 | risk not in `VALID_RISK_LEVELS` | **dropping** — `risk = None` at line 196 | `risk.enum = [low, medium, high, critical]` | **reference implementation** of the pattern |
| V-008 | depends_on references non-existent spec id | line 420 | id not in `all_ids` | **permissive** — dangling string stays in `depends_on` | `dependsOn.items` is plain non-empty string (no resolution constraint) | **not-permissive-given-current-schema** — no fix unless schema adds resolution requirement |
| V-010 | (no emission found) | — | — | **dormant** | n/a | surface open question |
| V-011 | amends_sections overlaps amended spec's `unamendable` | line 467 | section anchor in `frozen` set | **permissive** — `amends_sections` still emitted with the overlapping entry | `amendsSections.items` is plain non-empty string | **not-permissive-given-current-schema** |
| V-012 | `kind` value not in `VALID_KINDS` enum | line 241 | `!VALID_KINDS.contains(&k.as_str())` | **permissive** — `kind` stays in the feature record (no `kind = None` after the violation) | `kind` is `{"type": "string"}` (no enum at schema level) | **not-permissive-given-current-schema** — no fix unless schema tightens kind to enum |
| V-013 (multiple) | per-kind required fields missing (capability/registry/profile) | lines 268, 280, 288, 298, 306, 314 | missing typed companion field | **structural** — about presence/absence of optional schema fields | each missing field is optional at schema level | no fix |
| V-014 | implements shape wrong (scalar with non-capability kind) | lines 334, 344 | `is_scalar && !kind_is_capability` or neither scalar nor list | **permissive** — implements stays as-is | `implements.oneOf = [string, array]` permits both | **not-permissive-given-current-schema** — V-014 enforces kind-aware policy above schema |
| V-015 (multiple) | capability/registry link integrity, selectable_by/selector equality | lines 514, 525, 538 | target spec missing, wrong kind, or mismatched selector | **permissive** — implements + selectable_by stay | string fields stay schema-conformant | **not-permissive-given-current-schema** |
| V-016 | corpus-wide primary-flag uniqueness violated | line 590 | path has `primary: true` on multiple specs | **permissive** — all primaries stay | `primary: boolean` — no uniqueness constraint in schema | **not-permissive-given-current-schema** |
| V-017 (multiple) | profile selects-target validity | lines 615, 635, 646, 677 | selects key/value doesn't resolve, or capability doesn't implement registry | **permissive** — selects map stays | `selects.additionalProperties: string` — no resolution constraint | **not-permissive-given-current-schema** |
| V-018 | status=retired requires retirement_rationale | line 358 | status="retired" && retirement_rationale None | **permissive** — status="retired" stays even with missing companion | `status.enum` includes "retired"; `retirementRationale` is optional object | **not-permissive-given-current-schema** — V-018 enforces companion policy above schema |
| V-019 | supersession back-link presence/resolution | lines 700, 708 | status="superseded" with None or dangling superseded_by | **permissive** — status="superseded" stays | `supersededBy` is optional string | **not-permissive-given-current-schema** |

## Permissive V-codes needing I12 fix

| V-NNN | current behavior | proposed fix pattern | affected fields | schema constraint that would be violated |
|---|---|---|---|---|
| V-002 (b) | `extraFrontmatter` map with > 8 entries is emitted as-is | After violation, **truncate `extra` to 8 entries** (keeping deterministic order — alphabetically first 8) and emit the truncated map. The V-002 (b) violation remains the source of truth for the rejection. | `extraFrontmatter` | `maxProperties = 8` → registry self-validation fires |

That is the only true permissive case. It matches the V-007 pattern precisely: when emission would violate schema, the offending content is removed; the diagnostic is the source of truth.

## Dropping V-codes (reference — already follow V-007 pattern)

| V-NNN | drop site | what gets dropped |
|---|---|---|
| V-005 | `tools/spec-compiler/src/lib.rs:1225` (`continue` in seq-loop) | duplicate alias entry; not added to feature's `out` list |
| V-006 | line 1204 (`continue`) | code_aliases entry with invalid pattern |
| V-007 | line 196 (`risk = None`) | invalid risk value |
| V-002 (a) | line 1267 (`None =>` branch — key not inserted) | wrong-typed extra-frontmatter value |
| V-002 (c) | line 1176 (`return Ok(None)`) | entire malformed code_aliases sequence |

## Structural V-codes

| V-NNN | shape concern |
|---|---|
| V-001 | spec.md file presence per feature directory |
| V-003 | duplicate feature id — second occurrence skipped, never enters features list |
| V-004 | standalone YAML files forbidden at repo root walk |
| V-013 | per-kind required-field presence — schema fields are individually optional; V-013 enforces presence policy |

## Not-permissive-given-current-schema (no fix needed unless schema tightens)

Eight V-codes record violations whose underlying schema permits the emitted value. The registry stays schema-conformant despite the violation. If the schema is ever tightened (e.g., `kind` gets an enum; `dependsOn` gets a resolution requirement), these would become permissive and need the V-007 pattern.

| V-NNN | what would need to tighten in schema |
|---|---|
| V-008 | `dependsOn.items.pattern` becomes `^[corpus-resolved-id]$` (not expressible without runtime knowledge) — practically: no fix |
| V-011 | `amendsSections` cross-spec resolution constraint — not expressible in JSON Schema |
| V-012 | `kind.enum = [capability, registry, profile, ...]` — **schema tightening is feasible**; spec 147 already defines `VALID_KINDS`, so adding an enum to the schema would make the V-012 fix trivial (drop kind to None when invalid). Surface as follow-up for the spec spine. |
| V-014 | `implements` `oneOf` already permits both shapes; tightening would couple to kind, which is one-of-many — not currently expressible |
| V-015 | requires runtime resolution — not expressible |
| V-016 | requires corpus-wide uniqueness — not expressible without registry-level constraint type |
| V-017 | requires runtime resolution + cross-spec graph — not expressible |
| V-018 | requires `if status==retired then required retirementRationale` — expressible via JSON Schema 2020-12 `if/then/else`. **Schema tightening is feasible.** |
| V-019 | requires `if status==superseded then required supersededBy && resolution` — partially expressible (presence yes, resolution no) |

## I12 readiness summary

- **Permissive V-codes to fix:** 1 (V-002 (b) — extraFrontmatter over-size truncation)
- **Estimated commits in I12:** 1 (single targeted fix in `tools/spec-compiler/src/lib.rs` near line 1280, plus a regression test)
- **Estimated complexity:** **low** — single function edit, single test file.
- **Out-of-scope follow-ups surfaced by D7:**
  - **V-010 dormancy** — code is declared in spec-types but never emitted. Either remove the constant or document the intended emitter (spec-lint? a planned check?). Surface as open question.
  - **Schema tightening opportunity for V-012 / V-018** — adding `kind.enum` to the registry schema would convert V-012 from policy-only to schema-enforceable; an `if/then` for V-018 would do likewise. These are policy-level decisions, not Epic 2 work.

## Open questions (surface for operator triage)

1. **V-010 dormancy.** Constant `V_010` is declared but no emission site exists in `tools/spec-compiler/src/lib.rs`. Confirm: (a) intended emitter is spec-lint (check `tools/spec-lint/src/`), (b) intended but unimplemented, or (c) reserved for future. Recommendation: I12 audits spec-lint too; if dormant, file a follow-up to either implement or remove.
2. **V-002 (b) fix semantics.** Truncating extraFrontmatter to the first 8 keys (alphabetical or insertion-order?) is a deterministic choice. Confirm operator preference; recommendation is alphabetical for predictability.
3. **`extraFrontmatter` policy direction.** Master plan defers schema tightening to follow-up; D7 confirms that V-002 (b) is the only permissive case under the current schema. If the schema were tightened (V-012 enum, V-018 if/then), the V-007 pattern would extend mechanically; the audit is forward-compatible.

## Cross-phase notes

- I12 lands after I4 (schema co-location). Schema move does not change schema content; V-002 (b) fix is independent of the move.
- I12 is the only phase that *modifies* spec-compiler logic. All other Epic 2 phases either move paths or update path references.
- The fix is small enough to be a single commit per master plan §Epic 2 phase index.
