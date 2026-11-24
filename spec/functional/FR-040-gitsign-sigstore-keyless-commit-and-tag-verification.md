---
id: FR-040
title: "Verify gitsign (Sigstore keyless) signatures on commits and annotated tags by delegating to gitsign"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-036"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-037"
    type: "references"
---

# FR-040: Verify gitsign (Sigstore keyless) signatures on commits and annotated tags by delegating to gitsign

## Description

Engineering Assurance SHALL extend `signature-verify` (FR-036, FR-037) so that a
commit or annotated tag carrying a gitsign signature is verified against a
caller-supplied certificate identity and OIDC issuer by running `gitsign` and
classifying its exit status. `git verify-commit` and `git verify-tag` do not
check who made a gitsign signature, so git is never the verifier for these
objects. Engineering Assurance adds no cryptography and no Sigstore client of
its own: gitsign decides, and Engineering Assurance only classifies the outcome,
names the identity and issuer that gitsign accepted, and refuses to guess when
gitsign cannot answer. A gitsign signature checked without a caller-supplied
identity is never reported valid.

The person a verified gitsign identity belongs to comes from a version-controlled
identity registry, read from a committed git revision as FR-037 defines for a
trust source, and never from the request. The registry maps a certificate
identity and OIDC issuer to a person's name. A verified identity and issuer that
match an entry are `mapped` to that person; one that matches no entry are
`unmapped`. A request that carries a person's name for the identity is refused.
The caller chooses the revision of the registry in its own repository, as it
chooses the allowed-signers file for an SSH signature, so the mapping is only
as trustworthy as the commit the consumer accepts. The result names the
resolved full commit id of the registry, so a consumer can check that commit
against its own history (for example, that it lies on a protected branch) and
re-run against the same bytes.

gitsign exits 0 when it accepts a signature and 1 for every failure: a bad
signature, an identity or issuer mismatch, a missing signature, and an
unreachable Sigstore trust-data or transparency-log service alike. Exit status
therefore cannot tell a bad signature from an infrastructure failure, so this
requirement reports no `signed_bad` for a gitsign signature. What it can prove
is acceptance, and an identity or issuer mismatch on an otherwise acceptable
signature (by probing with wildcard matchers). Everything else is
`unverifiable`.

A gitsign signature is a CMS (X.509) signature, and other X.509 signers
produce the same signature block. One rule covers all of them: every CMS block
is delegated to gitsign, and a CMS signature that gitsign does not accept is
`unverifiable` with reason `gitsign_verification_failed`.

Verification by gitsign is not hermetic: it contacts the Sigstore TUF
repository and the Rekor transparency log, so a gitsign run needs network
access. This is accepted for gitsign signatures only; the SSH path of FR-036
stays offline.

Out of scope: Fulcio, Rekor and transparency-log checks beyond what `gitsign`
itself performs, except that FR-041 verifies the Rekor entry embedded in an
approval tag through the `sigstore-verify` library to report a trusted signing
time (FR-040-CON-2), offline trust roots, the production of signatures, any policy
about which identities may approve or release, and OpenPGP signatures (which
remain `unverifiable`, FR-036).

## Inputs

- The request document of FR-036 gains one optional member, `gitsign`, an
  object with:
  - `certificate_identity`: the certificate identity (for example an email
    address or workflow URI) the signature must carry, a non-empty string of at
    most 256 bytes with no control character or leading `-`;
  - `certificate_oidc_issuer`: the OIDC issuer URL the certificate must name,
    with the same bounds;
  Both members are required when `gitsign` is present. The `gitsign` object
  carries no person's name: a `principal` member is refused (below), and any
  other unknown member is rejected.
- The request document gains one optional member, `identity_registry`, a
  string `<rev>:<path>` addressing the identity registry in the repository under
  verification, as the trust-source reading of FR-037 defines. It is required
  when `gitsign` is present and rejected when `gitsign` is absent.
  `allowed_signers` stays required in every request (FR-036), so one request
  carries the trust inputs for either signature kind.
