"""Bounded, inventory-first onboarding for existing repositories."""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

import yaml

from engineering_assurance import PACKAGE_ROOT

ARTIFACT_TYPES = frozenset(
    {
        "AssuranceProfile",
        "MeasurementPlan",
        "ArchitectureDescription",
        "ComponentAssuranceContract",
        "AssuranceArgument",
    }
)
FRONTMATTER = re.compile(r"\A---\n(.*?)\n---(?:\n|\Z)", re.DOTALL)


class OnboardingError(ValueError):
    """Raised when onboarding cannot safely operate within its boundary."""


@dataclass(frozen=True)
class Validation:
    path: str
    artifact_type: str
    valid: bool
    diagnostics: tuple[str, ...] = ()


@dataclass
class Inventory:
    decisions: list[str] = field(default_factory=list)
    measurements: list[Validation] = field(default_factory=list)
    assurance_artifacts: list[Validation] = field(default_factory=list)
    evidence_references: list[str] = field(default_factory=list)
    producer_configurations: list[str] = field(default_factory=list)
    unresolved_inputs: list[str] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class OnboardingRequest:
    repository_root: Path
    decision_boundary: str | None
    decision_owner: str | None
    requested_artifact: str | None = None
    justification: str | None = None
    target: Path | None = None
    frontmatter: dict[str, Any] | None = None


@dataclass(frozen=True)
class OnboardingResult:
    status: str
    inventory: Inventory
    recommendation: str
    artifact_path: str | None = None


def _selected_root(root: Path) -> Path:
    if not root.is_absolute():
        raise OnboardingError("repository root must be selected as an absolute path")
    resolved = root.resolve(strict=True)
    if not resolved.is_dir():
        raise OnboardingError("repository root is not a directory")
    return resolved


def _confined(root: Path, target: Path, *, must_exist: bool = True) -> Path:
    if target.is_absolute() or ".." in target.parts:
        raise OnboardingError(f"target is not a confined relative path: {target}")
    candidate = root / target
    resolved = candidate.resolve(strict=must_exist)
    if not resolved.is_relative_to(root):
        raise OnboardingError(f"target escapes repository root: {target}")
    return resolved


def _frontmatter(path: Path) -> tuple[dict[str, Any] | None, str | None]:
    try:
        text = path.read_text()
    except (OSError, UnicodeDecodeError) as error:
        return None, f"unreadable:{error.__class__.__name__}"
    match = FRONTMATTER.match(text)
    if match is None:
        return None, None
    try:
        data = yaml.safe_load(match.group(1))
    except yaml.YAMLError:
        return None, "malformed-frontmatter"
    if not isinstance(data, dict):
        return None, "malformed-frontmatter"
    return data, None


def _quire_binary(explicit: str | None) -> str | None:
    return explicit or os.environ.get("QUIRE_BIN") or shutil.which("quire")


def _validate_artifact(
    root: Path,
    path: Path,
    artifact_type: str,
    quire_bin: str | None,
) -> Validation:
    executable = _quire_binary(quire_bin)
    if executable is None:
        return Validation(
            path.relative_to(root).as_posix(),
            artifact_type,
            False,
            ("quire-unavailable",),
        )
    completed = subprocess.run(
        [
            executable,
            "validate",
            "--module",
            str(PACKAGE_ROOT),
            "--strict",
            str(path),
        ],
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
    )
    diagnostics = tuple(
        line for line in completed.stderr.splitlines() if line.strip()
    )
    return Validation(
        path.relative_to(root).as_posix(),
        artifact_type,
        completed.returncode == 0,
        diagnostics,
    )


def inventory_repository(root: Path, *, quire_bin: str | None = None) -> Inventory:
    """Inspect existing material without writing to the selected repository."""
    selected = _selected_root(root)
    inventory = Inventory()
    for directory, names, filenames in os.walk(selected, followlinks=False):
        current = Path(directory)
        names[:] = sorted(
            name
            for name in names
            if name not in {".git", "node_modules", "__pycache__"}
            and not (current / name).is_symlink()
        )
        for filename in sorted(filenames):
            path = current / filename
            relative = path.relative_to(selected)
            if path.is_symlink():
                inventory.unresolved_inputs.append(
                    f"symlink-not-inspected:{relative.as_posix()}"
                )
                continue
            suffix = path.suffix.casefold()
            if suffix not in {".md", ".json", ".yaml", ".yml"}:
                continue
            lowered_parts = {part.casefold() for part in relative.parts}
            producer = (
                relative.parts[:2] == (".github", "workflows")
                or "producer" in path.stem.casefold()
                or "producers" in lowered_parts
            )
            if producer:
                inventory.producer_configurations.append(relative.as_posix())
            elif "evidence" in lowered_parts:
                inventory.evidence_references.append(relative.as_posix())

            if suffix != ".md":
                continue
            data, error = _frontmatter(path)
            if error:
                inventory.unresolved_inputs.append(
                    f"{error}:{relative.as_posix()}"
                )
                continue
            if data is None:
                continue
            artifact_type = data.get("type")
            if artifact_type == "MeasurementPlan":
                inventory.measurements.append(
                    _validate_artifact(
                        selected, path, artifact_type, quire_bin
                    )
                )
            elif isinstance(artifact_type, str) and artifact_type in ARTIFACT_TYPES:
                inventory.assurance_artifacts.append(
                    _validate_artifact(
                        selected, path, artifact_type, quire_bin
                    )
                )
            elif (
                isinstance(artifact_type, str)
                and "decision" in artifact_type.casefold()
            ) or "decisions" in lowered_parts:
                inventory.decisions.append(relative.as_posix())

    inventory.decisions.sort()
    inventory.measurements.sort(key=lambda item: item.path)
    inventory.assurance_artifacts.sort(key=lambda item: item.path)
    inventory.evidence_references.sort()
    inventory.producer_configurations.sort()
    inventory.unresolved_inputs.sort()
    return inventory


