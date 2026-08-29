from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).parents[1]
CHECKER_PATH = ROOT / "scripts" / "check_content_rights.py"
SPEC = importlib.util.spec_from_file_location("content_rights", CHECKER_PATH)
assert SPEC is not None and SPEC.loader is not None
CHECKER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = CHECKER
SPEC.loader.exec_module(CHECKER)


def categories(text: str, path: str = "candidate.md") -> set[str]:
    return {item.category for item in CHECKER.text_findings(path, text)}


def test_current_tree_passes_rights_check() -> None:
    result = subprocess.run(
        [sys.executable, str(CHECKER_PATH), "--tree"],
        cwd=ROOT,
        capture_output=True,
        check=False,
        text=True,
    )
    assert result.returncode == 0, result.stderr


def test_workstation_locations_are_detected_without_storing_one() -> None:
    unix = "/" + "home" + "/person/private/file.txt"
    mac = "/" + "Users" + "/person/private/file.txt"
    root = "/" + "root" + "/private/file.txt"
    windows = "C:" + "\\" + "Users\\person\\private\\file.txt"
    tilde = "~" + "/private/file.txt"
    assert "unix workstation location" in categories(unix)
    assert "unix workstation location" in categories(mac)
    assert "root workstation location" in categories(root)
    assert "windows workstation location" in categories(windows)
    assert "tilde workstation location" in categories(tilde)


def test_external_identifiers_are_detected_from_fragments() -> None:
    identifier = "IS" + "O 1234"
    assert "external publication identifier" in categories(identifier)


def test_semantic_external_content_shapes_are_detected() -> None:
    inventory = "clause " + "inventory"
    matrix = "applicability " + "matrix"
    mapping = "standard " + "crosswalk"
    legal = "legal " + "advice"
    assert "external rule inventory" in categories(inventory)
    assert "applicability matrix" in categories(matrix)
    assert "external crosswalk" in categories(mapping)
    assert "legal review material" in categories(legal)


def test_unapproved_urls_and_long_encoded_payloads_are_detected() -> None:
    url = "https:" + "//example.invalid/source"
    encoded = "A" * 260
    assert "unapproved external URL" in categories(url)
    assert "encoded payload" in categories(encoded)


def test_failure_rendering_never_needs_matched_text() -> None:
    secret = "/" + "home" + "/person/private-value"
    findings = CHECKER.text_findings("candidate.md", secret)
    rendered = "\n".join(
        f"{item.path}:{item.line}: {item.category}" for item in findings
    )
    assert secret not in rendered


def test_file_type_policy_rejects_research_containers() -> None:
    assert {".pdf", ".docx", ".xlsx"} <= CHECKER.FORBIDDEN_SUFFIXES
    assert ".py" in CHECKER.TEXT_SUFFIXES


def test_binary_control_payload_is_rejected() -> None:
    candidate = ROOT / ".pytest_cache" / "temporary-binary-check"
    candidate.write_bytes(b"text\x00payload")
    try:
        assert {item.category for item in CHECKER.inspect(candidate)} == {
            "binary control payload"
        }
    finally:
        candidate.unlink()


def test_protected_token_environment_fails_closed(monkeypatch) -> None:
    token = "private" + "-marker"
    monkeypatch.setenv("ASSURANCE_PROTECTED_TOKENS", token)
    assert "protected local token" in categories(token)