- The identity registry is one UTF-8 JSON document with the members `protocol`,
  equal to `engineering-assurance.gitsign-identity-registry/v1`, and `entries`,
  an array of at most 4096 objects. Each entry has exactly `certificate_identity`
  and `certificate_oidc_issuer`, with the bounds the request `gitsign` members
  have, and `principal`, a non-empty string of at most 256 bytes with no control
  character. No other member is accepted at any level. Two entries with the same
  identity and issuer are invalid, whether or not their principals differ.
- For the pure classifier: the target kind, the signature format read from the
  signature header (SSH, CMS, or other), whether an identity was supplied,
  whether the repository sets a `gitsign.*` key, the outcome of making the
  tag snapshot (made, could not be made, stored object did not re-hash to the
  resolved id), whether the snapshot ref still named the resolved id before and
  after each invocation, and the observed outcome of each delegated invocation
  (below): did not start, timed out, output cut short, or exit status.
- For kind `tag`, the request `revision` names the tag: it is either a bare
  tag name or `refs/tags/<name>`, and the name is the part after
  `refs/tags/`. A name that begins with `-` is `revision_unresolved`, because
  `gitsign` would read it as an option. For kind `commit` it is any revision FR-036 accepts.

## Outputs

- The FR-036 verdict gains `signature_kind` (`ssh` or `gitsign`), present
  whenever the signature block was identified as SSH or CMS, and absent for
  `unsigned`, for unsupported formats, and for failures before the signature
  could be read. Existing members and their meanings are unchanged.
- For a gitsign signature `status` is:
  - `signed_valid`: `gitsign` with the supplied identity and issuer exits 0.
  - `signed_untrusted`: gitsign accepts the signature with wildcard matchers
    but not with the supplied identity or issuer. The verdict adds `mismatch`,
    one of `identity`, `issuer`, `identity_and_issuer`.
  - `unverifiable`: every other case, including a gitsign signature with no
    `gitsign` member in the request. `signed_bad` is never reported.
  - `unsigned` is as FR-036: no signature block.
- On `signed_valid`, the verdict adds `certificate` with `identity` and
  `oidc_issuer`, copied from the request values that gitsign accepted, and
  `signer` is `source` `gitsign_registry`. Its `mapping` is `mapped`, with
  `principal` equal to the `principal` of the registry entry whose
  `certificate_identity` and `certificate_oidc_issuer` equal the accepted
  identity and issuer exactly, when such an entry exists, and `unmapped` with no
  principal otherwise; the status stays `signed_valid` in both cases. On
  `signed_untrusted`, `signer` is `source` `gitsign_registry`, `mapping`
  `unmapped`, no principal (FR-037), and no `certificate` member, because the
  certificate's actual identity is not read from gitsign's message text.
- Whenever the registry revision resolved to a commit, the verdict adds
  `identity_registry_commit`, the full commit id it resolved to, in every
  status. The member is absent when the revision did not resolve or the
  registry reading was `git_failed`.
- `reason` gains seven values in the closed list: `gitsign_not_started`,
  `gitsign_identity_missing`, `gitsign_verification_failed` (gitsign did not
  accept the signature even with wildcard matchers, exited 2, or exited with
  any status other than 0 or 1), `gitsign_repository_config` (the
  repository sets a `gitsign.*` key), `snapshot_unavailable` (the tag
  snapshot could not be made), `snapshot_object_mismatch` (the stored
  snapshot object did not hash to the resolved tag object id) and
  `snapshot_ref_changed` (`refs/tags/verified` did not name the resolved id
  before or after a `gitsign` run). Each of the three snapshot reasons
  accompanies `unverifiable`, never `signed_bad` or `signed_untrusted`. The
  list is otherwise as FR-036.
- `signed_off_meaning` is reported as FR-037 for every status with a readable
  message.
