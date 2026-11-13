---
id: FR-042
title: "Create a signed annotated tag that approves a change-assurance record"
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
  - target: "ix://agent-ix/engineering-assurance/FR-041"
    type: "references"
---

# FR-042: Create a signed annotated tag that approves a change-assurance record

## Description

Engineering Assurance SHALL provide an `approval-tag` command that writes one
signed annotated git tag in the form that `approval-verify`
([FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md))
accepts as a person's approval of one change-assurance record. Writing that
message by hand is error-prone: the trailers must follow a subject paragraph,
carry the record id and digest byte for byte, and carry a meaning. The command
composes the message, and git signs the tag.

The command never signs on anyone's behalf. It runs `git tag -s` and passes no
key, program or format, so git signs with the key and the program that the
caller's own git configuration names: an SSH key (`gpg.format=ssh`) or
`gitsign` (`gpg.format=x509` with `gpg.x509.program` set; `gitsign` is the x509
program FR-040 verifies, and a tag signed by another x509 program is created but
reads `unverifiable` under `approval-verify`). The signer must be
non-interactive (an agent-held or unlocked key, or a program that needs no
terminal): the command gives git no terminal and no passphrase prompt can be
answered. It adds no cryptography, never pushes, never writes git configuration,
and never replaces a tag; the only ref it deletes is the unsigned tag it has
just written, as the Behavior states. Who may approve, and which meaning suffices, are not decided here.

A record is resolved from a record file the caller names, and its digest is an
input. The command reads the file only to confirm it is the change-assurance
record the request names: a JSON object of the current record schema version
(`schema_version` 2) whose `record_type` is `change_assurance`, whose
`record_id` equals the request's and whose `review_workflow` holds a
`signed_approval` object. The record holds no digest member. The caller obtains
the record digest from the record's owner (Quoin's `record-digest` command,
which is `sha256:` plus the lowercase hexadecimal SHA-256 of the exact stored
bytes of the revision) and supplies it as `record_digest`; the command copies it
verbatim into the tag. Engineering Assurance has no hashing and never
computes, checks or compares the digest, so a wrong digest is not detected here.
It is caught when the record's owner verifies the approval, which compares the
claimed digest with the stored bytes and does not approve a mismatch: the tag
then simply fails to approve.

Out of scope: pushing the tag, choosing or creating a signing key, computing
or checking a record digest, recognising a stale or wrong digest, deciding who may approve, signing with OpenPGP
(FR-036 does not verify it), and verifying the tag (FR-041 does).

## Inputs

- One request document `engineering-assurance.approval-tag-request/v1` with the
  members `protocol`, `repository` (absolute directory path), `tag` (a bare tag
  name that does not start with `refs/`), `revision` (optional, default
  `HEAD`; the commit the tag points to), `record` (absolute path of the record
  file), `record_id`, `record_digest` and `meaning`. Unknown members are rejected.
- `tag` and `revision` follow the revision bounds of FR-036 (non-empty, at most
  256 bytes, not option-like, no whitespace or control character). `record_id` and `record_digest`
  follow the bounds of FR-041, `record_digest` is further `sha256:` followed by 64
  lowercase hexadecimal characters, and `record_id`, as `meaning` does, must read
  back unchanged from a trailer: it has no leading or trailing ASCII
  whitespace. `meaning` is the `Signed-off-meaning` value of FR-037: non-empty,
  at most 256 bytes, no control character. There is no default meaning.
- The caller's git configuration, read by git itself: `gpg.format`,
  `user.signingkey` and `gpg.ssh.defaultKeyCommand`, from the system, global and
  repository files, with the environment the command is given.

## Outputs

- On success, one line of JSON on stdout and exit status 0: `protocol` equal to
  `engineering-assurance.approval-tag-result/v1`, `status` equal to `created`,
  `tag`, `tag_object` (the id of the new tag object), `target` (the commit id the
  tag points to), `record_id`, `record_digest` (as requested) and `meaning`.
- The tag message is a subject paragraph, then the trailers
  `Approved-Record-Id`, `Approved-Record-Digest` and `Signed-off-meaning`, in
  that order, with the names FR-041 and FR-037 fix.
- A request that is refused writes the `engineering-assurance.error/v1` machine
  error for capability `approval-tag` to stdout and its message to stderr, exits
  with status 2, and leaves the repository without the tag. The codes are
  `approval_tag_request_unreadable`, `approval_tag_request_invalid`,
  `approval_tag_record_unresolved`, `approval_tag_repository_unavailable`,
  `approval_tag_tag_invalid`, `approval_tag_tag_exists`,
  `approval_tag_revision_unresolved`, `approval_tag_signing_unconfigured`,
  `approval_tag_create_failed`, `approval_tag_local_io_failed`, `approval_tag_git_unavailable`,
  `approval_tag_git_failed` and `approval_tag_result_encoding_failed`.

