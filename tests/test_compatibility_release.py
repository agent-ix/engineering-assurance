# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX

"""The scoped release gate binds accepted pins to shipped artifacts."""

import importlib
import json
import shutil
import sys
from pathlib import Path

import pytest

# The Rust source audit examines scripts/ as text after pytest runs. Keep the
# import below from leaving a binary bytecode artifact in that source tree.
sys.dont_write_bytecode = True
qualify = importlib.import_module("scripts.check_compatibility_release").qualify

ROOT = Path(__file__).resolve().parents[1]
MATRIX = Path("engineering_assurance/compatibility-matrix.json")


def staged_root(tmp_path: Path) -> Path:
    matrix = json.loads((ROOT / MATRIX).read_text())
    paths = [
        MATRIX,
        Path("README.md"),
        Path("Cargo.toml"),
        Path("package.json"),
        Path("setup.cfg"),
        Path("engineering_assurance/manifest.yaml"),
        Path("engineering_assurance/skills/assurance-onboarding/SKILL.md"),
        Path(".claude-plugin/plugin.json"),
        Path(".codex-plugin/plugin.json"),
        Path(".github/plugin/plugin.json"),
    ]
    ea = next(
        component
        for component in matrix["components"]
        if component["name"] == "engineering-assurance"
    )
    paths.extend(Path(artifact["path"]) for artifact in ea["artifacts"])
    for path in paths:
        (tmp_path / path).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / path, tmp_path / path)
    return tmp_path


def test_exact_accepted_matrix_and_release_surfaces_pass(tmp_path: Path) -> None:
    """Trace: FR-012-AC-1, FR-012-AC-4, TC-079, TC-082."""
    qualify(staged_root(tmp_path))


def test_changed_component_pin_refuses_prior_human_acceptance(tmp_path: Path) -> None:
    """Trace: FR-012-AC-1, FR-012-AC-4, TC-079, TC-082."""
    root = staged_root(tmp_path)
    matrix = json.loads((root / MATRIX).read_text())
    matrix["components"][1]["version"] = "0.24.2"
    (root / MATRIX).write_text(json.dumps(matrix, indent=2) + "\n")
    with pytest.raises(ValueError, match="exact human-accepted candidate"):
        qualify(root)


def test_changed_schema_bytes_refuse_release(tmp_path: Path) -> None:
    """Trace: FR-012-AC-1, TC-079."""
    root = staged_root(tmp_path)
    schema = root / "engineering_assurance/schemas/measurement-plan-frontmatter.schema.json"
    schema.write_bytes(schema.read_bytes() + b"\n")
    with pytest.raises(ValueError, match="schema bytes differ"):
        qualify(root)


def test_plugin_version_and_install_reference_must_match_tag(tmp_path: Path) -> None:
    """Trace: FR-007-AC-3, TC-037; FR-012-AC-1, TC-079."""
    root = staged_root(tmp_path)
    plugin = root / ".codex-plugin/plugin.json"
    data = json.loads(plugin.read_text())
    data["version"] = "0.4.0"
    plugin.write_text(json.dumps(data) + "\n")
    with pytest.raises(ValueError, match="plugin version differs"):
        qualify(root)

    shutil.copy2(ROOT / ".codex-plugin/plugin.json", plugin)
    readme = root / "README.md"
    readme.write_text(readme.read_text().replace("--ref v0.4.1", "--ref v0.4.0"))
    with pytest.raises(ValueError, match="install documentation lacks"):
        qualify(root)
