from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest
import yaml

ROOT = Path(__file__).parents[1]
PILOT = ROOT / "pilots" / "assurance-workflows"
DEFINITIONS = PILOT / "workflows"
CANONICAL = ROOT / "engineering_assurance" / "skills" / "assurance-onboarding"
CANONICAL_DEFINITIONS = CANONICAL / "workflows"


def definitions() -> dict[str, dict]:
    return {
        path.parent.name: yaml.safe_load(path.read_text())
        for path in sorted(DEFINITIONS.glob("*/def.yaml"))
    }


def run_invariant(tmp_path: Path, name: str, instance: dict) -> object:
    module = (PILOT / "scripts" / "invariants.js").as_uri()
    script = tmp_path / "check.mjs"
    script.write_text(
        "import { invariants } from "
        + json.dumps(module)
        + ";\n"
        + "const result = await invariants["
        + json.dumps(name)
        + "]({ instance: "
        + json.dumps(instance)
        + " });\nconsole.log(JSON.stringify(result));\n"
    )
    completed = subprocess.run(
        ["node", str(script)],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(completed.stdout)


def test_workflow_inventory_and_versions_are_exact() -> None:
    data = definitions()
    assert set(data) == {
        "assurance-intake",
        "architecture-evaluation",
        "measurement-promotion",
        "change-assurance",
    }
    assert data["change-assurance"]["version"] == "0.2.0"
    assert {
        value["version"] for key, value in data.items() if key != "change-assurance"
    } == {"0.1.0"}


def test_every_terminal_transition_is_human_gated() -> None:
    for definition in definitions().values():
        terminal = {
            phase["name"] for phase in definition["phases"] if phase.get("terminal")
        }
        transitions = [
            transition
            for transition in definition["transitions"]
            if transition["to"] in terminal
        ]
        assert {item["to"] for item in transitions} == terminal
        assert all(item["defaultGate"] == "hitl" for item in transitions)
        assert all(item["invariants"] == [] for item in transitions)


def test_terminal_gate_override_fails(tmp_path: Path) -> None:
    result = run_invariant(
        tmp_path,
        "shared.terminal_gates",
        {
            "defName": "change-assurance",
            "items": {},
            "gateConfig": {
                "decision_ready->approved": "auto",
                "decision_ready->rejected": "hitl",
            },
        },
    )
    assert result["ok"] is False
    assert result["code"] == "terminal_gate_override"


def test_adjacent_measurement_promotion_passes(tmp_path: Path) -> None:
    request = {
        "interviewId": "promotion",
        "plan_path": "fixtures/MP-001.md",
        "definition_version": "v1",
        "prior_stage": "baseline",
        "proposed_stage": "branch-comparison",
    }
    evidence = {
        "plan_path": "fixtures/MP-001.md",
        "definition_version": "v1",
        "prior_stage": "baseline",
        "proposed_stage": "branch-comparison",
        "stability": "fixed fixture",
        "decision_yield": "one changed decision",
        "limitations": "fictional sample",
        "owner": "measurement-owner",
    }
    assert (
        run_invariant(
            tmp_path,
            "measurement.promotion_ready",
            {
                "defName": "measurement-promotion",
                "items": {
                    "promotion_request": [request],
                    "promotion_evidence": [evidence],
                },
            },
        )
        is True
    )


def test_non_adjacent_measurement_promotion_fails(tmp_path: Path) -> None:
    result = run_invariant(
        tmp_path,
        "measurement.promotion_ready",
        {
            "defName": "measurement-promotion",
            "items": {
                "promotion_request": [
                    {
                        "interviewId": "promotion",
                        "prior_stage": "observe",
                        "proposed_stage": "gate",
                    }
                ]
            },
        },
    )
    assert result["code"] == "promotion_must_advance_one_stage"


def test_missing_change_arrays_fail_closed(tmp_path: Path) -> None:
    result = run_invariant(
        tmp_path,
        "change.impact_ready",
        {
            "defName": "change-assurance",
            "items": {
                "change_request": [
                    {
                        "interviewId": "change",
                        "source_revision": "a" * 40,
                        "profile_path": "fixtures/AP-001.md",
                        "baseline_id": "baseline-1",
                    }
                ],
                "impact_snapshot": [
                    {
                        "source_revision": "a" * 40,
                        "profile_path": "fixtures/AP-001.md",
                        "baseline_id": "baseline-1",
                        "changed_nodes": [],
                    }
                ],
            },
        },
    )
    assert result["code"] == "impact_snapshot_incomplete"


def test_unexpected_invalid_exception_fails_closed(tmp_path: Path) -> None:
    result = run_invariant(
        tmp_path,
        "shared.exceptions_ready",
        {
            "defName": "change-assurance",
            "items": {
                "change_request": [
                    {"interviewId": "change", "exceptions_expected": False}
                ],
                "exception": [
                    {
                        "owner": "decision-owner",
                        "expires_at": "not-a-date",
                        "rationale": "fictional",
                        "impact": "unknown",
                    }
                ],
            },
        },
    )
    assert result["code"] == "owned_current_exception_required"


def test_architecture_review_must_match_description(tmp_path: Path) -> None:
    result = run_invariant(
        tmp_path,
        "architecture.review_ready",
        {
            "defName": "architecture-evaluation",
            "items": {
                "architecture_request": [
                    {
                        "interviewId": "architecture",
                        "description_path": "spec/AD-001.md",
                    }
                ],
                "review_validation": [
                    {
                        "valid": True,
                        "artifact_type": "SpecReview",
                        "analysis": "architecture-evaluation",
                        "subject_path": "spec/AD-999.md",
                        "path": "reviews/SR-001.md",
                    }
                ],
            },
        },
    )
    assert result["code"] == "architecture_review_missing"


def test_change_review_must_match_source_revision(tmp_path: Path) -> None:
    result = run_invariant(
        tmp_path,
        "change.review_ready",
        {
            "defName": "change-assurance",
            "items": {
                "change_request": [
                    {"interviewId": "change", "source_revision": "a" * 40}
                ],
                "review_validation": [
                    {
                        "valid": True,
                        "artifact_type": "SpecReview",
                        "analysis": "code-review",
                        "source_revision": "b" * 40,
                        "path": "reviews/SR-002.md",
                    }
                ],
            },
        },
    )
    assert result["code"] == "code_review_missing"


def test_ix_flow_can_load_every_definition(tmp_path: Path) -> None:
    executable = os.environ.get("IX_FLOW_BIN") or shutil.which("ix-flow")
    if executable is None:
        if os.environ.get("REQUIRE_IX_FLOW") == "1":
            pytest.fail("ix-flow is required")
        pytest.skip("ix-flow is unavailable")
    for name in definitions():
        completed = subprocess.run(
            [
                executable,
                "run",
                name,
                "--path",
                str(PILOT),
                "--id",
                f"test-{name}",
                "--state-dir",
                str(tmp_path / name),
                "--json",
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        assert completed.returncode == 0, completed.stderr
        payload = json.loads(completed.stdout)
        assert payload["data"]["defName"] == name


def test_pilot_and_canonical_workflows_are_equivalent() -> None:
    """Trace: FR-007-AC-2, TC-036; FR-007-CON-1, TC-043."""
    pilot = definitions()
    canonical = {
        path.parent.name: yaml.safe_load(path.read_text())
        for path in sorted(CANONICAL_DEFINITIONS.glob("*/def.yaml"))
    }
    assert set(pilot) == {
        "assurance-intake",
        "architecture-evaluation",
        "measurement-promotion",
        "change-assurance",
    }
    assert canonical == pilot


def test_pilot_invariant_surface_delegates_to_canonical() -> None:
    """Trace: FR-007-AC-2, TC-036."""
    compatibility = (PILOT / "scripts" / "invariants.js").read_text()
    assert "engineering_assurance/skills/assurance-onboarding" in compatibility


def test_ix_flow_can_load_every_canonical_definition(tmp_path: Path) -> None:
    """Trace: FR-002-AC-3, TC-011; FR-007-AC-1, TC-035."""
    executable = os.environ.get("IX_FLOW_BIN") or shutil.which("ix-flow")
    if executable is None:
        if os.environ.get("REQUIRE_IX_FLOW") == "1":
            pytest.fail("ix-flow is required")
        pytest.skip("ix-flow is unavailable")
    for name in definitions():
        completed = subprocess.run(
            [
                executable,
                "run",
                name,
                "--path",
                str(CANONICAL),
                "--id",
                f"canonical-{name}",
                "--state-dir",
                str(tmp_path / name),
                "--json",
            ],
            check=False,
            capture_output=True,
            text=True,
        )
        assert completed.returncode == 0, completed.stderr
        assert json.loads(completed.stdout)["data"]["defName"] == name
