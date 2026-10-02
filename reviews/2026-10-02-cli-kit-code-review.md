---
id: SR-137
title: Shared CLI output Rust diff review
type: SpecReview
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-028"
    type: reviews
---

# SR-137: Shared CLI output Rust diff review

## Summary

Reviewed direct shared output consumption against the committed conventions and Rust review checklist. Domain encoders, machine documents, statuses and diagnostics retain their existing boundary behavior.

## Scope

Reviewed revision e0c83367796e7cb35ff6f7200a3e4f98fb4101b9. Workflow diff is empty. Files examined:

- `Cargo.lock`
- `Cargo.toml`
- `deny.toml`
- `reviews/2026-10-02-cli-kit-spec-review.md`
- `spec/functional/FR-028-shared-cli-output.md`
- `src/content_rights.rs`
- `src/main.rs`
- `tests/content_rights_parity.rs`
- `tests/default_features.rs`
- `tests/workflow_host_cli.rs`

## Verdict

Pass. The full-only optional dependency preserves minimal consumers. Machine-error JSON retains field order and its final newline. The rights-policy addition accepts the exact required dependency identity only at the three root metadata paths; adverse tests reject look-alike identities and nested/prose paths. No credential operation or downstream policy relocation occurs.

## Validation

Complete lint, test, Rust foundation, individual feature matrix, package audit and document validation gates passed. Real process tests verify successful response-byte stability, malformed request exact bytes and stream roles. The complete rights check passed. The fixture temporary root is canonical, preserving the verifier's identity requirement. A second complete gate is required before merge.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder). | Reviewed scope |