## Behavior

- The command SHALL resolve the record from `record` and `record_id`: the file
  is a regular file of at most 8 MiB holding a JSON object whose
  `schema_version` is 2, whose `record_type` is `change_assurance`, whose
  `record_id` equals the request's and whose `review_workflow.signed_approval`
  is an object; any other file, including a schema 1 record, yields
  `approval_tag_record_unresolved`.
- The command SHALL copy `record_digest` verbatim into `Approved-Record-Digest`
  and SHALL NOT compute, check or compare it with the record.
- The command SHALL refuse a `meaning` that is empty, over-long, holds a control
  character or would not read back unchanged, and SHALL NOT supply a meaning of
  its own.
- The command SHALL compose the message as a pure function with no I/O, as a
  subject paragraph, a blank line and the three trailers, one per line.
- The command SHALL decide whether git can sign a verifiable tag from the
  values git reports for `git config --get`, and SHALL NOT read configuration
  files itself. `gpg.format` unset, empty or any value other than `ssh` or
  `x509` is refused naming `gpg.format`; with `ssh`, a `user.signingkey` and a
  `gpg.ssh.defaultKeyCommand` that are both unset or empty is refused naming
  `user.signingkey`; with `x509`, `gpg.x509.program` unset or empty is refused naming
  `gpg.x509.program` and no key setting is required. A refusal is
  `approval_tag_signing_unconfigured`, its message names the setting, and no tag
  is written.
- The command SHALL create the tag with `git tag -s`, `--cleanup=verbatim`, the
  message from a file, and `--` before the tag name and the target commit id.
- The command SHALL NOT pass `-f`, `-u`, `-m` with the message, a signing-program
  option or `-c` to git.
- The command SHALL refuse a tag name that `git check-ref-format` rejects for
  `refs/tags/<name>`, and a name that `git rev-parse --verify` already resolves
  as a revision (such as `HEAD`, `@` or `heads/<branch>`), which
  `approval-verify` could not resolve to the new tag (`approval_tag_tag_invalid`);
  a tag name that already
  exists (`approval_tag_tag_exists`), and a revision that does not resolve to a
  commit (`approval_tag_revision_unresolved`), each without writing a tag.
- The command SHALL report `approval_tag_create_failed` when `git tag -s` exits
  nonzero, is killed or times out, and SHALL NOT classify git's or a signing
  program's message text.
- The command SHALL, after `git tag -s` succeeds, read the tag object with
  `git cat-file tag` and require a signature block of a kind FR-036 splits as SSH
  or X.509, because git 2.34 exits 0 and writes an unsigned tag when an SSH
  signature fails (a missing key file, a locked key with no agent).
- The command SHALL report `approval_tag_create_failed`, and no `created`
  result, for a tag object without such a block.
- The command SHALL remove a tag it wrote that is unsigned, or whose creation
  failed after the ref update, with `git update-ref -d refs/tags/<name> <id>`
  naming the id it read (compare-and-delete), and SHALL delete no other ref.
- The command SHALL report a failure to prepare its message file as
  `approval_tag_local_io_failed`.
- The command SHALL run only `git rev-parse`, `git check-ref-format`,
  `git config --get`, `git tag`, `git cat-file tag` and `git update-ref -d` of
  the ref it wrote, and SHALL NOT run `git push`, `git config`
  with a write, or any other program itself.
- The command SHALL find `git` only in the absolute entries of `PATH`, never in
  the repository, and SHALL give git that same cut `PATH`, so the signing program
  git starts is found there only.
- The command SHALL run git in the repository with the caller's configuration,
  unlike the verification commands, because that configuration is the signing
  configuration; it SHALL remove the repository-selecting and
  configuration-injecting environment variables FR-036 removes.
- The command SHALL run git through the process host of FR-019 with a 60 second
  wall-clock timeout and an 8 MiB capture bound, and the signer SHALL be
  non-interactive.
- The command SHALL pass a tag name, revision and target to git only after the
  option-like and whitespace checks, after `--` or `--end-of-options`.
- The command SHALL contain no signature, certificate or trailer-parsing code,
  and no cryptography.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-042-CON-1 | The command SHALL create a signature only through `git tag -s` under the caller's git configuration, and never pushes, writes git configuration, replaces a tag or deletes any ref except the one tag ref this run just wrote and found unsigned or failed. | Safety | Test |
