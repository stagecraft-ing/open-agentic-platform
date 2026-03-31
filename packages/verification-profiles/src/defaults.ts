import type { VerificationSkill } from "./types.js";

/**
 * Bundled platform default skills.
 * These provide baseline verification for common project types.
 * Local project skills (`.verification/skills/`) override these by name (R-004).
 * Phase 6 fleshes out additional defaults.
 */

const lint: VerificationSkill = {
  name: "lint",
  description: "Run linting across all source files",
  determinism: "deterministic",
  safety_tier: "safe",
  steps: [
    {
      command: "npm run lint",
      timeout: 120,
      read_only: true,
      network: "deny",
    },
  ],
};

const typeCheck: VerificationSkill = {
  name: "type-check",
  description: "Run static type checking",
  determinism: "deterministic",
  safety_tier: "safe",
  steps: [
    {
      command: "npx tsc --noEmit",
      timeout: 180,
      read_only: true,
      network: "deny",
    },
  ],
};

const unitTests: VerificationSkill = {
  name: "unit-tests",
  description: "Run the project unit test suite",
  determinism: "mostly_deterministic",
  safety_tier: "safe",
  steps: [
    {
      command: "npm test",
      timeout: 300,
      read_only: true,
      network: "deny",
    },
  ],
};

/** All bundled platform default skills, keyed by name. */
export function getDefaultSkills(): Map<string, VerificationSkill> {
  const map = new Map<string, VerificationSkill>();
  for (const skill of [lint, typeCheck, unitTests]) {
    map.set(skill.name, skill);
  }
  return map;
}
