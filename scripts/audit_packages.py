#!/usr/bin/env python3
"""Build packages locally and verify that their member sets are allowlisted."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path

from check_content_rights import text_findings

ROOT = Path(__file__).parents[1]


def audit_text_member(name: str, data: bytes) -> None:
    """Apply the repository text policy to bytes that will leave the tree."""
    if b"\x00" in data:
        raise SystemExit(f"package member is not text: {name}")
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as error:
        raise SystemExit(f"package member is not UTF-8: {name}") from error
    policy_name = "LICENSE" if Path(name).name == "LICENSE" else name
    findings = text_findings(policy_name, text)
    if findings:
        categories = sorted({finding.category for finding in findings})
        raise SystemExit(
            f"package member failed content-rights audit: {name}: {categories}"
        )


def run(
    command: list[str], *, environment: dict[str, str] | None = None
) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        env=environment,
    )
    if completed.returncode != 0:
        raise SystemExit(
            f"{command[0]} failed with exit {completed.returncode}:\n"
            f"{completed.stderr}"
        )
    return completed


def main() -> int:
    with tempfile.TemporaryDirectory(prefix="ea-package-audit-") as raw:
        output = Path(raw)
        run(
            [
                sys.executable,
                "-m",
                "pip",
                "wheel",
                ".",
                "--no-deps",
                "--no-build-isolation",
                "--wheel-dir",
                str(output),
            ]
        )
        wheels = list(output.glob("*.whl"))
        if len(wheels) != 1:
            raise SystemExit("expected one wheel")
        with zipfile.ZipFile(wheels[0]) as archive:
            names = set(archive.namelist())
            expected_package = {
                path.relative_to(ROOT).as_posix()
                for path in (ROOT / "engineering_assurance").rglob("*")
                if path.is_file() and "__pycache__" not in path.parts
            }
            metadata_root = "engineering_assurance-0.2.0.dist-info"
            expected_metadata = {
                f"{metadata_root}/{name}"
                for name in ("LICENSE", "METADATA", "RECORD", "WHEEL", "top_level.txt")
            }
            expected_wheel = expected_package | expected_metadata
            if names != expected_wheel:
                extra = sorted(names - expected_wheel)
                missing = sorted(expected_wheel - names)
                raise SystemExit(
                    f"wheel member mismatch: extra={extra}, missing={missing}"
                )
            metadata = archive.read(f"{metadata_root}/METADATA").decode("utf-8")
            if "Classifier: Private :: Do Not Upload\n" not in metadata:
                raise SystemExit("wheel lacks the private-package refusal classifier")
            for name in names:
                if name.endswith("/"):
                    continue
                data = archive.read(name)
                audit_text_member(name, data)

        environment = os.environ.copy()
        environment["npm_config_cache"] = str(output / "npm-cache")
        packed = run(
            [
                "npm",
                "pack",
                "--json",
                "--pack-destination",
                str(output),
            ],
            environment=environment,
        )
        report = json.loads(packed.stdout)
        files = {item["path"] for item in report[0]["files"]}
        allowed = {
            "CONTENT_RIGHTS.md",
            "LICENSE",
            "README.md",
            "manifest.yaml",
            "package.json",
            *{
                path.relative_to(ROOT / "engineering_assurance").as_posix()
                for directory in ("schemas", "skeletons")
                for path in (ROOT / "engineering_assurance" / directory).glob("*")
            },
        }
        if files != allowed:
            extra = sorted(files - allowed)
            missing = sorted(allowed - files)
            raise SystemExit(f"npm member mismatch: extra={extra}, missing={missing}")
        archives = list(output.glob("*.tgz"))
        if len(archives) != 1:
            raise SystemExit("expected one npm archive")
        with tarfile.open(archives[0], "r:gz") as archive:
            archived_files = {
                member.name.removeprefix("package/")
                for member in archive.getmembers()
                if member.isfile()
            }
            if archived_files != allowed:
                extra = sorted(archived_files - allowed)
                missing = sorted(allowed - archived_files)
                raise SystemExit(
                    f"npm archive mismatch: extra={extra}, missing={missing}"
                )
            for member in archive.getmembers():
                if not member.isfile():
                    continue
                extracted = archive.extractfile(member)
                if extracted is None:
                    raise SystemExit(f"could not read npm member: {member.name}")
                audit_text_member(
                    member.name.removeprefix("package/"), extracted.read()
                )
    print("package audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
