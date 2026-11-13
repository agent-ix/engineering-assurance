---
id: SR-20261113124202
title: "Code review and Rust review of release-verify and Baseline resolution (engineering-assurance PR 228)"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance@fe1c308; PR 228 (EA-100); Cargo.toml; README.md; spec/tests.md; src/approval_verify_host.rs; src/lib.rs; src/main.rs; src/release.rs; src/release_host.rs; src/signature_verify_host.rs; tests/package_boundary.rs; tests/release.rs; tests/release_verify_cli.rs"
review_set: subset
---

# Code review and Rust review of release-verify and Baseline resolution (engineering-assurance PR 228)

## Summary

Ticket: EA-100. PR agent-ix/engineering-assurance#228 at head fe1c308, diff `origin/main...HEAD` (12 files, +2312/-33). Code-review with the rust-review lane folded in. Checked against the merged FR-046, FR-047 (as amended by EA-101), FR-049, FR-050, FR-051, FR-053 and NFR-007, and the reused FR-036, FR-037, FR-041 and FR-042. The EA-100 ticket text was read as data.

Measured at head: `cargo test --features full --test release --test release_verify_cli --test package_boundary` passes (5 + 33 + 2). `cargo test --no-default-features --features release --test release` passes. `cargo clippy --all-targets --features full` is clean and `cargo fmt --check` is clean. `tests/source_audit.rs` fails only on the known red from origin/main: tc_127 on approval_trail_host.rs and tc_117 on document_converter_producer_cli.rs tc_376. tc_117 stops at its first failure. Behind it, EA-93's three release tests in tests/semantic_exports.rs (lines 607, 628 and 667) have no `#[trace]`, which confirms the coder's report. That failure is on main and is not counted against this PR. release_host.rs uses `std::process::Output` only as a struct type, so tc_127 does not flag it.

Two mutations were run and both were killed. Disabling the repeated-path refusal in resolve_baseline makes tc_393 fail. Accepting a release commit whose first parent is the baseline makes tc_395 fail.

Two hostile-repository fixtures were built from scratch with fictional keys, and each one produced `released` where the spec requires `not_released`. See FND-001.

## Verdict

**FAIL**. One high finding. A repository's replace refs or graft file can change what release-verify reads under a valid signature (FND-001).

