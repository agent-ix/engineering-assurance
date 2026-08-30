from __future__ import annotations

import json
from pathlib import Path

from scripts.audit_packages import ROOT_DATA_FILES, npm_allowlist

ROOT = Path(__file__).parents[1]


def test_package_contract_declares_canonical_and_host_payloads() -> None:
    """Trace: FR-003-AC-1..AC-2, TC-014..TC-015; NFR-003, TC-040."""
    setup = (ROOT / "setup.cfg").read_text()
    npm = json.loads((ROOT / "package.json").read_text())
    assert "skills/assurance-onboarding/SKILL.md" in setup
    assert "pilots/assurance-workflows/scripts" in setup
    for path in ROOT_DATA_FILES:
        assert (ROOT / path).is_file()
    assert "engineering_assurance/skills/" in npm["files"]
    assert "pilots/assurance-workflows/" in npm["files"]
    assert "engineering_assurance/INSTALL.md" in npm_allowlist()


def test_install_docs_separate_module_plugin_and_sources() -> None:
    """Trace: FR-003-AC-6, TC-019; FR-007-AC-3, TC-037."""
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
