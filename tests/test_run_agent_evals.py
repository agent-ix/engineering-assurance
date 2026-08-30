from __future__ import annotations

import os
from pathlib import Path

from scripts.run_agent_evals import build_command, search_path


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
