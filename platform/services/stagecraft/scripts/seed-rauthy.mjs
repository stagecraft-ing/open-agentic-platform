#!/usr/bin/env node
// Rauthy seeder (spec 106 FR-002, extended by spec 107).
//
// Idempotently ensures the GitHub upstream auth provider, OAP custom user
// attributes, the `oap` custom scope, scope grants on stagecraft/SPA/OPC
// OIDC clients, AND (spec 107) that each client's `redirect_uris` allow-list
// contains the URIs derived from APP_BASE_URL plus the OPC deep-link scheme.
// Intended to run as a Helm pre-install/pre-upgrade hook BEFORE the
// stagecraft-api Deployment rolls, so a failure fails the Helm release
// and keeps the old pod serving traffic.
//
// Required env vars:
//   RAUTHY_URL                       e.g. https://auth.example.com
//   RAUTHY_ADMIN_TOKEN               admin API-Key secret in `name$secret` form
//   RAUTHY_ADMIN_TOKEN_NAME          (optional) admin API-Key name; defaults
//                                    to the prefix of RAUTHY_ADMIN_TOKEN
//                                    before the `$`
//   GITHUB_UPSTREAM_CLIENT_ID        GitHub OAuth App client_id for Rauthy
//   GITHUB_UPSTREAM_CLIENT_SECRET    GitHub OAuth App client_secret
//   RAUTHY_CLIENT_ID                 stagecraft OIDC client id (to allow-list scope)
//   APP_BASE_URL                     canonical stagecraft origin (spec 107)
//                                    e.g. https://stagecraft.example.com
//
// Optional:
//   OIDC_SPA_CLIENT_ID               SPA client id (web frontend)
//   OPC_CLIENT_ID                    OPC desktop client id
//   OPC_REDIRECT_URI                 override for OPC deep-link
//                                    (default: opc://auth/callback, spec 080 FR-006)

const RAUTHY_URL = must("RAUTHY_URL");
const RAUTHY_ADMIN_TOKEN = must("RAUTHY_ADMIN_TOKEN");
const GITHUB_UPSTREAM_CLIENT_ID = must("GITHUB_UPSTREAM_CLIENT_ID");
const GITHUB_UPSTREAM_CLIENT_SECRET = must("GITHUB_UPSTREAM_CLIENT_SECRET");
const RAUTHY_CLIENT_ID = must("RAUTHY_CLIENT_ID");
const APP_BASE_URL = mustAbsoluteUrl("APP_BASE_URL");
const OIDC_SPA_CLIENT_ID = process.env.OIDC_SPA_CLIENT_ID || "";
const OPC_CLIENT_ID = process.env.OPC_CLIENT_ID || "";
const OPC_REDIRECT_URI = process.env.OPC_REDIRECT_URI || "opc://auth/callback";

// Rauthy admin API takes an API-Key header of the form `API-Key <name>$<secret>`.
// If the admin-token secret already contains a `$`, treat it as the full token.
// Otherwise accept a separate RAUTHY_ADMIN_TOKEN_NAME env var.
function buildAdminAuthHeader() {
  if (RAUTHY_ADMIN_TOKEN.includes("$")) {
    return `API-Key ${RAUTHY_ADMIN_TOKEN}`;
  }
  const name = process.env.RAUTHY_ADMIN_TOKEN_NAME;
  if (!name) {
    die(
      "RAUTHY_ADMIN_TOKEN does not look like `name$secret`. Set " +
        "RAUTHY_ADMIN_TOKEN_NAME, or include the name prefix in " +
        "RAUTHY_ADMIN_TOKEN.",
    );
  }
  return `API-Key ${name}$${RAUTHY_ADMIN_TOKEN}`;
}

const ADMIN_AUTH = buildAdminAuthHeader();

// ---------------------------------------------------------------------------
// OAP custom user attributes (spec 106 FR-002 step 2).
// Attribute names are stable identifiers consumed by `validateJwt` and the
// membership resolver. Descriptions are for the Rauthy admin UI only.
// ---------------------------------------------------------------------------
// Rauthy 0.35 validates `desc` against `[a-zA-Z0-9À-ÿ-\s]{2,128}`. Keep these
// strings free of punctuation beyond hyphens and whitespace — no parens,
// pipes, slashes, dots, or colons — or the POST will 400 on every seed.
const OAP_ATTRS = [
  { name: "oap_user_id", desc: "OAP internal user id uuid" },
  { name: "oap_org_id", desc: "Selected OAP organisation id" },
  { name: "oap_org_slug", desc: "Selected OAP organisation slug" },
  { name: "oap_workspace_id", desc: "Active OAP workspace id" },
  { name: "github_login", desc: "GitHub handle from upstream IDP" },
  { name: "idp_provider", desc: "Upstream IDP type github or oidc" },
  { name: "idp_login", desc: "Upstream IDP login or display name" },
  { name: "avatar_url", desc: "User avatar URL" },
  { name: "platform_role", desc: "OAP platform role owner admin or member" },
];

