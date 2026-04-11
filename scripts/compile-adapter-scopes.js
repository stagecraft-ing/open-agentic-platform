#!/usr/bin/env node
// compile-adapter-scopes.js
// Reads factory/adapters/*/manifest.yaml and compiles adapter scopes to
// build/adapter-scopes.json and platform/services/stagecraft/api/factory/adapter-scopes.json
//
// Zero external dependencies — uses only Node.js built-ins.

"use strict";

const fs = require("fs");
const path = require("path");

// ---------------------------------------------------------------------------
// Minimal YAML extractor
// Supports only the simple scalar and list patterns present in manifest.yaml.
// All manifests use 2-space indentation and UTF-8 encoding.
// ---------------------------------------------------------------------------

// Command keys whose values represent executable command strings.
// Other keys under `commands:` (timeout_ms, working_dir, etc.) are skipped.
const COMMAND_KEYS = new Set([
  "install", "compile", "test", "lint", "dev", "format_check",
  "format", "type_check", "seed", "migrate", "gen_client",
]);

/**
 * Collect all executable command strings from the `commands:` section.
 *
 * Handles:
 *   install: "npm install"         <- top-level scalar (COMMAND_KEYS only)
 *   feature_verify:                <- simple list
 *     - "npm run build"
 *   pre_verify:                    <- list of mappings
 *     - command: "npx tsc --noEmit"
 *       working_dir: "."           <- skip
 *       timeout_ms: 30000          <- skip
 *
 * @param {string[]} lines - All lines of the YAML file
 * @returns {string[]} All command strings found in the commands section
 */
