---
id: FR-041
title: "Verify a signed annotated tag as a named person's approval of an exact change-assurance record"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-036"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-037"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-040"
    type: "references"
---

# FR-041: Verify a signed annotated tag as a named person's approval of an exact change-assurance record

## Description

Engineering Assurance SHALL provide an `approval-verify` command that reports
whether a signed annotated git tag is a named person's approval of one exact
change-assurance record. The tag message carries the record id and the record
digest as trailers, so the signature binds the signer to the record content
the digest names. The command adds no cryptography and no signature rules: it
runs the signature verification of FR-036, FR-037 and FR-040 on the tag and
adds only the record binding and the approval meaning. A consumer such as Quoin
retains the result as data and never re-verifies it.

An annotated tag is the approval object because a squash merge replaces a
commit a person signed with a commit the hosting service signed, so a commit
signature cannot carry an approval through a squash. A tag keeps pointing at the
object it names after a squash. A signed commit, a merge commit, a git note and
a lightweight tag are not approvals.

The binding is generic: it names a record by an opaque id and an opaque digest
string, and reads nothing from the record. Any record kind whose consumer
computes a digest can reuse it.

Out of scope: computing the record digest (the caller supplies the digest the
record's owner defines), creating or pushing the tag, deciding who may approve
or which meaning suffices (FR-037 exposes the meaning as recorded text only),
signature rules of any kind (FR-036, FR-037, FR-040 own them), and approvals
carried by commits, merge commits or notes.

## Inputs

- One request document `engineering-assurance.approval-verify-request/v1` with
  the members `protocol`, `repository` (absolute directory path), `tag` (a bare
  tag name or `refs/tags/<name>`, as the tag `revision` of FR-040),
  `record_id` and `record_digest` (non-empty strings of at most 256 bytes with
  no control character and no leading `-`), `allowed_signers` (absolute file
  path, required, as FR-036 and FR-037), and optionally `gitsign` (the object
  FR-040 defines, with the same bounds). Unknown members are rejected.
- Trailer names, fixed by this requirement: `Approved-Record-Id` and
  `Approved-Record-Digest`, read from the tag message. The approval meaning is
  the `Signed-off-meaning` trailer of FR-037.

## Outputs

- One line of JSON on stdout, the result with the member `protocol` equal to
  `engineering-assurance.approval-verify-result/v1`, and exit status 0 for every
  status, as FR-036 does. The members are:
  - `status`, one of `approved`, `wrong_record`, `untrusted_signer`,
    `meaning_missing`, `signed_bad`, `unsigned`, `unverifiable`;
  - `tag`, the tag name the command resolved, and `record_id` and
    `record_digest` as requested;
  - `signature`, the FR-036 verdict for the tag exactly as FR-036, FR-037 and
    FR-040 define it, without a second `protocol` member;
  - on `approved` only: `approver`, the `principal` of the FR-037 signer;
    `meaning`, the array of `Signed-off-meaning` values in order; and
    `signature_kind`, `ssh` or `gitsign` as FR-040 defines it;
  - `binding`, when the tag message was readable: `record_id` and
    `record_digest`, each one of `match`, `mismatch`, `missing`, `repeated`.
- `status` is decided in this order, the first rule that applies winning:
  1. the FR-036 verdict `unsigned` (including a lightweight tag with reason
     `lightweight_tag`) is `unsigned`; `signed_bad` is `signed_bad`;
     `unverifiable` is `unverifiable`; `signed_untrusted` (a good signature by a
     key or identity the supplied trust inputs do not accept, so the signer is
     `unmapped`) is `untrusted_signer`;
  2. `signed_valid` with `binding` other than `match` for either member is
     `wrong_record`;
  3. `signed_valid`, both bindings `match`, and `signed_off_meaning` `missing`
     is `meaning_missing`;
  4. otherwise `approved`.
- A request that yields no verdict writes the `engineering-assurance.error/v1`
  machine error for capability `approval-verify` to stdout and its message to
  stderr, and exits with status 2. The codes are
  `approval_verify_request_unreadable`, `approval_verify_request_invalid` and
  `approval_verify_allowed_signers_invalid`.

## Behavior

- The command SHALL obtain the `signature` verdict by the signature-verify
  verification of FR-036, FR-037 and FR-040 with kind `tag`, the request's
  `tag` as the revision, and the request's `allowed_signers` and `gitsign`,
  and SHALL NOT restate, relax or re-implement any rule they define.
- The command SHALL report a lightweight tag as `unsigned` and a revision that
  does not resolve to a tag as `unverifiable`, as FR-040 defines, and SHALL
  accept no commit, merge commit or note as the approval object.
- The command SHALL read the two record trailers from the tag message with the
  semantics of `git interpret-trailers --parse` and the same preparation
  FR-037 states for the tag message (signature block removed, no patch
  divider), and SHALL compare each value to the request by exact byte
  equality, with no case folding, trimming beyond trailer parsing, or
  normalization.
- The command SHALL report a binding `missing` when its trailer is absent,
  `repeated` when it occurs more than once (no occurrence is then preferred),
  `mismatch` when its single value differs from the request, and `match`
  otherwise.
- The command SHALL derive `approver` only from the FR-037 `principal`, and
  SHALL NOT derive a person from the tag's tagger, the commit author, an email
  or a key comment.
- The command SHALL report the binding for every status with a readable tag
  message, and the binding SHALL NOT change a status decided by rule 1.
- The command SHALL NOT default a missing trailer, and SHALL NOT report
  `approved` for a tag whose message lacks `Signed-off-meaning`.
- The command SHALL NOT interpret the digest, read the record, or decide what
  a meaning permits.
- The decision of `status` from the verdict and the binding SHALL be a pure
  function with no I/O; the git and trust-tool calls stay in the
  signature-verify adapters.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-041-CON-1 | A result SHALL NOT be `approved` unless the FR-036 verdict for the tag is `signed_valid` with a mapped signer, the tag message carries exactly one `Approved-Record-Id` equal to the request's `record_id`, exactly one `Approved-Record-Digest` equal to the request's `record_digest`, and a `Signed-off-meaning` trailer. | Safety | Test |
| FR-041-CON-2 | The command SHALL contain no signature, certificate or trailer-parsing code of its own and SHALL NOT duplicate a rule of FR-036, FR-037 or FR-040. | Architecture | Inspection |

## Acceptance Criteria

Stand-in note: where `gitsign` is not installed, criteria that need it are
verified with the FR-040 stand-in executable, as FR-040 states; the SSH
criteria use real signatures from throwaway keys generated in the test.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-041-AC-1 | An annotated tag validly signed by a key the allowed-signers file maps to a principal, whose message carries one `Approved-Record-Id` and one `Approved-Record-Digest` equal to the request and a `Signed-off-meaning` trailer, yields `approved` with `approver` equal to that principal, `meaning` equal to the trailer values in order, `signature_kind` `ssh` and `tag` equal to the tag name. | Test |
| FR-041-AC-2 | A valid, mapped tag whose `Approved-Record-Digest` differs from the request by one character yields `wrong_record` with `binding.record_digest` `mismatch`, and no `approver`. | Test |
| FR-041-AC-3 | A valid, mapped tag whose `Approved-Record-Id` differs from the request yields `wrong_record` with `binding.record_id` `mismatch`; a tag lacking either trailer yields `wrong_record` with that binding `missing`; a tag repeating either trailer yields `wrong_record` with that binding `repeated`, even when one of the repeated values equals the request. | Test |
| FR-041-AC-4 | A tag with a good signature by a key the allowed-signers file does not name, whose message carries matching trailers, yields `untrusted_signer`, no `approver`, and a `signature` verdict of `signed_untrusted` with mapping `unmapped`. | Test |
| FR-041-AC-5 | A valid, mapped tag with matching record trailers and no `Signed-off-meaning` trailer yields `meaning_missing` and no `meaning` default; the same tag with an empty-valued trailer yields `approved` with the empty value. | Test |
| FR-041-AC-6 | A tag whose signature does not verify yields `signed_bad`; an unsigned annotated tag yields `unsigned`; a lightweight tag yields `unsigned` with `signature` reason `lightweight_tag`; a name resolving to no tag, or to a branch or commit, yields `unverifiable`; none of them yields `approved` even when the message carries matching trailers. | Test |
| FR-041-AC-7 | Record trailers placed in the tagged commit's message but not in the tag message do not bind the record, and a signed commit, a merge commit and a git note naming the record are each refused as an approval (they are not tags). | Test |
| FR-041-AC-8 | A gitsign-signed tag with a `gitsign` member whose identity and issuer gitsign accepts, and matching trailers, yields `approved` with `signature_kind` `gitsign` and `approver` equal to the request's `gitsign.principal`; a tag whose identity gitsign does not accept yields `untrusted_signer`; a CMS-signed tag with no `gitsign` member yields `unverifiable`. | Test |
| FR-041-AC-9 | The decision from the FR-036 verdict and the binding is exercised without starting any process, covering every combination of verdict status, binding member and meaning presence, and `approved` is returned for exactly one combination (CON-1). | Property |
| FR-041-AC-10 | A request with an unknown member, an empty or over-long `record_id` or `record_digest`, a leading `-`, a control character, a relative `repository` or `allowed_signers` path, or an invalid `gitsign` member writes `approval_verify_request_invalid` and exits 2; an invalid allowed-signers file writes `approval_verify_allowed_signers_invalid` and exits 2; neither starts git. | Test |
| FR-041-AC-11 | Every status other than a request error exits 0 and writes one JSON line carrying the result protocol, and a validly signed tag whose trailers are unrelated to the request still carries the unchanged `signature` verdict that `signature-verify` yields for the same tag. | Test |
| FR-041-AC-12 | Static inspection shows no signature, certificate or trailer-parsing code in the approval module and no restated FR-036, FR-037 or FR-040 rule (CON-2). | Inspection |

## Dependencies

- **Upstream**: [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md),
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
  [FR-040](./FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md).
- **Downstream**: consumers that retain a verified approval as data; Quoin's
  change-assurance verification is the first.
