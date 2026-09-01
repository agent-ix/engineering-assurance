"""FR-012 — the pinned shared-assurance compatibility matrix.

Every assertion runs against the matrix data. Nothing here executes a tool:
the classification rules are pure by construction, and that is what makes it
possible to test the answer for a version nobody has installed.
"""

from __future__ import annotations

import json

import pytest

from engineering_assurance.compatibility import (
    MATRIX_PATH,
    MatrixError,
    accepted,
    classify,
    classify_all,
    component,
    load_matrix,
    verify_artifact_digests,
)
from engineering_assurance.compatibility_corpus import CORPUS_SUBMODULE

MATRIX = load_matrix()
REPO_ROOT = CORPUS_SUBMODULE.parent


def test_every_pin_is_a_released_artifact() -> None:
    """Trace: FR-012-AC-1, TC-079."""
    assert MATRIX["components"], "the matrix pins nothing"
    for entry in MATRIX["components"]:
        assert entry["released"] is True, f"{entry['name']} is not released"
        assert entry["release"], f"{entry['name']} names no release"
        assert entry["version"], f"{entry['name']} pins no version"
        # A branch name, a tag-less revision, or "latest" is the exact failure
        # this criterion exists to prevent.
        assert entry["version"] not in {"main", "latest", "HEAD"}
        assert "branch" not in entry["release"].lower()

    # The four components the campaign actually depends on are all present.
    for name in ("quire-cli", "quoin", "ix-flow", "engineering-assurance"):
        assert component(MATRIX, name)


def test_unknown_and_incompatible_are_distinct_and_neither_passes() -> None:
    """Trace: FR-012-AC-2, TC-080."""
    pinned = component(MATRIX, "quoin")["version"]

    assert classify(MATRIX, "quoin", pinned).verdict == "compatible"

    # A version the matrix names and rules out.
    ruled_out = classify(MATRIX, "quoin", "0.22.5")
    assert ruled_out.verdict == "incompatible"
    assert "change-assurance" in ruled_out.reason

    # The tagged-but-unpublished one is named too, so nobody has to rediscover
    # why it does not exist on the registry.
    unpublished = classify(MATRIX, "quoin", "0.23.0")
    assert unpublished.verdict == "incompatible"
    assert "never published" in unpublished.reason

    # A version nothing has said anything about.
    untested = classify(MATRIX, "quoin", "99.99.99")
    assert untested.verdict == "unknown"
    assert "never seen it" in untested.reason

    # Absent is its own answer, and it is not incompatible.
    absent = classify(MATRIX, "quoin", None)
    assert absent.verdict == "unknown"
    assert absent.observed is None
    assert "not observed" in absent.reason

    # None of the three non-pinned answers satisfies the gate.
    for observed in ("0.22.5", "0.23.0", "99.99.99", None):
        assert not accepted(
            classify_all(
                MATRIX,
                {entry["name"]: entry["version"] for entry in MATRIX["components"]}
                | {"quoin": observed},
            )
        )


def test_the_gate_requires_every_component_and_says_so() -> None:
    """Trace: FR-012-AC-3, TC-081."""
    everything = {entry["name"]: entry["version"] for entry in MATRIX["components"]}
    assert accepted(classify_all(MATRIX, everything))

    # One unobserved component is enough to withhold the gate. "Mostly pinned"
    # is not a state the migration decision has.
    for name in everything:
        partial = dict(everything)
        partial[name] = None
        assert not accepted(classify_all(MATRIX, partial)), name

    gate = MATRIX["gate"]
    assert "unknown" in gate["unknown_is_not_pass"].lower()
    assert "compatible" in gate["rule"]


def test_human_acceptance_is_pending_and_an_agent_cannot_grant_it() -> None:
    """Trace: FR-012-AC-4, TC-082."""
    acceptance = MATRIX["accepted"]
    assert acceptance["state"] == "pending_human_acceptance"
    assert acceptance["accepted_by"] is None
    assert acceptance["accepted_at"] is None
    assert "human" in acceptance["note"].lower()
    assert "agent may prepare" in acceptance["note"]


def test_pinned_artifact_digests_match_this_tree() -> None:
    """Trace: FR-012-AC-5, TC-083."""
    assert verify_artifact_digests(MATRIX, REPO_ROOT) == []

    # And the check is not vacuous: it hashes files that exist here.
    present = [
        artifact
        for entry in MATRIX["components"]
        for artifact in entry.get("artifacts", [])
        if (REPO_ROOT / artifact["path"]).is_file()
    ]
    assert len(present) >= 10, "the digest check examined too few artifacts"


def test_upgrade_and_rollback_are_stated_per_component() -> None:
    """Trace: FR-012-AC-6, TC-084."""
    rollback = MATRIX["rollback"]
    for name in ("quoin", "quire-cli", "engineering-assurance", "corpus"):
        assert name in rollback, f"{name} has no rollback note"
        assert rollback[name].strip(), f"{name}'s rollback note is empty"
    assert "None of the above" in rollback["irreversible"]

    upgrade = MATRIX["upgrade"]
    assert "quire-cli" in upgrade["order"]
    assert "check_compatibility_matrix" in upgrade["verification"]
    # The upgrade explicitly does not touch a campaign repository.
    assert "Migrations are" in upgrade["what_does_not_change"]

    # Publication changes no repository's CI posture.
    assert "manual-dispatch only" in MATRIX["hosted_ci"]

    # The release channel is the public registry, and the internal mirror is
    # ruled out by name. A pin naming npm.ix cannot install or publish from CI,
    # and the mirror lagging a real publish already produced one wrong reading
    # while this matrix was prepared.
    registry = MATRIX["registry"]
    assert registry["release_channel"] == "public npm registry (registry.npmjs.org)"
    assert "npm.ix" in registry["rule"]
    assert "MUST NOT appear" in registry["rule"]
    assert "npm.ix" in registry["mirror_is_not_an_oracle"]
    for entry in MATRIX["components"]:
        assert "npm.ix" not in entry["release"], entry["name"]
        assert "npm.ix" not in entry["version"], entry["name"]


def test_an_unknown_matrix_version_is_refused() -> None:
    """Trace: FR-012-AC-7, TC-085."""
    raw = json.loads(MATRIX_PATH.read_text(encoding="utf-8"))
    assert raw["matrix_version"] == "engineering-assurance.compatibility-matrix/v1"

    with pytest.raises(MatrixError):
        component(MATRIX, "not-a-component")


def test_the_classifier_executes_nothing() -> None:
    """Trace: FR-012-AC-8, TC-086."""
    source = (REPO_ROOT / "engineering_assurance" / "compatibility.py").read_text(
        encoding="utf-8"
    )
    for forbidden in (
        "import subprocess",
        "import socket",
        "import urllib",
        "import requests",
        "shutil.which",
        ".write_text(",
        ".write_bytes(",
    ):
        assert forbidden not in source, f"the classifier reaches for {forbidden}"

    # The observing half is a separate file, and it is the only one allowed to
    # ask the environment anything.
    observer = (REPO_ROOT / "scripts" / "check_compatibility_matrix.py").read_text(
        encoding="utf-8"
    )
    assert "subprocess" in observer
    assert "shutil.which" in observer
