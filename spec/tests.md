---
id: TM-001
title: engineering-assurance Onboarding Test Matrix
type: TestMatrix
---

# Engineering-Assurance Onboarding Test Matrix

## Overview

This matrix records completed verification for the canonical assurance-onboarding
and verification-semantics baseline and the verification of the Rust port.

## Test Matrix Rules

1. Every criterion maps to at least one test case.
2. Local-source and repository-source installation are exercised across every
   supported agent surface.
3. Exact supported-agent set and thin-manifest constraints include rejecting
   values outside their declared boundary.
4. Documented malformed, unavailable, missing-input, missing-executable, and
   gate-override paths are explicit test cases.
5. Workflow interruption, resume, waiting, rejection, and invalid gate override
   are distinct transitions or transition failures.
6. Existing valid artifacts, absent justification, malformed output, unavailable
   producers, missing targets, and unexpected package members are explicit edge
   cases.
7. Agent evaluations cover seven concrete scenario variants on each of four hosts:
   existing profile, no profile, malformed producer, unavailable producer,
   interruption/resume, explicit acceptance, and explicit rejection.
8. Every requirement-verifying Rust test uses the canonical ix-trace-rs import
   and bare trace attribute form that Quire reconciles.
9. Requirement Acceptance Criteria use broad verification-method categories
   such as `Test`, while this matrix's `Type` column names the concrete test
   level or evidence form (`Unit`, `Integration`, `Property`, `Static`, and so
   on). An AC's `Test` may therefore map to an `Integration` TC; the columns are
   related at different levels of specificity rather than one shared vocabulary.

## Requirements Traceability

### Stakeholder Requirement Coverage

| Stakeholder Req | Trace to US/FR | Test/Validation | Status |
|-----------------|----------------|-----------------|-----------------|
| StR-001 | FR-001, FR-004, FR-005 | StR-001-VC-1..VC-3 → TC-001..TC-003 | ✅ Passing |
| StR-003 | FR-014..FR-018, NFR-005 | StR-003-VC-1..VC-4 → TC-096, TC-100, TC-115 | ✅ Package boundary, reference-byte parity and executable-language audit passing |

### User Story Coverage

| User Story | Acceptance Criteria | Test Cases | Status |
|------------|---------------------|------------|-----------------|
| US-001 | US-001-EX-1 | TC-004 | ✅ Passing |
| US-001 | US-001-EX-2 | TC-005 | ✅ Passing |
| US-002 | US-002-EX-1 | TC-010 | ✅ Passing |
| US-002 | US-002-EX-2 | TC-011 | ✅ Passing |
| US-003 | US-003-EX-1 | TC-026 | ✅ Passing |
| US-003 | US-003-EX-2 | TC-028 | ✅ Passing |
| US-004 | US-004-EX-1 | TC-021 | ✅ Passing |
| US-004 | US-004-EX-2 | TC-020 | ✅ Passing |

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Status |
|----------------|---------------------|------------|-----------------|
| FR-001 | FR-001-AC-1 | TC-004 | ✅ Passing |
| FR-001 | FR-001-AC-2 | TC-005 | ✅ Passing |
| FR-001 | FR-001-AC-3 | TC-006 | ✅ Passing |
| FR-001 | FR-001-AC-4 | TC-007 | ✅ Passing |
| FR-001 | FR-001-AC-5 | TC-008 | ✅ Passing |
| FR-001 | FR-001-AC-6 | TC-044 | ✅ Passing |
| FR-001 | FR-001-AC-7 | TC-045 | ✅ Passing |
| FR-001 | FR-001-AC-8 | TC-174 | ✅ Passing |
| FR-001 | FR-001-AC-9 | TC-202 | ✅ Valid, invalid, capped-findings, unavailable-validation and all-valid summary cases passing |
| FR-002 | FR-002-AC-1 | TC-009 | ✅ Passing |
| FR-002 | FR-002-AC-2 | TC-010 | ✅ Passing |
| FR-002 | FR-002-AC-3 | TC-011 | ✅ Passing |
| FR-002 | FR-002-AC-4 | TC-012 | ✅ Passing |
| FR-002 | FR-002-AC-5 | TC-013 | ✅ Passing |
| FR-003 | FR-003-AC-1 | TC-014 | ✅ Passing |
| FR-003 | FR-003-AC-2 | TC-015 | ✅ Passing |
| FR-003 | FR-003-AC-3 | TC-016 | ✅ Passing |
| FR-003 | FR-003-AC-4 | TC-017 | ✅ Passing |
| FR-003 | FR-003-AC-5 | TC-018 | ✅ Passing |
| FR-003 | FR-003-AC-6 | TC-019 | ✅ Passing |
| FR-003 | FR-003-AC-7 | TC-194 | ✅ Export declaration, 2020-12 and derivation checks passing |
| FR-003 | FR-003-AC-8 | TC-195 | ✅ Skeletons validate; measured consumer verdicts and a format-asserting comparison hold at default options |
| FR-004 | FR-004-AC-1 | TC-020 | ✅ Passing |
| FR-004 | FR-004-AC-2 | TC-021 | ✅ Passing |
| FR-004 | FR-004-AC-3 | TC-022 | ✅ Passing |
| FR-004 | FR-004-AC-4 | TC-023 | ✅ Passing |
| FR-004 | FR-004-AC-5 | TC-024 | ✅ Passing |
| FR-004 | FR-004-AC-6 | TC-025 | ✅ Passing |
| FR-004 | FR-004-AC-7 | TC-046 | ✅ Passing |
| FR-005 | FR-005-AC-1 | TC-026 | ✅ Passing |
| FR-005 | FR-005-AC-2 | TC-027 | ✅ Passing |
| FR-005 | FR-005-AC-3 | TC-028 | ✅ Passing |
| FR-005 | FR-005-AC-4 | TC-029 | ✅ Passing |
| FR-005 | FR-005-AC-5 | TC-030 | ✅ Passing |
| FR-005 | FR-005-AC-6 | TC-047 | ✅ Passing |
| FR-005 | FR-005-AC-7 | TC-048 | ✅ Passing |
| FR-006 | FR-006-AC-1 | TC-031 | ✅ Passing |
| FR-006 | FR-006-AC-2 | TC-032 | ✅ Passing |
| FR-006 | FR-006-AC-3 | TC-033 | ✅ Passing |
| FR-006 | FR-006-AC-4 | TC-034 | ✅ Passing |
| FR-006 | FR-006-AC-5 | TC-049 | ✅ Passing |
| FR-007 | FR-007-AC-1 | TC-035 | ✅ Passing |
| FR-007 | FR-007-AC-2 | TC-036 | ✅ Passing |
| FR-007 | FR-007-AC-3 | TC-037 | ✅ Passing |
| FR-007 | FR-007-AC-4 | TC-043 | ✅ Passing |
| FR-014 | FR-014-AC-1 | TC-096 | ✅ Rust package foundation backed |
| FR-014 | FR-014-AC-2 | TC-098 | ✅ `tests/package_boundary.rs` runs every machine-facing command in its table through a refusal and requires exactly one versioned JSON line on stdout with diagnostics on stderr; `tests/manifest_host_cli.rs` covers the success path. `agent-evals` is outside the table because it launches real agents, and `onboard-report` is human-readable text, not a machine result |
| FR-014 | FR-014-AC-3 | TC-099 | ✅ Unknown protocol, malformed and oversized input, escaping target, unavailable host, and invalid host response each refuse with no write (onboarding, workflow-host and process-adapter tests) |
| FR-014 | FR-014-AC-4 | TC-101 | ✅ Parsed library-module audit, lexical-alias/comment adverse cases, and mutations passing |
| FR-014 | FR-014-AC-5 | TC-138 | ✅ Isolated offline downstream resolution/compile and activated direct-dependency census passing; `full`-package gate unaffected |
| FR-014 | FR-014-AC-6 | TC-189 | ✅ Grammar-selected requirement-test audit: doc-comment accept/absent/empty/malformed cases and attribute-only import findings passing |
| FR-014 | FR-014-AC-7 | TC-190 | ✅ `tests/feature_matrix.rs` metadata/list equality, empty-derivation refusal, and `make rust-features` compile of each feature alone passing; enforced in CI through the existing `rust-tests` target |
| FR-014 | FR-014-AC-8 | TC-196 | ✅ `tests/feature_matrix.rs`: no feature graph other than `full` resolves a binary-only crate; `serde_json/arbitrary_precision` is on exactly for the `exact-numbers` features |
| FR-014 | FR-014-AC-9 | TC-197 | ✅ `tests/default_features.rs`: the default graph equals the empty-feature graph, resolves none of the binary-only crates and no `serde_json`; `--features full` still resolves the binary's crates |
| FR-015 | FR-015-AC-1 | TC-100 | ✅ Fixture-generation slice is backed against reference bytes captured once from the retired implementation and committed to this repository; no retained implementation is executed as a differential oracle in this row's qualification path |
| FR-015 | FR-015-AC-2 | TC-102 | ✅ Evidence-availability and semantic-reference state vocabularies backed |
| FR-015 | FR-015-AC-3 | TC-103 | ✅ Evidence, semantic, and accepted-corpus path refusals plus bounded population/depth checks backed |
| FR-015 | FR-015-AC-4 | TC-104 | ✅ Semantic ownership, contract-family, library-capability, and inert-fixture static slices backed |
| FR-016 | FR-016-AC-1 | TC-105 | ✅ Rust onboarding core, CLI, retained-reference parity, and confined publication backed |
| FR-016 | FR-016-AC-2 | TC-106 | ✅ Rust evaluator and CLI parity backed |
| FR-016 | FR-016-AC-3 | TC-107 | ✅ Rust lifecycle host and adverse cases pass |
| FR-016 | FR-016-AC-5 | TC-201 | ✅ Single-binding refusal cases with retained-reference parity passing |
| FR-017 | FR-017-AC-1 | TC-109 | ✅ Backed by the tagged tests |
| FR-017 | FR-017-AC-2 | TC-110 | ✅ Backed by the tagged tests |
| FR-017 | FR-017-AC-3 | TC-111 | ✅ Bounded Rust package/archive/install candidate and preceding qualification slices backed |
| FR-017 | FR-017-AC-4 | TC-112 | ✅ npm lifecycle, content-rights, and package-audit gates are declarative Rust dispatch; plugin manifests, `opencode.json`, `package.json` scripts and the cli-agent-evals config are audited, GitHub workflow YAML is not |
| FR-017 | FR-017-AC-9 | TC-136 | ✅ Provider responses omit absent optional scenario metadata from the emitted bytes |
| FR-017 | FR-017-AC-10 | TC-137 | ✅ Coverage refusals carry condition-specific codes and authored locations |
| FR-017 | FR-017-AC-6 | TC-120 | ✅ Typed Rust policy, retained-Python differential, adverse boundaries, ordering, resource ceilings, and mutation probes passing |
| FR-017 | FR-017-AC-7 | TC-121 | ✅ Rust manifest classifier backed by locally authored, non-authoritative fixture schema and registry (PLAT-992); real conformance of the retained manifest to the FR-035 schema and upstream edge registry is additionally asserted by the `#[ignore]`d `tests/manifest_host_cli.rs::tc_121_real_manifest_conforms_to_the_authoritative_schema_and_edge_registry`, run with explicit operator-supplied schema/registry roots (PLAT-994) |
| FR-018 | FR-018-AC-2 | TC-113 | ✅ Content-rights, package-audit, and compatibility/fixture-generation cutovers plus their deletion candidates backed |
| FR-018 | FR-018-AC-4 | TC-115 | ✅ `tests/python_port/module.rs`: the only first-party non-Rust executable paths are the configuration-only package `__init__.py`, the two ix-flow JavaScript invariant providers, and the cli-agent-evals host configuration |
| FR-020 | FR-020-AC-1 | TC-139 | ✅ Schema accept/refuse cases and skeleton validation passing |
| FR-020 | FR-020-AC-2 | TC-140 | ✅ Rust construction/deserialization refusals and schema/enum wire-set equality passing |
| FR-020 | FR-020-AC-3 | TC-141 | ✅ Unversioned objective add/remove/change findings naming `objective`, and versioned no-finding cases passing |
| FR-020 | FR-020-AC-4 | TC-142 | ✅ Isolated offline downstream compile with only `measurement` and whole-graph `serde_json` absence passing |
| FR-021 | FR-021-AC-1 | TC-144 | ✅ Decision-rule schema accept/refuse cases and `metric` requirement passing |
| FR-021 | FR-021-AC-2 | TC-145 | ✅ Closed estimator schema cases, prose fields, and skeleton validation passing |
| FR-021 | FR-021-AC-3 | TC-146 | ✅ Rust decision-rule construction/deserialization refusals and round trip passing |
| FR-021 | FR-021-AC-4 | TC-147 | ✅ Estimator, comparator, and baseline schema/enum wire-set equality passing |
| FR-021 | FR-021-AC-5 | TC-148 | ✅ Comparator, margin, and evaluation-refusal cases passing |
| FR-021 | FR-021-AC-6 | TC-149 | ✅ Onboarding checklist decision-rule vocabulary and consistency rules passing |
| FR-021 | FR-021-AC-7 | TC-142 | ✅ Minimal `measurement` consumer reaches the decision-rule types without `serde_json` |
| FR-021 | FR-021-AC-8 | TC-141 | ✅ Unversioned estimator and decision-rule edits yield findings naming each changed member |
| FR-021 | FR-021-AC-9 | TC-144, TC-146 | ✅ Schema and Rust direction-agreement and constant-predictor/estimator cases passing |
| FR-021 | FR-021-AC-10 | TC-171 | ✅ Schema and Rust external-reference baseline estimator-agnostic, eq, refusal and evaluation cases passing |
| FR-021 | FR-021-AC-12 | TC-188 | ✅ Relative and absolute margin evaluation, mode validation, default and round trip, and schema/Rust wire parity passing |
| FR-021 | FR-021-AC-13 | TC-192 | ✅ Schema and Rust `interval_level`, `Interval` and `ConfidenceLevel` validation, canonical round trip, wire parity and definition-change cases passing |
| FR-021 | FR-021-AC-14 | TC-193 | ✅ Unfavourable-bound evaluation, level and interval refusals, and point-form refusal passing |
| FR-021 | FR-021-AC-15 | TC-239 | ✅ Reported bound kind and value for gt/ge/lt/le, threshold, baseline and relative-margin references, and shared refusals passing |
| FR-021 | FR-021-AC-16 | TC-240 | ✅ Passing |
| FR-021 | FR-021-AC-17 | TC-241 | ✅ Passing |
| FR-021 | FR-021-AC-18 | TC-242 | ✅ Passing |
| FR-021 | FR-021-AC-19 | TC-243 | ✅ Passing |
| FR-021 | FR-021-AC-20 | TC-245 | ✅ Skeleton Interpretation and onboarding skill warning passing |
| FR-022 | FR-022-AC-1 | TC-150 | ✅ Wire spelling, rendering, parsing, and serde round trip backed |
| FR-022 | FR-022-AC-2 | TC-151 | ✅ Exact-match typed refusal backed |
| FR-022 | FR-022-AC-3 | TC-152 | ✅ Exhaustive listing, compile-time trait-absence probe, and downstream ordering refusals backed |
| FR-022 | FR-022-AC-4 | TC-153 | ✅ Isolated offline claim-strength-only consumer compile and dependency census backed |
| FR-022 | FR-022-AC-5 | TC-150 | ✅ Ownership-registry entry, misdeclaration refusals, and generated listings backed |
| FR-023 | FR-023-AC-1 | TC-143 | ✅ Passing |
| FR-023 | FR-023-AC-2 | TC-143 | ✅ Passing |
| FR-023 | FR-023-AC-3 | TC-143 | ✅ Passing |
| FR-024 | FR-024-AC-1 | TC-154 | ✅ Protected-apparatus schema cases from the shared case table passing |
| FR-024 | FR-024-AC-2 | TC-155 | ✅ Negative-control schema cases and gate-stage requirement passing |
| FR-024 | FR-024-AC-3 | TC-156 | ✅ Rust apparatus-path refusals, shared-table schema agreement, and set semantics passing |
| FR-024 | FR-024-AC-4 | TC-157 | ✅ Control-kind wire parity and Rust negative-control refusals passing |
| FR-024 | FR-024-AC-5 | TC-158, TC-141 | ✅ Unversioned protected-list edits yield findings; reorder and versioned edits do not |
| FR-024 | FR-024-AC-6 | TC-159 | ✅ Skeleton, onboarding skill, entry-description checklist, and warning-free checklists for every type passing |
| FR-024 | FR-024-AC-7 | TC-142 | ✅ Minimal `measurement` consumer reaches the apparatus and control types without `serde_json` |
| FR-024 | FR-024-AC-8 | TC-155 | ✅ Procedure and apparatus-edit `protected_apparatus` requirements passing; independent gate accepted |
| FR-024 | FR-024-AC-10 | TC-185 | ✅ Candidate `execution_procedure` path and protected-list schema cases passing; Quoin checks exact membership and source bytes |
| FR-024 | FR-024-AC-11 | TC-203 | ✅ Gate-stage ground-truth requirement and closed-value cases passing |
| FR-025 | FR-025-AC-1 | TC-163 | ✅ Profile `measurement_policy` schema accept/refuse cases and stage-enum parity passing |
| FR-025 | FR-025-AC-2 | TC-160 | ✅ Recommend-mode pass for every checker status passing |
| FR-025 | FR-025-AC-3 | TC-161, TC-164 | ✅ Require-mode accept and typed refusals passing |
| FR-025 | FR-025-AC-4 | TC-162 | ✅ Exception override, expiry and ownership cases passing |
| FR-025 | FR-025-AC-5 | TC-160, TC-161, TC-162 | ✅ Rust/JavaScript outcome parity and malformed-item refusals passing |
| FR-025 | FR-025-AC-6 | TC-164 | ✅ Workflow item schemas, canonical/pilot equality and human-gated terminals passing |
| FR-025 | FR-025-AC-7 | TC-165 | ✅ Skeleton, onboarding skill and warning-free checklist passing |
| FR-026 | FR-026-AC-1 | TC-166 | ✅ Objective steering-field schema accept/refuse and range cases passing |
| FR-026 | FR-026-AC-2 | TC-167 | ✅ Rust steering-field construction, refusal and deserialization cases passing |
| FR-026 | FR-026-AC-3 | TC-168 | ✅ Steering-only edits excluded from the definition-change check; bound edits still reported |
| FR-026 | FR-026-AC-4 | TC-169 | ✅ Smallest and largest steering values produce the identical decision-rule verdict as none at all |
| FR-026 | FR-026-AC-5 | TC-170 | ✅ Skeleton carries all three steering fields with an advisory-only section, no warning |
| FR-026 | FR-026-AC-6 | TC-142 | ✅ Minimal `measurement` consumer reaches the steering-field accessors without `serde_json` |
| FR-027 | FR-027-AC-1 | TC-206 | ✅ Atomic publish creation and sidecar-cleanup cases passing |
| FR-027 | FR-027-AC-2 | TC-207, TC-213 | ✅ Identical, different, directory, symlink, pre-commit-race and concurrent-writer cases passing |
| FR-027 | FR-027-AC-4 | TC-211 | ✅ Written, already-present and collision outcome cases passing |
| FR-027 | FR-027-AC-5 | TC-212 | ✅ Durable publish on written and already-present paths passing; Windows compile-checked |
| FR-029 | FR-029-AC-1 | TC-214 | ✅ `tests/producer_execute_cli.rs` passing (completed-state assertions run on Linux only; not executed on the macOS authoring host) |
| FR-029 | FR-029-AC-2 | TC-215 | ✅ `tests/producer_execute_cli.rs` passing (host-independent) |
| FR-029 | FR-029-AC-3 | TC-216 | ✅ `tests/producer_execute_cli.rs` passing (`timed_out` assertion runs on Linux only; not executed on the macOS authoring host) |
| FR-029 | FR-029-AC-4 | TC-217 | ✅ `tests/producer_execute_cli.rs` passing (live result-line schema check sees `containment_failure` off Linux; the `completed` and `failed` shapes are checked against the real serializer) |
| FR-029 | FR-029-AC-5 | TC-218 | ✅ `tests/producer_execute_cli.rs` passing (completed-state check is Linux only; off Linux the same run reports `containment_failure`) |
| FR-030 | FR-030-AC-1 | TC-219 | ✅ Round-trip and deterministic-bytes cases passing |
| FR-030 | FR-030-AC-2 | TC-220 | ✅ Every state, open and attributed completed obligations preserved; acceptance members refused |
| FR-030 | FR-030-AC-3 | TC-221 | ✅ Unknown, duplicate, non-canonical, unordered and oversize refusals passing |
| FR-030 | FR-030-AC-4 | TC-222 | ✅ Downgrade, newer, malformed and foreign protocol refusals passing |
| FR-030 | FR-030-AC-5 | TC-223 | ✅ Per-loss and per-improvement mutation refusals passing |
| FR-030 | FR-030-AC-6 | TC-224 | ✅ Writer refusals passing |
| FR-030 | FR-030-AC-7 | TC-231 | ✅ Offline downstream crate compile, run and dependency census passing |
| FR-030 | FR-030-CON-1 | TC-220 | ✅ No acceptance member admitted |
| FR-031 | FR-031-AC-1 | TC-225 | ✅ Candidate-bound publication, no sidecar, invalid writes nothing |
| FR-031 | FR-031-AC-2 | TC-226 | ✅ Idempotent retry passing |
| FR-031 | FR-031-AC-3 | TC-227 | ✅ Attempt reuse conflict and marker passing |
| FR-031 | FR-031-AC-4 | TC-228 | ✅ Eight-writer non-identical and identical races passing |
| FR-031 | FR-031-AC-5 | TC-229 | ✅ Unbound, partial, non-regular and missing object refusals passing |
| FR-031 | FR-031-AC-6 | TC-230 | ✅ Independent attempts and candidates passing |
| IT-007 | IT-007-SC-01, IT-007-SC-02 | TC-231 | ✅ Real downstream consumer passing |
| FR-032 | FR-032-AC-1 | TC-232 | ✅ `tests/scope_footprint.rs` passing |
| FR-032 | FR-032-AC-2 | TC-233 | ✅ `tests/scope_footprint.rs` passing |
| FR-032 | FR-032-AC-3 | TC-234 | ✅ `tests/scope_footprint.rs` passing |
| FR-032 | FR-032-AC-4 | TC-235 | ✅ `tests/scope_footprint.rs` passing |
| FR-032 | FR-032-AC-5 | TC-236 | ✅ `tests/scope_footprint.rs` passing |
| FR-032 | FR-032-AC-6 | TC-237 | ✅ `tests/scope_footprint.rs` passing |
| FR-032 | FR-032-AC-7 | TC-238 | ✅ `tests/scope_footprint.rs` passing |
| FR-032 | FR-032-CON-1 | TC-234 | ✅ Truncated and unavailable footprints never complete |
| FR-036 | FR-036-AC-1 | TC-250, TC-262 | ✅ `tests/signature_verify_cli.rs` passing |
| FR-036 | FR-036-AC-2 | TC-251 | ✅ `tests/signature_verify_cli.rs` passing |
| FR-036 | FR-036-AC-3 | TC-252 | ✅ `tests/signature_verify_cli.rs` passing |
| FR-036 | FR-036-AC-4 | TC-253, TC-263 | ✅ `tests/signature_verify_cli.rs` passing |
| FR-036 | FR-036-AC-5 | TC-254 | ✅ `tests/signature_verify_cli.rs` passing |
| FR-036 | FR-036-AC-6 | TC-255 | ✅ `src/signature_verify_host.rs` unit test passing |
| FR-036 | FR-036-AC-7 | TC-256 | ✅ `tests/signature_verify.rs` and `tests/signature_verify_cli.rs` passing |
| FR-036 | FR-036-AC-8 | TC-257 | ✅ `tests/signature_verify_cli.rs` passing |
| FR-036 | FR-036-AC-9 | TC-248, TC-249 | ✅ `tests/signature_verify.rs` passing |
| FR-036 | FR-036-AC-10 | TC-258, TC-260, TC-261 | ✅ `tests/signature_verify_cli.rs` passing |
| FR-036 | FR-036-AC-11 | TC-259 | ✅ `tests/signature_verify_cli.rs` passing |
| FR-036 | FR-036-CON-1 | TC-248, TC-249, TC-250, TC-260, TC-261 | ✅ A valid status needs grade `G` with `git verify-commit` accepting, or `git verify-tag` exit 0 |
| FR-036 | FR-036-AC-12, FR-036-CON-2 | Inspection | ✅ No crate dependency added; only git and `ssh-keygen` are invoked |
| FR-033 | FR-033-AC-1 | TC-244 | ✅ `tests/python_port/module.rs` schema accept/refuse cases passing |
| FR-034 | FR-034-AC-1 | TC-246 | ✅ `tests/python_port/module.rs` schema accept/refuse cases passing |
| FR-035 | FR-035-AC-1 | TC-247 | ✅ `tests/python_port/module.rs` schema accept/refuse cases passing |