| FR-042-CON-2 | The command SHALL contain no cryptography, no signature or certificate code and no digest computation or check. | Architecture | Inspection |

## Acceptance Criteria

Stand-in note: where `gitsign` is not installed, the gitsign criteria use the
FR-040 stand-in as the verifier and a stand-in signing program, configured as
`gpg.x509.program`, that emits a CMS-shaped signature block; the SSH criteria
use real signatures from throwaway keys generated in the test.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-042-AC-1 | With `gpg.format=ssh` and a throwaway `user.signingkey`, a request creates an annotated tag that `approval-verify` reports as `approved` for the same record id and the requested `record_digest`, which appears verbatim in the tag, with `approver` equal to the allowed-signers principal of that key, `meaning` equal to the requested meaning and `signature_kind` `ssh`; the command prints one `created` JSON line at exit 0. | Test |
| FR-042-AC-2 | With `gpg.format=x509` and a stand-in signing program, the created tag is reported `approved` by `approval-verify` with the FR-040 stand-in `gitsign` accepting the identity, `signature_kind` `gitsign` and `approver` equal to the request's `gitsign.principal`; the stand-in signing program was started by git with the configured settings. | Test |
| FR-042-AC-3 | The tag message is a subject paragraph followed by exactly `Approved-Record-Id`, `Approved-Record-Digest` and `Signed-off-meaning` trailers in that order, and the trailer reader of FR-037 reads back the record id, the digest and the meaning exactly. | Test |
| FR-042-AC-4 | A request with no `meaning`, an empty one, one with a leading or trailing space, a control character or over 256 bytes writes `approval_tag_request_invalid`, exits 2 and writes no tag, and a created tag carries the requested meaning and no other `Signed-off-meaning`. | Test |
| FR-042-AC-5 | The tag is signed by the key the repository's git configuration names and by no other: with two keys, switching `user.signingkey` changes the signer, and a run with a configured remote leaves the remote without the tag, the configuration files unchanged and no ref other than the new tag changed. | Test |
| FR-042-AC-6 | With `gpg.format` unset, `openpgp`, `ssh` without `user.signingkey`, or `x509` without `gpg.x509.program`, the command writes `approval_tag_signing_unconfigured`, its message names `gpg.format`, `user.signingkey` or `gpg.x509.program` respectively, exits 2 and writes no tag. | Test |
| FR-042-AC-7 | A record file that is missing, not a regular file, over 8 MiB, not JSON, not an object, of `schema_version` other than 2, of another `record_type`, of another `record_id`, or without a `review_workflow.signed_approval` object yields `approval_tag_record_unresolved` at exit 2 and no tag; a `record_digest` that is not `sha256:` and 64 lowercase hex characters yields `approval_tag_request_invalid` and no tag. | Test |
| FR-042-AC-8 | An invalid tag name, a name starting with `refs/`, a name that already resolves as a revision (`HEAD`, `@`, `heads/main`, a branch name), an existing tag name (whose object is unchanged afterwards) and an unresolvable revision are refused with their codes at exit 2 and no new tag. | Test |
| FR-042-AC-9 | A request with an unknown member, a wrong protocol, a relative `repository` or `record`, an option-like or spaced `tag` or `revision`, or a bad `record_id` or `record_digest` writes `approval_tag_request_invalid` and exits 2 without starting git; unreadable JSON writes `approval_tag_request_unreadable`. | Test |
| FR-042-AC-10 | A relative `PATH` entry holding a `git` or a signing program inside the repository is never run, and with only relative entries the command writes `approval_tag_git_unavailable` and no tag. | Test |
| FR-042-AC-11 | A signing program that exits nonzero, and an SSH signer that fails while git still exits 0 (a missing key file, a passphrase-protected key with no agent), each yield `approval_tag_create_failed` at exit 2 with the tag ref absent afterwards and no other ref changed. | Test |
| FR-042-AC-12 | The signing decision is exercised for every combination of `gpg.format`, `user.signingkey` and `gpg.ssh.defaultKeyCommand` without starting a program, and the message composition and meaning check are pure. | Test |
| FR-042-AC-13 | Static inspection shows no cryptography, signature, certificate or digest code in the approval-tag modules, only the git subcommands the Behavior lists, and no `-f`, `-u` or push (CON-1, CON-2). | Inspection |

## Dependencies

- **Upstream**: [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md)
  (the trailer names and the form it verifies),
  [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md),
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
  [FR-040](./FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md).
- **Downstream**: an approver, or a tool acting for one, who needs one command to
  produce a tag `approval-verify` accepts.
