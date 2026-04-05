"""Elucid verification harness CLI.

Usage:
    python -m elucid preflight --build-spec PATH --adapter PATH [--artifacts PATH]
    python -m elucid gate --stage STAGE_ID --project PATH --checks-file PATH
    python -m elucid feature --feature-id ID --type api|ui --project PATH --commands CMD [CMD...]
    python -m elucid invariants --project PATH --adapter PATH
    python -m elucid state init --adapter NAME --version VER --build-spec PATH --state PATH
    python -m elucid state show --state PATH
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import yaml


def cmd_preflight(args: argparse.Namespace) -> int:
    from .preflight import run_preflight

    results = run_preflight(
        Path(args.build_spec),
        Path(args.adapter),
        Path(args.artifacts) if args.artifacts else None,
    )
    return _report(results)


def cmd_gate(args: argparse.Namespace) -> int:
    from .gate import run_stage_gate

    with open(args.checks_file) as f:
        checks = yaml.safe_load(f)

    passed, results = run_stage_gate(args.stage, Path(args.project), checks)
    return _report(results)


def cmd_feature(args: argparse.Namespace) -> int:
    from .gate import run_feature_gate

    expected = [Path(f) for f in args.files] if args.files else None
    passed, results = run_feature_gate(
        args.feature_id, args.type, Path(args.project), args.commands, expected
    )
    return _report(results)


def cmd_invariants(args: argparse.Namespace) -> int:
    from .gate import run_invariants

    manifest_path = Path(args.adapter) / "manifest.yaml"
    with open(manifest_path) as f:
        manifest = yaml.safe_load(f)

    invariants = manifest.get("validation", {}).get("invariants", [])
    passed, results = run_invariants(Path(args.project), invariants)
    return _report(results)


def cmd_state_init(args: argparse.Namespace) -> int:
    from .preflight import hash_file
    from .state import init_state

    state = init_state(
        args.adapter_name,
        args.adapter_version,
        args.build_spec,
        hash_file(Path(args.build_spec)),
        Path(args.state),
    )
    print(json.dumps({"status": "initialized", "pipeline_id": state["pipeline"]["id"]}))
    return 0


def cmd_state_show(args: argparse.Namespace) -> int:
    from .state import load_state

    state = load_state(Path(args.state))
    if not state:
        print("No pipeline state found.", file=sys.stderr)
        return 1

    p = state["pipeline"]
    scaff = state["scaffolding"]
    api_done = len(scaff["api"]["operations_completed"])
    api_fail = len(scaff["api"]["operations_failed"])
    api_remain = len(scaff["api"]["operations_remaining"])
    ui_done = len(scaff["ui"]["pages_completed"])
    ui_fail = len(scaff["ui"]["pages_failed"])
    ui_remain = len(scaff["ui"]["pages_remaining"])

    print(f"Pipeline: {p['id']}")
    print(f"Status:   {p['status']}")
    print(f"Adapter:  {p['adapter']['name']} v{p['adapter']['version']}")
    print(f"Started:  {p['started_at']}")
    print(f"Updated:  {p['updated_at']}")
    print()

    stages = state.get("stages", {})
    if stages:
        print("Stages:")
        for sid, s in stages.items():
            gate_status = ""
            if "gate" in s:
                gate_status = " [PASS]" if s["gate"].get("passed") else " [FAIL]"
            print(f"  {sid}: {s.get('status', '?')}{gate_status}")
        print()

    print(f"API scaffolding:  {api_done} done, {api_fail} failed, {api_remain} remaining")
    print(f"UI scaffolding:   {ui_done} done, {ui_fail} failed, {ui_remain} remaining")
    print(f"Configure:        {scaff['configure']['status']}")
    print(f"Trim:             {scaff['trim']['status']}")

    errors = state.get("errors", [])
    if errors:
        print(f"\nErrors: {len(errors)}")
        for e in errors[-3:]:
            print(f"  [{e['error_type']}] {e['stage']}: {e['message'][:80]}")

    return 0


def _report(results: list) -> int:
    """Print results and return exit code."""
    passed = sum(1 for r in results if r.passed)
    failed = sum(1 for r in results if not r.passed and r.severity == "error")
    warned = sum(1 for r in results if not r.passed and r.severity == "warning")

    for r in results:
        icon = "PASS" if r.passed else ("FAIL" if r.severity == "error" else "WARN")
        print(f"  [{icon}] {r.check_id}: {r.message}")

    print(f"\n{passed} passed, {failed} failed, {warned} warnings")

    if any(not r.passed and r.severity == "error" for r in results):
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(prog="elucid", description="Elucid verification harness")
    sub = parser.add_subparsers(dest="command")

    # preflight
    pf = sub.add_parser("preflight", help="Run pre-flight checks")
    pf.add_argument("--build-spec", required=True, help="Path to build-spec.yaml")
    pf.add_argument("--adapter", required=True, help="Path to adapter directory")
    pf.add_argument("--artifacts", help="Path to business artifacts directory")

    # gate
    g = sub.add_parser("gate", help="Run stage gate checks")
    g.add_argument("--stage", required=True, help="Stage ID")
    g.add_argument("--project", required=True, help="Project root path")
    g.add_argument("--checks-file", required=True, help="YAML file with checks array")

    # feature
    feat = sub.add_parser("feature", help="Run per-feature verification")
    feat.add_argument("--feature-id", required=True, help="Feature ID (operation or page)")
    feat.add_argument("--type", required=True, choices=["api", "ui"], help="Feature type")
    feat.add_argument("--project", required=True, help="Project root path")
    feat.add_argument("--commands", nargs="+", required=True, help="Verify commands")
    feat.add_argument("--files", nargs="*", help="Expected files to check")

    # invariants
    inv = sub.add_parser("invariants", help="Run adapter architecture invariants")
    inv.add_argument("--project", required=True, help="Project root path")
    inv.add_argument("--adapter", required=True, help="Path to adapter directory")

    # state
    st = sub.add_parser("state", help="Pipeline state management")
    st_sub = st.add_subparsers(dest="state_command")

    st_init = st_sub.add_parser("init", help="Initialize pipeline state")
    st_init.add_argument("--adapter-name", required=True)
    st_init.add_argument("--adapter-version", required=True)
    st_init.add_argument("--build-spec", required=True)
    st_init.add_argument("--state", required=True, help="Path to write pipeline-state.json")

    st_show = st_sub.add_parser("show", help="Show pipeline state summary")
    st_show.add_argument("--state", required=True, help="Path to pipeline-state.json")

    args = parser.parse_args()

    if args.command == "preflight":
        return cmd_preflight(args)
    elif args.command == "gate":
        return cmd_gate(args)
    elif args.command == "feature":
        return cmd_feature(args)
    elif args.command == "invariants":
        return cmd_invariants(args)
    elif args.command == "state":
        if args.state_command == "init":
            return cmd_state_init(args)
        elif args.state_command == "show":
            return cmd_state_show(args)
    else:
        parser.print_help()
        return 1

    return 0


if __name__ == "__main__":
    sys.exit(main())
