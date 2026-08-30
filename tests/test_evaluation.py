from __future__ import annotations

import json
import subprocess
from dataclasses import replace
from pathlib import Path

import pytest

from engineering_assurance.evaluation import (
    SCENARIO_VARIANTS,
    SUPPORTED_HOSTS,
    EvaluationEnvelope,
    aggregate_evaluations,
    probe_host,
    required_matrix,
)
from engineering_assurance.evidence import GoverningVersions, VersionIdentity
from engineering_assurance.workflow import DecisionEvent

DIGEST = "a" * 64


def governing() -> GoverningVersions:
    def identity(name: str) -> VersionIdentity:
        return VersionIdentity(name, "1.2.3", DIGEST)

    return GoverningVersions(
        module=identity("engineering-assurance"),
        plugin=identity("engineering-assurance-plugin"),
        skill=identity("assurance-onboarding"),
        workflow=identity("architecture-evaluation"),
        quire=identity("quire"),
        quoin=identity("quoin"),
        ix_flow=identity("ix-flow"),
        schema=identity("evaluation-envelope"),
        producer=identity("cli-agent-evals"),
    )


def envelope(host: str, scenario: str) -> EvaluationEnvelope:
    terminal = None
    if scenario in {"human-acceptance", "human-rejection"}:
        choice = "accept" if scenario == "human-acceptance" else "reject"
        terminal = DecisionEvent(
            run_id=f"{host}-{choice}-run",
            workflow="architecture-evaluation",
            workflow_version="0.1.0",
            owner="architecture-owner",
            choice=choice,
            outcome="accepted" if choice == "accept" else "rejected",
            timestamp="2026-08-30T00:00:00.000Z",
        )
    outcomes = {
        "existing-profile": "reused",
        "no-profile": "no-applicable-work",
        "malformed-producer": "validation-failure",
        "unavailable-producer": "unavailable",
        "interruption-resume": "resumed",
        "human-acceptance": "accepted",
        "human-rejection": "rejected",
    }
    return EvaluationEnvelope(
        host=host,
        scenario=scenario,
        execution_status="executed",
        passed=True,
        suite_revision="suite-v1",
        fixture_revision="fixtures-v1",
        source_revision="a" * 40,
        host_version="1.2.3",
        governing=governing(),
        transcript_path=f"transcripts/{host}-{scenario}.jsonl",
        transcript_digest=DIGEST,
        command_count=4,
        elapsed_ms=1200,
        human_prompt_count=1 if terminal else 0,
        manual_translation_count=0,
        repeated_prompt_count=0,
        observed_outcome=outcomes[scenario],
        terminal_event=terminal,
    )


def complete_matrix() -> list[EvaluationEnvelope]:
    return [envelope(cell.host, cell.scenario) for cell in required_matrix()]


def test_suite_matrix_is_seven_variants_across_four_hosts() -> None:
    """Trace: FR-006-AC-1, TC-031."""
    matrix = required_matrix()
    assert len(matrix) == 28
    assert {cell.host for cell in matrix} == set(SUPPORTED_HOSTS)
    assert {cell.scenario for cell in matrix} == set(SCENARIO_VARIANTS)
    fixture = json.loads(
        (Path(__file__).parents[1] / "evals/fixtures/suite.json").read_text()
    )
    assert set(fixture["scenarios"]) == set(SCENARIO_VARIANTS)


