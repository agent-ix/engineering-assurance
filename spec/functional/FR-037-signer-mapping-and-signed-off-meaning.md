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
the file does not accept at signing time is reported `signed_untrusted` with mapping `unmapped`,
and a message without the trailer is reported as lacking it. Neither is
defaulted or guessed: attributing a signature to a person, or a meaning to a
signature, is a claim that needs its own evidence.

## Inputs

- The allowed-signers file named in the FR-036 request, in the format git
  documents for `gpg.ssh.allowedSignersFile`. It is required: git refuses to
  verify an SSH signature without it.
- For kind `commit`, the commit message; for kind `tag`, the tag message.
- An optional request member `trailer_keys`: an array of at most 16 trailer
  keys, for example `["Formal-rule"]`, whose values the caller wants read from
  the same message. Each key is 1 to 64 bytes of ASCII letters, digits and `-`,
  begins with a letter, and is matched case-insensitively as git matches
  trailer keys. Omitted or an empty array, no key is read.

## Outputs

- On the FR-036 verdict: `signer` with `source` (`allowed_signers` for an SSH signature, `request` for a
  gitsign signature, FR-040), `mapping` (`mapped` or `unmapped`) and,
  when mapped, `principal`, the name from the allowed-signers file (for an SSH
  signature; FR-040 defines the source for a gitsign signature): for a
  commit git's `%GS` code, and for a tag the first line printed by
  `ssh-keygen -Y find-principals -s <extracted signature> -f <allowed_signers>`
  when it exits 0; and `signed_off_meaning` with `presence` (`present` or `missing`) and,
  when present, the trailer `values` in order.
- `signer` is present only for `signed_valid` (`mapped`) and `signed_untrusted`
  (`unmapped`). There is no other `principal` member; FR-040 sets it from the request for a gitsign signature.
- An accepted signature without a principal is `unverifiable` with reason
  `unrecognized_output`: a commit graded `G` whose `%GS` is empty, and a tag
  git accepted for which `ssh-keygen -Y find-principals` fails or prints
  nothing.
- `trailers`, present only when the request carried a non-empty `trailer_keys`
  and the message was read successfully: an object with one member per
  requested key, spelled as the request spelled it, whose value is the array of
  that key's trailer values in message order. A key with no trailer in the
  message yields an empty array, never a missing member and never a default
  value. `trailers` is omitted, not emitted with empty arrays, when
  `trailer_keys` is omitted or `[]`, when no message was read (a lightweight
  tag or an unresolved revision), and when the trailer read failed (a NUL byte
  or output that is not valid UTF-8), so an omitted `trailers` is never read as
  "no trailer".
- A `trailer_keys` member that is not an array, holds an element that is not a
  valid key, holds more than 16 keys, or repeats a key (compared
  case-insensitively) writes the machine error
  `signature_verify_trailer_keys_invalid` and exits with status 2, without
  starting git. This check is part of request validation: it runs after the
  request is parsed, reports a wrongly typed `trailer_keys` as
  `signature_verify_trailer_keys_invalid` and not `signature_verify_request_invalid`,
  and runs before the allowed-signers file is examined, so a request with both
  an invalid `trailer_keys` and an invalid allowed-signers file yields
  `signature_verify_trailer_keys_invalid`.
- `signed_off_meaning` is omitted when no message was read (a lightweight tag,
  an unresolved revision, or a trailer read that failed); a message that holds a
  NUL byte, or trailer output that is not valid UTF-8, is a trailer read that
  failed, and no byte is replaced.
- A missing, unreadable or invalid allowed-signers file writes the machine
  error `signature_verify_allowed_signers_invalid` and exits with status 2.

## Behavior

- For an SSH signature the command SHALL report `mapped` only when git's own
  verification, run against the supplied file, accepts the signing key for a
  principal in that file, and SHALL report the key `unmapped` otherwise. For a
  gitsign signature FR-040 governs.
- The command SHALL NOT infer a person from the commit author, committer,
  tagger, email, or key comment.
- The command SHALL refuse the allowed-signers file, before git is started,
  when it is not a readable regular file, exceeds 1 MiB, or is not valid UTF-8.
- The command SHALL NOT re-implement the allowed-signers grammar.
- A line of the file that git cannot parse authorizes no key; git skips it and
  grades from the remaining lines, and the command SHALL report that grade.
- The command SHALL read the `Signed-off-meaning:` trailer with the semantics
  of `git interpret-trailers --parse` over the object's subject and body,
  parsed with no patch divider (`--no-divider`), so a `---` line does not end
  the message. For a
  tag it SHALL first remove the trailing signature block from the tag message,
  because git appends that block after the trailer.
- The command SHALL yield every value of a repeated trailer in order, each as
  git prints it with leading and trailing ASCII whitespace removed.
- The command SHALL report a trailer with an empty value as `present` with
  that empty value.
- The command SHALL yield `presence` `missing` and no `values` for a message
  without the trailer, and SHALL NOT substitute a default meaning.
- The command SHALL expose the meaning as recorded text.
- The command SHALL NOT interpret the meaning, validate it against a
  vocabulary, or decide that it is sufficient for any purpose.
- The command SHALL read every `trailer_keys` key with the same git trailer
  parsing as `Signed-off-meaning:`: `git interpret-trailers --parse` with
  `--no-divider`, the same tag signature-block removal, the same value trimming
  and the same UTF-8 and NUL handling, so one parse serves all keys.
