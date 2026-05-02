#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/120-factory-extraction-stage/spec.md — FR-003, FR-004
// Spec: specs/121-claim-provenance-enforcement/spec.md — FR-007 (extension)
// Spec: specs/125-schema-parity-walker-rebuild/spec.md — §3.2 (descriptor walker)
//
// Schema parity check.
//
// Compares the structural fingerprint of each stagecraft TS schema (the
// source of truth) with the fingerprint emitted by the matching Rust
// mirror in `crates/factory-contracts/` during `cargo test`. Drift on
// either side fails CI before any runtime divergence can ship.
//
// Walker dispatcher (spec 125). Each TS schema is walked through one of
// two paths:
//
//   - Descriptor walker (`walkDescriptor`, in `./walk-descriptor.mjs`).
//     Consumes a plain-data `SchemaNode` exported next to a hand-rolled
//     validator. This is the canonical path going forward — it is
//     dependency-free and stable under Encore.ts's TS parser, which
//     crashes on `zod/v4/classic/schemas.d.cts` if zod is imported from
//     any file the API tree transitively touches (see the file header on
//     `extractionOutput.ts`).
//
//   - Zod walker (`walkType`, inline below). Walks a `ZodTypeAny` tree by
//     introspecting `_def.type`. Retained for the provenance + stakeholder
//     surfaces: those Rust mirrors exist, but their TS mirrors are
//     reserved by specs 121 §8 / 122 and not yet authored. When those
//     files land, they will land as descriptors and this walker becomes
//     deletable.
//
// The `walk(node)` dispatcher selects the descriptor walker when `node.kind`
// is a string (the `SchemaNode` discriminator) and falls through to the
// zod walker otherwise.
//
// Spec 121/122 reserved-mode behaviour: when a TS mirror file does not
// exist, the parity check records the Rust-side fingerprint and emits an
// informational line — it does NOT fail. Once the TS mirror lands, the
// comparison activates automatically (existence flip).
//
// Run order:
//   1. cargo test --manifest-path crates/factory-contracts/Cargo.toml
//      (writes build/schema-parity/{rust-knowledge,rust-provenance,rust-stakeholder-doc}-schema.json)
//   2. bun run tools/schema-parity-check/index.mjs   (this file — needs
//      a runtime that can import .ts, hence bun)
//
// Exit codes:
//   0  all configured fingerprints match (or recorded for later comparison)
//   1  fingerprints differ — drift detected
//   2  internal error (rust file missing, walk failed, missing exports, etc.)

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { walkDescriptor } from "./walk-descriptor.mjs";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, "..", "..");
const TS_SCHEMA_PATH = path.join(
  REPO_ROOT,
  "platform/services/stagecraft/api/knowledge/extractionOutput.ts",
);
const RUST_FINGERPRINT_PATH = path.join(
  REPO_ROOT,
  "build/schema-parity/rust-knowledge-schema.json",
);
const RUST_MIRROR_PATH = path.join(
  REPO_ROOT,
  "crates/factory-contracts/src/knowledge.rs",
);

const TS_PROVENANCE_PATH = path.join(
  REPO_ROOT,
  "platform/services/stagecraft/api/governance/provenancePolicy.ts",
);
const RUST_PROVENANCE_FINGERPRINT_PATH = path.join(
  REPO_ROOT,
  "build/schema-parity/rust-provenance-schema.json",
);
const RUST_PROVENANCE_MIRROR_PATH = path.join(
  REPO_ROOT,
  "crates/factory-contracts/src/provenance.rs",
);

// Spec 122 — stakeholder-doc grammar.
const TS_STAKEHOLDER_DOC_PATH = path.join(
  REPO_ROOT,
  "platform/services/stagecraft/api/governance/stakeholderDocPolicy.ts",
);
const RUST_STAKEHOLDER_DOC_FINGERPRINT_PATH = path.join(
  REPO_ROOT,
  "build/schema-parity/rust-stakeholder-doc-schema.json",
);
const RUST_STAKEHOLDER_DOC_MIRROR_PATH = path.join(
  REPO_ROOT,
  "crates/factory-contracts/src/stakeholder_docs.rs",
);

