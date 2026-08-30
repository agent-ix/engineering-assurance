#!/usr/bin/env python3
"""Run the onboarding matrix behind one stable repository command."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
from pathlib import Path
from typing import Sequence

ROOT = Path(__file__).resolve().parents[1]
LOCAL_BIN = ROOT / ".agent-evals" / "bin"
SUITE = ROOT / "evals" / "cli-agent-evals.config.mjs"
VERSION_PATTERN = re.compile(r"^version:\s*[\"']?([^\"'\s]+)", re.MULTILINE)


def search_path(base_path: str | None, *, local_bin: Path = LOCAL_BIN) -> str:
    entries = [str(local_bin)]
    if base_path:
        entries.append(base_path)
    return os.pathsep.join(entries)


def file_identity(name: str, version: str, path: Path) -> dict[str, str]:
    return {
        "name": name,
        "version": version,
        "digest": hashlib.sha256(path.resolve().read_bytes()).hexdigest(),
    }


def command_identity(command: str, *, search_path_value: str) -> dict[str, str]:
    executable = shutil.which(command, path=search_path_value)
    if executable is None:
        raise SystemExit(f"{command} is not available for the governing snapshot")
    completed = subprocess.run(
        [executable, "--version"],
        check=False,
        capture_output=True,
        text=True,
        timeout=15,
    )
    lines = (completed.stdout or completed.stderr).strip().splitlines()
    if completed.returncode != 0 or not lines:
        raise SystemExit(f"{command} version probe failed: {completed.returncode}")
    return file_identity(command, lines[0], Path(executable))


def runtime_package_identity(
    name: str, version: str, executable: Path
) -> dict[str, str]:
    """Bind a Node CLI identity to every file that can affect its runtime."""
    resolved_executable = executable.resolve()
    package_root = resolved_executable.parent.parent
    manifest = package_root / "package.json"
    dist = package_root / "dist"
    if not manifest.is_file() or not dist.is_dir():
        raise SystemExit(
            f"unable to locate the runtime package for {resolved_executable}"
        )

    runtime_files = [manifest, resolved_executable]
    runtime_files.extend(path for path in dist.rglob("*") if path.is_file())
    digest = hashlib.sha256()
    for path in sorted(set(runtime_files)):
        digest.update(path.relative_to(package_root).as_posix().encode())
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return {"name": name, "version": version, "digest": digest.hexdigest()}


def producer_identity(command: str, *, search_path_value: str) -> dict[str, str]:
    executable = shutil.which(command, path=search_path_value)
    if executable is None:
        raise SystemExit(f"{command} is not available for the governing snapshot")
    completed = subprocess.run(
        [executable, "--version"],
        check=False,
        capture_output=True,
        text=True,
        timeout=15,
    )
    lines = (completed.stdout or completed.stderr).strip().splitlines()
    if completed.returncode != 0 or not lines:
        raise SystemExit(f"{command} version probe failed: {completed.returncode}")
    return runtime_package_identity(command, lines[0], Path(executable))


def manifest_version(path: Path) -> str:
    match = VERSION_PATTERN.search(path.read_text())
    if match is None:
        raise SystemExit(f"version missing from {path.relative_to(ROOT)}")
    return match.group(1)


def source_revision() -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        timeout=15,
    )
    revision = completed.stdout.strip()
    if completed.returncode != 0 or not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise SystemExit("unable to resolve immutable source revision")
    return revision


def governing_snapshot(agent: str, *, search_path_value: str) -> dict[str, object]:
    manifest = ROOT / "engineering_assurance" / "manifest.yaml"
    plugin = ROOT / ".codex-plugin" / "plugin.json"
    skill = (
        ROOT / "engineering_assurance" / "skills" / "assurance-onboarding" / "SKILL.md"
    )
    contract = ROOT / "evals" / "result-contract.mjs"
    module_version = manifest_version(manifest)
    plugin_version = json.loads(plugin.read_text())["version"]
    workflows: dict[str, dict[str, str]] = {}
    for name in ("assurance-intake", "architecture-evaluation"):
        definition = (
            ROOT
            / "engineering_assurance"
            / "skills"
            / "assurance-onboarding"
            / "workflows"
            / name
            / "def.yaml"
        )
        workflows[name] = file_identity(
            name,
            manifest_version(definition),
            definition,
        )

    return {
        "source_revision": source_revision(),
        "host": command_identity(agent, search_path_value=search_path_value),
        "governing": {
            "module": file_identity("engineering-assurance", module_version, manifest),
            "plugin": file_identity(
                "engineering-assurance-plugin", plugin_version, plugin
            ),
            "skill": file_identity("assurance-onboarding", plugin_version, skill),
            "quire": command_identity("quire", search_path_value=search_path_value),
            "quoin": command_identity("quoin", search_path_value=search_path_value),
            "ix_flow": command_identity("ix-flow", search_path_value=search_path_value),
            "schema": file_identity(
                "evaluation-result-contract", "evaluation-result-v1", contract
            ),
            "producer": producer_identity(
                "cli-evals", search_path_value=search_path_value
            ),
        },
        "workflows": workflows,
    }


def build_command(
    executable: str,
    *,
    agent: str,
    run: str,
    filter_id: str | None = None,
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
    ]
    if filter_id:
        command.extend(["--filter", filter_id])
    else:
        command.append(f"--{run}")
    command.extend(["--agent", agent])
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
    parser.add_argument("--filter")
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

    snapshot = governing_snapshot(args.agent, search_path_value=env["PATH"])
    snapshot_path = ROOT / ".agent-evals" / f"governing-{args.agent}.json"
    snapshot_path.parent.mkdir(parents=True, exist_ok=True)
    snapshot_path.write_text(f"{json.dumps(snapshot, indent=2, sort_keys=True)}\n")
    env["EA_EVAL_GOVERNING_PATH"] = str(snapshot_path)

    report = args.report if args.report.is_absolute() else ROOT / args.report
    report.parent.mkdir(parents=True, exist_ok=True)
    command = build_command(
        cli_evals,
        agent=args.agent,
        run=args.run,
        filter_id=args.filter,
        model=args.model,
        keep=args.keep,
        report=report,
    )
    print(f"running {args.run} onboarding evaluation on {args.agent}")
    return subprocess.run(command, cwd=ROOT, env=env, check=False).returncode


if __name__ == "__main__":
    raise SystemExit(main())