### Non-Functional Requirement Coverage

| Non-Functional Req | Verification Method | Evidence/Test Cases | Status |
|--------------------|---------------------|---------------------|--------|
| NFR-001 | Install, discovery, byte comparison, and static scan | TC-038 | ✅ Passing |
| NFR-002 | Agent evaluation against fixture-authorized outcomes | TC-039 | ✅ `tests/evaluation_parity.rs`: the aggregate gate withholds on any unsupported addition, any inferred terminal decision, and any incomplete or failing cell of the declared qualification population, and ignores an unqualified host |
| NFR-003 | Wheel/npm member audit plus compatibility invocation | TC-040 | ✅ Passing |
| NFR-005 | Exact toolchain build, unsafe audit, executable-path audit, Quire reconciliation, matrix row integrity | TC-115..TC-118, TC-205 | ✅ Executable-language and host-configuration audit (workflow YAML excluded), trace-marker and reconciliation checks passing |

### Constraint Coverage

| Constraint | Test Cases | Status |
|------------|------------|-----------------|
| FR-002-CON-1 | TC-041 | ✅ Passing |
| FR-002-CON-2 | TC-042 | ✅ Passing |
| FR-007-CON-1 | TC-043 | ✅ Passing |
| FR-014-CON-1 | TC-101 | ✅ Parsed library-module containment audit passing |
| FR-014-CON-2 | TC-101 | ✅ Parsed library-module containment audit passing |
| FR-014-CON-3 | TC-096 | ✅ Rust package foundation backed |
| FR-015-CON-2 | TC-104 | ✅ Inert generated-fixture slice backed natively against the committed projections |
| FR-015-CON-3 | TC-104 | ✅ Semantic-contract slice backed over a counted contract population |
| FR-016-CON-1 | TC-107 | ✅ Delegated ix-flow state/chain behavior passes |
| FR-016-CON-2 | TC-115 | ✅ A foreign-language path outside the approved set fails the executable-language audit |
| FR-016-CON-3 | TC-106 | ✅ Deterministic I/O-free Rust evaluator backed |
| FR-016-CON-4 | TC-105 | ✅ Pure library and binary-only host adapter backed |
| FR-016-CON-5 | TC-107 | ✅ Supported ix-flow CLI-only lifecycle path passing |
| FR-016-CON-6 | TC-107 | ✅ State-file isolation and delegated chain verification passing |
| FR-016-CON-7 | TC-107 | ✅ Direct argv and display-only next-action guards passing |
| FR-017-CON-1 | TC-109 | ✅ Backed by the tagged tests |
| FR-017-CON-2 | TC-112 | ✅ npm lifecycle hooks backed; GitHub workflow YAML is not audited |
| FR-018-CON-1 | TC-113 | ✅ Backed by the tagged tests |
| FR-022-CON-1 | TC-153 | ✅ Claim-strength-only consumer resolves no `serde_json` |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
|---------|-------|------|----------|-----------|--------|
| TC-001 | Inventory precedes every onboarding proposal | E2E | P0 | StR-001-VC-1 | ✅ |
| TC-002 | Unjustified profile is not scaffolded | E2E | P0 | StR-001-VC-2 | ✅ |
| TC-003 | Named human owns every terminal outcome | Integration | P0 | StR-001-VC-3 | ✅ |
| TC-004 | Existing valid profile is inventoried and reused | E2E | P0 | FR-001-AC-1, US-001-EX-1 | ✅ |
| TC-005 | No-profile repository creates no generic profile | E2E | P0 | FR-001-AC-2, US-001-EX-2 | ✅ |
| TC-006 | Justified artifact uses installed skeleton and Quire | Integration | P0 | FR-001-AC-3 | ✅ |
| TC-007 | Incomplete boundary requests input without writing | E2E | P0 | FR-001-AC-4 | ✅ |
| TC-008 | Inventory separates all required collections | Unit | P1 | FR-001-AC-5 | ✅ |
| TC-009 | Exactly one canonical onboarding skill exists | Static | P0 | FR-002-AC-1 | ✅ |
| TC-010 | Four host surfaces resolve one canonical skill | Integration | P0 | FR-002-AC-2, US-002-EX-1 | ✅ |
| TC-011 | Canonical skill exposes exactly four workflows | Integration | P0 | FR-002-AC-3, US-002-EX-2 | ✅ |
| TC-012 | Behavioral text in a host manifest is rejected | Static | P1 | FR-002-AC-4 | ✅ |
| TC-013 | Missing or escaping canonical target is rejected | Unit | P0 | FR-002-AC-5 | ✅ |
| TC-014 | Wheel includes explicit module and onboarding members | Integration | P0 | FR-003-AC-1 | ✅ |
| TC-015 | Npm archive includes explicit module and onboarding members | Integration | P0 | FR-003-AC-2 | ✅ |
| TC-016  | Local-source installation preserves discovery | Integration | P0 | FR-003-AC-3              | ✅ Installed module bundle is authoritative; a repository copy does not stand in for it |
| TC-017 | Repository-source installation preserves discovery | Integration | P0 | FR-003-AC-4 | ✅ |
| TC-018 | Unexpected, missing, or escaping package content fails audit | Unit | P0 | FR-003-AC-5 | ✅ |
| TC-019 | Module/plugin and local/repository install procedures are distinct | Static | P1 | FR-003-AC-6 | ✅ |
| TC-020 | Observed producer retains the complete governing-version tuple | E2E | P0 | FR-004-AC-1, US-004-EX-2 | ✅ |
| TC-021 | Invocation failure remains unavailable | E2E | P0 | FR-004-AC-2, US-004-EX-1 | ✅ |
| TC-022 | Deferred producer remains not computed | Unit | P1 | FR-004-AC-3 | ✅ |
| TC-023 | Excluded producer remains not applicable | Unit | P1 | FR-004-AC-4 | ✅ |
| TC-024 | Malformed output or missing provenance fails validation | E2E | P0 | FR-004-AC-5 | ✅ |
| TC-025 | Persisted evidence delegates to Quoin | Integration | P0 | FR-004-AC-6 | ✅ |
| TC-026 | Interrupted ix-flow run resumes without repetition | Integration | P0 | FR-005-AC-1, US-003-EX-1 | ✅ |
| TC-027 | All terminal transitions remain human gated | Static | P0 | FR-005-AC-2 | ✅ |
| TC-028 | Explicit rejection records one attributed event and no success state | Integration | P0 | FR-005-AC-3, US-003-EX-2 | ✅ |
| TC-029 | Missing human choice leaves run non-terminal | Integration | P0 | FR-005-AC-4 | ✅ |
| TC-030 | Automatic terminal-gate override fails closed | Unit | P0 | FR-005-AC-5 | ✅ |
| TC-031 | Evaluation suite executes all five classes and seven variants on four hosts | E2E | P0 | FR-006-AC-1 | ✅ |
| TC-032 | Evaluation envelopes retain versions, transcript paths, effort, and outcomes | Property | P0 | FR-006-AC-2 | ✅ |
| TC-033 | Missing executable fails aggregate gate | E2E | P0 | FR-006-AC-3 | ✅ |
| TC-034 | Aggregate passes only for complete passing scenarios | Property | P0 | FR-006-AC-4 | ✅ |
| TC-035 | Four pilot workflow invocations still load | Integration | P0 | FR-007-AC-1 | ✅ |
| TC-036 | Pilot and canonical workflows are equivalent | Property | P0 | FR-007-AC-2 | ✅ |
| TC-037 | Canonical install docs precede compatibility path | Static | P1 | FR-007-AC-3 | ✅ |
| TC-038 | Cross-agent canonical parity reaches all thresholds | Integration | P0 | NFR-001-AC-1, NFR-001-AC-2, NFR-001-AC-3 | ✅ |
| TC-039 | Evaluation produces zero unsupported outcomes | E2E | P0 | NFR-002-AC-1, NFR-002-AC-2, NFR-002-AC-3 | ✅ |
| TC-040 | Package contract stability reaches all thresholds | Integration | P0 | NFR-003-AC-1, NFR-003-AC-2, NFR-003-AC-3 | ✅ |
| TC-041 | Supported-agent set rejects missing, extra, or duplicate hosts | Property | P1 | FR-002-CON-1 | ✅ |
| TC-042 | Thin manifests reject behavioral sections and copied workflows | Property | P1 | FR-002-CON-2 | ✅ |
| TC-043 | Compatibility inventory is exactly the four promoted pilots | Property | P1 | FR-007-AC-4, FR-007-CON-1 | ✅ |
| TC-044 | Malformed or conflicting existing artifacts remain unchanged and require human resolution | E2E | P0 | FR-001-AC-6 | ✅ |
| TC-045 | Artifact publication is staged, Quire-validated, atomic, and confined to the selected root | Integration | P0 | FR-001-AC-7 | ✅ |
| TC-046 | Exactly one availability state exists for every considered producer | Property | P0 | FR-004-AC-7 | ✅ |
| TC-047 | Explicit acceptance records one attributed event and no prior acceptance state | Integration | P0 | FR-005-AC-6 | ✅ |
| TC-048 | A run-id binding mismatch is refused without changing either run | Property | P0 | FR-005-AC-7 | ✅ |
| TC-049 | Equivalent runs retain explicit acceptance and rejection on every host | E2E | P0 | FR-006-AC-5 | ✅ |
| TC-052 | Complete cross-component semantic fixture validates, keeps every semantic identity distinct, covers all eight concepts, and retains `measurement_plan` as an external authoritative link | Integration | P0 | StR-002-VC-1, IT-005 | ✅ Native Rust fixture validation |
| TC-053 | Ownership/type-fit registry names all eight concepts exactly once, assigns each the reviewed authority, and stays non-executing; an incomplete, duplicated, re-assigned, or executing registry is refused with its typed reason | Static | P0 | US-005-AC-1 | ✅ Native Rust registry validation with refusal mutants |
| TC-056 | Every concept has exactly one authority and link direction | Property | P0 | FR-008-AC-1 | ✅ Native Rust registry and reference validation |
| TC-057 | Definition, execution, result, evidence, and report ids stay distinct | Property | P0 | FR-008-AC-2 | ✅ Native Rust bundle validation |
| TC-058 | Absent, concept-confused, self-referencing, and omitted required semantic links each fail validation with a distinct typed reason | Property | P0 | FR-008-AC-3 | ✅ Native Rust bundle validation |
| TC-059 | The pure semantic library reaches for no filesystem, environment, or child-program capability, and the audit's own detector is proven against capability mutants | Static | P0 | FR-008-AC-4, NFR-004-AC-2 | ✅ Native Rust source audit with mutants |
| TC-060 | Complete producer tuple and definition version survive projection for every producer-owned concept in a non-empty population; an absent tuple, an absent tuple field, and an empty environment each refuse | Unit | P0 | FR-009-AC-1 | ✅ Native Rust reference validation |
| TC-061 | A non-empty declared non-success-state set survives every language projection without collapsing, and the committed projections agree across Python, TypeScript, and Rust | Property | P0 | FR-009-AC-2 | ✅ Native Rust state and projection comparison |
| TC-062 | Unknown versions and missing provenance fail explicitly | Property | P0 | FR-009-AC-3 | ✅ Native Rust fixture refusals |
| TC-063 | A reference whose source field path is not an absolute JSON pointer, or whose source record id is empty, is refused with its typed reason | Property | P0 | FR-009-AC-4 | ✅ Native Rust reference validation |
| TC-068 | Schema/ownership audit reads a non-empty contract population and finds no duplicate record family, and every value this library produces still conforms to the schemas this repository publishes for it | Static | P0 | NFR-004-AC-1 | ✅ Native Rust contract audit and published-schema conformance |
| TC-087 | Every family in the decision table carries exactly one of keep, delete, or replace | Static | P0 | FR-013-AC-1 | ✅ |
| TC-088 | The decision table accounts for every recurring script family present in the eight repositories, and states when the sources cannot be read | Integration | P0 | FR-013-AC-2 | ✅ |
| TC-089 | Repository-local generic evidence schemas and stdout-derived verdicts are forbidden by name, and a domain-output schema is permitted | Static | P0 | FR-013-AC-3 | ✅ |
| TC-090 | Domain output validation, evidence intake, audit, and human decision each name a distinct owner | Static | P0 | FR-013-AC-4 | ✅ |
| TC-091 | Rollback is defined per failure mode, legacy history is never rewritten, and deletion is last | Static | P0 | FR-013-AC-5 | ✅ |
| TC-092 | The review checklist covers inventory, both prohibitions, byte-identical legacy evidence, every non-success state, and manual dispatch | Static | P0 | FR-013-AC-6 | ✅ |
| TC-093 | All eight repositories appear exactly once in the Agent A/B/C allocation | Static | P0 | FR-013-AC-7 | ✅ |
| TC-094 | The contract changes no trigger and makes no qualification claim | Unit | P0 | FR-013-AC-8, FR-013-CON-2, FR-013-CON-3 | ✅ |
| TC-096 | Existing repository builds the named Rust library and CLI | Compile | P0 | StR-003-VC-1, FR-014-AC-1, FR-014-CON-3 | ✅ Rust package foundation backed |
| TC-098 | Machine CLI output obeys the versioned stdout/stderr contract | Property | P0 | FR-014-AC-2 | ✅ `tests/package_boundary.rs` command table and `tests/manifest_host_cli.rs` passing; `agent-evals` (launches real agents) and `onboard-report` (human-readable text) are outside the table |
| TC-099 | Invalid protocol, input, root, host, and host response fail before side effects | Property | P0 | FR-014-AC-3 | ✅ Refusal-before-write cases across onboarding, workflow-host and process adapter passing |
| TC-100 | For each ADR-002 semantic/projection/fixture capability row, Rust agrees byte-for-byte with the accepted identity over focused fictional and adverse cases, including generated arbitrary-precision integer boundaries | Integration | P0 | StR-003-VC-3, FR-015-AC-1 | ✅ Fixture-generation slice is backed against reference bytes captured once from the retired implementation and committed to this repository; no retained implementation is executed as a differential oracle in this row's qualification path |
| TC-101 | Parsed Rust syntax for every reusable-library module rejects filesystem, environment, child-program, network, persistence, and arbitrary-stdout capability paths, including lexical aliases declared in that source, without treating comments or literals as code or claiming cross-document compiler resolution | Static | P0 | FR-014-AC-4, FR-014-CON-1, FR-014-CON-2 | ✅ AST audit, closed findings, adverse scope/alias cases, and mutation evidence passing |
| TC-102 | Every successful and non-successful semantic state remains distinct; typed state spellings and exact-one untyped-label validation remain callable without string parsing | Integration | P0 | FR-015-AC-2 | ✅ evidence-availability and semantic-reference state vocabularies backed |
| TC-103 | Invalid semantic inputs, including non-finite/retained-domain-overflow numbers and invalid version tokens, fail without a source-byte change; independent cases retain leading/trailing and interior ASCII space boundaries; every exercised semantic refusal exposes a typed reason without diagnostic-prose matching | Integration | P0 | FR-015-AC-3 | ✅ evidence and semantic refusals backed |
| TC-104 | Contract ownership and inert foreign-language fixtures remain bounded over non-empty audited populations | Static | P0 | FR-015-AC-4, FR-015-CON-2, FR-015-CON-3 | ✅ semantic ownership, contract-family, library-capability, and inert-fixture slices backed |
| TC-105 | Rust onboarding produces the declared complete sorted inventory, status, recommendation, and artifact path for accepted existing, absent, conflicting, malformed, unavailable, unjustified, incomplete-boundary, and valid-authoring fixtures without executing another implementation; duplicate-key and merge-key artifact identities remain malformed; newly authored artifacts retain the declared parsed frontmatter and Markdown body and pass Quire, while malformed requests, unsupported types, absolute or parent-traversing targets, symlink escapes, existing destinations, and invalid staged artifacts publish nothing | Property | P0 | FR-016-AC-1, FR-016-CON-4 | ✅ Native explicit-oracle core/CLI, fail-closed YAML identity, real Quire validation, confined publication, and legacy lane removal passing |
| TC-106 | Rust and the retained invariant provider return the same ordered typed outcomes for all eleven canonical invariants and valid boundary fixtures at one explicit evaluation instant; unknown invariant names and malformed Rust requests fail before an outcome | Integration | P0 | FR-016-AC-2, FR-016-CON-3 | ✅ Rust evaluator, retained-reference parity, fail-closed probes, and CLI boundary passing |
| TC-107 | Rust start/resume and explicit decision coordination preserves ix-flow state ownership, recovers only pristine interrupted initialization, remains idempotent across gate interruption windows, requires ix-flow to verify an intact event chain, refuses binding/transition/decision conflicts before mutation, reports ambiguous post-mutation host failures as indeterminate for status reconciliation, bounds malformed/oversized/timed-out responses, and never supplies an automatic gate override or executes next-action text | Integration | P0 | FR-016-AC-3, FR-016-CON-1, FR-016-CON-5, FR-016-CON-6, FR-016-CON-7, FR-028-AC-1, FR-028-AC-2 | ✅ implementation and adverse cases pass |
| TC-109 | Rust evaluation completes the declared host-scenario matrix without inferred decisions | E2E | P0 | FR-017-AC-1, FR-017-CON-1 | ✅ Backed by the tagged tests |
| TC-136 | A `describe` response omits `title` and `canary` from the emitted bytes for every scenario that carries neither, still emits `canary` for the canary scenario and `use_case` for every scenario, and emits no null anywhere in the described catalogue | Integration | P0 | FR-017-AC-9 | ✅ emitted-byte omission backed |
| TC-137 | Each coverage refusal condition — incomplete totals, unbacked rows, status lies, first-party untracked symbols, a fatal local diagnostic, a skipped status classification, and an unparseable document — yields its own code, every offending row is named with its authored document and line, all failing conditions are reported together, and no repository gap is reported as a coverage-tool failure | Integration | P0 | FR-017-AC-10 | ✅ condition-specific located refusals backed |
| TC-110 | The pure typed Rust evaluator preserves the exact declared-population contract and deterministically withholds aggregation for missing, duplicate, malformed, unsupported, unavailable, failed, changed-governing-identity, changed-workflow, unsupported-addition, invalid-transcript-reference, invalid-count, outcome-mismatch, and terminal-pair cases; input permutation cannot change the result, oversized input refuses before decoding, and the reusable boundary performs no I/O | Property | P0 | FR-017-AC-2, FR-017-CON-1, FR-017-CON-3 | ✅ typed Rust aggregation, retained-Python differential cases, adverse cases, portable path/refusal rules, resource ceiling, and mutation probes passing |
| TC-111 | Each package, rights, manifest, integration, and publication-refusal capability has independent positive and negative subcases matching the retained gate; npm stage/cleanup covers missing, linked, special, non-portable, oversized, excessive, pre-existing, changed, exact-copy, rollback, idempotent-absence, direct-JSON, and hook-empty-stdout states; the content-rights tree adapter covers exact Git selection, tracked/untracked/ignored entries, gitlinks, links, special files, protected tokens, repository-root identity, deterministic findings, and every exact/over resource boundary; the package-audit adapter covers pre-build linked/special/unsafe/over-limit source refusal including wheel build configuration, direct bounded builder/installer processes and descendant groups, fail-closed hosts without process-group containment, invocation-owned package-manager cache/temporary paths, bounded top-level output correspondence, preflight-absent and exact post-process npm staging cleanup on success and failure, exact archive selection, validated regular/zero-payload-directory kinds, raw tar extension-header refusal before preprocessing, safe unique member names, exact/over archive bytes, total entries, member bytes, and aggregate expanded bytes, traceability coverage that counts an untracked trace tag as a gap only when it lies outside every declared submodule, independent report/archive allowlist agreement, every content-rights category without disclosure, exact license/private metadata, offline installs, thin host discovery, canonical/pilot YAML equivalence, installed links/special files/escapes, and cross-format canonical byte identity | Property | P0 | FR-017-AC-3 | ✅ bounded Rust package, rights, manifest, integration, publication-refusal, archive, install, correspondence, and adverse-case slices reviewed and passing |
| TC-112 | Package-manager and host files contain declarative dispatch only; npm lifecycle hooks invoke the local Rust CLI in explicit hook mode without embedded staging/refusal semantics or contamination of npm output; `make test` invokes the Rust content-rights tree adapter and `make package-audit` invokes the Rust package-audit adapter after their respective correspondence; local qualification, real-agent evaluation, publication, and release operations remain explicit manual actions | Static | P0 | FR-017-AC-4, FR-017-CON-2 | ✅ npm lifecycle, content-rights, and package-audit Rust dispatch backed; GitHub workflow YAML is not audited |
| TC-113 | Removal refuses incomplete parity, and direct invocations that still use the old path; content-rights removal additionally requires exact full-tree status/finding correspondence and an independently passing Rust tree and package-audit dispatch, followed by removal of every executable import or subprocess reference to the deleted Python paths | Property | P0 | FR-018-AC-2, FR-018-CON-1 | ✅ content-rights and package-audit cutovers plus final four-file deletion candidate backed |
| TC-115 | Final audit finds no unapproved non-Rust semantic or assertion logic | Static | P0 | StR-003-VC-2, StR-003-VC-4, FR-016-CON-2, FR-018-AC-4, NFR-005-AC-4 | ✅ `tests/python_port/module.rs` executable-language audit passing |
| TC-117 | Parsed first-party Rust tests under `src/` and `tests/` use an exact unaliased `ix_trace_rs::trace` import and bare trace attributes carrying both TC and AC literals; absent, aliased, path-qualified, malformed, invalid-source, and over-limit cases fail closed | Static | P0 | NFR-005-AC-2 | ✅ Parsed repository census, canonical-form adverse cases, and mutation evidence passing |
| TC-118  | Quire reconciles every Rust test marker without missing, orphaned, or duplicate bindings | Integration | P0       | NFR-005-AC-3             | ✅ Real Quire reconciliation over this repository; submodule markers excluded, empty population refused |
| TC-119 | The pure Rust content-rights classifier matches retained finding/exception behavior, rejects unsafe paths without echoing them, preserves Unicode protected-token matching, leaks no matched content, and remains deterministic and I/O-free | Property | P0 | FR-017-AC-5, FR-017-CON-3, FR-028-AC-4 | ✅ typed Rust classifier, retained-Python differential, exhaustive boundary cases, and mutation probes passing |
| TC-120 | The pure Rust package-membership classifier matches retained extra/missing behavior for safe unique names; rejects invalid expected policies and population limits; withholds on invalid, duplicate, unexpected, or missing observed members without echoing unsafe paths; remains permutation-invariant and I/O-free; and does not select or decode a package format | Property | P0 | FR-017-AC-6, FR-017-CON-3 | ✅ Typed Rust policy, retained-Python differential, adverse boundaries, ordering, resource ceilings, and mutation probes passing |
| TC-121 | The pure Rust module-manifest classifier accepts the retained valid module; consumes caller-supplied module and artifact schemas without discovering, fetching, or copying one; and deterministically withholds for every malformed manifest/schema/registry/resource/frontmatter/heading case without unsafe-reference or source-byte disclosure, I/O, schema discovery, or copied manifest grammar | Property | P0 | FR-017-AC-7, FR-017-CON-3 | ✅ `tests/manifest_parity.rs` (locally authored, non-authoritative fixture schema/registry; the retained module is accepted against those fixtures, not against the FR-035 schema — see PLAT-992); real FR-035/edge-registry conformance of the retained manifest is separately backed by `tests/manifest_host_cli.rs`'s `#[ignore]`d `tc_121_real_manifest_conforms_to_the_authoritative_schema_and_edge_registry`, requiring explicit `EA_MANIFEST_SCHEMA_ROOT`/`EA_MANIFEST_REGISTRY_ROOT` roots (PLAT-994) |
| TC-122 | A valid Rust synthetic executable and retained input run once through the public Rust executor and produce one typed completed result carrying producer/caller/adapter provenance, monotonic timing, bounded raw evidence, immutable retained output bytes, caller-adapter observation and canonical result bytes; a declared fixed output present before launch is removed (a symlink as a link, its target untouched) and observed only if the producer writes it | Integration | P0 | FR-019-AC-1 | ✅ Rust fixture, non-canonical refusal, retained-output adapter, path-replacement, stale flat and nested declared-output removal, missing-parent skip, symlink and directory refusal cases passing |
| TC-123 | List order is significant while declared sets normalize; structurally invalid input produces no result; capability/adapter/cancellation mismatches refuse before launch; the valid child observes no shell or ambient environment and reads a declared input mutated after preflight as its preflight snapshot through the descriptor argument | Property | P0 | FR-019-AC-2, FR-019-CON-1 | ✅ typed preflight refusals, closed environment and post-preflight declared-input mutation cases passing |
| TC-124 | Unavailable, refused, failed, timed-out, malformed-response, containment-failure, cancelled, and completed results remain distinct and canonically serializable; launched states retain bounded raw process evidence where observable and only completed carries the caller-owned typed observation | Property | P0 | FR-019-AC-3 | ✅ all eight Rust-fixture states, launched-evidence cases passing |
| TC-125 | Exact wall-clock, stream, input-byte, output-count/byte, and concurrency boundaries are admitted and the next value is invalid or terminates; admitted non-zero exits reach the adapter; ordinary descendants are reaped; an escaping-descendant mutant becomes containment-failure under the explicitly cooperative profile | Integration | P0 | FR-019-AC-4, FR-019-CON-4 | ✅ Rust fixture exact/over resource limits, admitted nonzero exit, cancellation, group reaping and escaping-descendant cases passing |
| TC-127 | Static ownership and capability checks admit process execution only in the FR-019 module and find no Engineering Assurance domain oracle, generic stdout verdict scraper, qualification decision, evidence store, Quoin schema clone, or second crate/repository | Static | P0 | FR-014-AC-4, FR-019-AC-5, FR-019-CON-2, FR-019-CON-3, FR-027-CON-1, NFR-004-AC-2 | ✅ parsed process-ownership census, shared-kernel cutover, forbidden parser/persistence dependency checks, and single-crate check passing |
| TC-128 | A minimal downstream crate compiles Engineering Assurance with default features disabled, only `producer-execution`, and an exact semver-compatible shared-dependency version older than Engineering Assurance's locked version; every normal Engineering Assurance dependency is a compatible range rather than an exact `=` requirement; Engineering Assurance activates no direct dependency used solely for package/archive, onboarding, CLI, YAML, regex or source-audit behavior while shared dependencies remain admissible and the `full` feature preserves existing package gates | Compile | P0 | FR-019-AC-7 | ✅ isolated offline downstream resolution/compile, normal dependency-requirement census and activated direct-dependency census passing; `full`-package gates passing |
| TC-129 | The Rust report adapter decodes `cli-agent-evals.report/v1`, admits only retained successful single-run samples beneath the explicit workspace root, locates the transcript, preserves failed-attempt and host-model behavior, refuses every malformed/version/path/link/kind/resource/population case, remains input-order invariant, and matches the retained Python aggregate artifact before Rust command cutover | Integration | P0 | FR-017-AC-1, FR-017-CON-1, FR-017-CON-3 | ✅ retained-report adapter, parity, and direct Rust dispatch passed; obsolete wrapper removed. Live declared-population execution remains TC-109. |
| TC-131 | The Rust manifest host adapter accepts the retained module only from explicit safe repository, authoritative-schema, and authoritative-registry roots, delegates its closed resource bundle to the pure qualifier, emits one versioned result, and refuses unsafe roots or resources before qualification | Integration | P0 | FR-017-AC-8, FR-017-CON-3, FR-014-AC-2 | ✅ `src/manifest_host.rs`, `tests/manifest_host_cli.rs`; explicit `make manifest-validate` dispatch with separate `MANIFEST_SCHEMA_ROOT`/`MANIFEST_REGISTRY_ROOT` (PLAT-994); obsolete wrapper removed |
| TC-138 | A minimal downstream crate compiles Engineering Assurance with default features disabled, only `source-audit`, and reaches `source_audit`; that consumer activates no `serde_json` direct dependency at all, so it cannot inherit an `arbitrary_precision` feature flip from elsewhere in a downstream workspace's feature unification; the `full` feature preserves the existing `source_audit` gate | Compile | P0 | FR-014-AC-5 | ✅ isolated offline downstream resolution/compile and activated direct-dependency census passing; `full`-package gate unaffected |
| TC-139 | The MeasurementPlan frontmatter schema accepts no objective, every direction with and without a numeric bound, and target with a bound; it rejects target without a bound, an unknown or missing direction, a non-numeric bound and an extra objective key; the skeleton's objective validates | Unit | P0 | FR-020-AC-1 | ✅ schema accept/refuse cases passing |
| TC-140 | The Rust Objective accepts every direction, refuses target without a bound and a non-finite bound with distinct typed errors through both construction and deserialization, refuses unknown directions and extra keys, and its Direction wire-name set equals the schema's direction enum | Unit | P0 | FR-020-AC-2 | ✅ construction, deserialization and wire-set parity cases passing |
| TC-141 | An objective, estimator, decision rule or protected_apparatus list added, removed or changed under an equal definition_version yields one typed finding carrying that version, each changed member in order, and both definitions; the same edits under a changed definition_version, and an unchanged definition, yield no finding | Unit | P0 | FR-020-AC-3, FR-021-AC-8, FR-024-AC-5 | ✅ finding and no-finding cases passing |
| TC-142 | A minimal downstream crate compiles Engineering Assurance with default features disabled and only `measurement`, reaches Objective, the definition-change check, Estimator, Comparator, Baseline, DecisionRule, ApparatusPath, ProtectedApparatus, NegativeControlKind, NegativeControl and NegativeControls, and resolves no serde_json package anywhere in its dependency graph | Compile | P0 | FR-020-AC-4, FR-021-AC-7, FR-024-AC-7, FR-026-AC-6 | ✅ isolated offline downstream compile and whole-graph census passing |
| TC-143 | The AssuranceArgument schema rejects a `supported` claim without `evidence_refs` and accepts it with one `ix://` reference; `open`, `challenged`, and `rejected` claims are accepted with or without references; empty, duplicated, and non-`ix://` reference lists are rejected | Unit | P0 | FR-023-AC-1, FR-023-AC-2, FR-023-AC-3 | ✅ `tests/python_port/module.rs` |
| TC-144 | The MeasurementPlan frontmatter schema accepts a decision rule with every comparator against a threshold and every baseline with and without a margin; it rejects an unknown or missing comparator, neither or both of threshold and baseline, a margin with a threshold or with eq, a non-numeric margin, an unknown baseline, eq against best-seen, a non-numeric threshold, an extra rule key and a prose rule; it accepts a non-finite threshold; it requires metric when statistical_design is present; it accepts comparators that agree with the objective direction and refuses those that disagree; and it allows constant-predictor only with estimator proportion | Unit | P0 | FR-021-AC-1, FR-021-AC-9 | ✅ schema accept/refuse and consistency cases passing |
| TC-145 | The schema accepts each of the five estimators and rejects any other value, keeps population, sampling, error_model and uncertainty as prose, and the skeleton's structured estimator and decision rule validate with an objective bound distinct from the rule's threshold | Unit | P0 | FR-021-AC-2 | ✅ estimator, prose-field and skeleton cases passing |
| TC-146 | The Rust DecisionRule accepts threshold and baseline rules, refuses neither or both references, a margin without a baseline or with eq, eq against best-seen and a non-finite threshold or margin with distinct typed errors through construction and deserialization, refuses unknown comparators, baselines, estimators, a non-numeric margin and extra keys, round-trips a valid rule, refuses a comparator that disagrees with the objective direction, and refuses constant-predictor with any estimator but proportion | Unit | P0 | FR-021-AC-3, FR-021-AC-9 | ✅ construction, deserialization, round-trip and consistency cases passing |
| TC-147 | The schema's estimator, comparator and baseline enums equal the Rust Estimator, Comparator and Baseline wire-name sets in order, and each ALL constant covers every variant | Unit | P0 | FR-021-AC-4 | ✅ wire-set parity and variant-coverage cases passing |
| TC-148 | Each comparator holds exactly for the estimates below, at or above the reference its symbol names, and none holds against NaN; a baseline rule compares against the baseline value moved by its margin in the direction of improvement, for higher- and lower-is-better rules and positive and negative margins; a missing or unexpected baseline value, a non-finite estimate or baseline value, and an overflowing reference are typed refusals | Unit | P0 | FR-021-AC-5 | ✅ comparator, margin, overflow and refusal cases passing |
| TC-149 | The onboarding checklist lists the estimator, comparator and baseline sets, the one-of choice between threshold and baseline, margin's dependency on baseline, metric's dependency on statistical_design, the direction and estimator consistency rules and the refused eq combinations, with no warning | Unit | P0 | FR-021-AC-6 | ✅ onboarding JSON checklist case passing |
| TC-150 | Each claim strength renders, serializes, parses, and deserializes as exactly `proven`, `bounded-checked`, `tested`, or `observed`; the ownership registry declares exactly those values with Engineering Assurance ownership, one strength per advanced result, and no order, and refuses each misdeclared owner, type, cardinality, ordering, population, order, duplicate, unknown value, or absent entry; the generated Python, TypeScript, and Rust listings carry the same values | Unit | P0 | FR-022-AC-1, FR-022-AC-5 | ✅ wire spelling, registry entry, misdeclaration refusals, and generated listings backed |
| TC-151 | Every label outside the claim-strength vocabulary, including case, separator, whitespace, unrelated, empty, numeric, and null values, is refused by parsing and deserialization with a typed error carrying the refused label | Unit | P0 | FR-022-AC-2 | ✅ exact-match typed refusal backed |
| TC-152 | The listing of claim strengths names every variant exactly once; the library build fails if an ordering is derived, and a downstream consumer that compares, orders, takes a maximum of, collects into an ordered set, or defaults a strength fails to compile for that reason | Compile | P0 | FR-022-AC-3 | ✅ exhaustive listing, trait-absence probe, and downstream ordering refusals backed |
| TC-153 | A minimal downstream crate compiles Engineering Assurance with default features disabled and only `claim-strength`, uses the type, and resolves no `serde_json` or other optional Engineering Assurance dependency anywhere in its graph | Compile | P0 | FR-022-AC-4, FR-022-CON-1 | ✅ isolated offline downstream compile and whole-graph dependency census backed |
| TC-154 | The MeasurementPlan frontmatter schema accepts no protected_apparatus below gate stage and every accepted file path and directory entry (`<directory>/**`) of the shared case table `tests/fixtures/apparatus-paths.json`, and rejects an empty list, a repeated entry, a non-list, a non-string entry and every refused entry of that table (empty, absolute, empty segment, `.` or `..` segment, `**` alone, `*` outside a final `/**`, forbidden or control character including a trailing newline, DEL and C1) | Unit | P0 | FR-024-AC-1 | ✅ schema accept/refuse cases from the shared table passing |
| TC-155 | The schema accepts each negative-control kind, requires negative_controls at gate stage, accepts an independent gate without protected_apparatus, requires protected_apparatus at every stage when execution_procedure or an apparatus-edit control is present, and rejects an empty list, unknown or missing kind, missing or empty description, extra key, repeated control and bare kind string | Unit | P0 | FR-024-AC-2, FR-024-AC-8 | ✅ schema accept/refuse, procedure and apparatus-edit cases passing |
| TC-156 | The Rust ApparatusPath accepts every accepted entry of the shared case table, reporting a directory entry's directory, and refuses every refused entry with the ApparatusPathError variant the table names through construction, parsing and deserialization; the schema pattern agrees on every case; ProtectedApparatus refuses an empty or repeated list, compares as a set and round-trips sorted | Unit | P0 | FR-024-AC-3 | ✅ shared-table construction, parsing, deserialization, pattern agreement and set cases passing |
| TC-157 | The schema's negative-control kinds equal the Rust NegativeControlKind wire names in order, ALL covers every variant, the gate-stage then requires negative_controls, and NegativeControl and NegativeControls refuse an empty description, an empty list, a repeated control, unknown kinds and extra keys, and round-trip | Unit | P0 | FR-024-AC-4 | ✅ wire parity, refusal and round-trip cases passing |
| TC-158 | A protected entry added, removed or changed, or the list added or removed, under an equal definition_version yields one typed finding naming protected_apparatus; the same edits under a changed definition_version, and a reordered list, yield no finding | Unit | P0 | FR-024-AC-5 | ✅ finding, versioned and reorder cases passing |
| TC-159 | The MeasurementPlan skeleton carries a valid protected_apparatus and negative_controls with a section for each, the onboarding skill describes both, the onboarding checklist lists both lists' shape, the entry description rather than the raw pattern, the control kinds and the procedure and apparatus-edit requirements, omits lists with nothing to say, and reports no warning for any artifact type | Unit | P0 | FR-024-AC-6 | ✅ skeleton, skill, onboarding JSON and text checklist cases passing |
| TC-160 | With no policy, a recommend policy, or a require policy not listing the proposed stage, measurement.promotion_ready passes with no details for a missing, mismatched, rejected, order-unattested, accepted and later-schema checker result, and the JavaScript provider emits the same bytes | Unit | P0 | FR-025-AC-2, FR-025-AC-5 | ✅ recommend-mode pass and parity cases passing |
| TC-161 | With a require policy listing the proposed stage, an attested accept passes; a missing result, unbound or other-plan evidence, another schema, reject, inconclusive, another definition version or candidate, a null or absent candidate, and a caller-supplied, none or git-shallow order each fail with their promotion_checker_* code and report; conflicting results fail as not_accepted in either order; the JavaScript provider emits the same bytes | Unit | P0 | FR-025-AC-3, FR-025-AC-5 | ✅ require-mode accept, refusal and parity cases passing |
| TC-162 | With a require policy, a current owned exception lets a rejected or missing result pass; an exception expiring at the evaluation instant or without an owner does not; an unknown verdict or mode, mistyped reasons or stages, an unknown member on either item, and a non-object checker item are refused before outcomes | Unit | P0 | FR-025-AC-4, FR-025-AC-5 | ✅ override, expiry, ownership, parity and malformed-item cases passing |
| TC-163 | The AssuranceProfile schema accepts no measurement_policy and either mode over a non-empty list of distinct MeasurementPlan stages, refuses a missing mode or stage list, unknown mode, empty, repeated, unknown or non-list stages, an extra member and a non-object policy, and its stage and mode values equal the MeasurementPlan stage enum and review_policy modes | Unit | P0 | FR-025-AC-1 | ✅ schema accept/refuse and vocabulary parity cases passing |
| TC-164 | The canonical and pilot measurement-promotion definitions are equal, declare measurement_verdict and measurement_policy item schemas, gate evidence_ready to decision_ready on promotion_ready and exceptions_ready, keep both terminals hitl, and the pilot provider refuses a rejected result under a require policy for gate | Integration | P0 | FR-025-AC-3, FR-025-AC-6 | ✅ definition, terminal-gate and pilot-provider cases passing |
| TC-165 | The AssuranceProfile skeleton carries a valid measurement_policy and a section explaining it, the onboarding skill names the run items, attested order, failure codes and override, and the onboarding checklist lists the policy's modes, stage values, list shape and required members with no warning | Unit | P0 | FR-025-AC-7 | ✅ skeleton, skill and onboarding checklist cases passing |
| TC-166 | The objective schema's properties are exactly direction, bound, weight, value_half_life and budget, with only direction required; it accepts no steering fields, all three together, weight 0, budget 0, and a small positive value_half_life; it rejects a negative weight, a negative budget, a zero or negative value_half_life, and a non-numeric (string, boolean, array, null) value for any of the three | Unit | P0 | FR-026-AC-1 | ✅ `tests/python_port/module.rs` schema accept/refuse and range cases passing |
| TC-167 | The Rust Objective::with_steering accepts every valid weight/value_half_life/budget combination including weight 0 and budget 0, and refuses a non-finite or negative weight, a non-finite, zero or negative value_half_life, and a non-finite or negative budget with distinct typed ObjectiveError variants through both construction and deserialization (including YAML `.inf`/`.nan`); a valid objective with steering fields round-trips | Unit | P0 | FR-026-AC-2 | ✅ construction, deserialization and round-trip cases passing |
| TC-168 | An objective edit that changes only weight, value_half_life or budget under an equal definition_version yields no definition-change finding in either direction; the same MeasurementDefinition pair with a bound change alongside unchanged steering fields still yields one finding naming objective | Unit | P0 | FR-026-AC-3 | ✅ steering-exclusion and bound-still-reported cases passing |
| TC-169 | Independently-parsed MeasurementPlan frontmatter documents (objective plus statistical_design.decision_rule) with no steering fields, the smallest accepted values (weight 0, smallest positive value_half_life, budget 0), and the largest finite values parse to equal decision rules and equal definitional objectives, agree on direction/comparator consistency, and produce the expected identical holds() verdict for a threshold rule and a baseline rule at every probed estimate including each exact boundary | Unit | P0 | FR-026-AC-4 | ✅ adversarial-vs-plain verdict-parity cases passing |
| TC-170 | The MeasurementPlan skeleton carries a valid objective with weight, value_half_life and budget, a "Steering Fields" section stating they are advisory only, and validates against the schema; the onboarding skill names the three fields and says they never gate; the onboarding checklist reports no MeasurementPlan warning | Unit | P0 | FR-026-AC-5 | ✅ `tests/python_port/module.rs` skeleton case passing |
| TC-171 | The schema and the Rust DecisionRule accept baseline external-reference with every estimator, including proportion, and with comparator eq, unlike constant-predictor's proportion-only restriction and best-seen's eq refusal; both refuse it with eq plus a margin and with a comparator disagreeing with the objective direction; evaluation without a supplied value is MissingBaselineValue naming external-reference, and a supplied value is moved by the margin; the Rust Baseline type round-trips external-reference through construction, serialization and deserialization | Unit | P0 | FR-021-AC-10 | ✅ external-reference acceptance, estimator-agnostic, eq-compatible, refusal and evaluation cases passing |
| TC-172 | A retired MeasurementPlan accepts the exact prose statistical design without newly introduced gate declarations, while active and proposed plans refuse prose and require all gate declarations; malformed or hybrid legacy designs are refused, and retired current designs stay valid | Unit | P0 | FR-021-AC-11, FR-024-AC-9 | ✅ focused Rust JSON Schema accept/refuse cases passing |
| TC-173 | A retired AssuranceProfile accepts historical profile_version 0.2; active and proposed profiles refuse it, invalid versions and both version spellings are refused, and current schema_version 0.2 remains valid | Unit | P0 | FR-025-AC-8 | ✅ focused Rust JSON Schema accept/refuse cases passing |
| TC-174 | `engineering-assurance onboard-report --help` and `-h` print usage on stdout and exit 0; an unknown argument, `--repo` without a path, and a repeated `--repo` print the error and usage on stderr and exit 2; none prints a report | Unit | P1 | FR-001-AC-8 | ✅ `tests/python_port/onboarding_script.rs` usage cases passing |
| TC-175 | A complete synthetic executor-observed host context exposes no raw machine or boot identifier, and an incomplete or malformed context is absent; ARM CPU-ID observations select only allowed CPUs and withhold context for mixed or missing selected tuples; Linux execution samples this context before launch | Unit | P0 | FR-019-AC-8 | ✅ parser cases passing |
| TC-176 | A bounded output-tree request changes identity with its path or required bit and rejects nested fixed outputs or overlapping trees; after execution a required tree that is absent fails the run and an absent non-required tree completes with no artifacts | Unit | P0 | FR-019-AC-4 | ✅ request-structure and absent-tree cases passing |
| TC-177 | FCD-generated procedure wire rejects unknown fields and argument kinds, while authored bounds and artifact roles are checked before request creation | Unit | P0 | FR-019-AC-9, VO-001 | ✅ `tests/campaign.rs` closed-wire and role cases passing |
| TC-178 | An authored output-tree role requires an exact runtime binding; its selected path is carried in the EA request | Unit | P0 | FR-019-AC-10, VO-001 | ✅ `tests/campaign.rs` declaration/binding cases passing |
| TC-179 | A campaign rejects stale or absent plan procedures, unresolved dependencies, and dependency cycles without recursive graph traversal | Unit | P0 | FR-019-AC-9, VO-004 | ✅ `tests/campaign.rs` graph cases passing |
| TC-180 | A procedure resolves into an EA bounded execution request reflecting selected input, arguments, environment, and source inputs | Unit | P0 | FR-019-AC-10, VO-001 | ✅ `tests/campaign.rs` resolver cases passing |
| TC-181 | A run rejects unknown members and malformed attempt inventories | Unit | P0 | FR-019-AC-11, VO-007 | ✅ `tests/campaign.rs` run-shape cases passing |
| TC-182 | A complete Git tree manifest expands tracked regular files, executable mode, and omitted symlink targets; a substituted parent link or unsupported mode refuses source projection | Integration | P0 | FR-019-AC-11, VO-005 | ✅ `tests/campaign.rs` real temporary Git repository cases passing |
| TC-183 | Dynamic dependency input role prefixes accept only declared path components, require a matching input when marked required, and reject undeclared near matches or malformed prefixes | Unit | P0 | FR-019-AC-10, VO-011 | ✅ `tests/campaign.rs` prefix binding cases passing |
| TC-184 | A declared producer input origin names a campaign source or declared member and output artifact, matches the selected runtime role and exact origin, and rejects an undeclared dynamic input | Integration | P0 | FR-019-AC-10, VO-012, EN-006 | ✅ `tests/campaign.rs` origin context and binding cases passing |
| TC-185 | The MeasurementPlan schema accepts a missing or safe repository-relative JSON `execution_procedure` with `protected_apparatus` declared, refuses unsafe or non-JSON paths and a missing protected list, and leaves exact protected-path membership and source-byte checks to Quoin | Unit | P0 | FR-024-AC-10 | ✅ `tests/python_port/campaign_measurement_plan.rs` schema cases passing |
| TC-186 | A source-bound EA request parses from its retained JSON with identical typed fields; unknown fields, explicit elided defaults and an invalid protocol refuse before replay | Unit | P0 | FR-019-AC-12 | ✅ `tests/campaign.rs` retained-request accept/refuse cases passing |
| TC-187 | A Rust fixture copied beside a sibling fixture and named by absolute path reports its own resolved executable path and `argv[0]` as the executable path exactly as named in the request, re-executes itself through that path, and runs its sibling from the same directory, and a `#!` script named by absolute path sees that path as `$0`, each completing through the executor | Integration | P0 | FR-019-AC-13 | ✅ self-path, `argv[0]`, self re-execution, sibling execution and `#!` script `$0` cases passing |
| TC-188 | A relative margin moves the baseline value by the margin times its absolute value in the direction of improvement, including for negative baselines and baselines of different scale; an absent margin mode is absolute and an absolute rule does not emit the key; a margin mode without a margin, on a threshold, or with an unknown value is refused; and the schema's margin_mode enum equals the Rust wire names | Unit | P1 | FR-021-AC-12 | ✅ evaluation, validation, round-trip and wire-parity cases passing |
| TC-189 | The requirement-test source audit is told one trace grammar. Under `DocComment`, `/// Trace:` lines whose identifiers are requirement-family ids pass; an absent line yields `DocTraceMissing`, and an empty list, malformed entry, test-case id or placeholder yields `DocTraceIdInvalid`; block, `#[doc]`, multi-line, inner-doc, lowercase-key, aliased-import and serialized-name cases are covered; under either grammar the other grammar's form does not satisfy the audit, and import-shape findings appear only under `Attribute` | Static | P0 | FR-014-AC-6 | ✅ `tests/source_audit.rs` doc-comment and grammar-exclusivity cases passing |
| TC-190 | The library compiles (`--lib`) with default features disabled and with each capability feature (metadata-derived, all but `default` and `full`) enabled alone; the Makefile's list equals the metadata set and covers everything `full` enables; an empty derivation fails and `make -n` lists every feature | Compile | P0 | FR-014-AC-7 | ✅ `tests/feature_matrix.rs` passing; run in CI via `make rust-foundation-gate` -> `rust-tests`, no workflow change |
| TC-192 | The schema and the Rust DecisionRule accept an interval_level strictly between 0 and 1 and refuse it with eq, at or beyond 0 and 1, and non-numeric, with typed errors; an absent level is not emitted and a level round-trips; Interval and ConfidenceLevel validate bounds, order, level and method with typed errors and round-trip; the key sits inside decision_rule so its edit without a version bump is a definition-change finding; the schema property and eq prohibition are asserted | Unit | P1 | FR-021-AC-13 | ✅ validation, round-trip, schema and definition-change cases passing |
| TC-193 | A rule with interval_level holds at the lower bound for gt/ge and the upper bound for lt/le for threshold and baseline references, refuses a missing interval, a lower observed level, an estimate outside its interval, a rule without interval_level and non-finite inputs, accepts an equal or higher level, and the point-form holds refuses the rule with IntervalRequired | Unit | P1 | FR-021-AC-14 | ✅ unfavourable-bound evaluation and typed refusal cases passing |
| TC-194 | The module manifest exports its artifact types and both campaign types through the semantic contract: each export is a declared type with a `data_schema`, every exported schema is 2020-12, each artifact type's `data_schema` is the same file as its `frontmatter_schema_ref`, the artifact schemas use no draft-07 form and every `format` at any depth is `date-time` with the one documented pattern, and the content-rights scan admits exactly the manifest's schema files (an unlisted file and a nested path are refused) | Static | P1 | FR-003-AC-7 | ✅ `tests/semantic_exports.rs` passing |
| TC-195 | Each artifact schema is a valid schema that accepts its skeleton; the `review_by` pattern alone gives the verdicts the consumer gave the original `format` check on 40 measured values and agrees with a format-asserting validator over a generated calendar, time, offset, fraction and separator set, with the leap-second and space-separator differences documented | Unit | P1 | FR-003-AC-8 | ✅ `tests/semantic_exports.rs` and `tests/python_port/module.rs` passing |
| TC-196 | For every feature, the normal-dependency graph (default features off) contains none of `cap-std`, `clap`, `tar`, `zip`, `flate2`, `tempfile` (only `full` may), and `serde_json/arbitrary_precision` is on exactly for the features that imply `exact-numbers` (`evidence`, `semantics`, `evaluation`, `evaluation-reports`, `exact-numbers`, `full`); `manifest` and `source-audit` keep the dependencies their modules need | Static | P0 | FR-014-AC-8 | ✅ `tests/feature_matrix.rs` passing (`cargo tree` per feature); mutation-checked with banned crates added to `evidence` and `exact-numbers` added to and removed from a feature |
| TC-197 | The `default` feature set is empty: the default-feature `cargo tree` (normal edges, and feature edges) equals the `--no-default-features` one, contains none of `cap-std`, `clap`, `tar`, `zip`, `flate2`, `tempfile` and no `serde_json` (so no `serde_json/arbitrary_precision`), while `--features full` still resolves `clap` and `serde_json` | Static | P0 | FR-014-AC-9, FR-028-AC-3 | ✅ `tests/default_features.rs` passing; mutation-checked with a capability added to `default` |
| TC-201 | From a passing projection with one binding broken, the Rust evaluator and the retained provider agree on a failed outcome with that binding's code: any stage pair but one step forward in the MeasurementPlan schema's stage order, or an unknown stage, is `promotion_must_advance_one_stage`; an impact snapshot missing a change array or naming another profile or baseline is `impact_snapshot_incomplete`; an expected but absent exception, or a recorded one unowned, unjustified, without impact, expired or undated whether or not expected, is `owned_current_exception_required`; a review for another subject, analysis or artifact type, an invalid one or one without a path is `architecture_review_missing` or `code_review_missing`; each unbroken projection passes | Unit | P0 | FR-016-AC-5 | ✅ `tests/workflow_invariants_parity.rs` passing; mutation-checked by reordering two stages and dropping the architecture subject binding |
| TC-202 | `engineering-assurance onboard-report --summary` with stub Quire and Quoin reports a valid and an invalid artifact with its findings, caps findings at ten with a `+N more` line, labels this checkout's module version, reports `null` for an absent tool, omits the orientation sections, and exits 1 on an invalid artifact, 3 with a stated reason without Quire, and 0 when every artifact validates | Integration | P1 | FR-001-AC-9 | ✅ `tests/python_port/onboarding_script.rs` summary cases passing |
| TC-203 | The MeasurementPlan schema accepts each ground-truth kind at and below gate, requires `ground_truth_kind` at gate as the only finding, leaves it optional below gate, and refuses an unknown, case-variant or empty value at every stage | Unit | P1 | FR-024-AC-11 | ✅ `tests/python_port/module.rs` schema cases passing |
| TC-205 | Every TC id named by a requirement coverage row has a Test Case Summary row, matching rows whatever their cell padding, with no allowlist of exceptions; a synthetic coverage row naming an absent TC is detected | Static | P1 | NFR-005-AC-5 | ✅ `tests/python_port/module.rs` matrix-integrity case passing |
| TC-206 | Publishing bytes to an absent destination creates it with exactly those bytes, and no `.tmp-` sidecar remains in the directory | Unit | P0 | FR-027-AC-1 | ✅ `src/atomic_publish.rs` passing |
| TC-207 | Publishing over an identical existing file succeeds and leaves it untouched; over a different file, a directory or a symbolic link it returns the typed collision with the destination unchanged; a destination appearing before the commit step is handled the same way; the sidecar is removed in every case | Unit | P0 | FR-027-AC-2 | ✅ `src/atomic_publish.rs` passing |
| TC-211 | A publish into an absent destination reports `Written`; repeating it, or a destination appearing before the commit step with identical bytes, reports `AlreadyPresent`; differing bytes remain the typed collision | Unit | P0 | FR-027-AC-4 | ✅ `src/atomic_publish.rs` passing |
| TC-212 | A publish with the sidecar and parent-directory flushes succeeds on the written and the already-present path for a nested destination, with no sidecar left; the Windows path compiles under `cargo check --target x86_64-pc-windows-msvc` and is not executed here | Unit | P0 | FR-027-AC-5 | ✅ `src/atomic_publish.rs` passing; Windows compile-checked only |
| TC-213 | Eight concurrent writers of differing bytes yield exactly one `Written` and seven collisions with one writer's bytes intact; eight writers of identical bytes yield exactly one `Written` and no error | Unit | P0 | FR-027-AC-2 | ✅ `src/atomic_publish.rs` passing |
| TC-214 | A valid fictional Rust producer request run through the real binary prints one newline-terminated canonical result line (protocol, provenance, timing, process evidence, `completed` state with the exit-code observation on Linux), exits 0, and the line equals its RFC 8785 canonicalization | Integration | P0 | FR-029-AC-1 | ✅ `tests/producer_execute_cli.rs` passing (completed-state assertions run on Linux only; not executed on the macOS authoring host) |
| TC-215 | Unreadable, oversized-stdin, non-conforming, unknown-field and structurally invalid requests each produce the matching typed machine error and exit 2 with no producer launch | Integration | P0 | FR-029-AC-2 | ✅ `tests/producer_execute_cli.rs` passing (host-independent) |
| TC-216 | A foreign response binding is refused with reason `adapter_binding`, and an unavailable producer and a timed-out producer keep their own closed states, each at exit 0 | Integration | P0 | FR-029-AC-3 | ✅ `tests/producer_execute_cli.rs` passing (`timed_out` assertion runs on Linux only; not executed on the macOS authoring host) |
| TC-217 | `--schema request` and `--schema result` print the published schemas; the request schema accepts the real request and refuses a mutant per required field; the result schema accepts the real result line | Integration | P0 | FR-029-AC-4 | ✅ `tests/producer_execute_cli.rs` passing (live result-line schema check sees `containment_failure` off Linux; the `completed` and `failed` shapes are checked against the real serializer) |
| TC-218 | The installed binary runs a request from an unrelated directory with an empty environment | E2E | P0 | FR-029-AC-5 | ✅ `tests/producer_execute_cli.rs` passing (completed-state check is Linux only; off Linux the same run reports `containment_failure`) |
| TC-219 | A package with one fact of every kind and two adopter subjects reads back equal; bytes are deterministic and independent of input order | Unit | P0 | FR-030-AC-1 | ✅ `tests/integrator_package.rs` passing |
| TC-220 | Every fact state and open and attributed completed obligations survive; open obligations are listed apart; acceptance, approval and disposition members and an accepted obligation state are refused | Unit | P0 | FR-030-AC-2, FR-030-CON-1 | ✅ `tests/integrator_package.rs` passing |
| TC-221 | Unknown fields and variants, invalid identifiers, duplicate keys, extra whitespace, a trailing newline, unordered facts, non-JSON, non-object and oversize input are refused | Unit | P0 | FR-030-AC-3 | ✅ `tests/integrator_package.rs` passing |
| TC-222 | Protocol versions below, above, malformed and foreign are refused as downgraded, unsupported and unknown | Unit | P0 | FR-030-AC-4 | ✅ `tests/integrator_package.rs` passing |
| TC-223 | Each deleted fact or subject, duplicate identity, unsupported established fact, missing license value, blank text, unattributed completion and empty subject is refused | Unit | P0 | FR-030-AC-5 | ✅ `tests/integrator_package.rs` passing |
| TC-224 | The writer refuses a missing kind, stale count, other protocol and oversize text | Unit | P0 | FR-030-AC-6 | ✅ `tests/integrator_package.rs` passing |
| TC-225 | Publication creates the candidate and attempt path, reads back equal, leaves no sidecar; an invalid package or missing root writes nothing | Integration | P0 | FR-031-AC-1 | ✅ `tests/integrator_package.rs` passing |
| TC-226 | Identical retries report `AlreadyPresent` and leave one unchanged object | Integration | P0 | FR-031-AC-2 | ✅ `tests/integrator_package.rs` passing |
| TC-227 | A changed package for an existing attempt is a conflict, replaces nothing, marks the attempt, and the original retry is a conflict | Integration | P0 | FR-031-AC-3 | ✅ `tests/integrator_package.rs` passing |
| TC-228 | Eight non-identical concurrent writers yield at most one `Written`, only conflicts otherwise and a conflict for readers; eight identical writers yield one `Written` and no error | Integration | P0 | FR-031-AC-4 | ✅ `tests/integrator_package.rs` passing |
| TC-229 | An object under another attempt or candidate, a truncated object, a directory, a missing object and a symbolic link are refused | Integration | P0 | FR-031-AC-5 | ✅ `tests/integrator_package.rs` passing |
| TC-230 | Attempts of one candidate and one attempt id under two candidates are independent verified objects | Integration | P0 | FR-031-AC-6 | ✅ `tests/integrator_package.rs` passing |
| TC-231 | A downstream crate with only `integrator-package` compiles, publishes twice and reads back equal, and resolves no unrelated dependency | Integration | P0 | FR-030-AC-7, IT-007-SC-01, IT-007-SC-02 | ✅ `tests/integrator_package.rs` passing |
| TC-232 | Exact file entries and directory prefixes covering every changed path yield `complete` with no uncovered path | Unit | P0 | FR-032-AC-1 | ✅ `tests/scope_footprint.rs` passing |
| TC-233 | A path outside the scope, a textual-prefix sibling, and an out-of-scope path in a truncated footprint yield `false_scope` with sorted uncovered paths | Unit | P0 | FR-032-AC-2 | ✅ `tests/scope_footprint.rs` passing |
| TC-234 | A truncated footprint with no uncovered path and an unavailable footprint yield `incomplete`, never `complete` | Unit | P0 | FR-032-AC-3, FR-032-CON-1 | ✅ `tests/scope_footprint.rs` passing |
| TC-235 | Each malformed scope entry or changed path form, a duplicate entry and over-bound inputs are refused | Unit | P0 | FR-032-AC-4 | ✅ `tests/scope_footprint.rs` passing |
| TC-236 | Unused scope entries are listed sorted and leave the status unchanged | Unit | P0 | FR-032-AC-5 | ✅ `tests/scope_footprint.rs` passing |
| TC-237 | The real binary on a temporary git repository reports complete, false_scope with both rename paths, and incomplete for an unknown revision and a non-repository, each at exit 0 | Integration | P0 | FR-032-AC-6 | ✅ `tests/scope_footprint.rs` passing |
| TC-238 | Unreadable and non-conforming requests, an invalid scope entry and a revision beginning with `-` yield machine errors at exit 2 | Integration | P0 | FR-032-AC-7 | ✅ `tests/scope_footprint.rs` passing |
| TC-239 | `holds_on_interval_reporting` reports the lower bound and its value for gt/ge and the upper bound and its value for lt/le, for threshold, absolute-margin baseline (higher and lower is better) and relative-margin baseline references, when the rule holds and when it does not; it refuses a missing interval, a rule without interval_level and a missing baseline value as `holds_on_interval` does; and `holds_on_interval` returns the same verdict | Unit | P1 | FR-021-AC-15 | ✅ reported bound cases and shared refusals passing |
| TC-240 | The schema requires answer_families with a constant-predictor baseline and refuses its absence; accepts a non-empty list of unique non-empty strings; refuses an empty list, duplicate, empty-string and non-string entries; and refuses the field with another baseline or a threshold rule | Unit | P0 | FR-021-AC-16 | ✅ Passing |
| TC-241 | Adding, removing or changing an answer family, or adding or removing the list, with an unchanged definition_version yields one finding naming statistical_design.answer_families; reordering the same families yields none; with a changed definition_version it yields none | Unit | P1 | FR-021-AC-17 | ✅ Passing |
| TC-242 | The skeleton's constant-predictor guidance shows answer_families and states that families come from it, not the Population prose; the onboarding checklist states the required/refused rule and the producer-families-equal-list requirement | Unit | P1 | FR-021-AC-18 | ✅ Passing |
| TC-243 | AnswerFamilies accepts a non-empty set of distinct non-empty AnswerFamily strings and refuses empty, repeated and empty-string entries with distinct typed errors through construction and deserialization; order does not affect equality and entries serialize sorted; DefinitionMember::AnswerFamilies has the expected path; a measurement-only consumer reaches the types | Unit | P0 | FR-021-AC-19 | ✅ Passing |
| TC-245 | The skeleton's Interpretation section and the onboarding skill state that detail from prose estimator or decision_rule moved into typed fields must be carried into the body Interpretation section | Unit | P2 | FR-021-AC-20 | ✅ Passing |
| TC-246 | The MeasurementPlan schema accepts an absent `paired_run`, each relation, and `minimum_pairs` 1 and 5, and refuses an unknown relation, `minimum_pairs` 0, -1 and "2", `within_delta` without `delta`, `delta` with `equal`, extra members and a non-object value | Unit | P0 | FR-034-AC-1 | ✅ `tests/python_port/module.rs` passing |
| TC-244 | The MeasurementPlan schema accepts `cross_profile_comparison` true and false and its absence, and refuses a string, number, null, array or object value | Unit | P0 | FR-033-AC-1 | ✅ `tests/python_port/module.rs` passing |
| TC-247 | The MeasurementPlan schema accepts `statistical_design` with neither, either or both of `population_set` and `population_exempt`, and refuses empty, repeated, empty-string and non-string ids, an extra or missing member, a non-object set, and an empty, blank or non-string exemption | Unit | P0 | FR-035-AC-1 | ✅ `tests/python_port/module.rs` passing |
| TC-248 | Commit grades G, U, B, N, E, the codes X, Y, R and an unrecognized code, a G grade git did not accept, and a non-SSH signature header map to their statuses and reasons; the commit signature header names its format | Unit | P0 | FR-036-AC-9, FR-036-CON-1 | ✅ `tests/signature_verify.rs` passing |
| TC-249 | Each tag outcome (no signature block, non-SSH block, `git verify-tag` exit 0, delegated check success, failure, not started) maps to its status in grading order; a tag object splits at its signature header line | Unit | P0 | FR-036-AC-9, FR-036-CON-1 | ✅ `tests/signature_verify.rs` passing |
| TC-250 | A commit and an annotated tag signed by an allowed key are `signed_valid`; the commit carries the `%GF` fingerprint and the tag none | Integration | P0 | FR-036-AC-1, FR-036-CON-1 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-251 | An unsigned commit and an unsigned annotated tag are `unsigned` with no reason | Integration | P0 | FR-036-AC-2 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-252 | A commit and a tag signed by an unlisted key are `signed_untrusted`; a commit and a tag altered after signing are `signed_bad` | Integration | P0 | FR-036-AC-3 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-253 | A lightweight tag is `unsigned` with reason `lightweight_tag` | Integration | P0 | FR-036-AC-4 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-254 | An unknown revision, a directory that is not a repository and a missing git executable are `unverifiable` with reasons `revision_unresolved`, `repository_unavailable` and `git_not_started` at exit 0 | Integration | P0 | FR-036-AC-5 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-255 | A simulated timeout and an output over the capture bound map to reasons `timeout` and `output_exceeded` | Unit | P0 | FR-036-AC-6 | ✅ `src/signature_verify_host.rs` passing |
| TC-256 | An empty, 257-byte, option-like, whitespace or control-character revision yields `signature_verify_revision_invalid` at exit 2 and does not start git | Integration | P0 | FR-036-AC-7 | ✅ `tests/signature_verify.rs` and `tests/signature_verify_cli.rs` passing |
| TC-257 | An unreadable or non-conforming request, including one without `allowed_signers`, yields the matching machine error at exit 2 | Integration | P0 | FR-036-AC-8 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-258 | An OpenPGP-signed commit and an OpenPGP-signed annotated tag are `unverifiable` with reason `unsupported_signature_format` | Integration | P0 | FR-036-AC-10 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-259 | With `ssh-keygen` absent, a tag git rejected is `unverifiable` with reason `ssh_keygen_not_started`, while unsigned and non-SSH tags need no `ssh-keygen` | Integration | P0 | FR-036-AC-11 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-260 | A tag whose last signature header line is a prefix-only non-SSH header is `unverifiable` with `unsupported_signature_format`, even when the repository config names a `gpg` that reports a trusted good signature | Integration | P0 | FR-036-AC-10, FR-036-CON-1 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-261 | A commit with an SSH `gpgsig-sha256` header and a PGP `gpgsig` header is `unverifiable` with `unsupported_signature_format` and no fingerprint, under the same lying `gpg` | Integration | P0 | FR-036-AC-10, FR-036-CON-1 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-262 | A repository with `log.showSignature=true` still grades a valid commit `signed_valid` with its fingerprint | Integration | P0 | FR-036-AC-1 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-263 | Kind `tag` reports `lightweight_tag` only for a `refs/tags/` ref to a commit; a commit id, branch or `HEAD` is `revision_unresolved` | Integration | P0 | FR-036-AC-4 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-264 | A commit and an annotated tag validly signed by a key the file names are `signed_valid` with mapping `mapped` and the file's principal | Integration | P0 | FR-037-AC-1, FR-037-CON-1 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-265 | A good signature by an unnamed key on a commit and a tag is `signed_untrusted`, mapping `unmapped`, no principal, even when the committer email equals a principal | Integration | P0 | FR-037-AC-2, FR-037-CON-1 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-266 | A missing, directory, FIFO, over-1-MiB or non-UTF-8 allowed-signers file yields `signature_verify_allowed_signers_invalid` at exit 2 without starting git; a file of exactly 1 MiB is accepted | Integration | P0 | FR-037-AC-3 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-267 | A file whose only line is unparsable is `signed_untrusted`; with a valid line for the signer added it is `signed_valid` with that principal | Integration | P0 | FR-037-AC-4 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-268 | A commit and a tag signed by a key whose entry has an expired `valid-before` are `signed_untrusted`, mapping `unmapped` | Integration | P0 | FR-037-AC-9 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-269 | One trailer yields `present` with its value, two yield both in order, a trailer after a `---` line is read, and a trailer outside the trailer block is not read | Integration | P0 | FR-037-AC-5 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-270 | A message without the trailer is `missing` with no values; an empty trailer is `present` with an empty value | Integration | P0 | FR-037-AC-6, FR-037-CON-2 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-271 | A `signed_bad` and an `unsigned` commit carrying the trailer report its values and keep their status | Integration | P0 | FR-037-AC-7 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-272 | A signed tag's trailer is read from the tag message, not the tagged commit's, despite the signature block | Integration | P0 | FR-037-AC-8 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-273 | Only grade `G` accepted by git with a principal, or an accepted tag with a found principal, is `mapped`; `U` and a rejected-check tag are `unmapped`; others carry no signer | Unit | P0 | FR-037-AC-1, FR-037-AC-2, FR-037-CON-1 | ✅ `tests/signature_verify.rs` passing |
| TC-274 | Parsed trailer output yields `present` values in order (case-insensitive token, trimmed) or `missing`, with an empty value kept | Unit | P0 | FR-037-AC-5, FR-037-AC-6, FR-037-CON-2 | ✅ `tests/signature_verify.rs` passing |
| TC-275 | An object's message is everything after its header block, including after a multi-line signature header | Unit | P0 | FR-037-AC-8 | ✅ `tests/signature_verify.rs` passing |
| TC-276 | Trailer config in the verified repository (`trailer.<token>.key`, `core.commentChar`) neither injects a `Signed-off-meaning` value nor hides a real one | Integration | P0 | FR-037-CON-2 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-277 | With the temporary directory inside the verified repository, whose config maps `fake` to `Signed-off-meaning`, a commit whose only trailer is `fake: injected` reports `missing` | Integration | P0 | FR-037-CON-2 | ✅ `tests/signature_verify_cli.rs` passing |
| TC-278 | With a stand-in `gitsign` exiting 0 for the exact identity and issuer, a CMS-signed commit and annotated tag are `signed_valid` with `signature_kind` `gitsign`, `certificate` and a `request`-sourced mapped signer carrying the request's principal | Integration | P0 | FR-040-AC-1, FR-040-CON-1 | 🚧 |
| TC-279 | The stand-in receives `verify` or `verify-tag`, the identity and issuer as separate arguments, then `--` and the resolved commit id or bare tag name (for a bare and a `refs/tags/` revision), runs in the repository, and git is never asked to verify the object | Integration | P0 | FR-040-AC-2 | 🚧 |
| TC-280 | A stand-in that exits 1 for the exact flags and 0 for the wildcard flags yields `signed_untrusted`, `unmapped`, no `certificate`, with `mismatch` `issuer`, `identity` or `identity_and_issuer` in at most four invocations | Integration | P0 | FR-040-AC-3 | 🚧 |
| TC-281 | A stand-in exiting 1 for every call, or 2 or another status at any step, yields `unverifiable` with `gitsign_verification_failed`, never `signed_bad` or `signed_untrusted` | Integration | P0 | FR-040-AC-4 | 🚧 |
| TC-282 | A CMS-signed commit and tag in a request without `gitsign` are `unverifiable` with `gitsign_identity_missing` and the stand-in is never started | Integration | P0 | FR-040-AC-5 | 🚧 |
| TC-283 | With no `gitsign` on `PATH`, a CMS-signed commit and tag with a supplied identity are `unverifiable` with `gitsign_not_started` at exit 0 | Integration | P0 | FR-040-AC-6 | 🚧 |
| TC-284 | A stand-in writing more than 8 MiB yields `unverifiable` with `output_exceeded` | Integration | P0 | FR-040-AC-7 | 🚧 |
| TC-285 | Unsigned and OpenPGP-signed commits and tags keep `unsigned` (no `signature_kind`) and `unsupported_signature_format`, with or without `gitsign` in the request, and the stand-in is never started | Integration | P0 | FR-040-AC-8 | 🚧 |
| TC-286 | A `gitsign` member with a missing field, an empty or oversized value, a leading `-` identity or issuer, a control character, an unknown member or a non-object yields `signature_verify_request_invalid` at exit 2 and does not start git | Integration | P0 | FR-040-AC-11 | 🚧 |
| TC-287 | An SSH-signed commit and tag in a request that carries `gitsign` keep their FR-036 and FR-037 verdict, with `signature_kind` `ssh` and `signer.source` `allowed_signers`, and the stand-in is never started | Integration | P0 | FR-040-AC-12 | 🚧 |
| TC-288 | A gitsign-signed commit and tag with a `Signed-off-meaning:` trailer report it for `signed_valid` and for `unverifiable` verdicts | Integration | P0 | FR-040-AC-13 | 🚧 |
| TC-289 | A `gitsign.*` key in local configuration, an `include.path` file, `config.worktree` or a newline injected through another key's value (also with a long s or Kelvin sign) yields `unverifiable` with `gitsign_repository_config` without starting the stand-in; an unrelated section proceeds; git is asked the unscoped `.*` query | Integration | P0 | FR-040-AC-17, FR-040-CON-1 | 🚧 |
| TC-290 | The stand-in's environment holds exactly `PATH`, `HOME`, `GIT_CONFIG_NOSYSTEM` and `GIT_CONFIG_GLOBAL` although the parent sets `SIGSTORE_ROOT_FILE`, `GITSIGN_FULCIO_ROOT`, `TUF_ROOT` and `GIT_DIR` | Integration | P0 | FR-040-AC-18 | 🚧 |
| TC-291 | A lightweight tag is `unsigned` with `lightweight_tag`; a tag object id, branch, unknown ref, `^{tag}` revision or `refs/tags/--help` tag name is `unverifiable` with `revision_unresolved`, without starting the stand-in | Integration | P0 | FR-040-AC-19 | 🚧 |
| TC-292 | Against real `gitsign` and the network, a public gitsign-signed commit and tag are `signed_valid` for their true identity and issuer and `signed_untrusted` with `mismatch` `issuer` for a wrong issuer | Integration | P2 | FR-040-AC-15 | 🚧 |
| TC-293 | The gitsign classifier maps exit 0; 1 then wildcard 1; 1 then wildcard 0 with each issuer and identity probe result; exit 2; other statuses; a signal; not started; timeout; cut short; no identity; and a repository `gitsign.*` key to their statuses, mismatches and reasons, asking for no more than four probes and none when it must not run | Unit | P0 | FR-040-AC-10, FR-040-CON-1 | 🚧 |
| TC-294 | A CMS `gpgsig` header (including trailing characters on its line) is CMS, a commit mixing CMS and SSH headers is unsupported, and a tag splits at a CMS header | Unit | P0 | FR-040-AC-9 | 🚧 |
| TC-295 | A simulated timeout, oversized output and unstartable `gitsign` and a signal-ended run are observed as the timed-out, cut-short, not-started and signaled runs | Unit | P0 | FR-040-AC-7 | 🚧 |
| TC-296 | The `gitsign` identity accepts values up to 256 bytes and refuses empty, oversized, control-character and leading-`-` identity and issuer values | Unit | P0 | FR-040-AC-11 | 🚧 |
| TC-297 | A relative `PATH` entry holding a `gitsign` or `git` inside the verified repository never supplies it: the absolute programs run, and with none the verdict is `gitsign_not_started` | Integration | P0 | FR-040-CON-1 | 🚧 |
| TC-298 | Config output is read line by line for a first token starting `gitsign.`, ASCII case-insensitively, or holding a non-ASCII byte before its first `.` (long s, Kelvin sign), and a `gitsign.` text elsewhere in a line is not a key | Unit | P0 | FR-040-AC-17 | 🚧 |
| TC-299 | On the SSH path, a relative `PATH` entry holding a `git` and an `ssh-keygen` inside the verified repository never supplies either, including the `ssh-keygen` git starts; with only that entry the command reports `git_not_started` | Integration | P0 | FR-036-AC-1, FR-036-CON-1 | 🚧 |
| TC-300 | The Rust fixture, run through `producer-execute` with the Syft request, completes and its CycloneDX artifact holds the declared package count | Integration | P0 | FR-038-AC-1, FR-039-AC-1 | 🚧 `tests/syft_producer_cli.rs`; tagged and ignored by default, NOT yet executed against a real Syft; run by `make syft-integration` |
| TC-301 | The npm fixture's artifact holds the declared package count with npm purls | Integration | P0 | FR-039-AC-2 | 🚧 `tests/syft_producer_cli.rs`; tagged and ignored by default, NOT yet executed against a real Syft; run by `make syft-integration` |
| TC-302 | The Python fixture's artifact holds the declared package count with PyPI purls | Integration | P0 | FR-039-AC-3 | 🚧 `tests/syft_producer_cli.rs`; tagged and ignored by default, NOT yet executed against a real Syft; run by `make syft-integration` |
| TC-303 | The container archive, scanned through an `oci-archive:` selector, holds the declared package count as library components named for the declared packages, with no operating-system or file component | Integration | P0 | FR-038-AC-2, FR-039-AC-4 | 🚧 `tests/syft_producer_cli.rs`; tagged and ignored by default, NOT yet executed against a real Syft; run by `make syft-integration` |
| TC-304 | A `.syft.yaml` and `.syft/config.yaml` planted inside `scan/` and an earlier SBOM and cache under `out/` do not change the produced components | Integration | P0 | FR-038-AC-7 | 🚧 `tests/syft_producer_cli.rs`; tagged and ignored by default, NOT yet executed against a real Syft; run by `make syft-integration` |
| TC-305 | A missing Syft executable yields `unavailable` and no artifact | Integration | P0 | FR-038-AC-3, FR-039-AC-6 | ✅ `tests/syft_producer_cli.rs` passing (Linux) |
| TC-306 | A non-zero exit yields `failed` and a one-second timeout yields `timed_out`, each with no artifact | Integration | P0 | FR-038-AC-4, FR-039-AC-6 | ✅ `tests/syft_producer_cli.rs` passing (Linux) |
| TC-307 | A stale file at the SBOM path is removed and absent from a failed result | Integration | P0 | FR-038-AC-6 | ✅ `tests/syft_producer_cli.rs` passing (Linux) |
| TC-308 | The Syft request has one `-o cyclonedx-json=` pair, the defined arguments, environment, inputs, stdin, outputs and containment, and validates against the published request schema | Unit | P0 | FR-038-AC-5, NFR-006-AC-2 | ✅ `tests/syft_producer_cli.rs` passing |
| TC-309 | `make syft-integration` with `EA_TEST_SYFT` unset, relative or not executable exits non-zero with a message naming the variable | Unit | P0 | FR-038-AC-8, FR-038-CON-1, FR-039-AC-5 | ✅ `tests/syft_producer_cli.rs` passing |
| TC-310 | The approval decision is exercised for every combination of signature status, both bindings (or an unreadable message) and meaning presence without starting a process, and `approved` is returned for exactly one combination, whether or not a signing time is supplied | Property | P0 | FR-041-AC-9, FR-041-AC-19, FR-041-CON-1 | 🚧 |
| TC-311 | The signature status decides before the record binding and the binding before the meaning; the approved result carries the principal, the meaning values in order and the signature kind | Unit | P0 | FR-041-AC-9 | 🚧 |
| TC-312 | A validly signed tag with an unreadable message decides `unverifiable` with reason `trailer_unreadable`, and an unverifiable verdict's reason is carried unchanged | Unit | P0 | FR-041-AC-15, FR-041-AC-6 | 🚧 |
| TC-313 | A record trailer is `missing`, `match`, `mismatch` (byte-exact, no case folding or trimming) or `repeated` (any values, one equal included) | Unit | P0 | FR-041-AC-3, FR-041-AC-2 | 🚧 |
| TC-314 | Trailer values are read from `git interpret-trailers --parse` output by an ASCII case-insensitive token, trimmed of ASCII whitespace only | Unit | P0 | FR-041-AC-3 | 🚧 |
| TC-315 | A record id or digest is non-empty, at most 256 bytes, free of control characters and not option-like | Unit | P0 | FR-041-AC-10 | 🚧 |
| TC-316 | A mapped, validly signed annotated tag whose message binds the record and carries `Signed-off-meaning` values is `approved` with `approver`, `meaning` in order, `signature_kind` `ssh` and the tag name, for a bare and a `refs/tags/` name; the result is one JSON line at exit 0 | Integration | P0 | FR-041-AC-1, FR-041-AC-11, FR-041-CON-1 | 🚧 |
| TC-317 | A validly signed tag whose trailers name another record yields `wrong_record` with the `signature` verdict that `signature-verify` yields for the same tag | Integration | P0 | FR-041-AC-11 | 🚧 |
| TC-318 | A digest differing by one character is `wrong_record` with `binding.record_digest` `mismatch` and no approval members; a tag edited after signing is `signed_bad` | Integration | P0 | FR-041-AC-2 | 🚧 |
| TC-319 | An id mismatch, a missing trailer and a repeated trailer (equal values, one wrong value, case-variant keys) each yield `wrong_record` with the binding `mismatch`, `missing` or `repeated` | Integration | P0 | FR-041-AC-3 | 🚧 |
| TC-320 | A good signature by a key the allowed-signers file does not name is `untrusted_signer` with no approver and an unmapped `signed_untrusted` signature | Integration | P0 | FR-041-AC-4 | 🚧 |
| TC-321 | A bound tag without `Signed-off-meaning` is `meaning_missing` with no meaning; an empty-valued trailer is `approved` with the empty value | Integration | P0 | FR-041-AC-5 | 🚧 |
| TC-322 | A tag edited after signing, an unsigned annotated tag, a lightweight tag and a name resolving to no tag, a branch or a commit are `signed_bad`, `unsigned`, `unsigned` with `lightweight_tag` and `unverifiable`, never `approved` | Integration | P0 | FR-041-AC-6 | 🚧 |
| TC-323 | Record trailers in the tagged commit's message do not bind (`missing`, `wrong_record`); a branch naming a signed commit or a signed merge commit is `unverifiable` with `revision_unresolved` | Integration | P0 | FR-041-AC-7 | 🚧 |
| TC-324 | A tag message of only trailer lines binds nothing (`missing` for both, `wrong_record`); the same trailers after a subject paragraph are `match` | Integration | P0 | FR-041-AC-13 | 🚧 |
| TC-325 | The result echoes the request's `allowed_signers` for every status, a key mapped by the caller's own file names that file's principal, and another file naming a different principal for the same key yields a different `approver` under a different `trust` | Integration | P0 | FR-041-AC-14 | 🚧 |
| TC-326 | With git unable to read the tag message, a validly signed tag is `unverifiable` with reason `trailer_unreadable` and no binding | Integration | P0 | FR-041-AC-15 | 🚧 |
| TC-327 | Unknown members, a wrong protocol, a bad record id or digest, relative paths, an option-like or spaced tag and an invalid `gitsign` member yield `approval_verify_request_invalid`; unreadable JSON and an invalid allowed-signers file yield their codes, all at exit 2 without starting git | Integration | P0 | FR-041-AC-10 | 🚧 |
| TC-328 | With a stand-in `gitsign` accepting the identity and issuer, a CMS-signed tag with matching trailers is `approved` with `signature_kind` `gitsign`, the request's principal as `approver` and the `gitsign` trust echoed | Integration | P0 | FR-041-AC-8 | 🚧 |
| TC-329 | A stand-in rejecting the exact identity yields `untrusted_signer`; a CMS-signed tag with no `gitsign` member yields `unverifiable` with `gitsign_identity_missing` | Integration | P0 | FR-041-AC-8 | 🚧 |
| TC-330 | A validly signed tag whose message holds a NUL byte or git's trailer output is not valid UTF-8 is `unverifiable` with reason `trailer_unreadable`, not `approved`, even when the truncated or lossily decoded value equals the request | Integration | P0 | FR-041-AC-15, FR-041-CON-1 | 🚧 |
| TC-331 | The tagger line gives a UTC instant from its epoch whatever the zone (epoch 0, a leap day and the last accepted instant included), and a malformed epoch, zone, extra field or missing email gives none | Unit | P0 | FR-041-AC-16, FR-041-AC-17 | 🚧 |
| TC-332 | Only the single tagger header counts: a tagger-shaped message line, two tagger headers, non-UTF-8 and empty objects give none | Unit | P0 | FR-041-AC-17, FR-041-CON-3 | 🚧 |
| TC-333 | The pure decision gives an approval the time and source, or `signed_at_missing`, and a non-approved decision carries no approval members | Unit | P0 | FR-041-AC-16, FR-041-AC-18, FR-041-AC-19 | 🚧 |
| TC-334 | An SSH approval created with `+0530` and `-0800` tagger dates reports the person, meaning and the UTC `signed_at` with source `tagger_date`, and `signature-verify` output for the same tag has no time member | Integration | P0 | FR-041-AC-16, FR-041-AC-21 | 🚧 |
| TC-335 | `wrong_record`, `untrusted_signer` and `meaning_missing` results carry no signing-time member | Integration | P0 | FR-041-AC-19 | 🚧 |
| TC-336 | A stand-in gitsign approval reports its tagger date as `signed_at` with source `tagger_date`, and a rejected identity is `untrusted_signer` without a time | Integration | P0 | FR-041-AC-20 | 🚧 |
| TC-337 | An approval whose tag has no tagger line or an unparseable one is still `approved` with `signed_at_missing` and neither `signed_at` nor `signed_at_source` | Integration | P0 | FR-041-AC-18, FR-041-CON-3 | 🚧 |
| TC-338 | The composed tag message is a subject paragraph, a blank line and the three trailers in order, and the trailer reader reads the record id, digest and meaning back exactly | Unit | P0 | FR-042-AC-3, FR-042-AC-12 | 🚧 |
| TC-339 | A meaning is non-empty, at most 256 bytes, free of control characters and of leading or trailing ASCII whitespace; a record id or digest that a trailer would not read back unchanged is refused | Unit | P0 | FR-042-AC-4, FR-042-AC-12 | 🚧 |
| TC-340 | The signing decision refuses every combination of `gpg.format`, `user.signingkey`, `gpg.ssh.defaultKeyCommand` and `gpg.x509.program` except `ssh` with a key or key command and `x509` with a program, and names the setting; only a tag with an SSH or X.509 signature block counts as signed | Property | P0 | FR-042-AC-6, FR-042-AC-11, FR-042-AC-12 | 🚧 |
| TC-341 | With an SSH signing configuration, the created tag verifies as `approved` under `approval-verify` with the approver, meaning and `ssh` kind, the requested digest verbatim, and a message of subject then the three trailers; one `created` line at exit 0 | Integration | P0 | FR-042-AC-1, FR-042-AC-3 | 🚧 |
| TC-342 | With `gpg.format=x509` and a stand-in signing program, the created tag verifies as `approved` with `signature_kind` `gitsign` under the stand-in `gitsign`, and git started the stand-in signing program | Integration | P0 | FR-042-AC-2 | 🚧 |
| TC-343 | A missing, empty, padded, control-character or over-long meaning is `approval_tag_request_invalid` and writes no tag | Integration | P0 | FR-042-AC-4 | 🚧 |
| TC-344 | The tag is signed by the key the configuration names and by no other, a configured remote receives nothing, the configuration files are unchanged and no other ref changes | Integration | P0 | FR-042-AC-5, FR-042-CON-1 | 🚧 |
| TC-345 | With `gpg.format` unset, `openpgp` or `ssh` without a key or `x509` without a program, the command is `approval_tag_signing_unconfigured`, names `gpg.format` or `user.signingkey`, exits 2 and writes no tag | Integration | P0 | FR-042-AC-6 | 🚧 |
| TC-346 | A missing, non-regular, oversized, non-JSON, non-object, schema 1, other-type, other-id or approval-policy-less record file is `approval_tag_record_unresolved` and writes no tag | Integration | P0 | FR-042-AC-7 | 🚧 |
| TC-347 | An invalid, `refs/`-prefixed, revision-resolving (`HEAD`, `@`) or existing tag name and an unresolvable revision are refused with their codes; an existing tag keeps its object | Integration | P0 | FR-042-AC-8 | 🚧 |
| TC-348 | Unknown members, a wrong protocol, relative paths, an option-like or spaced tag or revision and a bad record id or digest are `approval_tag_request_invalid`; unreadable JSON is `approval_tag_request_unreadable`; none starts git | Integration | P0 | FR-042-AC-9 | 🚧 |
| TC-349 | A relative `PATH` entry holding a `git` or a signing program in the repository is never run, and with only relative entries the command is `approval_tag_git_unavailable` | Integration | P0 | FR-042-AC-10 | 🚧 |
| TC-350 | A signing program that exits nonzero, a missing SSH key file and a locked SSH key with no agent are `approval_tag_create_failed` at exit 2 and leave no tag | Integration | P0 | FR-042-AC-11 | 🚧 |
| TC-351 | The trail order is by claimed time ascending, then absent times, then tag name in byte order, whatever the input order; a tag names the record when any `Approved-Record-Id` value equals the id byte for byte (repeated, different and case-variant values included) | Unit | P0 | FR-043-AC-12, FR-043-AC-5 | 🚧 |
| TC-352 | An approval and a later revocation meaning of one record are both listed `approved`, oldest first by claimed time, even when created or named in the opposite order | Integration | P0 | FR-043-AC-1, FR-043-CON-3 | 🚧 |
| TC-353 | A bad-signature tag, an unmapped-signer tag, a different-digest tag and a tag without a meaning, all naming the record, are listed with `signed_bad`, `untrusted_signer`, `wrong_record` and `meaning_missing` | Integration | P0 | FR-043-AC-2, FR-043-CON-3 | 🚧 |
| TC-354 | Tags naming a different record, lightweight tags and annotated tags without the id trailer are not listed | Integration | P0 | FR-043-AC-3 | 🚧 |
| TC-355 | Each entry equals the `approval-verify` result for the same tag less the request echo, plus `claimed_at` and `object` | Integration | P0 | FR-043-AC-4, FR-043-CON-1 | 🚧 |
| TC-356 | Entries without a readable tagger date follow timed entries ordered by tag name, and `claimed_at` is reported for every status | Integration | P0 | FR-043-AC-5 | 🚧 |
| TC-357 | More than 10000 refs under `refs/tags` yields `truncated` true with `tag_limit` and `considered` 10000; a small trail yields `truncated` false | Integration | P0 | FR-043-AC-6 | 🚧 |
| TC-358 | A tag named `--help`, a revision-like name and a name with quote, dollar and semicolon characters are listed correctly and nothing runs | Integration | P0 | FR-043-AC-7 | 🚧 |
| TC-359 | A validly signed tag whose message holds a NUL byte is listed in `unreadable` with reason `trailer_unreadable`, is not an entry, and the result is not `complete` | Integration | P0 | FR-043-AC-8 | 🚧 |
| TC-360 | `--format table` prints a header, one row per entry in trail order, one row per unreadable tag and the final count line at exit 0 | Integration | P0 | FR-043-AC-9 | 🚧 |
| TC-361 | Invalid requests, an invalid allowed-signers file and a path that is not a git repository yield their `approval_trail_*` codes at exit 2 | Integration | P0 | FR-043-AC-10 | 🚧 |
| TC-362 | With a stand-in `gitsign`, a CMS-signed tag naming the record is listed `approved` with `signature_kind` `gitsign`, and a rejected identity is listed `untrusted_signer` | Integration | P0 | FR-043-AC-11 | 🚧 |
| TC-363 | An annotated tag whose ref name holds a non-UTF-8 byte is listed in `unreadable` with an escaped name, `tag_escaped`, the raw name in `tag_hex` and reason `tag_name_not_utf8` | Integration | P0 | FR-043-AC-14 | 🚧 |
| TC-364 | A 9 MB tag naming the record with a `revoked` meaning is listed in `unreadable` with reason `output_exceeded`, as `approval-verify` reports it `unverifiable` | Integration | P0 | FR-043-AC-15 | 🚧 |
| TC-365 | With the total time expired the trail has no entries, `truncated` true, `time_limit`, `considered` counting only the refs reached, and `complete` false | Unit | P0 | FR-043-AC-16 | 🚧 |
| TC-366 | Every entry carries the verified tag object id, equal for two ref names of one object | Integration | P0 | FR-043-AC-17 | 🚧 |
| TC-367 | The table replaces escape, C1, bidirectional and zero-width characters in a tag name or meaning | Integration | P0 | FR-043-AC-18 | 🚧 |
| TC-368 | A pandoc stand-in request over the fixture Markdown completes with one `document` artifact | Integration | P0 | FR-045-AC-1 | ✅ |
| TC-369 | A pandoc request writing `typ/document.typ` then a typst request compiling it, over one capability root, each complete with one `document` artifact | Integration | P0 | FR-045-AC-2 | ✅ |
| TC-370 | A missing pandoc or typst executable yields `unavailable`, no artifact and the echoed producer name and executable; a directory yields `refused` | Integration | P0 | FR-045-AC-3 | ✅ |
| TC-371 | A converter exiting 7 with a stderr message yields `failed` with the exit code and the stderr bytes retained | Integration | P0 | FR-045-AC-4 | ✅ |
| TC-372 | A converter exiting 0 without writing the required output yields `failed` and no artifact | Integration | P0 | FR-045-AC-4 | ✅ |
| TC-373 | The converter requests pass the published request schema and carry the defined caller, environment, inputs, stdin, outputs, containment, cancellation and arguments | Unit | P0 | FR-045-AC-5 | ✅ |
| TC-374 | Real pandoc converts the fixture to a DOCX zip whose `word/document.xml` holds the fixture headings and code text | Integration | P0 | FR-045-AC-6 | 🚧 |
| TC-375 | Real pandoc and typst convert the fixture to a PDF starting with `%PDF` | Integration | P0 | FR-045-AC-6 | 🚧 |
| TC-376 | `make converter-integration` with `EA_TEST_PANDOC` or `EA_TEST_TYPST` unset, relative or not executable exits non-zero with a message naming the variable | Unit | P0 | FR-045-CON-1 | ✅ |
| TC-377 | The licence-bearing npm fixture, run through `producer-execute` with the Syft request, yields the declared package count with the licence field and text the expected-result file names per purl, and no `licenses` on the unlicensed entry | Integration | P0 | FR-039-AC-8 | ✅ `tests/syft_producer_cli.rs` passing against Syft 1.57.1; ignored by default, run by `make syft-integration` |
| TC-378 | The Baseline skeleton validates against the Baseline schema and its items carry each of the three kinds | Unit | P0 | FR-046-AC-1 | ✅ `tests/semantic_exports.rs` passing |
| TC-379 | The Baseline schema refuses an unknown member, an empty `items`, an identical repeated item, an absolute path, a path with `..`, `.`, an empty segment, a glob, a control character or a leading-space segment, and an unknown `kind`; a repeated `path` with a different `kind` is the resolution's refusal | Unit | P0 | FR-046-AC-2 | ✅ `tests/semantic_exports.rs` passing |

