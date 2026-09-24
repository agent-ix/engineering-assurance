# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX

"""Qualify the accepted v0.4.1 compatibility and plugin release boundary."""

from __future__ import annotations

import configparser
import hashlib
import json
import sys
import tomllib
from pathlib import Path

ACCEPTED_CANDIDATE_SHA256 = (
    "26fb2ea02d8a9bc3cb97d0e43914be00dbcc57eee927267cba6c3ab139a22cc3"
)
PENDING_NOTE = (
    "The candidate pins Quire 0.33.0, Quoin 0.24.1, ix-flow 0.2.3, and "
    "Engineering Assurance 0.4.1. An agent may prepare the matrix; only a human "
    "may accept it after reviewing the complete candidate and its gates."
)
RELEASE_DESCRIPTION = (
    "git tag v0.4.1 \u2014 pre-stabilization source distribution; "
    "npm publication is refused by design"
)


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def qualify(root: Path) -> None:
    """Refuse drift from the accepted matrix and its shipped release surfaces."""
    matrix = json.loads(
        (root / "engineering_assurance/compatibility-matrix.json").read_text()
    )
    acceptance = matrix["accepted"]
    _require(acceptance["state"] == "accepted", "matrix is not accepted")
    _require(
        acceptance["accepted_by"] == "Peter Krenesky"
        and acceptance["accepted_at"] == "2026-09-23",
        "matrix acceptance attribution differs from the human decision",
    )
    _require(
        ACCEPTED_CANDIDATE_SHA256 in acceptance["note"],
        "acceptance does not identify the exact reviewed candidate",
    )
    components = {item["name"]: item for item in matrix["components"]}
    _require(len(components) == 4, "matrix must pin exactly four components")
    ea = components["engineering-assurance"]
    _require(ea["released"] is True, "accepted EA pin is not released")
    _require(ea["release"] == RELEASE_DESCRIPTION, "EA release tag differs")

    # Reconstruct the exact bytes Peter accepted before decision metadata and
    # the self-release state were transcribed. This binds every component pin,
    # schema digest, and other matrix field to the named decision.
    candidate = json.loads(json.dumps(matrix))
    candidate["accepted"] = {
        "state": "pending_human_acceptance",
        "accepted_by": None,
        "accepted_at": None,
        "note": PENDING_NOTE,
    }
    candidate_ea = next(
        item for item in candidate["components"] if item["name"] == "engineering-assurance"
    )
    candidate_ea["released"] = False
    candidate_ea["release"] = "planned " + RELEASE_DESCRIPTION
    candidate_bytes = (json.dumps(candidate, ensure_ascii=True, indent=2) + "\n").encode()
    _require(
        hashlib.sha256(candidate_bytes).hexdigest() == ACCEPTED_CANDIDATE_SHA256,
        "matrix contents differ from the exact human-accepted candidate",
    )

    artifacts = ea["artifacts"]
    _require(len(artifacts) == 10, "matrix must retain ten schema digests")
    for artifact in artifacts:
        path = artifact["path"]
        _require(
            path.startswith("engineering_assurance/schemas/")
            and Path(path).name == Path(path).as_posix().split("/")[-1]
            and ".." not in Path(path).parts,
            f"schema path escapes the module: {path}",
        )
        actual = hashlib.sha256((root / path).read_bytes()).hexdigest()
        _require(actual == artifact["sha256"], f"schema bytes differ: {path}")

    version = ea["version"]
    _require(version == "0.4.1", "EA version differs from accepted pin")
    _require(
        tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
        == version,
        "Cargo package version differs",
    )
    _require(
        json.loads((root / "package.json").read_text())["version"] == version,
        "npm package version differs",
    )
    setup = configparser.ConfigParser()
    setup.read(root / "setup.cfg")
    _require(setup["metadata"]["version"] == version, "wheel version differs")
    _require(
        f"version: {version}" in (root / "engineering_assurance/manifest.yaml").read_text(),
        "Quire module version differs",
    )
    for path in [
        ".claude-plugin/plugin.json",
        ".codex-plugin/plugin.json",
        ".github/plugin/plugin.json",
    ]:
        plugin = json.loads((root / path).read_text())
        _require(plugin["version"] == version, f"plugin version differs: {path}")
        _require(
            plugin["skills"] == "./engineering_assurance/skills/",
            f"plugin skill root differs: {path}",
        )
    _require(
        (root / "engineering_assurance/skills/assurance-onboarding/SKILL.md").is_file(),
        "canonical onboarding skill is missing",
    )
    readme = (root / "README.md").read_text()
    for command in [
        "--tag v0.4.1",
        "github:agent-ix/engineering-assurance//engineering_assurance@v0.4.1",
        "/plugin marketplace add agent-ix/engineering-assurance@v0.4.1",
        "codex plugin marketplace add agent-ix/engineering-assurance --ref v0.4.1",
    ]:
        _require(command in readme, f"install documentation lacks {command}")


if __name__ == "__main__":
    try:
        qualify(Path(__file__).resolve().parents[1])
    except (KeyError, OSError, ValueError, json.JSONDecodeError) as error:
        print(f"compatibility release gate refused: {error}", file=sys.stderr)
        raise SystemExit(1) from error
    print("compatibility release gate accepted exact v0.4.1 matrix and plugins")
