---
id: SR-087
title: "Producer-execution consumer-boundary base review"
type: SpecReview
analysis: base
scope: "FR-019, IT-006, TC-122 through TC-128"
review_set: base
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/IT-006"
    type: reviews
---

## Summary

The owner-selected base review re-examined the Engineering Assurance #34
provider boundary after the first real consumer review. Five gaps prevented a
consumer from retaining the exact result, trusting output bytes, treating the
working input population as closed, qualifying the implementation without
foreign-language fixtures, or compiling only the provider surface. All five
findings were closed in the specification through `/specify`; implementation
and consumer acceptance remain pending.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-043 | high | Closed in specification: a typed Rust result plus protocol string did not define retainable canonical result bytes or an identity. Every portable result field and caller-owned serializable observation now participates in RFC 8785 canonical bytes and `sha256-jcs` result identity; live handles are excluded only when their exact digest metadata is included. | FR-019 Outputs; FR-019-AC-1; TC-122 |
| FND-044 | high | Closed in specification: output observation hashed a pathname and dropped the descriptor, forcing retention to reopen a mutable path. Each output is now copied into a sealed retained snapshot; its digest and metadata come from that snapshot, and both adapter and caller can read the same bytes. | FR-019 Outputs; FR-019 Behavior; FR-019-AC-1; TC-122 |
| FND-045 | high | Closed in specification: declared input files were snapshotted, but the producer still ran in the original capability root and could read undeclared or changed files. The request input list now defines the complete staged working projection, and extra, changed-unbound, or post-preflight source-root files are not visible through the working directory. | FR-019 Behavior; FR-019-AC-2; TC-123 |
| FND-046 | high | Closed in specification: the producer contract tests executed Python and shell fixtures despite the owner-language boundary applying to executable test fixtures. TC-122 through TC-125 now require one Rust fixture binary; foreign runtimes remain supported protocol inputs but are not first-party gate implementations. | FR-019 Behavior; IT-006; TC-122 through TC-125 |
| FND-047 | medium | Closed in specification: a consumer of the monolithic default crate compiled unrelated package/archive, onboarding, CLI, YAML, regex and source-audit dependencies. A dedicated `producer-execution` Cargo feature with defaults disabled now defines the lightweight consumer surface while the default full feature preserves the package. | FR-019 Behavior; FR-019-AC-7; IT-006; TC-128 |

## Base checklist result

- Canonical result bytes cover every portable result field and state, with a
  typed refusal when the caller observation cannot be represented.
- Output identity and output access refer to one immutable retained snapshot;
  no caller or adapter must reopen the producer pathname.
- The staged working tree is a closed projection of declared inputs and output
  parents, without claiming an adversarial operating-system sandbox.
- The lightweight feature preserves the existing crate boundary while avoiding
  unrelated default-package dependencies.
- Rust-owned fixture executables prove the provider behavior; foreign producer
  support remains part of the public protocol rather than its qualification
  implementation.
- TC-122 through TC-125 regress to pending where the old fixture or incomplete
  interface cannot satisfy the amended requirement. TC-126 remains the real
  consumer gate and TC-128 proves the dependency boundary.

## Decision

**PASS after specification remediation.** Implementation may continue on the
existing crate and pull request. It must close FND-043 through FND-047 and pass
TC-122 through TC-125 plus TC-128 before the provider is handed back to the
consumer for TC-126. The change does not authorize a new crate, repository,
generic CLI runner, domain oracle, evidence store, or Quire language work.
