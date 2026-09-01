"""Read-only verification semantics, compatibility mappings, and projections.

This module never executes a producer and never writes an evidence record.  It
validates references to authoritative records and derives review-only views.
"""

from __future__ import annotations

import hashlib
import json
from copy import deepcopy
from pathlib import Path
from typing import Any, Iterable, Mapping

from jsonschema import Draft7Validator, FormatChecker

from engineering_assurance import PACKAGE_ROOT

CONTRACT_ROOT = PACKAGE_ROOT / "contracts"
SCHEMA_ROOT = PACKAGE_ROOT / "schemas"
OWNERSHIP_PATH = CONTRACT_ROOT / "verification-semantics-ownership-v1.json"

CONCEPT_AUTHORITIES = {
    "verification_definition": "quire",
    "verification_execution": "native_producer",
    "check_result": "native_producer",
    "evidence": "quoin",
    "measurement": "quoin",
    "diagnostic": "originating_producer",
    "report": "quoin",
    "human_decision": "ix_flow",
}

PRODUCER_CONCEPTS = {
    "verification_execution",
    "check_result",
    "evidence",
    "measurement",
    "diagnostic",
}

REQUIRED_LINKS = {
    "verification_execution": {"definition"},
    "check_result": {"definition", "execution"},
    "evidence": {"result"},
    "measurement": {"measurement_plan", "evidence"},
    "report": {"evidence"},
    "human_decision": {"decision_subject"},
}

# Only links between semantic references are resolved inside a bundle.  A
# MeasurementPlan is an Engineering Assurance definition with its own identity,
# so measurement_plan deliberately remains an external authoritative link.
INTERNAL_LINKS = {
    "definition",
    "execution",
    "result",
    "evidence",
    "report",
    "decision_subject",
}

LINK_TARGET_CONCEPTS = {
    "definition": "verification_definition",
    "execution": "verification_execution",
    "result": "check_result",
    "evidence": "evidence",
    "report": "report",
    "decision_subject": "report",
}

PGM01_STATE_MAP = {
    "pass": "passed",
    "passed": "passed",
    "fail": "failed",
    "failed": "failed",
    "error": "error",
    "skipped": "skipped",
    "inconclusive": "inconclusive",
    "unavailable": "unavailable",
}


class SemanticContractError(ValueError):
    """A read-only semantic projection is invalid or incompatible."""


def _load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _schema(name: str) -> Mapping[str, Any]:
    return _load_json(SCHEMA_ROOT / name)


def _validate(instance: Any, schema_name: str) -> None:
    validator = Draft7Validator(_schema(schema_name), format_checker=FormatChecker())
    errors = sorted(
        validator.iter_errors(instance),
        key=lambda error: (tuple(str(part) for part in error.path), error.message),
    )
    if errors:
        location = "/" + "/".join(str(part) for part in errors[0].path)
        raise SemanticContractError(f"{schema_name}{location}: {errors[0].message}")


def load_ownership_registry() -> dict[str, Any]:
    """Load and validate the packaged ownership registry."""
    registry = _load_json(OWNERSHIP_PATH)
    _validate(registry, "verification-semantics-ownership-v1.schema.json")
    concepts = registry["concepts"]
    by_name = {item["concept"]: item for item in concepts}
    if len(by_name) != len(concepts):
        raise SemanticContractError("ownership registry repeats a semantic concept")
    if set(by_name) != set(CONCEPT_AUTHORITIES):
        raise SemanticContractError("ownership registry has an incomplete concept set")
    for concept, authority in CONCEPT_AUTHORITIES.items():
        if by_name[concept]["authority"] != authority:
            raise SemanticContractError(
                f"{concept} authority must be {authority}, not "
                f"{by_name[concept]['authority']}"
            )
    if registry["non_executing"] is not True:
        raise SemanticContractError("ownership registry must remain non-executing")
    return registry


def validate_semantic_reference(reference: Mapping[str, Any]) -> dict[str, Any]:
    """Validate one non-persisted reference to an authoritative record."""
    value = deepcopy(dict(reference))
    _validate(value, "semantic-reference-v1.schema.json")
    concept = value["concept"]
    expected_authority = CONCEPT_AUTHORITIES[concept]
    if value["authority"] != expected_authority:
        raise SemanticContractError(f"{concept} authority must be {expected_authority}")
    if concept in PRODUCER_CONCEPTS and "producer" not in value:
        raise SemanticContractError(f"{concept} requires the complete producer tuple")
    missing_links = REQUIRED_LINKS.get(concept, set()) - set(value["links"])
    if missing_links:
        raise SemanticContractError(
            f"{concept} is missing links: {sorted(missing_links)}"
        )
    if value["semantic_id"] in value["links"].values():
        raise SemanticContractError("a semantic reference cannot link to itself")
    return value


