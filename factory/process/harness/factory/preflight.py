"""Pre-flight checker — validates inputs before pipeline starts."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

import yaml

from .checks.runners import CheckResult


def load_yaml(path: Path) -> dict[str, Any]:
    """Load a YAML file."""
    with open(path) as f:
        return yaml.safe_load(f)


def load_json(path: Path) -> dict[str, Any]:
    """Load a JSON file."""
    with open(path) as f:
        return json.load(f)


def hash_file(path: Path) -> str:
    """SHA-256 hash of a file."""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def run_preflight(
    build_spec_path: Path,
    adapter_path: Path,
    artifacts_path: Path | None = None,
) -> list[CheckResult]:
    """Run all pre-flight checks. Returns list of CheckResults."""
    results: list[CheckResult] = []

    # PF-001: Build Spec exists and is valid YAML
    if not build_spec_path.exists():
        results.append(CheckResult("PF-001", False, f"Build Spec not found: {build_spec_path}"))
        return results  # Can't continue without Build Spec

    try:
        build_spec = load_yaml(build_spec_path)
        results.append(CheckResult("PF-001", True, "Build Spec is valid YAML"))
    except Exception as e:
        results.append(CheckResult("PF-001", False, f"Build Spec invalid: {e}"))
        return results

    # PF-002: Adapter manifest exists and is valid YAML
    manifest_path = adapter_path / "manifest.yaml"
    if not manifest_path.exists():
        results.append(CheckResult("PF-002", False, f"Adapter manifest not found: {manifest_path}"))
        return results

    try:
        manifest = load_yaml(manifest_path)
        results.append(CheckResult("PF-002", True, "Adapter manifest is valid YAML"))
    except Exception as e:
        results.append(CheckResult("PF-002", False, f"Adapter manifest invalid: {e}"))
        return results

    # PF-003: Capability checks
    cap_results = check_capabilities(build_spec, manifest)
    results.extend(cap_results)

    # PF-004: Scaffold exists
    scaffold_source = manifest.get("scaffold", {}).get("source", "scaffold/")
    scaffold_path = adapter_path / scaffold_source
    if scaffold_path.exists():
        results.append(CheckResult("PF-004", True, f"Scaffold directory exists: {scaffold_path}"))
    else:
        results.append(CheckResult("PF-004", False, f"Scaffold not found: {scaffold_path}", "warning"))

    # PF-005: Required agent prompts exist
    agents = manifest.get("agents", {})
    required_agents = ["api_scaffolder", "ui_scaffolder", "data_scaffolder", "configurer", "trimmer"]
    for agent_key in required_agents:
        agent_file = agents.get(agent_key)
        if agent_file:
            agent_path = adapter_path / agent_file
            if agent_path.exists():
                results.append(CheckResult("PF-005", True, f"Agent exists: {agent_key}"))
            else:
                results.append(CheckResult("PF-005", False, f"Agent not found: {agent_path}"))
        else:
            results.append(CheckResult("PF-005", False, f"Agent not declared: {agent_key}"))

    # PF-006: Pattern files exist
    patterns = manifest.get("patterns", {})
    for category, entries in patterns.items():
        if isinstance(entries, dict):
            for name, path_str in entries.items():
                if path_str:
                    pattern_path = adapter_path / path_str
                    if not pattern_path.exists():
                        results.append(CheckResult(
                            "PF-006", False,
                            f"Pattern not found: {category}.{name} → {pattern_path}",
                            "warning",
                        ))

    # PF-007: Business artifacts exist (if path provided)
    if artifacts_path:
        if artifacts_path.exists() and any(artifacts_path.iterdir()):
            results.append(CheckResult("PF-007", True, f"Business artifacts found in {artifacts_path}"))
        else:
            results.append(CheckResult("PF-007", False, f"No business artifacts in {artifacts_path}"))

    return results


def check_capabilities(
    build_spec: dict[str, Any],
    manifest: dict[str, Any],
) -> list[CheckResult]:
    """Check that the adapter's capabilities satisfy the Build Spec requirements."""
    results: list[CheckResult] = []
    caps = manifest.get("capabilities", {})

    project = build_spec.get("project", {})
    auth = build_spec.get("auth", {})

    # Variant check
    variant = project.get("variant", "single-public")
    if variant == "dual" and not caps.get("dual_stack"):
        results.append(CheckResult(
            "PF-003", False,
            f"Build Spec requires dual variant but adapter lacks dual_stack capability",
        ))
    elif variant.startswith("single") and not caps.get("single_stack"):
        results.append(CheckResult(
            "PF-003", False,
            f"Build Spec requires {variant} but adapter lacks single_stack capability",
        ))
    else:
        results.append(CheckResult("PF-003", True, f"Variant '{variant}' supported"))

    # Auth method checks
    supported_methods = {a["method"] for a in manifest.get("supported_auth", [])}
    for audience_name, audience_cfg in auth.get("audiences", {}).items():
        method = audience_cfg.get("method", "")
        if method and method != "mock" and method not in supported_methods:
            results.append(CheckResult(
                "PF-003", False,
                f"Auth method '{method}' (audience '{audience_name}') not in adapter supported_auth",
            ))
        elif method and method != "mock":
            results.append(CheckResult(
                "PF-003", True,
                f"Auth method '{method}' (audience '{audience_name}') supported",
            ))

    # Integration capability checks
    for integration in build_spec.get("integrations", []):
        int_type = integration.get("type", "")
        if int_type == "file-storage" and not caps.get("file_uploads"):
            results.append(CheckResult(
                "PF-003", False,
                f"Integration '{integration.get('name')}' requires file_uploads capability",
            ))
        if int_type == "email" and not caps.get("email_notifications"):
            results.append(CheckResult(
                "PF-003", False,
                f"Integration '{integration.get('name')}' requires email_notifications capability",
            ))

    # Audit check
    if build_spec.get("audit", {}).get("enabled") and not caps.get("audit_logging"):
        results.append(CheckResult(
            "PF-003", False,
            "Build Spec requires audit logging but adapter lacks audit_logging capability",
        ))

    return results