def render_from_skeleton(
    artifact_type: str,
    replacements: dict[str, Any],
) -> str:
    if artifact_type not in ARTIFACT_TYPES:
        raise OnboardingError(f"unsupported artifact type: {artifact_type}")
    skeleton = PACKAGE_ROOT / "skeletons" / f"{artifact_type}.md"
    text = skeleton.read_text()
    match = FRONTMATTER.match(text)
    if match is None:
        raise OnboardingError(f"installed skeleton is malformed: {skeleton.name}")
    data = yaml.safe_load(match.group(1))
    if not isinstance(data, dict):
        raise OnboardingError(f"installed skeleton is malformed: {skeleton.name}")
    data.update(replacements)
    data["type"] = artifact_type
    body = text[match.end() :]
    title = data.get("title")
    if isinstance(title, str) and title.strip():
        body = re.sub(r"\A(\n?)# .*", rf"\1# {title}", body, count=1)
    frontmatter = yaml.safe_dump(data, sort_keys=False).rstrip()
    return f"---\n{frontmatter}\n---\n{body}"


def publish_validated_artifact(
    root: Path,
    target: Path,
    artifact_type: str,
    replacements: dict[str, Any],
    *,
    quire_bin: str | None = None,
) -> Path:
    """Validate a same-directory staging file before atomic publication."""
    selected = _selected_root(root)
    destination = _confined(selected, target, must_exist=False)
    if destination.exists():
        raise OnboardingError(f"artifact already exists: {target}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination = _confined(selected, target, must_exist=False)
    content = render_from_skeleton(artifact_type, replacements)
    staged: Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=destination.parent,
            prefix=f".{destination.name}.",
            suffix=".staged",
            delete=False,
        ) as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
            staged = Path(handle.name)
        validation = _validate_artifact(
            selected, staged, artifact_type, quire_bin
        )
        if not validation.valid:
            raise OnboardingError(
                "Quire rejected staged artifact: "
                + "; ".join(validation.diagnostics)
            )
        os.replace(staged, destination)
        staged = None
        return destination
    finally:
        if staged is not None and staged.exists():
            staged.unlink()


def run_onboarding(
    request: OnboardingRequest,
    *,
    quire_bin: str | None = None,
) -> OnboardingResult:
    """Inventory first, then recommend reuse, no work, or one justified artifact."""
    inventory = inventory_repository(
        request.repository_root,
        quire_bin=quire_bin,
    )
    if not request.decision_boundary or not request.decision_owner:
        return OnboardingResult(
            "needs-input",
            inventory,
            "Provide the exact decision boundary and human decision owner.",
        )
    if request.requested_artifact is None:
        return OnboardingResult(
            "no-applicable-work",
            inventory,
            "No assurance artifact or governed workflow is justified by the request.",
        )
    if request.requested_artifact not in ARTIFACT_TYPES:
        raise OnboardingError(
            f"unsupported artifact type: {request.requested_artifact}"
        )

    candidates = [
        item
        for item in (*inventory.measurements, *inventory.assurance_artifacts)
        if item.artifact_type == request.requested_artifact
    ]
    if any(not item.valid for item in candidates) or len(candidates) > 1:
        return OnboardingResult(
            "needs-human-selection",
            inventory,
            "Applicable artifacts are malformed or conflicting; preserve them and select or correct one.",
        )
    if len(candidates) == 1:
        return OnboardingResult(
            "reuse",
            inventory,
            f"Reuse the applicable validated {request.requested_artifact}.",
            candidates[0].path,
        )
    if not request.justification:
        return OnboardingResult(
            "no-applicable-work",
            inventory,
            f"No justification was supplied for a new {request.requested_artifact}.",
        )
    if request.target is None or request.frontmatter is None:
        return OnboardingResult(
            "needs-input",
            inventory,
            "Provide a confined target and artifact frontmatter before authoring.",
        )
    published = publish_validated_artifact(
        request.repository_root,
        request.target,
        request.requested_artifact,
        request.frontmatter,
        quire_bin=quire_bin,
    )
    return OnboardingResult(
        "authored",
        inventory,
        f"Authored and validated one justified {request.requested_artifact}.",
        published.relative_to(request.repository_root.resolve()).as_posix(),
    )