const OAP_SCOPE = "oap";
const OAP_SCOPE_ATTRS = OAP_ATTRS.map((a) => a.name);

// ---------------------------------------------------------------------------
// Upstream GitHub provider (spec 106 FR-002 step 1).
// ---------------------------------------------------------------------------
const GITHUB_PROVIDER = {
  name: "github",
  typ: "github",
  enabled: true,
  issuer: "https://github.com",
  authorization_endpoint: "https://github.com/login/oauth/authorize",
  token_endpoint: "https://github.com/login/oauth/access_token",
  userinfo_endpoint: "https://api.github.com/user",
  use_pkce: false,
  client_secret_basic: false,
  client_secret_post: true,
  auto_onboarding: true,
  auto_link: true,
  client_id: GITHUB_UPSTREAM_CLIENT_ID,
  client_secret: GITHUB_UPSTREAM_CLIENT_SECRET,
  scope: "read:user user:email",
};

// ---------------------------------------------------------------------------
// Request helper
// ---------------------------------------------------------------------------
async function rauthy(method, path, body) {
  const url = `${RAUTHY_URL.replace(/\/$/, "")}${path}`;
  const init = {
    method,
    headers: {
      Authorization: ADMIN_AUTH,
      Accept: "application/json",
    },
  };
  if (body !== undefined) {
    init.headers["Content-Type"] = "application/json";
    init.body = JSON.stringify(body);
  }

  const resp = await fetch(url, init);
  const text = await resp.text();
  let json;
  if (text) {
    try {
      json = JSON.parse(text);
    } catch {
      // leave json undefined; caller handles non-JSON
    }
  }
  return { status: resp.status, ok: resp.ok, text, json };
}

// ---------------------------------------------------------------------------
// Step 1 — upstream GitHub provider
// ---------------------------------------------------------------------------
async function ensureGithubProvider() {
  log("[1/6] Ensuring GitHub upstream auth provider");

  // Rauthy 0.35: POST /auth/v1/providers returns the list (not GET).
  const list = await rauthy("POST", "/auth/v1/providers", {});
  if (!list.ok) {
    die(
      `Failed to list upstream providers: ${list.status} ${list.text.slice(0, 200)}`,
    );
  }

  const existing = Array.isArray(list.json)
    ? list.json.find((p) => p.name === GITHUB_PROVIDER.name)
    : null;

  if (!existing) {
    const create = await rauthy(
      "POST",
      "/auth/v1/providers/create",
      GITHUB_PROVIDER,
    );
    if (!create.ok) {
      die(
        `Failed to create GitHub provider: ${create.status} ${create.text.slice(0, 400)}`,
      );
    }
    log(`   created provider ${GITHUB_PROVIDER.name} (id=${create.json?.id})`);
    return;
  }

  // Converge drift: re-PUT whenever client_id or scope differs. Don't
  // re-PUT unconditionally to avoid gratuitous writes.
  const needsUpdate =
    existing.client_id !== GITHUB_PROVIDER.client_id ||
    existing.scope !== GITHUB_PROVIDER.scope ||
    existing.typ !== GITHUB_PROVIDER.typ;

  if (needsUpdate) {
    const update = await rauthy(
      "PUT",
      `/auth/v1/providers/${existing.id}`,
      GITHUB_PROVIDER,
    );
    if (!update.ok) {
      die(
        `Failed to update GitHub provider ${existing.id}: ${update.status} ${update.text.slice(0, 400)}`,
      );
    }
    log(`   updated provider ${existing.name} (id=${existing.id})`);
  } else {
    log(`   provider ${existing.name} already current (id=${existing.id})`);
  }
}

// ---------------------------------------------------------------------------
// Step 2 — custom user attributes
// ---------------------------------------------------------------------------
async function ensureUserAttributes() {
  log("[2/6] Ensuring OAP custom user attributes");

  for (const attr of OAP_ATTRS) {
    const resp = await rauthy("POST", "/auth/v1/users/attr", attr);
    if (resp.ok) {
      log(`   created attr ${attr.name}`);
      continue;
    }
    // 409 or any "already exists"-style error → idempotent no-op.
    if (resp.status === 409 || /already exists|duplicate/i.test(resp.text)) {
      log(`   attr ${attr.name} already present`);
      continue;
    }
    die(
      `Failed to create attr ${attr.name}: ${resp.status} ${resp.text.slice(0, 300)}`,
    );
  }
}