function fail(code, message) {
  process.stderr.write(message + "\n");
  process.exit(code);
}

if (!fs.existsSync(RUST_FINGERPRINT_PATH)) {
  fail(
    2,
    `schema-parity-check: rust fingerprint not found at ${path.relative(REPO_ROOT, RUST_FINGERPRINT_PATH)}\n` +
      `  Run: cargo test --manifest-path crates/factory-contracts/Cargo.toml`,
  );
}

if (!fs.existsSync(RUST_PROVENANCE_FINGERPRINT_PATH)) {
  fail(
    2,
    `schema-parity-check: provenance rust fingerprint not found at ${path.relative(REPO_ROOT, RUST_PROVENANCE_FINGERPRINT_PATH)}\n` +
      `  Run: cargo test --manifest-path crates/factory-contracts/Cargo.toml`,
  );
}

if (!fs.existsSync(RUST_STAKEHOLDER_DOC_FINGERPRINT_PATH)) {
  fail(
    2,
    `schema-parity-check: stakeholder-doc rust fingerprint not found at ${path.relative(REPO_ROOT, RUST_STAKEHOLDER_DOC_FINGERPRINT_PATH)}\n` +
      `  Run: cargo test --manifest-path crates/factory-contracts/Cargo.toml`,
  );
}

const stagecraftDir = path.join(REPO_ROOT, "platform/services/stagecraft");
let extractionOutputDescriptor;
let tsSchemaVersion;
try {
  const mod = await import(TS_SCHEMA_PATH);
  extractionOutputDescriptor = mod.extractionOutputDescriptor;
  tsSchemaVersion = mod.KNOWLEDGE_SCHEMA_VERSION;
} catch (e) {
  fail(
    2,
    `schema-parity-check: failed to import ${path.relative(REPO_ROOT, TS_SCHEMA_PATH)}\n  ${e.message}\n` +
      `  Run from a runtime that handles .ts (e.g. \`node --experimental-strip-types\` 22+, or \`bun run\`).\n` +
      `  Or run \`cd ${path.relative(REPO_ROOT, stagecraftDir)} && npm install\` then retry under bun.`,
  );
}

if (
  !extractionOutputDescriptor ||
  typeof extractionOutputDescriptor.kind !== "string" ||
  typeof tsSchemaVersion !== "string"
) {
  fail(
    2,
    `schema-parity-check: ${path.relative(REPO_ROOT, TS_SCHEMA_PATH)} is missing exports\n` +
      `  Required: extractionOutputDescriptor (a SchemaNode with a string \`kind\`), KNOWLEDGE_SCHEMA_VERSION`,
  );
}

// Spec 125 dispatcher. A `SchemaNode` carries a string `kind`; any other
// shape is assumed to be a `ZodTypeAny` and routed to `walkType`. Once the
// provenance + stakeholder-doc TS mirrors land as descriptors, every call
// site here will resolve through `walkDescriptor` and `walkType` can go.
function walk(node) {
  if (node && typeof node.kind === "string") {
    return walkDescriptor(node);
  }
  return walkType(node);
}

// ---------------------------------------------------------------------------
// Zod walker (spec 121 / 122 reserved-mode legacy).
//
// Reads the structural shape directly out of a `ZodTypeAny._def` tree. Used
// by the provenance + stakeholder-doc parity surfaces only — those TS
// mirrors don't exist yet, but when they land per specs 121 §8 / 122, they
// will land as plain-data descriptors (the canonical post-spec-125 shape)
// and this walker becomes deletable. Until then, the dispatcher above
// keeps both paths alive so neither surface regresses.
//
// Do NOT extend this function. New parity surfaces should ship with a
// `SchemaNode` descriptor (see `walk-descriptor.mjs`).
// ---------------------------------------------------------------------------

