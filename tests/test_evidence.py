from __future__ import annotations

from dataclasses import replace

import pytest

from engineering_assurance.evidence import (
    AVAILABILITY_STATES,
    EvidenceValidationError,
    GoverningVersions,
    OperatorObservation,
    ProducerAttempt,
    VersionIdentity,
    classify_producer,
    validate_state_labels,
)

DIGEST = "a" * 64


def identity(name: str, version: str = "1.2.3") -> VersionIdentity:
    return VersionIdentity(name, version, DIGEST)


def governing() -> GoverningVersions:
    return GoverningVersions(
        module=identity("engineering-assurance"),
        plugin=identity("engineering-assurance-plugin"),
        skill=identity("assurance-onboarding"),
        workflow=identity("assurance-intake"),
        quire=identity("quire"),
        quoin=identity("quoin"),
        ix_flow=identity("ix-flow"),
        schema=identity("producer-output-v1"),
        producer=identity("fictional-producer"),
    )


def observation(
    outcome: str = "succeeded",
    exit_code: int | None = 0,
    diagnostic: str | None = None,
) -> OperatorObservation:
    return OperatorObservation(
        command=("fictional-producer", "--json"),
        elapsed_ms=17,
        exit_code=exit_code,
        outcome=outcome,
        diagnostic_category=diagnostic,
    )


def observed_attempt() -> ProducerAttempt:
    return ProducerAttempt(
        producer_id="fictional-producer",
        applicable=True,
        invoked=True,
        observation=observation(),
        governing=governing(),
        output={"valid": True, "count": 3},
        output_valid=True,
        quoin_reference="ix://agent-ix/quoin/EvidenceRecord-001",
    )


def test_valid_output_is_observed_with_complete_provenance() -> None:
    """Trace: FR-004-AC-1, TC-020."""
    result = classify_producer(observed_attempt())
    assert result.valid
    assert result.availability == "observed"
    assert result.governing == governing()
    assert result.output_digest is not None
    assert result.observation.command == ("fictional-producer", "--json")
    assert result.observation.elapsed_ms == 17
    assert result.observation.exit_code == 0


def test_invocation_failure_is_unavailable_with_failure_category() -> None:
    """Trace: FR-004-AC-2, TC-021."""
    result = classify_producer(
        replace(
            observed_attempt(),
            observation=observation("failed", 127, "executable-not-found"),
            output=None,
            output_valid=False,
            governing=None,
            quoin_reference=None,
        )
    )
    assert result.valid
    assert result.availability == "unavailable"
    assert result.observation.diagnostic_category == "executable-not-found"


def test_deferred_producer_names_next_action_or_owner() -> None:
    """Trace: FR-004-AC-3, TC-022."""
    result = classify_producer(
        ProducerAttempt(
            producer_id="fictional-producer",
            applicable=True,
            invoked=False,
            observation=observation("not-run", None),
            owner="measurement-owner",
        )
    )
    assert result.valid
    assert result.availability == "not_computed"
    assert result.owner == "measurement-owner"


def test_excluded_producer_retains_boundary_rationale() -> None:
    """Trace: FR-004-AC-4, TC-023."""
    result = classify_producer(
        ProducerAttempt(
            producer_id="fictional-producer",
            applicable=False,
            invoked=False,
            observation=observation("not-run", None),
            boundary_rationale="outside the selected service boundary",
        )
    )
    assert result.valid
    assert result.availability == "not_applicable"
    assert result.boundary_rationale


@pytest.mark.parametrize(
    "attempt, error",
    [
        (replace(observed_attempt(), output_valid=False), "producer-output-malformed"),
        (replace(observed_attempt(), governing=None), "governing-versions-missing"),
        (
            replace(
                observed_attempt(),
                governing=replace(
                    governing(),
                    producer=identity("fictional-producer", "latest"),
                ),
            ),
            "producer:identity-version-mutable",
        ),
    ],
)
def test_malformed_or_mutable_provenance_is_not_observed(
    attempt: ProducerAttempt,
    error: str,
) -> None:
    """Trace: FR-004-AC-5, TC-024."""
    result = classify_producer(attempt)
    assert not result.valid
    assert result.availability is None
    assert error in result.validation_errors


def test_observed_evidence_requires_quoin_handoff() -> None:
    """Trace: FR-004-AC-6, TC-025."""
    invalid = classify_producer(replace(observed_attempt(), quoin_reference=None))
    assert invalid.availability is None
    assert invalid.validation_errors == ("quoin-handoff-missing",)
    valid = classify_producer(observed_attempt())
    assert valid.quoin_reference == "ix://agent-ix/quoin/EvidenceRecord-001"


@pytest.mark.parametrize(
    "labels",
    [
        (),
        ("observed", "observed"),
        ("observed", "unavailable"),
        ("unknown",),
    ],
)
def test_zero_duplicate_conflicting_or_unknown_states_fail(
    labels: tuple[str, ...],
) -> None:
    """Trace: FR-004-AC-7, TC-046."""
    with pytest.raises(EvidenceValidationError):
        validate_state_labels(labels)


def test_every_declared_state_is_individually_valid() -> None:
    """Trace: FR-004-AC-7, TC-046."""
    for state in AVAILABILITY_STATES:
        assert validate_state_labels((state,)) == state
