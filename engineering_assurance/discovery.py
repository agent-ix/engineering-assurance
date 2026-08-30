"""Validate the repository-owned assurance onboarding discovery bundle."""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable


class DiscoveryError(ValueError):
    """Raised when a host surface does not resolve the canonical bundle."""


@dataclass(frozen=True)
class HostSurface:
    """A supported host and the manifest that exposes its skill source."""

    name: str
    manifest: str
    allowed_keys: frozenset[str]


PLUGIN_KEYS = frozenset({"name", "version", "description", "skills"})
HOST_SURFACES = (
    HostSurface("claude-code", ".claude-plugin/plugin.json", PLUGIN_KEYS),
    HostSurface("codex", ".codex-plugin/plugin.json", PLUGIN_KEYS),
    HostSurface(
        "opencode",
        "opencode.json",
        frozenset({"$schema", "skills"}),
    ),
    HostSurface(
        "github-copilot",
        ".github/plugin/plugin.json",
        PLUGIN_KEYS,
    ),
)
EXPECTED_HOSTS = frozenset(surface.name for surface in HOST_SURFACES)
EXPECTED_WORKFLOWS = frozenset(
    {
        "assurance-intake",
        "architecture-evaluation",
        "measurement-promotion",
        "change-assurance",
    }
)
CANONICAL_SKILLS = Path("engineering_assurance/skills")
CANONICAL_SKILL = CANONICAL_SKILLS / "assurance-onboarding"


def _within(root: Path, candidate: Path) -> Path:
    root = root.resolve()
    resolved = candidate.resolve()
    if not resolved.is_relative_to(root):
        raise DiscoveryError(f"discovery target escapes bundle: {candidate}")
    return resolved


def _skill_source(document: dict[str, Any], host: str) -> str:
    value = document.get("skills")
    if host == "opencode":
        if not isinstance(value, list) or len(value) != 1:
            raise DiscoveryError("opencode must declare exactly one skill source")
        value = value[0]
    if not isinstance(value, str) or not value.strip():
        raise DiscoveryError(f"{host} has no canonical skill source")
    return value


def validate_manifest_document(
    root: Path,
    surface: HostSurface,
    document: dict[str, Any],
) -> Path:
    """Validate one thin manifest and return its canonical skill file."""
    unknown = set(document) - surface.allowed_keys
    if unknown:
        raise DiscoveryError(
            f"{surface.name} manifest embeds unsupported content: {sorted(unknown)}"
        )
    source = _skill_source(document, surface.name)
    source_root = _within(root, root / source)
    expected_root = (root / CANONICAL_SKILLS).resolve()
    if source_root != expected_root:
        raise DiscoveryError(
            f"{surface.name} does not resolve the canonical skill source"
        )
    skill = _within(root, source_root / "assurance-onboarding" / "SKILL.md")
    if not skill.is_file():
        raise DiscoveryError(f"canonical skill is missing: {skill}")
    return skill


def validate_discovery(
    root: Path,
    surfaces: Iterable[HostSurface] = HOST_SURFACES,
) -> dict[str, Path]:
    """Resolve every supported host to the one canonical skill."""
    selected = tuple(surfaces)
    names = [surface.name for surface in selected]
    if len(names) != len(set(names)) or frozenset(names) != EXPECTED_HOSTS:
        raise DiscoveryError(f"supported host set is not exact: {names}")

    skills = sorted((root / CANONICAL_SKILLS).glob("*/SKILL.md"))
    expected_skill = root / CANONICAL_SKILL / "SKILL.md"
    if skills != [expected_skill]:
        raise DiscoveryError(
            "bundle must contain exactly one canonical assurance-onboarding skill"
        )

    workflows = {
        path.parent.name
        for path in (root / CANONICAL_SKILL / "workflows").glob("*/def.yaml")
    }
    if workflows != EXPECTED_WORKFLOWS:
        raise DiscoveryError(f"canonical workflow set is not exact: {workflows}")

    resolved: dict[str, Path] = {}
    for surface in selected:
        manifest = _within(root, root / surface.manifest)
        if not manifest.is_file():
            raise DiscoveryError(f"host manifest is missing: {surface.manifest}")
        document = json.loads(manifest.read_text())
        if not isinstance(document, dict):
            raise DiscoveryError(f"host manifest is not an object: {surface.manifest}")
        resolved[surface.name] = validate_manifest_document(root, surface, document)
    if len(set(resolved.values())) != 1:
        raise DiscoveryError("supported hosts resolve different skill files")
    return resolved