function unwrap(zod) {
  while (zod?._def?.type === "pipe") zod = zod._def.in;
  return zod;
}

function isOptional(zod) {
  return unwrap(zod)?._def?.type === "optional";
}

function walkType(zod) {
  zod = unwrap(zod);
  if (zod?._def?.type === "optional" || zod?._def?.type === "nullable") {
    return walkType(zod._def.innerType);
  }
  switch (zod?._def?.type) {
    case "string":
      return { kind: "string" };
    case "number": {
      const isInt = (zod._def.checks ?? []).some(
        (c) => c?.format === "safeint" || c?._zod?.def?.format === "safeint",
      );
      return { kind: isInt ? "int" : "number" };
    }
    case "boolean":
      return { kind: "boolean" };
    case "unknown":
    case "any":
      return { kind: "unknown" };
    case "array":
      return { kind: "array", element: walkType(zod._def.element) };
    case "record":
      return {
        kind: "map",
        key: walkType(zod._def.keyType),
        value: walkType(zod._def.valueType),
      };
    case "object": {
      const shape = zod._def.shape;
      const fields = Object.entries(shape)
        .map(([name, t]) => ({
          name,
          required: !isOptional(t),
          type: walkType(t),
        }))
        .sort((a, b) => a.name.localeCompare(b.name));
      return { kind: "object", fields };
    }
    case "tuple": {
      // Zod 4: items live on `_def.items`. Map each positional schema to a
      // type fingerprint preserving order (tuples are positional, NOT
      // alphabetical).
      const items = (zod._def.items ?? []).map((t) => walkType(t));
      return { kind: "tuple", items };
    }
    case "enum": {
      // Zod 4: `_def.entries` is `Record<string, string>` for native string
      // enums and a value list for plain `z.enum([...])`. We sort to keep
      // the fingerprint order-independent.
      const raw = zod._def.entries ?? zod._def.values ?? {};
      const values = Array.isArray(raw) ? [...raw] : Object.values(raw);
      values.sort();
      return { kind: "enum", values };
    }
    case "union":
    case "discriminatedUnion": {
      // The Rust side uses #[serde(tag = "mode")] which is the Zod
      // `discriminatedUnion("mode", [...])` shape. Each option is an
      // object whose discriminator literal is the variant tag.
      const discriminator = zod._def.discriminator;
      const options = zod._def.options ?? [];
      const variants = options
        .map((opt) => {
          const inner = unwrap(opt);
          const shape = inner?._def?.shape ?? {};
          const tagSchema = shape[discriminator];
          const tag =
            tagSchema?._def?.values?.[0] ?? tagSchema?._def?.value ?? null;
          const fields = Object.entries(shape)
            .filter(([name]) => name !== discriminator)
            .map(([name, t]) => ({
              name,
              required: !isOptional(t),
              type: walkType(t),
            }))
            .sort((a, b) => a.name.localeCompare(b.name));
          return { tag, fields };
        })
        .sort((a, b) => String(a.tag).localeCompare(String(b.tag)));
      return { kind: "discriminatedUnion", discriminator, variants };
    }
    default:
      throw new Error(
        `schema-parity-check: unhandled zod type: ${zod?._def?.type}`,
      );
  }
}

let tsFingerprint;
try {
  tsFingerprint = {
    version: tsSchemaVersion,
    root: walk(extractionOutputDescriptor),
  };
} catch (e) {
  fail(2, `schema-parity-check: knowledge descriptor walk failed — ${e.message}`);
}

const rustFingerprint = JSON.parse(
  fs.readFileSync(RUST_FINGERPRINT_PATH, "utf8"),
);

