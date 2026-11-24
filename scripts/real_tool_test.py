#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX

"""Run Engineering Assurance's available real-tool checks and retain the result."""

from __future__ import annotations

import datetime as dt
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TRACE_IDS = {
    "syft": ["FR-038-AC-1", "FR-038-AC-2", "FR-038-AC-7", "FR-038-AC-10", "FR-039-AC-1", "FR-039-AC-2", "FR-039-AC-3", "FR-039-AC-4", "FR-039-AC-8", "FR-039-AC-9"],
    "osv-scanner": ["FR-044-AC-1", "FR-044-AC-2", "FR-044-AC-5", "FR-044-AC-8", "FR-044-AC-13", "FR-044-AC-14"],
    "gitsign-signature": ["FR-040-AC-15", "FR-040-AC-20"],
    "gitsign-approval": ["FR-041-AC-26"],
}


def run_version(command: list[str]) -> tuple[str | None, str | None]:
    try:
        completed = subprocess.run(command, check=False, capture_output=True, text=True, timeout=10)
    except (OSError, subprocess.TimeoutExpired) as error:
        return None, str(error)
    output = (completed.stdout or completed.stderr).strip()
    if completed.returncode != 0:
        return None, f"command {command!r} exited {completed.returncode}: {output}"
    return output.splitlines()[0] if output else None, None


def executable_from_env(name: str) -> tuple[Path | None, str | None]:
    value = os.environ.get(name, "")
    path = Path(value) if value else None
    if path is None or not path.is_absolute() or not path.is_file() or not os.access(path, os.X_OK):
        return None, f"{name} must name an absolute executable file"
    return path, None


def resolve_locked_build(value: str | None = None, *, use_standard_fallback: bool = False) -> str | None:
    if value:
        candidate = Path(value).expanduser()
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return str(candidate.resolve())
        resolved = shutil.which(value)
        if resolved:
            return resolved
    if value not in (None, "locked-build.sh") or (value == "locked-build.sh" and not use_standard_fallback):
        return None
    standard = Path.home() / "dev/worktrees/locked-build.sh"
    if standard.is_file() and os.access(standard, os.X_OK):
        return str(standard.resolve())
    return None


def add_entry(
    entries: list[dict[str, object]],
    symbol: str,
    outcome: str,
    trace_ids: list[str],
    config: dict[str, str],
) -> None:
    entries.append({"symbol": symbol, "outcome": outcome, "traceIds": trace_ids, "config": config})


def missing_entry(
    entries: list[dict[str, object]], symbol: str, trace_ids: list[str], config: dict[str, str], reason: str
) -> None:
    config["execution_state"] = "not-executed"
    config["error"] = reason
    add_entry(entries, symbol, "error", trace_ids, config)


