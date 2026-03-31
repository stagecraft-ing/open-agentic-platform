import { readFile } from "node:fs/promises";
import { join } from "node:path";
import type { GateResult, VerificationDiagnostic } from "./types.js";
import { parseProfileFile } from "./parser.js";
import { loadSkillLibrary } from "./loader.js";
import { executeProfile } from "./runner.js";
import type { ExecutionOptions } from "./runner.js";

/** Directory within a project root where profiles are stored. */
const PROFILES_DIR = ".verification/profiles";

/**
 * Evaluate a post-session verification gate (FR-004).
 *
 * Loads the named profile from `<projectRoot>/.verification/profiles/<profileName>.yaml`,
 * resolves all skill references from the project's skill library,
 * executes the profile, and applies gate semantics:
 *
 * - If `profile.gate` is true and any skill fails, `passed` is false (delivery blocked).
 * - If `profile.gate` is false, `passed` is always true regardless of skill results (advisory only).
 *
 * Returns a `GateResult` with detailed per-skill results and failure information.
 */
export async function evaluatePostSessionGate(
  profileName: string,
  projectRoot: string,
  opts?: ExecutionOptions,
): Promise<GateResult> {
  const start = Date.now();

  // Load and parse the profile YAML.
  const profilePath = join(projectRoot, PROFILES_DIR, `${profileName}.yaml`);
  let content: string;
  try {
    content = await readFile(profilePath, "utf-8");
  } catch {
    // Also try .yml extension.
    const ymlPath = join(projectRoot, PROFILES_DIR, `${profileName}.yml`);
    try {
      content = await readFile(ymlPath, "utf-8");
    } catch {
      return {
        passed: false,
        gated: true,
        profile: profileName,
        results: [],
        failedSkills: [],
        durationMs: Date.now() - start,
      };
    }
  }

  const parseResult = parseProfileFile(content, profilePath);
  if (!parseResult.profile) {
    return {
      passed: false,
      gated: true,
      profile: profileName,
      results: [],
      failedSkills: [],
      durationMs: Date.now() - start,
    };
  }

  const profile = parseResult.profile;

  // Load skill library from project.
  const library = await loadSkillLibrary(projectRoot);

  // Execute the profile.
  const profileResult = await executeProfile(profile, library, opts);

  // Apply gate semantics (FR-004, P3-004):
  // If gate is true, delivery blocked on any failure.
  // If gate is false, passed is always true (advisory mode).
  const gated = profile.gate;
  const passed = gated ? profileResult.passed : true;

  return {
    passed,
    gated,
    profile: profile.name,
    results: profileResult.skills,
    failedSkills: profileResult.failedSkills,
    durationMs: Date.now() - start,
  };
}

/**
 * Collect diagnostics from loading a profile without executing it.
 * Useful for validation-only workflows (e.g., linting profile configs).
 */
export async function loadProfileDiagnostics(
  profileName: string,
  projectRoot: string,
): Promise<VerificationDiagnostic[]> {
  const profilePath = join(projectRoot, PROFILES_DIR, `${profileName}.yaml`);
  let content: string;
  try {
    content = await readFile(profilePath, "utf-8");
  } catch {
    const ymlPath = join(projectRoot, PROFILES_DIR, `${profileName}.yml`);
    try {
      content = await readFile(ymlPath, "utf-8");
    } catch {
      return [
        {
          code: "VP_PROFILE_NOT_FOUND",
          severity: "error",
          message: `Profile "${profileName}" not found at ${profilePath} or ${ymlPath}.`,
          filePath: profilePath,
        },
      ];
    }
  }

  const parseResult = parseProfileFile(content, profilePath);
  const library = await loadSkillLibrary(projectRoot);

  const diagnostics = [...parseResult.diagnostics, ...library.diagnostics];

  // Check for unresolvable skill references.
  if (parseResult.profile) {
    for (const skillName of parseResult.profile.skills) {
      if (!library.skills.has(skillName)) {
        diagnostics.push({
          code: "VP_SKILL_NOT_FOUND",
          severity: "error",
          message: `Skill "${skillName}" referenced in profile "${profileName}" not found in library.`,
          filePath: profilePath,
        });
      }
    }
  }

  return diagnostics;
}
