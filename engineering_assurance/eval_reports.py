"""Strict loading of retained CLI-agent reports into the evaluation matrix."""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from engineering_assurance.evaluation import (
    EvaluationAggregate,
    EvaluationEnvelope,
    aggregate_evaluations,
)
from engineering_assurance.evidence import (
    REQUIRED_GOVERNING_IDENTITIES,
    GoverningVersions,
    VersionIdentity,
)
from engineering_assurance.workflow import DecisionEvent


@dataclass(frozen=True)
class EvaluationReportCollection:
    """Retained successful cells plus diagnostics from every attempted run."""

    envelopes: tuple[EvaluationEnvelope, ...]
    failed_attempts: tuple[str, ...]
    errors: tuple[str, ...]
    models: tuple[tuple[str, str], ...]


def _text(value: Any) -> str:
    return value if isinstance(value, str) else ""


def _integer(value: Any) -> int | None:
    return value if isinstance(value, int) and not isinstance(value, bool) else None


def _governing(payload: Any) -> GoverningVersions:
    if not isinstance(payload, dict):
        raise ValueError("governing identities missing")
    identities: dict[str, VersionIdentity] = {}
    for name in REQUIRED_GOVERNING_IDENTITIES:
        value = payload.get(name)
        if not isinstance(value, dict):
            raise ValueError(f"governing identity missing: {name}")
        identities[name] = VersionIdentity(
            name=_text(value.get("name")),
            version=_text(value.get("version")),
            digest=_text(value.get("digest")),
        )
    return GoverningVersions(**identities)


def _terminal_event(payload: Any) -> DecisionEvent | None:
    if payload is None:
        return None
    if not isinstance(payload, dict):
        raise ValueError("terminal event is not an object")
    return DecisionEvent(
        run_id=_text(payload.get("run_id")),
        workflow=_text(payload.get("workflow")),
        workflow_version=_text(payload.get("workflow_version")),
        owner=_text(payload.get("owner")),
        choice=_text(payload.get("choice")),
        outcome=_text(payload.get("outcome")),
        timestamp=_text(payload.get("timestamp")),
    )


def _run_failure(host: str, scenario: str, run: dict[str, Any]) -> str:
    failures = run.get("failures")
    detail = (
        ", ".join(str(item) for item in failures) if isinstance(failures, list) else ""
    )
    return ":".join(
        part
        for part in (
            host,
            scenario,
            _text(run.get("exitReason")) or "unknown",
            detail or "unspecified failure",
        )
    )


def _transcript_error(run: dict[str, Any]) -> str | None:
    workdir_text = _text(run.get("workDir"))
    relative_text = _text(run.get("transcriptPath"))
    if not workdir_text or not relative_text:
        return "transcript-path-missing"
    workdir = Path(workdir_text)
    relative = Path(relative_text)
    if relative.is_absolute() or ".." in relative.parts:
        return "transcript-path-invalid"
    transcript = workdir / relative
    if not transcript.is_file():
        return "transcript-missing"
    observed = hashlib.sha256(transcript.read_bytes()).hexdigest()
    if observed != _text(run.get("transcriptDigest")):
        return "transcript-digest-mismatch"
    return None


