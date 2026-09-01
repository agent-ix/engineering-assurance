"""Tests for the complete Engineering Assurance integration evidence gate."""

from __future__ import annotations

import hashlib
from pathlib import Path

import pytest

from scripts.check_integration_evidence import (
    _current_tool_failures,
    aggregate_metadata_failures,
    coverage_failures,
    governing_file_failures,
    parse_args,
    retained_report_paths,
    verify_aggregate,
)
from scripts.run_agent_evals import (
    command_identity,
    runtime_command_identity,
    search_path,
)


class TestTraceabilityEvidence:
    """Verifies fail-closed interpretation of Quire coverage evidence."""

    def test_complete_traceability_report_passes(self, tmp_path: Path) -> None:
        """
        Description:
            Accept complete machine-readable traceability evidence. TC-038.

        Assumptions:
            - Quire inspected the repository matrix and source symbols.

        Criteria:
            - All 68 test-case rows are backed.
            - Informational module diagnostics do not become repository gaps.
        """
        payload = {
            "unbacked_rows": [],
            "status_lies": [],
            "untracked_symbols": [],
            "diagnostics": [
                {
                    "reason": "archetype-matches-nothing",
                    "path": "/installed/module/manifest.yaml",
                },
                {
                    "reason": "catch-all-universal",
                    "path": str(tmp_path / "spec/FR-001.md"),
                },
            ],
            "groups": [
                {
                    "document": "spec/tests.md",
                    "target": "test-case",
                    "backed": 68,
                    "total": 68,
                }
            ],
            "totals": {"backed": 92, "total": 92},
        }

        assert coverage_failures(payload, tmp_path) == ()

    def test_incomplete_or_partially_read_report_fails(self, tmp_path: Path) -> None:
        """
        Description:
            Reject incomplete coverage and repository-local census diagnostics. TC-038.

        Assumptions:
            - A required NFR section did not match the authored document.

        Criteria:
            - Missing backing is reported.
            - A partial local census is reported instead of treated as complete.
        """
        payload = {
            "unbacked_rows": [{"row_id": "TC-049"}],
            "status_lies": [],
            "untracked_symbols": [],
            "diagnostics": [
                {
                    "reason": "section-matches-nothing",
                    "path": "spec/non-functional/NFR-001.md",
                }
            ],
            "groups": [],
            "totals": {"backed": 91, "total": 92},
        }

        failures = coverage_failures(payload, tmp_path)

        assert "traceability is 91/92, expected complete backing" in failures
        assert "unbacked rows: TC-049" in failures
        assert any("section-matches-nothing" in item for item in failures)
        assert "test-case traceability group is missing" in failures

    def test_schema_valid_status_column_diagnostic_is_compensated(
        self, tmp_path: Path
    ) -> None:
        """
        Description:
            Preserve the schema-owned TestMatrix header while checking markers locally.

        Assumptions:
            - The installed traceability declaration still requests `Status`.

        Criteria:
            - The known `Coverage Status` diagnostic is not misreported as a row gap.
            - Other completeness checks remain mandatory.
        """
        payload = {
            "unbacked_rows": [],
            "status_lies": [],
            "untracked_symbols": [],
            "diagnostics": [
                {
                    "reason": "status-column-matches-nothing",
                    "path": "spec/tests.md",
                }
            ],
            "groups": [
                {
                    "document": "spec/tests.md",
                    "target": "test-case",
                    "backed": 68,
                    "total": 68,
                }
            ],
            "totals": {"backed": 101, "total": 101},
        }

        assert coverage_failures(payload, tmp_path) == ()