def validate_semantic_bundle(
    references: Iterable[Mapping[str, Any]],
) -> list[dict[str, Any]]:
    """Validate cross-links among an in-memory set of semantic references."""
    values = [validate_semantic_reference(reference) for reference in references]
    identifiers = [value["semantic_id"] for value in values]
    if len(set(identifiers)) != len(identifiers):
        raise SemanticContractError("semantic identifiers must be distinct")
    known = {value["semantic_id"]: value for value in values}
    for value in values:
        internal_links = {
            relationship: target
            for relationship, target in value["links"].items()
            if relationship in INTERNAL_LINKS
        }
        missing = set(internal_links.values()) - set(known)
        if missing:
            raise SemanticContractError(
                f"{value['semantic_id']} has missing references: {sorted(missing)}"
            )
        for relationship, target in internal_links.items():
            expected = LINK_TARGET_CONCEPTS[relationship]
            actual = known[target]["concept"]
            if actual != expected:
                raise SemanticContractError(
                    f"{value['semantic_id']} link {relationship} must target "
                    f"{expected}, not {actual}"
                )
    return values


def validate_compatibility_fixture(fixture: Mapping[str, Any]) -> dict[str, Any]:
    """Validate exact source-version premises and their semantic references."""
    value = deepcopy(dict(fixture))
    _validate(value, "verification-semantics-fixture-v1.schema.json")
    references = validate_semantic_bundle(value["references"])
    premises = {
        (
            premise["concept"],
            premise["schema_identity"],
            premise["schema_version"],
        )
        for premise in value["source_version_premises"]
    }
    if len(premises) != len(value["source_version_premises"]):
        raise SemanticContractError("source-version premises must be distinct")
    observed = {
        (
            reference["concept"],
            reference["source"]["schema_identity"],
            reference["source"]["schema_version"],
        )
        for reference in references
    }
    if observed != premises:
        missing = sorted(observed - premises)
        unused = sorted(premises - observed)
        raise SemanticContractError(
            f"source-version premises differ: missing={missing}, unused={unused}"
        )
    value["references"] = references
    return value


def validate_report_projection(report: Mapping[str, Any]) -> dict[str, Any]:
    """Validate a bounded report view with no inferred overall verdict."""
    value = deepcopy(dict(report))
    _validate(value, "assurance-report-projection-v1.schema.json")
    forbidden = {"trust_score", "overall_score", "overall_verdict", "approved"}
    overlap = forbidden & set(value)
    if overlap:
        raise SemanticContractError(
            f"report contains forbidden verdicts: {sorted(overlap)}"
        )
    return value


def render_report_json(report: Mapping[str, Any]) -> str:
    """Render a deterministic JSON projection without changing its semantics."""
    value = validate_report_projection(report)
    return json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n"


def _markdown_text(value: Any) -> str:
    return " ".join(str(value).splitlines()).replace("|", "\\|")


def render_report_markdown(report: Mapping[str, Any]) -> str:
    """Render the bounded claims/evidence/counterevidence/gaps report shape."""
    value = validate_report_projection(report)
    lines = [f"# Assurance report: {_markdown_text(value['subject'])}", ""]
    lines.extend(["## Claims", ""])
    lines.extend(
        f"- `{item['claim_id']}` [{item['status']}]: "
        f"{_markdown_text(item['statement'])}"
        for item in value["claims"]
    )
    if not value["claims"]:
        lines.append("- No claims declared.")
    for heading, key in (
        ("Evidence", "evidence"),
        ("Counterevidence", "counterevidence"),
    ):
        lines.extend(["", f"## {heading}", ""])
        entries = value[key]
        lines.extend(
            f"- `{entry['semantic_ref']}` ({entry['relation']})" for entry in entries
        )
        if not entries:
            lines.append(f"- No {key} declared.")
    lines.extend(["", "## Gaps", ""])
    lines.extend(
        f"- `{gap['gap_id']}`: {_markdown_text(gap['summary'])} "
        f"(owner: {_markdown_text(gap['owner'])}; action: "
        f"{_markdown_text(gap['action'])})"
        for gap in value["gaps"]
    )
    if not value["gaps"]:
        lines.append("- No gaps declared.")
    lines.extend(["", "## Owner", "", _markdown_text(value["owner"])])
    lines.extend(["", "## Actions", ""])
    lines.extend(f"- {_markdown_text(action)}" for action in value["actions"])
    if not value["actions"]:
        lines.append("- No actions declared.")
    lines.extend(
        [
            "",
            "## Human decision reference",
            "",
            _markdown_text(value.get("decision_ref", "No decision recorded.")),
            "",
        ]
    )
    return "\n".join(lines)