## Option Permutation Matrix

| Test Case | Install Source | Host | Expected Behavior |
|-----------|----------------|------|-------------------|
| TC-010 | local | Claude Code | Resolves canonical onboarding skill |
| TC-010 | local | Codex | Resolves canonical onboarding skill |
| TC-010 | local | opencode | Resolves canonical onboarding skill |
| TC-010 | local | GitHub Copilot | Resolves canonical onboarding skill |
| TC-010 | repository | Claude Code | Resolves canonical onboarding skill |
| TC-010 | repository | Codex | Resolves canonical onboarding skill |
| TC-010 | repository | opencode | Resolves canonical onboarding skill |
| TC-010 | repository | GitHub Copilot | Resolves canonical onboarding skill |

## Agent Evaluation Permutation Matrix

The **declared qualification population** is Claude Code. Codex, opencode and
GitHub Copilot remain supported hosts that Engineering Assurance can evaluate,
but no release claims retained evidence for them, so the gate does not demand
cells it would have to invent.

| Scenario Class | Required Variants | Declared Hosts | Required Cells |
|----------------|-------------------|----------------|----------------|
| Existing repository | applicable valid profile | 1 | 1 |
| No applicable profile | bounded no-profile decision | 1 | 1 |
| Producer failure | malformed output; unavailable executable | 1 | 2 |
| Interruption | interrupt then resume | 1 | 2 |
| Human terminal decision | explicit acceptance; explicit rejection | 1 | 2 |

