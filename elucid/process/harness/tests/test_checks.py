"""Tests for check runners."""

import json
from pathlib import Path

import pytest

from elucid.checks.runners import (
    run_artifact_exists,
    run_command,
    run_cross_reference,
    run_file_check,
    run_grep_absent,
    run_grep_present,
    run_schema_validation,
)


@pytest.fixture
def fixtures(tmp_path: Path) -> Path:
    """Create test fixture files."""
    # Valid JSON
    (tmp_path / "valid.json").write_text('{"entities": [{"name": "Foo"}]}')
    # Empty file
    (tmp_path / "empty.json").write_text("")
    # Source files with patterns
    src = tmp_path / "src"
    src.mkdir()
    (src / "good.ts").write_text("import { pool } from '../db.js'\nconst x = 1\n")
    (src / "bad.ts").write_text("import jwt from 'jsonwebtoken'\n")
    return tmp_path


def test_artifact_exists_pass(fixtures: Path):
    result = run_artifact_exists("T-001", fixtures / "valid.json")
    assert result.passed


def test_artifact_exists_missing(fixtures: Path):
    result = run_artifact_exists("T-002", fixtures / "missing.json")
    assert not result.passed
    assert "not found" in result.message


def test_artifact_exists_empty(fixtures: Path):
    result = run_artifact_exists("T-003", fixtures / "empty.json")
    assert not result.passed
    assert "empty" in result.message


def test_grep_absent_pass(fixtures: Path):
    result = run_grep_absent("T-004", "jsonwebtoken", fixtures / "src" / "good.ts")
    assert result.passed


def test_grep_absent_fail(fixtures: Path):
    result = run_grep_absent("T-005", "jsonwebtoken", fixtures / "src")
    assert not result.passed
    assert "Pattern found" in result.message


def test_grep_present_pass(fixtures: Path):
    result = run_grep_present("T-006", "import.*pool", fixtures / "src")
    assert result.passed


def test_grep_present_fail(fixtures: Path):
    result = run_grep_present("T-007", "nonexistent_pattern_xyz", fixtures / "src")
    assert not result.passed


def test_command_succeeds(fixtures: Path):
    result = run_command("T-008", "echo hello", fixtures)
    assert result.passed


def test_command_fails(fixtures: Path):
    result = run_command("T-009", "false", fixtures)
    assert not result.passed


def test_command_timeout(fixtures: Path):
    result = run_command("T-010", "sleep 10", fixtures, timeout=1)
    assert not result.passed
    assert "timed out" in result.message


def test_file_check_pass(fixtures: Path):
    result = run_file_check("T-011", [fixtures / "valid.json", fixtures / "src" / "good.ts"])
    assert result.passed


def test_file_check_fail(fixtures: Path):
    result = run_file_check("T-012", [fixtures / "valid.json", fixtures / "missing.json"])
    assert not result.passed
    assert "Missing" in result.message


def test_cross_reference_pass():
    result = run_cross_reference(
        "T-013",
        ["UC-001", "UC-002"],
        {"UC-001", "UC-002", "UC-003"},
        "UC coverage",
    )
    assert result.passed


def test_cross_reference_fail():
    result = run_cross_reference(
        "T-014",
        ["UC-001", "UC-002", "UC-999"],
        {"UC-001", "UC-002"},
        "UC coverage",
    )
    assert not result.passed
    assert "UC-999" in result.message


def test_schema_validation_pass(fixtures: Path):
    schema = {
        "type": "object",
        "required": ["entities"],
        "properties": {"entities": {"type": "array"}},
    }
    schema_path = fixtures / "schema.json"
    schema_path.write_text(json.dumps(schema))

    result = run_schema_validation("T-015", fixtures / "valid.json", schema_path)
    assert result.passed


def test_schema_validation_fail(fixtures: Path):
    schema = {
        "type": "object",
        "required": ["missing_field"],
        "properties": {"missing_field": {"type": "string"}},
    }
    schema_path = fixtures / "strict_schema.json"
    schema_path.write_text(json.dumps(schema))

    result = run_schema_validation("T-016", fixtures / "valid.json", schema_path)
    assert not result.passed