def _mapping(
    source_path: str, target_concept: str, target_field: str, value: Any
) -> dict[str, Any]:
    return {
        "source_path": source_path,
        "target_concept": target_concept,
        "target_field": target_field,
        "value": value,
    }


def _unmapped(source_path: str, reason: str) -> dict[str, str]:
    return {"source_path": source_path, "reason": reason}


def _compatibility_base(
    *, raw: bytes, schema_version: str, record_id: str
) -> dict[str, Any]:
    return {
        "mapping_version": "engineering-assurance.pgm01-compatibility-view/v1",
        "source_schema_version": schema_version,
        "source_record_id": record_id,
        "source_digest": hashlib.sha256(raw).hexdigest(),
        "outcome": "lossy",
        "mappings": [],
        "unmapped_fields": [],
        "limitations": [],
    }


def _required_object(record: Mapping[str, Any], key: str) -> Mapping[str, Any]:
    value = record.get(key)
    if not isinstance(value, Mapping):
        raise SemanticContractError(f"legacy field /{key} must be an object")
    return value


def _required_list(record: Mapping[str, Any], key: str) -> list[Any]:
    value = record.get(key)
    if not isinstance(value, list):
        raise SemanticContractError(f"legacy field /{key} must be an array")
    return value


def _required_string(record: Mapping[str, Any], key: str) -> str:
    value = record.get(key)
    if not isinstance(value, str) or not value:
        raise SemanticContractError(f"legacy field /{key} must be a string")
    return value


def _required_digest(record: Mapping[str, Any], key: str) -> str:
    value = _required_string(record, key)
    if len(value) != 64 or any(
        character not in "0123456789abcdef" for character in value
    ):
        raise SemanticContractError(f"legacy field /{key} must be a SHA-256 digest")
    return value


def _required_non_negative_integer(record: Mapping[str, Any], key: str) -> int:
    value = record.get(key)
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise SemanticContractError(
            f"legacy field /{key} must be a non-negative integer"
        )
    return value


def _map_pgm01_v1(raw: bytes, record: Mapping[str, Any]) -> dict[str, Any]:
    record_id = _required_string(record, "recordId")
    view = _compatibility_base(
        raw=raw,
        schema_version="quire.pgm01-evidence/v1",
        record_id=record_id,
    )
    collector = _required_object(record, "collector")
    environment = _required_object(record, "environment")
    checks = _required_list(record, "checks")
    outputs = _required_list(record, "outputs")
    limitations = _required_list(record, "limitations")
    view["mappings"].extend(
        [
            _mapping("/recordId", "report", "legacy_record_id", record_id),
            _mapping(
                "/subjectRevision",
                "verification_execution",
                "source_revision",
                _required_string(record, "subjectRevision"),
            ),
            _mapping(
                "/repository",
                "verification_execution",
                "repository",
                _required_string(record, "repository"),
            ),
            _mapping(
                "/collector/implementation",
                "verification_execution",
                "producer.identity",
                _required_string(collector, "implementation"),
            ),
            _mapping(
                "/collector/implementationRevision",
                "verification_execution",
                "producer.source_revision",
                _required_string(collector, "implementationRevision"),
            ),
            _mapping(
                "/environment",
                "verification_execution",
                "producer.environment",
                deepcopy(dict(environment)),
            ),
        ]
    )
    for index, check in enumerate(checks):
        if not isinstance(check, Mapping):
            raise SemanticContractError(
                f"legacy field /checks/{index} must be an object"
            )
        status = check.get("status")
        if status not in PGM01_STATE_MAP:
            raise SemanticContractError(
                f"legacy field /checks/{index}/status is unknown"
            )
        view["mappings"].append(
            _mapping(
                f"/checks/{index}/status",
                "check_result",
                "state",
                PGM01_STATE_MAP[status],
            )
        )
    for index, output in enumerate(outputs):
        if not isinstance(output, str) or not output:
            raise SemanticContractError(
                f"legacy field /outputs/{index} must be a string"
            )
        view["mappings"].append(
            _mapping(f"/outputs/{index}", "evidence", "retained_output.path", output)
        )
    for index, limitation in enumerate(limitations):
        if not isinstance(limitation, str) or not limitation:
            raise SemanticContractError(
                f"legacy field /limitations/{index} must be a string"
            )
        view["mappings"].append(
            _mapping(f"/limitations/{index}", "report", "limitation", limitation)
        )
    view["unmapped_fields"].extend(
        [
            _unmapped(
                "/collector/version",
                "PGM-01 v1 did not record a distinct producer version",
            ),
            _unmapped(
                "/configurationDigest",
                "PGM-01 v1 did not record a configuration digest",
            ),
            _unmapped(
                "/definitionVersion",
                "PGM-01 v1 did not record a governing definition version",
            ),
            _unmapped(
                "/outputs/*/digest",
                "output digests live in a separate checksum file, not this manifest",
            ),
            _unmapped(
                "/decision",
                "the legacy merge-readiness label is not an ix-flow human decision event",
            ),
        ]
    )
    view["limitations"].append(
        "PGM-01 v1 is readable only as a lossy view; missing identities remain missing."
    )
    return view


