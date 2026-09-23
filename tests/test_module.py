from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
from pathlib import Path

import yaml
from jsonschema import Draft7Validator, FormatChecker

import engineering_assurance as package

ROOT = Path(__file__).parents[1]


def manifest() -> dict:
    return yaml.safe_load(package.MANIFEST_PATH.read_text())


def frontmatter(path: Path) -> dict:
    match = re.match(r"---\n(.*?)\n---\n", path.read_text(), re.DOTALL)
    assert match is not None
    return yaml.safe_load(match.group(1))


def schema(name: str) -> dict:
    return json.loads(
        (package.PACKAGE_ROOT / "schemas" / f"{name}.json").read_text()
    )


def test_module_inventory_is_exact() -> None:
    data = manifest()
    assert data["version"] == "0.3.1"
    assert [item["name"] for item in data["artifact_types"]] == [
        "AssuranceProfile",
        "MeasurementPlan",
        "ArchitectureDescription",
        "ComponentAssuranceContract",
        "AssuranceArgument",
    ]
    assert data["lint_rules"] == []
    assert data["object_types"] == []
    assert "edge_types" not in data


def test_only_the_configuration_package_entry_point_remains_python() -> None:
    python_sources = sorted(path.name for path in package.PACKAGE_ROOT.glob("*.py"))
    assert python_sources == ["__init__.py"]
    source = (package.PACKAGE_ROOT / "__init__.py").read_text(encoding="utf-8")
    assert "def " not in source
    assert "class " not in source


def test_every_schema_and_skeleton_is_valid() -> None:
    for artifact in manifest()["artifact_types"]:
        schema_path = package.PACKAGE_ROOT / artifact["frontmatter_schema_ref"]
        skeleton_path = (
            package.PACKAGE_ROOT / "skeletons" / f"{artifact['name']}.md"
        )
        contract = json.loads(schema_path.read_text())
        Draft7Validator.check_schema(contract)
        errors = list(
            Draft7Validator(
                contract, format_checker=FormatChecker()
            ).iter_errors(frontmatter(skeleton_path))
        )
        assert errors == []
        body = skeleton_path.read_text()
        for locator in artifact["body_extraction"]["yield_pattern"]["match"].values():
            assert locator["required"] is True
            assert f"## {locator['after_heading']}" in body


def test_profile_schema_version_and_profile_kind_are_optional() -> None:
    contract = schema("assurance-profile-frontmatter.schema")
    assert "schema_version" not in contract["required"]
    assert "profile_kind" not in contract["required"]
    impact = contract["$defs"]["impact"]["properties"]
    assert impact["verifiability"]["type"] == "object"
    assert impact["detect_before_harm"]["properties"]["control_ref"] == {
        "type": "string",
        "pattern": "^ix://",
    }
    classes = impact["verifiability"]["properties"]["class"]["enum"]
    assert classes == ["cheap-conclusive", "probabilistic", "proxy-only"]


def _minimal_profile(**overrides: object) -> dict:
    profile = {
        "id": "AP-900",
        "title": "t",
        "type": "AssuranceProfile",
        "status": "proposed",
        "owner": "o",
        "relationships": [],
    }
    profile.update(overrides)
    return profile


def test_profile_measurement_policy_is_closed_and_uses_plan_stages() -> None:
    """Trace: FR-025-AC-1, TC-163."""
    contract = schema("assurance-profile-frontmatter.schema")
    plan = schema("measurement-plan-frontmatter.schema")
    policy = contract["$defs"]["measurement_policy"]
    assert policy["properties"]["stages"]["items"]["enum"] == plan["properties"][
        "stage"
    ]["enum"]
    assert policy["properties"]["mode"]["enum"] == contract["$defs"][
        "review_policy"
    ]["properties"]["mode"]["enum"]
    assert "measurement_policy" not in contract["required"]
    validator = Draft7Validator(contract)
    accepted = [
        None,
        {"mode": "recommend", "stages": ["observe"]},
        {"mode": "require", "stages": ["gate"]},
        {"mode": "require", "stages": ["target", "gate"]},
    ]
    for value in accepted:
        document = (
            _minimal_profile()
            if value is None
            else _minimal_profile(measurement_policy=value)
        )
        assert list(validator.iter_errors(document)) == [], value
    refused = [
        {"mode": "require"},
        {"stages": ["gate"]},
        {"mode": "enforced", "stages": ["gate"]},
        {"mode": "advisory", "stages": ["gate"]},
        {"mode": "require", "stages": []},
        {"mode": "require", "stages": ["gate", "gate"]},
        {"mode": "require", "stages": ["release"]},
        {"mode": "require", "stages": "gate"},
        {"mode": "require", "stages": ["gate"], "exceptions": []},
        "require",
    ]
    for value in refused:
        errors = list(validator.iter_errors(_minimal_profile(measurement_policy=value)))
        assert errors != [], value


