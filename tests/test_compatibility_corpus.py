"""FR-011 — the accepted compatibility fixture set, as an enforcing gate.

This is the gate FR-010 deferred: until it passed, "the reconciled contract
reads real history" was a claim. Every assertion below runs against retained
bytes, offline, with no producer executed and no repository contacted.
"""

from __future__ import annotations

import hashlib
import json
import os
import stat
import subprocess

import jsonschema
import pytest

from engineering_assurance.compatibility_corpus import (
    CORPUS_SUBMODULE,
    REQUIRED_KINDS,
    case,
    cases_by_kind,
    chain_artifact,
    corpus_paths,
    load_corpus,
    receipt_schema,
    retained_bytes,
)
from engineering_assurance.verification_semantics import map_pgm01_bytes

CORPUS = load_corpus()
LEGACY_CASES = [c for c in CORPUS["cases"] if c["family"].startswith("pgm01")]


def mapping_values(view: dict, source_path: str) -> list:
    return [m["value"] for m in view["mappings"] if m["source_path"] == source_path]


def test_every_retained_artifact_is_the_artifact_recorded() -> None:
    """Trace: FR-011-AC-1, TC-069."""
    for entry in CORPUS["cases"]:
        retained_bytes(entry)
    for entry in CORPUS["producer_cases"]:
        if entry["retention"] == "retained":
            retained_bytes(entry)
    for entry in CORPUS["chain"]["artifacts"]:
        retained_bytes(entry)

    # A real legacy case must still match the digest the SOURCE repository
    # recorded for it. That is what proves the retained copy is the immutable
    # record and not an edited lookalike — and it is the assertion that would
    # fail first if anyone rewrote the source evidence tree.
    for entry in LEGACY_CASES:
        if entry["derivation"] is not None or entry["origin"] is None:
            continue
        raw = retained_bytes(entry)
        assert hashlib.sha256(raw).hexdigest() == entry["origin"]["recorded_sha256"], (
            f"{entry['id']} no longer matches the digest "
            f"{entry['origin']['repository']} recorded for it"
        )


def test_corpus_covers_every_required_state_and_labels_constructions() -> None:
    """Trace: FR-011-AC-2, TC-070."""
    kinds = {entry["kind"] for entry in CORPUS["cases"]}
    missing = sorted(REQUIRED_KINDS - kinds)
    assert missing == [], f"the accepted corpus is missing: {', '.join(missing)}"

    # Every constructed case says what was changed and why, so a reader can
    # tell a real record from one built to fill a hole in the corpus.
    for entry in CORPUS["cases"]:
        if entry["derivation"] is None:
            continue
        assert entry["derivation"]["operation"], f"{entry['id']} names no edit"
        assert entry["derivation"]["reason"], f"{entry['id']} gives no reason"

    # And the corpus states the limitation that made those constructions
    # necessary, rather than presenting them as found history.
    limitations = " ".join(CORPUS["limitations"])
    assert "derived" in limitations
    assert "not-computed" in limitations


@pytest.mark.parametrize("entry", LEGACY_CASES, ids=lambda e: e["id"])
def test_every_legacy_case_maps_to_its_recorded_outcome(entry: dict) -> None:
    """Trace: FR-011-AC-3, TC-071."""
    raw = retained_bytes(entry)
    kwargs = {}
    reference = entry["expected"].get("verify_against_digest_of")
    if reference:
        kwargs["expected_digest"] = hashlib.sha256(
            retained_bytes(case(CORPUS, reference))
        ).hexdigest()

    view = map_pgm01_bytes(raw, **kwargs)
    assert view["outcome"] == entry["expected"]["outcome"]

    for required in entry["expected"].get("required_mappings", []):
        assert required["value"] in mapping_values(view, required["source_path"]), (
            f"{entry['id']} lost {required['source_path']} -> {required['value']}"
        )
    if entry["expected"].get("require_no_mappings"):
        assert view["mappings"] == []
    assert view["limitations"], f"{entry['id']} states no limitation"


def test_no_non_success_case_is_read_as_a_success() -> None:
    """Trace: FR-011-AC-4, TC-072."""
    for kind in ("failed", "unavailable", "not_computed", "malformed", "tampered"):
        for entry in cases_by_kind(CORPUS, kind):
            raw = retained_bytes(entry)
            kwargs = {}
            reference = entry["expected"].get("verify_against_digest_of")
            if reference:
                kwargs["expected_digest"] = hashlib.sha256(
                    retained_bytes(case(CORPUS, reference))
                ).hexdigest()
            view = map_pgm01_bytes(raw, **kwargs)

            # `compatible` is the only outcome that would mean "read cleanly as
            # a current record". None of these may reach it, and none of them
            # may report a passed check either.
            assert view["outcome"] != "compatible", f"{entry['id']} read as clean"
            assert "passed" not in mapping_values(view, "/checks/0/status"), (
                f"{entry['id']} turned a non-success check into a pass"
            )


