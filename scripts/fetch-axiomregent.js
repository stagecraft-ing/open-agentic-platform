#!/usr/bin/env node
// Fetch the axiomregent sidecar binary from a GitHub Release.
//
// Usage:
//   node scripts/fetch-axiomregent.js              # fetch for current platform
//   node scripts/fetch-axiomregent.js --check      # skip if binary already present
//   node scripts/fetch-axiomregent.js --version=v0.1.1  # pin to a release tag
//
// The script tries `gh` CLI first (handles auth automatically), then falls back
// to the GitHub REST API (requires GITHUB_TOKEN for private repos).
//
// Zero external dependencies — Node.js built-ins only.

const https = require("https");
const fs = require("fs");
const path = require("path");
const { execSync, execFileSync } = require("child_process");
const os = require("os");

const REPO = "stagecraft-ing/open-agentic-platform";
const REPO_ROOT = path.resolve(__dirname, "..");
const BIN_DIR = path.join(
  REPO_ROOT,
  "apps",
  "desktop",
  "src-tauri",
  "binaries"
);

// ---------------------------------------------------------------------------
// Platform detection
// ---------------------------------------------------------------------------

function detectTriple() {
  const platform = process.platform;
  const arch = process.arch;

  const map = {
    "darwin:arm64": "aarch64-apple-darwin",
    "darwin:x64": "x86_64-apple-darwin",
    "linux:x64": "x86_64-unknown-linux-gnu",
    "linux:arm64": "aarch64-unknown-linux-gnu",
    "win32:x64": "x86_64-pc-windows-msvc",
  };

  const key = `${platform}:${arch}`;
  const triple = map[key];
  if (!triple) {
    console.error(
      `Unsupported platform: ${platform}/${arch}. Supported: ${Object.keys(map).join(", ")}`
    );
    process.exit(1);
  }
  return triple;
}

function assetName(triple) {
  const name = `axiomregent-${triple}`;
  return triple.includes("windows") ? `${name}.exe` : name;
}

function destPath(triple) {
  return path.join(BIN_DIR, assetName(triple));
}

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

function parseArgs() {
  const args = process.argv.slice(2);
  let check = false;
  let version = null;

  for (const arg of args) {
    if (arg === "--check") {
      check = true;
    } else if (arg.startsWith("--version=")) {
      version = arg.slice("--version=".length);
    } else if (arg === "--help" || arg === "-h") {
      console.log(
        "Usage: node scripts/fetch-axiomregent.js [--check] [--version=<tag>]"
      );
      process.exit(0);
    } else {
      console.error(`Unknown argument: ${arg}`);
      process.exit(1);
    }
  }
  return { check, version };
}

// ---------------------------------------------------------------------------
// Binary presence check
// ---------------------------------------------------------------------------

function binaryPresent(dest) {
  try {
    fs.accessSync(dest, fs.constants.F_OK);
  } catch {
    return false;
  }
  // On Unix, also verify executable permission
  if (process.platform !== "win32") {
    try {
      fs.accessSync(dest, fs.constants.X_OK);
    } catch {
      return false;
    }
  }
  return true;
}

// ---------------------------------------------------------------------------
// gh CLI helpers
// ---------------------------------------------------------------------------

