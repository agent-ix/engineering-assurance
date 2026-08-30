from __future__ import annotations

import shutil
from pathlib import Path

import pytest

from engineering_assurance import PACKAGE_ROOT
from engineering_assurance.onboarding import (
    OnboardingError,
    OnboardingRequest,
    inventory_repository,
    run_onboarding,
)


def copy_skeleton(root: Path, artifact_type: str, target: str) -> Path:
    destination = root / target
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(
        (PACKAGE_ROOT / "skeletons" / f"{artifact_type}.md").read_bytes()
    )
    return destination


def quire() -> str:
    executable = shutil.which("quire")
    if executable is None:
        pytest.skip("quire is unavailable")
    return executable


def request(root: Path, **changes) -> OnboardingRequest:
    values = {
        "repository_root": root,
        "decision_boundary": "one fictional candidate revision",
        "decision_owner": "release-owner",
    }
    values.update(changes)
    return OnboardingRequest(**values)


def test_existing_valid_profile_is_inventoried_and_reused(tmp_path: Path) -> None:
    """Trace: FR-001-AC-1, TC-004."""
    existing = copy_skeleton(tmp_path, "AssuranceProfile", "spec/AP-001.md")
    before = existing.read_bytes()
    result = run_onboarding(
        request(tmp_path, requested_artifact="AssuranceProfile"),
        quire_bin=quire(),
    )
    assert result.status == "reuse"
    assert result.artifact_path == "spec/AP-001.md"
    assert existing.read_bytes() == before


def test_unjustified_request_creates_no_generic_profile(tmp_path: Path) -> None:
    """Trace: FR-001-AC-2, TC-005."""
    result = run_onboarding(
        request(tmp_path, requested_artifact="AssuranceProfile")
    )
    assert result.status == "no-applicable-work"
    assert list(tmp_path.rglob("*.md")) == []


def test_justified_artifact_uses_skeleton_and_real_quire(tmp_path: Path) -> None:
    """Trace: FR-001-AC-3, TC-006."""
    result = run_onboarding(
        request(
            tmp_path,
            requested_artifact="AssuranceProfile",
            justification="material retained-result decision",
            target=Path("spec/AP-002.md"),
            frontmatter={
                "id": "AP-002",
                "title": "Fictional candidate decision profile",
                "owner": "release-owner",
            },
        ),
        quire_bin=quire(),
    )
    assert result.status == "authored"
    text = (tmp_path / "spec/AP-002.md").read_text()
    assert "id: AP-002" in text
    assert "# Fictional candidate decision profile" in text


def test_incomplete_boundary_requests_input_and_writes_nothing(tmp_path: Path) -> None:
    """Trace: FR-001-AC-4, TC-007."""
    result = run_onboarding(
        request(
            tmp_path,
            decision_boundary=None,
            requested_artifact="AssuranceProfile",
            justification="not actionable without the boundary",
            target=Path("spec/AP-003.md"),
            frontmatter={"id": "AP-003"},
        )
    )
    assert result.status == "needs-input"
    assert not (tmp_path / "spec/AP-003.md").exists()


def test_inventory_keeps_context_collections_separate(tmp_path: Path) -> None:
    """Trace: FR-001-AC-5, TC-008."""
    copy_skeleton(tmp_path, "MeasurementPlan", "spec/MP-001.md")
    decision = tmp_path / "decisions/release.md"
    decision.parent.mkdir()
    decision.write_text("---\ntype: DecisionRecord\n---\n# Decision\n")
    producer = tmp_path / ".github/workflows/assurance.yml"
    producer.parent.mkdir(parents=True)
    producer.write_text("name: fictional assurance producer\n")
    evidence = tmp_path / "spec/evidence/observation.json"
    evidence.parent.mkdir()
    evidence.write_text("{}\n")
    result = inventory_repository(tmp_path, quire_bin=quire())
    assert result.decisions == ["decisions/release.md"]
    assert [item.path for item in result.measurements] == ["spec/MP-001.md"]
    assert result.assurance_artifacts == []
    assert result.producer_configurations == [
        ".github/workflows/assurance.yml"
    ]
    assert result.evidence_references == ["spec/evidence/observation.json"]
    assert result.unresolved_inputs == []


def test_malformed_applicable_artifact_is_preserved_for_human(tmp_path: Path) -> None:
    """Trace: FR-001-AC-6, TC-044."""
    artifact = copy_skeleton(tmp_path, "AssuranceProfile", "spec/AP-bad.md")
    text = artifact.read_text().replace("owner: juniper-release-owner\n", "")
    artifact.write_text(text)
    before = artifact.read_bytes()
    result = run_onboarding(
        request(tmp_path, requested_artifact="AssuranceProfile"),
        quire_bin=quire(),
    )
    assert result.status == "needs-human-selection"
    assert result.inventory.assurance_artifacts[0].valid is False
    assert artifact.read_bytes() == before


def test_validation_failure_and_escaping_target_publish_nothing(
    tmp_path: Path,
) -> None:
    """Trace: FR-001-AC-7, TC-045."""
    invalid_target = Path("spec/AP-invalid.md")
    with pytest.raises(OnboardingError, match="Quire rejected"):
        run_onboarding(
            request(
                tmp_path,
                requested_artifact="AssuranceProfile",
                justification="fictional decision",
                target=invalid_target,
                frontmatter={"owner": None},
            ),
            quire_bin=quire(),
        )
    assert not (tmp_path / invalid_target).exists()
    assert list((tmp_path / "spec").glob("*.staged")) == []

    with pytest.raises(OnboardingError, match="confined relative path"):
        run_onboarding(
            request(
                tmp_path,
                requested_artifact="AssuranceProfile",
                justification="fictional decision",
                target=Path("../AP-escape.md"),
                frontmatter={"id": "AP-escape"},
            ),
            quire_bin=quire(),
        )
    assert not (tmp_path.parent / "AP-escape.md").exists()
