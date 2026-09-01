from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
from pathlib import Path

import yaml
from jsonschema import Draft7Validator, FormatChecker

import engineering_assurance as package
from scripts import validate_manifest

ROOT = Path(__file__).parents[1]


def test_manifest_schema_resolves_from_installed_module_root(
    tmp_path: Path, monkeypatch,
) -> None:
    """Trace: FR-003-AC-3, TC-016 — installed modules are authoritative."""
    schema_path = tmp_path / "spec-artifacts-iso" / "module-manifest.schema.json"
    schema_path.parent.mkdir(parents=True)
    schema_path.write_text("{}")
    monkeypatch.delenv("MODULE_MANIFEST_SCHEMA", raising=False)
    monkeypatch.setenv("IX_FILAMENT_MODULES_PATH", str(tmp_path))
    assert validate_manifest.shared_manifest_schema() == schema_path


def manifest() -> dict:
    return yaml.safe_load(package.MANIFEST_PATH.read_text())


def frontmatter(path: Path) -> dict:
    match = re.match(r"---\n(.*?)\n---\n", path.read_text(), re.DOTALL)
    assert match is not None
    return yaml.safe_load(match.group(1))


def schema(name: str) -> dict:
    return json.loads(
        (package.PACKAGE_ROOT / "schemas" / f"{name}.json").read_text()
    )


def test_module_inventory_is_exact() -> None:
    data = manifest()
    assert data["version"] == "0.2.0"
    assert [item["name"] for item in data["artifact_types"]] == [
        "AssuranceProfile",
        "MeasurementPlan",
        "ArchitectureDescription",
        "ComponentAssuranceContract",
        "AssuranceArgument",
    ]
    assert data["lint_rules"] == []
    assert data["object_types"] == []
    assert "edge_types" not in data


def test_every_schema_and_skeleton_is_valid() -> None:
    for artifact in manifest()["artifact_types"]:
        schema_path = package.PACKAGE_ROOT / artifact["frontmatter_schema_ref"]
        skeleton_path = (
            package.PACKAGE_ROOT / "skeletons" / f"{artifact['name']}.md"
        )
        contract = json.loads(schema_path.read_text())
        Draft7Validator.check_schema(contract)
        errors = list(
            Draft7Validator(
                contract, format_checker=FormatChecker()
            ).iter_errors(frontmatter(skeleton_path))
        )
        assert errors == []
        body = skeleton_path.read_text()
        for locator in artifact["body_extraction"]["yield_pattern"]["match"].values():
            assert locator["required"] is True
            assert f"## {locator['after_heading']}" in body


def test_profile_v02_is_advisory_and_legacy_compatible() -> None:
    contract = schema("assurance-profile-frontmatter.schema")
    assert "profile_version" not in contract["required"]
    assert "profile_kind" not in contract["required"]
    impact = contract["$defs"]["impact"]["properties"]
    assert impact["verifiability"]["type"] == "object"
    assert impact["detect_before_harm"]["properties"]["control_ref"] == {
        "type": "string",
        "pattern": "^ix://",
    }
    classes = impact["verifiability"]["properties"]["class"]["enum"]
    assert classes == ["cheap-conclusive", "probabilistic", "proxy-only"]


def test_measurement_stages_and_statistical_design_are_explicit() -> None:
    contract = schema("measurement-plan-frontmatter.schema")
    assert contract["properties"]["metric"]["pattern"] == "^[a-z][a-z0-9_.-]*$"
    assert contract["properties"]["definition_version"] == {
        "type": "string",
        "minLength": 1,
    }
    assert contract["properties"]["stage"]["enum"] == [
        "observe",
        "baseline",
        "branch-comparison",
        "trend",
        "ratchet",
        "target",
        "gate",
    ]
    required = contract["$defs"]["statistical_design"]["required"]
    assert required == [
        "population",
        "sampling",
        "repetitions",
        "estimator",
        "error_model",
        "uncertainty",
        "decision_rule",
    ]