function diff(a, b, pathParts = []) {
  const here = pathParts.join(".") || "<root>";
  if (typeof a !== typeof b) {
    return [`${here}: TS is ${typeof a}, Rust is ${typeof b}`];
  }
  if (a === null || b === null || typeof a !== "object") {
    return a === b ? [] : [`${here}: TS=${JSON.stringify(a)}, Rust=${JSON.stringify(b)}`];
  }
  if (Array.isArray(a) !== Array.isArray(b)) {
    return [`${here}: TS array=${Array.isArray(a)}, Rust array=${Array.isArray(b)}`];
  }
  if (Array.isArray(a)) {
    const issues = [];
    if (a.length !== b.length) {
      issues.push(`${here}: TS has ${a.length} entries, Rust has ${b.length}`);
    }
    const max = Math.max(a.length, b.length);
    for (let i = 0; i < max; i++) {
      const tsEntry = a[i];
      const rustEntry = b[i];
      if (tsEntry === undefined) {
        issues.push(`${here}[${i}]: present in Rust only — ${JSON.stringify(rustEntry)}`);
        continue;
      }
      if (rustEntry === undefined) {
        issues.push(`${here}[${i}]: present in TS only — ${JSON.stringify(tsEntry)}`);
        continue;
      }
      const label = tsEntry?.name ?? rustEntry?.name ?? String(i);
      issues.push(...diff(tsEntry, rustEntry, [...pathParts, label]));
    }
    return issues;
  }
  const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
  const issues = [];
  for (const k of keys) {
    if (!(k in a)) {
      issues.push(`${here}.${k}: present in Rust only — ${JSON.stringify(b[k])}`);
      continue;
    }
    if (!(k in b)) {
      issues.push(`${here}.${k}: present in TS only — ${JSON.stringify(a[k])}`);
      continue;
    }
    issues.push(...diff(a[k], b[k], [...pathParts, k]));
  }
  return issues;
}

