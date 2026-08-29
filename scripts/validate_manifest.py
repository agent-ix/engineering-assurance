#!/usr/bin/env python3
"""Validate the module inventory and every authored fixture."""

from __future__ import annotations

import json
import os
from pathlib import Path

import yaml
from jsonschema import Draft7Validator, FormatChecker

ROOT = Path(__file__).parents[1]
PACKAGE = ROOT / "engineering_assurance"


def frontmatter(path: Path) -> dict:
    text = path.read_text()
    if not text.startswith("---\n") or "\n---\n" not in text[4:]:
        raise ValueError(f"{path.name}: missing frontmatter")
    return yaml.safe_load(text.split("\n---\n", 1)[0][4:])


def shared_manifest_schema() -> Path | None:
    configured = os.environ.get("MODULE_MANIFEST_SCHEMA")
    candidates = [
        Path(configured) if configured else None,
        ROOT.parent
        / "spec-artifacts-iso"
        / "spec_artifacts_iso"
        / "module-manifest.schema.json",
        ROOT
        / ".deps"
        / "spec-artifacts-iso"
        / "spec_artifacts_iso"
        / "module-manifest.schema.json",
    ]
    return next((path for path in candidates if path and path.is_file()), None)


def main() -> int:
    manifest = yaml.safe_load((PACKAGE / "manifest.yaml").read_text())
    assert manifest["name"] == "engineering-assurance"
    assert manifest["version"] == "0.2.0"
    shared = shared_manifest_schema()
    if shared is None:
        raise SystemExit("shared module manifest schema is unavailable")
    shared_schema = json.loads(shared.read_text())
    shared_errors = list(Draft7Validator(shared_schema).iter_errors(manifest))
    if shared_errors:
        raise SystemExit(shared_errors[0].message)
    registry_manifest = yaml.safe_load((shared.parent / "manifest.yaml").read_text())
    registered_edges = set(registry_manifest["edge_types"])
    used_edges = {
        edge
        for artifact in manifest["artifact_types"]
        for edge in artifact["allowed_links"]
    }
    missing_edges = used_edges - registered_edges
    if missing_edges:
        raise SystemExit(f"unregistered edge types: {sorted(missing_edges)}")

    names = set()
    for artifact in manifest["artifact_types"]:
        name = artifact["name"]
        if name in names:
            raise SystemExit(f"duplicate artifact type: {name}")
        names.add(name)
        schema_path = PACKAGE / artifact["frontmatter_schema_ref"]
        skeleton_path = PACKAGE / "skeletons" / f"{name}.md"
        schema = json.loads(schema_path.read_text())
        Draft7Validator.check_schema(schema)
        errors = list(
            Draft7Validator(schema, format_checker=FormatChecker()).iter_errors(
                frontmatter(skeleton_path)
            )
        )
        if errors:
            raise SystemExit(f"{name}: {errors[0].message}")
        skeleton = skeleton_path.read_text()
        sections = artifact["body_extraction"]["yield_pattern"]["match"].values()
        for locator in sections:
            heading = f"## {locator['after_heading']}"
            if locator["required"] is not True or heading not in skeleton:
                raise SystemExit(f"{name}: missing required heading {heading}")
    print("module validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
