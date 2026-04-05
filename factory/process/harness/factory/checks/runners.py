"""Individual check runners implementing the verification contract check types."""

from __future__ import annotations

import json
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass
class CheckResult:
    check_id: str
    passed: bool
    message: str
    severity: str = "error"

    def to_dict(self) -> dict:
        return {
            "id": self.check_id,
            "passed": self.passed,
            "message": self.message,
            "severity": self.severity,
        }


def run_schema_validation(
    check_id: str,
    artifact_path: Path,
    schema_path: Path,
    severity: str = "error",
) -> CheckResult:
    """Validate a JSON artifact against a JSON Schema."""
    try:
        import jsonschema

        with open(artifact_path) as f:
            data = json.load(f)
        with open(schema_path) as f:
            schema = json.load(f)

        jsonschema.validate(data, schema)
        return CheckResult(check_id, True, "Schema validation passed", severity)
    except FileNotFoundError as e:
        return CheckResult(check_id, False, f"File not found: {e}", severity)
    except json.JSONDecodeError as e:
        return CheckResult(check_id, False, f"Invalid JSON: {e}", severity)
    except ImportError:
        # Fallback: just check it's valid JSON
        try:
            with open(artifact_path) as f:
                json.load(f)
            return CheckResult(check_id, True, "JSON valid (jsonschema not installed, schema not checked)", "warning")
        except Exception as e:
            return CheckResult(check_id, False, f"Invalid JSON: {e}", severity)
    except Exception as e:
        return CheckResult(check_id, False, f"Validation failed: {e}", severity)


def run_artifact_exists(
    check_id: str,
    path: Path,
    severity: str = "error",
) -> CheckResult:
    """Check that a file exists and is non-empty."""
    if not path.exists():
        return CheckResult(check_id, False, f"File not found: {path}", severity)
    if path.stat().st_size == 0:
        return CheckResult(check_id, False, f"File is empty: {path}", severity)
    return CheckResult(check_id, True, f"File exists: {path}", severity)


def run_grep_absent(
    check_id: str,
    pattern: str,
    scope: Path,
    severity: str = "error",
    excludes: list[str] | None = None,
) -> CheckResult:
    """Ensure a regex pattern does NOT appear in any files under scope."""
    cmd = ["grep", "-rEn", pattern, str(scope)]
    if excludes:
        for exc in excludes:
            cmd.extend(["--exclude", exc])

    result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)

    if result.returncode == 0:
        # Matches found — this is a failure
        matches = result.stdout.strip().split("\n")[:5]  # first 5 matches
        return CheckResult(
            check_id,
            False,
            f"Pattern found ({len(result.stdout.strip().split(chr(10)))} matches): {'; '.join(matches)}",
            severity,
        )
    # No matches — pass
    return CheckResult(check_id, True, f"Pattern not found in {scope}", severity)


def run_grep_present(
    check_id: str,
    pattern: str,
    scope: Path,
    severity: str = "error",
) -> CheckResult:
    """Ensure a regex pattern DOES appear in at least one file under scope."""
    result = subprocess.run(
        ["grep", "-rEl", pattern, str(scope)],
        capture_output=True,
        text=True,
        timeout=30,
    )

    if result.returncode == 0 and result.stdout.strip():
        files = result.stdout.strip().split("\n")
        return CheckResult(check_id, True, f"Pattern found in {len(files)} file(s)", severity)
    return CheckResult(check_id, False, f"Pattern not found in {scope}", severity)


def run_command(
    check_id: str,
    command: str,
    cwd: Path,
    severity: str = "error",
    timeout: int = 120,
) -> CheckResult:
    """Run a shell command and check exit code."""
    try:
        result = subprocess.run(
            command,
            shell=True,
            cwd=str(cwd),
            capture_output=True,
            text=True,
            timeout=timeout,
        )

        if result.returncode == 0:
            return CheckResult(check_id, True, f"Command succeeded: {command}", severity)

        # Capture tail of output for error feedback
        output = (result.stdout + result.stderr).strip()
        if len(output) > 2000:
            output = "..." + output[-2000:]
        return CheckResult(
            check_id,
            False,
            f"Command failed (exit {result.returncode}): {output}",
            severity,
        )
    except subprocess.TimeoutExpired:
        return CheckResult(check_id, False, f"Command timed out after {timeout}s: {command}", severity)
    except Exception as e:
        return CheckResult(check_id, False, f"Command error: {e}", severity)


def run_file_check(
    check_id: str,
    paths: list[Path],
    severity: str = "error",
) -> CheckResult:
    """Check that all expected files exist."""
    missing = [p for p in paths if not p.exists()]
    if missing:
        return CheckResult(
            check_id,
            False,
            f"Missing files: {', '.join(str(p) for p in missing)}",
            severity,
        )
    return CheckResult(check_id, True, f"All {len(paths)} files exist", severity)


def run_cross_reference(
    check_id: str,
    source_ids: list[str],
    target_ids: set[str],
    description: str,
    severity: str = "error",
) -> CheckResult:
    """Check that all source IDs appear in the target set."""
    missing = [sid for sid in source_ids if sid not in target_ids]
    if missing:
        return CheckResult(
            check_id,
            False,
            f"{description}: missing {len(missing)} — {', '.join(missing[:5])}",
            severity,
        )
    return CheckResult(
        check_id,
        True,
        f"{description}: all {len(source_ids)} IDs found",
        severity,
    )
