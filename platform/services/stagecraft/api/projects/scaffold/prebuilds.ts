// Spec 112 §5.3 operation 2 — profile prebuilds (minimal / public / internal / dual).
//
// Each adapter.scaffold.profile is pre-warmed so subsequent per-request
// scaffolds only need a clean copy + project-specific setup step. The
// warm set is driven by the adapter's declared profile list (spec 112
// §8) rather than a hardcoded stagecraft list.

import type { AdapterScaffoldBlock } from "./types";

export interface PrebuiltProfile {
  name: string;
  variant: string;
  path: string;
  warmedAt: string;
}

/**
 * Given an adapter's scaffold block, enumerate which profiles should be
 * pre-warmed on startup. Returns the profile names that production
 * infrastructure is expected to materialise alongside the template cache.
 * The actual copy/compile work is deferred to the infra layer; this
 * function is pure so the planner and tests can reason about it.
 */
export function declaredPrebuildProfiles(
  scaffold: AdapterScaffoldBlock
): Array<{ name: string; variant: string; isDefault: boolean }> {
  const out: Array<{ name: string; variant: string; isDefault: boolean }> = [];
  for (const profile of scaffold.profiles ?? []) {
    out.push({
      name: profile.name,
      variant: profile.variant,
      isDefault: profile.default === true,
    });
  }
  return out;
}

export function pickProfile(
  scaffold: AdapterScaffoldBlock,
  requested: { profileName?: string; variant: string }
): { name: string; variant: string; modules: string[] } | null {
  const profiles = scaffold.profiles ?? [];
  if (profiles.length === 0) return null;

  if (requested.profileName) {
    const found = profiles.find((p) => p.name === requested.profileName);
    if (!found) return null;
    if (found.variant !== requested.variant) return null;
    return { name: found.name, variant: found.variant, modules: found.modules ?? [] };
  }

  // No explicit profile name → pick default-for-variant, else first-for-variant.
  const candidates = profiles.filter((p) => p.variant === requested.variant);
  if (candidates.length === 0) return null;
  const picked =
    candidates.find((p) => p.default === true) ?? candidates[0];
  return { name: picked.name, variant: picked.variant, modules: picked.modules ?? [] };
}
