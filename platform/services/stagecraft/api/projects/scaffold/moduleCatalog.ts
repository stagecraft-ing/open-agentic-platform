// Spec 112 §5.3 — module catalog; spec 138 realised-scaffold surface.
// Cut over 2026-06-11 (spec 199 FR-007 hygiene) from the retired
// template-distributor catalog to template's real `modules/`
// directory: descriptors mirror each module's `manifest.json` (the catalog
// add-module.ts actually composes). Two template facts shape this
// file (scripts/setup-app.ts header):
//
//   1. Profiles select the default AUTH_DRIVER — auth is NOT a module.
//      setup-app.ts owns minimal/public/internal; the dual variant is the
//      separate setup-dual-app.ts generator. The express-session-era
//      `auth-*` / `session-store-*` ids are retired (auth is stateless
//      RS256 JWT).
//   2. NO modules ship by default; optional domain modules are composed
//      via `--with <module>`, so the per-profile built-in sets and the
//      pre-checked presets are empty by design.

export interface ModuleDescriptor {
  id: string;
  displayName: string;
  category: string;
  description: string;
  requires: string[];
  conflicts: string[];
}

export const MODULE_CATALOG: ModuleDescriptor[] = [
  // ── Infrastructure ──────────────────────────────────────────────────
  {
    id: "security-core",
    displayName: "Security Core",
    category: "Infrastructure",
    description:
      "Cross-cutting security overlay — documents the CORS origin knob " +
      "(encore.app global_cors); Helmet/CSP, rate limiting, and structured " +
      "logging already ship in the base app",
    requires: [],
    conflicts: [],
  },
  {
    id: "api-gateway",
    displayName: "API Gateway",
    category: "Infrastructure",
    description:
      "BFF gateway opt-in — documents the private-backend config knobs and " +
      "adds the /connectivity test page; the gateway proxy service itself " +
      "ships in the base app",
    requires: ["security-core"],
    conflicts: [],
  },
  // ── Data ────────────────────────────────────────────────────────────
  {
    id: "data-postgres",
    displayName: "PostgreSQL",
    category: "Data",
    description:
      "Declarative marker — persistence is the base app's " +
      'SQLDatabase("app"); Encore owns pooling, health, and migrations',
    requires: [],
    conflicts: [],
  },
  {
    id: "data-redis",
    displayName: "Redis",
    category: "Data",
    description:
      "Optional rate-limit backend — when REDIS_URL is set, lib/rate-limit " +
      "uses it instead of the in-memory limiter. Not a session store (auth " +
      "is stateless RS256 JWT)",
    requires: [],
    conflicts: [],
  },
  // ── Application ─────────────────────────────────────────────────────
  {
    id: "user-management",
    displayName: "User/Role Management",
    category: "Application",
    description:
      "User + role management as an Encore service — app-managed role " +
      "catalog with admin CRUD behind auth + requireRole; the reference " +
      "shape for feature modules",
    requires: [],
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

// Empty by design (template fact 2): profiles select AUTH_DRIVER
// only; no module ships as part of a profile. Kept as a map so `extrasFor`
// keeps its shape if a future template version reintroduces built-ins.
export const PROFILE_MODULES: Record<Profile, string[]> = {
  minimal: [],
  public: [],
  internal: [],
  dual: [],
};

// Order ensures user-selected dependencies are installed before dependents
// (api-gateway requires security-core per its manifest).
export const INSTALL_ORDER: string[] = [
  "security-core",
  "data-postgres",
  "data-redis",
  "api-gateway",
  "user-management",
];

// User-selectable pre-checked presets per profile — empty by design
// (template fact 2): modules are opt-in composition, never implied
// by the variant. The auth axis the old presets encoded lives in
// AUTH_DRIVER now.
export const PRESETS: Record<Profile, string[]> = {
  minimal: [],
  public: [],
  internal: [],
  dual: [],
};

/**
 * Pick the prebuild profile from the build-spec variant. Maps variants to
 * the `_prebuilt-{profile}` names the setup scripts emit:
 *   "dual"            → "dual"
 *   "single-public"   → "public"
 *   "single-internal" → "internal"
 *   any other value   → "minimal"
 *
 * Modules no longer participate: the retired catalog inferred the profile
 * from `auth-*` module ids, but auth is the AUTH_DRIVER profile axis, not
 * a module (template fact 1).
 */
export function pickProfileFromModules(
  variant: string,
  _modules: string[]
): Profile {
  if (variant === "dual") return "dual";
  if (variant === "single-public") return "public";
  if (variant === "single-internal") return "internal";
  return "minimal";
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
