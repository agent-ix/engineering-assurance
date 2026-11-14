---
title: "EA-123 PR #274 round 1 review dispositions"
type: ReviewDisposition
relationships:
  - target: ix://agent-ix/engineering-assurance/EA-123
    type: records
---

## Reviewer findings

| Finding | Disposition | Evidence |
| --- | --- | --- |
| SR-20261114084858 FND-001 | Fixed | `write_file` verifies every existing checkout parent with `symlink_metadata`, rejects symlinks/non-directories, and creates the final file exclusively. `checked_out_symlink_cannot_redirect_private_write_outside_the_clone` seeds a malicious symlink in the fixed ref and verifies the external target remains absent. |
| SR-20261114084858 FND-002 | Fixed | The private Git adapter and sink now call `record_sink::config::valid_repository`; duplicate validators were removed. |
| SR-20261114084858 FND-003 | Fixed | The adapter initializes a local repository, adds the destination remote, and fetches only the fixed `refs/heads/engineering-assurance-private-records` ref. `private_store_fetches_only_its_fixed_ref_and_rejects_ref_configuration` verifies unrelated advertised branches are absent from tracking refs and alternative ref configuration fails. |
| SR-20261114084858 FND-004 | Fixed | Invalid, malformed, or untrusted existing outcomes map to `StorePreparation::OutcomeIncomplete`; Git storage failures remain write failures. The malformed outcome and incomplete prior-outcome assertions cover the fail-closed mapping. |
| SR-20261114084858 FND-005 | Fixed | `signed_approval_fixture` verifies the fixture signer established the bundle and exact payload before persistence. The lookup test no longer inserts trusted bytes after lookup. |
| SR-20261114084858 FND-006 | Fixed | Split the overlong test bodies into focused assertion helpers and removed needless borrows. `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes without lint suppressions. |
| SR-20261114084859 FND-001 | Fixed | TC-976 now tags independent-process concurrent distinct-ID writes and the five-rejected-push bounded retry test. |
| SR-20261114084859 FND-002 | Fixed | TC-983 tags changed-outcome and foreign-`record_publisher` identity tests; the foreign fixture is verified under its own identity policy before rejection by the configured policy. |
| SR-20261114084859 FND-003 | Fixed | TC-984 tags a same-ID two-process race and asserts no more than one selected-sink invocation and a verified terminal outcome. |
| SR-20261114084859 FND-004 | Fixed | TC-993 tags fixed-ref-only fetch and rejection of an alternative ref. |
| SR-20261114084859 FND-005 | Fixed | TC-987 tags exact statement/bundle envelope assertions and rejection of unknown envelope members. |
| SR-20261114084859 FND-006 | Fixed | TC-988 tags the canonical signer-input bytes assertion and exact returned bundle round trip. |
| SR-20261114084859 FND-007 | Fixed | TC-991's fixture signature is established and checked before storage; lookup verifies the unchanged payload and rejects a one-byte alteration. |
| SR-20261114084859 FND-008 | Fixed | TC-975 now asserts an identical stale write is reapplied after a competing non-fast-forward update and preserves bytes. |
| SR-20261114084859 FND-009 | Fixed | TC-978 compares the canonical stored statement against the fixture signer's pre-storage payload and exact returned bundle. |
| SR-20261114084859 FND-010 | Fixed | TC-980 checks changed-sink and changed-outcome replacement both fail and preserve the original marker blob. |
| SR-20261114084859 FND-011 | Fixed | TC-981 exercises both published and refused backend replay and verifies the sink layer suppresses reinvocation. |
| SR-20261114084859 FND-012 | Fixed | TC-982 checks a required public form removed from a retained private-store publication is rejected before sink invocation. |
| SR-20261114084859 FND-013 | Fixed | TC-989 asserts ascending record-ID result order, exact repository/PR/commit matching, and fixed-ref lookup. |
| SR-20261114084859 FND-014 | Fixed | TC-990 asserts malformed private data, missing/malformed outcomes, Git read failure, and no partial result after a later record fails. |
| SR-20261114084859 FND-015 | Fixed | TC-992 mutates a structurally valid signed outcome field and separately alters its bundle; both make lookup fail. |
| SR-20261114084859 FND-016 | Fixed | TC-979 now points to the existing sink/outcome Rust tests and records their passing status rather than pending. |

## Verification

- `cargo test --locked --features private-git-store --lib private_git_store`: 13 passed.
- `cargo test --locked --features record-sink --test record_sink`: 34 passed.
- `cargo test --locked --features full --test source_audit`: 18 passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `make content-rights-tree`: accepted, 709 entries, no findings.
- Workspace formatting check still reports pre-existing formatting differences in generated campaign files on the branch base. The changed Rust file was formatted directly with rustfmt.
