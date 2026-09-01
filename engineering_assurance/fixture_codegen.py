"""Deterministic generated-language projections for semantic fixtures."""

from __future__ import annotations

import json

from engineering_assurance import FIXTURE_ROOT

SEMANTIC_FIXTURE_ROOT = FIXTURE_ROOT / "verification-semantics"


def load_non_success_states() -> list[str]:
    return json.loads(
        (SEMANTIC_FIXTURE_ROOT / "non-success-states.json").read_text(encoding="utf-8")
    )


def load_canonical_fixture() -> dict[str, object]:
    return json.loads(
        (SEMANTIC_FIXTURE_ROOT / "canonical-references.json").read_text(
            encoding="utf-8"
        )
    )


def render_generated_state_fixtures() -> dict[str, str]:
    states = load_non_success_states()
    python_lines = [
        '"""Generated fixture; semantic source is non-success-states.json."""',
        "",
        "NON_SUCCESS_STATES = (",
        *(f'    "{state}",' for state in states),
        ")",
        "",
    ]
    typescript_lines = [
        "// Generated fixture; semantic source is non-success-states.json.",
        "export const NON_SUCCESS_STATES = [",
        *(f'  "{state}",' for state in states),
        "] as const;",
        "",
    ]
    rust_lines = [
        "// Generated fixture; semantic source is non-success-states.json.",
        "pub const NON_SUCCESS_STATES: &[&str] = &[",
        *(f'    "{state}",' for state in states),
        "];",
        "",
    ]
    return {
        "non_success_states.py": "\n".join(python_lines),
        "non_success_states.ts": "\n".join(typescript_lines),
        "non_success_states.rs": "\n".join(rust_lines),
    }


def render_generated_canonical_fixtures() -> dict[str, str]:
    canonical_json = json.dumps(
        load_canonical_fixture(), sort_keys=True, separators=(",", ":")
    )
    quoted_json = json.dumps(canonical_json)
    return {
        "canonical_references.py": "\n".join(
            [
                '"""Generated fixture; semantic source is canonical-references.json."""',
                f"CANONICAL_FIXTURE_JSON = {quoted_json}",
                "",
            ]
        ),
        "canonical_references.ts": "\n".join(
            [
                "// Generated fixture; semantic source is canonical-references.json.",
                f"export const CANONICAL_FIXTURE_JSON = {quoted_json};",
                "",
            ]
        ),
        "canonical_references.rs": "\n".join(
            [
                "// Generated fixture; semantic source is canonical-references.json.",
                f'pub const CANONICAL_FIXTURE_JSON: &str = r#"{canonical_json}"#;',
                "",
            ]
        ),
    }


def render_generated_fixtures() -> dict[str, str]:
    return {
        **render_generated_state_fixtures(),
        **render_generated_canonical_fixtures(),
    }


def committed_generated_fixtures() -> dict[str, str]:
    generated = SEMANTIC_FIXTURE_ROOT / "generated"
    return {
        name: (generated / name).read_text(encoding="utf-8")
        for name in render_generated_fixtures()
    }


__all__ = [
    "SEMANTIC_FIXTURE_ROOT",
    "committed_generated_fixtures",
    "load_canonical_fixture",
    "load_non_success_states",
    "render_generated_canonical_fixtures",
    "render_generated_fixtures",
    "render_generated_state_fixtures",
]