- A malformed `gitsign` member, or an `identity_registry` that is present
  without `gitsign`, absent with `gitsign`, or not a well-formed `<rev>:<path>`
  address (FR-037), writes the machine error
  `signature_verify_request_invalid` and exits with status 2, as FR-036.
- A request that otherwise validates, with both `gitsign` and
  `identity_registry`, and whose `gitsign` member carries `principal`, writes
  the machine error `signature_verify_gitsign_principal_refused` and exits with
  status 2. The value is not used, not echoed and not a fallback.
- An identity registry source that is `invalid` (FR-037), or whose content is not
  valid JSON, has a wrong `protocol`, has an entry that breaks the bounds above,
  has an unknown member, or repeats an identity and issuer writes the machine
  error `signature_verify_identity_registry_invalid` and exits with status 2.
- A registry reading that is `git_failed` (FR-037) yields `unverifiable` at exit
  0 with the FR-036 reason for the failure (`git_not_started`, `timeout`,
  `output_exceeded` or `repository_unavailable`), and no machine error.

## Behavior

- The command SHALL validate the request first, so that a `gitsign` member
  without `identity_registry` is `signature_verify_request_invalid`.
- The command SHALL refuse a `gitsign` member that carries `principal` only
  after the request validates.
- The command SHALL read the identity registry as a trust source once, after
  that refusal, whatever the signature kind of the object.
- The command SHALL validate the content of the registry before it starts any
  signature verifier.
- The command SHALL map a verified gitsign identity to a person only by exact
  string equality of the accepted identity and issuer with a registry entry.
- The command SHALL NOT map by a pattern, a prefix, case folding or a partial
  match.
- The command SHALL identify the signature kind from the signature block, not
  from message text: for a commit, the `gpgsig` and `gpgsig-sha256` headers,
  matched as prefixes as FR-036 does, where a block beginning
  `-----BEGIN SIGNED MESSAGE-----` is CMS; for a tag, the signature block that
  begins at the signature header line.
- The command SHALL treat a commit as gitsign-signed only when every signature
  header is CMS; a commit with mixed formats stays `unverifiable` with reason
  `unsupported_signature_format`.
- The command SHALL NOT ask git to verify a CMS-signed object.
- The command SHALL keep the unchanged SSH path for SSH-signed objects.
- The command SHALL report `unverifiable` with reason `gitsign_identity_missing`
  for a CMS-signed object when the request has no `gitsign` member, without
  starting `gitsign`.
- The command SHALL NOT report `signed_valid` for a CMS-signed object without
  a `gitsign` member.
- When a CMS-signed object has a supplied identity, the command SHALL ask git
  for the configuration gitsign will see before it runs `gitsign`, by running
  exactly the query gitsign runs, `git config --get-regexp .*`, in the
  repository, so that includes (`include.path`, `includeIf`) and
  `config.worktree` are read as gitsign reads them. The check SHALL run in
  exactly the environment gitsign gets (`PATH`, `HOME`,
  `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`, and no
  `GIT_CONFIG_COUNT` or `GIT_CONFIG_PARAMETERS`). gitsign splits that output
  into lines and reads the first space-separated token of each line as a key,
  so a value holding a newline can inject a key. The command SHALL therefore
  read the output the same way and, when any line's first token begins with
  `gitsign.` (ASCII case-insensitive), or contains a non-ASCII byte before its
  first `.`, report `unverifiable` with reason
  `gitsign_repository_config` without starting `gitsign`. Exit 0 with no such
  line, or exit 1 (no key at all), means no key is set. The non-ASCII rule fails
  closed: gitsign compares the prefix with Unicode simple case folding, under
  which `gitſign.` (long s) and a Kelvin sign read as `gitsign.`, and the command
  does not implement that folding; any other outcome is
  `unverifiable` with the matching FR-036 reason. gitsign reads trust settings
  such as `gitsign.fulcioRoot` from that configuration, and a repository under
  verification must not choose the roots that verify it.
