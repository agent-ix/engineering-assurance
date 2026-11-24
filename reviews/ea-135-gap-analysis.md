---
id: SR-20261115040629
title: "gap-analysis of EA-135 gitsign identity registry implementation"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance; spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md; spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md; spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md; spec/functional/FR-042-create-signed-approval-tag-for-a-change-assurance-record.md; spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md; spec/functional/FR-047-release-record-and-signed-release-tag.md; spec/functional/FR-080-signed-proof-integrity-statement-verification.md; spec/tests.md; src/; tests/"
review_set: subset
---

## Summary

Ticket: EA-135. PR agent-ix/engineering-assurance#301 at a291f14c5. This is a planless gap
analysis (plan completion: not assessed) over the computed matrix (`quoin matrix --json`,
quoin 0.36.8), the tests the diff adds or changes, and the source.

Every criterion in scope is tagged:

- FR-037-AC-19..22;
- FR-040-AC-1, 3, 11, 12 and 25..31;
- FR-041-AC-8, 10, 14 and 25;
- FR-042-AC-2;
- FR-043-AC-10 and 11;
- FR-047-AC-14.

The rows TC-310, TC-325 and TC-1037..TC-1048 each have tests. TC-1026 (FR-080-AC-5) is
untagged and marked "planned, not executed". `proof-checkpoint-verify` has no host in `src/`,
so the FR-080 amendment is spec-only, and that is not a gap in this PR.

No stub, `todo!`, `unimplemented!` or unreachable branch was found in the new code. Every new
function is reached from a capability entry point. No code lacks an owning requirement.

Mutation checks are listed in the companion code-review artifact (SR-20261115040628). The
important behaviours each have a test that fails when the behaviour is removed, except the
symbolic-link and non-UTF-8 registry cases (FR-040-AC-27 / FR-037-AC-20), which are recorded
there. The FR-040-AC-30 timeout case uses a real 60 s wait. It is deterministic and was
observed passing in 60.14 s.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-041-AC-25 was amended by the spec PR to cover moving the branch the `identity_registry` revision names, and editing the working-tree registry, after capture through `approval-verify`. Its bound tests (tc_293, tc_338) predate the registry, and `tests/approval_verify_cli.rs` never mentions it. The equivalent `signature-verify` behaviour is tested (tc_1044, tc_1047), but no `approval-verify` test proves the approval verdict uses the registry bytes captured at the reading. | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:265; tests/approval_verify_cli.rs:1017 |
| FND-002 | low | FR-037-AC-19 says a graft naming the commit changes neither the commit nor the content. tc_1044 exercises working-tree edits, index edits, a replace ref and a moved branch, but no graft. No trust-source test writes `info/grafts`. The behaviour rests on the shared runner's `GIT_GRAFT_FILE` setting, tested only for `release-history`. | spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:189; tests/signature_verify_gitsign.rs:1692-1728 |
| FND-003 | low | FR-040-AC-27 requires that an unusable registry starts no signature verifier for an SSH-signed object as well. tc_1043 asserts only the gitsign stand-in's call count (`ssh.calls() == 0`). It never observes that `git verify-commit`, `git verify-tag` or `ssh-keygen` did not run for the SSH half. | spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md:350; tests/signature_verify_gitsign.rs:1672-1687 |
| FND-004 | low | FR-040-AC-29 says the commit author, committer, tagger, email and certificate text change nothing. tc_1045 varies only the commit author, committer and message, and never a tag's tagger or the certificate text. | spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md:352; tests/signature_verify_gitsign.rs:1731-1763 |

## Verdict

The trace coverage is complete for the ACs this PR implements. One AC branch is untested in a
way that matters (FND-001, medium). Three narrower gaps are low. Together with the
code-review findings, this needs one fix round. Nothing found is a behavioural defect.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (dd08bdb40): `tc_1011_the_registry_captured_before_verification_survives_a_moved_branch_and_an_edited_worktree` in tests/approval_verify_gitsign.rs is bound to FR-041-AC-25. While gitsign runs, the stand-in moves `refs/heads/registry` to an empty registry and blanks the working-tree `registry.json`. The result stays `approved` with the captured principal, and `trust.identity_registry_commit` equals the commit resolved at capture. |
| FND-002 | still-open | The graft added to tc_1044 cannot change the result. It grafts new parents onto `committed`, but the address is `main:registry.json`, which never follows parents. The commit and its tree are the same whether or not grafts are honoured. Removing `GIT_GRAFT_FILE` from the git runner still passes tc_1044 (mutation survived). For the graft to bite, the address must reach the content through ancestry: for example `main~1:registry.json` (or `main^`), with a graft that redirects that parent to the decoy. |
| FND-003 | fixed | Round 1: `ssh_signed_fixture` now records `git` and `ssh-keygen` through wrappers. tc_1043 asserts that no `verify-commit` or `verify-tag` appears in the git log and that `ssh-keygen` never ran, for every unusable-registry case on SSH objects. |
| FND-004 | fixed | Round 1: tc_1045 adds three CMS tags verified against `first:registry.json`: a different tagger name, email and time, and different certificate bytes in the signature block. Each still maps to "First Person". |
