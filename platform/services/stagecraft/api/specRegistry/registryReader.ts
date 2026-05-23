// Spec 163 / spec 103 — registry-consumer subprocess wrapper.
//
// Spec 103 mandates that .derived/spec-registry/registry.json be read
// only through the registry-consumer binary by orchestrated workflows.
// This module is the stagecraft-side enforcement of that discipline:
// every call into the spec-spine on behalf of the Requirements view
// (FR-001..FR-006) routes through `spawnRegistryConsumer` below; no
// other module in `api/specRegistry/` parses the registry JSON file.
//
// The binary path is resolved from REGISTRY_CONSUMER_BIN (env). Tests
// pass `binaryPath` directly so they do not depend on the developer's
// ambient environment.

import { spawn } from "node:child_process";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import type {
  SpecDetail,
  SpecEdge,
  SpecListRow,
  SpecReference,
  SpecRelationships,
} from "./types";

export interface ReaderOptions {
  /** Override for REGISTRY_CONSUMER_BIN; tests set this. */
  binaryPath?: string;
  /** Subprocess timeout. Default 30s — registry-consumer is fast. */
  timeoutMs?: number;
}

const DEFAULT_TIMEOUT_MS = 30_000;

function resolveBinary(opts: ReaderOptions): string {
  const bin = opts.binaryPath ?? process.env.REGISTRY_CONSUMER_BIN;
  if (!bin) {
    throw new Error(
      "registry-consumer binary path is not configured. Set REGISTRY_CONSUMER_BIN or build the binary: " +
        "`cargo build --release --manifest-path tools/spec-spine/registry-consumer/Cargo.toml`"
    );
  }
  return bin;
}

interface SpawnResult {
  stdout: string;
}

function spawnRegistryConsumer(
  bin: string,
  args: string[],
  timeoutMs: number
): Promise<SpawnResult> {
  return new Promise((resolveP, rejectP) => {
    const proc = spawn(bin, args, { stdio: ["ignore", "pipe", "pipe"] });
    const out: Buffer[] = [];
    const err: Buffer[] = [];
    proc.stdout.on("data", (d: Buffer) => out.push(d));
    proc.stderr.on("data", (d: Buffer) => err.push(d));
    const timer = setTimeout(() => proc.kill("SIGKILL"), timeoutMs).unref();
    proc.on("close", (code) => {
      clearTimeout(timer);
      if (code !== 0) {
        rejectP(
          new Error(
            `registry-consumer ${args.join(" ")} exited ${code}: ${Buffer.concat(err)
              .toString("utf8")
              .slice(0, 2000)}`
          )
        );
        return;
      }
      resolveP({ stdout: Buffer.concat(out).toString("utf8") });
    });
    proc.on("error", rejectP);
  });
}

/** Shape we accept from `registry-consumer list --json`. */
interface RawListRow {
  id: string;
  title: string;
  status: string;
  implementation: string;
  kind?: string | null;
  summary?: string | null;
  specPath: string;
  extraFrontmatter?: Record<string, unknown>;
  references?: Array<{ role?: string; unit?: unknown }>;
  category?: string[] | string | null;
  // Spec 130 relationship-graph fields — opaque arrays passed through
  // for the grouping projections. Indexed access in
  // `relationshipFieldsFor` below.
  [k: string]: unknown;
}

const RELATIONSHIP_FIELD_NAMES = [
  "establishes",
  "extends",
  "refines",
  "supersedes",
  "amends",
  "coAuthority",
  "constrains",
] as const;

function relationshipFieldsFor(raw: RawListRow): Record<string, unknown[]> {
  const out: Record<string, unknown[]> = {};
  for (const name of RELATIONSHIP_FIELD_NAMES) {
    const v = (raw as Record<string, unknown>)[name];
    if (Array.isArray(v) && v.length > 0) {
      out[name] = v;
    }
  }
  return out;
}

function projectListRow(raw: RawListRow): SpecListRow {
  const extra = raw.extraFrontmatter ?? {};
  const categories = Array.isArray(raw.category)
    ? raw.category.filter((s): s is string => typeof s === "string")
    : typeof raw.category === "string"
      ? [raw.category]
      : [];
  const hasDecompositionOrigin = Array.isArray(raw.references)
    ? raw.references.some((r) => r?.role === "decomposition-origin")
    : false;
  return {
    id: raw.id,
    title: raw.title,
    status: raw.status,
    implementation: raw.implementation,
    kind: raw.kind ?? null,
    categories,
    summary: raw.summary ?? null,
    specPath: raw.specPath,
    extraFrontmatter: extra,
    relationshipFields: relationshipFieldsFor(raw),
    hasDecompositionOrigin,
  };
}