def _map_pgm01_v2(raw: bytes, record: Mapping[str, Any]) -> dict[str, Any]:
    record_id = _required_string(record, "recordId")
    view = _compatibility_base(
        raw=raw,
        schema_version="quire.pgm01-evidence/v2",
        record_id=record_id,
    )
    collector = _required_object(record, "collector")
    parameters = _required_object(record, "parameters")
    profile = _required_object(record, "profile")
    commands = _required_list(record, "commands")
    limitations = _required_list(record, "limitations")
    view["mappings"].extend(
        [
            _mapping("/recordId", "report", "legacy_record_id", record_id),
            _mapping(
                "/subjectRevision",
                "verification_execution",
                "source_revision",
                _required_string(record, "subjectRevision"),
            ),
            _mapping(
                "/repository",
                "verification_execution",
                "repository",
                _required_string(record, "repository"),
            ),
            _mapping(
                "/collector/id",
                "verification_execution",
                "producer.identity",
                _required_string(collector, "id"),
            ),
            _mapping(
                "/collector/version",
                "verification_execution",
                "producer.version",
                _required_string(collector, "version"),
            ),
            _mapping(
                "/collector/sha256",
                "verification_execution",
                "producer.executable_digest",
                _required_digest(collector, "sha256"),
            ),
            _mapping(
                "/parameters/sha256",
                "verification_execution",
                "producer.configuration_digest",
                _required_digest(parameters, "sha256"),
            ),
            _mapping(
                "/profile/sha256",
                "verification_definition",
                "definition_version",
                _required_digest(profile, "sha256"),
            ),
        ]
    )
    overall = _required_string(record, "overallStatus")
    if overall not in PGM01_STATE_MAP:
        raise SemanticContractError("legacy field /overallStatus is unknown")
    view["mappings"].append(
        _mapping("/overallStatus", "check_result", "state", PGM01_STATE_MAP[overall])
    )
    for index, command in enumerate(commands):
        if not isinstance(command, Mapping):
            raise SemanticContractError(
                f"legacy field /commands/{index} must be an object"
            )
        status = command.get("status")
        if status not in PGM01_STATE_MAP:
            raise SemanticContractError(
                f"legacy field /commands/{index}/status is unknown"
            )
        view["mappings"].append(
            _mapping(
                f"/commands/{index}/status",
                "check_result",
                "state",
                PGM01_STATE_MAP[status],
            )
        )
        for stream in ("stdout", "stderr"):
            retained = command.get(stream)
            if not isinstance(retained, Mapping):
                raise SemanticContractError(
                    f"legacy field /commands/{index}/{stream} must be an object"
                )
            for field in ("path", "sha256", "bytes"):
                if field not in retained:
                    raise SemanticContractError(
                        f"legacy field /commands/{index}/{stream}/{field} is missing"
                    )
                if field == "path":
                    mapped_value: Any = _required_string(retained, field)
                elif field == "sha256":
                    mapped_value = _required_digest(retained, field)
                else:
                    mapped_value = _required_non_negative_integer(retained, field)
                view["mappings"].append(
                    _mapping(
                        f"/commands/{index}/{stream}/{field}",
                        "evidence",
                        f"retained_output.{field}",
                        mapped_value,
                    )
                )
    for index, limitation in enumerate(limitations):
        if not isinstance(limitation, str) or not limitation:
            raise SemanticContractError(
                f"legacy field /limitations/{index} must be a string"
            )
        view["mappings"].append(
            _mapping(f"/limitations/{index}", "report", "limitation", limitation)
        )
    disposition = _required_string(record, "historicalDisposition")
    if disposition == "retracted":
        view["mappings"].append(
            _mapping("/historicalDisposition", "evidence", "state", "stale")
        )
    elif disposition != "active":
        raise SemanticContractError("legacy historicalDisposition is unknown")
    view["unmapped_fields"].extend(
        [
            _unmapped(
                "/environment",
                "PGM-01 v2 did not record a complete execution environment",
            ),
            _unmapped(
                "/commands/*/corroboration",
                "generic transcript corroboration is not imported as a verdict",
            ),
            _unmapped(
                "/quoin/status",
                "legacy intake status is not evidence sufficiency or a check result",
            ),
        ]
    )
    view["limitations"].append(
        "PGM-01 v2 is readable only as a lossy view; corroboration and intake status do not establish success."
    )
    return view


