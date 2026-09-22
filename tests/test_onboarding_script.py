from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path

import pytest

ROOT = Path(__file__).parents[1]
ONBOARD_JS = (
    ROOT
    / "engineering_assurance"
    / "skills"
    / "assurance-onboarding"
    / "scripts"
    / "onboard.js"
)

NODE = shutil.which("node")


def run_onboard_json(tmp_path: Path) -> dict:
    target_repo = tmp_path / "consumer"
    target_repo.mkdir()
    completed = subprocess.run(
        [NODE, str(ONBOARD_JS), "--repo", str(target_repo), "--json"],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


@pytest.mark.skipif(NODE is None, reason="node is not installed")
def test_onboard_js_json_checklist_lists_nested_objective_requirements(
    tmp_path: Path,
) -> None:
    """Trace: FR-020-AC-1, TC-139.

    A MeasurementPlan's `objective` block is itself optional, but once
    present its own `direction` field is required
    (`$defs.objective.required`). onboard.js's §4 checklist must surface that
    nested requirement, not just the top-level `required` list, or an author
    who reads the checklist and adds `objective:` with no `direction` finds
    out only from a schema rejection.
    """
    report = run_onboard_json(tmp_path)
    plan_checklist = report["artifactChecklists"]["MeasurementPlan"]
    conditional = plan_checklist["conditionalRequired"]

    objective_entries = [
        entry for entry in conditional if entry["when"] == "objective is present"
    ]
    assert objective_entries, conditional
    assert any(
        "objective.direction" in entry["required"] for entry in objective_entries
    ), conditional


@pytest.mark.skipif(NODE is None, reason="node is not installed")
def test_onboard_js_json_checklist_lists_the_decision_rule_vocabulary(
    tmp_path: Path,
) -> None:
    """Trace: FR-021-AC-6, TC-149.

    The structured decision rule sits two levels down
    (`statistical_design.decision_rule`), and its "exactly one of threshold
    or baseline" constraint is a `oneOf`. The §4 checklist must surface the
    closed estimator, comparator and baseline sets, the one-of choice, and
    `margin`'s dependency on `baseline`, without a WARNING.
    """
    report = run_onboard_json(tmp_path)
    plan_checklist = report["artifactChecklists"]["MeasurementPlan"]
    enums = plan_checklist["enums"]
    assert enums["statistical_design.estimator"] == [
        "proportion",
        "count",
        "mean",
        "median",
        "ratio",
    ]
    assert enums["statistical_design.decision_rule.comparator"] == [
        "gt",
        "ge",
        "lt",
        "le",
        "eq",
    ]
    assert enums["statistical_design.decision_rule.baseline"] == [
        "constant-predictor",
        "prior-collection",
        "best-seen",
    ]
    assert {
        "when": "statistical_design.decision_rule is present",
        "fields": [
            "statistical_design.decision_rule.threshold",
            "statistical_design.decision_rule.baseline",
        ],
    } in plan_checklist["exactlyOneOf"]
    conditional = plan_checklist["conditionalRequired"]
    assert {
        "when": "statistical_design.decision_rule.margin is present",
        "required": ["statistical_design.decision_rule.baseline"],
    } in conditional
    assert {
        "when": "statistical_design is present",
        "required": ["metric"],
    } in conditional
    assert plan_checklist["warnings"] == []
