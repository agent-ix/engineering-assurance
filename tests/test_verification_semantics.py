from __future__ import annotations

import ast
import hashlib
import json
from copy import deepcopy

import pytest

from engineering_assurance import FIXTURE_ROOT, PACKAGE_ROOT
from engineering_assurance.fixture_codegen import (
    committed_generated_fixtures,
    load_canonical_fixture,
    load_non_success_states,
    render_generated_fixtures,
)
from engineering_assurance.verification_semantics import (
    CONCEPT_AUTHORITIES,
    SemanticContractError,
    load_ownership_registry,
    map_pgm01_bytes,
    render_report_json,
    render_report_markdown,
    validate_compatibility_fixture,
    validate_report_projection,
    validate_semantic_bundle,
    validate_semantic_reference,
)

SEMANTIC_FIXTURES = FIXTURE_ROOT / "verification-semantics"


def load_fixture(name: str) -> object:
    return json.loads((SEMANTIC_FIXTURES / name).read_text(encoding="utf-8"))


def canonical_references() -> list[dict[str, object]]:
    fixture = load_fixture("canonical-references.json")
    assert isinstance(fixture, dict)
    references = fixture["references"]
    assert isinstance(references, list)
    return references


def mapping_by_source(view: dict[str, object]) -> dict[str, list[dict[str, object]]]:
    grouped: dict[str, list[dict[str, object]]] = {}
    for mapping in view["mappings"]:
        assert isinstance(mapping, dict)
        grouped.setdefault(str(mapping["source_path"]), []).append(mapping)
    return grouped


def test_registry_assigns_every_concept_to_exactly_one_authority() -> None:
    """Trace: US-005-AC-1, FR-008-AC-1, TC-053, TC-056."""
    registry = load_ownership_registry()
    concepts = registry["concepts"]
    assert len(concepts) == len(CONCEPT_AUTHORITIES) == 8
    assert {item["concept"]: item["authority"] for item in concepts} == (
        CONCEPT_AUTHORITIES
    )
    assert registry["non_executing"] is True


def test_canonical_references_preserve_identity_authority_and_links() -> None:
    """Trace: StR-002-VC-1, FR-008-AC-2, TC-052, TC-057."""
    fixture = load_fixture("canonical-references.json")
    assert isinstance(fixture, dict)
    references = validate_compatibility_fixture(fixture)["references"]
    assert len({item["semantic_id"] for item in references}) == len(references)
    assert (
        next(item for item in references if item["concept"] == "measurement")["links"][
            "measurement_plan"
        ]
        == "MP-001"
    )


def test_missing_internal_reference_is_rejected() -> None:
    """Trace: FR-008-AC-3, TC-058."""
    references = canonical_references()
    references.pop(0)
    with pytest.raises(SemanticContractError, match="missing references"):
        validate_semantic_bundle(references)


def test_wrong_internal_reference_type_is_rejected() -> None:
    """Trace: FR-008-AC-3, TC-058."""
    references = canonical_references()
    references[3]["links"]["result"] = "sem:report-001"
    with pytest.raises(SemanticContractError, match="must target check_result"):
        validate_semantic_bundle(references)


def test_authority_mismatch_and_duplicate_identity_are_rejected() -> None:
    """Trace: FR-008-AC-1, FR-008-AC-3, TC-056, TC-058."""
    references = canonical_references()
    references[0]["authority"] = "quoin"
    with pytest.raises(SemanticContractError, match="authority must be quire"):
        validate_semantic_reference(references[0])

    references = canonical_references()
    references[5]["semantic_id"] = references[4]["semantic_id"]
    with pytest.raises(SemanticContractError, match="distinct"):
        validate_semantic_bundle(references)


def test_producer_tuple_is_complete_and_fail_closed() -> None:
    """Trace: FR-009-AC-1, FR-009-AC-3, TC-060, TC-062."""
    producer_fields = {
        "identity",
        "version",
        "configuration_digest",
        "source_revision",
        "environment",
        "definition_version",
    }
    for reference in canonical_references():
        if reference["concept"] in {
            "verification_execution",
            "check_result",
            "evidence",
            "measurement",
            "diagnostic",
        }:
            assert set(reference["producer"]) == producer_fields

    broken = deepcopy(canonical_references()[1])
    del broken["producer"]["configuration_digest"]
    with pytest.raises(SemanticContractError, match="configuration_digest"):
        validate_semantic_reference(broken)

    broken = deepcopy(canonical_references()[1])
    del broken["producer"]
    with pytest.raises(SemanticContractError, match="producer tuple"):
        validate_semantic_reference(broken)


