"""Complete-only aggregation for the four-host onboarding evaluation suite."""

from __future__ import annotations

import re
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

from engineering_assurance.evidence import GoverningVersions, SHA256
from engineering_assurance.workflow import DecisionEvent

SUPPORTED_HOSTS = ("claude", "codex", "opencode", "copilot")
SCENARIO_VARIANTS = (
    "existing-profile",
    "no-profile",
    "malformed-producer",
    "unavailable-producer",
    "interruption-resume",
    "human-acceptance",
    "human-rejection",
)
EXPECTED_OUTCOMES = {
    "existing-profile": "reused",
    "no-profile": "no-applicable-work",
    "malformed-producer": "validation-failure",
    "unavailable-producer": "unavailable",
    "interruption-resume": "resumed",
    "human-acceptance": "accepted",
    "human-rejection": "rejected",
}
MUTABLE = re.compile(r"(?:^|[-_.])(latest|main|master|head|current)(?:$|[-_.])", re.I)


@dataclass(frozen=True)
class EvaluationCell:
    host: str
    scenario: str


@dataclass(frozen=True)
class HostProbe:
    host: str
    executable: str | None
    version: str | None
    available: bool
    diagnostic: str | None = None


@dataclass(frozen=True)
class EvaluationEnvelope:
    host: str
    scenario: str
    execution_status: str
    passed: bool
    suite_revision: str
    fixture_revision: str
    source_revision: str
    host_version: str | None = None
    governing: GoverningVersions | None = None
    transcript_path: str | None = None
    transcript_digest: str | None = None
    command_count: int | None = None
    elapsed_ms: int | None = None
    human_prompt_count: int | None = None
    manual_translation_count: int | None = None
    repeated_prompt_count: int | None = None
    observed_outcome: str | None = None
    terminal_event: DecisionEvent | None = None
    unsupported_additions: tuple[str, ...] = ()
    diagnostic: str | None = None

    @property
    def cell(self) -> EvaluationCell:
        return EvaluationCell(self.host, self.scenario)

    def errors(self) -> tuple[str, ...]:
        errors: list[str] = []
        if self.host not in SUPPORTED_HOSTS:
            errors.append("host-unsupported")
        if self.scenario not in SCENARIO_VARIANTS:
            errors.append("scenario-unsupported")
        for name, value in (
            ("suite", self.suite_revision),
            ("fixture", self.fixture_revision),
            ("source", self.source_revision),
        ):
            if not value.strip() or MUTABLE.search(value):
                errors.append(f"{name}-revision-not-immutable")
        if self.execution_status != "executed":
            errors.append("scenario-not-executed")
            if not self.diagnostic:
                errors.append("not-executed-diagnostic-missing")
            return tuple(errors)
        if not self.host_version or MUTABLE.search(self.host_version):
            errors.append("host-version-not-immutable")
        if self.governing is None:
            errors.append("governing-versions-missing")
        else:
            errors.extend(self.governing.errors())
        if (
            not self.transcript_path
            or Path(self.transcript_path).is_absolute()
            or ".." in Path(self.transcript_path).parts
        ):
            errors.append("transcript-path-invalid")
        if not self.transcript_digest or not SHA256.fullmatch(
            self.transcript_digest
        ):
            errors.append("transcript-digest-invalid")
        for name, value in (
            ("command", self.command_count),
            ("elapsed", self.elapsed_ms),
            ("human-prompt", self.human_prompt_count),
            ("manual-translation", self.manual_translation_count),
            ("repeated-prompt", self.repeated_prompt_count),
        ):
            if not isinstance(value, int) or value < 0:
                errors.append(f"{name}-count-invalid")
        expected = EXPECTED_OUTCOMES.get(self.scenario)
        if self.observed_outcome != expected:
            errors.append("observed-outcome-mismatch")
        if self.unsupported_additions:
            errors.append("unsupported-assurance-addition")

        terminal_choice = {
            "human-acceptance": "accept",
            "human-rejection": "reject",
        }.get(self.scenario)
        if terminal_choice is None:
            if self.terminal_event is not None:
                errors.append("unexpected-terminal-event")
        elif self.terminal_event is None:
            errors.append("terminal-event-missing")
        elif (
            self.terminal_event.choice != terminal_choice
            or self.terminal_event.outcome != expected
            or self.terminal_event.owner.strip() == ""
            or self.terminal_event.workflow_version.strip() == ""
            or self.terminal_event.run_id.strip() == ""
            or self.terminal_event.timestamp.strip() == ""
        ):
            errors.append("terminal-event-invalid")
        if not self.passed:
            errors.append("scenario-failed")
        return tuple(errors)