- The command SHALL delegate verification to `gitsign` as the only verifier,
  passing `--certificate-identity` and `--certificate-oidc-issuer` from the
  request as separate arguments, never through a shell, and the target after a
  `--` argument:
  - for kind `commit`, `gitsign verify -- <commit-id>`, where `<commit-id>` is the
    commit id git resolved from the request revision, never the request
    revision text;
  - for kind `tag`, `gitsign verify-tag -- verified`, run in the isolated tag
    snapshot below, where `verified` is the fixed name of the snapshot's only
    tag ref and never the request's tag name or an object id (gitsign v0.18.0
    verifies a tag ref in a non-bare repository and cannot verify a bare
    repository or a tag object id); a revision that does not resolve to an
    annotated tag under `refs/tags/<name>` is `unverifiable` with reason
    `revision_unresolved`, and a lightweight tag is `unsigned` with reason
    `lightweight_tag` as FR-036.
- The command SHALL verify a CMS-signed annotated tag only in an isolated tag
  snapshot: an ordinary, non-bare repository in a private temporary directory.
- The command SHALL store in the snapshot exactly one object, the resolved
  annotated tag object, as a loose object, with no alternates and no commit,
  tree or other tag.
- The command SHALL give the snapshot exactly one ref, `refs/tags/verified`,
  naming the resolved tag object id.
- The command SHALL create the snapshot without init templates, in the
  environment pinned below, and SHALL give it a configuration holding only
  core repository keys, with no `gitsign.*`, `include` or `includeIf` key and
  no key injected from the caller's, global or system configuration.
- The command SHALL re-hash the stored object and report `unverifiable` with
  reason `snapshot_object_mismatch` unless it hashes to the resolved id.
- The command SHALL report `unverifiable` with reason `snapshot_unavailable`
  when the snapshot cannot be made.
- The command SHALL check that `refs/tags/verified` names the resolved id at
  the start and at the end of each `gitsign` run.
- When `refs/tags/verified` does not name the resolved id at either check, the
  command SHALL report `unverifiable` with reason `snapshot_ref_changed`.
- The command SHALL NOT report `signed_bad` or `signed_untrusted` for any
  snapshot refusal.
- While the snapshot is in use, the command SHALL verify the resolved object
  whatever happens to the source ref or the source configuration.
- When the source repository's `gitsign.*` configuration check finds a key,
  the command SHALL make no snapshot.
- The command SHALL classify from exit statuses only, using at most four
  invocations of `gitsign`:
  1. the exact identity and issuer: exit 0 is `signed_valid`;
  2. when step 1 exits 1, the same command with
     `--certificate-identity-regexp '.*'` and
     `--certificate-oidc-issuer-regexp '.*'` in place of the exact flags: a
     failure is `unverifiable` with reason `gitsign_verification_failed`;
  3. when step 2 exits 0, the exact identity with the wildcard issuer: exit 0
     means the issuer differs (`mismatch` `issuer`);
  4. when step 3 exits 1, the wildcard identity with the exact issuer: exit 0
     means the identity differs (`identity`) and exit 1 means both differ
     (`identity_and_issuer`).
  An exit status of 2 or any status other than 0 or 1 at any step is
  `unverifiable` with reason `gitsign_verification_failed`. An invocation that
  cannot start, times out or exceeds the capture bound makes the verdict
  `unverifiable` with the matching FR-036 reason.
- The command SHALL pin the program: it runs the executable named `gitsign`,
  and SHALL NOT take a name or path from the request, the repository or git
  configuration. This program-resolution rule covers every external program the
  command runs, `git`, `ssh-keygen` and `gitsign` alike: each is found only in
  the absolute entries of the inherited `PATH`, is run by that absolute path,
  and is given a `PATH` without its relative entries, so the repository under
  verification (the working directory) can never supply a program, directly or
  through a program the command starts. It SHALL report `unverifiable` with reason `gitsign_not_started`
  when `gitsign` cannot start.
