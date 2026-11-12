---
id: FR-036
title: "Verify SSH signatures on commits and annotated tags by delegating to git"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-028"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-032"
    type: "references"
---

# FR-036: Verify SSH signatures on commits and annotated tags by delegating to git

## Description

Engineering Assurance SHALL report whether a commit or an annotated tag carries
a valid SSH signature, by running `git verify-commit` or `git verify-tag` and
classifying what git reports. It adds no cryptography of its own: git decides
whether a signature is valid, and Engineering Assurance only classifies the
outcome, names the signing key as git reports it, and refuses to guess when git
cannot answer. A consumer that records a signed approval or release reads the
result as data and does not re-verify.

Out of scope: gitsign and Sigstore keyless signing (`git verify-commit` does
not check the signer identity for them, so a later requirement will define
them), X.509 and OpenPGP signatures, and any policy about squash merges or
which signed commits count as an approval.

## Inputs

- For the pure classifier: a target kind (`commit` or `tag`) and the observed
  git outcome, being the exit status, the captured status text, and flags
  saying the output was cut short or the process timed out.
- For the `signature-verify` command, one request document
  `engineering-assurance.signature-verify-request/v1` with the members
  `protocol`, `repository` (absolute directory path), `revision`, `kind`
  (`commit` or `tag`) and, optionally, `allowed_signers` (absolute file path,
  see FR-037).

## Outputs

- A verdict with `status` (`signed_valid`, `signed_bad`, `unsigned` or
  `unverifiable`), `kind`, `key_fingerprint` and `principal` as git reports
  them (absent unless git reports them), and `reason` for `unverifiable`.
- The command writes one line of JSON to stdout, the verdict with the member
  `protocol` equal to `engineering-assurance.signature-verify-result/v1`, and
  exits with status 0 for every status, as FR-029 does for every closed state.
- A request that yields no verdict writes the `engineering-assurance.error/v1`
  machine error for capability `signature-verify` to stdout and its message to
  stderr, and exits with status 2. The codes are
  `signature_verify_request_unreadable`, `signature_verify_request_invalid`
  and `signature_verify_revision_invalid`.

## Behavior

- The command SHALL run `git verify-commit` for kind `commit` and
  `git verify-tag` for kind `tag`, through the process host limits of FR-019
  (wall-clock timeout and capture bound), without the user's or the system's
  git configuration other than the signature-related settings the request
  names, and without repository-selecting environment variables. It SHALL
  NOT implement signature parsing or verification itself.
- The verdict SHALL be `signed_valid` when git exits 0 and its status text
  reports a good SSH signature; `signed_bad` when git reports a signature that
  does not verify (bad signature, unknown or untrusted key, revoked or expired
  key); and `unsigned` when git reports that the object carries no signature.
  The classifier SHALL treat the three as distinct and SHALL NOT treat
  `signed_bad` or `unverifiable` as `unsigned`.
- The verdict SHALL be `unverifiable` with a reason when git cannot start, the
  repository or revision cannot be resolved, git exits nonzero for a reason
  that is not a signature outcome, git times out, its output exceeds the
  capture bound, or its output matches no known outcome. It SHALL NOT be
  `signed_valid` in any of those cases.
- Kind `tag` SHALL require an annotated tag. A lightweight tag (a ref directly
  to a commit) carries no signature of its own and SHALL yield `unsigned` with
  reason `lightweight_tag`, never a verification of the commit it points to.
- `key_fingerprint` and `principal` SHALL be copied as git reports them for a
  good or bad signature, and SHALL be absent otherwise. The command SHALL NOT
  derive, normalize or invent either.
- The command SHALL refuse a revision that is empty, longer than 256 bytes,
  begins with `-`, or contains a control character or whitespace, before it
  starts git.
- The command SHALL NOT modify the repository, fetch from a network, or decide
  that a signed object is acceptable.
- The pure classifier SHALL perform no I/O. It lives in the `signature-verify`
  Cargo feature; the git adapter lives in the binary, as for FR-032.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-036-CON-1 | A result SHALL NOT be `signed_valid` unless git itself exited 0 and reported a good SSH signature. | Safety | Test |
| FR-036-CON-2 | The implementation SHALL contain no signature-parsing or cryptographic code and SHALL add no cryptography dependency. | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-036-AC-1 | On a real repository with a fictional test key, a validly signed commit and a validly signed annotated tag each yield `signed_valid` with the `key_fingerprint` git reports. | Test |
| FR-036-AC-2 | An unsigned commit and an unsigned annotated tag each yield `unsigned`, distinct from `signed_bad`. | Test |
| FR-036-AC-3 | A commit whose content was altered after signing, and a commit signed by a key absent from the allowed-signers file, each yield `signed_bad`. | Test |
| FR-036-AC-4 | A lightweight tag yields `unsigned` with reason `lightweight_tag`. | Test |
| FR-036-AC-5 | A missing git executable, an unknown revision, a directory that is not a repository, a git timeout and output above the capture bound each yield `unverifiable` with a reason, exit 0, and never `signed_valid` or `unsigned`. | Test |
| FR-036-AC-6 | A revision that is empty, 257 bytes long, begins with `-`, or contains whitespace or a control character yields `signature_verify_revision_invalid` and exit status 2, and does not start git. | Test |
| FR-036-AC-7 | An unreadable or non-conforming request yields the matching machine error and exit status 2. | Test |
| FR-036-AC-8 | The classifier maps captured git outcomes for good, bad, unknown-key, unsigned and unrecognized output to the right status without running git. | Test |
| FR-036-AC-9 | The `signature-verify` feature and the binary add no dependency that performs signature verification or cryptography. | Inspection |

## Dependencies

- **Upstream**: [FR-019](./FR-019-bounded-producer-execution.md) for the process
  host limits; [FR-028](./FR-028-shared-cli-output.md) for stdout and
  machine-error encoding; [FR-029](./FR-029-producer-execute-command.md) for the
  exit-code convention; [FR-032](./FR-032-declared-scope-footprint-verification.md)
  for the pure-core and git-adapter split.
- **Downstream**: [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
  and consumers that record signed approvals and read this result as data.
