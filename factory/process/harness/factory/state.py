"""Pipeline state manager — durable execution state for crash recovery."""

from __future__ import annotations

import json
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


def init_state(
    adapter_name: str,
    adapter_version: str,
    build_spec_path: str,
    build_spec_hash: str,
    state_path: Path,
) -> dict[str, Any]:
    """Create a fresh pipeline-state.json."""
    state: dict[str, Any] = {
        "pipeline": {
            "id": str(uuid.uuid4()),
            "factory_version": "0.1.0",
            "started_at": _now(),
            "updated_at": _now(),
            "completed_at": None,
            "status": "running",
            "adapter": {"name": adapter_name, "version": adapter_version},
            "build_spec": {"path": str(build_spec_path), "hash": build_spec_hash},
        },
        "stages": {},
        "scaffolding": {
            "data": {"status": "pending", "entities_completed": [], "entities_remaining": [], "entities_failed": []},
            "api": {"status": "pending", "operations_completed": [], "operations_remaining": [], "operations_failed": []},
            "ui": {"status": "pending", "pages_completed": [], "pages_remaining": [], "pages_failed": []},
            "configure": {"status": "pending", "steps_completed": []},
            "trim": {"status": "pending", "files_removed": []},
        },
        "verification": {"last_full_run": None, "consistency": []},
        "errors": [],
        "audit": [],
    }
    _write(state, state_path)
    return state


def load_state(state_path: Path) -> dict[str, Any] | None:
    """Load existing pipeline state, or None if not found."""
    if not state_path.exists():
        return None
    with open(state_path) as f:
        return json.load(f)


def save_state(state: dict[str, Any], state_path: Path) -> None:
    """Persist state to disk."""
    state["pipeline"]["updated_at"] = _now()
    _write(state, state_path)


def update_stage(
    state: dict[str, Any],
    stage_id: str,
    *,
    status: str,
    artifacts: list[dict[str, str]] | None = None,
    gate: dict[str, Any] | None = None,
) -> None:
    """Update a stage's status, artifacts, and gate results."""
    stage = state["stages"].setdefault(stage_id, {})
    stage["status"] = status
    if status == "in_progress" and "started_at" not in stage:
        stage["started_at"] = _now()
    if status in ("completed", "failed"):
        stage["completed_at"] = _now()
    if artifacts is not None:
        stage["artifacts"] = artifacts
    if gate is not None:
        stage["gate"] = gate


def record_operation_completed(
    state: dict[str, Any],
    operation_id: str,
    files_created: list[str],
) -> None:
    """Mark an API operation as successfully scaffolded."""
    api = state["scaffolding"]["api"]
    api["operations_completed"].append({
        "operation_id": operation_id,
        "files_created": files_created,
        "verified_at": _now(),
    })
    if operation_id in api["operations_remaining"]:
        api["operations_remaining"].remove(operation_id)


def record_operation_failed(
    state: dict[str, Any],
    operation_id: str,
    error: str,
    retries: int,
    max_retries: int = 3,
) -> None:
    """Mark an API operation as failed after max retries."""
    api = state["scaffolding"]["api"]
    api["operations_failed"].append({
        "operation_id": operation_id,
        "error": error,
        "retries": retries,
        "max_retries": max_retries,
    })
    if operation_id in api["operations_remaining"]:
        api["operations_remaining"].remove(operation_id)


def record_page_completed(
    state: dict[str, Any],
    page_id: str,
    files_created: list[str],
) -> None:
    """Mark a UI page as successfully scaffolded."""
    ui = state["scaffolding"]["ui"]
    ui["pages_completed"].append({
        "page_id": page_id,
        "files_created": files_created,
        "verified_at": _now(),
    })
    if page_id in ui["pages_remaining"]:
        ui["pages_remaining"].remove(page_id)


def record_page_failed(
    state: dict[str, Any],
    page_id: str,
    error: str,
    retries: int,
    max_retries: int = 3,
) -> None:
    """Mark a UI page as failed after max retries."""
    ui = state["scaffolding"]["ui"]
    ui["pages_failed"].append({
        "page_id": page_id,
        "error": error,
        "retries": retries,
        "max_retries": max_retries,
    })
    if page_id in ui["pages_remaining"]:
        ui["pages_remaining"].remove(page_id)


def record_error(
    state: dict[str, Any],
    stage: str,
    error_type: str,
    message: str,
    feature: str | None = None,
    retry_number: int = 0,
) -> None:
    """Append to the persistent error log."""
    state["errors"].append({
        "timestamp": _now(),
        "stage": stage,
        "feature": feature,
        "error_type": error_type,
        "message": message,
        "retry_number": retry_number,
        "resolved": False,
    })


def record_audit(
    state: dict[str, Any],
    event: str,
    stage: str,
    details: str = "",
) -> None:
    """Record a human confirmation or override."""
    state["audit"].append({
        "timestamp": _now(),
        "event": event,
        "stage": stage,
        "details": details,
    })


def complete_pipeline(state: dict[str, Any], success: bool) -> None:
    """Mark the pipeline as completed or failed."""
    state["pipeline"]["status"] = "completed" if success else "failed"
    state["pipeline"]["completed_at"] = _now()


def is_operation_done(state: dict[str, Any], operation_id: str) -> bool:
    """Check if an operation has already been scaffolded (for resume)."""
    completed = {op["operation_id"] for op in state["scaffolding"]["api"]["operations_completed"]}
    return operation_id in completed


def is_page_done(state: dict[str, Any], page_id: str) -> bool:
    """Check if a page has already been scaffolded (for resume)."""
    completed = {p["page_id"] for p in state["scaffolding"]["ui"]["pages_completed"]}
    return page_id in completed


def _write(state: dict[str, Any], path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w") as f:
        json.dump(state, f, indent=2)
