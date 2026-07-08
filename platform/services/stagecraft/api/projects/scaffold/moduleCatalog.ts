// Spec 112 §5.3: module catalog; spec 138 realised-scaffold surface.
// Spec 227 Stage 1 (catalog derivation): the module descriptor list is no
// longer a hand-authored constant. It is derived at request time from the
// adapter's module `manifest.json` rows in the substrate (the
// stagecraft-resident adapter surface, spec 160), so the drift-prone module
// prose (notably the retired data-redis rate-limit-backend label) cannot
// diverge from the adapter by construction. The Encore-side loader +
// `GET /api/factory/module-catalog` endpoint live in
// `api/factory/moduleCatalog.ts`; this file stays PURE (no Encore imports) so
// the derivation is unit-testable without infra.
//
// Two template facts still shape the profile helpers (scripts/setup-app.ts):
//   1. Profiles select the default AUTH_DRIVER; auth is NOT a module.
//   2. NO modules ship by default; optional domain modules are composed
//      via `--with <module>`.

export interface ModuleDescriptor {
  id: string;
  displayName: string;
  category: string;
  description: string;
  requires: string[];
  conflicts: string[];
  /** Module lifecycle status from the manifest (e.g. "stable"). */
  status?: string;
}

/**
 * The subset of a module `manifest.json` the catalog derives from. Ids/prose
 * are manifest-owned; `displayName`/`category` are stagecraft presentation
 * (see PRESENTATION below) because the manifest has no source for them.
 */
export interface RawModuleManifest {
  name: string;
  description?: string;
  requires?: string[];
  conflicts?: string[];
  status?: string;
}

// Transitional presentation overlay (spec 227 Stage 1). The module manifest
// carries no display label or grouping category, so these two UI-only fields
// are supplied here rather than derived. This is deliberately a thin, cosmetic
// map: the drift-prone content (description/requires/conflicts) derives from
// the manifest, while a label and a section heading do not. Stage 2's form
// re-sectioning (Infrastructure / Application-features / Base-app config)
// replaces the `category` grouping, at which point this overlay shrinks or
// goes away. Unknown-upstream modules fall back to a title-cased id + "Other".
// A Map (not a plain object) so an externally-authored module named `__proto__`
// or `constructor` misses cleanly (`.get` returns undefined) rather than
// resolving to a truthy prototype value and bypassing the fallback.
const PRESENTATION = new Map<string, { displayName: string; category: string }>([
  ["security-core", { displayName: "Security Core", category: "Infrastructure" }],
  ["api-gateway", { displayName: "API Gateway", category: "Infrastructure" }],
  ["data-postgres", { displayName: "PostgreSQL", category: "Data" }],
  ["data-redis", { displayName: "Redis", category: "Data" }],
  ["user-management", { displayName: "User/Role Management", category: "Application" }],
]);

