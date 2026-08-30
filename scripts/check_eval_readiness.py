#!/usr/bin/env python3
"""Fail closed unless every live onboarding-evaluation executable is available."""

from __future__ import annotations

import json
import shutil
import subprocess
from dataclasses import asdict, dataclass

REQUIRED_COMMANDS = (
    "cli-evals",
    "claude",
    "codex",
    "opencode",
    "copilot",
    "quire",
    "quoin",
    "ix-flow",
)


@dataclass(frozen=True)
class CommandProbe:
    command: str
    available: bool
    executable: str | None
    version: str | None
    diagnostic: str | None = None


def probe(command: str, *, search_path: str | None = None) -> CommandProbe:
    executable = shutil.which(command, path=search_path)
    if executable is None:
        return CommandProbe(command, False, None, None, "executable-not-found")
    completed = subprocess.run(
        [executable, "--version"],
        check=False,
        capture_output=True,
        text=True,
        timeout=15,
    )
    lines = (completed.stdout or completed.stderr).strip().splitlines()
    if completed.returncode != 0 or not lines:
        return CommandProbe(
            command,
            False,
            executable,
            None,
            f"version-probe-exit-{completed.returncode}",
        )
    return CommandProbe(command, True, executable, lines[0])


def main() -> int:
    results = [probe(command) for command in REQUIRED_COMMANDS]
    ready = all(result.available for result in results)
    print(
        json.dumps(
            {
                "ready": ready,
                "required_cells": 28,
                "commands": [asdict(result) for result in results],
            },
            indent=2,
        )
    )
    return 0 if ready else 1


if __name__ == "__main__":
    raise SystemExit(main())