def test_all_non_success_states_validate_without_collapsing() -> None:
    """Trace: FR-009-AC-2, TC-061."""
    template = canonical_references()[2]
    states = load_non_success_states()
    assert "failed" in states
    assert "skipped" in states
    assert "unreadable" in states
    for state in states:
        candidate = deepcopy(template)
        candidate["state"] = state
        assert validate_semantic_reference(candidate)["state"] == state


def test_generated_language_state_fixtures_are_deterministic() -> None:
    """Trace: FR-009-AC-2, TC-061."""
    assert committed_generated_fixtures() == render_generated_fixtures()


def test_generated_language_canonical_fixtures_are_semantically_equal() -> None:
    """Trace: StR-002-VC-1, FR-009-AC-2, TC-052, TC-061."""
    committed = committed_generated_fixtures()
    python_tree = ast.parse(committed["canonical_references.py"])
    assignment = next(node for node in python_tree.body if isinstance(node, ast.Assign))
    python_json = ast.literal_eval(assignment.value)

    typescript_line = committed["canonical_references.ts"].splitlines()[1]
    typescript_json = json.loads(typescript_line.split(" = ", 1)[1].removesuffix(";"))

    rust_line = committed["canonical_references.rs"].splitlines()[1]
    rust_json = rust_line.split('r#"', 1)[1].removesuffix('"#;')

    expected = load_canonical_fixture()
    assert json.loads(python_json) == expected
    assert json.loads(typescript_json) == expected
    assert json.loads(rust_json) == expected


def test_unknown_fixture_source_version_fails_explicitly() -> None:
    """Trace: FR-009-AC-3, TC-062."""
    fixture = load_canonical_fixture()
    fixture["references"][0]["source"]["schema_version"] = "99"
    with pytest.raises(SemanticContractError, match="source-version premises differ"):
        validate_compatibility_fixture(fixture)


@pytest.mark.parametrize("name", ["pgm01-v1.json", "pgm01-v2.json"])
def test_pgm01_mapping_is_read_only_traceable_and_lossy(name: str) -> None:
    """Trace: US-005-AC-3, FR-009-AC-4, FR-010-AC-1, TC-055, TC-063, TC-064."""
    path = SEMANTIC_FIXTURES / name
    before = path.read_bytes()
    view = map_pgm01_bytes(before)
    after = path.read_bytes()
    assert after == before
    assert view["source_digest"] == hashlib.sha256(before).hexdigest()
    assert view["outcome"] == "lossy"
    assert view["limitations"]
    assert view["unmapped_fields"]
    assert all(item["source_path"].startswith("/") for item in view["mappings"])
    assert all(item["source_path"].startswith("/") for item in view["unmapped_fields"])


def test_pgm01_v1_preserves_known_identity_result_outputs_and_limits() -> None:
    """Trace: FR-010-AC-2, TC-065."""
    raw = (SEMANTIC_FIXTURES / "pgm01-v1.json").read_bytes()
    view = map_pgm01_bytes(raw)
    mappings = mapping_by_source(view)
    assert mappings["/subjectRevision"][0]["value"] == "b" * 40
    assert mappings["/collector/implementation"][0]["value"] == ("fictional collector")
    assert mappings["/checks/0/status"][0]["value"] == "passed"
    assert mappings["/checks/1/status"][0]["value"] == "skipped"
    assert mappings["/outputs/0"][0]["value"] == "result.json"
    missing = {item["source_path"] for item in view["unmapped_fields"]}
    assert {"/collector/version", "/configurationDigest", "/decision"} <= missing


def test_pgm01_v2_preserves_producer_config_definition_result_and_outputs() -> None:
    """Trace: FR-010-AC-2, TC-065."""
    raw = (SEMANTIC_FIXTURES / "pgm01-v2.json").read_bytes()
    view = map_pgm01_bytes(raw)
    mappings = mapping_by_source(view)
    assert mappings["/collector/id"][0]["value"] == "fictional-collector"
    assert mappings["/collector/version"][0]["value"] == "1.0.0"
    assert mappings["/parameters/sha256"][0]["target_field"] == (
        "producer.configuration_digest"
    )
    assert mappings["/profile/sha256"][0]["target_field"] == "definition_version"
    assert mappings["/overallStatus"][0]["value"] == "failed"
    assert mappings["/commands/0/stdout/sha256"][0]["target_concept"] == ("evidence")
    unmapped = {item["source_path"] for item in view["unmapped_fields"]}
    assert "/commands/*/corroboration" in unmapped
    assert "/quoin/status" in unmapped


