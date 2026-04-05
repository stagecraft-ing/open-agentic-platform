"""Tests for pipeline state manager."""

import json
from pathlib import Path

import pytest

from elucid.state import (
    complete_pipeline,
    init_state,
    is_operation_done,
    is_page_done,
    load_state,
    record_error,
    record_operation_completed,
    record_operation_failed,
    record_page_completed,
    save_state,
    update_stage,
)


@pytest.fixture
def state_path(tmp_path: Path) -> Path:
    return tmp_path / ".elucid" / "pipeline-state.json"


@pytest.fixture
def state(state_path: Path) -> dict:
    return init_state("test-adapter", "1.0.0", "build-spec.yaml", "abc123", state_path)


def test_init_creates_file(state_path: Path, state: dict):
    assert state_path.exists()
    loaded = json.loads(state_path.read_text())
    assert loaded["pipeline"]["status"] == "running"
    assert loaded["pipeline"]["adapter"]["name"] == "test-adapter"


def test_load_state(state_path: Path, state: dict):
    loaded = load_state(state_path)
    assert loaded is not None
    assert loaded["pipeline"]["id"] == state["pipeline"]["id"]


def test_load_nonexistent():
    assert load_state(Path("/nonexistent/path.json")) is None


def test_save_updates_timestamp(state_path: Path, state: dict):
    original = state["pipeline"]["updated_at"]
    save_state(state, state_path)
    loaded = load_state(state_path)
    assert loaded["pipeline"]["updated_at"] >= original


def test_update_stage(state: dict):
    update_stage(state, "business-requirements", status="in_progress")
    assert state["stages"]["business-requirements"]["status"] == "in_progress"
    assert "started_at" in state["stages"]["business-requirements"]

    update_stage(
        state, "business-requirements",
        status="completed",
        artifacts=[{"path": "requirements/brd.md", "type": "brd"}],
        gate={"passed": True, "checks": [{"id": "S1-001", "passed": True}]},
    )
    assert state["stages"]["business-requirements"]["status"] == "completed"
    assert state["stages"]["business-requirements"]["gate"]["passed"]


def test_record_operation_completed(state: dict):
    state["scaffolding"]["api"]["operations_remaining"] = ["list-orgs", "create-org"]

    record_operation_completed(state, "list-orgs", ["services/org.service.ts"])

    assert len(state["scaffolding"]["api"]["operations_completed"]) == 1
    assert state["scaffolding"]["api"]["operations_completed"][0]["operation_id"] == "list-orgs"
    assert "list-orgs" not in state["scaffolding"]["api"]["operations_remaining"]


def test_record_operation_failed(state: dict):
    state["scaffolding"]["api"]["operations_remaining"] = ["bad-op"]

    record_operation_failed(state, "bad-op", "TypeError: x is undefined", 3)

    assert len(state["scaffolding"]["api"]["operations_failed"]) == 1
    assert state["scaffolding"]["api"]["operations_failed"][0]["retries"] == 3
    assert "bad-op" not in state["scaffolding"]["api"]["operations_remaining"]


def test_record_page_completed(state: dict):
    state["scaffolding"]["ui"]["pages_remaining"] = ["dashboard"]

    record_page_completed(state, "dashboard", ["views/DashboardView.vue"])

    assert len(state["scaffolding"]["ui"]["pages_completed"]) == 1
    assert is_page_done(state, "dashboard")
    assert not is_page_done(state, "other-page")


def test_is_operation_done(state: dict):
    assert not is_operation_done(state, "list-orgs")
    record_operation_completed(state, "list-orgs", ["file.ts"])
    assert is_operation_done(state, "list-orgs")


def test_record_error(state: dict):
    record_error(state, "api-scaffolding", "compile", "TS2345: Argument not assignable", feature="list-orgs")
    assert len(state["errors"]) == 1
    assert state["errors"][0]["error_type"] == "compile"


def test_complete_pipeline_success(state: dict):
    complete_pipeline(state, success=True)
    assert state["pipeline"]["status"] == "completed"
    assert state["pipeline"]["completed_at"] is not None


def test_complete_pipeline_failure(state: dict):
    complete_pipeline(state, success=False)
    assert state["pipeline"]["status"] == "failed"