def test_real_legacy_records_preserve_identity_producer_and_limits() -> None:
    """Trace: FR-011-AC-5, TC-073."""
    entry = case(CORPUS, "legacy-v1-passing")
    view = map_pgm01_bytes(retained_bytes(entry))
    source = json.loads(retained_bytes(entry))

    assert mapping_values(view, "/subjectRevision") == [source["subjectRevision"]]
    assert mapping_values(view, "/repository") == [source["repository"]]
    assert mapping_values(view, "/collector/implementation") == [
        source["collector"]["implementation"]
    ]
    assert mapping_values(view, "/collector/implementationRevision") == [
        source["collector"]["implementationRevision"]
    ]
    assert mapping_values(view, "/environment") == [source["environment"]]

    # What the legacy record could not carry stays named, not filled in.
    unmapped = {item["source_path"] for item in view["unmapped_fields"]}
    assert {"/collector/version", "/configurationDigest", "/decision"} <= unmapped

    # An inconclusive check survives as inconclusive rather than rounding.
    inconclusive = map_pgm01_bytes(
        retained_bytes(case(CORPUS, "legacy-v1-inconclusive"))
    )
    states = [
        m["value"] for m in inconclusive["mappings"] if m["target_field"] == "state"
    ]
    assert "inconclusive" in states
    assert "passed" in states


def test_current_model_receipt_validates_against_the_packaged_schema() -> None:
    """Trace: FR-011-AC-6, TC-074."""
    entry = case(CORPUS, "current-verification-receipt")
    receipt = json.loads(retained_bytes(entry))
    jsonschema.validate(receipt, receipt_schema(CORPUS))
    assert receipt["outcome"] == entry["expected"]["outcome"] == "valid"

    # The receipt binds the record, the attestation, and the retained output
    # that the chain actually carried — not a summary of them.
    record = json.loads(retained_bytes(chain_artifact(CORPUS, "change_assurance_record")))
    attestation = json.loads(
        retained_bytes(chain_artifact(CORPUS, "proof_attestation"))
    )
    assert receipt["record_digest"] == record["digest"]
    assert receipt["proofs"][0]["attestation_digest"] == attestation["digest"]
    assert (
        receipt["proofs"][0]["retained_output_digest"]
        == attestation["retained_output"]["digest"]
    )

    # The Quire export is referenced, not republished. Its identity is read
    # back from the retained attestation rather than asserted separately, so
    # the reference cannot drift from the evidence that binds it.
    export = next(
        entry
        for entry in CORPUS["chain"]["referenced_inputs"]
        if entry["role"] == "quire_export"
    )
    assert export["retention"] == "referenced"
    assert export["blake3"] == attestation["retained_output"]["digest"]
    assert export["size_bytes"] == attestation["retained_output"]["size_bytes"]
    assert export["bound_by"] == "chain/attestation-sealed.json"
    assert "workstation" not in export["reason"] or export["reason"]
    assert receipt["candidate_revision"] == CORPUS["chain"]["subject"]["revision"]

    # The chain is pinned to exact tools, and says plainly that the Quoin side
    # is a source revision rather than a released artifact.
    # Both sides of the chain are released artifacts, named by their release
    # and pinned to the source revision that produced them. Engineering
    # Assurance #8 turned the Quoin side from a source build into a release,
    # and the chain reproduced byte for byte across that change.
    tools = CORPUS["chain"]["tools"]
    assert tools["quire"]["version"] == "0.31.0"
    assert tools["quoin"]["version"] == "0.23.1"
    assert tools["quoin"]["release"] == "npm @agent-ix/quoin@0.23.1"
    assert len(tools["quoin"]["source_revision"]) == 40
    assert "released artifact" in tools["quoin"]["note"]


