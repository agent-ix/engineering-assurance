---
id: FR-036
title: "Verify SSH signatures on commits and annotated tags by delegating to git"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-028"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-029"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-032"
    type: "references"
---

# FR-036: Verify SSH signatures on commits and annotated tags by delegating to git

## Description

Engineering Assurance SHALL report whether a commit or an annotated tag carries
a valid SSH signature, by running `git verify-commit` or `git verify-tag` against
a caller-supplied allowed-signers file and classifying what git reports. It adds
no cryptography of its own: git decides whether a signature is valid, and
Engineering Assurance only classifies the outcome, names the signing key as git
reports it, and refuses to guess when git cannot answer. A consumer that records
a signed approval or release reads the result as data and does not re-verify.

Out of scope: gitsign and Sigstore keyless signing (`git verify-commit` does
not check the signer identity for them, so a later requirement will define
them), the production of signatures, and any policy about squash merges or
which signed commits count as an approval. Signatures in other formats
(OpenPGP, X.509, gitsign) are not verified; they yield `unverifiable`.

## Inputs

- For the pure classifier: a target kind (`commit` or `tag`) and the observed
  outcome. For a commit that is git's exit status and signature grade code
  (one of `G`, `U`, `B`, `X`, `Y`, `R`, `E`, `N`). For a tag that is whether
  the tag object carries a signature block, the exit status of
  `git verify-tag`, and the exit status of the delegated `ssh-keygen` check.
  For both it is the signature format read from the signature header, the key
  fingerprint, and flags saying the output was cut short or the process timed
  out.
- For the `signature-verify` command, one request document
  `engineering-assurance.signature-verify-request/v1` with the members
  `protocol`, `repository` (absolute directory path), `revision`, `kind`
  (`commit` or `tag`) and `allowed_signers` (absolute file path, required;
  see FR-037).

## Outputs

- A verdict with `status`, `kind`, `key_fingerprint` (as git reports it; present
  for `signed_valid`, `signed_untrusted` and `signed_bad` when git reports one)
  and `reason`. The `signer` member (principal and mapping) is defined by FR-037.
- `status` is one of:
  - `signed_valid`: git reports a good signature by a key the allowed-signers
    file names (commit: grade `G`; tag: `git verify-tag` exit 0).
  - `signed_untrusted`: git reports a good signature by a key the file does
    not accept at signing time, being a key it does not name or a key whose
    entry is outside its validity window (commit: grade `U`; tag: `git verify-tag` nonzero and the
    delegated signature check succeeded).
  - `signed_bad`: a signature that does not verify (commit: grade `B`; tag:
    the delegated signature check failed).
  - `unsigned`: the object carries no signature (commit: grade `N`; tag: no
    SSH signature block in the tag object).
  - `unverifiable`: no classification is possible.
- `reason` is present only for `unsigned` with value `lightweight_tag` and for
  `unverifiable`, where it is exactly one of `git_not_started`,
  `repository_unavailable`, `revision_unresolved`, `timeout`,
  `output_exceeded`, `signature_unchecked` (grade `E`),
  `ssh_keygen_not_started`, `unsupported_signature_format`, `unrecognized_output`.
- The command writes one line of JSON to stdout, the verdict with the member
  `protocol` equal to `engineering-assurance.signature-verify-result/v1`, and
  exits with status 0 for every status, as FR-029 does for every closed state.
- A request that yields no verdict writes the `engineering-assurance.error/v1`
  machine error for capability `signature-verify` to stdout and its message to
  stderr, and exits with status 2. The codes are
  `signature_verify_request_unreadable`, `signature_verify_request_invalid`,
  `signature_verify_revision_invalid` and, from FR-037,
  `signature_verify_allowed_signers_invalid`.

## Behavior

- The command SHALL run `git verify-commit` for kind `commit` and
  `git verify-tag` for kind `tag`, with `gpg.ssh.allowedSignersFile` set to the
  request's file.
- The command SHALL grade a commit with `git log -1 --format=%G?`, read
  alongside `git verify-commit`. The `%G?` code is the commit mechanism only;
  it grades commits and not tag objects.
- The command SHALL grade an annotated tag object itself, never the commit it
  points to, in this order:
  - a tag object with no `-----BEGIN SSH SIGNATURE-----` block is `unsigned`;
  - otherwise `git verify-tag` exit 0 is `signed_valid`;
  - otherwise the command runs `ssh-keygen -Y check-novalidate -n git` with the
    extracted signature over the tag payload, being the tag object with the
    signature block removed: success is `signed_untrusted` and failure is
    `signed_bad`.
- The command SHALL split the tag object at the signature block header line.
- The command SHALL pass the two parts to `ssh-keygen` unchanged.
- The command SHALL require `ssh-keygen` only for the third step, run it under
  the same process limits, and report `unverifiable` with reason
  `ssh_keygen_not_started` when it cannot start.
- The command SHALL run git without the user's or the system's git
  configuration other than that setting, and without repository-selecting
  environment variables.
