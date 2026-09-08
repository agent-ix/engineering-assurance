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
---

# FR-017: Run evaluations and repository qualification through Rust

## Description

Engineering Assurance SHALL perform agent-evaluation planning, result
validation, aggregation, package staging, publication refusal, content-rights
checking, manifest validation, and release-gate assertions through the Rust
library and CLI.

## Inputs

- The canonical agent-evaluation scenarios and supported-host set.
- Versioned cli-agent-evals results and retained transcripts.
- Package manifests, staged archives, rights policy, and integration evidence.

## Outputs

- Versioned per-run and aggregate evaluation results.
- Deterministic qualification diagnostics and exit statuses.
- Audited wheel/npm contents and an enforced public-registry refusal.

## Behavior

- Evaluation results SHALL retain the governing versions, commands, transcript
  digests, effort, elapsed time, interaction counts, outcomes, and explicit
  terminal events required by FR-006.
- The aggregate gate SHALL fail when any required host-scenario cell is absent,
  invalid, unavailable, or unsuccessful.
- Package audits SHALL compare the complete staged member set against an
  explicit allowlist and reject missing, extra, or escaping members.
- Content-rights qualification SHALL inspect the complete tracked tree and every
  staged package member.
- Registry publication SHALL remain refused.
- Hosted workflow dispatch posture SHALL remain unchanged.

## Error Conditions

A missing host executable, incomplete scenario cell, invalid result contract,
revision mismatch, changed governing file, unexpected package member, content
rights denial, and attempted publication each fail the corresponding gate.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-017-CON-1 | The Rust evaluator SHALL NOT infer a human terminal decision. | Responsibility | Test |
| FR-017-CON-2 | Package-manager and CI configuration SHALL contain no first-party semantic assertion. | Architecture | Test |
| FR-017-CON-3 | The migration SHALL NOT dispatch hosted CI. | Operational | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-017-AC-1 | The Rust evaluation path completes all 28 required host-scenario cells and produces results equivalent to the retained reference for success and every declared failure. | E2E (TC-109) |
| FR-017-AC-2 | Missing, malformed, unavailable, stale-revision, changed-governing-file, and incomplete evaluation inputs each withhold the aggregate gate. | Property (TC-110) |
| FR-017-AC-3 | Rust package, rights, manifest, integration, and publication-refusal gates match the retained pass/fail corpus and reject extra, missing, or escaping package members. | Property (TC-111) |
| FR-017-AC-4 | Static inspection finds only declarative dispatch in package-manager and CI configuration and confirms that workflow triggers are unchanged (CON-2, CON-3). | Test (TC-112) |

## Dependencies

- **Upstream**: FR-006, FR-014, FR-015, and a reviewed cli-agent-evals
  structured external-suite interface.
- **Downstream**: FR-018 requires these qualification gates before removal.