def test_producer_cases_name_a_real_producer_and_a_target_concept() -> None:
    """Trace: FR-011-AC-7, TC-075."""
    concepts = set()
    languages = set()
    for entry in CORPUS["producer_cases"]:
        # A referenced case is pinned by digest and deliberately not copied
        # here. It still has to name what it is and why it is not retained,
        # so "referenced" can never become a quiet way to list nothing.
        assert entry["retention"] in {"retained", "referenced"}
        if entry["retention"] == "retained":
            retained_bytes(entry)
        else:
            assert len(entry["source_sha256"]) == 64, f"{entry['id']} is unpinned"
            assert "NOT" in entry["note"], f"{entry['id']} does not say why"
            assert "retained_path" not in entry
        assert entry["producer"], f"{entry['id']} names no producer"
        assert entry["path"], f"{entry['id']} names no source path"
        assert entry["feeds"] in {
            "verification_definition",
            "verification_execution",
            "check_result",
            "evidence",
            "measurement",
            "diagnostic",
            "report",
            "human_decision",
        }, f"{entry['id']} feeds an unknown concept"
        concepts.add(entry["feeds"])
        languages.add(entry["language"])

    # Cross-language is the point: a contract that only ever reads its own
    # language's output has not been tested against the campaign.
    assert {"rust", "typescript"} <= languages
    assert len(concepts) >= 4, "the producer cases exercise too few concepts"
    assert {"check_result", "measurement"} <= concepts

    # A real governed producer case, named in the ticket, is present.
    code_graph = next(
        entry
        for entry in CORPUS["producer_cases"]
        if entry["producer"] == "agent-ix/quire-code-rs"
    )
    assert len(code_graph["revision"]) == 40


def test_the_corpus_is_read_only_and_executes_nothing() -> None:
    """Trace: FR-011-AC-8, TC-076."""
    before = {path: path.read_bytes() for path in corpus_paths()}
    for entry in CORPUS["cases"]:
        if entry["family"].startswith("pgm01"):
            kwargs = {}
            reference = entry["expected"].get("verify_against_digest_of")
            if reference:
                kwargs["expected_digest"] = hashlib.sha256(
                    retained_bytes(case(CORPUS, reference))
                ).hexdigest()
            map_pgm01_bytes(retained_bytes(entry), **kwargs)
    after = {path: path.read_bytes() for path in corpus_paths()}
    assert after == before, "mapping the corpus changed the corpus"

    # No retained artifact is executable: these are records, and a record that
    # can be run is a record that will be.
    for path in corpus_paths():
        assert not path.stat().st_mode & stat.S_IXUSR, f"{path} is executable"

    # Call-shaped tokens, not words: the module's own docstring says it writes
    # nothing, and a prose match would have made that sentence the failure.
    source = (
        CORPUS_SUBMODULE.parent / "engineering_assurance" / "compatibility_corpus.py"
    ).read_text("utf-8")
    for forbidden in (
        "import subprocess",
        "import socket",
        "import urllib",
        "import requests",
        "import shutil",
        ".write_text(",
        ".write_bytes(",
        "os.remove",
    ):
        assert forbidden not in source, f"the corpus reader reaches for {forbidden}"


def test_the_corpus_reproduces_from_its_recorded_sources() -> None:
    """Trace: FR-011-AC-9, TC-077.

    The builder lives beside the corpus, in the pinned submodule. Skipped where
    the source repositories are not checked out: the corpus is self-verifying
    offline, and this is the stronger check a maintainer runs where the sources
    exist. Skipping is stated, never silent.
    """
    checkouts = CORPUS_SUBMODULE.parent.parent
    for repository in ("quire-contract-ir", "quire-code-rs", "quoin"):
        if not (checkouts / repository / ".git").exists():
            pytest.skip(f"source repository {repository} is not checked out")
    result = subprocess.run(
        ["python3", "scripts/build_compatibility_corpus.py", "--check"],
        cwd=CORPUS_SUBMODULE,
        env={**os.environ, "ASSURANCE_SOURCE_ROOT": str(checkouts)},
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stdout + result.stderr


def test_the_pinned_corpus_is_the_reviewed_corpus() -> None:
    """Trace: FR-011-AC-10, TC-078.

    The gitlink is the whole provenance of the corpus in this repository. A
    floating branch would let the accepted fixture set change under the gate
    without a reviewed commit here.
    """
    recorded = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=CORPUS_SUBMODULE,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()
    # Read from the index, not HEAD: the index is what a reviewer sees in the
    # diff and what the next commit will carry, so the check holds while the
    # pin is being changed as well as after.
    gitlink = subprocess.run(
        ["git", "ls-files", "-s", "corpus"],
        cwd=CORPUS_SUBMODULE.parent,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()
    assert gitlink, "corpus is not tracked as a gitlink"
    assert gitlink[0] == "160000", "corpus is tracked as files, not a submodule"
    assert gitlink[1] == recorded, (
        f"the checked-out corpus is {recorded}, the pinned commit is {gitlink[1]}"
    )
