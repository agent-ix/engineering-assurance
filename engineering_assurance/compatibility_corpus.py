"""The accepted compatibility-fixture corpus (FR-011).

Read-only access to the corpus retained by `agent-ix/qa-corpus`, pinned here as
a submodule and read in place. Nothing in this module executes a producer,
contacts a repository, or writes a byte.

The corpus lives in that repository rather than this one because it retains
real governance evidence, and this repository's publication boundary permits
fictional fixtures only. What that buys is worth stating: the gate reads real
records without this public tree ever holding them.

The corpus answers one question — *does the reconciled contract read real
history and real producer output?* — and refuses to answer a second one. It
proves nothing about the live state of the source repositories; it proves that
these exact bytes behave this exact way.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

from engineering_assurance import PACKAGE_ROOT

#: The pinned `agent-ix/qa-corpus` submodule, read in place.
CORPUS_SUBMODULE = PACKAGE_ROOT.parent / "corpus"
CORPUS_ROOT = CORPUS_SUBMODULE / "compatibility"
CORPUS_PATH = CORPUS_ROOT / "corpus.json"

CORPUS_VERSION = "engineering-assurance.compatibility-corpus/v1"

#: Every state the campaign must be able to carry through the contract. A
#: corpus missing one of these is not an accepted corpus — the gate says so by
#: name rather than by count, because "7 of 8 kinds" tells nobody which.
REQUIRED_KINDS = frozenset(
    {
        "legacy",
        "current",
        "malformed",
        "unavailable",
        "not_computed",
        "failed",
        "stale",
        "tampered",
    }
)


class CorpusError(ValueError):
    """The retained corpus does not describe itself correctly."""


def corpus_available() -> bool:
    """Whether the pinned corpus submodule is checked out.

    An uninitialized submodule is an empty directory, not an error — but a gate
    that silently passes over one proves nothing. Callers say so out loud.
    """
    return CORPUS_PATH.is_file()


def load_corpus() -> dict[str, Any]:
    """Parse the corpus index, refusing an unknown corpus version."""
    if not corpus_available():
        raise CorpusError(
            "the qa-corpus submodule is not checked out; run "
            "`git submodule update --init corpus`"
        )
    corpus = json.loads(CORPUS_PATH.read_text(encoding="utf-8"))
    version = corpus.get("corpus_version")
    if version != CORPUS_VERSION:
        raise CorpusError(f"unknown corpus version: {version!r}")
    for key in ("cases", "producer_cases", "chain", "limitations"):
        if key not in corpus:
            raise CorpusError(f"corpus is missing {key}")
    return corpus


def retained_bytes(entry: dict[str, Any]) -> bytes:
    """Read one retained artifact and prove it is the artifact recorded.

    The digest check is the whole contract of this function. A corpus whose
    bytes drifted from its index would otherwise still pass every behavioural
    assertion below it, against different bytes than the ones reviewed.
    """
    if entry.get("retention") == "referenced":
        raise CorpusError(
            f"{entry.get('id', entry.get('role'))} is referenced by digest, "
            "not retained; read its source repository to obtain the bytes"
        )
    path = CORPUS_ROOT / entry["retained_path"]
    raw = path.read_bytes()
    actual = hashlib.sha256(raw).hexdigest()
    expected = entry["retained_sha256"]
    if actual != expected:
        raise CorpusError(
            f"{entry['retained_path']} is {actual}, recorded as {expected}"
        )
    return raw


def case(corpus: dict[str, Any], case_id: str) -> dict[str, Any]:
    for entry in corpus["cases"]:
        if entry["id"] == case_id:
            return entry
    raise CorpusError(f"no such case: {case_id}")


def cases_by_kind(corpus: dict[str, Any], kind: str) -> list[dict[str, Any]]:
    return [entry for entry in corpus["cases"] if entry["kind"] == kind]


def chain_artifact(corpus: dict[str, Any], role: str) -> dict[str, Any]:
    for entry in corpus["chain"]["artifacts"]:
        if entry["role"] == role:
            return entry
    raise CorpusError(f"no such chain artifact: {role}")


def receipt_schema(corpus: dict[str, Any]) -> dict[str, Any]:
    """Quoin's packaged receipt schema, as retained beside the receipt."""
    entry = chain_artifact(corpus, "receipt_schema")
    return json.loads(retained_bytes(entry))


def corpus_paths() -> list[Path]:
    """Every file the corpus retains, for the read-only assertions."""
    return sorted(p for p in CORPUS_ROOT.rglob("*") if p.is_file())


__all__ = [
    "CORPUS_PATH",
    "CORPUS_ROOT",
    "CORPUS_SUBMODULE",
    "corpus_available",
    "CORPUS_VERSION",
    "CorpusError",
    "REQUIRED_KINDS",
    "case",
    "cases_by_kind",
    "chain_artifact",
    "corpus_paths",
    "load_corpus",
    "receipt_schema",
    "retained_bytes",
]
