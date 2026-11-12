---
id: FR-037
title: "Map verified signers to named people and expose the signed-off meaning"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-036"
    type: "references"
---

# FR-037: Map verified signers to named people and expose the signed-off meaning

## Description

Engineering Assurance SHALL attach to a signature result (FR-036) the named
person an allowed-signers file maps the signing key to, and the meaning a
signer declared in a `Signed-off-meaning:` trailer. A good signature by a key
the file does not name is reported `signed_untrusted` with mapping `unmapped`,
and a message without the trailer is reported as lacking it. Neither is
defaulted or guessed: attributing a signature to a person, or a meaning to a
signature, is a claim that needs its own evidence.

## Inputs

- The allowed-signers file named in the FR-036 request, in the format git
  documents for `gpg.ssh.allowedSignersFile`. It is required: git refuses to
  verify an SSH signature without it.
- For kind `commit`, the commit message; for kind `tag`, the tag message.

## Outputs

- On the FR-036 verdict: `signer` with `mapping` (`mapped` or `unmapped`) and,
  when mapped, `principal`, the name git reports from the allowed-signers
  file; and `signed_off_meaning` with `presence` (`present` or `missing`) and,
  when present, the trailer `values` in order.
- `signer` is present only for `signed_valid` (`mapped`) and `signed_untrusted`
  (`unmapped`). There is no other `principal` member.
- A missing, unreadable or invalid allowed-signers file writes the machine
  error `signature_verify_allowed_signers_invalid` and exits with status 2.

## Behavior

- The command SHALL report `mapped` only when git's own verification, run
  against the supplied file, accepts the signing key for a principal in that
  file, and SHALL report the key `unmapped` otherwise.
- The command SHALL NOT infer a person from the commit author, committer,
  tagger, email, or key comment.
- The command SHALL refuse the allowed-signers file, before git is started,
  when it is not a readable regular file, exceeds 1 MiB, or is not valid UTF-8.
- The command SHALL NOT re-implement the allowed-signers grammar.
- When git rejects a line of the file and names the file as the cause, the
  command SHALL report `signature_verify_allowed_signers_invalid`.
- When git rejects a line of the file without naming the file, the command SHALL
  report `unverifiable` with reason `unrecognized_output`.
- The command SHALL read the `Signed-off-meaning:` trailer with the semantics
  of `git interpret-trailers --parse` over the object's subject and body. For a
  tag it SHALL first remove the trailing signature block from the tag message,
  because git appends that block after the trailer.
- The command SHALL yield every value of a repeated trailer in order.
- The command SHALL report a trailer with an empty value as `present` with
  that empty value.
- The command SHALL yield `presence` `missing` and no `values` for a message
  without the trailer, and SHALL NOT substitute a default meaning.
- The command SHALL expose the meaning as recorded text.
- The command SHALL NOT interpret the meaning, validate it against a
  vocabulary, or decide that it is sufficient for any purpose.
- The command SHALL report the trailer for every status that has a readable
  message, including `signed_bad`, and the trailer SHALL NOT change a status.
- The trailer collection SHALL perform no I/O; the file read and git calls live
  in the adapter.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-037-CON-1 | A signer SHALL NOT be reported `mapped` unless git accepted the key against the allowed-signers file. | Safety | Test |
| FR-037-CON-2 | A missing trailer SHALL NOT be reported as `present` or given a default value. | Safety | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-037-AC-1 | A commit validly signed by a key the allowed-signers file names yields `signed_valid` with `mapping` `mapped` and the principal name from that file. | Test |
| FR-037-AC-2 | A commit with a good signature by a key the file does not name yields `signed_untrusted` with `mapping` `unmapped` and no principal, none derived from author or email. | Test |
| FR-037-AC-3 | An allowed-signers file that is missing, a directory, over 1 MiB or not UTF-8 yields `signature_verify_allowed_signers_invalid` and exit status 2, and does not start git. | Test |
| FR-037-AC-4 | A file with a line git rejects yields `signature_verify_allowed_signers_invalid` or `unverifiable` with reason `unrecognized_output`, and never `signed_valid`. | Test |
| FR-037-AC-5 | A message with one `Signed-off-meaning:` trailer yields `present` with that value; a message with two yields both in order; a trailer in the body but not in the trailer block is not read. | Test |
| FR-037-AC-6 | A message with no such trailer yields `missing` with no values, and a trailer with an empty value yields `present` with an empty value. | Test |
| FR-037-AC-7 | A `signed_bad` and an `unsigned` commit that carry the trailer report its values and keep their status. | Test |
| FR-037-AC-8 | A signed annotated tag whose message carries the trailer yields `present` with that value read from the tag message, not from the tagged commit's message, despite the appended signature block. | Test |

## Dependencies

- **Upstream**: [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md).
- **Downstream**: consumers that record who signed and what the signer meant,
  reading this result as data.
