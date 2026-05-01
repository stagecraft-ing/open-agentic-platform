---
id: "125-schema-parity-walker-rebuild"
slug: schema-parity-walker-rebuild
title: Schema-Parity Walker — Rebuild for Hand-Rolled Validators
status: draft
implementation: pending
owner: bart
created: "2026-05-01"
risk: medium
summary: >
  Restores the `make ci-schema-parity` gate after commit `b6859d3` removed
  zod from `api/knowledge/extractionOutput.ts` in favour of hand-rolled
  TypeScript validators (the architectural fix for an Encore.ts parser
  crash). The current parity tool walks a zod schema tree; with no zod
  schema present it short-circuits with `missing exports` and the gate
  fails. This spec replaces the zod walker with a Validator-aware
  introspection path so `extractionOutputSchema` is no longer required
  and the Rust↔TS fingerprint check resumes its job.
depends_on:
  - "120"  # factory-extraction-stage (originating spec for the parity check)
  - "121"  # claim-provenance-enforcement (parity extension precedent)
  - "122"  # stakeholder-doc-inversion (third parity surface)
implements:
  - path: tools/schema-parity-check/index.mjs
  - path: platform/services/stagecraft/api/knowledge/extractionOutput.ts
---

# 125 — Schema-Parity Walker — Rebuild for Hand-Rolled Validators

## 1. Problem Statement

`make ci-schema-parity` (and therefore `make ci`, `make registry`) fails on
`main` HEAD as of 2026-05-01 with:

```
schema-parity-check: platform/services/stagecraft/api/knowledge/extractionOutput.ts is missing exports
  Required: extractionOutputSchema, KNOWLEDGE_SCHEMA_VERSION
```

Origin: commit `b6859d3 fix(stagecraft): hand-roll API validators, drop zod
from Encore parse path` (2026-05-01) replaced the zod schema in
`extractionOutput.ts` with TypeScript interfaces + a `Validator` class.
The architectural fix was correct: Encore.ts's TS parser crashed walking
`zod/v4/classic/schemas.d.cts`. But the parity walker
(`tools/schema-parity-check/index.mjs`) was built against the zod tree —
it imports `extractionOutputSchema` and recursively introspects
`zod._def.type` — and now short-circuits before any comparison happens.

The Rust mirror in `crates/factory-contracts/src/knowledge.rs` still
emits the same fingerprint shape as before. The drift gate on the Rust
side is healthy; only the TS side of the comparison is broken.

While the gate is broken, real Rust↔TS schema drift can land on `main`
unobserved — the original justification for the parity check (spec 120
FR-003/FR-004) regresses. Spec 121's provenance schema and spec 122's
stakeholder-doc grammar parity checks share the same walker and are
therefore on the same blast radius.

## 2. Decision

Two options were considered:

A. **Re-add zod alongside the validator.** Reject. The zod removal was
   load-bearing for Encore parser stability; layering zod back in (even
   in a separate file) re-introduces the crash surface unless we can
   prove the Encore parser only walks files imported transitively from
   API handlers, which is non-trivial to maintain.

B. **Make the walker introspect the new validator.** Accept. Author a
   small **schema descriptor** alongside the `Validator` class — a plain
   data structure that mirrors the field tree the validator already
   walks — and update `tools/schema-parity-check/index.mjs` to walk that
   descriptor instead of zod. The descriptor is plain TS objects, so it
   never triggers the Encore parser bug.