The aggregate gate therefore requires all 7 of 7 declared host-scenario cells
with complete evaluation envelopes. Completeness is unchanged — every declared
host must retain every scenario; only the declared population is smaller.

Narrowing the population makes one aggregation class unreachable:
`workflow-version-mismatch` compares governing workflow identity across hosts
within one scenario, and with a single declared host there is exactly one cell
per scenario to compare. The check and its failure variant are retained, and
become reachable again the moment a second host is declared.

## Rust Port Permutation Matrix

| Test Case | Axis | Variants | Expected |
|---|---|---|---|
| TC-098, TC-099 | Protocol/result | supported v1; unknown version; malformed input; escaping root; unavailable or invalid host | one versioned result for success; every adverse case fails before a write or downstream action |
| TC-100, TC-102, TC-103 | Semantic state | success; unavailable; not-computed; not-applicable; failed; inconclusive; malformed; stale; tampered; unreadable; arbitrary-precision integer; NaN/infinity/overflow-to-non-finite; blank and invalid-character version tokens; other version tokens | success on the shared supported domain; each non-success outcome and typed refusal reason remains distinct; non-finite numeric inputs and blank or invalid-character version tokens are refused, and any other version token is accepted |
| TC-107 | ix-flow lifecycle host | new run; pristine unbound recovery; interrupted resume; missing/accept/reject/repeated/opposite choice; invalid phase; binding drift; intact/broken chain; absent/malformed/oversized/timed-out host; automatic-gate evidence; hostile next-action text | preserve ix-flow ownership, exact binding, verified event integrity, one attributed decision, bounded typed output, pre-mutation history on refusal, and status-reconciled state after an indeterminate mutation; pass no gate override and execute no returned command text |
| TC-109, TC-110 | Remaining host integration | supported request/result; absent host; malformed response; invalid transition; incomplete scenario | preserve host ownership and fail unsupported inputs explicitly |
| TC-113 | Cutover | old only; additive parity; direct invocation updated; parity failure | delete only after local parity |
| TC-100 | Pure capability | semantic validation; bounded projection; fixture generation; canonical identity | each ADR-002 row has an independently reported old/new corpus and adverse-case result |
| TC-111 | Qualification capability | package members; rights; manifest; npm stage/clean; integration; publication refusal | each gate has one retained passing case and every named refusal case; cleanup deletes nothing unless every present staged byte corresponds |
| TC-120 | Package membership | exact match; empty expected/actual; extra; missing; duplicate expected/actual; absolute, backslash, trailing-slash, empty, dot, parent, control, non-normalized, and overlong path; 65,536 and 65,537 entries; permuted inputs | exact safe membership is accepted; invalid policy or resource limits refuse without a result; every observed mismatch withholds with deterministic typed findings and no unsafe-path disclosure |
| TC-121 | Module manifest | retained valid bundle; malformed/duplicate/merge-key manifest; invalid or mismatching authoritative schema; identity drift; duplicate artifact; unregistered edge; missing/duplicate/extra resource; unsafe schema reference; malformed/invalid artifact schema; missing/malformed/duplicate/merge-key/schema-invalid skeleton frontmatter; optional or absent required locator heading; exact/over document, artifact-count, and combined-byte ceilings; permuted resources | valid input is accepted; ceiling violations refuse without a result; every semantic/input defect withholds through deterministic typed findings without source-byte or unsafe-reference disclosure |
| TC-106 | Invariant request | eleven canonical names; ordered multi-name request; unknown name; malformed Rust request; expired/current exception at one explicit instant | exact ordered Rust/reference parity for the valid shared domain or a typed pre-outcome refusal |
| TC-105 | Onboarding request | retained inventory/status cases; absent Quire; malformed, duplicate-key, or merge-key frontmatter; duplicate artifact; unsupported type; absolute, parent, symlink, and existing targets; valid/invalid staged artifact | exact Rust/reference inventory and decision parity on the supported domain, explicit fail-closed YAML identity handling, equivalent parsed frontmatter plus identical Markdown body for valid publication, or a typed refusal with no published bytes |

