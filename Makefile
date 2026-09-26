.PHONY: rust-features rust-feature-list lint test manifest-validate compatibility-observe package-audit validate-docs rust-format rust-clippy rust-toolchain rust-tests rust-docs rust-deps rust-audit rust-foundation-gate agent-evals agent-evals-aggregate integration-traceability integration-evidence integration-gate release-gate

EVAL_AGENT ?= codex
EVAL_RUN ?= canary
EVAL_MODEL ?=
EVAL_FILTER ?=
EVAL_KEEP ?= 1
EVAL_REPORT ?= evals/reports/$(EVAL_AGENT)-$(EVAL_RUN).json
EVAL_REPORTS ?=
EVAL_AGGREGATE_REPORT ?=
EVAL_WORKSPACE_ROOT ?=
EVAL_SOURCE_REVISION ?= $(shell git rev-parse HEAD)
MANIFEST_SCHEMA_ROOT ?=
MANIFEST_REGISTRY_ROOT ?=
PYTHON ?= python3
QUIRE ?= quire
CARGO ?= cargo

lint:
	$(PYTHON) -m ruff check .

test:
	CARGO_BUILD_JOBS=2 cargo +1.98.1 run --locked --quiet --features full -- content-rights-tree --root .
	CARGO_BUILD_JOBS=2 cargo +1.98.1 test --locked --features full --test python_port

manifest-validate:
	@test -n "$(strip $(MANIFEST_SCHEMA_ROOT))" || { \
		echo "MANIFEST_SCHEMA_ROOT is required for explicit manifest validation" >&2; \
		exit 2; \
	}
	@test -n "$(strip $(MANIFEST_REGISTRY_ROOT))" || { \
		echo "MANIFEST_REGISTRY_ROOT is required for explicit manifest validation" >&2; \
		exit 2; \
	}
	CARGO_BUILD_JOBS=2 $(CARGO) +1.98.1 run --locked --quiet --features full -- manifest-validate \
		--root . \
		--schema-root "$(MANIFEST_SCHEMA_ROOT)" \
		--registry-root "$(MANIFEST_REGISTRY_ROOT)"

# Classify the toolchain on this machine against the reviewed compatibility
# matrix. This is a manual command, deliberately not part of `integration-gate`
# or hosted CI: it answers a question about the operator's installed toolchain,
# and both of those run where `quire`, `quoin` and `ix-flow` are not all on PATH
# at their pinned versions, so wiring it in would report unknown for a component
# that is present but invoked another way. The matrix names it as the upgrade
# verification step and this target is where that name resolves. The one fact it
# used to be the only detector of — hosted CI and the matrix pinning different
# Quire CLI versions — is now asserted by TC-130 in `tests/compatibility_cli.rs`,
# which needs nothing installed and so runs under `make rust-tests`.
compatibility-observe:
	CARGO_BUILD_JOBS=2 $(CARGO) +1.98.1 run --locked --quiet --features full -- compatibility-observe --root .

package-audit:
	CARGO_BUILD_JOBS=2 cargo +1.98.1 run --locked --quiet --features full -- package-audit --root .

validate-docs:
	$(QUIRE) validate --scope "$(CURDIR)" "spec/**/*.md" "plan/**/*.md" "reviews/**/*.md"

rust-format:
	$(CARGO) fmt -- --check

rust-clippy:
	$(CARGO) clippy --workspace --all-targets --all-features --locked -- -D warnings

rust-toolchain:
	$(CARGO) check --workspace --all-targets --all-features --locked

rust-tests:
	$(CARGO) test --workspace --all-targets --all-features --locked

rust-docs:
	RUSTDOCFLAGS="-D warnings" $(CARGO) doc --workspace --all-features --no-deps --locked

rust-deps:
	$(CARGO) deny --locked check

rust-audit:
	$(CARGO) audit

# Feature matrix (EA-20): every capability feature must compile ALONE, with
# default features off, so a module gated on the wrong feature cannot hide
# behind `--all-features`. The list is read from Cargo.toml via `cargo
# metadata` (the `engineering-assurance` package's features, minus `default`
# and the `full` umbrella), so a newly declared feature is checked without
# editing this file. Scope: `--lib` only (the binary needs `full`), both
# `check`, building its unit tests (`test --no-run`) and a warning-free
# `doc`; it needs python3 for
# the derivation. No workflow file runs this target
# directly; CI enforces it through `make rust-foundation-gate` -> `rust-tests`,
# which runs tests/feature_matrix.rs (TC-190), which runs this target.
# The derivation is a `$(shell)` so `make -n` prints the commands without
# running them; an empty or failed derivation is an error, not a pass. Cargo
# parallelism is pinned by CARGO_BUILD_JOBS, so no `+` jobserver prefix (it
# would make `make -n` execute the cargo lines).
EA_FEATURES = $(shell $(CARGO) metadata --no-deps --format-version 1 --locked 2>/dev/null | python3 -c 'import json,sys; p=next(p for p in json.load(sys.stdin)["packages"] if p["name"]=="engineering-assurance"); [print(f) for f in sorted(p["features"]) if f not in ("default","full")]' 2>/dev/null)

