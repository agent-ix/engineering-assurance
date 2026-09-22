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
    closed estimator, comparator and baseline sets, the one-of choice,
    `margin`'s dependency on `baseline`, the direction and estimator
    consistency rules, and the refused `eq` combinations, without a WARNING.
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
    constraints = {
        entry["when"]: entry["constraint"]
        for entry in plan_checklist["conditionalConstraints"]
    }
    assert constraints == {
        "objective.direction = higher": (
            "statistical_design.decision_rule.comparator must be one of gt, ge"
        ),
        "objective.direction = lower": (
            "statistical_design.decision_rule.comparator must be one of lt, le"
        ),
        "objective.direction = zero": (
            "either (statistical_design.decision_rule.comparator must be eq) or "
            "(statistical_design.decision_rule.threshold required and "
            "statistical_design.decision_rule.comparator must be le and "
            "statistical_design.decision_rule.threshold must be 0)"
        ),
        "statistical_design.decision_rule.baseline = constant-predictor": (
            "statistical_design.estimator must be proportion"
        ),
        "statistical_design.decision_rule.comparator = eq": (
            "statistical_design.decision_rule.baseline must not be best-seen and "
            "statistical_design.decision_rule.margin must be absent"
        ),
    }
    assert plan_checklist["warnings"] == []


@pytest.mark.skipif(NODE is None, reason="node is not installed")
def test_onboard_js_reports_combined_then_and_warns_on_an_empty_one_of(
    tmp_path: Path,
) -> None:
    """Trace: FR-021-AC-6, TC-149.

    A `then` that carries both `required` and a forbidden value reports both
    halves, and an empty `oneOf` (unsatisfiable, and not an "exactly one of"
    choice) is a loud warning rather than an empty exactly-one-of entry.
    """
    module = tmp_path / "engineering_assurance"
    script_dir = module / "skills" / "assurance-onboarding" / "scripts"
    script_dir.mkdir(parents=True)
    shutil.copy(ONBOARD_JS, script_dir / "onboard.js")
    (module / "skeletons").mkdir()
    (module / "schemas").mkdir()
    (module / "manifest.yaml").write_text(
        "artifact_types:\n"
        "  - name: Probe\n"
        "    frontmatter_schema_ref: schemas/probe.schema.json\n"
    )
    (module / "schemas" / "probe.schema.json").write_text(
        json.dumps(
            {
                "type": "object",
                "properties": {
                    "mode": {"enum": ["a", "b"]},
                    "rule": {
                        "type": "object",
                        "properties": {"x": {"type": "number"}},
                        "oneOf": [],
                    },
                },
                "allOf": [
                    {
                        "if": {
                            "properties": {"mode": {"const": "a"}},
                            "required": ["mode"],
                        },
                        "then": {
                            "required": ["rule"],
                            "properties": {"kind": {"not": {"const": "z"}}},
                        },
                    }
                ],
            }
        )
    )
    target_repo = tmp_path / "consumer"
    target_repo.mkdir()
    completed = subprocess.run(
        [NODE, str(script_dir / "onboard.js"), "--repo", str(target_repo), "--json"],
        check=True,
        capture_output=True,
        text=True,
    )
    probe = json.loads(completed.stdout)["artifactChecklists"]["Probe"]
    assert {"when": "mode = a", "required": ["rule"]} in probe["conditionalRequired"]
    assert probe["conditionalConstraints"] == [
        {"when": "mode = a", "constraint": "kind must not be z"}
    ]
    assert probe["exactlyOneOf"] == []
    assert any("oneOf" in warning for warning in probe["warnings"]), probe["warnings"]


@pytest.mark.skipif(NODE is None, reason="node is not installed")
def test_onboard_js_json_checklist_lists_protected_apparatus_and_negative_controls(
    tmp_path: Path,
) -> None:
    """Trace: FR-024-AC-6, TC-159.

    `protected_apparatus` is a list of pattern-checked strings and
    `negative_controls` a list of `{kind, description}` objects required at
    gate stage. The §4 checklist must surface both lists' shape, the item
    pattern, the closed control kinds, and the gate-stage requirement, with no
    WARNING.
    """
    schema_path = (
        ROOT
        / "engineering_assurance"
        / "schemas"
        / "measurement-plan-frontmatter.schema.json"
    )
    item_pattern = json.loads(schema_path.read_text())["$defs"]["apparatus_path"][
        "pattern"
    ]
    report = run_onboard_json(tmp_path)
    plan_checklist = report["artifactChecklists"]["MeasurementPlan"]
    assert plan_checklist["arrays"]["protected_apparatus"] == {
        "minItems": 1,
        "uniqueItems": True,
        "itemPattern": item_pattern,
    }
    assert plan_checklist["arrays"]["negative_controls"] == {
        "minItems": 1,
        "uniqueItems": True,
    }
    assert plan_checklist["enums"]["negative_controls.kind"] == [
        "suppressed-observation",
        "gain-within-noise",
        "stale-evidence",
        "apparatus-edit",
        "selective-reporting",
    ]
    conditional = plan_checklist["conditionalRequired"]
    assert {
        "when": "stage = gate",
        "required": ["ground_truth_kind", "negative_controls"],
    } in conditional
    assert {
        "when": "negative_controls is present",
        "required": ["negative_controls.kind", "negative_controls.description"],
    } in conditional
    assert plan_checklist["warnings"] == []
