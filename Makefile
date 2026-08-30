.PHONY: lint test package-audit eval-readiness agent-evals

EVAL_AGENT ?= codex
EVAL_RUN ?= canary
EVAL_MODEL ?=
EVAL_KEEP ?= 1
EVAL_REPORT ?= evals/reports/$(EVAL_AGENT)-$(EVAL_RUN).json
PYTHON ?= python

lint:
	$(PYTHON) -m ruff check .

test:
	$(PYTHON) scripts/check_content_rights.py --tree
	$(PYTHON) -m pytest
	$(PYTHON) scripts/validate_manifest.py

package-audit:
	$(PYTHON) scripts/audit_packages.py

eval-readiness:
	PATH="$(CURDIR)/.agent-evals/bin:$(PATH)" $(PYTHON) scripts/check_eval_readiness.py

agent-evals:
	PATH="$(CURDIR)/.agent-evals/bin:$(PATH)" $(PYTHON) scripts/run_agent_evals.py \
		--agent "$(EVAL_AGENT)" \
		--run "$(EVAL_RUN)" \
		$(if $(strip $(EVAL_MODEL)),--model "$(EVAL_MODEL)") \
		$(if $(filter 1 true yes,$(EVAL_KEEP)),--keep) \
		--report "$(EVAL_REPORT)"
