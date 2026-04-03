import fs from "node:fs";

function must(name: string): string {
  const v = process.env[name];
  if (!v) throw new Error(`Missing env var ${name}`);
  return v;
}

const LOGTO_ADMIN_URL = must("LOGTO_ADMIN_URL"); // ex https://logto-admin.stagecraft.ing
const LOGTO_MANAGEMENT_API_ACCESS_TOKEN = must("LOGTO_MANAGEMENT_API_ACCESS_TOKEN");
const DEPLOYD_AUDIENCE = must("DEPLOYD_AUDIENCE"); // ex https://api.deployd.xyz
const DEPLOYD_SCOPE = process.env.DEPLOYD_SCOPE ?? "deployd:deploy";

// Where to write outputs for terraform.tfvars
const OUT_PATH = process.env.OUT_PATH ?? "logto.m2m.out.json";

const M2M_APP_NAME = "stagecraft-m2m";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function logtoFetch(
  method: string,
  path: string,
  body?: unknown
): Promise<Response> {
  const url = `${LOGTO_ADMIN_URL}${path}`;
  const res = await fetch(url, {
    method,
    headers: {
      Authorization: `Bearer ${LOGTO_MANAGEMENT_API_ACCESS_TOKEN}`,
      "Content-Type": "application/json",
    },
    ...(body !== undefined && { body: JSON.stringify(body) }),
  });
  return res;
}

