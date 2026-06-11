// Spec 198 FR-013(a) — the deterministic, synchronous, fail-closed gate on
// every substrate `user_body` write (PD-6).
//
// Rules are pure functions over the candidate content (+ the row's kind);
// the first violated rule wins and the write is refused with an
// attributable rule id. A model may detect (FR-013 d, spec 200) — only
// these rules may block. No I/O, no model calls, no clock: the same input
// always produces the same verdict.
//
// Wired into: `artifacts.ts::applyOverrideCore`, `conflicts.ts`
// (`edit_and_accept` — the side-door override revision), and the
// user-authored agent writes in `api/agents/catalog.ts` (FR-013 governs
// the write path class, not one endpoint).

import { parse as parseYaml } from "yaml";

// ---------------------------------------------------------------------------
// Verdict shape
// ---------------------------------------------------------------------------

export type OverrideGateVerdict =
  | { ok: true }
  | { ok: false; ruleId: string; detail: string };

export type OverrideGateInput = {
  content: string;
  /** Substrate row kind — drives the kind-stability parse check. */
  kind: string;
  /** Row path — picks JSON vs YAML for structured kinds. */
  path: string;
};

/** Size ceiling for a `user_body` revision (PD-6 default). */
export const OVERRIDE_MAX_BYTES = 256 * 1024;

/** Base64-ish runs longer than this are refused as encoded blobs (ASI01 m6). */
const ENCODED_BLOB_RUN_CHARS = 2048;

// ---------------------------------------------------------------------------
// Rules — evaluated in declaration order; first hit refuses.
// ---------------------------------------------------------------------------

// A high surrogate not followed by a low surrogate, or a low surrogate not
// preceded by a high one — the malformed-UTF-16 shapes that cannot encode
// to UTF-8.
const LONE_SURROGATE =
  /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/;

// U+200B..U+200F zero-width + directional marks, U+202A..U+202E embedding
// overrides, U+2060..U+2064 invisible operators, U+2066..U+2069 isolates,
// U+FEFF BOM/ZWNBSP. The Trojan-Source / hidden-instruction carrier class.
const ZERO_WIDTH_BIDI =
  /[\u200B-\u200F\u202A-\u202E\u2060-\u2064\u2066-\u2069\uFEFF]/;

// data: URIs smuggle payloads past content review in markdown/HTML sinks.
const DATA_URI = /\bdata:[a-z0-9.+-]+\/[a-z0-9.+-]+;base64,/i;

// ANSI escape sequences (terminal injection when bodies are echoed to TTYs).
// eslint-disable-next-line no-control-regex
const ANSI_ESCAPE = /\u001B/;

// CONST-002 secret shapes: PEM blocks, VCS/cloud token prefixes, JWT triplets.
const PEM_BLOCK = /-----BEGIN [A-Z0-9 ]+-----/;
const TOKEN_SHAPES: Array<{ name: string; re: RegExp }> = [
  { name: "github-token", re: /\bgh[pousr]_[A-Za-z0-9]{20,}\b/ },
  { name: "github-app-token", re: /\bghs_[A-Za-z0-9]{20,}\b/ },
  { name: "gitlab-pat", re: /\bglpat-[A-Za-z0-9_-]{20,}\b/ },
  { name: "aws-access-key", re: /\bAKIA[0-9A-Z]{16}\b/ },
  { name: "slack-token", re: /\bxox[baprs]-[A-Za-z0-9-]{10,}\b/ },
  { name: "anthropic-key", re: /\bsk-ant-[A-Za-z0-9_-]{20,}\b/ },
];
const JWT_LIKE =
  /\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\b/;

// Kinds whose content is structured; an override that no longer parses
// would silently change the row's classification downstream (PD-6
// "kind stability"). JSON for .json paths, YAML otherwise.
const STRUCTURED_KINDS = new Set([
  "contract-schema",
  "governance-envelope",
  "adapter-manifest",
  "reference-data",
]);

/**
 * Run the FR-013(a) gate. Deterministic; throws nothing — callers turn a
 * refusal into their surface's attributable error + audit record.
 */
export function runOverrideGate(input: OverrideGateInput): OverrideGateVerdict {
  const { content, kind, path } = input;

  // gate.utf8 — lone surrogates cannot round-trip through storage as UTF-8.
  // (Regex form of String.prototype.isWellFormed; the tsconfig lib target
  // predates es2024.)
  if (LONE_SURROGATE.test(content)) {
    return refuse("gate.utf8", "content contains unpaired surrogates");
  }

  // gate.size-ceiling
  const bytes = Buffer.byteLength(content, "utf8");
  if (bytes > OVERRIDE_MAX_BYTES) {
    return refuse(
      "gate.size-ceiling",
      `content is ${bytes} bytes; the override ceiling is ${OVERRIDE_MAX_BYTES}`,
    );
  }

  // gate.kind-stability — structured kinds must still parse as their shape.
  if (STRUCTURED_KINDS.has(kind)) {
    const parseError = structuredParseError(content, path);
    if (parseError) {
      return refuse(
        "gate.kind-stability",
        `override would break the row's '${kind}' classification: ${parseError}`,
      );
    }
  }

  // Carrier refusals (ASI01 m6).
  if (ZERO_WIDTH_BIDI.test(content)) {
    return refuse(
      "gate.carrier.zero-width-bidi",
      "content contains zero-width or bidirectional control characters",
    );
  }
  if (content.includes("<!--")) {
    return refuse(
      "gate.carrier.html-comment",
      "content contains an HTML comment (hidden-payload carrier)",
    );
  }
  if (DATA_URI.test(content)) {
    return refuse("gate.carrier.data-uri", "content contains a base64 data: URI");
  }
  if (hasEncodedBlobRun(content)) {
    return refuse(
      "gate.carrier.encoded-blob",
      `content contains a base64-like run longer than ${ENCODED_BLOB_RUN_CHARS} characters`,
    );
  }
  if (ANSI_ESCAPE.test(content)) {
    return refuse(
      "gate.carrier.ansi-escape",
      "content contains ANSI escape sequences",
    );
  }

  // Secrets scan (CONST-002 class).
  if (PEM_BLOCK.test(content)) {
    return refuse("gate.secret.pem", "content contains a PEM block");
  }
  for (const t of TOKEN_SHAPES) {
    if (t.re.test(content)) {
      return refuse(
        "gate.secret.token",
        `content matches the ${t.name} credential shape`,
      );
    }
  }
  if (JWT_LIKE.test(content)) {
    return refuse("gate.secret.jwt", "content contains a JWT-shaped string");
  }

  return { ok: true };
}

function refuse(ruleId: string, detail: string): OverrideGateVerdict {
  return { ok: false, ruleId, detail };
}

function structuredParseError(content: string, path: string): string | null {
  if (path.toLowerCase().endsWith(".json")) {
    try {
      JSON.parse(content);
      return null;
    } catch (e) {
      return `not valid JSON (${(e as Error).message})`;
    }
  }
  try {
    parseYaml(content);
    return null;
  } catch (e) {
    return `not valid YAML (${(e as Error).message})`;
  }
}

// Long base64-ish runs. Whitespace breaks a run; `=` only counts at the
// tail, so prose with long unbroken words does not false-positive (the
// alphabet still requires [A-Za-z0-9+/]).
const ENCODED_BLOB_RE = new RegExp(
  `[A-Za-z0-9+/]{${ENCODED_BLOB_RUN_CHARS},}={0,2}`,
);

function hasEncodedBlobRun(content: string): boolean {
  return ENCODED_BLOB_RE.test(content);
}
