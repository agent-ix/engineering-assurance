#!/usr/bin/env python3
"""Build a complete-only aggregate from retained CLI-agent reports."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Sequence

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from engineering_assurance.eval_reports import (  # noqa: E402
    aggregate_report_collection,
    load_cli_eval_reports,
)


def _source_revision() -> str:
    completed = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return completed.stdout.strip()


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", type=Path, action="append", required=True)
    parser.add_argument("--source-revision", default=None)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    source_revision = args.source_revision or _source_revision()
    reports = tuple(path if path.is_absolute() else ROOT / path for path in args.report)
    collection = load_cli_eval_reports(reports, source_revision)
    aggregate = aggregate_report_collection(collection)
    payload = {
        "revision": "evaluation-aggregate-v1",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "source_revision": source_revision,
        "reports": [
            {"path": str(path.relative_to(ROOT)), "digest": _digest(path)}
            for path in reports
            if path.is_file() and path.is_relative_to(ROOT)
        ],
        "models": dict(collection.models),
        "failed_attempts": list(collection.failed_attempts),
        "required_cells": aggregate.required_cells,
        "complete_cells": aggregate.complete_cells,
        "ok": aggregate.ok,
        "failures": list(aggregate.failures),
    }
    output = args.output if args.output.is_absolute() else ROOT / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    print(
        f"evaluation aggregate: {aggregate.complete_cells}/"
        f"{aggregate.required_cells} complete"
    )
    print(f"report: {output}")
    if aggregate.failures:
        for failure in aggregate.failures:
            print(f"- {failure}")
    return 0 if aggregate.ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
