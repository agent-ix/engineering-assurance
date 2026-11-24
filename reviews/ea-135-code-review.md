---
id: SR-20261115040628
title: "code-review and rust-review of EA-135 gitsign identity registry implementation"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; src/signature_verify.rs; src/signature_verify_host.rs; src/approval_verify.rs; src/approval_verify_host.rs; src/approval_trail_host.rs; src/release_host.rs; tests/signature_verify.rs; tests/signature_verify_gitsign.rs; tests/signature_verify_cli.rs; tests/approval_verify.rs; tests/approval_verify_gitsign.rs; tests/approval_trail_gitsign.rs; tests/approval_tag_cli.rs; tests/release_verify_cli.rs"
review_set: subset
---

## Summary

Ticket: EA-135. PR agent-ix/engineering-assurance#301 at a291f14c5, diff `origin/main...HEAD`
on top of the merged spec PR #299. Code review with the Rust review lane folded in.

The gitsign person now comes only from an identity registry read as `<rev>:<path>`.
`GitsignIdentity` no longer carries a principal. `SignerSource::Request` was renamed to
`GitsignRegistry` everywhere. No compatibility path is left: no `request` source, no
principal fallback, and no stale gitsign `principal` in README, docs or schemas. A
`principal` member is accepted by the parser (any JSON type, through `IgnoredAny`) only so
it can be refused. It is `skip_serializing`, so it is never echoed.

The registry is read by git object only. The code resolves `<rev>^{commit}`, lists the path
with `--literal-pathspecs ls-tree -z --full-tree <commit> -- <path>`, accepts only a 100644
or 100755 blob, checks `cat-file -s` against 1 MiB, then runs `cat-file blob <id>`. The git
runner already sets `GIT_NO_REPLACE_OBJECTS=1` and a dead `GIT_GRAFT_FILE`. Nothing reads the
working tree or the index.

`git_failed` is kept separate from `invalid` in all four capabilities, as FR-037 requires:
- `signature-verify` and `approval-verify` return `unverifiable` at exit 0;
- `approval-trail` returns `approval_trail_repository_unavailable`;
- `release-verify` returns `release_verify_git_failed`.

`identity_registry_commit` is attached on every verdict path once the revision resolved
(commit, tag, lightweight tag, unresolved target, SSH) and is absent on `git_failed`.

Error codes and exit status 2 match FR-040, FR-041, FR-043 and FR-047. Validation order
matches FR-040 Behavior:
1. `trailer_keys`;
2. request shape;
3. `gitsign` and `identity_registry` validation;
4. principal refusal;
5. allowed-signers;
6. the registry read;
7. the verifier.

`approval_verify::decide` puts `signed_valid` + `unmapped` as `untrusted_signer` ahead of the
`trailer_unreadable`, binding and meaning rules, as FR-041 rule 1 orders them.

Mutation checks (each applied, run with focused tests, then reverted; tree clean at a291f14c5):

| Mutation | Result |
| --- | --- |
| Unmapped signer mapped to the identity | killed: tc_1041 (pure and CLI), tc_329, tc_362 |
| Principal refusal removed | killed: tc_1042, tc_327, tc_361, tc_1048 |
| Duplicate identity and issuer accepted | killed: tc_1043 (pure and CLI) |
| Registry read from the working tree | killed: tc_1044 |
| `git_failed` classified as invalid | killed: tc_1046, tc_325, tc_361, tc_1048 |
| Registry commit emitted on a git failure | killed: tc_1046, tc_325 |
| 1 MiB size check removed | killed: tc_1043 |
| Symbolic link (mode 120000) accepted | **survived** (FND-002) |
| Non-UTF-8 content decoded lossily | **survived** (FND-003) |

The coder's open questions:

- **Is `identity_registry` echoed in the `signature-verify` result?** FR-040 Outputs add only
  `identity_registry_commit` to the verdict. The `trust` echo belongs to FR-041, FR-043 and
  FR-080. Not echoing it in `signature-verify` is correct, so there is no finding.
- **Order of the allowed-signers and registry checks.** FR-037 refuses the allowed-signers
  file "before git is started", and the registry read starts git. Allowed-signers must
  therefore come first, which the code does. There is no finding.

The four `source_audit` failures. Each was run once with the assertions temporarily turned
into a report of every file, then reverted:

- **tc_653.** Offline cargo cannot resolve the full dependency tree. This is environmental.
  The PR adds no dependency.
- **tc_101.** The only hit is `src/receipt_record.rs` (Clock). `src/signature_verify.rs` is
  clean.
