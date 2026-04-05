"""Tests for pre-flight checker using real Elucid examples."""

from pathlib import Path

import pytest

from elucid.preflight import run_preflight


# Use the real examples from the elucid repo
REPO_ROOT = Path(__file__).parent.parent.parent.parent
BUILD_SPEC = REPO_ROOT / "contract" / "examples" / "cfs-womens-shelter.build-spec.yaml"
ADAPTER_DIR = REPO_ROOT / "adapters" / "aim-vue-node"


@pytest.fixture
def build_spec() -> Path:
    return BUILD_SPEC


@pytest.fixture
def adapter_dir() -> Path:
    return ADAPTER_DIR


def test_preflight_real_examples(build_spec: Path, adapter_dir: Path):
    """Run preflight against the actual CFS Build Spec and AIM adapter."""
    results = run_preflight(build_spec, adapter_dir)

    errors = [r for r in results if not r.passed and r.severity == "error"]
    assert len(errors) == 0, f"Pre-flight errors: {[r.message for r in errors]}"

    # Should have successful checks for spec, manifest, variant, auth
    passed_ids = {r.check_id for r in results if r.passed}
    assert "PF-001" in passed_ids  # Build Spec valid
    assert "PF-002" in passed_ids  # Adapter manifest valid
    assert "PF-003" in passed_ids  # Capabilities match


def test_preflight_missing_build_spec(adapter_dir: Path, tmp_path: Path):
    """Pre-flight fails if Build Spec doesn't exist."""
    results = run_preflight(tmp_path / "nonexistent.yaml", adapter_dir)
    assert any(not r.passed and r.check_id == "PF-001" for r in results)


def test_preflight_missing_adapter(build_spec: Path, tmp_path: Path):
    """Pre-flight fails if adapter manifest doesn't exist."""
    results = run_preflight(build_spec, tmp_path / "no-adapter")
    assert any(not r.passed and r.check_id == "PF-002" for r in results)


def test_preflight_capability_mismatch(build_spec: Path, adapter_dir: Path, tmp_path: Path):
    """Pre-flight catches capability mismatches."""
    # Create a fake adapter that lacks dual_stack
    fake_adapter = tmp_path / "fake-adapter"
    fake_adapter.mkdir()
    (fake_adapter / "manifest.yaml").write_text(
        "adapter:\n  name: fake\n  version: '1.0'\ncapabilities:\n  dual_stack: false\n  single_stack: true\nsupported_auth: []\n"
    )

    results = run_preflight(build_spec, fake_adapter)
    cap_failures = [r for r in results if not r.passed and r.check_id == "PF-003"]
    # Should fail: dual variant required but adapter lacks dual_stack
    assert len(cap_failures) > 0
    assert any("dual" in r.message for r in cap_failures)
