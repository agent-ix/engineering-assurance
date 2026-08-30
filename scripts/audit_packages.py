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

import yaml

ROOT = Path(__file__).parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

try:
    from scripts.check_content_rights import text_findings
except ModuleNotFoundError:  # Direct execution puts scripts/ first on sys.path.
    from check_content_rights import text_findings
from engineering_assurance.discovery import (  # noqa: E402
    CANONICAL_SKILL,
    EXPECTED_WORKFLOWS,
    validate_discovery,
)

VERSION = "0.2.0"
PILOT_ROOT = Path("pilots/assurance-workflows")
WORKFLOW_NAMES = tuple(sorted(EXPECTED_WORKFLOWS))
ROOT_DATA_FILES = (
    Path(".claude-plugin/plugin.json"),
    Path(".codex-plugin/plugin.json"),
    Path(".github/plugin/plugin.json"),
    Path("opencode.json"),
    PILOT_ROOT / "README.md",
    PILOT_ROOT / "SKILL.md",
    PILOT_ROOT / "scripts/invariants.js",
    *(PILOT_ROOT / "workflows" / name / "def.yaml" for name in WORKFLOW_NAMES),
)


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


def npm_allowlist() -> set[str]:
    """Return the explicit npm archive contract (TC-015, TC-018, TC-040)."""
    module_members = {
        "manifest.yaml",
        *{
            path.relative_to(ROOT / "engineering_assurance").as_posix()
            for directory in ("schemas", "skeletons")
            for path in (ROOT / "engineering_assurance" / directory).glob("*")
        },
    }
    onboarding_members = {
        path.relative_to(ROOT).as_posix()
        for path in (
            ROOT / "engineering_assurance" / "skills" / "assurance-onboarding"
        ).rglob("*")
        if path.is_file()
    }
    return {
        "CONTENT_RIGHTS.md",
        "LICENSE",
        "README.md",
        "package.json",
        "engineering_assurance/INSTALL.md",
        "engineering_assurance/manifest.yaml",
        *(path.as_posix() for path in ROOT_DATA_FILES),
        *module_members,
        *onboarding_members,
    }


def assert_module_root(module_root: Path) -> None:
    expected = {"manifest.yaml", "schemas", "skeletons"}
    missing = sorted(name for name in expected if not (module_root / name).exists())
    if missing:
        raise SystemExit(f"installed module root is incomplete: {missing}")


def assert_workflow_equivalence(bundle_root: Path) -> None:
    canonical_root = bundle_root / CANONICAL_SKILL / "workflows"
    pilot_root = bundle_root / PILOT_ROOT / "workflows"
    for name in WORKFLOW_NAMES:
        canonical = yaml.safe_load((canonical_root / name / "def.yaml").read_text())
        pilot = yaml.safe_load((pilot_root / name / "def.yaml").read_text())
        if canonical != pilot:
            raise SystemExit(f"installed pilot workflow diverges: {name}")


def audit_installed_bundle(bundle_root: Path, module_root: Path) -> None:
    """Exercise installed module and discovery trees (TC-016, TC-017)."""
    assert_module_root(module_root)
    validate_discovery(bundle_root)
    assert_workflow_equivalence(bundle_root)
    escaping = [path for path in bundle_root.rglob("*") if path.is_symlink()]
    if escaping:
        raise SystemExit(f"installed bundle contains links: {escaping}")


def canonical_members(bundle_root: Path) -> dict[str, bytes]:
    root = bundle_root / CANONICAL_SKILL
    return {
        path.relative_to(root).as_posix(): path.read_bytes()
        for path in root.rglob("*")
        if path.is_file()
    }


def main() -> int:
    """Audit exact archives and real installs (TC-014..TC-018, TC-040)."""
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
            metadata_root = f"engineering_assurance-{VERSION}.dist-info"
            expected_metadata = {
                f"{metadata_root}/{name}"
                for name in ("METADATA", "RECORD", "WHEEL", "top_level.txt")
            }
            license_candidates = {
                f"{metadata_root}/LICENSE",
                f"{metadata_root}/licenses/LICENSE",
            }
            emitted_licenses = names & license_candidates
            if len(emitted_licenses) != 1:
                raise SystemExit(
                    f"wheel license member mismatch: {sorted(emitted_licenses)}"
                )
            data_root = f"engineering_assurance-{VERSION}.data/data"
            expected_data = {
                f"{data_root}/{path.as_posix()}" for path in ROOT_DATA_FILES
            }
            expected_wheel = (
                expected_package
                | expected_data
                | expected_metadata
                | emitted_licenses
            )
            if names != expected_wheel:
                extra = sorted(names - expected_wheel)
                missing = sorted(expected_wheel - names)
                raise SystemExit(
                    f"wheel member mismatch: extra={extra}, missing={missing}"
                )
            wheel_license = archive.read(next(iter(emitted_licenses)))
            if wheel_license != (ROOT / "LICENSE").read_bytes():
                raise SystemExit("wheel license does not match the canonical file")
            metadata = archive.read(f"{metadata_root}/METADATA").decode("utf-8")
            if "Classifier: Private :: Do Not Upload\n" not in metadata:
                raise SystemExit("wheel lacks the private-package refusal classifier")
            for name in names:
                if name.endswith("/"):
                    continue
                data = archive.read(name)
                audit_text_member(name, data)

        wheel_install = output / "wheel-install"
        run(
            [
                sys.executable,
                "-m",
                "pip",
                "install",
                "--no-index",
                "--no-deps",
                "--target",
                str(wheel_install),
                str(wheels[0]),
            ]
        )
        audit_installed_bundle(
            wheel_install,
            wheel_install / "engineering_assurance",
        )

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
        allowed = npm_allowlist()
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

        npm_install = output / "npm-install"
        run(
            [
                "npm",
                "install",
                "--ignore-scripts",
                "--offline",
                "--prefix",
                str(npm_install),
                str(archives[0]),
            ],
            environment=environment,
        )
        npm_bundle = (
            npm_install
            / "node_modules"
            / "@agent-ix"
            / "engineering-assurance"
        )
        audit_installed_bundle(npm_bundle, npm_bundle)
        if canonical_members(wheel_install) != canonical_members(npm_bundle):
            raise SystemExit("installed canonical bundles are not byte-identical")
    print("package audit passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