def map_pgm01_bytes(
    raw: bytes, *, expected_digest: str | None = None
) -> dict[str, Any]:
    """Map immutable PGM-01 v1/v2 bytes without writing or synthesizing fields."""
    digest = hashlib.sha256(raw).hexdigest()
    if expected_digest is not None and (
        len(expected_digest) != 64
        or any(character not in "0123456789abcdef" for character in expected_digest)
    ):
        raise SemanticContractError("expected digest must be a SHA-256 digest")
    if expected_digest is not None and digest != expected_digest:
        view = _compatibility_base(
            raw=raw, schema_version="unknown", record_id="tampered-source"
        )
        view["outcome"] = "incompatible"
        view["unmapped_fields"].append(
            _unmapped("/", "tampered source digest differs from expected identity")
        )
        view["limitations"].append("No field from the altered source was interpreted.")
        _validate(view, "pgm01-compatibility-view-v1.schema.json")
        return view
    try:
        decoded = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        view = _compatibility_base(
            raw=raw, schema_version="unknown", record_id="unreadable-source"
        )
        view["outcome"] = "unreadable"
        view["unmapped_fields"].append(_unmapped("/", f"invalid JSON: {error}"))
        view["limitations"].append("No legacy field was interpreted.")
        return view
    if not isinstance(decoded, Mapping):
        view = _compatibility_base(
            raw=raw, schema_version="unknown", record_id="unreadable-source"
        )
        view["outcome"] = "unreadable"
        view["unmapped_fields"].append(
            _unmapped("/", "legacy record must be a JSON object")
        )
        view["limitations"].append("No legacy field was interpreted.")
        return view
    schema_version = decoded.get("schemaVersion")
    record_id = decoded.get("recordId")
    if schema_version not in {
        "quire.pgm01-evidence/v1",
        "quire.pgm01-evidence/v2",
    }:
        view = _compatibility_base(
            raw=raw,
            schema_version=str(schema_version or "unknown"),
            record_id=str(record_id or "incompatible-source"),
        )
        view["outcome"] = "incompatible"
        view["unmapped_fields"].append(
            _unmapped("/schemaVersion", "unknown PGM-01 schema version")
        )
        view["limitations"].append("No unknown schema was treated as empty or current.")
        return view
    try:
        if schema_version.endswith("/v1"):
            view = _map_pgm01_v1(raw, decoded)
        else:
            view = _map_pgm01_v2(raw, decoded)
    except SemanticContractError as error:
        view = _compatibility_base(
            raw=raw,
            schema_version=schema_version,
            record_id=str(record_id or "unreadable-source"),
        )
        view["outcome"] = "unreadable"
        view["unmapped_fields"].append(_unmapped("/", str(error)))
        view["limitations"].append("The malformed legacy record was not accepted.")
    if view["source_digest"] != digest:
        raise AssertionError("compatibility mapping changed source identity")
    _validate(view, "pgm01-compatibility-view-v1.schema.json")
    return view


__all__ = [
    "CONCEPT_AUTHORITIES",
    "OWNERSHIP_PATH",
    "SemanticContractError",
    "load_ownership_registry",
    "map_pgm01_bytes",
    "render_report_json",
    "render_report_markdown",
    "validate_compatibility_fixture",
    "validate_report_projection",
    "validate_semantic_bundle",
    "validate_semantic_reference",
]
