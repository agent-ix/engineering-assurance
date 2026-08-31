"""Thin ix-flow integration for bound, resumable human decisions."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import yaml

from engineering_assurance import PACKAGE_ROOT
from engineering_assurance.discovery import EXPECTED_WORKFLOWS

CANONICAL_SKILL = PACKAGE_ROOT / "skills" / "assurance-onboarding"
TERMINAL_PHASES = {
    "assurance-intake": {"accept": "accepted", "reject": "rejected"},
    "architecture-evaluation": {"accept": "accepted", "reject": "rejected"},
    "measurement-promotion": {
        "accept": "promoted",
        "reject": "not_promoted",
    },
    "change-assurance": {"accept": "approved", "reject": "rejected"},
}
RUN_ID = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$")


class WorkflowError(ValueError):
    """Raised before mutating a mismatched or unsafe workflow run."""


@dataclass(frozen=True)
class WorkflowBinding:
    run_id: str
    repository_id: str
    workflow: str
    workflow_version: str
    decision_boundary: str
    decision_owner: str

    def validate(self) -> None:
        values = asdict(self)
        missing = sorted(name for name, value in values.items() if not value.strip())
        if missing:
            raise WorkflowError(f"workflow binding fields are missing: {missing}")
        if self.workflow not in EXPECTED_WORKFLOWS:
            raise WorkflowError(f"unsupported workflow: {self.workflow}")
        if not RUN_ID.fullmatch(self.run_id):
            raise WorkflowError("run id contains unsupported path characters")

    def item(self) -> dict[str, str]:
        return {"id": "binding", **asdict(self)}


@dataclass(frozen=True)
class WorkflowSnapshot:
    run_id: str
    workflow: str
    workflow_version: str
    phase: str
    state_version: int
    next_actions: tuple[str, ...]
    open_gates: tuple[dict[str, Any], ...]


@dataclass(frozen=True)
class DecisionEvent:
    run_id: str
    workflow: str
    workflow_version: str
    owner: str
    choice: str
    outcome: str
    timestamp: str


def _executable(explicit: str | None) -> str:
    executable = explicit or os.environ.get("IX_FLOW_BIN") or shutil.which("ix-flow")
    if executable is None:
        raise WorkflowError("ix-flow is unavailable")
    return executable


def _invoke(
    arguments: list[str],
    state_dir: Path,
    *,
    ix_flow_bin: str | None = None,
) -> tuple[int, dict[str, Any]]:
    completed = subprocess.run(
        [
            _executable(ix_flow_bin),
            *arguments,
            "--state-dir",
            str(state_dir),
            "--json",
        ],
        check=False,
        capture_output=True,
        text=True,
    )
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise WorkflowError(
            f"ix-flow returned non-JSON output: {completed.stderr.strip()}"
        ) from error
    if not isinstance(payload, dict):
        raise WorkflowError("ix-flow returned a non-object response")
    return completed.returncode, payload


def _definition_version(workflow: str) -> str:
    definition = CANONICAL_SKILL / "workflows" / workflow / "def.yaml"
    if not definition.is_file():
        raise WorkflowError(f"canonical workflow is missing: {workflow}")
    document = yaml.safe_load(definition.read_text())
    version = document.get("version") if isinstance(document, dict) else None
    if not isinstance(version, str) or not version:
        raise WorkflowError(f"canonical workflow version is missing: {workflow}")
    return version


def _require_success(payload: dict[str, Any], operation: str) -> dict[str, Any]:
    if payload.get("ok") is not True:
        error = payload.get("error")
        raise WorkflowError(f"ix-flow {operation} failed: {error}")
    data = payload.get("data")
    if not isinstance(data, dict):
        raise WorkflowError(f"ix-flow {operation} returned no run data")
    return data


def _snapshot(payload: dict[str, Any]) -> WorkflowSnapshot:
    data = payload.get("data")
    if not isinstance(data, dict):
        raise WorkflowError("ix-flow response contains no run data")
    next_actions = payload.get("nextActions", [])
    open_gates = payload.get("open_gates", data.get("openGates", []))
    return WorkflowSnapshot(
        run_id=data["id"],
        workflow=data["defName"],
        workflow_version=data["defVersion"],
        phase=data["phase"],
        state_version=data["stateVersion"],
        next_actions=tuple(next_actions),
        open_gates=tuple(open_gates),
    )


def _assert_binding(data: dict[str, Any], binding: WorkflowBinding) -> None:
    if data.get("defName") != binding.workflow:
        raise WorkflowError("run id is bound to a different workflow")
    if data.get("defVersion") != binding.workflow_version:
        raise WorkflowError("run id is bound to a different workflow version")
    items = data.get("items", {}).get("run_binding", [])
    if len(items) != 1 or items[0] != binding.item():
        raise WorkflowError("run id is bound to a different repository or boundary")


def start_or_resume(
    binding: WorkflowBinding,
    state_dir: Path,
    *,
    ix_flow_bin: str | None = None,
) -> WorkflowSnapshot:
    """Create one bound ix-flow run or resume it without replacing state."""
    binding.validate()
    canonical_version = _definition_version(binding.workflow)
    if binding.workflow_version != canonical_version:
        raise WorkflowError(
            f"requested workflow version {binding.workflow_version} is not canonical {canonical_version}"
        )
    state_dir.mkdir(parents=True, exist_ok=True)
    _, status = _invoke(
        ["status", binding.run_id],
        state_dir,
        ix_flow_bin=ix_flow_bin,
    )
    if status.get("ok") is True:
        data = _require_success(status, "status")
        _assert_binding(data, binding)
        _, resumed = _invoke(
            ["resume", binding.run_id],
            state_dir,
            ix_flow_bin=ix_flow_bin,
        )
        _require_success(resumed, "resume")
        return _snapshot(resumed)
    if status.get("error", {}).get("code") != "instance_not_found":
        raise WorkflowError(f"ix-flow status failed: {status.get('error')}")

    _, created = _invoke(
        [
            "run",
            binding.workflow,
            "--path",
            str(CANONICAL_SKILL),
            "--id",
            binding.run_id,
        ],
        state_dir,
        ix_flow_bin=ix_flow_bin,
    )
    created_data = _require_success(created, "run")
    if created_data.get("defVersion") != binding.workflow_version:
        raise WorkflowError("created workflow version does not match its binding")
    _, bound = _invoke(
        [
            "add-item",
            binding.run_id,
            "run_binding",
            "--item",
            json.dumps(binding.item(), separators=(",", ":")),
        ],
        state_dir,
        ix_flow_bin=ix_flow_bin,
    )
    bound_data = _require_success(bound, "add binding")
    _assert_binding(bound_data, binding)
    return _snapshot(bound)


def _decision_event(
    data: dict[str, Any],
    binding: WorkflowBinding,
    choice: str,
) -> DecisionEvent:
    outcome = TERMINAL_PHASES[binding.workflow][choice]
    matching = [
        event
        for event in data.get("events", [])
        if event.get("kind") == "gate.acknowledged"
        and event.get("payload", {}).get("to") == outcome
        and event.get("payload", {}).get("approver") == binding.decision_owner
        and event.get("payload", {}).get("note") == choice
    ]
    if len(matching) != 1:
        raise WorkflowError("terminal decision does not have exactly one owner event")
    payload = matching[0]["payload"]
    return DecisionEvent(
        run_id=binding.run_id,
        workflow=binding.workflow,
        workflow_version=binding.workflow_version,
        owner=binding.decision_owner,
        choice=choice,
        outcome=outcome,
        timestamp=payload["acknowledgedAt"],
    )


def decide(
    binding: WorkflowBinding,
    state_dir: Path,
    choice: str | None,
    *,
    automatic: bool = False,
    ix_flow_bin: str | None = None,
) -> WorkflowSnapshot | DecisionEvent:
    """Leave a run at its gate or record one explicit owner decision."""
    if automatic:
        raise WorkflowError("automatic terminal-gate override is forbidden")
    snapshot = start_or_resume(
        binding,
        state_dir,
        ix_flow_bin=ix_flow_bin,
    )
    if choice is None:
        return snapshot
    if choice not in {"accept", "reject"}:
        raise WorkflowError(f"unsupported terminal choice: {choice}")
    outcome = TERMINAL_PHASES[binding.workflow][choice]
    opposite = TERMINAL_PHASES[binding.workflow][
        "reject" if choice == "accept" else "accept"
    ]
    if snapshot.phase == opposite:
        raise WorkflowError("run already has the opposite terminal outcome")
    if snapshot.phase == outcome:
        _, status = _invoke(
            ["status", binding.run_id],
            state_dir,
            ix_flow_bin=ix_flow_bin,
        )
        data = _require_success(status, "status")
        return _decision_event(data, binding, choice)
    if snapshot.phase != "decision_ready":
        raise WorkflowError(
            f"run is not at its terminal decision gate: {snapshot.phase}"
        )

    _, deferred = _invoke(
        ["advance", binding.run_id, outcome],
        state_dir,
        ix_flow_bin=ix_flow_bin,
    )
    if deferred.get("state") != "gate_deferred":
        raise WorkflowError("terminal transition did not defer to a human gate")
    gates = deferred.get("open_gates", [])
    selected = [gate for gate in gates if gate.get("to") == outcome]
    if len(selected) != 1:
        raise WorkflowError("terminal decision gate is missing or ambiguous")
    token = selected[0]["token"]
    _, acknowledged = _invoke(
        [
            "ack",
            binding.run_id,
            token,
            "--reviewer",
            binding.decision_owner,
            "--kind",
            "decision",
            "--note",
            choice,
        ],
        state_dir,
        ix_flow_bin=ix_flow_bin,
    )
    _require_success(acknowledged, "ack")
    _, advanced = _invoke(
        ["advance", binding.run_id, outcome],
        state_dir,
        ix_flow_bin=ix_flow_bin,
    )
    data = _require_success(advanced, "terminal advance")
    if data.get("phase") != outcome:
        raise WorkflowError("terminal advance retained the wrong outcome")
    return _decision_event(data, binding, choice)
