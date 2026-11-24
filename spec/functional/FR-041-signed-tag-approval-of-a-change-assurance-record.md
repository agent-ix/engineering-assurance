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
change-assurance record. The named person is the principal that the supplied trust
input (the allowed-signers file, or the `gitsign` identity with the version-controlled
identity registry) names for the signing key; the result is not an established identity, and it echoes that
trust input so a consumer can tell which trust root named the approver and can
re-run the command. The tag message carries the record id and the record
digest as trailers, so the signature binds the signer to the record content
the digest names. The command adds no cryptography of its own and no signature rules (the
log-entry verification below is the existing library's): it
runs the signature verification of FR-036, FR-037 and FR-040 on the tag and
adds only the record binding and the approval meaning. A consumer such as Quoin
retains the result as data and never re-verifies it.

An `approved` result also reports when the approval was signed, so that the
person, the time and the meaning appear together, as an electronic record needs.
It reports two times and never merges them. `signed_at` is the tag's tagger
date, which sits inside the signed payload, so the signature covers it and a
validly signed tag cannot have it altered; it is still the signer's own claim,
and the result names that source (`tagger_date`). `trusted_at` is a time an
independent transparency log attested. A gitsign signature made in gitsign's
offline Rekor storage mode carries the Rekor log entry in the signature block
of the tag object itself, as an unauthenticated CMS attribute (OID
1.3.6.1.4.1.57264.3.1, a protobuf Rekor `TransparencyLogEntry`, as gitsign's
`docs/verification.md` documents), holding `integratedTime`, the signed entry
timestamp (SET) and an inclusion proof. Because that attribute is not covered
by the CMS signature, `integratedTime` read from it is only a claim until the
SET, the inclusion proof and the binding of the entry to this signature are
verified. The command reads the entry from the tag bytes it already holds,
verifies it through the `sigstore-verify` library Engineering Assurance already
depends on, writes no verification code of its own, and reports `trusted_at`
only when that verification succeeds. It never reads `integratedTime` without
that verification, never reads the local clock for a time, and never runs
`gitsign` or reads `gitsign` output text for a time. An SSH signature has no
transparency-log entry, so an SSH approval has no baseline trusted time; a
signer-independent time for SSH needs a timestamp-authority token (RFC 3161),
which is a reserved seam (`trusted_at_source` `rfc3161`) and is not built. A
signed tag whose log entry is absent (gitsign online storage mode), cannot be
verified, or is not bound to the signature has `signed_at` and no `trusted_at`.

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
  no control character and no leading `-`), `allowed_signers` (required: an absolute file
  path, or a `<rev>:<path>` trust source read from a committed revision, as
  FR-036 and FR-037), and optionally `gitsign` (the object
  FR-040 defines, with the same bounds, and no `principal`) with its
  `identity_registry` (FR-040), required with `gitsign`. Unknown members are
  rejected.
- Trailer names, fixed by this requirement: `Approved-Record-Id` and
  `Approved-Record-Digest`, read from the tag message. The approval meaning is
  the `Signed-off-meaning` trailer of FR-037.

## Outputs