class TestEvaluationAggregateEvidence:
    """Verifies retained evaluation and governing-file evidence."""

    def test_complete_aggregate_metadata_passes(self) -> None:
        """
        Description:
            Accept the complete-only four-host aggregate contract. TC-034, TC-039.

        Assumptions:
            - Report bytes and individual envelopes are checked separately.

        Criteria:
            - Exactly 28 of 28 passing cells are required.
            - The source revision and model selections remain explicit.
        """
        payload = {
            "revision": "evaluation-aggregate-v1",
            "source_revision": "a" * 40,
            "required_cells": 28,
            "complete_cells": 28,
            "ok": True,
            "failures": [],
            "models": {
                "claude": "model-a",
                "codex": "runner-default",
                "copilot": "runner-default",
                "opencode": "model-b",
            },
            "reports": [{"path": "evals/report.json", "digest": "b" * 64}],
            "failed_attempts": [],
        }

        assert aggregate_metadata_failures(payload) == ()

    def test_partial_or_ambiguous_aggregate_fails(self) -> None:
        """
        Description:
            Reject a partial aggregate or one with incomplete host selection. TC-034.

        Assumptions:
            - Twenty-seven successful cells cannot represent the required matrix.

        Criteria:
            - Cell-count drift is reported.
            - Missing host/model state is reported.
        """
        payload = {
            "revision": "evaluation-aggregate-v1",
            "source_revision": "a" * 40,
            "required_cells": 28,
            "complete_cells": 27,
            "ok": True,
            "failures": [],
            "models": {"claude": "model-a"},
            "reports": [],
            "failed_attempts": [],
        }

        failures = aggregate_metadata_failures(payload)

        assert "evaluation aggregate is 27/28, expected 28/28" in failures
        assert "evaluation model selection does not name the four hosts" in failures
        assert "evaluation aggregate names no retained reports" in failures

    def test_release_verifier_rejects_stale_source_revision(
        self, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
    ) -> None:
        """Trace: FR-006-AC-7, TC-051.

        Description:
            Reject release evidence retained for a different checkout. TC-051.

        Assumptions:
            - The aggregate contains a well-formed immutable source revision.
            - The current repository HEAD is a different immutable revision.

        Criteria:
            - FR-006-AC-7: post-run verification reports source-revision drift.
        """
        aggregate = tmp_path / "aggregate.json"
        aggregate.write_text(
            '{"revision":"evaluation-aggregate-v1",'
            '"source_revision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",'
            '"required_cells":28,"complete_cells":28,"ok":true,'
            '"failures":[],"models":{},"reports":[],"failed_attempts":[]}\n'
        )
        monkeypatch.setattr(
            "scripts.check_integration_evidence.repository_revision",
            lambda _root: "b" * 40,
        )

        failures = verify_aggregate(aggregate, tmp_path)

        assert (
            "evaluation source revision differs from current repository HEAD"
            in failures
        )

    def test_governing_file_digest_drift_fails(self, tmp_path: Path) -> None:
        """
        Description:
            Compare current governing bytes with retained evaluation identities. TC-032.

        Assumptions:
            - The retained identity records a lowercase SHA-256 digest.

        Criteria:
            - Unchanged bytes pass.
            - Changed bytes produce a named governing-file failure.
        """
        artifact = tmp_path / "SKILL.md"
        artifact.write_text("governed content\n")
        identity = {
            "name": "assurance-onboarding",
            "version": "0.2.0",
            "digest": hashlib.sha256(artifact.read_bytes()).hexdigest(),
        }

        assert governing_file_failures({"skill": identity}, {"skill": artifact}) == ()

        artifact.write_text("drifted content\n")

        assert governing_file_failures({"skill": identity}, {"skill": artifact}) == (
            "governing file digest changed: skill",
        )

    def test_post_run_verifier_detects_ix_flow_dist_runtime_drift(
        self, tmp_path: Path
    ) -> None:
        """
        Description:
            Reconcile retained ix-flow identity against the full current runtime. TC-050.

        Assumptions:
            - Every command resolves through the repository-local evaluation bin.
            - The ix-flow launcher and reported version remain unchanged.

        Criteria:
            - FR-006-AC-6: unchanged governing tools pass reconciliation.
            - A dist-only ix-flow change produces the named ix_flow failure.
        """
        local_bin = tmp_path / ".agent-evals" / "bin"
        local_bin.mkdir(parents=True)

        for command in ("quire", "quoin"):
            executable = local_bin / command
            executable.write_text(f"#!/bin/sh\nprintf '%s\\n' '{command}/1.0.0'\n")
            executable.chmod(0o755)

        runtime_files: dict[str, Path] = {}
        for command in ("ix-flow", "cli-evals"):
            package = tmp_path / "tools" / command
            executable = package / "bin" / command
            runtime = package / "dist" / "runtime.js"
            executable.parent.mkdir(parents=True)
            runtime.parent.mkdir()
            (package / "package.json").write_text(
                f'{{"name":"{command}","version":"1.0.0"}}\n'
            )
            executable.write_text(f"#!/bin/sh\nprintf '%s\\n' '{command}/1.0.0'\n")
            executable.chmod(0o755)
            runtime.write_text("export const behavior = 'before';\n")
            (local_bin / command).symlink_to(executable)
            runtime_files[command] = runtime

        selected_path = search_path("", local_bin=local_bin)
        identities = {
            "quire": command_identity("quire", search_path_value=selected_path),
            "quoin": command_identity("quoin", search_path_value=selected_path),
            "ix_flow": runtime_command_identity(
                "ix-flow", search_path_value=selected_path
            ),
            "producer": runtime_command_identity(
                "cli-evals", search_path_value=selected_path
            ),
        }

        assert _current_tool_failures(identities, tmp_path) == ()

        runtime_files["ix-flow"].write_text("export const behavior = 'after';\n")

        assert _current_tool_failures(identities, tmp_path) == (
            "governing executable identity changed: ix_flow",
        )

    def test_retained_report_symlink_escape_fails(self, tmp_path: Path) -> None:
        """
        Description:
            Reject a retained-report path that resolves outside the repository.

        Assumptions:
            - A repository-relative directory is replaced with a symlink.

        Criteria:
            - TC-032: report-byte verification never reads outside its root.
        """
        outside = tmp_path.parent / f"{tmp_path.name}-outside"
        outside.mkdir()
        report = outside / "retained.json"
        report.write_text("{}\n")
        (tmp_path / "reports").symlink_to(outside, target_is_directory=True)
        aggregate = {
            "reports": [
                {
                    "path": "reports/retained.json",
                    "digest": hashlib.sha256(report.read_bytes()).hexdigest(),
                }
            ]
        }

        paths, failures = retained_report_paths(aggregate, tmp_path)

        assert paths == ()
        assert failures == (
            "retained report path escapes the repository: reports/retained.json",
        )


class TestIntegrationEvidenceCommand:
    """Verifies explicit selection of tracked and operational evidence modes."""

    def test_requires_an_explicit_evidence_mode(self) -> None:
        """
        Description:
            Keep real-agent release evidence distinct from repository traceability.

        Assumptions:
            - Operational reports are intentionally not tracked in the repository.

        Criteria:
            - TC-038: omitting the evidence mode fails closed.
            - Traceability-only mode does not invent an aggregate path.
            - Release mode retains the caller-supplied aggregate path.
        """
        with pytest.raises(SystemExit):
            parse_args([])

        traceability = parse_args(["--traceability-only"])
        release = parse_args(["--aggregate", "evals/reports/aggregate.json"])

        assert traceability.traceability_only is True
        assert traceability.aggregate is None
        assert release.traceability_only is False
        assert release.aggregate == Path("evals/reports/aggregate.json")
