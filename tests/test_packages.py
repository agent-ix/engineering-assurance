from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

from engineering_assurance.discovery import validate_discovery
from scripts.audit_packages import ROOT_DATA_FILES, member_mismatch, npm_allowlist

ROOT = Path(__file__).parents[1]


def test_wheel_contract_declares_canonical_and_host_payloads() -> None:
    """Trace: FR-003-AC-1, TC-014."""
    setup = (ROOT / "setup.cfg").read_text()
    assert "skills/assurance-onboarding/SKILL.md" in setup
    assert "pilots/assurance-workflows/scripts" in setup
    for path in ROOT_DATA_FILES:
        assert (ROOT / path).is_file()


def test_npm_contract_declares_canonical_and_host_payloads() -> None:
    """Trace: FR-003-AC-2, TC-015."""
    npm = json.loads((ROOT / "package.json").read_text())
    assert "engineering_assurance/skills/" in npm["files"]
    assert "pilots/assurance-workflows/" in npm["files"]
    assert "engineering_assurance/INSTALL.md" in npm_allowlist()


def test_install_docs_separate_module_plugin_and_sources() -> None:
    """Trace: FR-003-AC-6, TC-019."""
    text = (ROOT / "engineering_assurance" / "INSTALL.md").read_text()
    headings = [
        "## Local-source module installation",
        "## Repository-source module installation",
        "## Local-source agent-plugin installation",
        "## Repository-source agent-plugin installation",
    ]
    positions = [text.index(heading) for heading in headings]
    assert positions == sorted(positions)
    canonical = text.index("engineering_assurance/skills/assurance-onboarding")
    compatibility = text.index("pilots/assurance-workflows")
    assert canonical < compatibility


def test_repository_source_install_preserves_discovery(tmp_path: Path) -> None:
    """Trace: FR-003-AC-4, TC-017."""
    target = tmp_path / "installed"
    completed = subprocess.run(
        [
            sys.executable,
            "-m",
            "pip",
            "install",
            "--no-index",
            "--no-deps",
            "--no-build-isolation",
            "--target",
            str(target),
            ".",
        ],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr
    validate_discovery(target)


def test_package_member_mismatch_detects_extra_and_missing() -> None:
    """Trace: FR-003-AC-5, TC-018."""
    extra, missing = member_mismatch(
        {"manifest.yaml", "unexpected.txt"},
        {"manifest.yaml", "schemas/profile.json"},
    )
    assert extra == ["unexpected.txt"]
    assert missing == ["schemas/profile.json"]


def test_package_contract_retains_prior_module_root_members() -> None:
    """Trace: NFR-003, TC-040."""
    allowed = npm_allowlist()
    assert {"manifest.yaml", "engineering_assurance/manifest.yaml"} <= allowed
    assert any(name.startswith("schemas/") for name in allowed)
    assert any(name.startswith("skeletons/") for name in allowed)


def test_canonical_docs_precede_compatible_pilot_path() -> None:
    """Trace: FR-007-AC-3, TC-037."""
    text = (ROOT / "engineering_assurance" / "INSTALL.md").read_text()
    assert text.index("engineering_assurance/skills/assurance-onboarding") < text.index(
        "pilots/assurance-workflows"
    )