def test_measurement_stages_and_statistical_design_are_explicit() -> None:
    contract = schema("measurement-plan-frontmatter.schema")
    assert contract["properties"]["metric"]["pattern"] == "^[a-z][a-z0-9_.-]*$"
    assert contract["properties"]["definition_version"] == {
        "type": "string",
        "minLength": 1,
    }
    assert contract["properties"]["stage"]["enum"] == [
        "observe",
        "baseline",
        "branch-comparison",
        "trend",
        "ratchet",
        "target",
        "gate",
    ]
    required = contract["$defs"]["statistical_design"]["required"]
    assert required == [
        "population",
        "sampling",
        "repetitions",
        "estimator",
        "error_model",
        "uncertainty",
        "decision_rule",
    ]
    assert contract["$defs"]["statistical_design"]["properties"][
        "minimum_population"
    ] == {
        "type": "integer",
        "minimum": 1,
        "description": (
            "The smallest population size below which a result must not be "
            "trusted -- e.g. refuses the 'decided from two examples' failure "
            "mode. Optional; when set, a result collected against a smaller "
            "population is invalid and must be refused or flagged, not "
            "silently accepted."
        ),
    }
    assert "minimum_population" not in required


def _minimal_measurement_plan(**overrides: object) -> dict:
    plan = {
        "id": "MP-900",
        "title": "t",
        "type": "MeasurementPlan",
        "status": "proposed",
        "owner": "o",
        "stage": "baseline",
        "relationships": [],
    }
    plan.update(overrides)
    return plan


_NEGATIVE_CONTROL = {
    "kind": "suppressed-observation",
    "description": "a dropped failing item lowers examined below the corpus size",
}


def test_ground_truth_kind_is_required_only_for_gate_stage() -> None:
    contract = schema("measurement-plan-frontmatter.schema")
    assert contract["properties"]["ground_truth_kind"]["enum"] == [
        "human-labelled",
        "agent-labelled",
        "mechanical",
    ]
    validator = Draft7Validator(contract, format_checker=FormatChecker())

    baseline = _minimal_measurement_plan(stage="baseline")
    assert list(validator.iter_errors(baseline)) == []

    gate_missing = _minimal_measurement_plan(stage="gate")
    errors = [e.message for e in validator.iter_errors(gate_missing)]
    assert "'ground_truth_kind' is a required property" in errors

    gate_present = _minimal_measurement_plan(
        stage="gate",
        ground_truth_kind="mechanical",
        negative_controls=[_NEGATIVE_CONTROL],
        protected_apparatus=["evals/harness.py"],
    )
    assert list(validator.iter_errors(gate_present)) == []

    bad_enum = _minimal_measurement_plan(
        stage="gate", ground_truth_kind="vibes-based"
    )
    assert list(validator.iter_errors(bad_enum)) != []


def test_subject_identity_is_optional_name_and_version() -> None:
    contract = schema("measurement-plan-frontmatter.schema")
    subject_identity = contract["properties"]["subject_identity"]
    assert subject_identity["required"] == ["name", "version"]
    assert set(subject_identity["properties"]) == {"name", "version"}
    assert "subject_identity" not in contract["required"]

    validator = Draft7Validator(contract, format_checker=FormatChecker())
    plan = _minimal_measurement_plan(
        subject_identity={"name": "juniper-classifier", "version": "2026.09.1"}
    )
    assert list(validator.iter_errors(plan)) == []

    incomplete = _minimal_measurement_plan(subject_identity={"name": "juniper"})
    assert list(validator.iter_errors(incomplete)) != []


def test_preregistration_is_optional_and_requires_a_sha256_bar_digest() -> None:
    contract = schema("measurement-plan-frontmatter.schema")
    preregistration = contract["properties"]["preregistration"]
    assert preregistration["required"] == ["bar_digest"]
    assert set(preregistration["properties"]) == {"bar_digest"}
    assert "preregistration" not in contract["required"]

    validator = Draft7Validator(contract, format_checker=FormatChecker())
    digest = "sha256:" + "a" * 64
    plan = _minimal_measurement_plan(preregistration={"bar_digest": digest})
    assert list(validator.iter_errors(plan)) == []

    malformed = _minimal_measurement_plan(
        preregistration={"bar_digest": "not-a-digest"}
    )
    assert list(validator.iter_errors(malformed)) != []

    extra_field = _minimal_measurement_plan(
        preregistration={"bar_digest": digest, "bar_source": "elsewhere.rs"}
    )
    assert list(validator.iter_errors(extra_field)) != []


