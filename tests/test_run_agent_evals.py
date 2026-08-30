from __future__ import annotations

import hashlib
import os
from pathlib import Path

from scripts.run_agent_evals import (
    build_command,
    file_identity,
    manifest_version,
    search_path,
)


def test_runner_builds_one_explicit_live_host_command(tmp_path: Path) -> None:
    """Trace: FR-006-AC-1, FR-006-AC-3, TC-031, TC-049."""
    report = tmp_path / "opencode-canary.json"

    command = build_command(
        "/tools/cli-evals",
        agent="opencode",
        run="canary",
        model="huggingface/Qwen/Qwen3-Coder-Next",
        keep=True,
        report=report,
        suite=Path("evals/suite.mjs"),
    )

    assert command == [
        "/tools/cli-evals",
        "run",
        "--suite",
        "evals/suite.mjs",
        "--canary",
        "--agent",
        "opencode",
        "--model",
        "huggingface/Qwen/Qwen3-Coder-Next",
        "--keep",
        "--report",
        str(report),
    ]


def test_runner_prefers_ignored_repository_tool_shims(tmp_path: Path) -> None:
    """Trace: FR-006-AC-1, TC-031."""
    assert search_path("/usr/bin", local_bin=tmp_path).split(os.pathsep) == [
        str(tmp_path),
        "/usr/bin",
    ]


def test_runner_snapshots_immutable_file_identity(tmp_path: Path) -> None:
    """Trace: FR-006-AC-2, TC-032."""
    artifact = tmp_path / "artifact.yaml"
    artifact.write_text("name: fixed\nversion: 1.2.3\n")

    assert manifest_version(artifact) == "1.2.3"
    assert file_identity("fixed", "1.2.3", artifact) == {
        "name": "fixed",
        "version": "1.2.3",
        "digest": hashlib.sha256(artifact.read_bytes()).hexdigest(),
    }