- One line of JSON on stdout, the result with the member `protocol` equal to
  `engineering-assurance.approval-verify-result/v1`, and exit status 0 for every
  status, as FR-036 does. The members are:
  - `status`, one of `approved`, `wrong_record`, `untrusted_signer`,
    `meaning_missing`, `signed_bad`, `unsigned`, `unverifiable`;
  - `tag`, the tag name the command resolved; `tag_object_id`, the canonical
    full object id of the annotated tag object whose message was parsed and
    whose signature was verified, or `null` when the request did not resolve to
    an annotated tag; and `record_id` and `record_digest` as requested;
  - `trust`, the trust input the verdict was made against, copied from the
    request: `allowed_signers` (the path or `<rev>:<path>` value as requested),
    `allowed_signers_sha256` (FR-037: `sha256:` and 64 lowercase hexadecimal
    digits of the exact bytes the run used), `allowed_signers_commit` (the
    full commit id a `<rev>:<path>` source resolved to; absent for a path), and
    `ea_version` (the version string of the running Engineering Assurance
    executable, as built), on every result whose allowed-signers reading
    succeeded (`allowed_signers_sha256` and `allowed_signers_commit` are absent
    when the reading was `git_failed`); and, when the request had one, `gitsign`
    (`certificate_identity` and `certificate_oidc_issuer`), `identity_registry`
    (the request's `<rev>:<path>` value) and `identity_registry_commit` (the full
    commit id it resolved to, as FR-040 reports it, present whenever the
    revision resolved and absent otherwise);
  - `reason`, present only when `status` is `unverifiable`: the FR-036 or FR-040
    reason, or `trailer_unreadable` when a validly signed tag's message could not
    be read;
  - `signature`, the FR-036 verdict for the tag exactly as FR-036, FR-037 and
    FR-040 define it, without a second `protocol` member;
  - on `approved` only: `approver`, the `principal` of the FR-037 signer;
    `meaning`, the array of `Signed-off-meaning` values in order;
    `signature_kind`, `ssh` or `gitsign` as FR-040 defines it; and the signing
    claimed time, which is exactly one of `signed_at` with `signed_at_source`,
    or `signed_at_missing`. `signed_at` is a UTC instant of the form
    `YYYY-MM-DDTHH:MM:SSZ`, and `signed_at_source` is `tagger_date`.
    `signed_at_missing` is `tagger_date_unreadable`, and appears, with neither
    other member, when the tag has no readable tagger date (rule below); and the
    trusted time, which is exactly one of `trusted_at` with `trusted_at_source`,
    or `trusted_at_missing`. `trusted_at` is a UTC instant of the form
    `YYYY-MM-DDTHH:MM:SSZ`, and `trusted_at_source` is `transparency_log`
    (`rfc3161` is reserved for a regulated-tier timestamp token and no result
    carries it). `trusted_at_missing` is one of `ssh_signature` (no log entry
    exists for an SSH signature), `no_log_entry` (a gitsign signature carrying
    no embedded entry), `log_entry_unverified` (an embedded entry failed any
    check of the Behavior section) and `trust_root_unavailable` (the Sigstore
    trusted root could not be obtained), and appears with neither other
    member. When `signed_at` and `trusted_at` are both present and differ, the
    result also carries `time_skew`, the integer number of seconds `signed_at`
    is later than `trusted_at` (negative when earlier). A status other than
    `approved` carries none of these members, because the signer of such a tag
    is not an accepted approver and a time beside an unaccepted signature would
    read as an attested one;
  - `binding`, when the tag message was readable: `record_id` and
    `record_digest`, each one of `match`, `mismatch`, `missing`, `repeated`.
- `status` is decided in this order, the first rule that applies winning:
  1. the FR-036 verdict `unsigned` (including a lightweight tag with reason
     `lightweight_tag`) is `unsigned`; `signed_bad` is `signed_bad`;
     `unverifiable` is `unverifiable`; `signed_untrusted` (a good signature by a
     key or identity the supplied trust inputs do not accept, so the signer is
     `unmapped`) is `untrusted_signer`; `signed_valid` whose signer is `unmapped`
     (a gitsign identity that no identity-registry entry names) is
     `untrusted_signer`, because an approval is by a named person; `signed_valid` with no readable tag
     message is `unverifiable` with reason `trailer_unreadable`;
  2. `signed_valid` with `binding` other than `match` for either member is
     `wrong_record`;
  3. `signed_valid`, both bindings `match`, and `signed_off_meaning` `missing`
     or holding any empty or Unicode White_Space-only value is `meaning_missing`;
  4. otherwise `approved`.
- A request that yields no verdict writes the `engineering-assurance.error/v1`
  machine error for capability `approval-verify` to stdout and its message to
  stderr, and exits with status 2. The codes are
  `approval_verify_request_unreadable`, `approval_verify_request_invalid`,
  `approval_verify_allowed_signers_invalid`,
  `approval_verify_gitsign_principal_refused`,
  `approval_verify_identity_registry_invalid` and
  `approval_verify_result_encoding_failed`, the middle two being the FR-040 refusals
  for a `gitsign` member that carries `principal` and for an identity registry
  whose source is `invalid` or whose content is unusable. A git failure while
  the registry is read is the `unverifiable` result of FR-040, not a machine
  error.

## Behavior

