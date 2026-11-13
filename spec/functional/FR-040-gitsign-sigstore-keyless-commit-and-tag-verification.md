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
itself performs, offline trust roots, the production of signatures, any policy
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
  - `principal`: the name of the person the caller declares this identity
    belongs to, a non-empty string of at most 256 bytes with no control
    character.
  All three members are required when `gitsign` is present; unknown members are
  rejected. `allowed_signers` stays required in every request (FR-036), so one
  request carries the trust inputs for either signature kind.
- For the pure classifier: the target kind, the signature format read from the
  signature header (SSH, CMS, or other), whether an identity was supplied,
  whether the repository sets a `gitsign.*` key, and the observed outcome of
  each delegated invocation (below): did not start, timed out, output cut
  short, or exit status.
- For kind `tag`, the request `revision` names the tag: it is either a bare
  tag name or `refs/tags/<name>`, and the name is the part after
  `refs/tags/`. For kind `commit` it is any revision FR-036 accepts.

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
  `signer` is `source` `request`, `mapping` `mapped` with `principal` equal to
  the request's `gitsign.principal`. On `signed_untrusted`, `signer` is
  `source` `request`, `mapping` `unmapped`, no principal (FR-037), and no
  `certificate` member, because the certificate's actual identity is not read
  from gitsign's message text.
- `reason` gains four values in the closed list: `gitsign_not_started`,
  `gitsign_identity_missing`, `gitsign_verification_failed` (gitsign did not
  accept the signature even with wildcard matchers, exited 2, or exited with
  any status other than 0 or 1) and `gitsign_repository_config` (the
  repository sets a `gitsign.*` key). The list is otherwise as FR-036.
- `signed_off_meaning` is reported as FR-037 for every status with a readable
  message.
- A malformed `gitsign` member writes the machine error
  `signature_verify_request_invalid` and exits with status 2, as FR-036.

## Behavior

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
  whether the configuration gitsign will see has any key matching
  `^gitsign\.` before it runs `gitsign`, by running the unscoped
  `git config --get-regexp '^gitsign\.'` in the repository, so that includes
  (`include.path`, `includeIf`) and `config.worktree` are read as gitsign reads
  them. The check SHALL run in exactly the environment gitsign gets (`PATH`,
  `HOME`, `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`, and no
  `GIT_CONFIG_COUNT` or `GIT_CONFIG_PARAMETERS`). Exit 0 means a key is set and
  the command SHALL report `unverifiable` with reason `gitsign_repository_config`
  without starting `gitsign`; exit 1 means none is set; any other outcome is
  `unverifiable` with the matching FR-036 reason. gitsign reads trust settings
  such as `gitsign.fulcioRoot` from that configuration, and a repository under
  verification must not choose the roots that verify it.
- The command SHALL delegate verification to `gitsign` as the only verifier,
  passing `--certificate-identity` and `--certificate-oidc-issuer` from the
  request as separate arguments, never through a shell:
  - for kind `commit`, `gitsign verify <commit-id>`, where `<commit-id>` is the
    commit id git resolved from the request revision, never the request
    revision text;
  - for kind `tag`, `gitsign verify-tag <name>`, where `<name>` is the tag name
    defined in Inputs; a revision that does not resolve to an annotated tag
    under `refs/tags/<name>` is `unverifiable` with reason
    `revision_unresolved`, and a lightweight tag is `unsigned` with reason
    `lightweight_tag` as FR-036.
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
  configuration. It SHALL report `unverifiable` with reason `gitsign_not_started`
  when `gitsign` cannot start.
- The command SHALL run `gitsign` with the repository as its working directory,
  through the process host of FR-019 with the same 60 second wall-clock timeout
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
- The command SHALL take `principal` only from the request.
- The command SHALL NOT classify from gitsign's free-form message text.
- The command SHALL NOT implement signature parsing, certificate handling or
  transparency-log access itself.
- The pure classifier SHALL perform no I/O.
- The pure classifier SHALL live in the `signature-verify` Cargo feature, and
  the gitsign adapter SHALL live in the binary, as for FR-036.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-040-CON-1 | A result SHALL NOT be `signed_valid` for a gitsign signature unless `gitsign` run with the request's exact identity and issuer, on a repository whose configuration, includes and worktree configuration included, has no `gitsign.*` key, and in the environment above, exited 0. | Safety | Test |
| FR-040-CON-2 | The implementation SHALL contain no cryptographic, certificate-parsing or Sigstore client code, and add no crate dependency that performs signature verification. | Architecture | Inspection |

## Acceptance Criteria