The descriptor sits in the same file as the validator and is exercised
by an in-file unit test ("the descriptor matches what `Validator`
actually checks") so drift between the two is caught locally rather
than at the parity gate.

## 3. Implementation

### 3.1 Schema descriptor

In `platform/services/stagecraft/api/knowledge/extractionOutput.ts`:

```ts
export const KNOWLEDGE_SCHEMA_VERSION = "1.0.0" as const;

// Plain-data descriptor of the structure the `Validator` enforces. Used
// by the schema-parity tool (spec 125) to compute a fingerprint without
// depending on zod.
export const extractionOutputDescriptor: SchemaNode = {
  kind: "object",
  fields: [
    { name: "schemaVersion", required: true, type: { kind: "string" } },
    { name: "kind", required: true, type: { kind: "enum", values: ["text", "pdf", "image"] } },
    { name: "extractor", required: true, type: extractorDescriptor },
    { name: "agentRun", required: false, type: agentRunDescriptor },
    { name: "pages", required: false, type: { kind: "array", element: pageDescriptor } },
    // ...full tree below
  ],
};
```

`SchemaNode` is the same union shape (`object | array | enum | string |
int | number | boolean | unknown | tuple | discriminatedUnion`) the
parity walker already builds; lifting it into the validator file means
the walker doesn't have to translate.

### 3.2 Parity walker rewrite

`tools/schema-parity-check/index.mjs` currently:

1. Imports `extractionOutputSchema` (zod tree).
2. Walks it via `walkType(zod._def)`.
3. Compares the result to the Rust fingerprint at
   `build/schema-parity/rust-knowledge-schema.json`.

After spec 125:

1. Imports `extractionOutputDescriptor` (plain TS data).
2. Walks the descriptor via a new `walkDescriptor(node)` that maps
   `SchemaNode` → fingerprint. The function is a strict, no-side-effects
   tree walker; no zod dependency anywhere.
3. Comparison logic against the Rust fingerprint is unchanged.

The provenance and stakeholder-doc parity surfaces stay zod-walking for
now (their TS mirrors don't exist yet on the spec 121 §8 reserved-mode
path), but the walker code path is split so the descriptor walker and
the zod walker live behind a `walk(node)` dispatcher keyed on the input
shape. Once specs 121 and 122 land their TS mirrors as descriptors, the
zod walker can be deleted entirely.

The descriptor pattern is the canonical answer for any future schema
that needs Rust↔TS parity without zod. Spec 123's duplex envelope types
(`AgentCatalogUpdated` etc.) are not currently parity-checked — they
use `ts-rs` to push Rust types to TS, not a fingerprinted comparison —
but should that direction ever reverse, descriptors are the recommended
shape.

### 3.3 In-file consistency test

A vitest case in `extractionOutput.test.ts` (or a new `descriptor.test.ts`)
asserts that the descriptor shape matches what `Validator` actually
checks for a representative valid + invalid payload pair. Specifically:

- Each required field in the descriptor must trip the validator when
  removed.
- Each non-required field must not trip the validator when removed.
- Each enum-typed field must reject a string outside the enum.

This is a **local** drift gate; it runs under `vitest` and catches
descriptor↔validator drift before a commit, complementing the parity
gate that catches descriptor↔Rust drift.

## 4. Migration

A single PR:

1. Adds `extractionOutputDescriptor` and `KNOWLEDGE_SCHEMA_VERSION` (the
   latter is already exported but called out for parity-tool symmetry).
2. Adds the in-file consistency test.
3. Updates `tools/schema-parity-check/index.mjs` to dispatch between
   descriptor and zod walkers, defaulting to the descriptor walker for
   the knowledge schema.
4. Removes the zod-walker dead path once provenance + stakeholder-doc
   surfaces have descriptors of their own (deferred — those are spec
   121 §8 / spec 122 work, not 125's).

## 5. Acceptance

A-1. `make ci-schema-parity` exits 0 on a clean tree.
A-2. `make ci` exits 0 on a clean tree (assuming all other gates pass).
A-3. Renaming any field in `extractionOutputDescriptor` without a
     matching change in `crates/factory-contracts/src/knowledge.rs`
     causes `make ci-schema-parity` to fail with a clear `<root>.foo`
     diff message.
A-4. Renaming any field in `extractionOutputDescriptor` without a
     matching change in `Validator` is caught by the in-file
     vitest case.
A-5. The `extractionOutput.ts` file remains free of any zod import
     (Encore parser stability invariant).

## 6. Non-Goals

- Rewriting the Rust-side fingerprint emission. The Rust mirror is
  fine; only the TS-side walker is changing.
- Migrating `provenancePolicy.ts` or `stakeholderDocPolicy.ts` to the
  descriptor pattern. Those files do not exist yet; spec 121 §8 / spec
  122 will land them, and the parity tool already supports the
  "reserved" mode for missing TS mirrors.
- Generalising the descriptor into a shared schema-DSL package. The
  descriptor lives next to the validator it describes, by design.

## 7. Open Questions

- Should the descriptor be auto-derived from the `Validator` (e.g. via a
  decorator-driven approach) instead of hand-written? Default: no — the
  hand-written descriptor + in-file test is simpler and avoids any
  framework dependency. Revisit if drift becomes a recurring problem.
- Could the parity tool diff TS interfaces directly via the TypeScript
  compiler API? Plausible, but heavier than the descriptor approach
  and pulls a `typescript` dependency into the build chain.
