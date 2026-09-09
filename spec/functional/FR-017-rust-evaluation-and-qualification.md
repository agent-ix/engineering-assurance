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

## Outputs

- Versioned per-run and aggregate evaluation results.
- Deterministic qualification diagnostics and exit statuses.
- Audited package contents and enforced publication refusal.

## Behavior

- Preserve the evaluation fields and explicit terminal events required by
  FR-006.
- Fail aggregation when a required host-scenario cell is absent, invalid,
  unavailable, or unsuccessful.
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

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-017-AC-1 | The Rust evaluation path completes all required host-scenario cells and matches retained success and declared failure behavior. | E2E (TC-109) |
| FR-017-AC-2 | Missing, malformed, unavailable, stale-revision, changed-governing-file, and incomplete evaluation inputs withhold aggregation. | Property (TC-110) |
| FR-017-AC-3 | Rust package, rights, manifest, integration, and publication-refusal checks match the retained pass/fail corpus and reject extra, missing, or escaping package members. | Property (TC-111) |
| FR-017-AC-4 | Static inspection finds only declarative dispatch in package-manager and host configuration; qualification is performed locally and real-agent evaluation, publication, and release operations remain explicit manual actions. | Test (TC-112) |

## Dependencies

- **Upstream**: FR-006, FR-014, FR-015, and a cli-agent-evals interface
  supported by that host's owner.
- **Downstream**: FR-018 requires these checks before old implementations are removed.