- **tc_127.** The only hit is `src/receipt_host.rs`.
- **tc_117.** This one already fails on main: `is_test_case_id` takes exactly three digits,
  and main already has four-digit tags such as TC-1003, TC-1008 and TC-1035, including one in
  `src/signature_verify_host.rs`. The PR makes it worse. Two previously clean files,
  `tests/signature_verify.rs` and `tests/release_verify_cli.rs`, and several new tests in
  `tests/signature_verify_gitsign.rs` now trip it (FND-001).

The PR causes none of these four failures.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | tc_117's `is_test_case_id` rejects four-digit test-case ids. The new trace tags TC-1037..TC-1048 are flagged `TraceTestCaseMissing` in `tests/signature_verify.rs` (tc_1038, tc_1041, tc_1043), `tests/release_verify_cli.rs` (tc_1048) and the new tests in `tests/signature_verify_gitsign.rs`. The cause is already on main, but this PR adds two newly failing files. Minimal fix: accept `TC-` followed by three or more ASCII digits, in place of `value.len() == 6`. | src/source_audit.rs:749-753 |
| FND-002 | medium | The FR-040-AC-27 / FR-037-AC-20 symbolic-link case is vacuous. The link blob's content is `elsewhere.json`, which is not JSON, so the registry is refused as unparsable whatever the mode check does. Accepting mode 120000 in `regular_file_object` still passes tc_1043. Fix: make the symlink entry's blob a valid registry document (for example `default_registry()`). | tests/signature_verify_gitsign.rs:1530-1537; src/signature_verify_host.rs:1374-1386 |
| FND-003 | medium | The FR-040-AC-27 / FR-037-AC-20 non-UTF-8 case is vacuous. The content `ff fe 7b 7d` is invalid JSON under any decoding, so replacing the `String::from_utf8` refusal with `from_utf8_lossy` still passes tc_1043. Fix: use a valid registry document whose principal string holds an invalid UTF-8 byte, which a lossy decode would accept. | tests/signature_verify_gitsign.rs:1568-1569; src/signature_verify_host.rs:1018 |
| FND-004 | low | The FR-040-AC-30 timeout case waits the real 60 s process timeout. It is deterministic: the wrapper `exec`s `sleep`, so the kill hits it, and it passed in 60.14 s. It is acceptable, but it is the only real-time timeout wait in the suite, and it takes `signature_verify_gitsign` from about 2 s to 60 s. It also sits behind three fast cases in one test fn. Move it into its own `#[test]` so the fast cases report independently. Do not add a production knob to shorten it. | tests/signature_verify_gitsign.rs:1798-1805 |
| FND-005 | low | Rust idiom: the `GitsignInputError` to error-code mapping is written four times. `approval_verify_host` has a `gitsign_error` helper, but `signature_verify_host`, `approval_trail_host` and `release_host` repeat inline closures. `load_registry` spells `engineering_assurance::signature_verify::Reason` in full instead of importing it. | src/approval_verify_host.rs:238-264; src/signature_verify_host.rs:301-304; src/approval_trail_host.rs:197-200; src/release_host.rs:468-471 |

## Verdict

No high finding. The behaviour matches FR-037, FR-040, FR-041, FR-042 AC-2, FR-043 and FR-047,
and the breaking rename is complete with no compatibility path. Not mergeable as is: fix
FND-001 to FND-003 in one fix round, as small test and audit edits. FND-004 and FND-005 are
optional.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1 (dd08bdb40): `is_test_case_id` now accepts `TC-` followed by three or more ASCII digits. The new test `tc_117_a_test_case_id_is_tc_and_three_or_more_digits` covers TC-117, TC-1035 and TC-12345, and rejects TC-12, TC-, TC-12a, full-width digits and lower case. `tc_117_every_first_party_rust_test_uses_canonical_trace_syntax` now passes. |
| FND-002 | fixed | Round 1: the symlink entry's blob is now `default_registry()`. Re-running the mutation that accepts mode 120000 in `regular_file_object` now fails tc_1043 (exit 0, expected 2). |
| FND-003 | fixed | Round 1: the non-UTF-8 fixture is a valid registry with `ff fe` inside the principal. Re-running the mutation that replaces `String::from_utf8` with `from_utf8_lossy` now fails tc_1043 (exit 0, expected 2). |
| FND-004 | fixed | Round 1: the real-timeout case is its own test, `tc_1046_a_git_that_times_out_while_reading_the_registry_is_unverifiable`, with no production knob. It passed in 60.16 s. |
| FND-005 | fixed | Round 1: one shared `GitsignInputError::refusal(invalid, principal)` is used by all four hosts. `Reason` is imported in `approval_verify_host`. |