def main() -> int:
    root = ROOT
    entries: list[dict[str, object]] = []
    available: list[tuple[str, str]] = []
    failed = False

    syft, syft_error = executable_from_env("EA_TEST_SYFT")
    syft_version: str | None = None
    expected: str | None = None
    if syft is not None:
        try:
            pin = json.loads((root / "spec/fixtures/syft-supported-release.json").read_text())
            if pin.get("name") != "syft":
                raise ValueError("release pin does not name syft")
            expected = pin["version"]
        except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            syft_error = f"cannot read Syft release pin: {error}"
        else:
            syft_version, syft_version_error = run_version([str(syft), "--version"])
            if syft_version_error:
                syft_error = syft_version_error
            elif expected is not None and syft_version != f"syft {expected}":
                syft_error = f"Syft version {syft_version!r} does not match supported release syft {expected}"
            else:
                available.append(("syft", "syft_producer_cli"))
                syft_config = {"tool_version": syft_version or "unknown", "supported_version": str(expected)}
    if syft is None or syft_error:
        syft_config = {"tool_version": syft_version or "unavailable"}
        if expected is not None:
            syft_config["supported_version"] = expected
        missing_entry(entries, "syft_producer_cli::real_tool_checks", TRACE_IDS["syft"], syft_config, syft_error or "Syft unavailable")
        failed = True

    scanner, scanner_error = executable_from_env("EA_TEST_OSV_SCANNER")
    scanner_errors = [scanner_error] if scanner_error else []
    scanner_version: str | None = None
    scanner_version_error: str | None = None
    if scanner is not None:
        scanner_version, scanner_version_error = run_version([str(scanner), "--version"])
        if scanner_version_error:
            scanner_errors.append(scanner_version_error)
    database = os.environ.get("EA_TEST_OSV_DB", "")
    mirror_date = os.environ.get("EA_TEST_OSV_MIRROR_DATE", "")
    if not re.fullmatch(r"\d{4}-\d{2}-\d{2}", mirror_date):
        scanner_errors.append("EA_TEST_OSV_MIRROR_DATE must be supplied as YYYY-MM-DD")
    else:
        try:
            dt.date.fromisoformat(mirror_date)
        except ValueError:
            scanner_errors.append("EA_TEST_OSV_MIRROR_DATE is not a calendar date")
    database_path = Path(database) if database else None
    if database_path is None or not database_path.is_absolute() or not database_path.is_dir():
        scanner_errors.append("EA_TEST_OSV_DB must name an absolute local database directory")
    elif not any(database_path.glob("osv-scalibr/*/all.zip")):
        scanner_errors.append("EA_TEST_OSV_DB must contain osv-scalibr/<ecosystem>/all.zip")
    scanner_error = "; ".join(scanner_errors) or None
    if scanner is not None and scanner_error is None:
        available.append(("osv-scanner", "osv_scanner_producer_cli"))
        scanner_config = {"tool_version": scanner_version or "unknown", "osv_mirror_date": mirror_date}
    if scanner is None or scanner_error:
        scanner_config = {
            "tool_version": scanner_version or "unavailable",
            "osv_mirror_date": mirror_date or "not-provided",
        }
        missing_entry(entries, "osv_scanner_producer_cli::real_tool_checks", TRACE_IDS["osv-scanner"], scanner_config, scanner_error or "OSV-Scanner unavailable")
        failed = True

    gitsign = shutil.which("gitsign")
    gitsign_version: str | None = None
    git_version: str | None = None
    ssh_version: str | None = None
    gitsign_error: str | None = None
    if gitsign is None:
        gitsign_error = "gitsign executable is unavailable on PATH"
    else:
        gitsign_version, gitsign_error = run_version([gitsign, "--version"])
        version_token = gitsign_version.split()[-1] if gitsign_version else ""
        if gitsign_error is None and version_token not in {"v0.18.0", "0.18.0"}:
            gitsign_error = f"gitsign version {gitsign_version!r} is not the required v0.18.0"
    git = shutil.which("git")
    ssh_keygen = shutil.which("ssh-keygen")
    ssh = shutil.which("ssh")
    if git is not None:
        git_version, git_error = run_version([git, "--version"])
        if gitsign_error is None and git_error:
            gitsign_error = git_error
    if ssh is not None:
        ssh_version, ssh_error = run_version([ssh, "-V"])
        if gitsign_error is None and ssh_error:
            gitsign_error = ssh_error
    if gitsign_error is None and (git is None or ssh_keygen is None or ssh is None):
        gitsign_error = "gitsign checks require git, ssh, and ssh-keygen on PATH"

    signature_inputs = ["FR_040_LIVE_REPOSITORY", "FR_040_LIVE_COMMIT", "FR_040_LIVE_TAG", "FR_040_LIVE_IDENTITY", "FR_040_LIVE_ISSUER"]
    approval_inputs = ["FR_041_LIVE_REPOSITORY", "FR_041_LIVE_TAG", "FR_041_LIVE_IDENTITY", "FR_041_LIVE_ISSUER", "FR_041_LIVE_RECORD_ID", "FR_041_LIVE_RECORD_DIGEST"]
    gitsign_common = {
        "tool_version": gitsign_version or "unavailable",
        "git_version": git_version or "unavailable",
        "ssh_keygen_version": ssh_version or "unavailable",
    }
    gitsign_groups = [
        ("gitsign-signature", "signature_verify_gitsign", "signature_verify_gitsign::tc_292", TRACE_IDS["gitsign-signature"], signature_inputs),
        ("gitsign-approval", "approval_verify_gitsign", "approval_verify_gitsign::tc_1017", TRACE_IDS["gitsign-approval"], approval_inputs),
    ]
    signature_config: dict[str, str] = {}
    approval_config: dict[str, str] = {}
    for group, test_name, symbol, trace_ids, inputs in gitsign_groups:
        group_error = gitsign_error or next((f"required live-test input {key} is unavailable" for key in inputs if not os.environ.get(key)), None)
        repository_key = next((key for key in inputs if key.endswith("_REPOSITORY")), None)
        repository_url = os.environ.get(repository_key, "") if repository_key else ""
        if group_error is None and (not repository_url.startswith("https://") or "@" in repository_url.split("/", 3)[2]):
            group_error = f"{repository_key} must be a public HTTPS URL without embedded credentials"
        if group_error:
            config = {**gitsign_common, "required_inputs": ",".join(inputs)}
            missing_entry(entries, f"{symbol}", trace_ids, config, group_error)
            failed = True
            continue
        config = {
            **gitsign_common,
            **{key.removeprefix("FR_040_LIVE_").removeprefix("FR_041_LIVE_").lower(): os.environ[key] for key in inputs},
        }
        available.append((group, test_name))
        if group == "gitsign-signature":
            signature_config = config
        else:
            approval_config = config

    if available:
        locked_build_value = os.environ.get("LOCKED_BUILD")
        locked_build = resolve_locked_build(
            locked_build_value,
            use_standard_fallback=os.environ.get("LOCKED_BUILD_DEFAULT") == "1" or locked_build_value is None,
        )
        if not locked_build:
            print("LOCKED_BUILD must name the locked-build.sh admission wrapper", file=sys.stderr)
            return 2
        cargo = os.environ.get("CARGO", "cargo")
        command = [locked_build, str(root / "target"), cargo, "test", "--locked", "--features", "full"]
        test_symbols = {
            "syft_producer_cli": ("syft_producer_cli::real_tool_checks", TRACE_IDS["syft"], syft_config),
            "osv_scanner_producer_cli": ("osv_scanner_producer_cli::real_tool_checks", TRACE_IDS["osv-scanner"], scanner_config),
            "signature_verify_gitsign": ("signature_verify_gitsign::tc_292", TRACE_IDS["gitsign-signature"], signature_config),
            "approval_verify_gitsign": ("approval_verify_gitsign::tc_1017", TRACE_IDS["gitsign-approval"], approval_config),
        }
        for _, test_name in available:
            command.extend(["--test", test_name])
        command.append("--")
        command.append("--ignored")
        print("Focused command: " + " ".join(command), flush=True)
        log = tempfile.NamedTemporaryFile(prefix="ea-real-tool-test-", suffix=".log", delete=False)
        log_path = Path(log.name)
        log.close()
        with log_path.open("w", encoding="utf-8") as output:
            completed = subprocess.run(command, cwd=root, stdout=output, stderr=subprocess.STDOUT, check=False)
        print(f"Focused command log: {log_path}")
        print(log_path.read_text(encoding="utf-8", errors="replace"), end="")
        running: str | None = None
        summaries: dict[str, bool] = {}
        for line in log_path.read_text(encoding="utf-8", errors="replace").splitlines():
            match = re.search(r"Running (?:unittests )?(?:tests/)?([^ /()]+)\.rs \(", line)
            if match:
                running = match.group(1)
            if line.startswith("test result:") and running:
                summary = re.search(r"\b(\d+) passed; (\d+) failed\b", line)
                summaries[running.removesuffix(".rs")] = (
                    summary is not None and int(summary.group(1)) > 0 and int(summary.group(2)) == 0
                )
                running = None
        for test_name, (symbol, trace_ids, config) in test_symbols.items():
            if test_name not in {name for _, name in available}:
                continue
            outcome = "pass" if summaries.get(test_name) else "error"
            entry_config = dict(config)
            entry_config["execution_state"] = "executed" if test_name in summaries else "not-executed"
            if test_name not in summaries:
                entry_config["error"] = f"test binary produced no result summary; cargo exit={completed.returncode}"
            add_entry(entries, symbol, outcome, trace_ids, entry_config)
            failed |= outcome != "pass"
        failed |= completed.returncode != 0

    result_file = tempfile.NamedTemporaryFile(prefix="ea-real-tool-test-", suffix=".json", delete=False, mode="w", encoding="utf-8")
    with result_file:
        json.dump({"entries": entries}, result_file, sort_keys=True)
        result_file.write("\n")
    tool_version, tool_error = run_version(["quoin", "--version"])
    if tool_error:
        print(f"cannot record real-tool result: {tool_error}", file=sys.stderr)
        return 2
    record = subprocess.run(
        ["quoin", "evidence", "record", "--suite", "SUITE-004", "--tool", f"engineering-assurance real-tool-test ({tool_version})", "--adapter", "entries", "--kind", "Integration", "--repo", str(root), "--results", result_file.name],
        cwd=root,
        check=False,
    )
    if record.returncode != 0:
        return record.returncode
    if failed:
        print("real-tool-test recorded an error or not-executed entry; no passing live-tool result is claimed", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
