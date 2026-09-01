"""Filesystem entry point for the configuration-only assurance module."""

from pathlib import Path

PACKAGE_ROOT = Path(__file__).parent
MANIFEST_PATH = PACKAGE_ROOT / "manifest.yaml"
CONTRACT_ROOT = PACKAGE_ROOT / "contracts"
FIXTURE_ROOT = PACKAGE_ROOT / "fixtures"

__all__ = ["CONTRACT_ROOT", "FIXTURE_ROOT", "MANIFEST_PATH", "PACKAGE_ROOT"]
