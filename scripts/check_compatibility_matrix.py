#!/usr/bin/env python3
"""Classify the installed toolchain against the pinned compatibility matrix.

This is the impure half of FR-012: it asks the environment what it has, then
hands the answers to `engineering_assurance.compatibility`, which decides. The
split is deliberate — the rules are tested without executing anything, and a
version this script failed to read becomes `unknown` rather than absent.

    python3 scripts/check_compatibility_matrix.py
    python3 scripts/check_compatibility_matrix.py --json

Exit status is 0 only when every pinned component is compatible. Unknown is not
a pass: a toolchain nobody has tested against this matrix is a fact about the
matrix, and it must never read as approval.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from engineering_assurance.compatibility import (  # noqa: E402
    accepted,
    classify_all,
    load_matrix,
    verify_artifact_digests,
)

REPO_ROOT = Path(__file__).resolve().parent.parent

SEMVER = re.compile(r"\b(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)\b")


def observe(command: list[str]) -> str | None:
    """Read one tool's self-reported version, or None if it cannot be read.

    Every failure path returns None rather than a guess: a missing binary, a
    non-zero exit, a timeout, and unparseable output are all "not observed",
    and the matrix treats that as unknown.
    """
    if shutil.which(command[0]) is None:
        return None
    try:
        completed = subprocess.run(
            command, capture_output=True, text=True, timeout=60, check=False
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if completed.returncode != 0:
        return None
    match = SEMVER.search(completed.stdout.strip())
    return match.group(1) if match else None


def observe_quire() -> str | None:
    """quire reports a structured provenance document; read the CLI version."""
    if shutil.which("quire") is None:
        return None
    try:
        completed = subprocess.run(
            ["quire", "provenance"], capture_output=True, text=True, timeout=60, check=False
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if completed.returncode != 0:
        return None
    try:
        return str(json.loads(completed.stdout)["cli"]["version"])
    except (json.JSONDecodeError, KeyError, TypeError):
        return None


def observe_self() -> str | None:
    """This repository's own released version, from its tag."""
    try:
        completed = subprocess.run(
            ["git", "-C", str(REPO_ROOT), "describe", "--tags", "--abbrev=0"],
            capture_output=True,
            text=True,
            timeout=60,
            check=False,
        )
    except (OSError, subprocess.SubprocessError):
        return None
    if completed.returncode != 0:
        return None
    return completed.stdout.strip().lstrip("v") or None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--json", action="store_true", help="emit the classification as JSON")
    args = parser.parse_args()

    matrix = load_matrix()
    observed = {
        "quire-cli": observe_quire(),
        "quoin": observe(["quoin", "--version"]),
        "ix-flow": observe(["ix-flow", "--version"]),
        "engineering-assurance": observe_self(),
    }
    classifications = classify_all(matrix, observed)
    mismatches = verify_artifact_digests(matrix, REPO_ROOT)
    ok = accepted(classifications) and not mismatches

    if args.json:
        print(
            json.dumps(
                {
                    "accepted": ok,
                    "acceptance_state": matrix["accepted"]["state"],
                    "components": [
                        {
                            "component": item.component,
                            "observed": item.observed,
                            "expected": item.expected,
                            "verdict": item.verdict,
                            "reason": item.reason,
                        }
                        for item in classifications
                    ],
                    "artifact_mismatches": mismatches,
                },
                indent=2,
            )
        )
    else:
        for item in classifications:
            print(f"{item.verdict:<12} {item.component:<24} {item.reason}")
        for mismatch in mismatches:
            print(f"{'mismatch':<12} {mismatch}")
        print()
        print(
            "gate: "
            + ("every component is the pinned version" if ok else "NOT satisfied")
        )
        print(f"human acceptance: {matrix['accepted']['state']}")

    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