Checked and clean:
- The change has no crypto, trailer-parsing, hashing or snapshot-reading code of its own. Signature, trailer and meaning rules come from the unchanged FR-041 path, through `ResultLine::verify`. TC-408 proves that the `approval` member is byte-equal to `approval-verify`.
- `git-blob:<id>` is the id that `rev-parse <commit>:<path>` reports, and there is no digest ceremony.
- Every non-release reason is distinct. The FR-041 status is always the first reason. `released` holds for exactly one combination: the property test tc_415 runs the full fact product, and tc_419 checks precedence.
- `--end-of-options` comes before every user-supplied revision. Every other expression starts with a git-reported hex id, so a path can never be read as an option. Request `tag` and `record_path` values that start with `-` are refused before git starts (TC-409).
- Root commits fall back to `diff-tree --root`, so they are never a child. Tags that point at a tree or a blob, and tag names that do not exist, give `record_unreadable` with no `approval` member. Shallow boundaries can only hide parents, so they fail closed.
- Output is bounded by the FR-036 process limits: 8 MiB of output and a 60 s timeout.
- resolve_baseline reads the Baseline at the baseline commit through git objects. A repeated `path` gives `baseline_file_invalid`. An item counts as present when the path names any object, including a tree or a gitlink. That is acceptable under FR-046, which says only "test each item path with git".
- The inventory changes are truthful. TC-098 has 17 cases with the new `release-verify` case. The `release` feature builds and tests on its own and is listed in the lib.rs feature table and in `full`.
- The README stance sentence and the `release-verify` command-table row both land in this PR, which meets FR-050-AC-1 to AC-3. The maintainer-only statement is kept at README.md:303-305, so the coder's concern about it is moot.
- An unquoted all-digit `baseline.commit` is parsed as a YAML integer and refused as `record_invalid`. That fails closed, and a 40-hex id is all digits only about once in 10^8, so no change is needed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Replace refs and grafts are honored, so a hostile repository can change what release-verify reads under a valid signature. run_git reuses the FR-036 isolation, which removes GIT_REPLACE_REF_BASE but sets neither GIT_NO_REPLACE_OBJECTS nor GIT_GRAFT_FILE. Reproduced twice. (a) `git replace <record blob> <forged blob>` keeps the git-blob id the signed tag binds, but cat-file returns the forged record. Result: `released` with forged accepted_anomalies, and the same trick could change baseline.commit, snapshot_path or decision_owner. (b) A signed merge release commit (`-s ours`) correctly gives release_commit_not_child_of_baseline. After `git replace --graft R B`, or a `.git/info/grafts` line, it gives `released`, which violates FR-047's rule that a merge commit is never released. GIT_NO_REPLACE_OBJECTS alone does not disable grafts in git 2.34 (measured), so both settings are needed. Fix: set GIT_NO_REPLACE_OBJECTS=1 and GIT_GRAFT_FILE=<nonexistent path> in Git::run, which also hardens the FR-041 tag reads. Add both fixtures as tests, and amend FR-036 to name the setting. | src/signature_verify_host.rs:333 |
| FND-002 | medium | A git probe that fails after the record was read is reported as record_unreadable with no `approval` member (release_host.rs:358-362), even though the record was read and FR-041 already ran. That contradicts FR-047 Outputs, where `approval` is present whenever the record was readable and valid, and it reports an environment failure (timeout, 8 MiB cap, exit status) as a record defect. Separately, commit_shape ignores the exit status of `rev-list --parents` (lines 455-467). A failed probe yields no parents, which falls through to `diff-tree --root` and is misreported as not-child plus changes-other-paths. Both paths fail closed (never `released`), but the reasons are false. Fix: check the rev-list status, and give probe failure its own outcome, either a spec'd reason or an exit-2 machine error. That needs an FR-047 amendment. | src/release_host.rs:358 |
| FND-003 | low | changes() decodes diff-tree paths with String::from_utf8_lossy and compares the result with record_path and snapshot_path. A non-UTF-8 path whose lossy form equals a record or snapshot path containing U+FFFD is counted as that path, which hides a changed other path from release_commit_changes_other_paths. Fix: compare raw bytes with `path.as_bytes()`. | src/release_host.rs:414 |
| FND-004 | low | resolve_baseline starts one `git rev-parse` per Baseline item, with no bound on the item count. Each probe may take up to 60 s, so a large Baseline means N processes. One `git ls-tree -z <commit> -- <paths>` or one `cat-file --batch-check` call answers every item at once. | src/release_host.rs:206 |
| FND-005 | low | release::frontmatter is a third Markdown frontmatter splitter, beside onboarding.rs:480 split_frontmatter and manifest.rs:601 split_frontmatter. Reuse one shared splitter instead of adding another. | src/release.rs:36 |
| FND-006 | low | The git() wrapper rebuilds a std::process::Output from CompletedProcess only to drop stderr. Use CompletedProcess directly, which removes a conversion and a misleading type. | src/release_host.rs:110 |
| FND-007 | low | FR-046 places resolve_baseline in 'the release module' as 'a library function'. The code places it in the binary adapter (release_host.rs, pub(crate)), because the library is process-free and git runs only in the binary. The choice is sound. Amend FR-046 to say 'the release adapter' so that FR-048 and readers find it where it is. | src/release_host.rs:180 |
| FND-008 | low | The result carries members that FR-047 Outputs and the FR-049 data contract do not name: `absent_items`, which FR-047-AC-5 'naming the path' needs, `release_commit`, and a top-level `signature_kind` flattened from Approval. Each is harmless and additive, but the consumer contract is undocumented. List them in FR-047 Outputs, or drop `release_commit` and `signature_kind`. | src/release_host.rs:298 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | The one-call ls-tree that replaced the per-item probes (FND-004 fix) passes Baseline item paths as pathspecs without --literal-pathspecs. The schema admits ':' and '(' in a path, so a Baseline item ':(exclude)spec/a.md' makes ls-tree exit 128. release-verify then writes release_verify_git_failed at exit 2 instead of a verdict (reproduced at b02b94c). An item ':(top)a' is magic-parsed but still reported absent, because presence is compared on bytes. This fails closed, but Baseline content rather than git produces the machine error. Fix: add `--literal-pathspecs` (or GIT_LITERAL_PATHSPECS=1) to the ls-tree call. | src/release_host.rs:173 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (bff73e2). Git::run now sets GIT_NO_REPLACE_OBJECTS=1 and GIT_GRAFT_FILE=/nonexistent/grafts. Every git call of signature-verify, approval-verify, approval-trail and release-verify goes through that runner, including run_with_stdin. The replace.sh and graft.sh reproductions now give the true record and release_commit_not_child_of_baseline. TC-423 and TC-424 both fail with the two settings removed (mutation run). FR-036 and FR-047 state the rule. |
| FND-002 | fixed | Round 1. Git failure is the machine error release_verify_git_failed at exit 2 (FR-047 Outputs, Behavior and AC-13). Start, timeout and capture-bound failures map to it through run_git Err. rev-parse treats exit 1 as absent and any other status as a failure; rev-list, cat-file, ls-tree and diff-tree statuses are checked. A tag naming nothing or peeling to a tree still gives record_unreadable (rev-parse exit 1 measured). Approval presence is unchanged for every verdict. TC-425 covers it. |
| FND-003 | fixed | Round 1. Paths are compared as raw bytes; TC-426 commits a 0xFF path beside a U+FFFD snapshot path. |
| FND-004 | fixed | Round 1. A single ls-tree call tests every item, and resolve_baseline refuses more than 1024 items as baseline_file_invalid (FR-046 Behavior, AC-6, TC-427). Placing the bound in the resolution rather than the schema is acceptable, and the spec states it. The new call's pathspec handling is FND-009. |
| FND-005 | accepted-no-change | The three splitters live in separate features (release, onboarding, manifest) and differ in fence semantics: CRLF fences, an empty block, and a trailing fence at end of file. Sharing one would change onboarding or manifest parsing or add a cross-feature dependency. Keeping them separate is acceptable for a low duplication finding. |
| FND-006 | fixed | Round 1. git() returns CompletedProcess directly. |
| FND-007 | fixed | Round 1. FR-046 now says 'a function in the release adapter' and gives the reason. |
| FND-008 | fixed | Round 1. release_commit and the top-level signature_kind are gone. DecisionInfo carries only approver, meaning and the signing time. absent_items is in FR-047 Outputs and FR-049-AC-1. The FR-049 wording is gap-analysis SR-20261113124203 FND-005. |