@dataclass(frozen=True)
class EvaluationAggregate:
    ok: bool
    required_cells: int
    complete_cells: int
    failures: tuple[str, ...]


def required_matrix() -> tuple[EvaluationCell, ...]:
    return tuple(
        EvaluationCell(host, scenario)
        for host in SUPPORTED_HOSTS
        for scenario in SCENARIO_VARIANTS
    )


def probe_host(host: str, *, search_path: str | None = None) -> HostProbe:
    if host not in SUPPORTED_HOSTS:
        raise ValueError(f"unsupported host: {host}")
    executable = shutil.which(host, path=search_path)
    if executable is None:
        return HostProbe(host, None, None, False, "executable-not-found")
    completed = subprocess.run(
        [executable, "--version"],
        check=False,
        capture_output=True,
        text=True,
        timeout=15,
    )
    output = (completed.stdout or completed.stderr).strip().splitlines()
    if completed.returncode != 0 or not output:
        return HostProbe(
            host,
            executable,
            None,
            False,
            f"version-probe-exit-{completed.returncode}",
        )
    return HostProbe(host, executable, output[0], True)


def aggregate_evaluations(
    envelopes: tuple[EvaluationEnvelope, ...] | list[EvaluationEnvelope],
) -> EvaluationAggregate:
    required = set(required_matrix())
    failures: list[str] = []
    grouped: dict[EvaluationCell, list[EvaluationEnvelope]] = {}
    for envelope in envelopes:
        grouped.setdefault(envelope.cell, []).append(envelope)
    missing = sorted(required - set(grouped), key=lambda item: (item.host, item.scenario))
    extra = sorted(set(grouped) - required, key=lambda item: (item.host, item.scenario))
    for cell in missing:
        failures.append(f"missing:{cell.host}:{cell.scenario}")
    for cell in extra:
        failures.append(f"extra:{cell.host}:{cell.scenario}")
    complete = 0
    for cell in sorted(required & set(grouped), key=lambda item: (item.host, item.scenario)):
        selected = grouped[cell]
        if len(selected) != 1:
            failures.append(f"duplicate:{cell.host}:{cell.scenario}")
            continue
        errors = selected[0].errors()
        if errors:
            failures.extend(
                f"{cell.host}:{cell.scenario}:{error}" for error in errors
            )
        else:
            complete += 1

    for host in SUPPORTED_HOSTS:
        decisions = {
            scenario: grouped.get(EvaluationCell(host, scenario), [])
            for scenario in ("human-acceptance", "human-rejection")
        }
        if all(len(values) == 1 for values in decisions.values()):
            acceptance = decisions["human-acceptance"][0].terminal_event
            rejection = decisions["human-rejection"][0].terminal_event
            if (
                acceptance is None
                or rejection is None
                or acceptance.choice != "accept"
                or rejection.choice != "reject"
                or acceptance.run_id == rejection.run_id
                or decisions["human-acceptance"][0].fixture_revision
                != decisions["human-rejection"][0].fixture_revision
                or decisions["human-acceptance"][0].source_revision
                != decisions["human-rejection"][0].source_revision
                or decisions["human-acceptance"][0].governing
                != decisions["human-rejection"][0].governing
            ):
                failures.append(f"{host}:terminal-pair-invalid")

    return EvaluationAggregate(
        ok=not failures and complete == len(required),
        required_cells=len(required),
        complete_cells=complete,
        failures=tuple(failures),
    )
