import { readFile } from "node:fs/promises";
import { join } from "node:path";
import type { GateResult, VerificationDiagnostic } from "./types.js";
import { parseProfileFile } from "./parser.js";
import { loadSkillLibrary } from "./loader.js";
import { executeProfile } from "./runner.js";
import type { ExecutionOptions } from "./runner.js";

/** Directory within a project root where profiles are stored. */
const PROFILES_DIR = ".verification/profiles";

async function loadProfileContent(
  profileName: string,
  projectRoot: string,
): Promise<{ content: string; filePath: string } | null> {
  const yamlPath = join(projectRoot, PROFILES_DIR, `${profileName}.yaml`);
  try {
    const content = await readFile(yamlPath, "utf-8");
    return { content, filePath: yamlPath };
  } catch {
    const ymlPath = join(projectRoot, PROFILES_DIR, `${profileName}.yml`);
    try {
      const content = await readFile(ymlPath, "utf-8");
      return { content, filePath: ymlPath };
    } catch {
      return null;
    }
  }
}

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
  const loaded = await loadProfileContent(profileName, projectRoot);
  if (!loaded) {
    return {
      passed: false,
      gated: true,
      profile: profileName,
      results: [],
      failedSkills: [],
      durationMs: Date.now() - start,
    };
  }

  const parseResult = parseProfileFile(loaded.content, loaded.filePath);
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
  const loaded = await loadProfileContent(profileName, projectRoot);
  if (!loaded) {
    const yamlPath = join(projectRoot, PROFILES_DIR, `${profileName}.yaml`);
    const ymlPath = join(projectRoot, PROFILES_DIR, `${profileName}.yml`);
    return [
      {
        code: "VP_PROFILE_NOT_FOUND",
        severity: "error",
        message: `Profile "${profileName}" not found at ${yamlPath} or ${ymlPath}.`,
        filePath: yamlPath,
      },
    ];
  }

  const parseResult = parseProfileFile(loaded.content, loaded.filePath);
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
          filePath: loaded.filePath,
        });
      }
    }
  }

  return diagnostics;
}
