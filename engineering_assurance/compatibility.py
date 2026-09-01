"""The pinned shared-assurance compatibility matrix (FR-012).

One reviewed set of component versions that are known to work together, with
the digests that identify the artifacts those versions ship.

Everything here is pure. It classifies a version a caller observed; it does not
observe one. `scripts/check_compatibility_matrix.py` does the observing, so the
rules can be tested without executing anything and the two concerns cannot be
confused for each other.

Three answers, and the difference between them is the whole point:

- `compatible`   — the exact pinned version.
- `incompatible` — a version the matrix names and rules out.
- `unknown`      — a version the matrix has never seen.

`unknown` is not a synonym for `incompatible`, and neither is a synonym for
`compatible`. A toolchain nobody has tested against this matrix is a fact about
the matrix, not a verdict about the toolchain — and it must never read as a
pass.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal

from engineering_assurance import PACKAGE_ROOT

MATRIX_PATH = PACKAGE_ROOT / "compatibility-matrix.json"
MATRIX_VERSION = "engineering-assurance.compatibility-matrix/v1"

Verdict = Literal["compatible", "incompatible", "unknown"]


class MatrixError(ValueError):
    """The matrix does not describe itself correctly."""


@dataclass(frozen=True)
class Classification:
    """One component's verdict, with the reason a reader has to act on."""

    component: str
    observed: str | None
    expected: str
    verdict: Verdict
    reason: str


def load_matrix() -> dict[str, Any]:
    """Parse the matrix, refusing an unknown matrix version."""
    matrix = json.loads(MATRIX_PATH.read_text(encoding="utf-8"))
    if matrix.get("matrix_version") != MATRIX_VERSION:
        raise MatrixError(f"unknown matrix version: {matrix.get('matrix_version')!r}")
    for key in ("accepted", "components", "gate", "rollback"):
        if key not in matrix:
            raise MatrixError(f"matrix is missing {key}")
    if not matrix["components"]:
        raise MatrixError("matrix pins no component")
    return matrix


def component(matrix: dict[str, Any], name: str) -> dict[str, Any]:
    for entry in matrix["components"]:
        if entry["name"] == name:
            return entry
    raise MatrixError(f"no such component: {name}")


def classify(matrix: dict[str, Any], name: str, observed: str | None) -> Classification:
    """Classify one observed version against the pin.

    `observed is None` means the component was not found at all, which is its
    own answer: absent is not incompatible, and it is certainly not compatible.
    """
    entry = component(matrix, name)
    expected = entry["version"]
    if observed is None:
        return Classification(
            component=name,
            observed=None,
            expected=expected,
            verdict="unknown",
            reason=f"{name} was not observed; nothing was checked against the pin",
        )
    if observed == expected:
        return Classification(
            component=name,
            observed=observed,
            expected=expected,
            verdict="compatible",
            reason=f"{name} {observed} is the pinned version",
        )
    if observed in entry.get("incompatible", []):
        return Classification(
            component=name,
            observed=observed,
            expected=expected,
            verdict="incompatible",
            reason=entry.get("incompatible_reasons", {}).get(
                observed, f"{name} {observed} is named incompatible by this matrix"
            ),
        )
    return Classification(
        component=name,
        observed=observed,
        expected=expected,
        verdict="unknown",
        reason=(
            f"{name} {observed} is not the pinned {expected} and this matrix has "
            "never seen it; it is untested, not approved and not rejected"
        ),
    )


def classify_all(
    matrix: dict[str, Any], observed: dict[str, str | None]
) -> list[Classification]:
    """Classify every pinned component, including ones the caller did not observe."""
    return [
        classify(matrix, entry["name"], observed.get(entry["name"]))
        for entry in matrix["components"]
    ]


def accepted(classifications: list[Classification]) -> bool:
    """Whether every component is the pinned version.

    Deliberately strict. The gate this feeds decides whether an enforcing
    migration may begin, and "mostly pinned" is not a state that decision has.
    """
    return all(item.verdict == "compatible" for item in classifications)


def digest_of(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def verify_artifact_digests(matrix: dict[str, Any], root: Path) -> list[str]:
    """Re-hash every artifact the matrix pins that is present in `root`.

    Returns the mismatches by path. An artifact the matrix names and this tree
    does not contain is not a mismatch — it is somebody else's tree — so it is
    skipped rather than reported as drift.
    """
    mismatches: list[str] = []
    for entry in matrix["components"]:
        for artifact in entry.get("artifacts", []):
            path = root / artifact["path"]
            if not path.is_file():
                continue
            actual = digest_of(path)
            if actual != artifact["sha256"]:
                mismatches.append(
                    f"{artifact['path']}: {actual}, matrix records {artifact['sha256']}"
                )
    return mismatches


__all__ = [
    "Classification",
    "MATRIX_PATH",
    "MATRIX_VERSION",
    "MatrixError",
    "Verdict",
    "accepted",
    "classify",
    "classify_all",
    "component",
    "digest_of",
    "load_matrix",
    "verify_artifact_digests",
]
