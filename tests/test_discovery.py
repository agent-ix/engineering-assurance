from __future__ import annotations

import json
import hashlib
from dataclasses import replace
from pathlib import Path

import pytest

from engineering_assurance.discovery import (
    CANONICAL_SKILL,
    EXPECTED_HOSTS,
    EXPECTED_WORKFLOWS,
    HOST_SURFACES,
    DiscoveryError,
    validate_discovery,
    validate_manifest_document,
)

ROOT = Path(__file__).parents[1]


def test_one_canonical_onboarding_skill_exists() -> None:
    """Trace: FR-002-AC-1, TC-009."""
    skills = list((ROOT / "engineering_assurance" / "skills").glob("*/SKILL.md"))
    assert skills == [ROOT / CANONICAL_SKILL / "SKILL.md"]


def test_four_host_surfaces_resolve_one_canonical_skill() -> None:
    """Trace: FR-002-AC-2, US-002-EX-1, TC-010."""
    resolved = validate_discovery(ROOT)
    assert set(resolved) == EXPECTED_HOSTS
    assert len(set(resolved.values())) == 1


def test_canonical_skill_exposes_exact_workflow_inventory() -> None:
    """Trace: FR-002-AC-3, US-002-EX-2, TC-011."""
    workflows = {
        path.parent.name
        for path in (ROOT / CANONICAL_SKILL / "workflows").glob("*/def.yaml")
    }
    assert workflows == EXPECTED_WORKFLOWS


def test_host_manifest_rejects_behavioral_content() -> None:
    """Trace: FR-002-AC-4, TC-012; FR-002-CON-2, TC-042."""
    surface = HOST_SURFACES[0]
    document = json.loads((ROOT / surface.manifest).read_text())
    document["instructions"] = "copied behavioral instructions"
    with pytest.raises(DiscoveryError, match="unsupported content"):
        validate_manifest_document(ROOT, surface, document)


@pytest.mark.parametrize("target", ["./missing-skills", "../../outside"])
def test_missing_or_escaping_canonical_target_is_rejected(target: str) -> None:
    """Trace: FR-002-AC-5, TC-013."""
    surface = HOST_SURFACES[0]
    document = json.loads((ROOT / surface.manifest).read_text())
    document["skills"] = target
    with pytest.raises(DiscoveryError):
        validate_manifest_document(ROOT, surface, document)


@pytest.mark.parametrize(
    "surfaces",
    [
        HOST_SURFACES[:-1],
        HOST_SURFACES + (HOST_SURFACES[0],),
        HOST_SURFACES + (replace(HOST_SURFACES[0], name="other"),),
    ],
)
def test_supported_host_set_is_exact(surfaces: tuple) -> None:
    """Trace: FR-002-CON-1, TC-041."""
    with pytest.raises(DiscoveryError, match="host set is not exact"):
        validate_discovery(ROOT, surfaces)


def test_host_manifests_are_metadata_and_one_target_only() -> None:
    """Trace: FR-002-CON-2, TC-042."""
    for surface in HOST_SURFACES:
        document = json.loads((ROOT / surface.manifest).read_text())
        assert not (set(document) - surface.allowed_keys)
        target = document["skills"]
        if isinstance(target, list):
            assert len(target) == 1
        else:
            assert isinstance(target, str)


def test_cross_agent_canonical_parity_meets_all_thresholds() -> None:
    """Trace: NFR-001, NFR-001-AC-1, NFR-001-AC-2, NFR-001-AC-3, TC-038."""
    resolved = validate_discovery(ROOT)
    assert len(resolved) == 4
    skill_digests = {
        hashlib.sha256(path.read_bytes()).hexdigest() for path in resolved.values()
    }
    assert len(skill_digests) == 1
    assert all(path == next(iter(resolved.values())) for path in resolved.values())