- The command SHALL ignore the repository's `trailer.*` configuration in every
  trailer read, both the `Signed-off-meaning:` read and each `trailer_keys` read,
  so repository configuration cannot add, alter, rename or fabricate a value.
- The command SHALL NOT interpret, validate or rank a `trailers` value, and the
  `trailers` members SHALL NOT change a status or a `signer`.
- The command SHALL report `trailers` for every status that has a readable
  message, including `signed_bad` and `unsigned`.
- The command SHALL report the trailer for every status that has a readable
  message, including `signed_bad`, and the trailer SHALL NOT change a status.
- The trailer collection SHALL perform no I/O; the file read and git calls live
  in the adapter.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-037-CON-1 | A signer SHALL NOT be reported `mapped` unless git accepted the key against the allowed-signers file (SSH) or FR-040-CON-1 holds (gitsign). | Safety | Test |
| FR-037-CON-2 | A missing trailer SHALL NOT be reported as `present` or given a default value. | Safety | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-037-AC-1 | A commit and an annotated tag validly signed by a key the allowed-signers file names each yield `signed_valid` with `mapping` `mapped` and the principal name from that file, the tag principal coming from `ssh-keygen -Y find-principals` on the tag's signature, on git 2.34.1 with `ssh-keygen` installed. | Test |
| FR-037-AC-2 | A commit with a good signature by a key the file does not name yields `signed_untrusted` with `mapping` `unmapped` and no principal, none derived from author or email. | Test |
| FR-037-AC-3 | An allowed-signers file that is missing, a directory, over 1 MiB or not UTF-8 yields `signature_verify_allowed_signers_invalid` and exit status 2, and does not start git. | Test |
| FR-037-AC-4 | A file whose only line is unparsable yields `signed_untrusted` for a validly signed commit, and a file with an unparsable line plus a valid line for the signer yields `signed_valid` with that principal. | Test |
| FR-037-AC-9 | A commit and an annotated tag signed by a key whose allowed-signers entry has expired (`valid-before` in the past) each yield `signed_untrusted` with mapping `unmapped`, and not `signed_valid`, on git 2.34.1 with `ssh-keygen` installed. | Test |
| FR-037-AC-5 | A message with one `Signed-off-meaning:` trailer yields `present` with that value; a message with two yields both in order; a trailer in the body but not in the trailer block is not read. | Test |
| FR-037-AC-6 | A message with no such trailer yields `missing` with no values, and a trailer with an empty value yields `present` with an empty value. | Test |
| FR-037-AC-7 | A `signed_bad` and an `unsigned` commit that carry the trailer report its values and keep their status. | Test |
| FR-037-AC-8 | A signed annotated tag whose message carries the trailer yields `present` with that value read from the tag message, not from the tagged commit's message, despite the appended signature block. | Test |
| FR-037-AC-10 | A commit whose message carries two `Formal-rule:` trailers and a request with `trailer_keys` `["Formal-rule"]` yield `trailers` `{"Formal-rule": [both values in order]}`, and the same message read with `["formal-rule"]` yields the values under the key spelled `formal-rule`. | Test |
| FR-037-AC-11 | A repository whose local config sets `trailer.fr.key` to `Formal-rule` and `trailer.sm.key` to `Signed-off-meaning`, and a commit whose message carries the alias trailers `fr: injected` and `sm: injected` and no real `Formal-rule:` or `Signed-off-meaning:` trailer, yields `trailers` `{"Formal-rule": []}` for `trailer_keys` `["Formal-rule"]` and `signed_off_meaning` `missing`; the same holds when the config sets `trailer.fr.command` or `trailer.formal-rule.command` to print a value. | Test |
| FR-037-AC-12 | A message with a `---` line between the subject and a real `Formal-rule:` trailer block yields that trailer's value in `trailers`. | Test |
| FR-037-AC-13 | A requested key absent from the message yields an empty array under that key. A request without `trailer_keys`, one with `trailer_keys` `[]`, one with keys against a lightweight tag, and one with keys against a message that holds a NUL byte each yield a result with no `trailers` member. | Test |
| FR-037-AC-14 | Each of these `trailer_keys` values yields `signature_verify_trailer_keys_invalid` and exit status 2, and does not start git: a string, an object, an array holding a non-string element, an empty key, a 65-byte key, a key beginning with a digit, a key beginning with `-`, a key containing a space, a colon or an underscore, a key containing a non-ASCII character, 17 distinct keys, and the same key twice with different case. | Test |
| FR-037-AC-17 | A `trailer_keys` array holding one 64-byte key made of letters, digits and `-` and beginning with a letter, and one holding 16 distinct valid keys, are each accepted and yield a verdict with a `trailers` member naming every requested key. | Test |
| FR-037-AC-18 | A request with an invalid `trailer_keys` and an allowed-signers file that is missing yields `signature_verify_trailer_keys_invalid`, not `signature_verify_allowed_signers_invalid`; a request with a valid `trailer_keys` and a missing allowed-signers file yields `signature_verify_allowed_signers_invalid`. | Test |
| FR-037-AC-15 | A `signed_bad` commit, an `unsigned` commit and a signed annotated tag whose message carry `Formal-rule:` trailers report them in `trailers`, with the tag value read from the tag message and not the tagged commit, and each keeps its status. | Test |
| FR-037-AC-16 | A `Formal-rule:` line in the message body outside the trailer block is not returned in `trailers`. | Test |

## Dependencies

- **Upstream**: [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md).
- **Downstream**: consumers that record who signed and what the signer meant,
  reading this result as data.