## Constraint Boundary Tests

| Constraint | Boundary Type | Test Value | Test Case | Expected |
|------------|---------------|------------|-----------|----------|
| FR-002-CON-1 | Min | All 4 declared hosts present | TC-041 | Pass |
| FR-002-CON-1 | Max | All 4 declared hosts present once | TC-041 | Pass |
| FR-002-CON-1 | Below Min | One declared host missing | TC-041 | Fail |
| FR-002-CON-1 | Above Max | Extra or duplicate host manifest | TC-041 | Fail |
| FR-002-CON-2 | Min | Discovery metadata plus one canonical target | TC-042 | Pass |
| FR-002-CON-2 | Above Max | Embedded behavioral section or workflow copy | TC-042 | Fail |
| FR-007-CON-1 | Min | Four compatible pilot names | TC-043 | Pass |
| FR-007-CON-1 | Below Min | One promoted pilot name missing | TC-043 | Fail |
| FR-007-CON-1 | Above Max | Undeclared compatibility alias added | TC-043 | Fail |

## State Transition Coverage

| Workflow Condition | Transition or Failure | Test Case | Expected |
|--------------------|-----------------------|-----------|----------|
| Persisted non-terminal run | Interrupted to resumed | TC-026 | Completed phase retained |
| Decision-ready run without choice | Remains decision-ready | TC-029 | Non-terminal |
| Decision-ready run with rejection | Decision-ready to rejection terminal | TC-028 | Rejected exactly once |
| Terminal transition configured automatic | Gate override attempt | TC-030 | Fails closed |
| New ix-flow run interrupted before binding | Pristine unbound to bound resume | TC-107 | Add exactly one matching binding, then resume |
| Decision-ready run interrupted after gate defer or acknowledgement | Retry same explicit choice | TC-107 | Complete exactly one attributed terminal decision |
| Terminal run receives opposite choice | Accepted/rejected conflict | TC-107 | Refuse without changing history |
| Legacy path only | Additive Rust path enabled | TC-113 | Both remain available; no completion claim |
| Old and Rust paths agree | Direct invocation updated | TC-113 | Rust path becomes the repository-owned invocation |
| Direct invocation and parity checks pass | Legacy path removed | TC-113 | Removal allowed once |

