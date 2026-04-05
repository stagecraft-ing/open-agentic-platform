"""Stage gate runner — executes verification checks for a pipeline stage."""

from __future__ import annotations

from pathlib import Path
from typing import Any

from .checks.runners import (
    CheckResult,
    run_artifact_exists,
    run_command,
    run_cross_reference,
    run_file_check,
    run_grep_absent,
    run_grep_present,
    run_schema_validation,
)


def run_stage_gate(
    stage_id: str,
    project_root: Path,
    checks: list[dict[str, Any]],
) -> tuple[bool, list[CheckResult]]:
    """Run all checks for a stage gate. Returns (all_passed, results)."""
    results: list[CheckResult] = []

    for check in checks:
        check_id = check["id"]
        check_type = check.get("type", "")
        severity = check.get("severity", "error")

        if check_type == "artifact-exists":
            artifact = check.get("artifact", "")
            result = run_artifact_exists(check_id, project_root / artifact, severity)

        elif check_type == "schema-validation":
            artifact = check.get("artifact", "")
            schema = check.get("schema", "")
            result = run_schema_validation(
                check_id, project_root / artifact, project_root / schema, severity
            )

        elif check_type == "grep-absent":
            pattern = check.get("pattern", "")
            scope = check.get("scope", ".")
            result = run_grep_absent(check_id, pattern, project_root / scope, severity)

        elif check_type == "grep-present":
            pattern = check.get("pattern", "")
            scope = check.get("scope", ".")
            result = run_grep_present(check_id, pattern, project_root / scope, severity)

        elif check_type == "command-succeeds":
            command = check.get("pattern", check.get("command", ""))
            result = run_command(check_id, command, project_root, severity)

        elif check_type == "file-check":
            paths = [project_root / p for p in check.get("files", [])]
            result = run_file_check(check_id, paths, severity)

        else:
            result = CheckResult(check_id, False, f"Unknown check type: {check_type}", severity)

        results.append(result)

    # All error-severity checks must pass
    all_passed = all(r.passed for r in results if r.severity == "error")
    return all_passed, results


def run_feature_gate(
    feature_id: str,
    feature_type: str,
    project_root: Path,
    commands: list[str],
    expected_files: list[Path] | None = None,
) -> tuple[bool, list[CheckResult]]:
    """Run verification after scaffolding a single feature."""
    results: list[CheckResult] = []

    # Run adapter build/test commands
    for i, cmd in enumerate(commands):
        check_id = f"SF-{feature_type.upper()}-{i+1:03d}"
        result = run_command(check_id, cmd, project_root)
        results.append(result)

    # Check expected files exist
    if expected_files:
        check_id = f"SF-{feature_type.upper()}-FILES"
        result = run_file_check(check_id, expected_files)
        results.append(result)

    all_passed = all(r.passed for r in results if r.severity == "error")
    return all_passed, results


def run_invariants(
    project_root: Path,
    invariants: list[dict[str, Any]],
) -> tuple[bool, list[CheckResult]]:
    """Run adapter architecture invariants."""
    results: list[CheckResult] = []

    for inv in invariants:
        check_id = inv["id"]
        check = inv.get("check", {})
        check_type = check.get("type", "")
        pattern = check.get("pattern", "")
        scope = check.get("scope", ".")
        severity = inv.get("severity", "error")

        if check_type == "grep-absent":
            result = run_grep_absent(check_id, pattern, project_root / scope, severity)
        elif check_type == "grep-present":
            result = run_grep_present(check_id, pattern, project_root / scope, severity)
        elif check_type == "command-succeeds":
            result = run_command(check_id, pattern, project_root, severity)
        elif check_type == "file-exists":
            result = run_artifact_exists(check_id, project_root / pattern, severity)
        elif check_type == "file-absent":
            path = project_root / pattern
            if path.exists():
                result = CheckResult(check_id, False, f"File should not exist: {path}", severity)
            else:
                result = CheckResult(check_id, True, f"File absent as expected: {path}", severity)
        else:
            result = CheckResult(check_id, False, f"Unknown invariant check type: {check_type}", severity)

        results.append(result)

    all_passed = all(r.passed for r in results if r.severity == "error")
    return all_passed, results