- The command SHALL obtain the `signature` verdict by the signature-verify
  verification of FR-036, FR-037 and FR-040 with kind `tag`, the request's
  `tag` as the revision, and the request's `allowed_signers`, `gitsign` and `identity_registry`,
  and SHALL NOT restate, relax or re-implement any rule they define.
- The command SHALL resolve an annotated tag once to its canonical full object
  id and SHALL read the tag message, tagger line and signature bytes from that
  immutable object. Its signature-verifier observation SHALL be for that same
  object id. It SHALL report the id as `tag_object_id` on every result after
  resolving an annotated tag, including refusals, and SHALL report `null` when
  no annotated tag object was resolved.
- If a signature verifier accepts only a mutable tag ref, the command SHALL
  verify an isolated ref that is bound to the resolved object id for the whole
  verifier invocation, or SHALL refuse as `unverifiable` if it cannot
  guarantee that binding. It SHALL NOT approve by combining parsed bytes from
  one object with a verifier result obtained from another object. This rule
  preserves the SSH and gitsign verifier distinctions of FR-036 and FR-040. For
  a CMS-signed tag the isolated ref is the `refs/tags/verified` ref of the
  isolated tag snapshot that FR-040 defines, and a refusal to verify it stays
  `unverifiable`, never `signed_bad` or `untrusted_signer`.
- The command SHALL report a lightweight tag as `unsigned` and a revision that
  does not resolve to a tag as `unverifiable`, as FR-040 defines, and SHALL
  accept no commit, merge commit or note as the approval object.
- The command SHALL read the two record trailers from the tag message with the
  semantics of `git interpret-trailers --parse` and the same preparation
  FR-037 states for the tag message (signature block removed, no patch
  divider).
- The command SHALL match trailer keys ASCII case-insensitively, as FR-037 does,
  so that every case variant of a key counts as an occurrence of that key.
- The command SHALL compare each trailer value to the request by exact byte
  equality, with no case folding, trimming beyond trailer parsing, or
  normalization.
- The command SHALL require the trailers to follow a subject paragraph, because
  git takes a message's only paragraph as its subject and reads no trailers from
  it; a tag message made only of trailer lines therefore yields binding `missing`
  for both members and status `wrong_record`.
- The command SHALL report a binding `missing` when its trailer is absent,
  `repeated` when it occurs more than once in any mix of key case (no occurrence
  is then preferred),
  `mismatch` when its single value differs from the request, and `match`
  otherwise.
- The command SHALL derive `approver` only from the FR-037 `principal`.
- The command SHALL NOT derive a person from the tag's tagger, the commit author,
  an email or a key comment.
- The command SHALL report the binding for every status with a readable tag
  message, and the binding SHALL NOT change a status decided by rule 1.
- The command SHALL read the signing time from the tagger line in the header
  block of the tag object it already read with `git cat-file tag`, never from
  the tag message. The tagger line is `tagger <name> <email> <epoch> <zone>`;
  the epoch counts seconds since 1970-01-01T00:00:00Z whatever the zone, so
  `signed_at` is the epoch formatted in UTC and the zone, which records only
  the signer's local offset, never moves the instant. The time is readable only
  when the header block holds exactly one `tagger` line, the text after the last
  `>` is exactly one space, an epoch of one to twelve ASCII digits no later than
  9999-12-31T23:59:59Z, one space and a zone of a sign and four digits with
  minutes below 60. Anything else is unreadable and no time is guessed.
- The command SHALL NOT default the signing time. It SHALL NOT use the
  verification time, the current time, the commit date or the author date for
  `signed_at` or `trusted_at`, and an `approved` result whose tagger date is unreadable stays
  `approved` and reports `signed_at_missing`.
- The command SHALL report the claimed time exactly as claimed, with no
  plausibility check: a date in the future, epoch 0, or a date before the
  signing key existed is reported verbatim as `signed_at`; the claim is the
  signer's in every rendering. For SSH, git 2.35 and later passes the claimed
  tagger date to `ssh-keygen` as `-Overify-time`, so key validity windows in the
  allowed-signers file are judged at the claimed time and not at the
  verification time (earlier git versions judge them at the verification time);
  an SSH approval has no trusted time (`trusted_at_missing` `ssh_signature`), so
  its key validity is judged at the claimed time on git 2.35 and later and is
  unattested. A deleted or expired allowed-signers entry is judged against the
  mapping the request supplies, which an auditor holds fixed by supplying the
  `<rev>:<path>` of the mapping in force at signing.