def _envelope(
    report_path: Path,
    host: str,
    scenario: str,
    run: dict[str, Any],
    expected_source_revision: str,
) -> tuple[EvaluationEnvelope | None, tuple[str, ...]]:
    prefix = f"{report_path}:{host}:{scenario}"
    errors: list[str] = []
    if _text(run.get("exitReason")) != "complete":
        errors.append(f"{prefix}:execution-not-complete")
    transcript_error = _transcript_error(run)
    if transcript_error:
        errors.append(f"{prefix}:{transcript_error}")
    checks = run.get("checks")
    result = checks.get("evaluation_result") if isinstance(checks, dict) else None
    if not isinstance(result, dict):
        return None, (*errors, f"{prefix}:evaluation-result-missing")
    if _text(result.get("host")) != host:
        errors.append(f"{prefix}:host-mismatch")
    if _text(result.get("source_revision")) != expected_source_revision:
        errors.append(f"{prefix}:source-revision-mismatch")
    try:
        governing = _governing(result.get("governing"))
        terminal_event = _terminal_event(result.get("terminal_event"))
    except (TypeError, ValueError) as exc:
        return None, (*errors, f"{prefix}:{exc}")
    if errors:
        return None, tuple(errors)
    unsupported = result.get("unsupported_additions")
    unsupported_additions = (
        tuple(str(item) for item in unsupported)
        if isinstance(unsupported, list)
        else ("unsupported-additions-unreported",)
    )
    return (
        EvaluationEnvelope(
            host=host,
            scenario=scenario,
            execution_status="executed",
            passed=run.get("ok") is True,
            suite_revision=_text(result.get("suite_revision")),
            fixture_revision=_text(result.get("fixture_revision")),
            source_revision=_text(result.get("source_revision")),
            host_version=_text(result.get("host_version")),
            governing=governing,
            transcript_path=_text(run.get("transcriptPath")),
            transcript_digest=_text(run.get("transcriptDigest")),
            command_count=_integer(result.get("command_count")),
            elapsed_ms=_integer(result.get("elapsed_ms")),
            human_prompt_count=_integer(result.get("human_prompt_count")),
            manual_translation_count=_integer(result.get("manual_translation_count")),
            repeated_prompt_count=_integer(result.get("repeated_prompt_count")),
            observed_outcome=_text(result.get("observed_outcome")),
            terminal_event=terminal_event,
            unsupported_additions=unsupported_additions,
        ),
        (),
    )


def load_cli_eval_reports(
    report_paths: list[Path] | tuple[Path, ...],
    expected_source_revision: str,
) -> EvaluationReportCollection:
    """Load passing cells while retaining failed attempts as diagnostics."""
    envelopes: list[EvaluationEnvelope] = []
    failed_attempts: list[str] = []
    errors: list[str] = []
    models: dict[str, set[str]] = {}
    for report_path in report_paths:
        try:
            payload = json.loads(report_path.read_text())
        except (OSError, json.JSONDecodeError) as exc:
            errors.append(f"{report_path}:report-unreadable:{exc}")
            continue
        if not isinstance(payload, dict):
            errors.append(f"{report_path}:report-not-object")
            continue
        host = _text(payload.get("agent"))
        model = _text(payload.get("model"))
        if not host or not model:
            errors.append(f"{report_path}:host-or-model-missing")
            continue
        models.setdefault(host, set()).add(model)
        if payload.get("repeats") != 1:
            errors.append(f"{report_path}:repeat-count-not-one")
        results = payload.get("results")
        if not isinstance(results, list):
            errors.append(f"{report_path}:results-missing")
            continue
        for result in results:
            if not isinstance(result, dict):
                errors.append(f"{report_path}:{host}:result-not-object")
                continue
            scenario = _text(result.get("useCase"))
            runs = result.get("runs")
            if not scenario or not isinstance(runs, list) or len(runs) != 1:
                errors.append(f"{report_path}:{host}:{scenario}:run-count-not-one")
                continue
            run = runs[0]
            if not isinstance(run, dict):
                errors.append(f"{report_path}:{host}:{scenario}:run-not-object")
                continue
            if run.get("ok") is not True:
                failed_attempts.append(_run_failure(host, scenario, run))
                continue
            envelope, envelope_errors = _envelope(
                report_path,
                host,
                scenario,
                run,
                expected_source_revision,
            )
            errors.extend(envelope_errors)
            if envelope is not None:
                envelopes.append(envelope)
    for host, values in sorted(models.items()):
        if len(values) != 1:
            errors.append(f"{host}:model-mismatch:{','.join(sorted(values))}")
    return EvaluationReportCollection(
        envelopes=tuple(envelopes),
        failed_attempts=tuple(failed_attempts),
        errors=tuple(errors),
        models=tuple(
            (host, next(iter(values)))
            for host, values in sorted(models.items())
            if len(values) == 1
        ),
    )


def aggregate_report_collection(
    collection: EvaluationReportCollection,
) -> EvaluationAggregate:
    """Apply complete-only aggregation after report-level validation."""
    aggregate = aggregate_evaluations(collection.envelopes)
    failures = (*collection.errors, *aggregate.failures)
    return EvaluationAggregate(
        ok=not failures and aggregate.ok,
        required_cells=aggregate.required_cells,
        complete_cells=aggregate.complete_cells,
        failures=failures,
    )