function titleCase(id: string): string {
  return id
    .split(/[-_]/)
    .filter(Boolean)
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

// The manifest body is JSON.parse'd and cast to RawModuleManifest, so a
// malformed upstream (e.g. `"requires": "security-core"` as a string instead of
// an array) would survive the cast. A bare spread `[...m.requires]` would then
// iterate the string into single characters. Coerce to a clean string[] so a
// malformed manifest yields an empty edge list rather than junk dependency ids.
function asStringArray(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((v): v is string => typeof v === "string")
    : [];
}

/**
 * Derive the module descriptor catalog from the adapter's module manifests.
 * `id`, `description`, `requires`, `conflicts`, `status` come from the manifest;
 * `displayName`/`category` come from the PRESENTATION overlay (title-cased id +
 * "Other" fallback). Sorted by id for deterministic output.
 */
export function deriveModuleCatalog(
  manifests: RawModuleManifest[]
): ModuleDescriptor[] {
  return manifests
    .filter((m) => typeof m.name === "string" && m.name.length > 0)
    .map((m) => {
      const overlay = PRESENTATION.get(m.name) ?? {
        displayName: titleCase(m.name),
        category: "Other",
      };
      return {
        id: m.name,
        displayName: overlay.displayName,
        category: overlay.category,
        description: typeof m.description === "string" ? m.description : "",
        requires: asStringArray(m.requires),
        conflicts: asStringArray(m.conflicts),
        status: typeof m.status === "string" ? m.status : undefined,
      };
    })
    .sort((a, b) => a.id.localeCompare(b.id));
}

/**
 * Dependency-respecting install order for the catalog: a module always sorts
 * after every id in its `requires` (e.g. security-core before api-gateway).
 * Kahn topological sort with a stable id tiebreak; `requires` entries pointing
 * at ids not in the catalog are ignored (they cannot gate an install here).
 */
export function deriveInstallOrder(catalog: ModuleDescriptor[]): string[] {
  const ids = new Set(catalog.map((m) => m.id));
  const deps = new Map<string, Set<string>>();
  for (const m of catalog) {
    deps.set(m.id, new Set(m.requires.filter((r) => ids.has(r) && r !== m.id)));
  }
  const order: string[] = [];
  const placed = new Set<string>();
  // Repeatedly emit every id whose deps are all placed, lowest id first, until
  // no progress. Any residual (a dependency cycle) is appended id-sorted so the
  // function is total rather than dropping modules.
  let progress = true;
  while (progress) {
    progress = false;
    const ready = catalog
      .map((m) => m.id)
      .filter(
        (id) =>
          !placed.has(id) &&
          [...(deps.get(id) ?? [])].every((d) => placed.has(d))
      )
      .sort((a, b) => a.localeCompare(b));
    for (const id of ready) {
      order.push(id);
      placed.add(id);
      progress = true;
    }
  }
  for (const m of catalog) {
    if (!placed.has(m.id)) order.push(m.id);
  }
  return order;
}

// A substrate path that is an adapter module's manifest.json. The factory
// source mirrors `adapters/**` into the substrate (translator.ts) where module
// manifests land in the catch-all `reference-data` kind, so the module-catalog
// loader selects them by PATH. Kept here (pure) so it is unit-testable without
// the Encore endpoint's runtime imports.
const MODULE_MANIFEST_PATH =
  /^adapters\/[^/]+\/modules\/[^/]+\/manifest\.json$/;

export function isModuleManifestPath(path: string): boolean {
  return MODULE_MANIFEST_PATH.test(path);
}

export type Profile = "minimal" | "public" | "internal" | "dual";

export const PROFILES: ReadonlyArray<Profile> = [
  "minimal",
  "public",
  "internal",
  "dual",
];

// Empty for Stage 1 (template fact 2): profiles select AUTH_DRIVER only; no
// module is treated as a profile built-in here. NOTE (spec 227 Stage 2): the
// adapter manifest's `scaffold.profiles[].modules` does declare per-profile
// defaults (internal ships ["user-management"]); Stage 2's form reframe will
// derive these from the manifest and surface "auto for Internal". Until then
// the runtime dedupe in perRequestScaffold.readInstalledModules keeps
// composition correct, so this stays empty and behavior is unchanged.
export const PROFILE_MODULES: Record<Profile, string[]> = {
  minimal: [],
  public: [],
  internal: [],
  dual: [],
};

// User-selectable pre-checked presets per profile; empty by design, same
// source as PROFILE_MODULES (see the Stage 2 note above).
export const PRESETS: Record<Profile, string[]> = {
  minimal: [],
  public: [],
  internal: [],
  dual: [],
};

/**
 * Pick the prebuild profile from the build-spec variant. Maps variants to
 * the `_prebuilt-{profile}` names the setup scripts emit:
 *   "dual"            to "dual"
 *   "single-public"   to "public"
 *   "single-internal" to "internal"
 *   any other value   to "minimal"
 *
 * Module-blind: auth is the AUTH_DRIVER profile axis, not a module
 * (template fact 1), so the second argument is unused.
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
 * profile, in dependency-respecting order (derived from the catalog). Modules
 * already shipped in the profile (PROFILE_MODULES built-ins) are filtered out;
 * unknown modules are dropped silently (the API layer rejects unknown ids via
 * `isKnownModule` before reaching this helper).
 */
export function extrasFor(
  catalog: ModuleDescriptor[],
  profile: Profile,
  selected: string[]
): string[] {
  const known = new Set(catalog.map((m) => m.id));
  const builtIn = new Set(PROFILE_MODULES[profile] ?? []);
  const wanted = new Set(selected.filter((id) => known.has(id)));
  return deriveInstallOrder(catalog).filter(
    (m) => wanted.has(m) && !builtIn.has(m)
  );
}

/**
 * Is the given id a known module in the derived catalog? Used to reject a
 * typo'd or removed module from the form before it is handed to add-module.ts.
 */
export function isKnownModule(
  catalog: ModuleDescriptor[],
  id: string
): boolean {
  return catalog.some((m) => m.id === id);
}