- The command SHALL run git through the process host of FR-019 with a 60 second
  wall-clock timeout and an 8 MiB capture bound, as the existing git adapters
  do.
- The command SHALL NOT implement signature parsing or verification itself.
- The classifier SHALL map the grade code and exit status to the statuses
  above and SHALL NOT classify from git's or ssh-keygen's free-form message
  text.
- The classifier SHALL report `unverifiable` with reason `unrecognized_output`
  for grades `X`, `Y` and `R`, which SSH signatures do not produce.
- The classifier SHALL treat `signed_untrusted`, `signed_bad` and `unverifiable`
  as distinct from `unsigned`.
- The classifier SHALL report `unverifiable` with reason
  `unsupported_signature_format` when the signature header is not an SSH
  signature, and SHALL NOT report `signed_bad` or `unsigned` for it.
- The classifier SHALL report `unverifiable` when git cannot start, the
  repository or revision cannot be resolved, git exits nonzero with a status
  code that is not a signature outcome, git times out, its output exceeds the
  capture bound, or its output matches no known outcome, with the matching
  reason, and SHALL NOT report `signed_valid` in those cases.
- Kind `tag` SHALL require an annotated tag. A lightweight tag (a ref directly
  to a commit) carries no signature of its own and SHALL yield `unsigned` with
  reason `lightweight_tag`, never a verification of the commit it points to.
- The command SHALL copy `key_fingerprint` as git reports it.
- The command SHALL NOT derive, normalize or invent a `key_fingerprint`.
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
| FR-036-CON-1 | A result SHALL NOT be `signed_valid` unless git itself reported the signature accepted for a key the allowed-signers file names, being grade `G` for a commit and `git verify-tag` exit 0 for a tag. | Safety | Test |
| FR-036-CON-2 | The implementation SHALL contain no signature-parsing or cryptographic code. | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-036-AC-1 | On a real repository with fictional test keys, a commit and an annotated tag signed by a key the allowed-signers file names each yield `signed_valid`, and the commit result carries the `key_fingerprint` git reports. | Test |
| FR-036-AC-2 | An unsigned commit and an unsigned annotated tag each yield `unsigned`, distinct from every other status. | Test |
| FR-036-AC-3 | A commit and an annotated tag signed by a key absent from the allowed-signers file each yield `signed_untrusted`, and a commit and a tag altered after signing each yield `signed_bad`, on git 2.34.1 with `ssh-keygen` installed. | Test |
| FR-036-AC-4 | A lightweight tag yields `unsigned` with reason `lightweight_tag`. | Test |
| FR-036-AC-5 | A missing git executable, an unknown revision and a directory that is not a repository each yield `unverifiable` with reason `git_not_started`, `revision_unresolved` and `repository_unavailable` respectively, exit 0, never `signed_valid` or `unsigned`. | Test |
| FR-036-AC-6 | A simulated process outcome at the adapter boundary for a timeout and for output above the 8 MiB bound yields `unverifiable` with reason `timeout` and `output_exceeded` respectively. | Test |
| FR-036-AC-7 | A revision that is empty, 257 bytes long, begins with `-`, or contains whitespace or a control character yields `signature_verify_revision_invalid` and exit status 2, and does not start git. | Test |
| FR-036-AC-8 | An unreadable or non-conforming request, including one without `allowed_signers`, yields the matching machine error and exit status 2. | Test |
| FR-036-AC-9 | The classifier maps the commit grade codes `G`, `U`, `B`, `N` and `E`, the codes `X`, `Y` and `R`, an unrecognized code, and a non-SSH signature header to the statuses and reasons above, and maps each tag outcome (no signature block, `git verify-tag` exit 0, delegated check success, delegated check failure) to its status, all from captured outcomes without running git or `ssh-keygen`. | Test |
| FR-036-AC-10 | An OpenPGP-signed commit yields `unverifiable` with reason `unsupported_signature_format`. | Test |
| FR-036-AC-11 | When `git verify-tag` exits nonzero on a signed tag and `ssh-keygen` cannot start, the verdict is `unverifiable` with reason `ssh_keygen_not_started`; a tag that `git verify-tag` accepts or that has no signature block does not need `ssh-keygen`. | Test |
| FR-036-AC-12 | The `signature-verify` feature and the binary add no crate dependency that performs signature verification or cryptography; the only verifying programs invoked are git and `ssh-keygen`. | Inspection |

## Dependencies

- **Upstream**: [FR-014](./FR-014-versioned-rust-boundary.md) for the Cargo
  feature discipline; [FR-019](./FR-019-bounded-producer-execution.md) for the
  process host limits; [FR-028](./FR-028-shared-cli-output.md) for stdout and
  machine-error encoding; [FR-029](./FR-029-producer-execute-command.md) for the
  exit-code convention; [FR-032](./FR-032-declared-scope-footprint-verification.md)
  for the pure-core and git-adapter split.
- **Downstream**: [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
  and consumers that record signed approvals and read this result as data.