def test_objective_is_optional_and_target_requires_a_bound() -> None:
    """Trace: FR-020-AC-1, TC-139."""
    contract = schema("measurement-plan-frontmatter.schema")
    assert "objective" not in contract["required"]
    validator = Draft7Validator(contract, format_checker=FormatChecker())

    def errors(**overrides: object) -> list[str]:
        plan = _minimal_measurement_plan(**overrides)
        return [error.message for error in validator.iter_errors(plan)]

    assert errors() == []
    for direction in ("higher", "lower", "zero"):
        assert errors(objective={"direction": direction}) == []
        assert errors(objective={"direction": direction, "bound": 0.95}) == []
    assert errors(objective={"direction": "target", "bound": 250}) == []

    assert "'bound' is a required property" in errors(
        objective={"direction": "target"}
    )
    assert errors(objective={"direction": "sideways"}) != []
    assert errors(objective={"bound": 1}) != []
    assert errors(objective={"direction": "target", "bound": "250"}) != []
    assert errors(objective={"direction": "higher", "epoch": 2}) != []


def test_measurement_plan_skeleton_shows_a_valid_objective() -> None:
    """Trace: FR-020-AC-1, TC-139."""
    skeleton = frontmatter(package.PACKAGE_ROOT / "skeletons" / "MeasurementPlan.md")
    assert skeleton["objective"]["direction"] in {"higher", "lower", "zero", "target"}
    validator = Draft7Validator(
        schema("measurement-plan-frontmatter.schema"), format_checker=FormatChecker()
    )
    assert list(validator.iter_errors(skeleton)) == []


def test_objective_steering_fields_are_optional_advisory_and_range_checked() -> None:
    """Trace: FR-026-AC-1, TC-166."""
    contract = schema("measurement-plan-frontmatter.schema")
    objective_schema = contract["$defs"]["objective"]
    assert set(objective_schema["properties"]) == {
        "direction",
        "bound",
        "weight",
        "value_half_life",
        "budget",
    }
    assert objective_schema["required"] == ["direction"]
    validator = Draft7Validator(contract, format_checker=FormatChecker())

    def errors(**objective_overrides: object) -> list[str]:
        plan = _minimal_measurement_plan(
            objective={"direction": "higher", **objective_overrides}
        )
        return [error.message for error in validator.iter_errors(plan)]

    # No steering fields at all, and every field present together.
    assert errors() == []
    assert (
        errors(weight=1.0, value_half_life=30, budget=50) == []
    )
    # Each field independently, including the advisory-only edge cases named
    # by PLAT-967: weight and budget may be exactly zero.
    assert errors(weight=0) == []
    assert errors(weight=2.5) == []
    assert errors(budget=0) == []
    assert errors(budget=1000) == []
    assert errors(value_half_life=0.001) == []
    assert errors(value_half_life=365) == []

    # Negative weight, negative budget, and a non-positive half-life are
    # rejected.
    assert errors(weight=-0.01) != []
    assert errors(budget=-1) != []
    assert errors(value_half_life=0) != []
    assert errors(value_half_life=-1) != []

    # Non-numeric values are rejected for every field.
    for field in ("weight", "value_half_life", "budget"):
        assert errors(**{field: "a lot"}) != []
        assert errors(**{field: True}) != []
        assert errors(**{field: [1]}) != []
        assert errors(**{field: None}) != []


