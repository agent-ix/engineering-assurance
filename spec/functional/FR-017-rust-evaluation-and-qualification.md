---
id: FR-017
title: "Run evaluations and repository qualification through Rust"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-006"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/NFR-003"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: "requires"
---

# FR-017: Run evaluations and repository qualification through Rust

## Description

Engineering Assurance SHALL port its agent-evaluation planning, result
validation, aggregation, package staging, publication refusal, content-rights
checking, manifest validation, and qualification assertions to Rust.

## Inputs

- Canonical agent-evaluation scenarios and supported hosts.
- Versioned cli-agent-evals results and retained transcripts.
- Package manifests, staged archives, rights policy, and integration evidence.
- For the pure aggregation boundary, one
  `engineering-assurance.evaluation-aggregate-request/v1` document containing
  evaluation envelopes for the closed host and scenario populations. Each
  envelope carries its execution status, pass state, immutable suite, fixture,
  source, host, and governing identities, relative transcript reference and
  lowercase SHA-256 digest, non-negative effort counts, observed outcome,
  unsupported-addition list, optional diagnostic, and the complete terminal
  event when the scenario requires a human decision.

## Outputs

- Versioned per-run and aggregate evaluation results.
- Deterministic qualification diagnostics and exit statuses.
- Audited package contents and enforced publication refusal.
- The pure aggregation boundary emits one
  `engineering-assurance.evaluation-aggregate-result/v1` value containing the
  pass decision, required-cell count, complete-cell count, and a deterministic
  ordered list of stable failure codes. It does not persist evidence or infer a
  release decision.

## Behavior

- Preserve the evaluation fields and explicit terminal events required by
  FR-006.
- Treat the supported host population as `claude`, `codex`, `opencode`, and
  `copilot`, and the scenario population as `existing-profile`, `no-profile`,
  `malformed-producer`, `unavailable-producer`, `interruption-resume`,
  `human-acceptance`, and `human-rejection`; their Cartesian product is exactly
  28 required cells.
- Validate envelopes through closed Rust types rather than untyped object
  walking. Unknown envelope fields, malformed request structure, an unknown
  protocol, unsupported host or scenario values, and invalid scalar types are
  request errors and produce no aggregate result. The byte boundary refuses
  input larger than 8 MiB before JSON decoding or aggregate allocation.
- An executed cell is complete only when it passed; every immutable identity is
  present and valid; its transcript reference is a normalized,
  forward-slash-delimited, relative and non-traversing protocol path; its
  transcript digest is lowercase SHA-256; every effort count is a non-negative
  integer; the observed outcome is the scenario's declared outcome; and
  unsupported additions are empty.
- A not-executed cell requires a non-empty diagnostic and remains incomplete.
  It cannot satisfy the aggregate even if it carries otherwise complete
  observations.
- Human-acceptance and human-rejection cells require complete terminal events
  with the corresponding choice and outcome, the governing workflow name and
  version, a non-empty owner, a safe run ID, and an RFC 3339 timestamp. Every
  other scenario requires no terminal event. For each host the two decision
  cells use distinct run IDs and identical source, fixture, and governing
  identities.
- Fail aggregation when a required host-scenario cell is absent, invalid,
  unavailable, unsuccessful, or duplicated. An unsupported host or scenario is
  a malformed request rather than an aggregate cell.
- Require one source revision, suite revision, fixture revision, and governing
  tuple across the aggregate. Require one workflow identity within each
  scenario while permitting the declared intake and architecture workflows to
  differ between scenarios.
- Order missing, duplicate, cell-validation, aggregate-identity,
  scenario-workflow, and terminal-pair failures deterministically by closed
  host/scenario order so equivalent input permutations produce the same result.
- Compare complete staged package membership with explicit allowlists and reject
  missing, extra, or escaping members.
- Inspect the complete selected tree and staged members for content rights.
- Keep publication refused.
- Keep package-manager and host configuration declarative; first-party semantic
  assertions live in Rust.

## Error Conditions

Missing hosts, incomplete scenarios, invalid results, revision mismatches,
changed governing files, unexpected package members, rights denials, and
attempted publication fail their corresponding local gate.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-017-CON-1 | The Rust evaluator SHALL NOT infer a human terminal decision. | Responsibility | Test |
| FR-017-CON-2 | Package-manager and host configuration SHALL contain no first-party semantic assertion. | Architecture | Test |
| FR-017-CON-3 | The reusable Rust evaluation validator and aggregator SHALL NOT access the filesystem, environment, process table, network, or system clock; transcript-byte loading, host execution, and current-HEAD comparison belong to explicit binary adapters. | Responsibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-017-AC-1 | The Rust evaluation path completes all required host-scenario cells and matches retained success and declared failure behavior. | E2E (TC-109) |
| FR-017-AC-2 | The pure Rust boundary preserves the 28-cell retained evaluation contract and deterministically withholds aggregation for every missing, duplicate, malformed, unsupported, unavailable, failed, stale-revision, changed-governing-identity, changed-workflow, unsupported-addition, invalid-transcript-reference, invalid-count, outcome-mismatch, and terminal-pair case; input permutation cannot change the result. | Property (TC-110) |
| FR-017-AC-3 | Rust package, rights, manifest, integration, and publication-refusal checks match the retained pass/fail corpus and reject extra, missing, or escaping package members. | Property (TC-111) |
| FR-017-AC-4 | Static inspection finds only declarative dispatch in package-manager and host configuration; qualification is performed locally and real-agent evaluation, publication, and release operations remain explicit manual actions. | Test (TC-112) |

## Dependencies

- **Upstream**: FR-006, FR-014, FR-015, and a cli-agent-evals interface
  supported by that host's owner.
- **Downstream**: FR-018 requires these checks before old implementations are removed.
- **Sequence**: FR-017-AC-2's pure typed validation and aggregation is an
  independently reviewable additive slice. It does not satisfy FR-017-AC-1,
  verify transcript bytes, compare the current repository revision, authorize
  host execution, or permit removal under FR-018.