- The command SHALL run `gitsign` for kind `commit` with the repository as its
  working directory, and for kind `tag` with the isolated tag snapshot as its
  working directory, through the process host of FR-019 with the same 60 second wall-clock timeout
  and 8 MiB capture bound per invocation.
- The command SHALL pass `gitsign` exactly this environment and nothing else:
  `PATH`, `HOME` (gitsign's TUF cache location), `GIT_CONFIG_NOSYSTEM=1` and
  `GIT_CONFIG_GLOBAL=/dev/null`. In particular no `SIGSTORE_*` or `GITSIGN_*`
  variable, no `TUF_ROOT`, and no repository-selecting variable of FR-036 is
  passed. Proxy variables are not passed, so a network that needs a proxy yields
  `gitsign_verification_failed`.
- The command SHALL NOT pass `gitsign` any flag that disables a check it
  performs.
- The command SHALL NOT derive a person from the certificate, the commit author,
  committer, tagger, email or key comment.
- The command SHALL take `principal` only from the identity registry.
- The command SHALL NOT take `principal` from the request, from a fallback to a
  request value when the registry has no entry, or from any other source.
- The command SHALL NOT classify from gitsign's free-form message text.
- The command SHALL NOT implement signature parsing, certificate handling or
  transparency-log access itself.
- The pure classifier SHALL perform no I/O.
- The pure classifier SHALL live in the `signature-verify` Cargo feature, and
  the gitsign adapter SHALL live in the binary, as for FR-036.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-040-CON-1 | A result SHALL NOT be `signed_valid` for a gitsign signature unless `gitsign` run with the request's exact identity and issuer, in the environment above, exited 0, the source repository's configuration (includes and worktree configuration included) has no `gitsign.*` key, and, for a tag, the isolated snapshot's configuration also has no `gitsign.*`, `include` or `includeIf` key. | Safety | Test |
| FR-040-CON-3 | A gitsign signer SHALL NOT be reported `mapped` unless the identity and issuer `gitsign` accepted equal an entry of the identity registry read from the committed revision the request names, and no `principal` is taken from a request. | Safety | Test |
| FR-040-CON-2 | The implementation SHALL contain no cryptographic, certificate-parsing or Sigstore client code of its own. | Architecture | Inspection |
| FR-040-CON-4 | The implementation SHALL add no crate dependency that performs signature verification, cryptography or Sigstore client access. | Architecture | Inspection |
| FR-040-CON-5 | When FR-041 verifies transparency-log time, the implementation SHALL use the existing `sigstore-verify` dependency's public `Verifier::verify` with the compiled-in production trusted root, without fetching or configuring another root. | Architecture | Inspection |
| FR-040-CON-6 | When FR-041 supplies the Rekor artifact to `Verifier::verify`, the implementation SHALL pass the CMS signed attributes re-encoded as DER with the explicit universal `SET OF` tag; `sigstore-verify` computes the artifact digest from those bytes. | Architecture | Inspection |
| FR-040-CON-7 | When FR-041 reads a Rekor entry from a tag signature, the implementation SHALL use a CMS/DER reader only to extract the signed-attributes DER, CMS signature octets, signing certificate and Rekor `TransparencyLogEntry`. | Architecture | Inspection |
| FR-040-CON-8 | When FR-041 accepts a Rekor entry for the tag payload, `gitsign` verification SHALL have verified the signed attributes' `messageDigest` against the payload of the same resolved immutable tag object and verified its CMS signature. | Safety | Inspection |
| FR-040-CON-9 | The implementation SHALL read no `integratedTime` without successful `sigstore-verify` verification. | Safety | Inspection |

## Acceptance Criteria

Stand-in note: `gitsign` is not installed on the build machines and keyless
signing needs a network and an OIDC login. Every criterion marked "Test
(stand-in)" is exercised only with a stand-in executable named `gitsign` on
`PATH` that records its arguments and environment and exits with a scripted
status, against real repositories whose commits and tags carry a fabricated CMS
signature block. Every criterion marked "Test (captured)" is exercised only by
the pure classifier on captured outcomes. FR-040-AC-15 and FR-040-AC-20 run the
real `gitsign`; they are ignored by default and need gitsign v0.18.0 and
available trust services. FR-041-AC-26 exercises the real verifier through
approval. The other criteria do not show that a real Sigstore signature
verifies. A wrong identity or issuer is distinct from `unverifiable`, while a
bad signature is `unverifiable`.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-040-AC-1 | With a stand-in `gitsign` exiting 0 for the exact identity and issuer, a CMS-signed commit and a CMS-signed annotated tag each yield `signed_valid` with `signature_kind` `gitsign`, `certificate` carrying that identity and issuer, and `signer` with `source` `gitsign_registry`, `mapping` `mapped` and the `principal` of the registry entry for that identity and issuer. | Test (stand-in) |
| FR-040-AC-2 | The stand-in receives, for a commit, `verify` and the resolved commit id and not the request revision text, run with the repository as the working directory; and for a tag, `verify-tag` and the fixed snapshot ref name `verified`, never the request's tag name or an object id, for both a bare-name and a `refs/tags/<name>` revision, run with the isolated tag snapshot as the working directory. Both carry `--certificate-identity` and `--certificate-oidc-issuer` with the request values as separate arguments and the target after a `--` argument, and git is not asked to verify the object. | Test (stand-in) |
| FR-040-AC-3 | With a stand-in that exits 1 for the exact flags and 0 for the wildcard flags, the verdict is `signed_untrusted` with `signer` `unmapped` and `source` `gitsign_registry`, no `certificate`, and `mismatch` `issuer` when only the issuer differs, `identity` when only the identity differs, and `identity_and_issuer` when both differ, using no more than four invocations. | Test (stand-in) |
| FR-040-AC-4 | With a stand-in that exits 1 for every invocation, the verdict is `unverifiable` with reason `gitsign_verification_failed`, never `signed_bad` and never `signed_untrusted`; a stand-in that exits 2, or any status other than 0 or 1, at any step gives the same result. | Test (stand-in) |
| FR-040-AC-5 | A CMS-signed commit and tag in a request without `gitsign` yield `unverifiable` with reason `gitsign_identity_missing`, the stand-in is never started, and the status is never `signed_valid`. | Test (stand-in) |
| FR-040-AC-6 | With no `gitsign` executable on `PATH`, a CMS-signed object and a supplied identity yield `unverifiable` with reason `gitsign_not_started`, exit 0. | Test |
| FR-040-AC-7 | A simulated timeout and a simulated output above 8 MiB at the gitsign adapter boundary yield `unverifiable` with reason `timeout` and `output_exceeded`. | Test (stand-in) |
| FR-040-AC-8 | An unsigned commit and tag still yield `unsigned` with no `signature_kind`, and an OpenPGP-signed commit and tag still yield `unverifiable` with reason `unsupported_signature_format`, whether or not `gitsign` is supplied. | Test |
| FR-040-AC-9 | A commit whose signature headers mix CMS and SSH yields `unverifiable` with reason `unsupported_signature_format`, and a commit whose `gpgsig` header begins with the CMS marker but has trailing characters on the same line is still detected as CMS. | Test (captured) |
| FR-040-AC-10 | The classifier maps the format, identity-supplied flag, repository-config flag, snapshot outcomes (could not be made; object did not re-hash; ref changed before or after a run) and invocation outcomes (exit 0; 1 then wildcard 1; 1 then wildcard 0 with each issuer and identity probe result; exit 2; other exit status; not started; timeout; cut short) to the statuses, `mismatch` values and reasons above, without running `gitsign` or git, and the classification does not change when the captured message text is replaced. | Test (captured) |
| FR-040-AC-11 | A request whose `gitsign` member lacks a field, has an empty or oversized value, a leading `-` identity, a control character, or an unknown member other than `principal`, a request with `gitsign` and no `identity_registry` (with or without a `principal` in `gitsign`), with `identity_registry` and no `gitsign`, or with an `identity_registry` that is not a well-formed `<rev>:<path>` address, yields `signature_verify_request_invalid` and exit status 2, and does not start git or `gitsign`. | Test |
| FR-040-AC-12 | An SSH-signed commit and tag in a request that also carries `gitsign` yield the FR-036 and FR-037 statuses, signer principal and mapping unchanged, with `signature_kind` `ssh` and `signer.source` `allowed_signers`, and `gitsign` is never started; the identity registry is still read and validated, so an invalid registry in such a request yields `signature_verify_identity_registry_invalid`. | Test (stand-in) |
| FR-040-AC-13 | A gitsign-signed commit and tag whose message carries a `Signed-off-meaning:` trailer report it as FR-037 does, and an `unverifiable` gitsign commit still reports it. | Test (stand-in) |
| FR-040-AC-14 | The `signature-verify` feature and the binary add no crate dependency that performs signature verification, cryptography or Sigstore access, other than the existing `sigstore-verify` dependency for FR-041 trusted time and a CMS/DER reader for extraction; the only verifying programs invoked are git, `ssh-keygen` and `gitsign`. | Inspection |
| FR-040-AC-15 | Against real `gitsign` v0.18.0 and available Sigstore trust services, a public gitsign-signed commit and annotated tag yield `signed_valid` for their true identity and issuer; the same objects yield `signed_untrusted` with `mismatch` `identity` for a wrong identity and `mismatch` `issuer` for a wrong issuer. The run records the exact gitsign version, object ids, identity, issuer, and network/trust-service outcome. | Test (live, ignored by default) |
| FR-040-AC-16 | The gitsign argument vectors, including `verify`, `verify-tag`, the four certificate flags and the wildcard values, match the command line documented for gitsign v0.18.0. | Inspection |
| FR-040-AC-17 | A CMS-signed commit in a repository that sets `gitsign.fulcioRoot` (or any other `gitsign.*` key) in its local configuration, in a file named by `include.path`, or in `config.worktree` with `extensions.worktreeConfig` enabled, or injected by a newline inside another key's value (including one whose first token holds a non-ASCII character, such as `gitſign.`), yields `unverifiable` with reason `gitsign_repository_config`, and the stand-in is never started; a repository with no such key proceeds. | Test (stand-in) |
| FR-040-AC-18 | The stand-in's recorded environment holds exactly `PATH`, `HOME`, `GIT_CONFIG_NOSYSTEM` and `GIT_CONFIG_GLOBAL`, even when the parent environment sets `SIGSTORE_ROOT_FILE`, `GITSIGN_FULCIO_ROOT`, `TUF_ROOT` and `GIT_DIR`. | Test (stand-in) |
| FR-040-AC-19 | A lightweight tag yields `unsigned` with reason `lightweight_tag`, and a tag revision that resolves to no annotated tag under `refs/tags/`, or whose name begins with `-` (`refs/tags/--help`), yields `unverifiable` with reason `revision_unresolved`, without starting `gitsign`. | Test (stand-in) |
| FR-040-AC-20 | Against real `gitsign` v0.18.0, changing the signed message of a public gitsign-signed annotated tag while retaining its original signature yields `unverifiable` with reason `gitsign_verification_failed`, never `signed_valid` or `signed_untrusted`. The test records the original and changed tag object ids. A missing executable remains `gitsign_not_started`; when the executable runs but the trust service or network is unavailable, the result is `unverifiable` and the live case is recorded unavailable, not as a successful interoperability run. Nonstandard exits, timeouts, output bounds and malformed adapter output remain covered by the adapter tests and do not become valid or bad-signature evidence. | Test (live, ignored by default) |
| FR-040-AC-21 | For a CMS-signed annotated tag verified through `signature-verify`, the stand-in observes a non-bare repository whose only object is the resolved tag object (no alternates, no commit, tree or other tag), whose only ref is `refs/tags/verified` naming the resolved id, and whose stored object hashes to that id. | Test (stand-in) |
| FR-040-AC-22 | When the stored snapshot object does not hash to the resolved id, the verdict is `unverifiable` with reason `snapshot_object_mismatch` and the stand-in is never started; when the snapshot cannot be made, the verdict is `unverifiable` with reason `snapshot_unavailable`; neither is `signed_bad` or `signed_untrusted`. | Test (stand-in) |
| FR-040-AC-23 | When `refs/tags/verified` is changed to another object before a `gitsign` run, or by the stand-in during a run, the verdict is `unverifiable` with reason `snapshot_ref_changed`, never `signed_valid`, `signed_bad` or `signed_untrusted`; moving or deleting the source tag ref after resolution leaves the verified object unchanged. | Test (stand-in) |
| FR-040-AC-24 | With a global `init.templateDir` holding a `gitsign.fulcioRoot` configuration and a `hooks` entry, and with a source repository configuration holding `include.path` and `includeIf` keys, the snapshot's configuration holds no `gitsign.*`, `include` or `includeIf` key, no template content is present, and the stand-in's configuration query inside the snapshot reports none; a source repository with a `gitsign.*` key is still `gitsign_repository_config` without a snapshot being made. | Test (stand-in) |
| FR-040-AC-25 | With a stand-in `gitsign` exiting 0, a registry committed at `<rev>:<path>` holding an entry for the exact identity and issuer yields `signed_valid` with `signer` `mapped` and that entry's `principal`; a registry with no entry for that identity and issuer yields `signed_valid` with `signer` `unmapped`, `source` `gitsign_registry` and no principal; an entry matching only the identity, only the issuer, or only by case, prefix or pattern does not map. | Test (stand-in) |
| FR-040-AC-26 | A request that otherwise validates, with `gitsign` carrying `principal` and with `identity_registry`, yields `signature_verify_gitsign_principal_refused` and exit status 2, does not start git or `gitsign`, and the value appears nowhere in the output, whether the registry holds an entry for the identity or none. | Test |
| FR-040-AC-27 | An `identity_registry` whose revision does not resolve to a commit, whose path is absent from that commit, is a symbolic link or directory, exceeds 1 MiB, is not UTF-8 or JSON, has a wrong `protocol`, has an unknown member, has an oversized or empty value, has more than 4096 entries, or repeats an identity and issuer yields `signature_verify_identity_registry_invalid` and exit status 2, and starts no signature verifier, for a CMS-signed and for an SSH-signed object. | Test |
| FR-040-AC-28 | The registry is read from the committed revision and not the working tree: with the working-tree registry holding an entry the committed registry lacks, the signer is `unmapped`; with the working-tree registry edited to drop an entry the committed registry holds, the signer is `mapped`. | Test (stand-in) |
| FR-040-AC-29 | The `mapped` principal depends only on the registry: the same accepted identity and issuer yield different principals under two committed registry revisions that name different people, and the commit author, committer, tagger, email and certificate text change nothing. | Test (stand-in) |
| FR-040-AC-30 | In a directory that is not a git repository, with a git that cannot start, and with a git that times out while the registry is read, the verdict is `unverifiable` at exit 0 with reason `repository_unavailable`, `git_not_started` and `timeout` respectively, not `signature_verify_identity_registry_invalid`, and no signature verifier starts; the verdict has no `identity_registry_commit` member. | Test |
| FR-040-AC-31 | Every verdict for a request with `gitsign` whose registry revision resolved to a commit carries `identity_registry_commit` equal to that full commit id, including after the branch the revision names has moved, and a verdict whose revision did not resolve carries none. | Test (stand-in) |

## Dependencies

- **Upstream**: [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md) for
  the request, verdict, classifier and hardening;
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md) for the signer and
  trailer model and the trust-source reading of the identity registry; [FR-019](./FR-019-bounded-producer-execution.md) for the
  process host limits.
- **Downstream**: consumers that record signed approvals and read this result as
  data.
