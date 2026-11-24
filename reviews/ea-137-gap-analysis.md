---
id: SR-20261124033412
title: "Gap analysis — EA-137 tag receipt implementation"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance; src/tag_receipt.rs; src/tag_receipt_host.rs; src/receipt_host.rs; src/approval_trail_host.rs; .github/workflows/tag-receipt*.yml; .github/workflows/missing-receipt-check.yml; related tests; FR-081/FR-082 and linked workflow/identity criteria"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/FR-081", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-082", type: references }
---

## Summary

Repository-driven gap analysis of EA-137's tag receipt requirements, implementation, workflows and test bindings; plan completion was not assessed. The pure comparison and Rust paths have matrix bindings, but the checked-in signing workflow downloads an older executable that cannot run the new commands, leaving receipt creation unimplemented in operation.

## Verdict

**FAIL** — FR-081's receipt-producing workflow is unusable with the pinned release executable.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The tag-receipt jobs download v0.10.0, whose source predates `tag-receipt-record` and `tag-receipt-note`; every configured tag push fails before it can create the receipt. | .github/workflows/tag-receipt.yml:41; FR-081-AC-1 | implementation-bug-despite-evidence |

## Coverage

- Reconciliation: `quoin matrix --repo . --json` (quoin 0.36.8, quire 0.52.0) plus `quire coverage --scope . --json`; the matrix covered 900 criteria: 680 tagged, 142 untagged, 60 method-without-symbol and 18 tagged by ignored tests. No run evidence was present in the matrix.
- Targeted FR-081/FR-082 and linked workflow/identity slice: 34 criteria; 33 tagged, 1 method-without-symbol (`FR-082-AC-9` is an Inspection criterion); all 34 had no run evidence before this review's focused gates.
- `FR-081`: all 11 ACs had one or more matrix binders. `FR-082`: 11 of 12 ACs had binders; `FR-082-AC-9` is Inspection and has no source symbol by method.
- `quire coverage` reported 1,355 backed of 1,603 rows repository-wide; this is repository baseline context, not attributed to this change. The changed FR-081 and FR-082 groups were 11/11 and 11/12 backed.
- Untraced behaviors / stubs in the reviewed slice: no stub found by source inspection; matrix showed the single expected inspection-only AC above.
- Semantic review: reviewed FR-081/FR-082 descriptions, behavior and criteria against the changed code, tests and workflows.
- Plan completion: not assessed
- Focused gates: `git diff --check`, changed-file `rustfmt --check`, 40 focused integration tests, 10 tag-receipt host unit tests, and focused Clippy with `-D warnings` passed.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-002 | high | The reusable workflow builds the CLI from the caller repository's default-branch checkout; adopters without the Engineering Assurance Cargo package fail in the build job. | .github/workflows/tag-receipt.yml:14-37; FR-070-AC-14 | implementation-bug-despite-evidence |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The workflow builds the CLI and checks both required tag-receipt commands before transferring it to the record and notes jobs. |
| FND-002 | still-open | The reusable workflow checks out the caller repository and runs its Cargo build there. Adopter callers without this crate's package cannot build the CLI. |

PR association correction: the original review marker's `pr=engineering-assurance#304` points to the already-merged spec PR (head `ea-137-spec`). This disposition reviews code branch `ea-137-code` at `98e6159b6e63550a00f2d3ba31b9cee030b03eb3`; no code PR exists for this branch at this head.

Disposition evidence at `98e6159b6e63550a00f2d3ba31b9cee030b03eb3`:

- FND-001 fixed excerpt from `.github/workflows/tag-receipt.yml`:

  `after_excerpt`:

  ```sh
  cargo build --locked --release --features full --bin engineering-assurance
  cli="$GITHUB_WORKSPACE/source/target/release/engineering-assurance"
  "$cli" --version
  "$cli" tag-receipt-record --help >/dev/null
  "$cli" tag-receipt-note --help >/dev/null
  ```

- FND-002 remains open: the build job's checkout omits `repository`, so `actions/checkout` checks out the caller's repository. The job then runs `cargo build --features full --bin engineering-assurance` in that checkout. The reusable workflow contract describes adopters calling this workflow and `FR-070-AC-14` requires its jobs to match the declared table; the added build job's assumption that callers contain this package is unfulfilled for adopters without it.
- Fix-round verification: `receipt_workflow_contract` passed (6 tests). A focused run of `protected_policy_lookup_distinguishes_absence_from_git_read_failures` was queued behind older build-lane waiters and had not completed when this artifact was updated.


Round 2 verification at `2412f44876177e549c4641efecb278c3378f4e82` (diff from `98e6159b6e63550a00f2d3ba31b9cee030b03eb3`):

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-002 | fixed | The build job checks out the public `agent-ix/engineering-assurance` repository at `main` into `source`, then builds the CLI there. External callers no longer need this Cargo package in their own repository. |

Round 2 evidence:

- FND-001 remains fixed: `.github/workflows/tag-receipt.yml:38-42` builds `engineering-assurance` from `source` and checks both receipt subcommands before upload.
- FND-002 is fixed: `.github/workflows/tag-receipt.yml:19-26` sets `repository: agent-ix/engineering-assurance`, `ref: main`, and `path: source`. GitHub reports that repository as public, so an external caller can fetch the build source; its own default-branch checkout is not used. `tests/receipt_workflow_contract.rs:428-465` asserts the repository/ref/path and build commands. The focused `receipt_workflow_contract` test passed (6 tests) after this fix commit, as reported for this disposition.
- No new finding: inspection confirms the reusable build reads the public source repository explicitly and builds the CLI from it, so callers without this crate can use the workflow.

PR association correction: the original review marker's `pr=engineering-assurance#304` identifies the already-merged spec PR (head `ea-137-spec`), not the reviewed code. This round reviews branch `ea-137-code` at `2412f44876177e549c4641efecb278c3378f4e82`; `gh pr list --head ea-137-code` returned no code PR.
