#!/usr/bin/env node
// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Bartek Kus
// Spec: specs/120-factory-extraction-stage/spec.md — FR-003, FR-004
//
// Schema parity check.
//
// Compares the structural fingerprint of stagecraft's `extractionOutputSchema`
// (the source of truth) with the fingerprint emitted by
// `crates/factory-contracts/src/knowledge.rs` during `cargo test`.
//
// Run order:
//   1. cargo test --manifest-path crates/factory-contracts/Cargo.toml
//      (writes build/schema-parity/rust-knowledge-schema.json)
//   2. node tools/schema-parity-check/index.mjs   (this file)
//
// Exit codes:
//   0  fingerprints match
//   1  fingerprints differ — drift detected
//   2  internal error (rust file missing, zod walk failed, etc.)

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

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

const stagecraftDir = path.join(REPO_ROOT, "platform/services/stagecraft");
let extractionOutputSchema;
let tsSchemaVersion;
try {
  const mod = await import(TS_SCHEMA_PATH);
  extractionOutputSchema = mod.extractionOutputSchema;
  tsSchemaVersion = mod.KNOWLEDGE_SCHEMA_VERSION;
} catch (e) {
  fail(
    2,
    `schema-parity-check: failed to import ${path.relative(REPO_ROOT, TS_SCHEMA_PATH)}\n  ${e.message}\n` +
      `  Run from a runtime that handles .ts (e.g. \`node --experimental-strip-types\` 22+, or \`bun run\`).\n` +
      `  Or run \`cd ${path.relative(REPO_ROOT, stagecraftDir)} && npm install\` then retry under bun.`,
  );
}

if (!extractionOutputSchema || typeof tsSchemaVersion !== "string") {
  fail(
    2,
    `schema-parity-check: ${path.relative(REPO_ROOT, TS_SCHEMA_PATH)} is missing exports\n` +
      `  Required: extractionOutputSchema, KNOWLEDGE_SCHEMA_VERSION`,
  );
}

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
    root: walkType(extractionOutputSchema),
  };
} catch (e) {
  fail(2, `schema-parity-check: zod walk failed — ${e.message}`);
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
    `schema-parity-check: OK (version ${tsSchemaVersion})\n` +
      `  ts: ${path.relative(REPO_ROOT, TS_SCHEMA_PATH)}\n` +
      `  rs: ${path.relative(REPO_ROOT, RUST_MIRROR_PATH)}\n`,
  );
  process.exit(0);
}

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