function ghAvailable() {
  try {
    execFileSync("gh", ["--version"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

function ghAuthStatus() {
  try {
    execFileSync("gh", ["auth", "status"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

/** Use `gh` to find the latest non-draft release that contains our asset. */
function ghResolveRelease(asset) {
  // gh release list doesn't support --json assets, so list tags first...
  const listOut = execFileSync(
    "gh",
    [
      "release",
      "list",
      "--repo",
      REPO,
      "--json",
      "tagName,isDraft",
      "--limit",
      "20",
    ],
    { encoding: "utf-8", stdio: ["ignore", "pipe", "ignore"] }
  );
  const releases = JSON.parse(listOut);

  // ...then check each non-draft release for the asset via gh release view
  for (const rel of releases) {
    if (rel.isDraft) continue;
    try {
      const viewOut = execFileSync(
        "gh",
        [
          "release",
          "view",
          rel.tagName,
          "--repo",
          REPO,
          "--json",
          "assets",
        ],
        { encoding: "utf-8", stdio: ["ignore", "pipe", "ignore"] }
      );
      const data = JSON.parse(viewOut);
      const hasAsset = data.assets.some((a) => a.name === asset);
      if (hasAsset) return rel.tagName;
    } catch {
      continue;
    }
  }
  return null;
}

function ghDownload(tag, asset) {
  fs.mkdirSync(BIN_DIR, { recursive: true });
  execFileSync(
    "gh",
    [
      "release",
      "download",
      tag,
      "--repo",
      REPO,
      "--pattern",
      asset,
      "--dir",
      BIN_DIR,
      "--clobber",
    ],
    { stdio: "inherit" }
  );
}

// ---------------------------------------------------------------------------
// GitHub REST API fallback (for environments without gh)
// ---------------------------------------------------------------------------

function githubToken() {
  return process.env.GITHUB_TOKEN || process.env.GH_TOKEN || null;
}

function httpsGet(url, headers = {}) {
  return new Promise((resolve, reject) => {
    const parsed = new URL(url);
    const opts = {
      hostname: parsed.hostname,
      path: parsed.pathname + parsed.search,
      headers: {
        "User-Agent": "fetch-axiomregent/1.0",
        Accept: "application/vnd.github+json",
        ...headers,
      },
    };
    https
      .get(opts, (res) => {
        // Follow redirects (GitHub sends 302 to S3 for asset downloads)
        if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
          resolve(httpsGet(res.headers.location, {}));
          return;
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () =>
          resolve({ status: res.statusCode, body: Buffer.concat(chunks) })
        );
      })
      .on("error", reject);
  });
}

async function apiResolveRelease(asset) {
  const token = githubToken();
  const headers = token ? { Authorization: `token ${token}` } : {};

  const resp = await httpsGet(
    `https://api.github.com/repos/${REPO}/releases?per_page=20`,
    headers
  );

  if (resp.status === 401 || resp.status === 403) {
    return { error: "auth" };
  }
  if (resp.status !== 200) {
    return {
      error: `GitHub API returned ${resp.status}: ${resp.body.toString().slice(0, 200)}`,
    };
  }

  const releases = JSON.parse(resp.body.toString());
  for (const rel of releases) {
    if (rel.draft) continue;
    const found = rel.assets.find((a) => a.name === asset);
    if (found) {
      return { tag: rel.tag_name, downloadUrl: found.url };
    }
  }
  return { error: "not_found" };
}

async function apiResolveTag(tag, asset) {
  const token = githubToken();
  const headers = token ? { Authorization: `token ${token}` } : {};

  const resp = await httpsGet(
    `https://api.github.com/repos/${REPO}/releases/tags/${tag}`,
    headers
  );

  if (resp.status === 401 || resp.status === 403) {
    return { error: "auth" };
  }
  if (resp.status === 404) {
    return { error: `Release '${tag}' not found.` };
  }
  if (resp.status !== 200) {
    return {
      error: `GitHub API returned ${resp.status}: ${resp.body.toString().slice(0, 200)}`,
    };
  }

  const rel = JSON.parse(resp.body.toString());
  const found = rel.assets.find((a) => a.name === asset);
  if (!found) {
    const available = rel.assets.map((a) => a.name).join(", ");
    return {
      error: `Release '${tag}' has no asset '${asset}'. Available: ${available}`,
    };
  }
  return { tag: rel.tag_name, downloadUrl: found.url };
}

async function apiDownload(downloadUrl, dest) {
  const token = githubToken();
  const headers = {
    Accept: "application/octet-stream",
  };
  if (token) headers.Authorization = `token ${token}`;

  const resp = await httpsGet(downloadUrl, headers);
  if (resp.status !== 200) {
    throw new Error(`Download failed with status ${resp.status}`);
  }

  fs.mkdirSync(path.dirname(dest), { recursive: true });
  fs.writeFileSync(dest, resp.body);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const { check, version } = parseArgs();
  const triple = detectTriple();
  const asset = assetName(triple);
  const dest = destPath(triple);

  console.log(`[fetch-axiomregent] platform: ${triple}, asset: ${asset}`);

  // --check: skip if binary already present and executable
  if (check && binaryPresent(dest)) {
    console.log(`[fetch-axiomregent] binary already present at ${dest}`);
    process.exit(0);
  }

  // Strategy 1: use gh CLI if available and authenticated
  const useGh = ghAvailable() && ghAuthStatus();

  if (useGh) {
    let tag = version;
    if (!tag) {
      console.log("[fetch-axiomregent] resolving latest release via gh CLI...");
      tag = ghResolveRelease(asset);
      if (!tag) {
        console.error(
          `[fetch-axiomregent] No release found containing '${asset}'.`
        );
        console.error(
          "  Build locally instead: make axiomregent  (or ./scripts/build-axiomregent.sh)"
        );
        process.exit(1);
      }
    }

    console.log(`[fetch-axiomregent] downloading ${asset} from release ${tag}...`);
    ghDownload(tag, asset);
    if (process.platform !== "win32") {
      fs.chmodSync(dest, 0o755);
    }
    console.log(`[fetch-axiomregent] saved to ${dest}`);
    process.exit(0);
  }

  // Strategy 2: GitHub REST API fallback
  console.log("[fetch-axiomregent] gh CLI not available, using GitHub API...");

  let result;
  if (version) {
    result = await apiResolveTag(version, asset);
  } else {
    result = await apiResolveRelease(asset);
  }

  if (result.error === "auth") {
    console.error(
      "[fetch-axiomregent] Authentication required for private repo."
    );
    console.error(
      "  Option 1: Install and authenticate gh CLI — gh auth login"
    );
    console.error(
      "  Option 2: Set GITHUB_TOKEN environment variable"
    );
    console.error(
      "  Option 3: Build locally — make axiomregent"
    );
    process.exit(1);
  }
  if (result.error === "not_found") {
    console.error(
      `[fetch-axiomregent] No release found containing '${asset}'.`
    );
    console.error(
      "  Build locally instead: make axiomregent  (or ./scripts/build-axiomregent.sh)"
    );
    process.exit(1);
  }
  if (result.error) {
    console.error(`[fetch-axiomregent] ${result.error}`);
    process.exit(1);
  }

  console.log(
    `[fetch-axiomregent] downloading ${asset} from release ${result.tag}...`
  );
  await apiDownload(result.downloadUrl, dest);

  if (process.platform !== "win32") {
    fs.chmodSync(dest, 0o755);
  }

  const size = fs.statSync(dest).size;
  if (size === 0) {
    fs.unlinkSync(dest);
    console.error("[fetch-axiomregent] Downloaded file is empty — aborting.");
    process.exit(1);
  }

  console.log(
    `[fetch-axiomregent] saved to ${dest} (${(size / 1048576).toFixed(1)} MB)`
  );
}

main().catch((err) => {
  console.error(`[fetch-axiomregent] ${err.message}`);
  process.exit(1);
});
