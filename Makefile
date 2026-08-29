.PHONY: lint test package-audit

lint:
	python -m ruff check .

test:
	python scripts/check_content_rights.py --tree
	python -m pytest
	python scripts/validate_manifest.py

package-audit:
	python scripts/audit_packages.py