/** List specs from the project's registry.json (FR-001). */
export async function listSpecs(
  registryPath: string,
  opts: ReaderOptions = {}
): Promise<SpecListRow[]> {
  const bin = resolveBinary(opts);
  const timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const { stdout } = await spawnRegistryConsumer(
    bin,
    ["--registry-path", registryPath, "list", "--json"],
    timeoutMs
  );
  const arr = JSON.parse(stdout) as RawListRow[];
  if (!Array.isArray(arr)) {
    throw new Error("registry-consumer list --json did not return an array");
  }
  return arr.map(projectListRow).sort((a, b) => a.id.localeCompare(b.id));
}

/** Shape we accept from `registry-consumer show <id> --json`. */
interface RawShowRecord extends RawListRow {
  references?: Array<{ role: string; unit: SpecReference["unit"] }>;
}

function stripFrontmatter(md: string): string {
  // Spec 000 grammar: frontmatter is a single leading `---` block.
  // We strip it for the rendered body so react-markdown does not
  // render YAML as a literal code block.
  if (!md.startsWith("---")) return md;
  const closing = md.indexOf("\n---", 3);
  if (closing < 0) return md;
  const after = md.indexOf("\n", closing + 4);
  return after < 0 ? "" : md.slice(after + 1);
}

/** Get one spec record + body (FR-006). */
export async function getSpecDetail(
  specId: string,
  registryPath: string,
  projectRoot: string,
  opts: ReaderOptions = {}
): Promise<SpecDetail> {
  const bin = resolveBinary(opts);
  const timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const { stdout } = await spawnRegistryConsumer(
    bin,
    ["--registry-path", registryPath, "show", specId, "--json"],
    timeoutMs
  );
  const raw = JSON.parse(stdout) as RawShowRecord;
  const list = projectListRow(raw);
  const references: SpecReference[] = Array.isArray(raw.references)
    ? raw.references
        .filter((r) => r && typeof r.role === "string" && r.unit)
        .map((r) => ({ role: r.role, unit: r.unit }))
    : [];

  let body = "";
  try {
    const raw = await readFile(join(projectRoot, list.specPath), "utf8");
    body = stripFrontmatter(raw);
  } catch {
    // A missing or unreadable spec.md is not fatal — the registry row
    // is still useful. The detail view renders an empty body banner.
    body = "";
  }

  return { ...list, body, references };
}

/**
 * Outgoing + incoming relationship edges for a spec (FR-006, spec 130).
 * Parsed from registry-consumer's `show-relationships` human-readable
 * output — the typed JSON form is not yet exposed by the binary, but
 * the human form is stable enough to scrape, line-shape per
 * tools/spec-spine/registry-consumer/src/main.rs `print_relationships_human`:
 *
 *   Relationships for: <id>
 *
 *   Outgoing (N):
 *     <kind>                              # path-only edges (e.g. constrains)
 *     <kind> → <other-spec>
 *     <kind> → <other-spec> [<paths>]
 *
 *   Incoming (M):
 *     <kind> ← <other-spec>
 */
export async function getSpecRelationships(
  specId: string,
  registryPath: string,
  opts: ReaderOptions = {}
): Promise<SpecRelationships> {
  const bin = resolveBinary(opts);
  const timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const { stdout } = await spawnRegistryConsumer(
    bin,
    ["--registry-path", registryPath, "show-relationships", specId],
    timeoutMs
  );
  return parseRelationshipsText(specId, stdout);
}

// Outgoing edge with named spec: `<kind> → <other> [<paths>]?`. The
// trailing `[paths]` is captured but not surfaced today — the
// Requirements view shows the relationship graph (other specs), not
// per-path detail.
const OUTGOING_LINE = /^\s*([a-z_]+)\s*→\s*([^\s\[]+)(?:\s*\[(.+)\])?\s*$/;
// Incoming edges always carry an other-spec.
const INCOMING_LINE = /^\s*([a-z_]+)\s*←\s*(\S+)\s*$/;

export function parseRelationshipsText(specId: string, text: string): SpecRelationships {
  const outgoing: SpecEdge[] = [];
  const incoming: SpecEdge[] = [];
  let section: "outgoing" | "incoming" | null = null;
  for (const lineRaw of text.split(/\r?\n/)) {
    const line = lineRaw.trimEnd();
    if (/^Outgoing\b/i.test(line)) {
      section = "outgoing";
      continue;
    }
    if (/^Incoming\b/i.test(line)) {
      section = "incoming";
      continue;
    }
    if (!section || !line.trim()) continue;
    if (section === "outgoing") {
      const m = OUTGOING_LINE.exec(line);
      if (!m) continue;
      const [, kind, otherSpec] = m;
      outgoing.push({ kind, otherSpec });
    } else {
      const m = INCOMING_LINE.exec(line);
      if (!m) continue;
      const [, kind, otherSpec] = m;
      incoming.push({ kind, otherSpec });
    }
  }
  return { id: specId, outgoing, incoming };
}