def test_pgm01_mapping_labels_malformed_unknown_stale_and_tampered_inputs() -> None:
    """Trace: FR-009-AC-3, FR-010-AC-3, TC-062, TC-066."""
    with pytest.raises(SemanticContractError, match="expected digest"):
        map_pgm01_bytes(b"{}", expected_digest="not-a-digest")
    assert map_pgm01_bytes(b"not-json")["outcome"] == "unreadable"
    unknown = json.dumps(
        {"schemaVersion": "quire.pgm01-evidence/v99", "recordId": "legacy-1"}
    ).encode()
    assert map_pgm01_bytes(unknown)["outcome"] == "incompatible"

    fixture = load_fixture("pgm01-v2.json")
    assert isinstance(fixture, dict)
    malformed = deepcopy(fixture)
    del malformed["parameters"]
    assert map_pgm01_bytes(json.dumps(malformed).encode())["outcome"] == "unreadable"

    malformed_output = deepcopy(fixture)
    malformed_output["commands"][0]["stdout"]["bytes"] = "64"
    assert map_pgm01_bytes(json.dumps(malformed_output).encode())["outcome"] == (
        "unreadable"
    )

    stale = deepcopy(fixture)
    stale["historicalDisposition"] = "retracted"
    stale_view = map_pgm01_bytes(json.dumps(stale).encode())
    assert any(
        item["source_path"] == "/historicalDisposition" and item["value"] == "stale"
        for item in stale_view["mappings"]
    )

    original = (SEMANTIC_FIXTURES / "pgm01-v2.json").read_bytes()
    tampered = original.replace(b"fictional-collector", b"fictional-tampered")
    original_digest = hashlib.sha256(original).hexdigest()
    tampered_view = map_pgm01_bytes(tampered, expected_digest=original_digest)
    assert tampered_view["outcome"] == "incompatible"
    assert tampered_view["mappings"] == []
    assert "tampered" in tampered_view["unmapped_fields"][0]["reason"]


def test_report_projection_is_bounded_and_renders_deterministically() -> None:
    """Trace: US-005-AC-2, FR-010-AC-4, TC-054, TC-067."""
    report = load_fixture("report-projection.json")
    assert isinstance(report, dict)
    assert validate_report_projection(report) == report
    assert render_report_json(report) == render_report_json(report)
    markdown = render_report_markdown(report)
    for heading in (
        "## Claims",
        "## Evidence",
        "## Counterevidence",
        "## Gaps",
        "## Owner",
        "## Actions",
        "## Human decision reference",
    ):
        assert heading in markdown
    assert "trust score" not in markdown.casefold()
    assert "overall verdict" not in markdown.casefold()

    forbidden = deepcopy(report)
    forbidden["overall_verdict"] = "passed"
    with pytest.raises(SemanticContractError):
        validate_report_projection(forbidden)


def test_semantic_module_has_no_execution_or_persistence_path() -> None:
    """Trace: FR-008-AC-4, NFR-004-AC-2, TC-059."""
    path = PACKAGE_ROOT / "verification_semantics.py"
    tree = ast.parse(path.read_text(encoding="utf-8"))
    imports = {
        alias.name
        for node in ast.walk(tree)
        if isinstance(node, (ast.Import, ast.ImportFrom))
        for alias in node.names
    }
    assert "subprocess" not in imports
    assert "os" not in imports
    forbidden_calls = {
        "system",
        "Popen",
        "run",
        "write_text",
        "write_bytes",
        "unlink",
    }
    calls = {
        node.func.attr
        for node in ast.walk(tree)
        if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)
    }
    assert not (calls & forbidden_calls)


def test_contracts_do_not_define_parallel_record_envelopes() -> None:
    """Trace: NFR-004-AC-1, TC-068."""
    contract_files = sorted((PACKAGE_ROOT / "contracts").glob("*.json"))
    schema_files = sorted((PACKAGE_ROOT / "schemas").glob("*semantic*.json")) + [
        PACKAGE_ROOT / "schemas" / "assurance-report-projection-v1.schema.json",
        PACKAGE_ROOT / "schemas" / "pgm01-compatibility-view-v1.schema.json",
    ]
    text = "\n".join(
        path.read_text(encoding="utf-8") for path in contract_files + schema_files
    )
    assert '"record_type"' not in text
    assert '"retention"' not in text
    assert '"evidence_store"' not in text
    assert "generic evidence envelope" not in text.casefold()
