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

Engineering Assurance SHALL attach to a valid SSH signature result (FR-036) the
named person an allowed-signers file maps the signing key to, and the meaning a
signer declared in a `Signed-off-meaning:` trailer. A verified key that the file
does not name is reported as unmapped, and a commit without the trailer is
reported as lacking it. Neither is defaulted or guessed: attributing a signature
to a person, or a meaning to a signature, is a claim that needs its own evidence.

## Inputs

- The allowed-signers file named in the FR-036 request, in the format git
  documents for `gpg.ssh.allowedSignersFile`: one principal per entry, with the
  key it is allowed to sign with.
- For kind `commit`, the commit message; for kind `tag`, the tag message.

## Outputs

- On the FR-036 verdict: `signer` with `mapping` (`mapped` or `unmapped`) and,
  when mapped, the `principal` name from the file; and `signed_off_meaning`
  with `presence` (`present` or `missing`) and, when present, the trailer
  `values` in order.
- `signer.mapping` is present only when `status` is `signed_valid`.
- A malformed allowed-signers file writes the machine error
  `signature_verify_allowed_signers_invalid` and exits with status 2.

## Behavior

- A verified signer SHALL be `mapped` only when git's own verification, run
  against the supplied allowed-signers file, accepts the signing key for a
  principal in that file. A key git verifies but the file does not name SHALL
  be `unmapped`. Engineering Assurance SHALL NOT infer a person from the
  commit author, committer, tagger, email, or key comment.
- Without an `allowed_signers` member, no signature SHALL be reported `mapped`
  and every `signed_valid` result SHALL carry `mapping` `unmapped`.
- The allowed-signers file SHALL be refused, before git is started, when it is
  not a readable regular file, exceeds 1 MiB, is not valid UTF-8, or has a
  non-comment line that does not parse as principals, options and a key type
  and key. The refusal SHALL NOT pass the file to git.
- The `Signed-off-meaning:` trailer SHALL be read with the semantics of
  `git interpret-trailers --parse` over the object's message. A repeated
  trailer SHALL yield every value in order. A trailer with an empty value is
  `present` with that empty value, never `missing`.
- A message without the trailer SHALL yield `presence` `missing` and no
  `values`; the command SHALL NOT substitute a default meaning.
- The meaning is exposed as recorded text. Engineering Assurance SHALL NOT
  interpret it, validate it against a vocabulary, or decide that it is
  sufficient for any purpose.
- The trailer SHALL be reported for every status that has a readable message,
  including `signed_bad`, and SHALL NOT upgrade a status.
- The pure parts (principal lookup result handling and trailer value
  collection) SHALL perform no I/O; the file read and git calls live in the
  adapter.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-037-CON-1 | A signer SHALL NOT be reported `mapped` unless the allowed-signers file names its key. | Safety | Test |
| FR-037-CON-2 | A missing trailer SHALL NOT be reported as `present` or given a default value. | Safety | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-037-AC-1 | A commit validly signed by a key the allowed-signers file names yields `signed_valid` with `mapping` `mapped` and that file's principal name. | Test |
| FR-037-AC-2 | A commit validly signed by a key git verifies but the file does not name yields `mapping` `unmapped`, with no principal derived from author or email. | Test |
| FR-037-AC-3 | A request without `allowed_signers` yields `unmapped` for a validly signed commit. | Test |
| FR-037-AC-4 | An allowed-signers file that is missing, a directory, over 1 MiB, not UTF-8, or has an unparsable non-comment line yields `signature_verify_allowed_signers_invalid` and exit status 2, and does not start git. | Test |
| FR-037-AC-5 | A message with one `Signed-off-meaning:` trailer yields `present` with that value; a message with two yields both in order; a trailer inside the message body but not in the trailer block is not read. | Test |
| FR-037-AC-6 | A message with no such trailer yields `missing` with no values, and a trailer with an empty value yields `present` with an empty value. | Test |
| FR-037-AC-7 | A `signed_bad` and an `unsigned` commit that carry the trailer report its values and keep their status. | Test |
| FR-037-AC-8 | The same validly signed annotated tag yields mapping and meaning from the tag message, not from the tagged commit's message. | Test |

## Dependencies

- **Upstream**: [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md).
- **Downstream**: consumers that record who signed and what the signer meant,
  reading this result as data.
