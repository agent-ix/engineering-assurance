---
id: SR-20261124033411
title: "Code review — EA-137 tag receipts"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; full 15-commit origin/main..a1f3e7220 diff; all changed workflows, Rust source, tests, docs and examples"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/FR-081", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-082", type: references }
---

## Summary

Reviewed the full EA-137 15-commit diff against `origin/main`, including Rust code, focused tests, workflows, docs and ruleset examples. The receipt flow has two operational defects: the workflows invoke commands absent from the pinned release binary, and a failed protected-policy object lookup is silently treated as policy absence.

## Verdict

**FAIL** — the configured tag workflow cannot record receipts with its pinned executable; the policy lookup also has a fail-open skip path.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The tag-receipt jobs download v0.10.0, whose source predates `tag-receipt-record` and `tag-receipt-note`; every configured tag push fails before it can create the receipt. | .github/workflows/tag-receipt.yml:41 | implementation-bug-despite-evidence |
| FND-002 | medium | Any `git cat-file -e` failure is treated as an absent policy, so a protected-base commit missing from the fetched bare repository (including a branch-advance race) makes a matching tag return `skipped` successfully without a receipt. | src/tag_receipt_host.rs:214 | implementation-bug-despite-evidence |

## Review evidence

- Base: `origin/main`; reviewed head: `a1f3e7220` (15 commits, including the merge); 26 changed paths.
- Relevant requirements inspected: FR-081, FR-082, FR-043, FR-064, FR-070 and FR-077. No applicable AssuranceProfile was present under `spec/`.
- `git diff --check`: passed. Focused `rustfmt --check` over all changed Rust source and test files: passed.
- Focused integration tests (`tag_receipt`, `approval_trail`, `approval_trail_cli`, `receipt_workflow_contract`, `adoption_rulesets`): 40 passed, 0 failed. Focused `engineering-assurance` binary unit tests for `tag_receipt_host::tests`: 10 passed, 0 failed.
- Focused Clippy for the library, CLI binary, `tag_receipt`, and `receipt_workflow_contract` targets: passed with `-D warnings` through the required wrapper.
- Focused Cargo tests were run only through `locked-build.sh`, with Rust 1.98.1 and `CARGO_PROFILE_DEV_DEBUG=0`; the first attempt was refused by the read-only worktree target path and the retry used `the disposable review target directory`.
- No full suite, `--all-targets`, `cargo deny`, or broad lint gate was run.
- `AGENTS.md` was present; `CLAUDE.md` and `CONTRIBUTING.md` were absent.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-003 | high | The reusable workflow builds the CLI from the caller repository's default-branch checkout; adopters without the Engineering Assurance Cargo package fail in the build job. | .github/workflows/tag-receipt.yml:14-37; FR-070-AC-14 | implementation-bug-despite-evidence |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The workflow builds the CLI and checks both required tag-receipt commands before transferring it to the record and notes jobs. |
| FND-002 | fixed | Policy lookup now reports absence only after a successful empty tree lookup; Git and process failures propagate as errors. |
| FND-003 | still-open | The reusable workflow checks out the caller repository and runs its Cargo build there. Adopter callers without this crate's package cannot build the CLI. |

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

- FND-002 fixed excerpt from `src/tag_receipt_host.rs`:

  `after_excerpt`:

  ```rust
  let present = policy_file_exists(&request.repository, &base_commit)
      .map_err(|()| TagReceiptHostError::Git)?;
  ```

  `policy_file_exists` uses `git ls-tree`: empty output from a successful command returns `Ok(false)`, the exact policy path returns `Ok(true)`, and command errors or unexpected output return `Err(())`. The new regression test covers absent policy, present policy, a missing commit and an unreadable repository.

- FND-003 remains open: the build job's checkout omits `repository`, so `actions/checkout` checks out the caller's repository. The job then runs `cargo build --features full --bin engineering-assurance` in that checkout. The reusable workflow contract describes adopters calling this workflow and `FR-070-AC-14` requires its jobs to match the declared table; the added build job's assumption that callers contain this package is unfulfilled for adopters without it.
- Fix-round verification: `receipt_workflow_contract` passed (6 tests). A focused run of `protected_policy_lookup_distinguishes_absence_from_git_read_failures` was queued behind older build-lane waiters and had not completed when this artifact was updated.


Round 2 verification at `2412f44876177e549c4641efecb278c3378f4e82` (diff from `98e6159b6e63550a00f2d3ba31b9cee030b03eb3`):

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-003 | fixed | The build job checks out the public `agent-ix/engineering-assurance` repository at `main` into `source`, then builds the CLI there. External callers no longer need this Cargo package in their own repository. |

Round 2 evidence:

- FND-001 remains fixed: `.github/workflows/tag-receipt.yml:38-42` builds `engineering-assurance` from `source` and checks both receipt subcommands before upload.
- FND-002 remains fixed: `src/tag_receipt_host.rs:202-203,232-250` propagates lookup errors; only successful empty `git ls-tree` output means absent. Present-path and error cases are distinguished.
- FND-003 is fixed: `.github/workflows/tag-receipt.yml:19-26` sets `repository: agent-ix/engineering-assurance`, `ref: main`, and `path: source`. GitHub reports that repository as public, so an external caller can fetch the build source; its own default-branch checkout is not used. `tests/receipt_workflow_contract.rs:428-465` asserts the repository/ref/path and build commands. The focused `receipt_workflow_contract` test passed (6 tests) after this fix commit, as reported for this disposition.
- No new finding: inspection confirms the reusable build reads the public source repository explicitly and builds the CLI from it, so callers without this crate can use the workflow.

PR association correction: the original review marker's `pr=engineering-assurance#304` identifies the already-merged spec PR (head `ea-137-spec`), not the reviewed code. This round reviews branch `ea-137-code` at `2412f44876177e549c4641efecb278c3378f4e82`; `gh pr list --head ea-137-code` returned no code PR.
