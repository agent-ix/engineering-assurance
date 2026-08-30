#!/usr/bin/env python3
"""Run the onboarding matrix behind one stable repository command."""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
from pathlib import Path
from typing import Sequence

ROOT = Path(__file__).resolve().parents[1]
LOCAL_BIN = ROOT / ".agent-evals" / "bin"
SUITE = ROOT / "evals" / "cli-agent-evals.config.mjs"


def search_path(base_path: str | None, *, local_bin: Path = LOCAL_BIN) -> str:
    entries = [str(local_bin)]
    if base_path:
        entries.append(base_path)
    return os.pathsep.join(entries)


def build_command(
    executable: str,
    *,
    agent: str,
    run: str,
    model: str | None,
    keep: bool,
    report: Path,
    suite: Path = SUITE,
) -> list[str]:
    command = [
        executable,
        "run",
        "--suite",
        str(suite),
        f"--{run}",
        "--agent",
        agent,
    ]
    if model:
        command.extend(["--model", model])
    if keep:
        command.append("--keep")
    command.extend(["--report", str(report)])
    return command


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--agent",
        required=True,
        choices=("claude", "codex", "opencode", "copilot"),
    )
    parser.add_argument("--run", choices=("canary", "all"), default="canary")
    parser.add_argument("--model")
    parser.add_argument("--keep", action="store_true")
    parser.add_argument("--report", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    env = os.environ.copy()
    env["PATH"] = search_path(env.get("PATH"))
    cli_evals = shutil.which("cli-evals", path=env["PATH"])
    host = shutil.which(args.agent, path=env["PATH"])
    if cli_evals is None:
        raise SystemExit("cli-evals is not available on PATH or in .agent-evals/bin")
    if host is None:
        raise SystemExit(
            f"{args.agent} is not available on PATH or in .agent-evals/bin"
        )

    report = args.report if args.report.is_absolute() else ROOT / args.report
    report.parent.mkdir(parents=True, exist_ok=True)
    command = build_command(
        cli_evals,
        agent=args.agent,
        run=args.run,
        model=args.model,
        keep=args.keep,
        report=report,
    )
    print(f"running {args.run} onboarding evaluation on {args.agent}")
    return subprocess.run(command, cwd=ROOT, env=env, check=False).returncode


if __name__ == "__main__":
    raise SystemExit(main())
