from __future__ import annotations

import hashlib
import json
import os
import shutil
import subprocess
from dataclasses import replace
from pathlib import Path

import pytest

from engineering_assurance.workflow import (
    CANONICAL_SKILL,
    DecisionEvent,
    WorkflowBinding,
    WorkflowError,
    decide,
    start_or_resume,
)


def ix_flow() -> str:
    executable = os.environ.get("IX_FLOW_BIN") or shutil.which("ix-flow")
    if executable is None:
        pytest.fail("ix-flow is required by the integration contract")
    return executable


def binding(run_id: str = "architecture-run") -> WorkflowBinding:
    return WorkflowBinding(
        run_id=run_id,
        repository_id="fictional-repository@revision-1",
        workflow="architecture-evaluation",
        workflow_version="0.1.0",
        decision_boundary="one fictional architecture boundary",
        decision_owner="architecture-owner",
    )


def command(state_dir: Path, *arguments: str) -> dict:
    completed = subprocess.run(
        [ix_flow(), *arguments, "--state-dir", str(state_dir), "--json"],
        check=False,
        capture_output=True,
        text=True,
    )
    payload = json.loads(completed.stdout)
    assert payload.get("ok") is True, payload
    return payload


def add_item(state_dir: Path, run_id: str, kind: str, item: dict) -> None:
    command(
        state_dir,
        "add-item",
        run_id,
        kind,
        "--item",
        json.dumps(item, separators=(",", ":")),
    )


def prepare_decision_gate(state_dir: Path, selected: WorkflowBinding) -> None:
    start_or_resume(selected, state_dir, ix_flow_bin=ix_flow())
    command(
        state_dir,
        "record-answers",
        selected.run_id,
        "architecture",
        "--answers",
        json.dumps(
            {
                "scope": "fictional component",
                "description_path": "spec/AD-001.md",
                "concerns": ["retained response"],
                "owner": selected.decision_owner,
            },
            separators=(",", ":"),
        ),
    )
    command(state_dir, "advance", selected.run_id, "scenarios_ready")
    add_item(
        state_dir,
        selected.run_id,
        "artifact_validation",
        {
            "id": "artifact-1",
            "artifact_type": "ArchitectureDescription",
            "path": "spec/AD-001.md",
            "valid": True,
        },
    )
    add_item(
        state_dir,
        selected.run_id,
        "architecture_scenario",
        {
            "id": "scenario-1",
            "concern": "retained response",
            "stimulus": "accepted request",
            "environment": "fictional runtime",
            "response": "retain result",
            "measure": "all retained",
        },
    )
    add_item(
        state_dir,
        selected.run_id,
        "review_validation",
        {
            "id": "review-1",
            "artifact_type": "SpecReview",
            "analysis": "architecture-evaluation",
            "path": "reviews/SR-001.md",
            "subject_path": "spec/AD-001.md",
            "valid": True,
        },
    )
    add_item(
        state_dir,
        selected.run_id,
        "operator_observation",
        {"id": "observation-1", "elapsed_minutes": 3, "command_count": 4},
    )
    command(state_dir, "advance", selected.run_id, "decision_ready")


def tree_digest(root: Path) -> str:
    digest = hashlib.sha256()
    for path in sorted(root.rglob("*")):
        if path.is_file():
            digest.update(path.relative_to(root).as_posix().encode())
            digest.update(path.read_bytes())
    return digest.hexdigest()


def test_interrupted_run_resumes_from_completed_phase(tmp_path: Path) -> None:
    """Trace: FR-005-AC-1, TC-026."""
    selected = binding()
    first = start_or_resume(selected, tmp_path, ix_flow_bin=ix_flow())
    command(tmp_path, "record-answers", selected.run_id, "architecture", "--answers", json.dumps({"scope": "s", "description_path": "spec/AD.md", "concerns": ["c"], "owner": "architecture-owner"}))
    command(tmp_path, "advance", selected.run_id, "scenarios_ready")
    resumed = start_or_resume(selected, tmp_path, ix_flow_bin=ix_flow())
    assert first.phase == "capture"
    assert resumed.phase == "scenarios_ready"
    assert resumed.state_version > first.state_version


