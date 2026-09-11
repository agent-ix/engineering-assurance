from __future__ import annotations

from pathlib import Path

from engineering_assurance.workflow import WorkflowBinding, WorkflowError, start_or_resume


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
