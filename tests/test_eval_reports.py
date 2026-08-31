"""Tests for assembling retained CLI-agent evaluation reports."""

from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

from engineering_assurance.eval_reports import (
    aggregate_report_collection,
    load_cli_eval_reports,
)
from engineering_assurance.evaluation import EXPECTED_OUTCOMES

DIGEST = "a" * 64
SOURCE_REVISION = "b" * 40


def _identity(name: str) -> dict[str, str]:
    return {"name": name, "version": "1.2.3", "digest": DIGEST}


def _governing() -> dict[str, dict[str, str]]:
    return {
        name: _identity(name)
        for name in (
            "module",
            "plugin",
            "skill",
            "workflow",
            "quire",
            "quoin",
            "ix_flow",
            "schema",
            "producer",
        )
    }


def _terminal_event(scenario: str) -> dict[str, str] | None:
    if scenario not in {"human-acceptance", "human-rejection"}:
        return None
    choice = "accept" if scenario == "human-acceptance" else "reject"
    return {
        "run_id": f"run-{choice}",
        "workflow": "architecture-evaluation",
        "workflow_version": "1.2.3",
        "owner": "architecture-owner",
        "choice": choice,
        "outcome": EXPECTED_OUTCOMES[scenario],
        "timestamp": "2026-08-30T00:00:00.000Z",
    }


def _successful_run(
    root: Path,
    host: str,
    scenario: str,
    *,
    source_revision: str = SOURCE_REVISION,
) -> dict[str, object]:
    workdir = root / f"{host}-{scenario}"
    workdir.mkdir()
    transcript = workdir / "EVAL_TRANSCRIPT.txt"
    transcript.write_text(f"{host}:{scenario}\n")
    transcript_digest = hashlib.sha256(transcript.read_bytes()).hexdigest()
    return {
        "ok": True,
        "exitReason": "complete",
        "failures": [],
        "latencyMs": 1200,
        "workDir": str(workdir),
        "transcriptPath": transcript.name,
        "transcriptDigest": transcript_digest,
        "checks": {
            "evaluation_result": {
                "revision": "evaluation-result-v1",
                "host": host,
                "host_version": "1.2.3",
                "source_revision": source_revision,
                "suite_revision": "suite-v1",
                "fixture_revision": "fixtures-v1",
                "governing": _governing(),
                "command_count": 4,
                "elapsed_ms": 1000,
                "human_prompt_count": 1 if _terminal_event(scenario) else 0,
                "manual_translation_count": 0,
                "repeated_prompt_count": 0,
                "observed_outcome": EXPECTED_OUTCOMES[scenario],
                "terminal_event": _terminal_event(scenario),
                "unsupported_additions": [],
            }
        },
    }


def _failed_run(root: Path, host: str, scenario: str) -> dict[str, object]:
    workdir = root / f"{host}-{scenario}-failed"
    workdir.mkdir()
    return {
        "ok": False,
        "exitReason": "timeout",
        "failures": ["result envelope unreadable"],
        "latencyMs": 480000,
        "workDir": str(workdir),
        "transcriptPath": "EVAL_TRANSCRIPT.txt",
        "transcriptDigest": None,
        "checks": {},
    }


def _write_report(
    path: Path,
    host: str,
    model: str | None,
    rows: list[tuple[str, dict[str, object]]],
) -> Path:
    payload = {
        "agent": host,
        "model": model,
        "repeats": 1,
        "ok": all(bool(run["ok"]) for _, run in rows),
        "results": [
            {
                "id": f"EA-{index:03d}",
                "useCase": scenario,
                "ok": run["ok"],
                "passRate": "1/1" if run["ok"] else "0/1",
                "runs": [run],
            }
            for index, (scenario, run) in enumerate(rows, start=1)
        ],
    }
    if model is None:
        payload.pop("model")
    path.write_text(json.dumps(payload))
    return path