## Edge Cases

| ID | Description | Related Req | Test Case | Risk if Untested |
|----|-------------|-------------|-----------|------------------|
| EC-001 | Valid applicable profile already exists | FR-001 | TC-004 | Duplicate or conflicting profile |
| EC-002 | Decision boundary justifies no profile | FR-001 | TC-005 | Generic assurance posture is invented |
| EC-003 | Canonical target is absent or escapes package | FR-002 | TC-013 | Host discovers stale or unsafe content |
| EC-004 | Package has an unexpected or missing member | FR-003 | TC-018 | Rights or install contract silently changes |
| EC-005 | Producer output is malformed | FR-004 | TC-024 | Invalid output counted as evidence |
| EC-006 | Selected producer executable is missing | FR-004 | TC-021 | Tooling failure becomes missing evidence |
| EC-007 | Agent stops after a persisted phase | FR-005 | TC-026 | Repeated work or lost decision context |
| EC-008 | Required evaluation executable is absent | FR-006 | TC-033 | Incomplete evaluation appears passing |
| EC-009 | Existing applicable artifacts conflict or are malformed | FR-001 | TC-044 | Onboarding silently selects or overwrites an artifact |
| EC-010 | Artifact target or manifest reference escapes its selected root | FR-001, FR-003 | TC-018, TC-045 | Installation or onboarding writes/loads unowned content |
| EC-011 | Run id is reused for a different repository or workflow | FR-005 | TC-048 | One run contaminates another decision boundary |
| EC-012 | Valid output lacks a governing version | FR-004 | TC-024 | Unreproducible evidence is admitted as observed |
| EC-014 | ix-flow or cli-agent-evals cannot load a structured external provider | FR-016, FR-017 | TC-109 | JavaScript remains an undeclared permanent dependency |
| EC-018 | A provider response serializes an absent optional field as a null the consuming host refuses | FR-017 | TC-136 | The described catalogue is rejected and no evaluation cell can run |
| EC-015 | A legacy path is deleted before its Rust replacement and direct invocation pass | FR-018 | TC-113 | Qualification or onboarding becomes unavailable |
| EC-016 | Path-qualified ix-trace-rs macro compiles but Quire cannot bind it | NFR-005 | TC-117, TC-118 | Passing Rust tests provide no requirement evidence |
| EC-017 | Content path or protected token attempts to escape or disclose selected bytes | FR-017 | TC-119 | Qualification reads unowned content or leaks the material it is meant to reject |

