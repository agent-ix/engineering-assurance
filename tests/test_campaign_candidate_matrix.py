# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX

"""The 0.5 Campaign candidate binds every generated schema to exact bytes."""

import hashlib
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MATRIX = ROOT / "engineering_assurance_v05/compatibility-matrix.json"
SCHEMA_DIR = ROOT / "engineering_assurance_v05/generated/campaign/json-schema"


def test_candidate_campaign_artifact_inventory_and_digests() -> None:
    """Trace: FR-012-AC-1; campaign release candidate artifact identity."""
    matrix = json.loads(MATRIX.read_text())
    component = next(
        item
        for item in matrix["components"]
        if item["name"] == "engineering-assurance"
    )
    expected_paths = {
        path.relative_to(ROOT).as_posix() for path in SCHEMA_DIR.glob("*.json")
    }
    campaign_artifacts = component["campaign_generated_artifacts"]
    assert {item["path"] for item in campaign_artifacts} == expected_paths
    all_artifacts = [
        *component["artifacts"],
        *campaign_artifacts,
        component["campaign_ir"],
    ]
    observed: dict[str, str] = {}
    for artifact in all_artifacts:
        path = artifact["path"]
        digest = artifact["sha256"]
        assert observed.setdefault(path, digest) == digest
        assert hashlib.sha256((ROOT / path).read_bytes()).hexdigest() == digest
