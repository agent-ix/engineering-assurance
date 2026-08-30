"""Evidence availability envelopes with immutable producer provenance."""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass
from typing import Any, Iterable

AVAILABILITY_STATES = frozenset(
    {"observed", "unavailable", "not_computed", "not_applicable"}
)
MUTABLE_VERSIONS = frozenset(
    {"latest", "main", "master", "head", "current", "nightly", "*"}
)
SHA256 = re.compile(r"^[0-9a-f]{64}$")
REQUIRED_GOVERNING_IDENTITIES = (
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


class EvidenceValidationError(ValueError):
    """Raised when evidence state or provenance is ambiguous."""


@dataclass(frozen=True)
class VersionIdentity:
    name: str
    version: str
    digest: str

    def errors(self) -> tuple[str, ...]:
        errors: list[str] = []
        if not self.name.strip():
            errors.append("identity-name-missing")
        version = self.version.strip().casefold()
        if not version:
            errors.append("identity-version-missing")
        elif version in MUTABLE_VERSIONS or any(
            marker in version for marker in (">", "<", "^", "~", "x")
        ):
            errors.append("identity-version-mutable")
        if not SHA256.fullmatch(self.digest):
            errors.append("identity-digest-not-sha256")
        return tuple(errors)


@dataclass(frozen=True)
class GoverningVersions:
    module: VersionIdentity
    plugin: VersionIdentity
    skill: VersionIdentity
    workflow: VersionIdentity
    quire: VersionIdentity
    quoin: VersionIdentity
    ix_flow: VersionIdentity
    schema: VersionIdentity
    producer: VersionIdentity

    def errors(self) -> tuple[str, ...]:
        errors: list[str] = []
        for field in REQUIRED_GOVERNING_IDENTITIES:
            identity = getattr(self, field, None)
            if not isinstance(identity, VersionIdentity):
                errors.append(f"{field}:identity-missing")
                continue
            errors.extend(f"{field}:{error}" for error in identity.errors())
        return tuple(errors)


@dataclass(frozen=True)
class OperatorObservation:
    command: tuple[str, ...]
    elapsed_ms: int
    exit_code: int | None
    outcome: str
    diagnostic_category: str | None = None

    def errors(self) -> tuple[str, ...]:
        errors: list[str] = []
        if not self.command or any(not item.strip() for item in self.command):
            errors.append("command-missing")
        if not isinstance(self.elapsed_ms, int) or self.elapsed_ms < 0:
            errors.append("elapsed-invalid")
        if self.outcome not in {"succeeded", "failed", "not-run"}:
            errors.append("outcome-invalid")
        if self.outcome == "not-run" and self.exit_code is not None:
            errors.append("not-run-has-exit-code")
        if self.outcome != "not-run" and not isinstance(self.exit_code, int):
            errors.append("exit-code-missing")
        if self.outcome == "failed" and not self.diagnostic_category:
            errors.append("failure-category-missing")
        return tuple(errors)


@dataclass(frozen=True)
class ProducerAttempt:
    producer_id: str
    applicable: bool
    invoked: bool
    observation: OperatorObservation
    governing: GoverningVersions | None = None
    output: dict[str, Any] | None = None
    output_valid: bool = False
    next_action: str | None = None
    owner: str | None = None
    boundary_rationale: str | None = None
    quoin_reference: str | None = None


@dataclass(frozen=True)
class EvidenceEnvelope:
    producer_id: str
    availability: str | None
    observation: OperatorObservation
    governing: GoverningVersions | None = None
    output_digest: str | None = None
    next_action: str | None = None
    owner: str | None = None
    boundary_rationale: str | None = None
    quoin_reference: str | None = None
    validation_errors: tuple[str, ...] = ()

    @property
    def valid(self) -> bool:
        return not self.validation_errors


def validate_state_labels(labels: Iterable[str]) -> str:
    """Return the one valid label or reject zero, duplicate, and conflict."""
    selected = tuple(labels)
    if len(selected) != 1:
        raise EvidenceValidationError(
            f"exactly one availability state is required: {selected}"
        )
    state = selected[0]
    if state not in AVAILABILITY_STATES:
        raise EvidenceValidationError(f"unsupported availability state: {state}")
    return state


def _output_digest(output: dict[str, Any]) -> str:
    encoded = json.dumps(
        output,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode()
    return hashlib.sha256(encoded).hexdigest()


def _invalid(attempt: ProducerAttempt, *errors: str) -> EvidenceEnvelope:
    return EvidenceEnvelope(
        producer_id=attempt.producer_id,
        availability=None,
        observation=attempt.observation,
        governing=attempt.governing,
        quoin_reference=attempt.quoin_reference,
        validation_errors=tuple(errors),
    )


def classify_producer(attempt: ProducerAttempt) -> EvidenceEnvelope:
    """Classify one producer without promoting invalid output to evidence."""
    if not attempt.producer_id.strip():
        return _invalid(attempt, "producer-id-missing")
    observation_errors = attempt.observation.errors()
    if observation_errors:
        return _invalid(attempt, *observation_errors)

    if not attempt.applicable:
        if not attempt.boundary_rationale:
            return _invalid(attempt, "boundary-rationale-missing")
        state = validate_state_labels(("not_applicable",))
        return EvidenceEnvelope(
            producer_id=attempt.producer_id,
            availability=state,
            observation=attempt.observation,
            boundary_rationale=attempt.boundary_rationale,
        )

    if not attempt.invoked:
        if not (attempt.next_action or attempt.owner):
            return _invalid(attempt, "next-action-or-owner-missing")
        state = validate_state_labels(("not_computed",))
        return EvidenceEnvelope(
            producer_id=attempt.producer_id,
            availability=state,
            observation=attempt.observation,
            next_action=attempt.next_action,
            owner=attempt.owner,
        )

    if attempt.observation.outcome == "failed":
        state = validate_state_labels(("unavailable",))
        return EvidenceEnvelope(
            producer_id=attempt.producer_id,
            availability=state,
            observation=attempt.observation,
        )

    if attempt.observation.outcome != "succeeded":
        return _invalid(attempt, "invoked-producer-has-invalid-outcome")
    if not attempt.output_valid or not isinstance(attempt.output, dict):
        return _invalid(attempt, "producer-output-malformed")
    if attempt.governing is None:
        return _invalid(attempt, "governing-versions-missing")
    governing_errors = attempt.governing.errors()
    if governing_errors:
        return _invalid(attempt, *governing_errors)
    if not attempt.quoin_reference or not attempt.quoin_reference.startswith(
        "ix://agent-ix/quoin/"
    ):
        return _invalid(attempt, "quoin-handoff-missing")

    state = validate_state_labels(("observed",))
    return EvidenceEnvelope(
        producer_id=attempt.producer_id,
        availability=state,
        observation=attempt.observation,
        governing=attempt.governing,
        output_digest=_output_digest(attempt.output),
        quoin_reference=attempt.quoin_reference,
    )