## Integration Test Matrix

### Cross-Project Integrations

| Integration ID | Purpose | Target Project | Type | Test Cases | Status |
|----------------|---------|----------------|------|------------|--------|
| INT-001 | Validate justified assurance artifacts | quire-rs | service | TC-006, TC-017, TC-045 | ✅ |
| INT-002 | Persist and render evidence without redefining policy | quoin | service | TC-025, TC-046 | ✅ |
| INT-003 | Load workflows, persist runs, resume, and gate decisions | ix-flow | service | TC-026, TC-028, TC-029, TC-030, TC-035, TC-047, TC-048 | ✅ |
| INT-004 | Discover canonical onboarding through supported hosts | agent discovery adapters | service | TC-010, TC-011, TC-038 | ✅ |
| INT-005 | Validate shared semantic references | Quire, Quoin, ix-flow, native producer fixtures | library | TC-052, TC-053, TC-056..TC-063, TC-068 | ✅ |
| INT-008 | Drive the ix-flow run lifecycle through the Rust workflow host | ix-flow | service | TC-107 | ✅ |
| INT-007 | Load Rust-owned scenarios and assertions through a structured host interface | cli-agent-evals | service | TC-109, TC-110 | ✅ |

INT-006 is withdrawn with FR-016-AC-4. Its row is removed and the identifier is not reused.

