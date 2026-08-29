#!/usr/bin/env python3
"""Fail closed on files outside the repository's publishable boundary."""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).parents[1]

TEXT_SUFFIXES = {
    "",
    ".cfg",
    ".css",
    ".html",
    ".js",
    ".json",
    ".md",
    ".mjs",
    ".py",
    ".sh",
    ".toml",
    ".txt",
    ".yaml",
    ".yml",
}
FORBIDDEN_SUFFIXES = {
    ".doc",
    ".docx",
    ".epub",
    ".gif",
    ".jpg",
    ".jpeg",
    ".ods",
    ".odt",
    ".pdf",
    ".png",
    ".ppt",
    ".pptx",
    ".xls",
    ".xlsx",
    ".zip",
}
SEMANTIC_POLICY_FILES = {
    "AGENTS.md",
    "CONTENT_RIGHTS.md",
    "content-rights.yaml",
    "scripts/check_content_rights.py",
    "tests/test_content_rights.py",
}
ALLOWED_URL_PREFIXES = {
    "http://json-schema.org/draft-07/schema#",
}
URL_POLICY_FILES = {"LICENSE"}

PATH_PATTERNS = (
    ("unix workstation location", re.compile(r"/(?:home|Users)/[^/\s\"'`]+")),
    ("root workstation location", re.compile("/" + r"root(?:/|\b)")),
    (
        "windows workstation location",
        re.compile(r"(?i)\b[A-Z]:[\\/](?:Users|Documents and Settings)[\\/]"),
    ),
    ("tilde workstation location", re.compile(r"(?<!\S)~/(?=\S)")),
)
IDENTIFIER_PATTERN = re.compile(
    r"\b(?:ISO(?:[ /_-]*IEC)?|IEC|IEEE|NIST|NPR|ECSS|DO)"
    r"[ /_:-]*\d+[A-Za-z]?(?:[-.:/]\d+)*\b",
    re.IGNORECASE,
)
SEMANTIC_PATTERNS = (
    ("external rule inventory", re.compile(r"\b(?:rule|clause) inventory\b", re.I)),
    ("applicability matrix", re.compile(r"\bapplicability (?:matrix|table)\b", re.I)),
    ("external crosswalk", re.compile(r"\b(?:standard|clause) crosswalk\b", re.I)),
    ("legal review material", re.compile(r"\b(?:counsel package|legal advice)\b", re.I)),
)
URL_PATTERN = re.compile(r"""https?://[^\s)\]>"']+""")
ENCODED_PATTERN = re.compile(r"(?<![A-Za-z0-9+/])[A-Za-z0-9+/]{240,}={0,2}(?![A-Za-z0-9+/])")


@dataclass(frozen=True)
class Finding:
    path: str
    line: int
    category: str


def tracked_and_untracked_files() -> list[Path]:
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
        stdout=subprocess.PIPE,
    )
    return [
        ROOT / raw.decode()
        for raw in completed.stdout.split(b"\0")
        if raw and not raw.decode().startswith(".git/")
    ]


def protected_tokens() -> tuple[str, ...]:
    raw = os.environ.get("ASSURANCE_PROTECTED_TOKENS", "")
    return tuple(token.casefold() for token in raw.splitlines() if token.strip())


def text_findings(relative: str, text: str) -> list[Finding]:
    findings: list[Finding] = []
    tokens = protected_tokens()
    semantic_checks = relative not in SEMANTIC_POLICY_FILES
    for number, line in enumerate(text.splitlines(), 1):
        for category, pattern in PATH_PATTERNS:
            if pattern.search(line):
                findings.append(Finding(relative, number, category))
        if IDENTIFIER_PATTERN.search(line):
            findings.append(Finding(relative, number, "external publication identifier"))
        if semantic_checks:
            for category, pattern in SEMANTIC_PATTERNS:
                if pattern.search(line):
                    findings.append(Finding(relative, number, category))
        if relative not in URL_POLICY_FILES:
            for url in URL_PATTERN.findall(line):
                if not any(url.startswith(prefix) for prefix in ALLOWED_URL_PREFIXES):
                    findings.append(
                        Finding(relative, number, "unapproved external URL")
                    )
        if ENCODED_PATTERN.search(line):
            findings.append(Finding(relative, number, "encoded payload"))
        folded = line.casefold()
        if any(token in folded for token in tokens):
            findings.append(Finding(relative, number, "protected local token"))
    return findings


def inspect(path: Path, *, root: Path = ROOT) -> list[Finding]:
    relative = path.relative_to(root).as_posix()
    if path.is_symlink():
        return [Finding(relative, 0, "symbolic link")]
    suffix = path.suffix.casefold()
    if suffix in FORBIDDEN_SUFFIXES:
        return [Finding(relative, 0, "forbidden file type")]
    if suffix not in TEXT_SUFFIXES:
        return [Finding(relative, 0, "unreviewed file type")]
    if path.stat().st_size > 512_000 and relative != "LICENSE":
        return [Finding(relative, 0, "oversized text file")]
    try:
        data = path.read_bytes()
        if b"\x00" in data:
            return [Finding(relative, 0, "binary control payload")]
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        return [Finding(relative, 0, "non-UTF-8 content")]
    lfs_pointer = "version " + "https:" + "//git-lfs.github.com/spec/v1"
    if text.startswith(lfs_pointer):
        return [Finding(relative, 1, "Git LFS pointer")]
    return text_findings(relative, text)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tree", action="store_true", required=True)
    parser.parse_args()

    findings = [
        finding
        for path in tracked_and_untracked_files()
        if path.is_file() or path.is_symlink()
        for finding in inspect(path)
    ]
    if findings:
        print("content-rights check failed", file=sys.stderr)
        for finding in sorted(set(findings), key=lambda item: (item.path, item.line, item.category)):
            print(
                f"{finding.path}:{finding.line}: {finding.category}",
                file=sys.stderr,
            )
        return 1
    print("content-rights check passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
