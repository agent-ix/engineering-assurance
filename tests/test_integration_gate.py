from __future__ import annotations

from pathlib import Path

from engineering_assurance.onboarding import OnboardingRequest, run_onboarding
from engineering_assurance.workflow import WorkflowBinding, WorkflowError, start_or_resume


def test_inventory_is_returned_before_any_recommendation(tmp_path: Path) -> None:
    """Trace: StR-001-VC-1, TC-001."""
    decision = tmp_path / "decisions/fictional.md"
    decision.parent.mkdir()
    decision.write_text("---\ntype: DecisionRecord\n---\n# Fictional decision\n")
    result = run_onboarding(
        OnboardingRequest(
            repository_root=tmp_path,
            decision_boundary=None,
            decision_owner="decision-owner",
        )
    )
    assert result.status == "needs-input"
    assert result.inventory.decisions == ["decisions/fictional.md"]


def test_unjustified_profile_is_never_scaffolded(tmp_path: Path) -> None:
    """Trace: StR-001-VC-2, TC-002."""
    result = run_onboarding(
        OnboardingRequest(
            repository_root=tmp_path,
            decision_boundary="one fictional boundary",
            decision_owner="decision-owner",
            requested_artifact="AssuranceProfile",
        )
    )
    assert result.status == "no-applicable-work"
    assert not list(tmp_path.rglob("*.md"))


def test_terminal_workflow_requires_a_named_human_owner(tmp_path: Path) -> None:
    """Trace: StR-001-VC-3, TC-003."""
    binding = WorkflowBinding(
        run_id="unnamed-owner-run",
        repository_id="fictional-repository@revision-1",
        workflow="architecture-evaluation",
        workflow_version="0.1.0",
        decision_boundary="one fictional boundary",
        decision_owner="",
    )
    try:
        start_or_resume(binding, tmp_path)
    except WorkflowError as error:
        assert "decision_owner" in str(error)
    else:
        raise AssertionError("an unnamed decision owner was accepted")