def test_measurement_plan_skeleton_shows_the_steering_fields() -> None:
    """Trace: FR-026-AC-5, TC-170."""
    skeleton = frontmatter(package.PACKAGE_ROOT / "skeletons" / "MeasurementPlan.md")
    objective = skeleton["objective"]
    assert objective["weight"] > 0
    assert objective["value_half_life"] > 0
    assert objective["budget"] >= 0
    validator = Draft7Validator(
        schema("measurement-plan-frontmatter.schema"), format_checker=FormatChecker()
    )
    assert list(validator.iter_errors(skeleton)) == []
    body = (package.PACKAGE_ROOT / "skeletons" / "MeasurementPlan.md").read_text(
        encoding="utf-8"
    )
    assert "## Steering Fields" in body
    assert "advisory only" in body.lower()
    skill = (
        package.PACKAGE_ROOT / "skills" / "assurance-onboarding" / "SKILL.md"
    ).read_text(encoding="utf-8")
    for field in ("`weight`", "`value_half_life`", "`budget`"):
        assert field in skill, field
    assert "never gate" in skill
    completed = subprocess.run(
        [
            "node",
            str(package.PACKAGE_ROOT / "skills" / "assurance-onboarding" / "scripts" / "onboard.js"),
            "--repo",
            str(ROOT),
            "--json",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    plan_checklist = json.loads(completed.stdout)["artifactChecklists"]["MeasurementPlan"]
    assert plan_checklist["warnings"] == []


def _statistical_design(**overrides: object) -> dict:
    design = {
        "population": "p",
        "sampling": "s",
        "repetitions": 5,
        "estimator": "proportion",
        "error_model": "e",
        "uncertainty": "u",
        "decision_rule": {"comparator": "ge", "threshold": 0.99},
    }
    design.update(overrides)
    return design


def _statistical_design_errors(
    objective: dict | None = None, **overrides: object
) -> list[str]:
    validator = Draft7Validator(
        schema("measurement-plan-frontmatter.schema"), format_checker=FormatChecker()
    )
    plan = _minimal_measurement_plan(
        metric="request_retention_rate",
        statistical_design=_statistical_design(**overrides),
    )
    if objective is not None:
        plan["objective"] = objective
    return [error.message for error in validator.iter_errors(plan)]


def test_decision_rule_is_a_closed_comparator_with_exactly_one_reference() -> None:
    """Trace: FR-021-AC-1, TC-144."""
    contract = schema("measurement-plan-frontmatter.schema")
    rule = contract["$defs"]["decision_rule"]
    assert rule["properties"]["comparator"]["enum"] == ["gt", "ge", "lt", "le", "eq"]
    assert rule["properties"]["baseline"]["enum"] == [
        "constant-predictor",
        "prior-collection",
        "best-seen",
    ]
    assert set(rule["properties"]) == {"comparator", "threshold", "baseline", "margin"}

    for comparator in ("gt", "ge", "lt", "le", "eq"):
        assert _statistical_design_errors(
            decision_rule={"comparator": comparator, "threshold": 0}
        ) == []
    assert _statistical_design_errors(
        decision_rule={"comparator": "eq", "baseline": "prior-collection"}
    ) == []
    for baseline in ("constant-predictor", "prior-collection", "best-seen"):
        assert _statistical_design_errors(
            decision_rule={"comparator": "gt", "baseline": baseline}
        ) == []
        assert _statistical_design_errors(
            decision_rule={"comparator": "ge", "baseline": baseline, "margin": -0.01}
        ) == []

    refused = {
        "unknown comparator": {"comparator": "approximately", "threshold": 1},
        "missing comparator": {"threshold": 1},
        "neither threshold nor baseline": {"comparator": "ge"},
        "both threshold and baseline": {
            "comparator": "ge",
            "threshold": 1,
            "baseline": "best-seen",
        },
        "margin with a threshold": {"comparator": "ge", "threshold": 1, "margin": 0.1},
        "margin with eq": {
            "comparator": "eq",
            "baseline": "prior-collection",
            "margin": 0.1,
        },
        "non-numeric margin": {
            "comparator": "ge",
            "baseline": "best-seen",
            "margin": "0.05",
        },
        "unknown baseline": {"comparator": "ge", "baseline": "vibes"},
        "eq against best-seen": {"comparator": "eq", "baseline": "best-seen"},
        "non-numeric threshold": {"comparator": "ge", "threshold": "0.99"},
        "duplicated repetitions": {"comparator": "ge", "threshold": 1, "repetitions": 5},
        "duplicated minimum_n": {"comparator": "ge", "threshold": 1, "minimum_n": 20},
    }
    for case, decision_rule in refused.items():
        assert _statistical_design_errors(decision_rule=decision_rule) != [], case
    assert _statistical_design_errors(decision_rule="escalate when low") != []

    # JSON Schema has no finiteness keyword: a YAML `.inf` threshold passes
    # the schema, and only the Rust DecisionRule refuses it (FR-021).
    assert _statistical_design_errors(
        decision_rule={"comparator": "ge", "threshold": float("inf")}
    ) == []

    validator = Draft7Validator(contract, format_checker=FormatChecker())
    without_metric = _minimal_measurement_plan(statistical_design=_statistical_design())
    assert "'metric' is a required property" in [
        error.message for error in validator.iter_errors(without_metric)
    ]


def test_decision_rule_agrees_with_objective_direction_and_estimator() -> None:
    """Trace: FR-021-AC-9, TC-144."""
    def rule(comparator: str, **reference: object) -> dict:
        return {"comparator": comparator, **(reference or {"threshold": 0})}

    agreeing = [
        ("higher", rule("gt")),
        ("higher", rule("ge")),
        ("lower", rule("lt")),
        ("lower", rule("le", baseline="best-seen")),
        ("zero", rule("eq")),
        ("zero", rule("eq", baseline="prior-collection")),
        ("zero", rule("le", threshold=0)),
        ("target", rule("lt", threshold=1)),
        ("target", rule("eq", threshold=1)),
    ]
    for direction, decision_rule in agreeing:
        objective = {"direction": direction, "bound": 1}
        assert _statistical_design_errors(
            objective=objective, decision_rule=decision_rule
        ) == [], (direction, decision_rule)
    disagreeing = [
        ("higher", rule("le")),
        ("higher", rule("eq")),
        ("lower", rule("ge")),
        ("lower", rule("gt", baseline="best-seen")),
        ("zero", rule("le", threshold=0.5)),
        ("zero", rule("le", baseline="prior-collection")),
        ("zero", rule("lt", threshold=0)),
    ]
    for direction, decision_rule in disagreeing:
        assert _statistical_design_errors(
            objective={"direction": direction}, decision_rule=decision_rule
        ) != [], (direction, decision_rule)

    constant = {"comparator": "gt", "baseline": "constant-predictor", "margin": 0.05}
    assert _statistical_design_errors(estimator="proportion", decision_rule=constant) == []
    for estimator in ("count", "mean", "median", "ratio"):
        assert _statistical_design_errors(
            estimator=estimator, decision_rule=constant
        ) != [], estimator
        assert _statistical_design_errors(
            estimator=estimator,
            decision_rule={"comparator": "ge", "baseline": "best-seen"},
        ) == [], estimator


def test_estimator_is_a_closed_vocabulary() -> None:
    """Trace: FR-021-AC-2, TC-145."""
    contract = schema("measurement-plan-frontmatter.schema")
    assert contract["$defs"]["statistical_design"]["properties"]["estimator"]["enum"] == [
        "proportion",
        "count",
        "mean",
        "median",
        "ratio",
    ]
    for estimator in ("proportion", "count", "mean", "median", "ratio"):
        assert _statistical_design_errors(estimator=estimator) == []
    for refused in ("retained-result proportion", "p90", "", 1):
        assert _statistical_design_errors(estimator=refused) != [], refused
    # Prose fields stay prose (FR-021): only estimator and decision_rule close.
    for prose in ("population", "sampling", "error_model", "uncertainty"):
        assert contract["$defs"]["statistical_design"]["properties"][prose] == {
            "type": "string",
            "minLength": 1,
        }


def test_measurement_plan_skeleton_shows_a_structured_decision_rule() -> None:
    """Trace: FR-021-AC-2, TC-145."""
    skeleton = frontmatter(package.PACKAGE_ROOT / "skeletons" / "MeasurementPlan.md")
    design = skeleton["statistical_design"]
    assert design["estimator"] == "proportion"
    assert design["decision_rule"] == {"comparator": "ge", "threshold": 0.99}
    # The objective's bound is an informational goal, distinct from the
    # evaluated threshold. weight/value_half_life/budget are advisory
    # steering fields (FR-026), asserted separately in
    # test_measurement_plan_skeleton_shows_the_steering_fields.
    assert skeleton["objective"]["direction"] == "higher"
    assert skeleton["objective"]["bound"] == 0.995
    validator = Draft7Validator(
        schema("measurement-plan-frontmatter.schema"), format_checker=FormatChecker()
    )
    assert list(validator.iter_errors(skeleton)) == []


# One case table for the `protected_apparatus` entry syntax (FR-024), shared
# with the Rust `ApparatusPath` tests in tests/measurement.rs.
APPARATUS_PATH_CASES = json.loads(
    (ROOT / "tests" / "fixtures" / "apparatus-paths.json").read_text()
)


def _apparatus_errors(**overrides: object) -> list[str]:
    validator = Draft7Validator(
        schema("measurement-plan-frontmatter.schema"), format_checker=FormatChecker()
    )
    plan = _minimal_measurement_plan(**overrides)
    return [error.message for error in validator.iter_errors(plan)]


def test_protected_apparatus_is_a_unique_list_of_safe_relative_paths() -> None:
    """Trace: FR-024-AC-1, TC-154."""
    contract = schema("measurement-plan-frontmatter.schema")
    assert "protected_apparatus" not in contract["required"]
    accepted = [case["path"] for case in APPARATUS_PATH_CASES["accepted"]]
    refused = [case["path"] for case in APPARATUS_PATH_CASES["refused"]]
    assert accepted and refused
    assert _apparatus_errors() == []
    assert _apparatus_errors(protected_apparatus=accepted) == []
    for path in accepted:
        assert _apparatus_errors(protected_apparatus=[path]) == [], path
    for path in refused:
        assert _apparatus_errors(protected_apparatus=[path]) != [], repr(path)
    assert _apparatus_errors(protected_apparatus=[]) != []
    assert _apparatus_errors(protected_apparatus=["a.json", "a.json"]) != []
    assert _apparatus_errors(protected_apparatus="corpus/*.json") != []
    assert _apparatus_errors(protected_apparatus=[7]) != []


def test_negative_controls_are_closed_and_required_at_gate_stage() -> None:
    """Trace: FR-024-AC-2, TC-155."""
    contract = schema("measurement-plan-frontmatter.schema")
    kinds = contract["$defs"]["negative_control"]["properties"]["kind"]["enum"]
    assert kinds == [
        "suppressed-observation",
        "gain-within-noise",
        "stale-evidence",
        "apparatus-edit",
        "selective-reporting",
    ]
    gate = {
        "stage": "gate",
        "ground_truth_kind": "mechanical",
        "protected_apparatus": ["evals/harness.py"],
    }
    for kind in kinds:
        control = {"kind": kind, "description": "declared guard"}
        assert _apparatus_errors(
            negative_controls=[control], protected_apparatus=["evals/harness.py"]
        ) == [], kind
        assert _apparatus_errors(negative_controls=[control], **gate) == [], kind
    # Optional below gate stage.
    for stage in ("observe", "baseline", "branch-comparison", "trend", "ratchet", "target"):
        assert _apparatus_errors(stage=stage) == [], stage

    gate_without = _apparatus_errors(stage="gate", ground_truth_kind="mechanical")
    assert "'negative_controls' is a required property" in gate_without
    assert "'protected_apparatus' is a required property" in gate_without
    refused = {
        "empty list": [],
        "unknown kind": [{"kind": "vibes", "description": "d"}],
        "missing kind": [{"description": "d"}],
        "missing description": [{"kind": "stale-evidence"}],
        "empty description": [{"kind": "stale-evidence", "description": ""}],
        "extra key": [{**_NEGATIVE_CONTROL, "severity": "high"}],
        "duplicate": [_NEGATIVE_CONTROL, _NEGATIVE_CONTROL],
        "bare kind": ["stale-evidence"],
    }
    for case, controls in refused.items():
        assert _apparatus_errors(negative_controls=controls, **gate) != [], case


def test_gate_and_apparatus_edit_plans_require_protected_apparatus() -> None:
    """Trace: FR-024-AC-8, TC-155."""
    control = {"kind": "suppressed-observation", "description": "d"}
    edit = {"kind": "apparatus-edit", "description": "d"}
    gate = {"stage": "gate", "ground_truth_kind": "mechanical"}

    assert "'protected_apparatus' is a required property" in _apparatus_errors(
        negative_controls=[control], **gate
    )
    assert _apparatus_errors(
        negative_controls=[control], protected_apparatus=["evals/**"], **gate
    ) == []
    # An apparatus-edit control needs a protected list at every stage.
    for stage in ("observe", "baseline", "trend", "gate"):
        extra = gate if stage == "gate" else {"stage": stage}
        assert "'protected_apparatus' is a required property" in _apparatus_errors(
            negative_controls=[control, edit], **extra
        ), stage
        assert _apparatus_errors(
            negative_controls=[control, edit],
            protected_apparatus=["evals/harness.py"],
            **extra,
        ) == [], stage
    # Other kinds below gate stage do not require it.
    assert _apparatus_errors(negative_controls=[control]) == []


def test_measurement_plan_skeleton_shows_apparatus_and_negative_controls() -> None:
    """Trace: FR-024-AC-6, TC-159."""
    skeleton = frontmatter(package.PACKAGE_ROOT / "skeletons" / "MeasurementPlan.md")
    assert skeleton["protected_apparatus"], skeleton
    assert skeleton["negative_controls"], skeleton
    kinds = {control["kind"] for control in skeleton["negative_controls"]}
    assert "apparatus-edit" in kinds
    validator = Draft7Validator(
        schema("measurement-plan-frontmatter.schema"), format_checker=FormatChecker()
    )
    assert list(validator.iter_errors(skeleton)) == []
    body = (package.PACKAGE_ROOT / "skeletons" / "MeasurementPlan.md").read_text()
    assert "## Protected Apparatus" in body
    assert "## Negative Controls" in body
    skill = (
        package.PACKAGE_ROOT / "skills" / "assurance-onboarding" / "SKILL.md"
    ).read_text()
    assert "`protected_apparatus`" in skill
    assert "`negative_controls`" in skill
    for kind in kinds | {"gain-within-noise", "stale-evidence", "selective-reporting"}:
        assert f"`{kind}`" in skill, kind


def test_component_contract_exposes_failure_and_control_boundaries() -> None:
    contract = schema("component-assurance-contract-frontmatter.schema")
    assert {
        "responsibility",
        "failure_behaviors",
        "version_pins",
        "controls",
        "isolation",
        "replacement",
    } <= set(contract["required"])
    assert contract["properties"]["kind"]["enum"] == [
        "deterministic",
        "stochastic",
        "human",
    ]


def test_argument_has_authored_claims_and_no_score() -> None:
    contract = schema("assurance-argument-frontmatter.schema")
    assert {"top_claim", "reasoning", "participants", "challenges"} <= set(
        contract["required"]
    )
    assert "score" not in json.dumps(contract).casefold()


def _argument_with_top_claim(**claim_overrides: object) -> dict:
    skeleton = package.PACKAGE_ROOT / "skeletons" / "AssuranceArgument.md"
    argument = frontmatter(skeleton)
    claim = {
        key: value
        for key, value in argument["top_claim"].items()
        if key != "evidence_refs"
    }
    claim.update(claim_overrides)
    argument["top_claim"] = claim
    return argument


def test_supported_claim_must_reference_evidence() -> None:
    """Trace: FR-023-AC-1, FR-023-AC-2, FR-023-AC-3, TC-143."""
    validator = Draft7Validator(
        schema("assurance-argument-frontmatter.schema"),
        format_checker=FormatChecker(),
    )
    evidence = "ix://example/juniper/evidence/request-loss-run"

    supported_without_refs = _argument_with_top_claim(status="supported")
    errors = [e.message for e in validator.iter_errors(supported_without_refs)]
    assert "'evidence_refs' is a required property" in errors

    supported_with_refs = _argument_with_top_claim(
        status="supported", evidence_refs=[evidence]
    )
    assert list(validator.iter_errors(supported_with_refs)) == []

    for status in ("open", "challenged", "rejected"):
        without_refs = _argument_with_top_claim(status=status)
        assert list(validator.iter_errors(without_refs)) == []
        with_refs = _argument_with_top_claim(status=status, evidence_refs=[evidence])
        assert list(validator.iter_errors(with_refs)) == []

    for refs in (
        [],
        [evidence, evidence],
        ["evidence/request-loss-run"],
        [""],
        ["ix:foo"],
    ):
        malformed = _argument_with_top_claim(status="supported", evidence_refs=refs)
        errors = list(validator.iter_errors(malformed))
        assert errors != [], refs
        assert any("top_claim" in list(error.absolute_path) for error in errors), refs


def test_onboarding_checklist_reports_the_nested_evidence_refs_condition() -> None:
    """Trace: FR-023-AC-1, TC-143.

    `onboard.js` section 4 derives conditional requirements from a schema's
    `allOf`. The claim's `evidence_refs` requirement is not on the
    AssuranceArgument schema's own top-level `allOf` -- it sits inside
    `$defs/claim`, reached only through `top_claim`'s `$ref`. Assert the
    onboarding checklist actually walks that `$ref` and surfaces the
    condition, rather than silently omitting it (or warning that it cannot
    read it).
    """
    script = (
        package.PACKAGE_ROOT
        / "skills"
        / "assurance-onboarding"
        / "scripts"
        / "onboard.js"
    )
    completed = subprocess.run(
        ["node", str(script), "--repo", str(ROOT), "--json"],
        check=True,
        capture_output=True,
        text=True,
    )
    report = json.loads(completed.stdout)
    argument_checklist = report["artifactChecklists"]["AssuranceArgument"]
    assert argument_checklist["warnings"] == []
    conditions = {
        (entry["when"], tuple(entry["required"]))
        for entry in argument_checklist["conditionalRequired"]
    }
    assert ("top_claim.status = supported", ("top_claim.evidence_refs",)) in conditions


def test_repository_has_only_governed_review_evidence() -> None:
    """Trace: StR-001-VC-1, TC-001; StR-003-VC-2, TC-097."""
    assert not (ROOT / "examples").exists()
    assert not (ROOT / "research").exists()
    review_files = sorted((ROOT / "reviews").glob("*.md"))
    assert review_files
    assert not [path for path in (ROOT / "reviews").rglob("*") if path.is_dir()]
    for path in review_files:
        metadata = frontmatter(path)
        assert metadata["type"] == "SpecReview"
        assert metadata["analysis"] in {"code-review", "gap-analysis"}
    assert {path.name for path in (ROOT / "plan").iterdir()} == {
        "PLAN-001-assurance-onboarding",
        "PLAN-002-verification-semantics",
    }
    assert {path.name for path in (ROOT / "docs").iterdir()} == {
        "compatibility-matrix.md",
        "consumption-boundary.md",
        "migration-contract.md",
        "structural-coverage.md",
        "verification-semantics",
    }


def test_module_payload_is_visible_to_git_and_rights_checks() -> None:
    expected = {
        path.relative_to(ROOT).as_posix()
        for path in package.PACKAGE_ROOT.rglob("*")
        if path.is_file() and "__pycache__" not in path.parts
    }
    completed = subprocess.run(
        [
            "git",
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
    )
    visible = {
        raw.decode() for raw in completed.stdout.split(b"\0") if raw
    }
    assert expected <= visible


def test_packages_are_private_and_have_no_release_configuration() -> None:
    npm = json.loads((ROOT / "package.json").read_text())
    assert npm["private"] is True
    assert "publishConfig" not in npm
    assert "prepublishOnly" in npm["scripts"]
    assert "Private :: Do Not Upload" in (ROOT / "setup.cfg").read_text()
    workflow_text = "\n".join(
        path.read_text() for path in (ROOT / ".github" / "workflows").glob("*.yml")
    )
    assert "publish" not in workflow_text.casefold()
    assert "upload-artifact" not in workflow_text.casefold()


def test_hosted_ci_is_manual_only() -> None:
    """Program invariant: opening or updating a PR must not dispatch hosted CI."""
    workflow = yaml.load(
        (ROOT / ".github" / "workflows" / "ci.yml").read_text(),
        Loader=yaml.BaseLoader,
    )
    assert set(workflow["on"]) == {"workflow_dispatch"}


def test_manual_verification_workflow_runs_the_rust_foundation_gate() -> None:
    workflow = yaml.load(
        (ROOT / ".github" / "workflows" / "ci.yml").read_text(),
        Loader=yaml.BaseLoader,
    )
    steps = workflow["jobs"]["verify"]["steps"]
    assert steps[0]["with"]["submodules"] == "recursive"
    rust_setup = next(
        step for step in steps if str(step.get("uses", "")).startswith("dtolnay/rust-toolchain@")
    )
    assert rust_setup["with"] == {
        "toolchain": "1.98.1",
        "components": "rustfmt, clippy",
    }
    installed_tools = {
        step["with"]["tool"]
        for step in steps
        if str(step.get("uses", "")).startswith("taiki-e/install-action@")
    }
    assert installed_tools == {"cargo-deny@0.19.8", "cargo-audit@0.22.2"}
    assert any(step.get("run") == "make rust-foundation-gate" for step in steps)

    makefile = (ROOT / "Makefile").read_text()
    foundation = next(
        line for line in makefile.splitlines() if line.startswith("rust-foundation-gate:")
    )
    assert {"rust-deps", "rust-audit"} <= set(foundation.split()[1:])
    assert "$(CARGO) deny --locked check" in makefile
    assert "$(CARGO) audit" in makefile


def test_structural_coverage_never_collapses_unknowns_into_success() -> None:
    text = (ROOT / "docs" / "structural-coverage.md").read_text().casefold()
    assert "exactly once" in text
    assert "silently merged into success" in text
    assert "quality score" in text


def test_quire_accepts_every_skeleton_without_diagnostics() -> None:
    executable = os.environ.get("QUIRE_BIN") or shutil.which("quire")
    if executable is None:
        if os.environ.get("REQUIRE_QUIRE") == "1":
            raise AssertionError("quire is required")
        return
    documents = [
        str(path.relative_to(ROOT))
        for path in sorted((package.PACKAGE_ROOT / "skeletons").glob("*.md"))
    ]
    completed = subprocess.run(
        [
            executable,
            "validate",
            "--module",
            str(package.PACKAGE_ROOT.relative_to(ROOT)),
            "--strict",
            *documents,
        ],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr
    diagnostics = completed.stderr.splitlines()
    assert diagnostics
    assert all(line.startswith("UnknownEdgeType:") for line in diagnostics)