- The command SHALL take a trusted time only for an `approved` result whose
  `signature_kind` is `gitsign`, and only from the Rekor log entry in the CMS
  signature block of the verified tag object, read from the object bytes it
  already holds. It SHALL accept the entry as a trusted time only when the
  `sigstore-verify` library verifies, against the Sigstore trusted root, the
  entry's signed entry timestamp and inclusion proof, that the entry is for
  this signature (its certificate and signed-content digest equal those of the
  verified tag), and that `integratedTime` lies within the validity window of
  the signing certificate. Any other outcome is `trusted_at_missing`
  `log_entry_unverified`, never a partly trusted time, and never a change to
  `status`, `approver` or `signature`. If the library offers no public way to
  verify an embedded entry, every gitsign result is `trusted_at_missing`
  `log_entry_unverified`; the command SHALL NOT add verification code of its own.
- The command SHALL judge the gitsign certificate's validity at `trusted_at`
  where one exists: the window check above is that judgement, and a certificate
  whose window does not contain the verified `integratedTime` yields no
  `trusted_at`. The command SHALL NOT judge any key at the local clock.
- The command SHALL NOT report `integratedTime`, the SET, the inclusion proof
  or any other transparency-log field of an unverified entry, anywhere in a
  result.
- The command SHALL obtain the trusted root through the verification library's
  own trusted-root support and SHALL report `trusted_at_missing`
  `trust_root_unavailable`, and make no other change to the result, when it
  cannot.
- The command SHALL NOT run `gitsign`, or change any invocation, status, probe
  or classification of FR-040, to obtain a signing time, and SHALL NOT parse
  `gitsign` output text for one.
- The command SHALL leave the `signature-verify` output of FR-036, FR-037 and
  FR-040 unchanged: it gains no signing-time member, and the `signature` member
  of an approval result stays exactly that output.
- The command SHALL NOT default a missing trailer.
- The command SHALL NOT report `approved` for a tag whose message lacks
  `Signed-off-meaning`.
- The command SHALL NOT interpret the digest, read the record, or decide what
  a meaning permits.
- The decision of `status` from the verdict and the binding SHALL be a pure
  function with no I/O; the git and trust-tool calls stay in the
  signature-verify adapters.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-041-CON-1 | A result SHALL NOT be `approved` unless the FR-036 verdict for the tag is `signed_valid` with a mapped signer, the tag message carries exactly one `Approved-Record-Id` equal to the request's `record_id`, exactly one `Approved-Record-Digest` equal to the request's `record_digest`, and a `Signed-off-meaning` trailer whose every value holds a character that is not Unicode White_Space. | Safety | Test |
| FR-041-CON-2 | The command SHALL contain no signature, certificate or trailer-parsing code of its own, and no rule of FR-036, FR-037 or FR-040 restated. | Architecture | Inspection |
| FR-041-CON-3 | A result SHALL NOT report a `signed_at` that was not read from a well-formed tagger line of the tag object that was verified. | Safety | Test |
| FR-041-CON-5 | A result SHALL NOT carry `trusted_at` unless the `sigstore-verify` library verified the log entry's SET and inclusion proof, the entry's binding to the verified tag's signature, and `integratedTime` within the certificate window; no `trusted_at` is read from the local clock, the tagger date or an unverified entry. | Safety | Test |
| FR-041-CON-4 | A result SHALL NOT be `approved` unless trailer parsing and signature verification refer to the same immutable annotated tag object, identified by `tag_object_id`. | Safety | Test |

## Acceptance Criteria