Stand-in note: `gitsign` is not installed on the build machines and keyless
signing needs a network and an OIDC login. Every criterion marked "Test
(stand-in)" is exercised only with a stand-in executable named `gitsign` on
`PATH` that records its arguments and environment and exits with a scripted
status, against real repositories whose commits and tags carry a fabricated CMS
signature block. Every criterion marked "Test (captured)" is exercised only by
the pure classifier on captured outcomes. FR-040-AC-15 is the only criterion
that runs the real `gitsign`; it is ignored by default and needs `gitsign` and
network access. The other criteria do not show that a real Sigstore signature
verifies. The procedure meets the ticket's wish for a distinct bad-signature
result only as far as exit status allows: a wrong identity or issuer is
distinct from `unverifiable`, and a bad signature is `unverifiable`.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-040-AC-1 | With a stand-in `gitsign` exiting 0 for the exact identity and issuer, a CMS-signed commit and a CMS-signed annotated tag each yield `signed_valid` with `signature_kind` `gitsign`, `certificate` carrying that identity and issuer, and `signer` with `source` `request`, `mapping` `mapped` and the request's `principal`. | Test (stand-in) |
| FR-040-AC-2 | The stand-in receives, for a commit, `verify <commit-id>` with the resolved commit id and not the request revision text, and for a tag, `verify-tag <name>` with the bare tag name for both a bare-name and a `refs/tags/<name>` revision; both carry `--certificate-identity` and `--certificate-oidc-issuer` with the request values as separate arguments, run with the repository as the working directory, and git is not asked to verify the object. | Test (stand-in) |
| FR-040-AC-3 | With a stand-in that exits 1 for the exact flags and 0 for the wildcard flags, the verdict is `signed_untrusted` with `signer` `unmapped`, no `certificate`, and `mismatch` `issuer` when only the issuer differs, `identity` when only the identity differs, and `identity_and_issuer` when both differ, using no more than four invocations. | Test (stand-in) |
| FR-040-AC-4 | With a stand-in that exits 1 for every invocation, the verdict is `unverifiable` with reason `gitsign_verification_failed`, never `signed_bad` and never `signed_untrusted`; a stand-in that exits 2, or any status other than 0 or 1, at any step gives the same result. | Test (stand-in) |
| FR-040-AC-5 | A CMS-signed commit and tag in a request without `gitsign` yield `unverifiable` with reason `gitsign_identity_missing`, the stand-in is never started, and the status is never `signed_valid`. | Test (stand-in) |
| FR-040-AC-6 | With no `gitsign` executable on `PATH`, a CMS-signed object and a supplied identity yield `unverifiable` with reason `gitsign_not_started`, exit 0. | Test |
| FR-040-AC-7 | A simulated timeout and a simulated output above 8 MiB at the gitsign adapter boundary yield `unverifiable` with reason `timeout` and `output_exceeded`. | Test (stand-in) |
| FR-040-AC-8 | An unsigned commit and tag still yield `unsigned` with no `signature_kind`, and an OpenPGP-signed commit and tag still yield `unverifiable` with reason `unsupported_signature_format`, whether or not `gitsign` is supplied. | Test |
| FR-040-AC-9 | A commit whose signature headers mix CMS and SSH yields `unverifiable` with reason `unsupported_signature_format`, and a commit whose `gpgsig` header begins with the CMS marker but has trailing characters on the same line is still detected as CMS. | Test (captured) |
| FR-040-AC-10 | The classifier maps the format, identity-supplied flag, repository-config flag and invocation outcomes (exit 0; 1 then wildcard 1; 1 then wildcard 0 with each issuer and identity probe result; exit 2; other exit status; not started; timeout; cut short) to the statuses, `mismatch` values and reasons above, without running `gitsign` or git, and the classification does not change when the captured message text is replaced. | Test (captured) |
| FR-040-AC-11 | A request whose `gitsign` member lacks a field, has an empty or oversized value, a leading `-` identity, a control character, or an unknown member yields `signature_verify_request_invalid` and exit status 2, and does not start git or `gitsign`. | Test |
| FR-040-AC-12 | An SSH-signed commit and tag in a request that also carries `gitsign` yield the FR-036 and FR-037 statuses, signer principal and mapping unchanged, with `signature_kind` `ssh` and `signer.source` `allowed_signers`, and `gitsign` is never started. | Test (stand-in) |
| FR-040-AC-13 | A gitsign-signed commit and tag whose message carries a `Signed-off-meaning:` trailer report it as FR-037 does, and an `unverifiable` gitsign commit still reports it. | Test (stand-in) |
| FR-040-AC-14 | The `signature-verify` feature and the binary add no crate dependency that performs signature verification, cryptography or Sigstore access; the only verifying programs invoked are git, `ssh-keygen` and `gitsign`. | Inspection |
| FR-040-AC-15 | Against real `gitsign` and the network, a public gitsign-signed commit and tag yield `signed_valid` for their true identity and issuer, `signed_untrusted` with `mismatch` `issuer` for a wrong issuer, and `unverifiable` with reason `gitsign_verification_failed` when the network is unavailable. | Test (live, ignored by default) |
| FR-040-AC-16 | The gitsign argument vectors, including `verify`, `verify-tag`, the four certificate flags and the wildcard values, match the command line documented for the gitsign release the adapter targets. | Inspection |
| FR-040-AC-17 | A CMS-signed commit in a repository that sets `gitsign.fulcioRoot` (or any other `gitsign.*` key) in its local configuration, in a file named by `include.path`, or in `config.worktree` with `extensions.worktreeConfig` enabled, yields `unverifiable` with reason `gitsign_repository_config`, and the stand-in is never started; a repository with no such key proceeds. | Test (stand-in) |
| FR-040-AC-18 | The stand-in's recorded environment holds exactly `PATH`, `HOME`, `GIT_CONFIG_NOSYSTEM` and `GIT_CONFIG_GLOBAL`, even when the parent environment sets `SIGSTORE_ROOT_FILE`, `GITSIGN_FULCIO_ROOT`, `TUF_ROOT` and `GIT_DIR`. | Test (stand-in) |
| FR-040-AC-19 | A lightweight tag yields `unsigned` with reason `lightweight_tag`, and a tag revision that resolves to no annotated tag under `refs/tags/` yields `unverifiable` with reason `revision_unresolved`, without starting `gitsign`. | Test (stand-in) |

## Dependencies

- **Upstream**: [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md) for
  the request, verdict, classifier and hardening;
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md) for the signer and
  trailer model; [FR-019](./FR-019-bounded-producer-execution.md) for the
  process host limits.
- **Downstream**: consumers that record signed approvals and read this result as
  data.