function extractCommandStrings(lines) {
  const results = [];

  // Find the line index of the section header
  let startIdx = -1;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].trim() === "commands:") {
      startIdx = i;
      break;
    }
  }
  if (startIdx === -1) return results;

  for (let i = startIdx + 1; i < lines.length; i++) {
    const raw = lines[i];
    const trimmed = raw.trimEnd();

    // Skip blank lines and comment-only lines
    if (!trimmed || trimmed.trim().startsWith("#")) continue;

    const indent = raw.search(/\S/);

    // Any top-level key signals the end of this section
    if (indent === 0) break;

    const t = trimmed.trim();

    // List item under a sub-key (feature_verify, pre_verify, post_verify):
    //   - "npm run build"  or  - command: "..."
    if (t.startsWith("- ")) {
      const rest = t.slice(2).trim();
      // Skip mapping list items (e.g. "- command: ..." or "- key: value")
      if (/^[\w_-]+:/.test(rest)) continue;
      // Plain string list item: - "value" or - value
      const val = rest.replace(/^["'](.*?)["']$/, "$1").trim();
      if (val) results.push(val);
      continue;
    }

    // Sub-mapping command field: command: "value"
    const cmdField = t.match(/^command:\s+["']?([^"'#\n]+?)["']?\s*(?:#.*)?$/);
    if (cmdField) {
      results.push(cmdField[1].trim());
      continue;
    }

    // Top-level command scalar — only for known command keys
    const keyMatch = t.match(/^([\w_-]+):\s+["']?([^"'#\n]+?)["']?\s*(?:#.*)?$/);
    if (keyMatch && COMMAND_KEYS.has(keyMatch[1])) {
      results.push(keyMatch[2].trim());
      continue;
    }
    // All other lines (sub-section headers, timeout_ms, working_dir, etc.) — skip
  }

  return results;
}

/**
 * Extract the adapter name from manifest.yaml.
 * Looks for: name: "aim-vue-node"  (under the adapter: section)
 */
function extractAdapterName(lines) {
  let inAdapter = false;
  for (const raw of lines) {
    const trimmed = raw.trim();
    if (trimmed === "adapter:") {
      inAdapter = true;
      continue;
    }
    if (inAdapter) {
      if (raw.search(/\S/) === 0 && trimmed !== "adapter:") break; // left section
      const m = trimmed.match(/^name:\s+["']?([^"'\s#]+)["']?\s*(?:#.*)?$/);
      if (m) return m[1];
    }
  }
  return null;
}

/**
 * Extract all directory_conventions values and derive unique top-level
 * directories (as "dir/" strings).
 *
 * Values look like:
 *   "apps/{stack}/src/services/{resource}.service.ts"  -> "apps/"
 *   "src/app/api/{resource}/route.ts"                  -> "src/"
 *   "api/{service}/{resource}.ts"                      -> "api/"
 *   ".env"                                             -> (skip — no subdirectory)
 *   null                                               -> (skip)
 *
 * Returns a sorted, deduplicated array of top-level path components ending "/".
 */
function extractDirectoryConventionDirs(lines) {
  const dirs = new Set();

  // Find section header
  let startIdx = -1;
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].trim() === "directory_conventions:") {
      startIdx = i;
      break;
    }
  }
  if (startIdx === -1) return [];

  for (let i = startIdx + 1; i < lines.length; i++) {
    const raw = lines[i];
    const trimmed = raw.trimEnd();
    if (!trimmed || trimmed.trim().startsWith("#")) continue;

    const indent = raw.search(/\S/);
    if (indent === 0) break; // back to top-level

    const t = trimmed.trim();

    // Match scalar: key: "value" or key: value
    const m = t.match(/^[\w_-]+:\s+(.+)$/);
    if (!m) continue;

    let val = m[1].trim();
    // Strip inline comments
    val = val.replace(/\s+#.*$/, "").trim();
    // Strip quotes
    val = val.replace(/^["'](.*?)["']$/, "$1").trim();

    // Skip null / empty
    if (!val || val === "null" || val === "~") continue;

    // Take the first path component
    const parts = val.split("/");
    const top = parts[0];

    // Skip if top component is a dotfile (e.g., ".env") or no subdirectory
    if (top.startsWith(".") || parts.length < 2) continue;

    dirs.add(top + "/");
  }

  return Array.from(dirs).sort();
}

/**
 * Extract the first word (binary name) from a command string.
 * "npm install"       -> "npm"
 * "npx tsc --noEmit"  -> "npx"
 * "cargo build"       -> "cargo"
 */
function commandBinary(cmd) {
  return cmd.trim().split(/\s+/)[0];
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const repoRoot = path.resolve(__dirname, "..");
const adaptersDir = path.join(repoRoot, "factory", "adapters");
const buildDir = path.join(repoRoot, "build");
const stagecraftDestDir = path.join(
  repoRoot,
  "platform",
  "services",
  "stagecraft",
  "api",
  "factory"
);

// Ensure build/ directory exists
if (!fs.existsSync(buildDir)) {
  fs.mkdirSync(buildDir, { recursive: true });
}

// Discover adapter directories
const adapterDirs = fs
  .readdirSync(adaptersDir)
  .filter((name) => {
    const full = path.join(adaptersDir, name);
    return (
      fs.statSync(full).isDirectory() &&
      fs.existsSync(path.join(full, "manifest.yaml"))
    );
  })
  .sort();

if (adapterDirs.length === 0) {
  console.error("No adapter manifests found under", adaptersDir);
  process.exit(1);
}

const compiled = {};

for (const adapterDir of adapterDirs) {
  const manifestPath = path.join(adaptersDir, adapterDir, "manifest.yaml");
  const content = fs.readFileSync(manifestPath, "utf8");
  const lines = content.split("\n");

  const adapterName = extractAdapterName(lines);
  if (!adapterName) {
    console.warn(`Warning: could not extract adapter name from ${manifestPath}, skipping.`);
    continue;
  }

  // Extract file_write_scope from directory_conventions
  const file_write_scope = extractDirectoryConventionDirs(lines);

  // Extract allowed_commands from commands section
  const commandValues = extractCommandStrings(lines);
  const binaries = new Set(commandValues.map(commandBinary).filter(Boolean));
  const allowed_commands = Array.from(binaries).sort();

  compiled[adapterName] = {
    file_write_scope,
    allowed_commands,
  };

  console.log(`  ${adapterName}:`);
  console.log(`    file_write_scope: [${file_write_scope.join(", ")}]`);
  console.log(`    allowed_commands: [${allowed_commands.join(", ")}]`);
}

const output = {
  compiled_at: new Date().toISOString(),
  adapters: compiled,
};

// Write to build/adapter-scopes.json
const buildOutputPath = path.join(buildDir, "adapter-scopes.json");
fs.writeFileSync(buildOutputPath, JSON.stringify(output, null, 2) + "\n");
console.log(`\nWrote ${buildOutputPath}`);

// Copy to stagecraft service directory
const stagecraftOutputPath = path.join(stagecraftDestDir, "adapter-scopes.json");
fs.writeFileSync(stagecraftOutputPath, JSON.stringify(output, null, 2) + "\n");
console.log(`Wrote ${stagecraftOutputPath}`);