def test_suite_config_loads_without_a_project_local_runner_package() -> None:
    """Trace: FR-006-AC-1, TC-031."""
    config = Path(__file__).parents[1] / "evals/cli-agent-evals.config.mjs"
    script = (
        "import(process.argv[1]).then(m => console.log(m.default.scenarios.length))"
    )
    completed = subprocess.run(
        ["node", "-e", script, config.as_uri()],
        check=False,
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr
    assert completed.stdout.strip() == "7"


def test_live_suite_publishes_the_exact_result_contract() -> None:
    """Trace: FR-006-AC-2, TC-032."""
    contract = Path(__file__).parents[1] / "evals/result-contract.mjs"
    script = """
      import(process.argv[1]).then(m => {
        const expectation = { expected: 'reused' };
        const identity = { name: 'fixed', version: '1.0.0', digest: 'a'.repeat(64) };
        const governing = Object.fromEntries(
          m.governingIdentities.map(name => [name, { ...identity, name }]),
        );
        const result = m.resultContract(expectation, {
          host: 'codex',
          host_version: 'codex-cli 1.0.0',
          source_revision: 'b'.repeat(40),
          suite_revision: 'suite-v1',
          fixture_revision: 'fixtures-v1',
          governing,
        });
        const envelope = {
          host: result.host,
          host_version: result.host_version,
          source_revision: result.source_revision,
          suite_revision: result.suite_revision,
          fixture_revision: result.fixture_revision,
          governing: result.governing,
          command_count: 1,
          elapsed_ms: 2,
          human_prompt_count: 0,
          manual_translation_count: 0,
          repeated_prompt_count: 0,
          observed_outcome: result.observed_outcome,
          terminal_event: null,
          unsupported_additions: [],
        };
        const valid = m.validateResult(envelope, result, 'complete');
        const drifted = m.validateResult(
          { ...envelope, source_revision: 'c'.repeat(40) },
          result,
          'complete',
        );
        console.log(JSON.stringify({ result, valid, drifted }));
      })
    """
    completed = subprocess.run(
        ["node", "-e", script, contract.as_uri()],
        check=False,
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr
    observed = json.loads(completed.stdout)
    result = observed["result"]
    assert result["observed_outcome"] == "reused"
    assert result["terminal_event_contract"] == {"required": False, "value": None}
    assert result["governing_identities"] == [
        "module",
        "plugin",
        "skill",
        "workflow",
        "quire",
        "quoin",
        "ix_flow",
        "schema",
        "producer",
    ]
    assert observed["valid"] == []
    assert observed["drifted"] == ["result field mismatch: source_revision"]


def test_live_suite_requires_the_complete_terminal_event_shape() -> None:
    """Trace: FR-006-AC-2, FR-006-AC-5, TC-032, TC-049."""
    contract = Path(__file__).parents[1] / "evals/result-contract.mjs"
    script = """
      import(process.argv[1]).then(m => {
        const identity = { name: 'fixed', version: '1.0.0', digest: 'a'.repeat(64) };
        const governing = Object.fromEntries(
          m.governingIdentities.map(name => [name, { ...identity, name }]),
        );
        governing.workflow = { ...identity, name: 'architecture-evaluation' };
        const expectation = {
          expected: 'accepted',
          choice: 'accept',
          input: { decision_owner: 'juniper-architecture-owner' },
        };
        const contract = m.resultContract(expectation, {
          host: 'copilot',
          host_version: 'copilot 1.0.0',
          source_revision: 'b'.repeat(40),
          suite_revision: 'suite-v1',
          fixture_revision: 'fixtures-v1',
          governing,
        });
        const terminal = {
          run_id: 'accept-run',
          workflow: 'architecture-evaluation',
          workflow_version: '1.0.0',
          owner: 'juniper-architecture-owner',
          choice: 'accept',
          outcome: 'accepted',
          timestamp: '2026-08-30T22:00:00Z',
        };
        const envelope = {
          host: contract.host,
          host_version: contract.host_version,
          source_revision: contract.source_revision,
          suite_revision: contract.suite_revision,
          fixture_revision: contract.fixture_revision,
          governing: contract.governing,
          command_count: 1,
          elapsed_ms: 2,
          human_prompt_count: 0,
          manual_translation_count: 0,
          repeated_prompt_count: 0,
          observed_outcome: contract.observed_outcome,
          terminal_event: terminal,
          unsupported_additions: [],
        };
        console.log(JSON.stringify({
          contract,
          valid: m.validateResult(envelope, contract, 'complete'),
          missing: m.validateResult(
            { ...envelope, terminal_event: 'decision_ready->accepted' },
            contract,
            'complete',
          ),
        }));
      })
    """
    completed = subprocess.run(
        ["node", "-e", script, contract.as_uri()],
        check=False,
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr
    observed = json.loads(completed.stdout)
    terminal = observed["contract"]["terminal_event_contract"]
    assert terminal["required"] is True
    assert terminal["choice"] == "accept"
    assert terminal["owner"] == "juniper-architecture-owner"
    assert observed["valid"] == []
    assert observed["missing"] == ["explicit terminal event missing"]


def test_complete_envelope_retains_versions_transcript_effort_and_outcome() -> None:
    """Trace: FR-006-AC-2, TC-032."""
    result = envelope("claude", "existing-profile")
    assert result.errors() == ()
    assert result.governing == governing()
    assert result.transcript_digest == DIGEST
    assert result.command_count == 4
    assert result.elapsed_ms == 1200
    assert result.observed_outcome == "reused"


@pytest.mark.parametrize(
    "changes, expected",
    [
        ({"governing": None}, "governing-versions-missing"),
        ({"transcript_digest": None}, "transcript-digest-invalid"),
        ({"command_count": None}, "command-count-invalid"),
        ({"observed_outcome": "invented"}, "observed-outcome-mismatch"),
    ],
)
def test_incomplete_envelope_fails_validation(changes: dict, expected: str) -> None:
    """Trace: FR-006-AC-2, TC-032."""
    assert (
        expected in replace(envelope("claude", "existing-profile"), **changes).errors()
    )


def test_missing_executable_is_not_executed_and_fails_gate(tmp_path: Path) -> None:
    """Trace: FR-006-AC-3, TC-033."""
    probe = probe_host("opencode", search_path=str(tmp_path))
    assert probe.available is False
    missing = replace(
        envelope("opencode", "existing-profile"),
        execution_status="not_executed",
        passed=False,
        host_version=None,
        governing=None,
        transcript_path=None,
        transcript_digest=None,
        command_count=None,
        elapsed_ms=None,
        human_prompt_count=None,
        manual_translation_count=None,
        repeated_prompt_count=None,
        observed_outcome=None,
        diagnostic=probe.diagnostic,
    )
    matrix = complete_matrix()
    index = next(
        index
        for index, item in enumerate(matrix)
        if item.host == "opencode" and item.scenario == "existing-profile"
    )
    matrix[index] = missing
    result = aggregate_evaluations(matrix)
    assert result.ok is False
    assert any("scenario-not-executed" in item for item in result.failures)


def test_aggregate_passes_only_for_28_complete_unique_cells() -> None:
    """Trace: FR-006-AC-4, TC-034."""
    complete = complete_matrix()
    result = aggregate_evaluations(complete)
    assert result.ok
    assert result.complete_cells == result.required_cells == 28
    assert not aggregate_evaluations(complete[:-1]).ok
    assert not aggregate_evaluations(complete + [complete[0]]).ok
    assert not aggregate_evaluations(
        [replace(complete[0], passed=False), *complete[1:]]
    ).ok


def test_unsupported_assurance_outcome_fails_aggregate() -> None:
    """Trace: NFR-002, TC-039."""
    complete = complete_matrix()
    changed = replace(
        complete[0],
        unsupported_additions=("unrequested AssuranceProfile",),
    )
    result = aggregate_evaluations([changed, *complete[1:]])
    assert not result.ok
    assert any("unsupported-assurance-addition" in item for item in result.failures)


def test_every_host_retains_distinct_acceptance_and_rejection_events() -> None:
    """Trace: FR-006-AC-5, TC-049."""
    complete = complete_matrix()
    result = aggregate_evaluations(complete)
    assert result.ok
    for host in SUPPORTED_HOSTS:
        selected = [
            item
            for item in complete
            if item.host == host and item.terminal_event is not None
        ]
        assert {item.terminal_event.choice for item in selected} == {
            "accept",
            "reject",
        }
        assert len({item.terminal_event.run_id for item in selected}) == 2