rust-feature-list:
	@printf '%s\n' $(EA_FEATURES)

rust-features:
	@features="$(EA_FEATURES)"; test -n "$$features" || { echo "rust-features: could not derive the feature list from cargo metadata (empty or failed)" >&2; exit 1; }
	CARGO_BUILD_JOBS=2 $(CARGO) check --lib --no-default-features --locked
	@for f in $(EA_FEATURES); do \
		echo "cargo check --lib --no-default-features --features $$f"; \
		CARGO_BUILD_JOBS=2 $(CARGO) check --lib --no-default-features --features $$f --locked || exit 1; \
		echo "cargo test --lib --no-run --no-default-features --features $$f"; \
		CARGO_BUILD_JOBS=2 $(CARGO) test --lib --no-run --no-default-features --features $$f --locked || exit 1; \
		echo "cargo doc --no-deps --no-default-features --features $$f"; \
		RUSTDOCFLAGS='-D warnings' CARGO_BUILD_JOBS=2 $(CARGO) doc --no-deps --no-default-features --features $$f --locked || exit 1; \
	done

rust-foundation-gate: rust-format rust-clippy rust-toolchain rust-tests rust-docs rust-deps rust-audit

agent-evals:
	CARGO_BUILD_JOBS=2 $(CARGO) +1.98.1 run --locked --quiet --features full -- agent-evals --root . \
		--agent "$(EVAL_AGENT)" \
		--run "$(EVAL_RUN)" \
		$(if $(strip $(EVAL_FILTER)),--filter "$(EVAL_FILTER)") \
		$(if $(strip $(EVAL_MODEL)),--model "$(EVAL_MODEL)") \
		$(if $(filter 1 true yes,$(EVAL_KEEP)),--keep) \
		--report "$(EVAL_REPORT)"

agent-evals-aggregate:
	@test -n "$(strip $(EVAL_REPORTS))" || { \
		echo "EVAL_REPORTS is required for retained evaluation aggregation" >&2; \
		exit 2; \
	}
	@test -n "$(strip $(EVAL_AGGREGATE_REPORT))" || { \
		echo "EVAL_AGGREGATE_REPORT is required for retained evaluation aggregation" >&2; \
		exit 2; \
	}
	@test -n "$(strip $(EVAL_WORKSPACE_ROOT))" || { \
		echo "EVAL_WORKSPACE_ROOT is required for retained transcript confinement" >&2; \
		exit 2; \
	}
	CARGO_BUILD_JOBS=2 $(CARGO) +1.98.1 run --locked --quiet --features full -- evaluation-aggregate \
		--root . \
		--workspace-root "$(EVAL_WORKSPACE_ROOT)" \
		$(foreach report,$(EVAL_REPORTS),--report "$(report)") \
		--source-revision "$(EVAL_SOURCE_REVISION)" \
		--output "$(EVAL_AGGREGATE_REPORT)"

integration-traceability:
	CARGO_BUILD_JOBS=2 $(CARGO) +1.98.1 run --locked --quiet --features full -- integration-evidence \
		--root . \
		--quire "$(QUIRE)" \
		--traceability-only

integration-evidence:
	@test -n "$(strip $(EVAL_AGGREGATE_REPORT))" || { \
		echo "EVAL_AGGREGATE_REPORT is required for real-agent release evidence" >&2; \
		exit 2; \
	}
	@test -n "$(strip $(EVAL_WORKSPACE_ROOT))" || { \
		echo "EVAL_WORKSPACE_ROOT is required for retained transcript confinement" >&2; \
		exit 2; \
	}
	CARGO_BUILD_JOBS=2 $(CARGO) +1.98.1 run --locked --quiet --features full -- integration-evidence \
		--root . \
		--quire "$(QUIRE)" \
		--artifact "$(EVAL_AGGREGATE_REPORT)" \
		--workspace-root "$(EVAL_WORKSPACE_ROOT)"

integration-gate: lint test package-audit validate-docs rust-foundation-gate integration-traceability

release-gate: integration-gate integration-evidence