### Integration Test Details

| Test Case | Integration | Scenario | Input | Expected | Priority |
|-----------|-------------|----------|-------|----------|----------|
| TC-006 | INT-001 | Justified artifact validation | Installed skeleton and fictional bounded inputs | Quire accepts artifact before recommendation | P0 |
| TC-017 | INT-001 | Repository-source module validation | Installed archive and fictional document | Existing module root validates | P0 |
| TC-025 | INT-002 | Evidence delegation | Valid observed producer result | Quoin owns retained record and report | P0 |
| TC-026 | INT-003 | Interrupted workflow resume | Persisted non-terminal run | Next action without repeated phase | P0 |
| TC-028 | INT-003 | Human rejection | Named owner rejection action | Rejection terminal only | P0 |
| TC-029 | INT-003 | Missing terminal choice | Decision-ready run | Remains non-terminal | P0 |
| TC-030 | INT-003 | Invalid gate override | Automatic terminal gate request | Fails closed | P0 |
| TC-035 | INT-003 | Compatibility discovery | Four pilot names | Every pilot loads | P0 |
| TC-045 | INT-001 | Validated publication | Staged valid/invalid artifacts and escaping target | Only valid in-root artifact is atomically published | P0 |
| TC-046 | INT-002 | Evidence-state exclusivity | Generated producer-state combinations | Exactly one catalogued state per producer | P0 |
| TC-047 | INT-003 | Human acceptance | Named owner acceptance action | One attributed acceptance terminal event | P0 |
| TC-048 | INT-003 | Run identity mismatch | Existing id with changed binding | Refused with both runs unchanged | P0 |
| TC-010 | INT-004 | Host discovery permutations | Two sources by four hosts | Same canonical skill | P0 |
| TC-011 | INT-004 | Workflow discovery | Canonical skill | Exactly four workflow definitions | P0 |
| TC-107 | INT-008 | Run lifecycle and human gates | Versioned Rust provider and fictional runs | Existing ix-flow state behavior is preserved | P0 |
| TC-109 | INT-007 | Complete declared-host evaluation | Seven scenarios on every declared host | One valid result cell per declared cell, with explicit decisions | P0 |
| TC-110 | INT-007 | Invalid or incomplete evaluation | Missing and malformed host results | Aggregate gate remains withheld | P0 |

## Engineering Assurance #5 Coverage

| Requirement | Criterion | Test Case | Status |
| --- | --- | --- | --- |
| StR-002 | StR-002-VC-1 | TC-052 | ✅ Passing |
| US-005 | US-005-AC-1 | TC-053 | ✅ Passing |
| FR-008 | FR-008-AC-1 | TC-056 | ✅ Passing |
| FR-008 | FR-008-AC-2 | TC-057 | ✅ Passing |
| FR-008 | FR-008-AC-3 | TC-058 | ✅ Passing |
| FR-008 | FR-008-AC-4 | TC-059 | ✅ Passing |
| FR-009 | FR-009-AC-1 | TC-060 | ✅ Passing |
| FR-009 | FR-009-AC-2 | TC-061 | ✅ Passing |
| FR-009 | FR-009-AC-3 | TC-062 | ✅ Passing |
| FR-009 | FR-009-AC-4 | TC-063 | ✅ Passing |
| FR-013 | FR-013-AC-1 | TC-087 | ✅ Passing |
| FR-013 | FR-013-AC-2 | TC-088 | ✅ Passing |
| FR-013 | FR-013-AC-3 | TC-089 | ✅ Passing |
| FR-013 | FR-013-AC-4 | TC-090 | ✅ Passing |
| FR-013 | FR-013-AC-5 | TC-091 | ✅ Passing |
| FR-013 | FR-013-AC-6 | TC-092 | ✅ Passing |
| FR-013 | FR-013-AC-7 | TC-093 | ✅ Passing |
| FR-013 | FR-013-AC-8 | TC-094 | ✅ Passing |
| NFR-004 | NFR-004-AC-1 | TC-068 | ✅ Passing |
| NFR-004 | NFR-004-AC-2 | TC-059 | ✅ Passing |

## Coverage Gaps

The completed baseline has no open gap: Quire reconciles TC-001..TC-049
to real tracking-tagged symbols, and the retained declared-population aggregate records the
selected host commands, models, governing versions, transcripts, and outcomes.
TC-052, TC-053, TC-056..TC-063 and TC-068 are backed natively by the Rust semantic library, whose
tracking-tagged tests carry every criterion the retired
`tests/test_verification_semantics.py` used to carry. TC-050, TC-051 and TC-069..TC-078 are
withdrawn together with FR-006-AC-6, FR-006-AC-7, FR-011 and FR-015-AC-5; TC-054, TC-055 and TC-064..TC-067 are withdrawn together with FR-010, US-005-AC-2, US-005-AC-3 and FR-015-CON-1; TC-199, TC-204 and
TC-208 are withdrawn together with FR-014-AC-11, FR-024-AC-12 and FR-027-AC-3; EC-013 is
withdrawn; their ids are not reused. TC-087..TC-094
remain in `tests/python_port/migration_contract.rs`, which reads the contract document
directly.

TC-109 depends on a cli-agent-evals interface supported by its owner.

## Test Execution Summary

| Category | Total | Passed | Failed | Blocked | Coverage |
|----------|-------|--------|--------|---------|----------|
| Unit | 12 | 12 | 0 | 0 | 100% |
| Integration | 20 | 20 | 0 | 0 | 100% |
| E2E | 13 | 13 | 0 | 0 | 100% |
| Property | 15 | 15 | 0 | 0 | 100% |
| Static | 8 | 8 | 0 | 0 | 100% |

The table reports only the completed baseline and does not count the 23 staged
Rust-port rows.
