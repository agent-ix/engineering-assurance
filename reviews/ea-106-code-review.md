---
id: SR-2422
title: "Code review — engineering-assurance EA-106 signing foundation"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; Cargo.toml; Cargo.lock; src/attestation_signing.rs; src/lib.rs; EA-106 acceptance criteria and scope"
review_set: subset
---

## Summary

Reviewed the frozen EA-106 diff at `0eb5215733db52c40c72f139b2d706b70d476765`, including the Rust implementation, dependency feature wiring, relevant EA-105 sink API, requirements, and focused tests. The candidate has material integration and subject-coverage gaps, and several claimed verification oracles do not exercise the public boundary or delegated Sigstore verifier.

## Verdict

**FAIL** — one high finding leaves the existing sink signing interface unimplemented; medium findings also cover missing subject behavior and insufficient security-path evidence.

## Scope

Examined all four changed paths (`Cargo.toml`, `Cargo.lock`, `src/attestation_signing.rs`, `src/lib.rs`), the EA-106 acceptance criteria and scope, the EA-105 `Signer`/`PublicPathProof` boundaries, FR-058, FR-064, NFR-010 and NFR-011. No applicable AssuranceProfile was found under `spec/`. The trace tags and their test bodies were checked against the computed matrix and stated criterion intent.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The module exposes standalone signing functions but does not implement EA-105's `Signer` trait, leaving the existing sink layer unable to use this signing stack or obtain `signed_payload`. | src/attestation_signing.rs:264-300; src/record_sink/sink.rs:197-219 |
| FND-002 | medium | Subject matching supports only `gitCommit` and `gitTree`; the requested `sha256` artifact-digest path cannot be selected or verified. | src/attestation_signing.rs:69-76, 123-130, 225-237 |
| FND-003 | medium | All verifier tests use `FixtureVerifier`; none exercises `SigstoreBundleVerifier` with a library-generated signed bundle, so the production certificate identity, issuer, transparency, and bundle checks have no behavioral oracle. | src/attestation_signing.rs:150-203, 434-478 |
| FND-004 | medium | The subject-mismatch oracle tests a mismatching commit only; the tree path has a success assertion but no mismatching tree or absent-digest case. | src/attestation_signing.rs:497-570 |
| FND-005 | medium | The no-OIDC test calls only `require_ambient_token(None)`, so it does not exercise either public signing entry point or observe that service setup/network access is skipped. | src/attestation_signing.rs:241-276, 287-300, 624-631 |
| FND-006 | medium | The rejecting-verifier test repeats the same valid commit input three times and then one mismatching commit; it omits distinct policy-mismatch, tampered-payload, and tree cases from the claimed AC 1–4 set. | src/attestation_signing.rs:574-611 |
| FND-007 | medium | `WorkflowIdentityPolicy` represents one `reference`, so callers cannot express and match a policy's listed ref set through this verifier API. | src/attestation_signing.rs:78-110, 212-218 |
| FND-008 | medium | The test tagged to FR-058-AC-1 uses an empty generic predicate and checks only the in-toto type/canonical bytes; it does not assert the approval predicate type, closed member set, repository subject, or merged-commit subject required by that criterion. | src/attestation_signing.rs:427-431, 613-622 |

## Validation

- `git diff --check origin/main...HEAD` — passed (exit 0).
- Wrapped `cargo fmt --check` — passed (exit 0).
- Wrapped `cargo check --locked --lib --no-default-features --features attestation-signing` — passed (exit 0).
- Wrapped `cargo clippy --locked --lib --no-default-features --features attestation-signing -- -D warnings` — passed (exit 0).
- Wrapped `cargo test --locked --lib --no-default-features --features attestation-signing attestation_signing::tests` — passed (5 passed, 0 failed).
- Every Cargo command above ran through `locked-build.sh` with the worktree-local `target` directory.
- No workflow diff or live signing test was run.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | TC-112's release guard misses GitHub CLI release operations such as `gh release upload` and `gh release edit`; a workflow can perform a release operation while this audit reports no violation. | tests/python_port/module.rs:1417-1426 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `SigstoreSigner` now implements the EA-105 trait and accepts `PublicPathProof` for public-log signing. |
| FND-002 | fixed | `SubjectKind::Sha256` selects and compares the `sha256` subject digest. |
| FND-003 | fixed | An opt-in integration test uses `TrustedRoot::production()` and `SigstoreBundleVerifier` to verify a public-good signed bundle. The test is available through the manually dispatched job; it was not run in this review. |
| FND-004 | fixed | Fixture assertions now reject a mismatching tree digest and an absent sha256 digest. |
| FND-005 | still-open | The test calls the injected `*_with` helpers and checks the missing-token error, but it does not observe whether the service-start closure or network setup was skipped, nor does it exercise the public `Signer` methods. The no-network guarantee remains unverified by this test oracle. |
| FND-006 | fixed | The rejecting-verifier input array now includes a trusted commit, wrong policy, tampered payload, and wrong-tree case, and checks that each propagates the delegated verifier's rejection. |
| FND-007 | fixed | `WorkflowIdentityPolicy.references` is a set and tests allow both main and release refs while rejecting an outside ref. |
| FND-008 | fixed | The generic Statement serialization test no longer claims FR-058-AC-1 trace coverage. |
| FND-005 | fixed | The `Signer` public-method test now sends both signing paths through the missing-OIDC backend, which calls the shared service-start closure and asserts its call count remains zero. |
| FND-009 | fixed | TC-112 now detects `gh release upload` and `gh release edit`, with negative fixtures for both; existing checks still cover package publication and non-manual triggers. |
