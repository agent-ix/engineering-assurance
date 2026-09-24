# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX

"""The Campaign candidate's procedure path is an exact, safe plan input."""

import json
from pathlib import Path

import pytest
import yaml
from jsonschema import Draft7Validator


ROOT = Path(__file__).resolve().parents[1]
MODULE = ROOT / "engineering_assurance_v05"
SCHEMA = json.loads(
    (MODULE / "schemas/measurement-plan-frontmatter.schema.json").read_text()
)
VALIDATOR = Draft7Validator(SCHEMA)


def plan() -> dict:
    skeleton = (MODULE / "skeletons/MeasurementPlan.md").read_text()
    return yaml.safe_load(skeleton.split("---", 2)[1])


def errors(value: dict) -> list:
    return list(VALIDATOR.iter_errors(value))


def test_procedure_path_is_optional_and_safe_json_files_are_valid() -> None:
    """Trace: FR-024-AC-10, TC-185."""
    Draft7Validator.check_schema(SCHEMA)
    candidate = plan()
    assert errors(candidate) == []
    for path in (
        "campaign/procedures/probe.json",
        "campaign/.procedures/probe.JSON",
        "campaign/procedures/.probe.json",
    ):
        candidate["execution_procedure"] = path
        candidate["protected_apparatus"].append(path)
        assert errors(candidate) == [], path
        candidate["protected_apparatus"].pop()


@pytest.mark.parametrize(
    "path",
    [
        "",
        "../escape.json",
        "/absolute/probe.json",
        "C:\\probe.json",
        "campaign//probe.json",
        "campaign/./probe.json",
        "campaign/../probe.json",
        "campaign/procedures/**",
        "campaign/procedure.yaml",
        "campaign/procedure.json/",
        "campaign\\procedure.json",
        "campaign/procedure.json\n",
        ".json",
        "campaign/.json",
        42,
    ],
)
def test_procedure_path_refuses_unsafe_or_non_json_values(path: object) -> None:
    """Trace: FR-024-AC-10, TC-185."""
    candidate = plan()
    candidate["execution_procedure"] = path
    assert errors(candidate) != [], path


def test_procedure_path_requires_protected_apparatus_declaration() -> None:
    """Trace: FR-024-AC-10, TC-185; Quoin checks exact path coverage."""
    candidate = plan()
    candidate["execution_procedure"] = "campaign/procedures/probe.json"
    candidate.pop("protected_apparatus")
    assert errors(candidate) != []
