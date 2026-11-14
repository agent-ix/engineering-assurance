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