def test_terminal_transitions_remain_human_gated() -> None:
    """Trace: FR-005-AC-2, TC-027."""
    for definition in (CANONICAL_SKILL / "workflows").glob("*/def.yaml"):
        import yaml

        document = yaml.safe_load(definition.read_text())
        terminal = {
            phase["name"] for phase in document["phases"] if phase.get("terminal")
        }
        transitions = [
            item for item in document["transitions"] if item["to"] in terminal
        ]
        assert {item["to"] for item in transitions} == terminal
        assert all(item["defaultGate"] == "hitl" for item in transitions)


def test_explicit_rejection_retains_only_rejection(tmp_path: Path) -> None:
    """Trace: FR-005-AC-3, TC-028."""
    selected = binding("reject-run")
    prepare_decision_gate(tmp_path, selected)
    event = decide(selected, tmp_path, "reject", ix_flow_bin=ix_flow())
    assert isinstance(event, DecisionEvent)
    assert event.choice == "reject"
    assert event.outcome == "rejected"
    status = command(tmp_path, "status", selected.run_id)
    assert status["data"]["phase"] == "rejected"
    assert not any(
        item.get("payload", {}).get("to") == "accepted"
        for item in status["data"]["events"]
    )


def test_missing_choice_leaves_run_nonterminal(tmp_path: Path) -> None:
    """Trace: FR-005-AC-4, TC-029."""
    selected = binding("no-choice-run")
    prepare_decision_gate(tmp_path, selected)
    snapshot = decide(selected, tmp_path, None, ix_flow_bin=ix_flow())
    assert snapshot.phase == "decision_ready"
    assert snapshot.open_gates == ()


def test_automatic_terminal_override_fails_without_mutation(tmp_path: Path) -> None:
    """Trace: FR-005-AC-5, TC-030."""
    selected = binding("override-run")
    prepare_decision_gate(tmp_path, selected)
    before = tree_digest(tmp_path)
    with pytest.raises(WorkflowError, match="automatic"):
        decide(
            selected,
            tmp_path,
            "accept",
            automatic=True,
            ix_flow_bin=ix_flow(),
        )
    assert tree_digest(tmp_path) == before


def test_acceptance_records_one_attributed_event(tmp_path: Path) -> None:
    """Trace: FR-005-AC-6, TC-047."""
    selected = binding("accept-run")
    prepare_decision_gate(tmp_path, selected)
    before = command(tmp_path, "status", selected.run_id)
    assert not any(
        item["kind"] == "gate.acknowledged" for item in before["data"]["events"]
    )
    event = decide(selected, tmp_path, "accept", ix_flow_bin=ix_flow())
    assert isinstance(event, DecisionEvent)
    assert event.run_id == selected.run_id
    assert event.workflow_version == "0.1.0"
    assert event.owner == "architecture-owner"
    assert event.choice == "accept"
    assert event.timestamp.endswith("Z")
    repeated = decide(selected, tmp_path, "accept", ix_flow_bin=ix_flow())
    assert repeated == event


@pytest.mark.parametrize(
    "changed",
    [
        {"repository_id": "other-repository@revision-1"},
        {"decision_boundary": "a different boundary"},
        {"workflow_version": "0.1.1"},
        {"workflow": "assurance-intake"},
    ],
)
def test_run_binding_mismatch_is_refused_without_mutation(
    tmp_path: Path,
    changed: dict,
) -> None:
    """Trace: FR-005-AC-7, TC-048."""
    selected = binding("bound-run")
    start_or_resume(selected, tmp_path, ix_flow_bin=ix_flow())
    before = tree_digest(tmp_path)
    with pytest.raises(WorkflowError):
        start_or_resume(replace(selected, **changed), tmp_path, ix_flow_bin=ix_flow())
    assert tree_digest(tmp_path) == before
