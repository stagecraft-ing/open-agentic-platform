// Spec 112 §5.3 — module catalog ported from
// template-distributor/src/server.ts (lines 108-232). Stagecraft Create's
// web UI lets the user pick optional modules; this module captures the
// closed set of modules + their dependencies + which preset templates
// already include them.
//
// The catalog is template-repo-shaped, not adapter-shaped: profile names
// match the directories `tsx scripts/setup-app.ts` / `setup-dual-app.ts`
// emit (`_prebuilt-{profile}`) and the script names `tsx scripts/add-module.ts`
// accepts.

export interface ModuleDescriptor {
  id: string;
  displayName: string;
  category: string;
  description: string;
  requires: string[];
  conflicts: string[];
}

export const MODULE_CATALOG: ModuleDescriptor[] = [
  // ── Authentication ──────────────────────────────────────────────────
  {
    id: "auth-saml",
    displayName: "SAML 2.0",
    category: "Authentication",
    description: "Alberta.ca Account auth (public/citizen-facing apps)",
    requires: [],
    conflicts: [],
  },
  {
    id: "auth-entra-id",
    displayName: "Entra ID",
    category: "Authentication",
    description: "Microsoft Entra ID / Azure AD (staff-facing apps)",
    requires: [],
    conflicts: [],
  },
  // ── Data Access ─────────────────────────────────────────────────────
  {
    id: "data-redis",
    displayName: "Redis",
    category: "Data Access",
    description: "Redis client with access-key and Entra ID auth modes",
    requires: [],
    conflicts: [],
  },
  {
    id: "data-postgres",
    displayName: "PostgreSQL",
    category: "Data Access",
    description: "PostgreSQL pool with Azure compliance (SSL, retry, metrics)",
    requires: [],
    conflicts: [],
  },
  // ── Session Store ───────────────────────────────────────────────────
  {
    id: "session-store-redis",
    displayName: "Redis Sessions",
    category: "Session Store",
    description: "Redis session store for express-session",
    requires: ["data-redis"],
    conflicts: ["session-store-postgres"],
  },
  {
    id: "session-store-postgres",
    displayName: "PostgreSQL Sessions",
    category: "Session Store",
    description: "PostgreSQL session store for express-session",
    requires: ["data-postgres"],
    conflicts: ["session-store-redis"],
  },
  // ── Infrastructure ──────────────────────────────────────────────────
  {
    id: "service-auth",
    displayName: "Service Auth",
    category: "Infrastructure",
    description:
      "Azure AD service-to-service JWT validation (Client Credentials flow)",
    requires: [],
    conflicts: [],
  },
  {
    id: "api-gateway",
    displayName: "API Gateway",
    category: "Infrastructure",
    description:
      "BFF gateway/proxy layer for routing requests to backend services",
    requires: [],
    conflicts: [],
  },
  {
    id: "api-docs",
    displayName: "API Docs",
    category: "Infrastructure",
    description: "OpenAPI/Swagger documentation UI served at /api-docs",
    requires: [],
    conflicts: [],
  },
  // ── Application ─────────────────────────────────────────────────────
  {
    id: "user-management",
    displayName: "User/Role Management",
    category: "Application",
    description:
      "Admin UI for user and role management with IdP-to-DB sync on login",
    requires: ["data-postgres"],
    conflicts: [],
  },
];

export type Profile = "minimal" | "public" | "internal" | "dual";

export const PROFILES: ReadonlyArray<Profile> = [
  "minimal",
  "public",
  "internal",
  "dual",
];

// Modules that setup-app.ts / setup-dual-app.ts install automatically per
// profile. Mirrors the module arrays in the template setup scripts —
// security-core and auth-core are always-on, hence not user-selectable.
export const PROFILE_MODULES: Record<Profile, string[]> = {
  minimal: [],
  public: [
    "security-core",
    "data-redis",
    "auth-saml",
    "session-store-redis",
    "api-gateway",
  ],
  internal: [
    "security-core",
    "data-postgres",
    "auth-entra-id",
    "session-store-postgres",
    "service-auth",
  ],
  dual: [
    "security-core",
    "data-redis",
    "auth-saml",
    "session-store-redis",
    "api-gateway",
    "data-postgres",
    "auth-entra-id",
    "session-store-postgres",
    "service-auth",
    "user-management",
  ],
};

// Order ensures user-selected dependencies are installed before dependents.
export const INSTALL_ORDER: string[] = [
  "data-redis",
  "data-postgres",
  "auth-saml",
  "auth-entra-id",
  "session-store-redis",
  "session-store-postgres",
  "service-auth",
  "api-gateway",
  "api-docs",
  "user-management",
];

// User-selectable presets per profile. `dual` is empty because dual
// modules are managed by setup-dual-app.ts.
export const PRESETS: Record<Profile, string[]> = {
  minimal: [],
  public: ["data-redis", "auth-saml", "session-store-redis", "api-gateway"],
  internal: [
    "data-postgres",
    "auth-entra-id",
    "session-store-postgres",
    "service-auth",
  ],
  dual: [],
};

/**
 * Detect the prebuild profile from the user's module selection. Auth driver
 * is the key signal: SAML → public, Entra ID → internal, otherwise minimal.
 */
export function detectProfile(
  modules: string[]
): "public" | "internal" | "minimal" {
  if (modules.includes("auth-saml")) return "public";
  if (modules.includes("auth-entra-id")) return "internal";
  return "minimal";
}

/**
 * Pick the prebuild profile from (variant, modules). Maps build-spec
 * variants to template-distributor profile names:
 *   "dual"            → "dual"
 *   "single-public"   → "public"
 *   "single-internal" → "internal"
 *   any other value   → detectProfile(modules)
 */
export function pickProfileFromModules(
  variant: string,
  modules: string[]
): Profile {
  if (variant === "dual") return "dual";
  if (variant === "single-public") return "public";
  if (variant === "single-internal") return "internal";
  return detectProfile(modules);
}

/**
 * Modules that need to be installed via add-module.ts on top of the prebuilt
 * profile, sorted in dependency-respecting INSTALL_ORDER. Modules already
 * shipped in the profile are filtered out. Unknown modules are dropped
 * silently — the API layer rejects unknown ids before reaching this helper.
 */
export function extrasFor(profile: Profile, selected: string[]): string[] {
  const builtIn = new Set(PROFILE_MODULES[profile] ?? []);
  const wanted = new Set(selected);
  return INSTALL_ORDER.filter((m) => wanted.has(m) && !builtIn.has(m));
}

/**
 * Is the given id a known module from MODULE_CATALOG? Used to reject the
 * occasional typo'd or removed module from the form before we hand it to
 * add-module.ts.
 */
export function isKnownModule(id: string): boolean {
  return MODULE_CATALOG.some((m) => m.id === id);
}