// ---------------------------------------------------------------------------
// Step 3 — `oap` scope mapping attrs into access and ID tokens
// ---------------------------------------------------------------------------
async function ensureOapScope() {
  log("[3/6] Ensuring `oap` custom scope");

  const list = await rauthy("GET", "/auth/v1/scopes");
  if (!list.ok) {
    die(`Failed to list scopes: ${list.status} ${list.text.slice(0, 200)}`);
  }

  const scopes = Array.isArray(list.json) ? list.json : [];
  const existing = scopes.find((s) => s.name === OAP_SCOPE);

  const body = {
    scope: OAP_SCOPE,
    attr_include_access: OAP_SCOPE_ATTRS,
    attr_include_id: OAP_SCOPE_ATTRS,
  };

  if (!existing) {
    const create = await rauthy("POST", "/auth/v1/scopes", body);
    if (!create.ok) {
      die(
        `Failed to create scope ${OAP_SCOPE}: ${create.status} ${create.text.slice(0, 300)}`,
      );
    }
    log(`   created scope ${OAP_SCOPE}`);
    return;
  }

  const drift =
    !sameSet(existing.attr_include_access || [], OAP_SCOPE_ATTRS) ||
    !sameSet(existing.attr_include_id || [], OAP_SCOPE_ATTRS);

  if (drift) {
    const update = await rauthy(
      "PUT",
      `/auth/v1/scopes/${existing.id}`,
      body,
    );
    if (!update.ok) {
      die(
        `Failed to update scope ${OAP_SCOPE}: ${update.status} ${update.text.slice(0, 300)}`,
      );
    }
    log(`   updated scope ${OAP_SCOPE} (converged attribute mappings)`);
  } else {
    log(`   scope ${OAP_SCOPE} already current`);
  }
}

// ---------------------------------------------------------------------------
// Spec 107 — converge `redirect_uris` on stagecraft/OPC OIDC clients.
// Runs before the scope-grant step so that scope grants target a client
// with a correct URI allow-list. Merge-over-replace: existing URIs
// (operator-added localhost entries, SAML test callbacks, etc.) are left
// alone; only missing targets are appended.
// ---------------------------------------------------------------------------

function computeTargetRedirectUris(clientId) {
  if (clientId === RAUTHY_CLIENT_ID) {
    return [
      `${APP_BASE_URL}/auth/rauthy/callback`,
      `${APP_BASE_URL}/auth/oidc/callback`,
    ];
  }
  if (clientId === OPC_CLIENT_ID) {
    return [OPC_REDIRECT_URI];
  }
  return [];
}

function diffRequired(existing, target) {
  const have = new Set(existing);
  return target.filter((uri) => !have.has(uri));
}

async function fetchClients() {
  const list = await rauthy("GET", "/auth/v1/clients");
  if (!list.ok) {
    die(`Failed to list clients: ${list.status} ${list.text.slice(0, 200)}`);
  }
  return Array.isArray(list.json) ? list.json : [];
}

async function ensureClientRedirectUris() {
  log("[4/6] Converging `redirect_uris` on stagecraft/OPC OIDC clients");

  const clients = await fetchClients();

  // stagecraft-server — hard requirement
  const server = clients.find((x) => x.id === RAUTHY_CLIENT_ID);
  if (!server) {
    die(
      `stagecraft-server client ${RAUTHY_CLIENT_ID} not found in Rauthy — ` +
        `create it in the Rauthy admin UI before running the seeder`,
    );
  }
  await convergeRedirectUris(server);

  // OPC — optional, warn-and-skip when missing
  if (OPC_CLIENT_ID) {
    const opc = clients.find((x) => x.id === OPC_CLIENT_ID);
    if (!opc) {
      log(
        `   skipped ${OPC_CLIENT_ID}: OPC client not present in Rauthy ` +
          `(desktop app may not be provisioned here)`,
      );
    } else {
      await convergeRedirectUris(opc);
    }
  }
}

async function convergeRedirectUris(client) {
  const target = computeTargetRedirectUris(client.id);
  if (target.length === 0) return;

  const existing = Array.isArray(client.redirect_uris)
    ? client.redirect_uris
    : [];
  const missing = diffRequired(existing, target);
  if (missing.length === 0) {
    log(
      `   client ${client.id} redirect_uris already cover target (${existing.length} entries)`,
    );
    return;
  }

  const update = await rauthy("PUT", `/auth/v1/clients/${client.id}`, {
    ...client,
    redirect_uris: [...existing, ...missing],
  });
  if (!update.ok) {
    die(
      `Failed to converge redirect_uris for ${client.id}: ${update.status} ${update.text.slice(0, 300)}`,
    );
  }
  log(
    `   client ${client.id} redirect_uris converged (added ${missing.length}: ${missing.join(", ")})`,
  );
}