def test_component_contract_exposes_failure_and_control_boundaries() -> None:
    contract = schema("component-assurance-contract-frontmatter.schema")
    assert {
        "responsibility",
        "failure_behaviors",
        "version_pins",
        "controls",
        "isolation",
        "replacement",
    } <= set(contract["required"])
    assert contract["properties"]["kind"]["enum"] == [
        "deterministic",
        "stochastic",
        "human",
    ]


def test_argument_has_authored_claims_and_no_score() -> None:
    contract = schema("assurance-argument-frontmatter.schema")
    assert {"top_claim", "reasoning", "participants", "challenges"} <= set(
        contract["required"]
    )
    assert "score" not in json.dumps(contract).casefold()


def test_repository_has_only_governed_review_evidence() -> None:
    """Trace: StR-001-VC-1, TC-001."""
    assert not (ROOT / "examples").exists()
    assert not (ROOT / "research").exists()
    review_files = sorted((ROOT / "reviews").glob("*.md"))
    assert review_files
    assert not [path for path in (ROOT / "reviews").rglob("*") if path.is_dir()]
    for path in review_files:
        metadata = frontmatter(path)
        assert metadata["type"] == "SpecReview"
        assert metadata["analysis"] in {"code-review", "gap-analysis"}
    assert {path.name for path in (ROOT / "plan").iterdir()} == {
        "PLAN-001-assurance-onboarding",
        "PLAN-002-verification-semantics",
    }
    assert {path.name for path in (ROOT / "docs").iterdir()} == {
        "compatibility-matrix.md",
        "migration-contract.md",
        "structural-coverage.md",
        "verification-semantics",
    }


def test_module_payload_is_visible_to_git_and_rights_checks() -> None:
    expected = {
        path.relative_to(ROOT).as_posix()
        for path in package.PACKAGE_ROOT.rglob("*")
        if path.is_file() and "__pycache__" not in path.parts
    }
    completed = subprocess.run(
        [
            "git",
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
        cwd=ROOT,
        check=True,
        capture_output=True,
    )
    visible = {
        raw.decode() for raw in completed.stdout.split(b"\0") if raw
    }
    assert expected <= visible


def test_packages_are_private_and_have_no_release_configuration() -> None:
    npm = json.loads((ROOT / "package.json").read_text())
    assert npm["private"] is True
    assert "publishConfig" not in npm
    assert "prepublishOnly" in npm["scripts"]
    assert "Private :: Do Not Upload" in (ROOT / "setup.cfg").read_text()
    workflow_text = "\n".join(
        path.read_text() for path in (ROOT / ".github" / "workflows").glob("*.yml")
    )
    assert "publish" not in workflow_text.casefold()
    assert "upload-artifact" not in workflow_text.casefold()


def test_hosted_ci_is_manual_only() -> None:
    """Program invariant: opening or updating a PR must not dispatch hosted CI."""
    workflow = yaml.load(
        (ROOT / ".github" / "workflows" / "ci.yml").read_text(),
        Loader=yaml.BaseLoader,
    )
    assert set(workflow["on"]) == {"workflow_dispatch"}


def test_structural_coverage_never_collapses_unknowns_into_success() -> None:
    text = (ROOT / "docs" / "structural-coverage.md").read_text().casefold()
    assert "exactly once" in text
    assert "silently merged into success" in text
    assert "quality score" in text


def test_quire_accepts_every_skeleton_without_diagnostics() -> None:
    executable = os.environ.get("QUIRE_BIN") or shutil.which("quire")
    if executable is None:
        if os.environ.get("REQUIRE_QUIRE") == "1":
            raise AssertionError("quire is required")
        return
    documents = [
        str(path.relative_to(ROOT))
        for path in sorted((package.PACKAGE_ROOT / "skeletons").glob("*.md"))
    ]
    completed = subprocess.run(
        [
            executable,
            "validate",
            "--module",
            str(package.PACKAGE_ROOT.relative_to(ROOT)),
            "--strict",
            *documents,
        ],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr
    diagnostics = completed.stderr.splitlines()
    assert diagnostics
    assert all(line.startswith("UnknownEdgeType:") for line in diagnostics)