const issues = diff(tsFingerprint, rustFingerprint);
if (issues.length === 0) {
  process.stdout.write(
    `schema-parity-check: knowledge OK (version ${tsSchemaVersion})\n` +
      `  ts: ${path.relative(REPO_ROOT, TS_SCHEMA_PATH)}\n` +
      `  rs: ${path.relative(REPO_ROOT, RUST_MIRROR_PATH)}\n`,
  );
} else {
  process.stderr.write(
    `schema-parity-check: DRIFT detected between\n` +
      `  ts: ${path.relative(REPO_ROOT, TS_SCHEMA_PATH)} (version=${tsSchemaVersion})\n` +
      `  rs: ${path.relative(REPO_ROOT, RUST_MIRROR_PATH)} (version=${rustFingerprint.version})\n\n`,
  );
  for (const issue of issues) {
    process.stderr.write(`  - ${issue}\n`);
  }
  process.stderr.write(
    `\nIf the TS schema changed, mirror the change in ${path.relative(REPO_ROOT, RUST_MIRROR_PATH)}.\n` +
      `If the Rust types changed, mirror them in ${path.relative(REPO_ROOT, TS_SCHEMA_PATH)}.\n` +
      `Then bump KNOWLEDGE_SCHEMA_VERSION on both sides if the change is breaking.\n`,
  );
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Provenance schema (spec 121).
//
// The TS mirror at provenancePolicy.ts is reserved by the spec but not yet
// authored. While it is absent, this block records the Rust fingerprint as
// already emitted by `cargo test` and emits an informational message — it
// does NOT fail CI. Once the TS file lands and exports
// `provenanceClaimSchema` + `PROVENANCE_SCHEMA_VERSION`, the comparison
// activates automatically.
// ---------------------------------------------------------------------------

const provenanceRustFingerprint = JSON.parse(
  fs.readFileSync(RUST_PROVENANCE_FINGERPRINT_PATH, "utf8"),
);

let provenanceHandled = false;
if (!fs.existsSync(TS_PROVENANCE_PATH)) {
  process.stdout.write(
    `schema-parity-check: provenance rust fingerprint recorded (version ${provenanceRustFingerprint.version})\n` +
      `  rs: ${path.relative(REPO_ROOT, RUST_PROVENANCE_MIRROR_PATH)}\n` +
      `  ts: ${path.relative(REPO_ROOT, TS_PROVENANCE_PATH)} — not yet authored (spec 121 §8 reserves the path)\n` +
      `  comparison will activate automatically when the TS mirror lands.\n`,
  );
  provenanceHandled = true;
}

if (!provenanceHandled) {
  let provenanceTsSchema;
  let provenanceTsVersion;
  try {
    const mod = await import(TS_PROVENANCE_PATH);
    provenanceTsSchema =
      mod.provenanceClaimSchema ?? mod.provenanceSchema ?? mod.default;
    provenanceTsVersion = mod.PROVENANCE_SCHEMA_VERSION;
  } catch (e) {
    fail(
      2,
      `schema-parity-check: failed to import ${path.relative(REPO_ROOT, TS_PROVENANCE_PATH)}\n  ${e.message}`,
    );
  }

  if (!provenanceTsSchema || typeof provenanceTsVersion !== "string") {
    fail(
      2,
      `schema-parity-check: ${path.relative(REPO_ROOT, TS_PROVENANCE_PATH)} is missing exports\n` +
        `  Required: provenanceClaimSchema (or provenanceSchema), PROVENANCE_SCHEMA_VERSION`,
    );
  }

  let provenanceTsFingerprint;
  try {
    provenanceTsFingerprint = {
      version: provenanceTsVersion,
      claim: walk(provenanceTsSchema),
    };
  } catch (e) {
    fail(2, `schema-parity-check: provenance walk failed — ${e.message}`);
  }

  const provenanceClaimRustFp = {
    version: provenanceRustFingerprint.version,
    claim: provenanceRustFingerprint.claim,
  };
  const provenanceIssues = diff(provenanceTsFingerprint, provenanceClaimRustFp);
  if (provenanceIssues.length === 0) {
    process.stdout.write(
      `schema-parity-check: provenance OK (version ${provenanceTsVersion})\n` +
        `  ts: ${path.relative(REPO_ROOT, TS_PROVENANCE_PATH)}\n` +
        `  rs: ${path.relative(REPO_ROOT, RUST_PROVENANCE_MIRROR_PATH)}\n`,
    );
  } else {
    process.stderr.write(
      `schema-parity-check: provenance DRIFT detected between\n` +
        `  ts: ${path.relative(REPO_ROOT, TS_PROVENANCE_PATH)} (version=${provenanceTsVersion})\n` +
        `  rs: ${path.relative(REPO_ROOT, RUST_PROVENANCE_MIRROR_PATH)} (version=${provenanceRustFingerprint.version})\n\n`,
    );
    for (const issue of provenanceIssues) {
      process.stderr.write(`  - ${issue}\n`);
    }
    process.stderr.write(
      `\nIf the TS schema changed, mirror the change in ${path.relative(REPO_ROOT, RUST_PROVENANCE_MIRROR_PATH)}.\n` +
        `If the Rust types changed, mirror them in ${path.relative(REPO_ROOT, TS_PROVENANCE_PATH)}.\n` +
        `Then bump PROVENANCE_SCHEMA_VERSION on both sides if the change is breaking.\n`,
    );
    process.exit(1);
  }
}

// ---------------------------------------------------------------------------
// Stakeholder-doc schema (spec 122).
//
// The TS mirror at stakeholderDocPolicy.ts is reserved by spec 122 but
// not yet authored. While it is absent, this block records the Rust
// fingerprint as already emitted by `cargo test` and emits an
// informational message — it does NOT fail CI. Once the TS file lands
// and exports `stakeholderDocSchema` + `STAKEHOLDER_DOC_SCHEMA_VERSION`,
// the comparison activates automatically.
// ---------------------------------------------------------------------------

const stakeholderRustFingerprint = JSON.parse(
  fs.readFileSync(RUST_STAKEHOLDER_DOC_FINGERPRINT_PATH, "utf8"),
);

if (!fs.existsSync(TS_STAKEHOLDER_DOC_PATH)) {
  process.stdout.write(
    `schema-parity-check: stakeholder-doc rust fingerprint recorded (version ${stakeholderRustFingerprint.version})\n` +
      `  rs: ${path.relative(REPO_ROOT, RUST_STAKEHOLDER_DOC_MIRROR_PATH)}\n` +
      `  ts: ${path.relative(REPO_ROOT, TS_STAKEHOLDER_DOC_PATH)} — not yet authored (spec 122 §8 reserves the path)\n` +
      `  comparison will activate automatically when the TS mirror lands.\n`,
  );
  process.exit(0);
}

let stakeholderTsSchema;
let stakeholderTsVersion;
try {
  const mod = await import(TS_STAKEHOLDER_DOC_PATH);
  stakeholderTsSchema =
    mod.stakeholderDocSchema ?? mod.stakeholderDoc ?? mod.default;
  stakeholderTsVersion = mod.STAKEHOLDER_DOC_SCHEMA_VERSION;
} catch (e) {
  fail(
    2,
    `schema-parity-check: failed to import ${path.relative(REPO_ROOT, TS_STAKEHOLDER_DOC_PATH)}\n  ${e.message}`,
  );
}

if (!stakeholderTsSchema || typeof stakeholderTsVersion !== "string") {
  fail(
    2,
    `schema-parity-check: ${path.relative(REPO_ROOT, TS_STAKEHOLDER_DOC_PATH)} is missing exports\n` +
      `  Required: stakeholderDocSchema, STAKEHOLDER_DOC_SCHEMA_VERSION`,
  );
}

let stakeholderTsFingerprint;
try {
  stakeholderTsFingerprint = {
    version: stakeholderTsVersion,
    stakeholderDoc: walk(stakeholderTsSchema),
  };
} catch (e) {
  fail(
    2,
    `schema-parity-check: stakeholder-doc walk failed — ${e.message}`,
  );
}

const stakeholderRustFp = {
  version: stakeholderRustFingerprint.version,
  stakeholderDoc: stakeholderRustFingerprint.stakeholderDoc,
};
const stakeholderIssues = diff(stakeholderTsFingerprint, stakeholderRustFp);
if (stakeholderIssues.length === 0) {
  process.stdout.write(
    `schema-parity-check: stakeholder-doc OK (version ${stakeholderTsVersion})\n` +
      `  ts: ${path.relative(REPO_ROOT, TS_STAKEHOLDER_DOC_PATH)}\n` +
      `  rs: ${path.relative(REPO_ROOT, RUST_STAKEHOLDER_DOC_MIRROR_PATH)}\n`,
  );
} else {
  process.stderr.write(
    `schema-parity-check: stakeholder-doc DRIFT detected between\n` +
      `  ts: ${path.relative(REPO_ROOT, TS_STAKEHOLDER_DOC_PATH)} (version=${stakeholderTsVersion})\n` +
      `  rs: ${path.relative(REPO_ROOT, RUST_STAKEHOLDER_DOC_MIRROR_PATH)} (version=${stakeholderRustFingerprint.version})\n\n`,
  );
  for (const issue of stakeholderIssues) {
    process.stderr.write(`  - ${issue}\n`);
  }
  process.stderr.write(
    `\nIf the TS schema changed, mirror the change in ${path.relative(REPO_ROOT, RUST_STAKEHOLDER_DOC_MIRROR_PATH)}.\n` +
      `If the Rust types changed, mirror them in ${path.relative(REPO_ROOT, TS_STAKEHOLDER_DOC_PATH)}.\n` +
      `Then bump STAKEHOLDER_DOC_SCHEMA_VERSION on both sides if the change is breaking.\n`,
  );
  process.exit(1);
}
