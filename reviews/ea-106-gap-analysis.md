---
id: SR-2423
title: "Gap analysis — EA-106 attestation-signing changes"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance; Cargo.toml; Cargo.lock; src/attestation_signing.rs; src/lib.rs; EA-106 changed behavior"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/FR-064", type: references }
  - { target: "ix://agent-ix/engineering-assurance/NFR-010", type: references }
---

## Summary

Ran the planless repository matrix and reverse-gap review, then checked the new EA-106 verifier behavior and its test bindings. The candidate's intended NFR-010-M-3 rejecting-verifier test is not bound in the computed matrix, and the implementation also omits the existing `Signer` integration and `sha256` artifact-subject path called for by the EA-106 scope.

## Verdict

**FAIL** — the computed matrix leaves NFR-010-M-3 untagged, and a core signing integration path remains absent.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | NFR-010-M-3 remains untagged: the new test's `Trace: NFR-010-M-3` comment is in `src/attestation_signing.rs`, but the computed matrix has no binder for this obligation, so the rejecting-library check is not recognized as evidence. | NFR-010-M-3, src/attestation_signing.rs::tests::rejecting_sigstore_verifier_rejects_all_statement_cases |
| FND-002 | high | The new attestation module has no implementation of the existing `record_sink::sink::Signer` contract, so the EA-105 sink cannot use its signing operations. | src/attestation_signing.rs::sign_public_path_with_ambient_oidc, src/record_sink/sink.rs::Signer |
| FND-003 | medium | EA-106's subject-matching scope includes `sha256` artifact digests, but the implementation's closed `GitSubjectKind` only chooses `gitCommit` or `gitTree`. | src/attestation_signing.rs::GitSubjectKind, src/attestation_signing.rs::verify_statement |

## Coverage

- Reconciliation: `quoin matrix --repo . --json` (quoin 0.30.1, quire 0.45.0 / engine 0.60.0).
- Repository matrix: 694 criteria — 447 tagged, 197 untagged, 9 tagged-by-ignored-test, and 41 method-without-symbol; no run evidence is bound. The repository-wide untagged and ignored-only criteria outside this change are not listed as PR findings.
- Relevant metric: NFR-010-M-3 is untagged with no binder; its new in-module test is not recognized by this matrix.
- Untraced behavior or stubs introduced by the diff: 2 material EA-106 scope gaps (the missing `Signer` implementation and missing `sha256` artifact-subject path), also recorded in the code-review artifact.
- Plan completion: not assessed.
- Semantic review: skipped; no opt-in was requested.
