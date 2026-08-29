"""Filesystem entry point for the configuration-only assurance module."""

from pathlib import Path

PACKAGE_ROOT = Path(__file__).parent
MANIFEST_PATH = PACKAGE_ROOT / "manifest.yaml"

__all__ = ["MANIFEST_PATH", "PACKAGE_ROOT"]
