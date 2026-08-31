#!/usr/bin/env python3
"""Fail closed unless traceability and retained agent evidence are complete."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import Any, Mapping, Sequence

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from engineering_assurance.eval_reports import (  # noqa: E402
    aggregate_report_collection,
    load_cli_eval_reports,
)
from engineering_assurance.evaluation import SUPPORTED_HOSTS  # noqa: E402
from scripts.run_agent_evals import (  # noqa: E402
    command_identity,
    runtime_command_identity,
    search_path,
)

SHA = re.compile(r"^[0-9a-f]{40}$")
DIGEST = re.compile(r"^[0-9a-f]{64}$")
PARTIAL_CENSUS_REASONS = frozenset(
    {
        "hollow-denominator",
        "marker-form-mismatch",
        "section-matches-nothing",
    }
)


def _mapping(value: Any) -> Mapping[str, Any]:
    return value if isinstance(value, Mapping) else {}


def _sequence(value: Any) -> list[Any]:
    return value if isinstance(value, list) else []


def _local_diagnostic(diagnostic: Mapping[str, Any], root: Path) -> bool:
    raw = diagnostic.get("path")
    if not isinstance(raw, str) or not raw:
        return True
    path = Path(raw)
    if not path.is_absolute():
        return True
    try:
        path.resolve().relative_to(root.resolve())
    except ValueError:
        return False
    return True


def coverage_failures(payload: Any, root: Path = ROOT) -> tuple[str, ...]:
    """Return fail-closed findings from one Quire coverage JSON document."""
    document = _mapping(payload)
    failures: list[str] = []
    totals = _mapping(document.get("totals"))
    backed = totals.get("backed")
    total = totals.get("total")
    if not isinstance(backed, int) or not isinstance(total, int) or total <= 0:
        failures.append("traceability totals are missing or have no population")
    elif backed != total:
        failures.append(f"traceability is {backed}/{total}, expected complete backing")

    for field, label in (
        ("unbacked_rows", "unbacked rows"),
        ("status_lies", "status lies"),
        ("untracked_symbols", "untracked symbols"),
    ):
        items = _sequence(document.get(field))
        if items:
            names = []
            for item in items:
                value = _mapping(item)
                names.append(
                    str(
                        value.get("row_id")
                        or value.get("trace_id")
                        or value.get("reference")
                        or "unknown"
                    )
                )
            failures.append(f"{label}: {', '.join(names)}")

    groups = _sequence(document.get("groups"))
    test_cases = next(
        (
            _mapping(group)
            for group in groups
            if _mapping(group).get("document") == "spec/tests.md"
            and _mapping(group).get("target") == "test-case"
        ),
        None,
    )
    if test_cases is None:
        failures.append("test-case traceability group is missing")
    elif test_cases.get("backed") != 51 or test_cases.get("total") != 51:
        failures.append(
            "test-case traceability is "
            f"{test_cases.get('backed')}/{test_cases.get('total')}, expected 51/51"
        )

    for item in _sequence(document.get("diagnostics")):
        diagnostic = _mapping(item)
        reason = diagnostic.get("reason")
        if reason in PARTIAL_CENSUS_REASONS and _local_diagnostic(diagnostic, root):
            failures.append(
                f"repository traceability census is partial: {reason}: "
                f"{diagnostic.get('path', 'unknown')}"
            )
    return tuple(failures)


def aggregate_metadata_failures(payload: Any) -> tuple[str, ...]:
    """Return findings from the complete-only aggregate metadata contract."""
    document = _mapping(payload)
    failures: list[str] = []
    if document.get("revision") != "evaluation-aggregate-v1":
        failures.append("evaluation aggregate revision is unsupported")
    source = document.get("source_revision")
    if not isinstance(source, str) or SHA.fullmatch(source) is None:
        failures.append("evaluation source revision is not an immutable SHA")
    required = document.get("required_cells")
    complete = document.get("complete_cells")
    if required != 28 or complete != 28:
        failures.append(
            f"evaluation aggregate is {complete}/{required}, expected 28/28"
        )
    if document.get("ok") is not True or document.get("failures") != []:
        failures.append("evaluation aggregate is not a clean pass")
    models = _mapping(document.get("models"))
    if set(models) != set(SUPPORTED_HOSTS) or not all(
        isinstance(value, str) and value for value in models.values()
    ):
        failures.append("evaluation model selection does not name the four hosts")
    if not _sequence(document.get("reports")):
        failures.append("evaluation aggregate names no retained reports")
    return tuple(failures)


def repository_revision(root: Path = ROOT) -> str:
    """Resolve the immutable revision that release evidence must describe."""
    completed = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
        timeout=15,
    )
    revision = completed.stdout.strip()
    if completed.returncode != 0 or SHA.fullmatch(revision) is None:
        raise RuntimeError("unable to resolve current source revision")
    return revision


def governing_file_failures(
    identities: Mapping[str, Mapping[str, Any]],
    files: Mapping[str, Path],
) -> tuple[str, ...]:
    """Compare current governing bytes with retained SHA-256 identities."""
    failures: list[str] = []
    for name, path in files.items():
        identity = _mapping(identities.get(name))
        expected = identity.get("digest")
        if not path.is_file():
            failures.append(f"governing file is missing: {name}")
            continue
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if not isinstance(expected, str) or DIGEST.fullmatch(expected) is None:
            failures.append(f"governing file identity is invalid: {name}")
        elif actual != expected:
            failures.append(f"governing file digest changed: {name}")
    return tuple(failures)


def _read_json(path: Path, label: str) -> tuple[dict[str, Any], tuple[str, ...]]:
    try:
        payload = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        return {}, (f"{label} is unreadable: {error}",)
    if not isinstance(payload, dict):
        return {}, (f"{label} is not an object",)
    return payload, ()


def retained_report_paths(
    aggregate: Mapping[str, Any], root: Path
) -> tuple[tuple[Path, ...], tuple[str, ...]]:
    paths: list[Path] = []
    failures: list[str] = []
    for item in _sequence(aggregate.get("reports")):
        record = _mapping(item)
        raw = record.get("path")
        expected = record.get("digest")
        if not isinstance(raw, str) or not raw:
            failures.append("retained report path is missing")
            continue
        relative = Path(raw)
        if relative.is_absolute() or ".." in relative.parts:
            failures.append(f"retained report path is not confined: {raw}")
            continue
        path = root / relative
        try:
            path.resolve().relative_to(root.resolve())
        except ValueError:
            failures.append(f"retained report path escapes the repository: {raw}")
            continue
        if not path.is_file():
            failures.append(f"retained report is missing: {raw}")
            continue
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if not isinstance(expected, str) or actual != expected:
            failures.append(f"retained report digest changed: {raw}")
            continue
        paths.append(path)
    return tuple(paths), tuple(failures)


def _retained_governing(
    report_paths: Sequence[Path],
) -> tuple[dict[str, Mapping[str, Any]], dict[str, Mapping[str, Any]], tuple[str, ...]]:
    shared: dict[str, Mapping[str, Any]] = {}
    workflows: dict[str, Mapping[str, Any]] = {}
    failures: list[str] = []
    for path in report_paths:
        payload, errors = _read_json(path, f"retained report {path}")
        failures.extend(errors)
        for result in _sequence(payload.get("results")):
            runs = _sequence(_mapping(result).get("runs"))
            if len(runs) != 1 or _mapping(runs[0]).get("ok") is not True:
                continue
            checks = _mapping(_mapping(runs[0]).get("checks"))
            evaluation = _mapping(checks.get("evaluation_result"))
            governing = _mapping(evaluation.get("governing"))
            for name in (
                "module",
                "plugin",
                "skill",
                "quire",
                "quoin",
                "ix_flow",
                "schema",
                "producer",
            ):
                identity = _mapping(governing.get(name))
                if name in shared and shared[name] != identity:
                    failures.append(f"retained governing identity drift: {name}")
                elif identity:
                    shared[name] = identity
            workflow = _mapping(governing.get("workflow"))
            workflow_name = workflow.get("name")
            if isinstance(workflow_name, str) and workflow_name:
                if workflow_name in workflows and workflows[workflow_name] != workflow:
                    failures.append(
                        f"retained workflow identity drift: {workflow_name}"
                    )
                workflows[workflow_name] = workflow
    return shared, workflows, tuple(failures)


def _current_tool_failures(
    identities: Mapping[str, Mapping[str, Any]], root: Path
) -> tuple[str, ...]:
    failures: list[str] = []
    selected_path = search_path(
        os.environ.get("PATH"), local_bin=root / ".agent-evals/bin"
    )
    commands = {"quire": "quire", "quoin": "quoin"}
    for name, command in commands.items():
        try:
            current = command_identity(command, search_path_value=selected_path)
        except SystemExit as error:
            failures.append(str(error))
            continue
        if current != _mapping(identities.get(name)):
            failures.append(f"governing executable identity changed: {name}")
    for name, command in {"ix_flow": "ix-flow", "producer": "cli-evals"}.items():
        try:
            current = runtime_command_identity(command, search_path_value=selected_path)
        except SystemExit as error:
            failures.append(str(error))
            continue
        if current != _mapping(identities.get(name)):
            failures.append(f"governing executable identity changed: {name}")
    return tuple(failures)


def verify_aggregate(path: Path, root: Path = ROOT) -> tuple[str, ...]:
    """Reconcile aggregate metadata, report bytes, envelopes, and current inputs."""
    aggregate, read_failures = _read_json(path, "evaluation aggregate")
    failures = [*read_failures, *aggregate_metadata_failures(aggregate)]
    source = aggregate.get("source_revision")
    if isinstance(source, str) and SHA.fullmatch(source) is not None:
        try:
            current_revision = repository_revision(root)
        except RuntimeError as error:
            failures.append(str(error))
        else:
            if source != current_revision:
                failures.append(
                    "evaluation source revision differs from current repository HEAD"
                )
    report_paths, report_failures = retained_report_paths(aggregate, root)
    failures.extend(report_failures)
    if report_paths and isinstance(source, str) and SHA.fullmatch(source):
        collection = load_cli_eval_reports(report_paths, source)
        recomputed = aggregate_report_collection(collection)
        if not recomputed.ok or recomputed.complete_cells != 28:
            failures.extend(collection.errors)
            failures.extend(recomputed.failures)
        if dict(collection.models) != _mapping(aggregate.get("models")):
            failures.append("aggregate model selections differ from retained reports")
        if list(collection.failed_attempts) != _sequence(
            aggregate.get("failed_attempts")
        ):
            failures.append("aggregate failed-attempt history differs from reports")

        shared, workflows, governing_failures = _retained_governing(report_paths)
        failures.extend(governing_failures)
        files = {
            "module": root / "engineering_assurance/manifest.yaml",
            "plugin": root / ".codex-plugin/plugin.json",
            "skill": root
            / "engineering_assurance/skills/assurance-onboarding/SKILL.md",
            "schema": root / "evals/result-contract.mjs",
        }
        failures.extend(governing_file_failures(shared, files))
        workflow_files = {
            name: root
            / "engineering_assurance/skills/assurance-onboarding/workflows"
            / name
            / "def.yaml"
            for name in workflows
        }
        failures.extend(governing_file_failures(workflows, workflow_files))
        failures.extend(_current_tool_failures(shared, root))
    return tuple(dict.fromkeys(failures))


def verify_coverage(quire: str, root: Path = ROOT) -> tuple[str, ...]:
    """Run Quire's machine-readable reconciliation and inspect matrix state."""
    completed = subprocess.run(
        [quire, "coverage", "--scope", str(root), "--json"],
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
    )
    if completed.stderr:
        sys.stderr.write(completed.stderr)
    if completed.returncode != 0:
        return (f"quire coverage exited {completed.returncode}",)
    try:
        payload = json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        return (f"quire coverage returned invalid JSON: {error}",)
    failures = list(coverage_failures(payload, root))
    matrix = root / "spec/tests.md"
    try:
        matrix_text = matrix.read_text()
    except OSError as error:
        failures.append(f"test matrix is unreadable: {error}")
    else:
        if "🚧" in matrix_text or "⛔" in matrix_text or "❌" in matrix_text:
            failures.append("test matrix retains non-passing status markers")
    return tuple(failures)


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    evidence = parser.add_mutually_exclusive_group(required=True)
    evidence.add_argument("--aggregate", type=Path)
    evidence.add_argument("--traceability-only", action="store_true")
    parser.add_argument("--quire", default="quire")
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    failures = list(verify_coverage(args.quire))
    if args.aggregate is not None:
        aggregate = (
            args.aggregate if args.aggregate.is_absolute() else ROOT / args.aggregate
        )
        failures.extend(verify_aggregate(aggregate))
    if failures:
        print("integration evidence failed")
        for failure in dict.fromkeys(failures):
            print(f"- {failure}")
        return 1
    if args.aggregate is None:
        print("integration evidence passed: traceability complete")
    else:
        print("integration evidence passed: traceability complete; evaluations 28/28")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
