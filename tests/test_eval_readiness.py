from __future__ import annotations

from pathlib import Path

from scripts.check_eval_readiness import REQUIRED_COMMANDS, probe


def test_readiness_requires_harness_hosts_and_domain_tools(tmp_path: Path) -> None:
    """Trace: FR-006-AC-1, FR-006-AC-3, TC-031, TC-033."""
    assert REQUIRED_COMMANDS == (
        "cli-evals",
        "claude",
        "codex",
        "opencode",
        "copilot",
        "quire",
        "quoin",
        "ix-flow",
    )
    result = probe("opencode", search_path=str(tmp_path))
    assert result.available is False
    assert result.diagnostic == "executable-not-found"