Stand-in note: where `gitsign` is not installed, criteria that need it are
verified with the FR-040 stand-in executable, as FR-040 states; the SSH
criteria use real signatures from throwaway keys generated in the test.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-041-AC-1 | An annotated tag validly signed by a key the allowed-signers file maps to a principal, whose message carries one `Approved-Record-Id` and one `Approved-Record-Digest` equal to the request and a `Signed-off-meaning` trailer, yields `approved` with `approver` equal to that principal, `meaning` equal to the trailer values in order, `signature_kind` `ssh` and `tag` equal to the tag name. | Test |
| FR-041-AC-2 | A valid, mapped tag whose `Approved-Record-Digest` differs from the request by one character yields `wrong_record` with `binding.record_digest` `mismatch`, and no `approver`. | Test |
| FR-041-AC-3 | A valid, mapped tag whose `Approved-Record-Id` differs from the request yields `wrong_record` with `binding.record_id` `mismatch`; a tag lacking either trailer yields `wrong_record` with that binding `missing`; a tag repeating either trailer yields `wrong_record` with that binding `repeated`, even when one of the repeated values equals the request or the repeats differ only in the case of the key. | Test |
| FR-041-AC-4 | A tag with a good signature by a key the allowed-signers file does not name, whose message carries matching trailers, yields `untrusted_signer`, no `approver`, and a `signature` verdict of `signed_untrusted` with mapping `unmapped`. | Test |
| FR-041-AC-5 | A valid, mapped tag with matching record trailers and no `Signed-off-meaning` trailer yields `meaning_missing` and no `meaning` default; the same tag with a trailer whose value is empty or Unicode White_Space-only, alone or repeated, or beside a non-blank value, yields `meaning_missing` and no `meaning`. | Test |
| FR-041-AC-6 | A tag whose signature does not verify yields `signed_bad`; an unsigned annotated tag yields `unsigned`; a lightweight tag yields `unsigned` with `signature` reason `lightweight_tag`; a name resolving to no tag, or to a branch or commit, yields `unverifiable`; none of them yields `approved` even when the message carries matching trailers. | Test |
| FR-041-AC-7 | Record trailers placed in the tagged commit's message but not in the tag message do not bind the record (binding `missing`, status `wrong_record`), and a name that resolves to a signed commit or a merge commit rather than an annotated tag yields `unverifiable` with `signature` reason `revision_unresolved`, not `approved`. | Test |
| FR-041-AC-8 | A gitsign-signed tag with a `gitsign` member whose identity and issuer gitsign accepts, and matching trailers, yields `approved` with `signature_kind` `gitsign` and `approver` equal to the `principal` of the identity-registry entry for that identity and issuer; a tag whose identity gitsign accepts but the registry does not name, or whose identity gitsign does not accept, yields `untrusted_signer`; a CMS-signed tag with no `gitsign` member yields `unverifiable`. | Test |
| FR-041-AC-9 | The decision from the FR-036 verdict and the binding is exercised without starting any process, covering every combination of verdict status, signer mapping, binding member and meaning presence, and `approved` is returned for exactly one combination (CON-1). | Property |
| FR-041-AC-10 | A request with an unknown member, an empty or over-long `record_id` or `record_digest`, a leading `-`, a control character, a relative `repository` or `allowed_signers` path, an invalid `gitsign` member, or an `identity_registry` that is absent with `gitsign` or present without it writes `approval_verify_request_invalid` and exits 2; an invalid allowed-signers file writes `approval_verify_allowed_signers_invalid`, a `gitsign` member carrying `principal` writes `approval_verify_gitsign_principal_refused`, and an unusable identity registry writes `approval_verify_identity_registry_invalid`, each exiting 2; none of them starts a signature verifier, and only the identity registry refusal starts git. A request that validates with a `principal` is refused before git starts. A git failure while the registry is read yields `unverifiable` at exit 0. | Test |
| FR-041-AC-13 | A tag message made only of the trailer lines, with no subject paragraph, yields binding `missing` for both members and status `wrong_record`; the same trailers after a subject paragraph yield `match`. | Test |
| FR-041-AC-14 | The result of an `approved` or any other status carries `trust` equal to the request's `allowed_signers`, `gitsign` and `identity_registry` members plus `identity_registry_commit`, the full commit id the registry revision resolved to, present whenever the revision resolved and absent when it did not or when the registry reading was `git_failed`; a tag allowed-signers file naming a different principal for the same key yields a different `approver` under the different `trust`. | Test |
| FR-041-AC-15 | A validly signed tag whose message cannot be read yields `unverifiable` with reason `trailer_unreadable` and not `wrong_record`; a message cannot be read when git cannot parse it, when it holds a NUL byte, or when the parsed trailer output is not valid UTF-8, and no byte is replaced or truncated to make a trailer match. | Test |
| FR-041-AC-11 | Every status other than a request error exits 0 and writes one JSON line carrying the result protocol, and a validly signed tag whose trailers are unrelated to the request still carries the unchanged `signature` verdict that `signature-verify` yields for the same tag. | Test |
| FR-041-AC-12 | Static inspection shows no signature, certificate or trailer-parsing code in the approval module and no restated FR-036, FR-037 or FR-040 rule (CON-2). | Inspection |
| FR-041-AC-16 | A mapped, validly signed SSH tag with matching trailers and a `Signed-off-meaning` trailer, created with a known tagger date in a non-UTC zone (`+0530`, and `-0800` across a UTC day boundary), yields `approved` carrying `approver`, `meaning`, `signed_at` equal to the UTC instant of that date and `signed_at_source` `tagger_date`, for a bare and a `refs/tags/` name. | Test |
| FR-041-AC-17 | The tagger time is read from the tag header block only: a tagger-shaped line in the message, two tagger headers, a missing or non-digit epoch, an epoch past 9999-12-31T23:59:59Z, a malformed zone, extra fields or non-UTF-8 text yield no time, and epochs `0`, a leap day and the last accepted instant format correctly. | Test (captured) |
| FR-041-AC-18 | An `approved` result for a tag whose tagger date is missing or malformed carries `signed_at_missing` `tagger_date_unreadable` and neither `signed_at` nor `signed_at_source`, together with `approver` and `meaning`, and the result carries no claimed-time member other than `signed_at_missing` (and no `time_skew`). | Test (stand-in) |
| FR-041-AC-22 | A valid SSH-signed annotated tag with matching approval trailers remains `approved` when its ref is unchanged; the result's `tag_object_id` equals an independent `git rev-parse` of the tag object, and that same id supplies the parsed message and SSH verification. | Test |
| FR-041-AC-23 | After the command reads a validly signed tag message, moving or deleting the named ref before signature verification cannot produce `approved`: the command verifies the already-resolved immutable object or returns `unverifiable` with no approval members. Any result after annotated-tag resolution reports the originally resolved `tag_object_id`; it never combines the original message with verification of a replacement object. For a CMS-signed tag the verifier sees only the isolated tag snapshot of FR-040: a non-bare repository holding only the resolved tag object, re-hashed to the resolved id, under the ref `refs/tags/verified`; a source ref moved or deleted after resolution does not change the verified object, and a snapshot ref that stops naming the resolved id during verification yields `unverifiable`, never `approved`. | Test |
| FR-041-AC-24 | The moved-ref and deleted-ref cases in AC-23 are synchronized at the boundary between message extraction and verifier observation; an unchanged-ref healthy control proves the synchronization does not itself cause refusal. Repeat the identity-binding cases for SSH and gitsign using the verifier appropriate to each signature type; a missing gitsign executable or unavailable trust service is reported as unavailable/unverifiable, not as a passing integration case. | Test |
| FR-041-AC-25 | With the annotated tag ref unchanged, mutate the allowed-signers bytes, move the branch the `identity_registry` revision names, edit the working-tree registry, or change relevant repository gitsign configuration after request/trust capture but before verifier execution. A synchronized adapter observation proves the verifier uses the captured trust/configuration snapshot and returns a verdict for that snapshot; if the implementation cannot preserve that snapshot, it returns `unverifiable` with no approval members. The test's independent oracle identifies the exact trust/configuration bytes observed by the verifier. | Test |
| FR-041-AC-26 | Against real `gitsign` v0.18.0 and available Sigstore trust services, `approval-verify` on a public gitsign-signed annotated tag with matching approval trailers and a `Signed-off-meaning` yields `approved`, carries the same `tag_object_id` as an independent resolution of the tag, and preserves the FR-040 signature verdict. Wrong identity or issuer yields `untrusted_signer` with the corresponding FR-040 mismatch; a changed signed tag object yields `unverifiable` with no approval members; a gitsign-disallowed repository trust configuration yields `unverifiable` with no approval members. Missing executable or unavailable trust service yields `unverifiable` with no approval members and is recorded unavailable, not as a passing integration run. The test records the gitsign version, tag object ids, identity, issuer, and network/trust-service outcome. | Test (live, ignored by default) |
| FR-041-AC-19 | `wrong_record`, `untrusted_signer` and `meaning_missing` results for tags with a readable tagger date carry none of `signed_at`, `signed_at_source`, `signed_at_missing`, `trusted_at`, `trusted_at_source`, `trusted_at_missing` and `time_skew`, and supplying a time to the pure decision changes no status or reason for any combination. | Test |
| FR-041-AC-20 | A gitsign-signed tag approved through the stand-in reports `signed_at` from its tagger date (normalized from a non-UTC zone) with `signed_at_source` `tagger_date`; a stand-in rejecting the identity still yields `untrusted_signer` with no time; the time read starts no `gitsign` run. | Test (stand-in) |
| FR-041-AC-21 | The `signature-verify` output for the same tag has no signing-time member and equals the `signature` member of the approval result. | Test |
| FR-041-AC-27 | An `approved` result for an SSH-signed tag carries `signed_at` with `tagger_date` and `trusted_at_missing` `ssh_signature`, and no `trusted_at`, `trusted_at_source` or `time_skew`, for every tagger date including a backdated one. | Test |
| FR-041-AC-28 | For a gitsign tag whose embedded Rekor entry the `sigstore-verify` library verifies (SET, inclusion proof, binding to the tag's signature, `integratedTime` inside the certificate window), the `approved` result carries `trusted_at` equal to `integratedTime` in UTC, `trusted_at_source` `transparency_log`, `signed_at` with `tagger_date`, and `time_skew` equal to the difference in seconds when the two differ and no `time_skew` when they are equal. The fixture is a captured real gitsign tag with its trusted root, so no network is needed. | Test (captured) |
| FR-041-AC-29 | A gitsign tag whose embedded entry has a tampered `integratedTime`, a SET that does not verify, an inclusion proof that does not verify, an entry copied from a different object, or an `integratedTime` outside the certificate window, yields `approved` with `trusted_at_missing` `log_entry_unverified` and no `trusted_at`, no `integratedTime` or other log field anywhere in the result, and an unchanged `status`, `approver` and `signature`. | Test (captured) |
| FR-041-AC-30 | A gitsign tag carrying no embedded entry yields `trusted_at_missing` `no_log_entry`; with the trusted root unavailable it yields `trusted_at_missing` `trust_root_unavailable`; in neither case does the result carry `trusted_at`, and `signed_at` is the tagger date, never the local clock. | Test (stand-in) |
| FR-041-AC-31 | On git 2.35 or later, an SSH tag whose tagger date is earlier than the `valid-before` of its allowed-signers entry (itself in the past) is `approved` with `signed_at` the backdated claim and `trusted_at_missing` `ssh_signature`, so the claim is reported as unattested. The test reads the git version and is skipped, not passed, on an older git (git 2.34.1 on this host). | Test |
| FR-041-AC-32 | A key whose allowed-signers entry has expired before both the signing and the verification time yields `untrusted_signer` on every git version, by the setup of FR-037-AC-9. | Test |
| FR-041-AC-33 | The trusted-time code reads the entry only through the `sigstore-verify` library and contains no certificate, SET, inclusion-proof or Merkle code of its own, and no function returns a log field except after verification. | Inspection |
| FR-041-AC-34 | With `allowed_signers` `<rev>:<path>`, the `approved` result's `trust` carries `allowed_signers` as requested, `allowed_signers_commit` equal to the full commit id the revision resolved to, `allowed_signers_sha256` equal to an independent SHA-256 of that committed blob, and `ea_version` equal to the running executable's version. After the working-tree file is edited and the branch moved, a re-run with the same `<rev>:<path>` (the revision held fixed by a tag) yields the same approver, digest and commit, while an absolute-path request against the edited file yields a different `allowed_signers_sha256`. | Test |
| FR-041-AC-35 | Every `approval-verify` result carries `trust.allowed_signers_sha256` and `trust.ea_version`; `allowed_signers_commit` is present exactly when the source was a `<rev>:<path>` that resolved; a git failure while the allowed-signers source is read yields `unverifiable` with neither digest nor commit. | Test |

## Dependencies

- **Upstream**: [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md),
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
  [FR-040](./FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md).
- **Downstream**: consumers that retain a verified approval as data; Quoin's
  change-assurance verification is the first.
