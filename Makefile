.PHONY: lint test package-audit validate-docs eval-readiness agent-evals agent-evals-aggregate integration-evidence integration-gate

EVAL_AGENT ?= codex
EVAL_RUN ?= canary
EVAL_MODEL ?=
EVAL_FILTER ?=
EVAL_KEEP ?= 1
EVAL_REPORT ?= evals/reports/$(EVAL_AGENT)-$(EVAL_RUN).json
EVAL_REPORTS ?=
EVAL_AGGREGATE_REPORT ?= evals/reports/aggregate-ea1ed8a.json
PYTHON ?= python
QUIRE ?= quire

lint:
	$(PYTHON) -m ruff check .

test:
	$(PYTHON) scripts/check_content_rights.py --tree
	$(PYTHON) -m pytest
	$(PYTHON) scripts/validate_manifest.py

package-audit:
	$(PYTHON) scripts/audit_packages.py

validate-docs:
	$(QUIRE) validate --scope "$(CURDIR)" "spec/**/*.md" "plan/**/*.md" "reviews/**/*.md"

eval-readiness:
	PATH="$(CURDIR)/.agent-evals/bin:$(PATH)" $(PYTHON) scripts/check_eval_readiness.py

agent-evals:
	PATH="$(CURDIR)/.agent-evals/bin:$(PATH)" $(PYTHON) scripts/run_agent_evals.py \
		--agent "$(EVAL_AGENT)" \
		--run "$(EVAL_RUN)" \
		$(if $(strip $(EVAL_FILTER)),--filter "$(EVAL_FILTER)") \
		$(if $(strip $(EVAL_MODEL)),--model "$(EVAL_MODEL)") \
		$(if $(filter 1 true yes,$(EVAL_KEEP)),--keep) \
		--report "$(EVAL_REPORT)"

agent-evals-aggregate:
	$(PYTHON) scripts/aggregate_agent_eval_reports.py \
		$(foreach report,$(EVAL_REPORTS),--report "$(report)") \
		--output "$(EVAL_AGGREGATE_REPORT)"

integration-evidence:
	$(PYTHON) scripts/check_integration_evidence.py \
		--quire "$(QUIRE)" \
		--aggregate "$(EVAL_AGGREGATE_REPORT)"

integration-gate: lint test package-audit validate-docs integration-evidence