// ---------------------------------------------------------------------------
// Step 5 — grant `oap` scope to stagecraft, SPA, and OPC OIDC clients
// ---------------------------------------------------------------------------
async function ensureClientScopeGrants() {
  log("[5/6] Granting `oap` scope to stagecraft/SPA/OPC OIDC clients");

  // Re-fetch: ensureClientRedirectUris may have PUT new state that we need
  // reflected here. Cheap call; hiqlite is local to Rauthy.
  const clients = await fetchClients();

  const targetIds = [RAUTHY_CLIENT_ID, OIDC_SPA_CLIENT_ID, OPC_CLIENT_ID].filter(
    Boolean,
  );

  for (const clientId of targetIds) {
    const c = clients.find((x) => x.id === clientId);
    if (!c) {
      log(`   skipped ${clientId}: not present in Rauthy (not yet created)`);
      continue;
    }

    const scopes = Array.isArray(c.scopes) ? c.scopes : [];
    if (scopes.includes(OAP_SCOPE)) {
      log(`   client ${clientId} already allows scope ${OAP_SCOPE}`);
      continue;
    }

    const update = await rauthy("PUT", `/auth/v1/clients/${clientId}`, {
      ...c,
      scopes: [...scopes, OAP_SCOPE],
    });
    if (!update.ok) {
      die(
        `Failed to grant ${OAP_SCOPE} to client ${clientId}: ${update.status} ${update.text.slice(0, 300)}`,
      );
    }
    log(`   granted ${OAP_SCOPE} to client ${clientId}`);
  }
}

// ---------------------------------------------------------------------------
// Step 6 — smoke validation. Re-read oap scope and fail if attr mapping is
// not in effect. Catches the case where Rauthy accepted the write but
// silently normalised it to something different. Also re-reads
// stagecraft-server's redirect_uris (spec 107 FR-002) so a silent Rauthy
// normalisation (trailing slash, case) fails at deploy instead of first login.
// ---------------------------------------------------------------------------
async function validateSeed() {
  log("[6/6] Validating seeded state");

  const list = await rauthy("GET", "/auth/v1/scopes");
  if (!list.ok) {
    die(`Validation: failed to re-read scopes: ${list.status}`);
  }
  const scope = (list.json || []).find((s) => s.name === OAP_SCOPE);
  if (!scope) {
    die(`Validation: scope ${OAP_SCOPE} missing after seed`);
  }
  if (!sameSet(scope.attr_include_access || [], OAP_SCOPE_ATTRS)) {
    die(
      `Validation: scope ${OAP_SCOPE} attr_include_access = ${JSON.stringify(scope.attr_include_access)}, expected ${JSON.stringify(OAP_SCOPE_ATTRS)}`,
    );
  }

  const clients = await fetchClients();
  const server = clients.find((x) => x.id === RAUTHY_CLIENT_ID);
  if (!server) {
    die(`Validation: stagecraft-server client ${RAUTHY_CLIENT_ID} missing after seed`);
  }
  const required = computeTargetRedirectUris(RAUTHY_CLIENT_ID);
  const existing = Array.isArray(server.redirect_uris) ? server.redirect_uris : [];
  const stillMissing = diffRequired(existing, required);
  if (stillMissing.length) {
    die(
      `Validation: redirect_uris for ${RAUTHY_CLIENT_ID} missing ${stillMissing.join(", ")} after seed`,
    );
  }

  log("   seed looks consistent");
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  const started = Date.now();
  log(`Rauthy seeder starting — target ${RAUTHY_URL}`);

  // Upstream GitHub auth provider is configured manually via the Rauthy admin
  // UI. Rauthy 0.35 does not expose the provider list/create bulk API under
  // /auth/v1/providers for API-Key callers, so ensureGithubProvider() is
  // intentionally skipped here. See seed-rauthy.mjs history for the retired
  // implementation.
  await ensureUserAttributes();
  await ensureOapScope();
  await ensureClientRedirectUris();
  await ensureClientScopeGrants();
  await validateSeed();

  log(`Rauthy seeder complete in ${Date.now() - started}ms.`);
}

main().catch((err) => {
  console.error("Rauthy seeder crashed:", err?.stack || err);
  process.exit(1);
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function must(name) {
  const v = process.env[name];
  if (!v) die(`${name} is required`);
  return v;
}

function mustAbsoluteUrl(name) {
  const v = must(name);
  try {
    const u = new URL(v);
    if (u.protocol !== "https:" && u.protocol !== "http:") {
      die(`${name} must be an http(s) URL, got ${u.protocol}`);
    }
    // Strip trailing slash so callers can template `${APP_BASE_URL}/path`
    // without double-slashes.
    return v.replace(/\/$/, "");
  } catch {
    die(`${name} is not an absolute URL: ${v}`);
  }
}

function die(msg) {
  console.error(msg);
  process.exit(1);
}

function log(msg) {
  console.log(msg);
}

function sameSet(a, b) {
  if (a.length !== b.length) return false;
  const s = new Set(a);
  for (const x of b) if (!s.has(x)) return false;
  return true;
}