class TestCliEvaluationReportLoading:
    """Verifies strict conversion of retained CLI reports into matrix cells."""

    def test_failed_attempt_can_be_replaced_by_one_targeted_success(
        self, tmp_path: Path
    ) -> None:
        """A failed run is retained diagnostically while one passing retry fills it."""
        full = _write_report(
            tmp_path / "full.json",
            "opencode",
            "opencode/mimo-v2.5-free",
            [
                (
                    "existing-profile",
                    _failed_run(tmp_path, "opencode", "existing-profile"),
                ),
                (
                    "no-profile",
                    _successful_run(tmp_path, "opencode", "no-profile"),
                ),
            ],
        )
        retry = _write_report(
            tmp_path / "retry.json",
            "opencode",
            "opencode/mimo-v2.5-free",
            [
                (
                    "existing-profile",
                    _successful_run(tmp_path, "opencode", "existing-profile"),
                )
            ],
        )

        collection = load_cli_eval_reports([full, retry], SOURCE_REVISION)

        assert not collection.errors
        assert len(collection.envelopes) == 2
        assert collection.failed_attempts == (
            "opencode:existing-profile:timeout:result envelope unreadable",
        )
        assert collection.models == (("opencode", "opencode/mimo-v2.5-free"),)

    def test_model_drift_is_rejected(self, tmp_path: Path) -> None:
        """One host cannot silently combine successful cells from two models."""
        first = _write_report(
            tmp_path / "first.json",
            "opencode",
            "model-a",
            [
                (
                    "existing-profile",
                    _successful_run(tmp_path, "opencode", "existing-profile"),
                )
            ],
        )
        second = _write_report(
            tmp_path / "second.json",
            "opencode",
            "model-b",
            [
                (
                    "no-profile",
                    _successful_run(tmp_path, "opencode", "no-profile"),
                )
            ],
        )

        collection = load_cli_eval_reports([first, second], SOURCE_REVISION)

        assert "opencode:model-mismatch:model-a,model-b" in collection.errors

    def test_omitted_model_records_runner_default_selection(
        self, tmp_path: Path
    ) -> None:
        """A host-default model is explicit configuration state, not missing evidence."""
        report = _write_report(
            tmp_path / "default-model.json",
            "codex",
            None,
            [
                (
                    "existing-profile",
                    _successful_run(tmp_path, "codex", "existing-profile"),
                )
            ],
        )

        collection = load_cli_eval_reports([report], SOURCE_REVISION)

        assert not collection.errors
        assert collection.models == (("codex", "runner-default"),)

    def test_source_and_transcript_drift_are_rejected(self, tmp_path: Path) -> None:
        """A stale source or changed retained transcript cannot enter the aggregate."""
        report = _write_report(
            tmp_path / "drifted.json",
            "codex",
            "codex-model",
            [
                (
                    "existing-profile",
                    _successful_run(
                        tmp_path,
                        "codex",
                        "existing-profile",
                        source_revision="c" * 40,
                    ),
                )
            ],
        )
        payload = json.loads(report.read_text())
        payload["results"][0]["runs"][0]["transcriptDigest"] = DIGEST
        report.write_text(json.dumps(payload))

        collection = load_cli_eval_reports([report], SOURCE_REVISION)

        assert any("source-revision-mismatch" in item for item in collection.errors)
        assert any("transcript-digest-mismatch" in item for item in collection.errors)

    def test_collection_preserves_complete_only_matrix_failure(
        self, tmp_path: Path
    ) -> None:
        """A partial report collection remains failed and names every missing cell."""
        report = _write_report(
            tmp_path / "partial.json",
            "claude",
            "claude-model",
            [
                (
                    "existing-profile",
                    _successful_run(tmp_path, "claude", "existing-profile"),
                )
            ],
        )

        aggregate = aggregate_report_collection(
            load_cli_eval_reports([report], SOURCE_REVISION)
        )

        assert not aggregate.ok
        assert aggregate.complete_cells == 1
        assert "missing:claude:no-profile" in aggregate.failures

    def test_aggregate_script_runs_directly(self) -> None:
        """The documented direct script entry point resolves the local package."""
        script = Path(__file__).parents[1] / "scripts/aggregate_agent_eval_reports.py"

        completed = subprocess.run(
            [sys.executable, str(script), "--help"],
            check=False,
            capture_output=True,
            text=True,
        )

        assert completed.returncode == 0, completed.stderr
        assert "--report" in completed.stdout
