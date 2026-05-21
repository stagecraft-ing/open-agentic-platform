# Segment 3 — Implementation Handoff

**Status:** Operational handoff. Lifetime ends when Segment 3 lands.
Delete or archive in the Segment-3 closure PR — this doc is ephemeral
working state, not governance.

You are picking up **Tier 2 Segment 3** of the spec 154
(logical-unit-ownership-grammar) rollout: the codebase-indexer
logical-unit resolver. The design pass is complete and every open
question is closed. Implementation only.

---

## Ground truth

Read these, in this order, before writing any code:

1. **Design doc** — [`specs/154-logical-unit-ownership-grammar/segment-3-design.md`](segment-3-design.md). The full design with OQ-1..OQ-7 closures. Cite this as the implementation reference; deviations need surface-and-discuss, not silent improvisation.
2. **Spec 154** — [`specs/154-logical-unit-ownership-grammar/spec.md`](spec.md), as amended by spec 155. §3 (per-kind semantics), §3.7 (standard exclusions), §6 (authority computation step 1: "Diff hunk → logical units").
3. **Spec 155** — [`specs/155-logical-unit-resolution-semantics/spec.md`](../155-logical-unit-resolution-semantics/spec.md). The amendment text that closed OQ-1..OQ-4.
4. **Four-commit chain (ground truth for the segment boundary):**
   - `3ad3eff1` (PR #176) — cut-D structural cleanup; spec-spine crate split, relationship graph plumbing.
   - `2b1fe8a3` (PR #183) — Segment 2; spec-compiler parses + type-checks logical units (V-021..V-024 + L-005).
   - `b941bf60` (PR #185) — Commit A; spec 155 lands + spec 154 amended + design doc committed.
   - `fda92edc` (PR #185) — Commit B; V-024 predicate expansion (`<`/`>` in symbol id).
5. **Tier 2 plan** — the prior session's handoff narrative (in the original session log; not yet checked in as a markdown artifact). Segment 3 is the resolver only — Segments 4 (gate refactor), 5 (corpus migration), 6 (legacy excision) are downstream.

---

## Scope

**In:** the codebase-indexer resolver per the design doc §1.

**Out:** anything in Segments 4–6. If implementation surfaces a Segment-4 or Segment-5 question, capture it in this handoff doc under "Surfaced for downstream" and keep moving.

---

## Constraints carried forward (do not re-derive — these are settled)

### From spec 155 amendments

- **`symbol:` id** — Rust item path only. Generic / lifetime / turbofish syntax (`<`, `>`) is V-024-rejected at parse time. The resolver's symbol index keys bare paths; matching is exact.
- **`module:` missing** — hard error.
- **`directory:` missing** — hard error.
- **`file:` missing in compile context** — unconditionally hard error; rename-trace following is the Segment-4 gate's concern, not the resolver's.

### From OQ-5 / OQ-6 verification

- **Schema version 2.0.0 → 2.1.0 is safe.** The two real consumers (`spec-code-coupling-check/src/lib.rs:298`, `oap-code-index-enrich/src/lib.rs:156`) use major-only comparison or field-touched-but-not-compared semantics. The Segment 3 bump can proceed without coordination.
- **`tree-sitter-rust` is already vendored** at `tools/vendor/grammars/tree-sitter-rust`. Mirror xray's Cargo pattern at `crates/xray/Cargo.toml:37` for the dep declaration; no new vendoring required.

### From OQ-7 decision

- **Inline-module span** includes the `mod foo {` declaration line. The `foo` identifier is part of module identity; excluding the declaration line would mean renaming `mod foo` to `mod bar` doesn't trigger the gate, contradicting the symbol/module-identity model.

### Auto-memory items (already persisted; do not re-author)

- `project_spec_154_segment_5_l005_worklist` — Segment 5 migration script must consume L-005 advisories as its candidate set, not re-derive.
- `project_spec_154_segment_6_explicit_only_flip` — Segment 6 closure must flip V-021..V-024's explicit-only gating so the checks apply uniformly post-excision.

---

## Stop conditions specific to Segment 3

Surface rather than improvise if any of the following:

1. **Tree-sitter symbol-extraction surfaces a Rust construct the design doc didn't anticipate.** Macros that synthesize items, `cfg`-gated items, item paths through type aliases, etc. The design doc's `symbol_index` module is sketched, not exhaustively specified — if real corpus content breaks the bare-path assumption, stop and discuss.
2. **Resolver performance materially regresses past the 10s warm-compile target** (design doc §7). `make ci`'s fast loop is the envelope; resolver compile-time cost must stay inside `make registry` and not bleed into `make ci`.
3. **A corpus spec resolves cleanly under the new resolver but produces a logical-unit set that doesn't match the prior path-list set.** That is a Segment 5 migration question (unit drift surfacing semantic gaps in the prior path-list claim), not a Segment 3 resolver fix. Capture in this doc under "Surfaced for downstream"; do not auto-correct.
4. **Determinism contract violation.** Two compile runs producing different `index.json` bytes is a hard stop. The sort key in design doc §3 must be enforced at function boundaries, not at caller sites.

---

## Operating principles (reminder)

- **One segment, one commit, one concern.** Segment 3 lands as one PR. Within the PR, commits may be split by concern (e.g. types + indexer module + tests) but the segment as a whole is one reviewable unit.
- **Amend FIRST then implement.** If implementation surfaces spec imprecision in spec 154 or 155, halt and surface — do not backfill spec text to match code. Spec 155 itself was authored precisely because this discipline holds.
- **SOLID over rushing.** Architect-agent design pass was the planning step; implementation follows the design doc. Skipping the resolver-shape design and inventing a different shape mid-implementation is exactly the failure mode this sequence is built to prevent.
- **Surface stop conditions, do not improvise.** When any of the four stop conditions above hits, halt and present the choice. That has been the discipline throughout this Tier 2 mission; it holds for Segment 3.

---

## Surfaced for downstream (Segment 4 / 5 / 6 capture)

(Empty at handoff time. Append items here as Segment 3 implementation surfaces them.)

---

## After Segment 3 lands

- This doc gets deleted (or archived) in the Segment-3 closure PR.
- Auto-memory items above remain — they're consumed by Segments 5 and 6.
- Segment 4 (gate refactor) begins: refactor `spec-code-coupling-check` to consume `ResolvedUnit` via the public API seam defined in design doc §10.