async function logtoJson<T = unknown>(
  method: string,
  path: string,
  body?: unknown
): Promise<T> {
  const res = await logtoFetch(method, path, body);
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Logto API ${method} ${path} → ${res.status}: ${text}`);
  }
  return res.json() as Promise<T>;
}

// ---------------------------------------------------------------------------
// Step 1: Ensure API Resource exists
// ---------------------------------------------------------------------------

interface ApiResource {
  id: string;
  indicator: string;
  name: string;
}

async function ensureApiResource(): Promise<ApiResource> {
  // Try to create; Logto returns 422 if the indicator already exists
  const createRes = await logtoFetch("POST", "/api/resources", {
    indicator: DEPLOYD_AUDIENCE,
    name: "deployd-api",
  });

  if (createRes.ok) {
    const resource = (await createRes.json()) as ApiResource;
    console.log(`Created API resource: ${resource.id} (${resource.indicator})`);
    return resource;
  }

  // Read error body before it becomes inaccessible
  const createError = await createRes.text().catch(() => "unknown");

  // Already exists — find it (page_size=100 covers most instances)
  const resources = await logtoJson<ApiResource[]>(
    "GET",
    "/api/resources?page_size=100"
  );
  const existing = resources.find((r) => r.indicator === DEPLOYD_AUDIENCE);
  if (!existing) {
    throw new Error(
      `Failed to create API resource (${createError}) and could not find existing resource for ${DEPLOYD_AUDIENCE}`
    );
  }
  console.log(
    `API resource already exists: ${existing.id} (${existing.indicator})`
  );
  return existing;
}

// ---------------------------------------------------------------------------
// Step 2: Ensure scope exists on the resource
// ---------------------------------------------------------------------------

interface ResourceScope {
  id: string;
  name: string;
  resourceId: string;
}

async function ensureScope(resourceId: string): Promise<ResourceScope> {
  // List existing scopes
  const scopes = await logtoJson<ResourceScope[]>(
    "GET",
    `/api/resources/${resourceId}/scopes`
  );
  const existing = scopes.find((s) => s.name === DEPLOYD_SCOPE);
  if (existing) {
    console.log(`Scope already exists: ${existing.name}`);
    return existing;
  }

  const scope = await logtoJson<ResourceScope>(
    "POST",
    `/api/resources/${resourceId}/scopes`,
    { name: DEPLOYD_SCOPE }
  );
  console.log(`Created scope: ${scope.name}`);
  return scope;
}

// ---------------------------------------------------------------------------
// Step 3: Ensure M2M application exists
// ---------------------------------------------------------------------------

interface LogtoApplication {
  id: string;
  name: string;
  secret: string;
  type: string;
}

async function ensureM2MApp(): Promise<LogtoApplication> {
  // Try to create
  const createRes = await logtoFetch("POST", "/api/applications", {
    name: M2M_APP_NAME,
    type: "MachineToMachine",
  });

  if (createRes.ok) {
    const app = (await createRes.json()) as LogtoApplication;
    console.log(`Created M2M application: ${app.id} (${app.name})`);
    return app;
  }

  // Read error body before it becomes inaccessible
  const createError = await createRes.text().catch(() => "unknown");

  // List and find existing (page_size=100 covers most instances)
  const apps = await logtoJson<LogtoApplication[]>(
    "GET",
    "/api/applications?page_size=100"
  );
  const existing = apps.find(
    (a) => a.name === M2M_APP_NAME && a.type === "MachineToMachine"
  );
  if (!existing) {
    throw new Error(
      `Failed to create M2M app (${createError}) and could not find existing app named "${M2M_APP_NAME}"`
    );
  }
  console.log(
    `M2M application already exists: ${existing.id} (${existing.name})`
  );
  return existing;
}

// ---------------------------------------------------------------------------
// Step 4: Create an M2M role and assign it to the application
// ---------------------------------------------------------------------------

interface LogtoRole {
  id: string;
  name: string;
  type: string;
}

const M2M_ROLE_NAME = "stagecraft-deployd-access";

async function ensureM2MRole(scopeId: string): Promise<LogtoRole> {
  // Try to create a machine-to-machine role with the deployd scope
  const createRes = await logtoFetch("POST", "/api/roles", {
    name: M2M_ROLE_NAME,
    description: "Grants stagecraft M2M app access to deployd-api",
    type: "MachineToMachine",
    scopeIds: [scopeId],
  });

  if (createRes.ok) {
    const role = (await createRes.json()) as LogtoRole;
    console.log(`Created M2M role: ${role.id} (${role.name})`);
    return role;
  }

  // Read error body before it becomes inaccessible
  const createError = await createRes.text().catch(() => "unknown");

  // Already exists — find it (page_size=100 covers most instances)
  const roles = await logtoJson<LogtoRole[]>(
    "GET",
    `/api/roles?type=MachineToMachine&page_size=100`
  );
  const existing = roles.find((r) => r.name === M2M_ROLE_NAME);
  if (!existing) {
    throw new Error(
      `Failed to create M2M role (${createError}) and could not find existing role "${M2M_ROLE_NAME}"`
    );
  }
  console.log(`M2M role already exists: ${existing.id} (${existing.name})`);
  return existing;
}

async function assignRoleToApp(
  appId: string,
  roleId: string
): Promise<void> {
  const res = await logtoFetch(
    "POST",
    `/api/roles/${roleId}/applications`,
    { applicationIds: [appId] }
  );

  if (res.ok || res.status === 422) {
    // 422 = already assigned
    console.log(`Assigned role ${roleId} to M2M app ${appId}`);
    return;
  }

  const text = await res.text();
  throw new Error(
    `Failed to assign role to M2M app: ${res.status} ${text}`
  );
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  console.log("Logto M2M Bootstrap");
  console.log("- Admin URL:", LOGTO_ADMIN_URL);
  console.log("- Deployd audience:", DEPLOYD_AUDIENCE);
  console.log("- Deployd scope:", DEPLOYD_SCOPE);
  console.log("");

  const resource = await ensureApiResource();
  const scope = await ensureScope(resource.id);
  const app = await ensureM2MApp();
  const role = await ensureM2MRole(scope.id);
  await assignRoleToApp(app.id, role.id);

  const output = {
    deploydAudience: DEPLOYD_AUDIENCE,
    deploydScope: DEPLOYD_SCOPE,
    logtoApiResourceId: resource.id,
    logtoM2MClientId: app.id,
    logtoM2MClientSecret: app.secret,
  };

  fs.writeFileSync(OUT_PATH, JSON.stringify(output, null, 2));
  console.log(`\nWrote ${OUT_PATH}. Paste values into terraform.tfvars.`);
}

main().catch((err) => {
  console.error("Bootstrap failed:", err);
  process.exit(1);
});
