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
person a trust source maps the signing key or certificate identity to (an
allowed-signers file for an SSH signature, a version-controlled identity
registry for a gitsign signature), and the meaning a
signer declared in a `Signed-off-meaning:` trailer. A good signature by a key
the file does not accept at signing time is reported `signed_untrusted` with mapping `unmapped`,
and a message without the trailer is reported as lacking it. Neither is
defaulted or guessed: attributing a signature to a person, or a meaning to a
signature, is a claim that needs its own evidence.

## Inputs

- The allowed-signers file named in the FR-036 request, in the format git
  documents for `gpg.ssh.allowedSignersFile`. It is required: git refuses to
  verify an SSH signature without it.
- For a gitsign signature, the identity registry FR-040 names, read as a trust
  source (below).
- For kind `commit`, the commit message; for kind `tag`, the tag message.
- An optional request member `trailer_keys`: an array of at most 16 trailer
  keys, for example `["Formal-rule"]`, whose values the caller wants read from
  the same message. Each key is 1 to 64 bytes of ASCII letters, digits and `-`,
  begins with a letter, and is matched case-insensitively as git matches
  trailer keys. Omitted or an empty array, no key is read.

## Outputs

- On the FR-036 verdict: `signer` with `source` (`allowed_signers` for an SSH
  signature, `gitsign_registry` for a gitsign signature, FR-040), `mapping`
  (`mapped` or `unmapped`) and, when mapped, `principal`, the name from the
  allowed-signers file (for an SSH signature) or from the matching
  identity-registry entry (for a gitsign signature, FR-040). For an SSH
  signature the name is, for a
  commit, git's `%GS` code, and for a tag the first line printed by
  `ssh-keygen -Y find-principals -s <extracted signature> -f <allowed_signers>`
  when it exits 0; and `signed_off_meaning` with `presence` (`present` or `missing`) and,
  when present, the trailer `values` in order.
- `signer` is present only for `signed_valid` and `signed_untrusted`. For an SSH
  signature `signed_valid` is `mapped` and `signed_untrusted` is `unmapped`; a
  gitsign signature's mapping is set by FR-040. There is no other `principal`
  member, and a `principal` never comes from a request.
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

## Trust-source reading

A trust source addressed as `<rev>:<path>` is read in one way, whichever
requirement names it (the identity registry of FR-040 is one). The reading has
three outcomes: the content was read, the source is `invalid`, or git could not
answer (`git_failed`).

- The command SHALL split `<rev>:<path>` at the first `:`. A revision that
  itself contains `:`, such as `HEAD@{12:00}` or `:/text`, cannot be addressed.
- If `<rev>` is empty, is longer than 256 bytes, holds a control character or
  begins with `-`, or if `<path>` is empty, is longer than 1024 bytes, holds a
  control character, begins with `/`, or has an empty, `.` or `..` component,
  then the address is not well formed, and the requirement that names the source
  treats it as a request error, before git starts.
- The command SHALL resolve `<rev>` once, in the repository under verification,
  to a full commit id, peeling an annotated tag to the commit it names, and
  SHALL ignore replace refs and grafts.
- The command SHALL read the content from that commit's tree by git object, as
  `git cat-file blob <commit-id>:<path>` does, and SHALL NOT read the working
  tree or the index.
- If `<rev>` does not resolve to a commit, the path is absent from that commit,
  the entry is not a regular file blob (mode 100644 or 100755; a symbolic link,
  submodule or tree is not), the content exceeds 1 MiB, or the content is not
  UTF-8, then the command SHALL treat the source as `invalid`.
- If git cannot start, times out, exceeds its output bound, or the path is not a
  repository git can use, then the command SHALL treat the reading as
  `git_failed`, and SHALL NOT report it as `invalid`.
- The command SHALL make the reading once per run and SHALL use the same bytes
  for every decision of that run. The reading yields the content and the
  resolved full commit id.
- If the outcome is not "read", then the command SHALL start no signature
  verifier.
- Each requirement that names a source gives its outcomes this effect, and no
  other: `invalid`, like a registry whose content breaks the rules of the
  requirement that names it, writes that capability's
  `identity_registry_invalid` machine error at exit 2; `git_failed` has the
  capability's existing outcome for a git failure on the target, which is
  `unverifiable` at exit 0 with the FR-036 reason for `signature-verify` and
  `approval-verify`, `approval_trail_repository_unavailable` for
  `approval-trail`, and `release_verify_git_failed` for `release-verify`.

## Behavior

- For an SSH signature the command SHALL report `mapped` only when git's own
  verification, run against the supplied file, accepts the signing key for a
  principal in that file, and SHALL report the key `unmapped` otherwise. For a
  gitsign signature FR-040 governs: the mapping comes from the identity registry
  and from nowhere else.
- The command SHALL NOT take a `principal` from a request, for any signature
  kind.
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
| FR-037-CON-1 | A signer SHALL NOT be reported `mapped` unless git accepted the key against the allowed-signers file (SSH), or FR-040-CON-1 holds and the verified identity and issuer match an entry of the identity registry (gitsign). No principal comes from a request. | Safety | Test |
| FR-037-CON-2 | A missing trailer SHALL NOT be reported as `present` or given a default value. | Safety | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-037-AC-1 | A commit and an annotated tag validly signed by a key the allowed-signers file names each yield `signed_valid` with `mapping` `mapped` and the principal name from that file, the tag principal coming from `ssh-keygen -Y find-principals` on the tag's signature, on git 2.34.1 with `ssh-keygen` installed. | Test |
| FR-037-AC-2 | A commit with a good signature by a key the file does not name yields `signed_untrusted` with `mapping` `unmapped` and no principal, none derived from author or email. | Test |
| FR-037-AC-3 | An allowed-signers file that is missing, a directory, over 1 MiB or not UTF-8 yields `signature_verify_allowed_signers_invalid` and exit status 2, and does not start git. | Test |
| FR-037-AC-4 | A file whose only line is unparsable yields `signed_untrusted` for a validly signed commit, and a file with an unparsable line plus a valid line for the signer yields `signed_valid` with that principal. | Test |
| FR-037-AC-9 | A commit and an annotated tag signed by a key whose allowed-signers entry has expired (`valid-before` in the past) each yield `signed_untrusted` with mapping `unmapped`, and not `signed_valid`, on git 2.34.1 with `ssh-keygen` installed. | Test |
| FR-037-AC-19 | A trust source `<rev>:<path>` is read from the git object of the commit `<rev>` resolves to: with the working-tree file edited, or the index changed, after the commit, the content read is the committed bytes; a replace ref or graft naming the commit changes neither the commit nor the content; moving the branch `<rev>` names after the reading does not change the bytes used in that run. | Test |
| FR-037-AC-20 | A `<rev>` that does not resolve to a commit, a path absent from that commit, a directory, symbolic link or submodule entry, content over 1 MiB and content that is not UTF-8 each make the source `invalid` and start no signature verifier; an address with an empty `<rev>`, an empty path, a leading `/`, a `..` component, a control character or a `<rev>` with a leading `-` is not well formed, is refused before git starts, and a `<rev>` such as `HEAD@{12:00}` is split at its first `:` and so addresses nothing. | Test |
| FR-037-AC-21 | With `<rev>` a branch name the reading yields the content and the full commit id the branch named at the reading, and with `<rev>` an annotated tag it yields the commit the tag peels to. | Test |
| FR-037-AC-22 | A git that cannot start, times out, exceeds its output bound, or runs in a directory that is not a repository makes the reading `git_failed`, which is not reported as `invalid`, and starts no signature verifier. | Test |
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
