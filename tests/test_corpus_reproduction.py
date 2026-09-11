"""FR-011-AC-9 — the accepted corpus reproduces from its recorded sources.

This is the one compatibility-corpus criterion with no native Rust home. The
builder that reproduces the corpus lives beside the corpus, in the pinned
`agent-ix/qa-corpus` submodule, and it reads the source repositories the corpus
was built from. Porting it here would mean copying another repository's build
tool into this one and making cross-repository checkouts a dependency of this
repository's gate, which is the opposite of what the corpus pin is for.

So it stays where it is, invoked rather than reimplemented. Every other FR-011
criterion is verified natively against the retained bytes in
`tests/compatibility_corpus.rs`; this one asks the stronger question those
cannot: are the retained bytes still what the recorded sources produce?

Skipped where the source repositories are not checked out — the corpus is
self-verifying offline, and this is the stronger check a maintainer runs where
the sources exist. Skipping is stated, never silent. Set
`ASSURANCE_SOURCE_ROOT` to the directory the source repositories sit in when
running from a worktree, which is one level deeper than they are.
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
CORPUS_SUBMODULE = REPO_ROOT / "corpus"
SOURCE_REPOSITORIES = ("quire-contract-ir", "quire-code-rs", "quoin")


def test_the_corpus_reproduces_from_its_recorded_sources() -> None:
    """Trace: FR-011-AC-9, TC-077."""
    if not (CORPUS_SUBMODULE / ".git").exists():
        pytest.skip(
            "the qa-corpus submodule is not checked out; run "
            "`git submodule update --init corpus`"
        )

    # A worktree lives one directory deeper than the checkout the source
    # repositories sit beside, so deriving the root from this file alone would
    # make the check permanently skip in every worktree. An explicit
    # ASSURANCE_SOURCE_ROOT wins, which is also what the builder itself reads.
    checkouts = Path(os.environ.get("ASSURANCE_SOURCE_ROOT") or REPO_ROOT.parent)
    missing = [
        repository
        for repository in SOURCE_REPOSITORIES
        if not (checkouts / repository / ".git").exists()
    ]
    if missing:
        pytest.skip(f"source repositories are not checked out: {', '.join(missing)}")

    result = subprocess.run(
        ["python3", "scripts/build_compatibility_corpus.py", "--check"],
        cwd=CORPUS_SUBMODULE,
        env={**os.environ, "ASSURANCE_SOURCE_ROOT": str(checkouts)},
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
