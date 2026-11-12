---
id: SR-16404
title: "spec-review of SSH-signed commit and tag verification (engineering-assurance PR 209)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance; spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md; spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md; spec/spec.md"
review_set: base
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-036
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-037
    type: references
---

# Spec review of SSH-signed commit and tag verification (engineering-assurance PR 209)

## Summary

Ticket: PLAT-1431 (implementation tickets PLAT-1432, PLAT-1433). Scope is `git diff origin/main...HEAD` only: two new FRs and two index lines in `spec/spec.md`. `quire validate --scope .` over the new FRs exits 0 with six grammar warnings. Base checklist run; failure-domain and EARS lenses applied inline in this one document rather than as separate SpecReviews. No AssuranceProfile instance in the repo applies, so no enforced run is required.

Git behaviour was measured, not assumed: a throwaway repository with two freshly generated fictional ed25519 keys under git 2.34.1, with system and global config disabled. Observed:

- No `gpg.ssh.allowedSignersFile`: `git verify-commit` on a validly signed commit prints `error: gpg.ssh.allowedSignersFile needs to be configured and exist for ssh signature verification` and exits 1.
- Key absent from the allowed-signers file: `Good "git" signature with ED25519 key SHA256:...` then `No principal matched.`, exit 1; `%G?` is `U`.
- Key present: `Good "git" signature for alice@example.test with ED25519 key SHA256:...`, exit 0; `%G?` is `G`.
- Unsigned commit: exit 1 with empty output. Unsigned annotated tag: `error: no signature found`, exit 1. Lightweight tag: `error: light: cannot verify a non-tag object of type commit.`
- Signed annotated tag: the SSH signature block is appended to the tag message after the trailer, so `git interpret-trailers --parse` over the raw message finds no trailer; over subject plus body without the signature it finds `Signed-off-meaning: release`. An empty-valued trailer is kept by `--parse`.

Checked and found consistent:

- Ticket ACs (PLAT-1432/1433, read as data): delegation to git with no own crypto (FR-036 Behavior, CON-2, AC-9); unsigned, bad and valid distinct (AC-1..3, AC-8); signer key named (`key_fingerprint`, AC-1); signer mapped or unmapped (FR-037 AC-1..3); trailer parsed (AC-5); missing trailer reported, not defaulted (AC-6, CON-2). All are covered in text; FND-001 and FND-002 make two of them unachievable as written.
- FR-032 pattern: request/result protocol names, exit 0 for every verdict, exit 2 with `engineering-assurance.error/v1`, pre-git revision refusal, pure core in a Cargo feature with the git adapter in the binary. Matches; the 256-byte revision bound matches the existing `MAX_REVISION_BYTES` in the scope-footprint adapter.
- FR-019 process-host limits: delegated to the shared process host as FR-019 requires of existing binary adapters; bounds themselves are not stated (FND-005).
- quoin FR-063/FR-065: no contradiction. Those requirements bind Quoin's own record and receipt (no Git command, no identity or signature claim); FR-036/037 live in Engineering Assurance and say consumers read the result as data. A future Quoin consumer that records signer identity would need its own Quoin requirement; that is outside this PR.
- SSH-only scope: gitsign/Sigstore, X.509 and OpenPGP are stated out of scope; what such a signature yields is not (FND-004).
- Public-repo hygiene: fixtures are fictional test keys; no real keys, no roadmap, delivery order or research content, no quoted standard text. No SHA, pin, digest, version or file-tracking ceremony added.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-037-AC-3 cannot pass: with no `allowed_signers` member git refuses to verify any SSH signature (`gpg.ssh.allowedSignersFile needs to be configured`, exit 1), so a validly signed commit can never reach `signed_valid` + `unmapped`. Make `allowed_signers` required (or define its absence as `unverifiable` with a named reason), and delete FR-037-AC-3 and the "Without an `allowed_signers` member" bullet. | spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:45-46,79; spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:37-40 |
| FND-002 | high | FR-037-AC-2 contradicts FR-036-AC-3 and FR-036-CON-1. A key absent from the allowed-signers file makes git print a good signature, `No principal matched.`, and exit 1. FR-036-CON-1 forbids `signed_valid` on a nonzero exit and FR-036-AC-3 calls this case `signed_bad`, yet FR-037-AC-2 expects a valid signature reported `unmapped`, and FR-037 Outputs allow `signer.mapping` only on `signed_valid`. Choose one model; recommended: a distinct status (for example `signed_untrusted`: signature good, key not in the file) carrying `key_fingerprint` and `mapping: unmapped`, so `signed_valid` always means mapped and tampering stays `signed_bad`. Update FR-036 statuses, CON-1, AC-3, AC-8 and FR-037 Outputs/AC-2 together. | spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:32,40-44,78; spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:64-69,93,102 |
| FND-003 | medium | For a signed annotated tag, git appends the SSH signature block to the tag message after the trailer, so `git interpret-trailers --parse` over "the object's message" finds no trailer and FR-037-AC-8 would report `missing`. Say the trailer is read from the tag message with the signature block removed (subject and body), and make AC-8 assert a trailer present on a signed tag. | spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:51-54,84 |
| FND-004 | medium | Non-SSH signatures are out of scope but no verdict is defined for meeting one. An OpenPGP, X.509 or gitsign signature is neither "good SSH", "no signature" nor a named non-signature failure, so it could be classified `signed_bad` or `unsigned`. Specify `unverifiable` with a named reason (for example `unsupported_signature_format`) and add a classifier case to AC-8. | spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:26-29,64-74,107 |
| FND-005 | medium | FR-036-AC-5's timeout and capture-bound cases name no bound and no seam. FR-036 cites FR-019 limits without values (FR-032 states 8 MiB; the existing adapter uses 60 s), and real `git verify-commit` output is a few lines, so "output above the capture bound" is unreachable end to end. State the timeout and capture bound, and say these two cases are driven through an injected process outcome at the adapter seam, as the scope-footprint adapter's oversized-output test does. | spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:58-61,104 |
| FND-006 | medium | `reason` is defined only for `unverifiable`, but Behavior gives `unsigned` the reason `lightweight_tag`, and no reason vocabulary is closed, so AC-5's "with a reason" passes for any string. Declare `reason` for `unverifiable` and the lightweight-tag `unsigned`, and list the closed reason codes (git not started, revision unresolved, not a repository, timeout, capture exceeded, unrecognized output, plus FND-001/FND-004's). | spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:44-46,75-77,104 |
| FND-007 | medium | The classifier keys on git's free status text and says `unsigned` is when "git reports that the object carries no signature", but for an unsigned commit git 2.34.1 prints nothing and exits 1. Name the observed signals the classifier maps, or classify on git's closed `%G?` codes (`G`, `B`, `U`, `X`, `Y`, `R`, `E`, `N`) from `git log`/`git for-each-ref` alongside the verify call, which are stable across git versions where the text is not. | spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:64-69,107 |
| FND-008 | low | `principal` is defined twice: FR-036 outputs a top-level `principal` "as git reports" for good or bad signatures, and FR-037 adds `signer.principal` from the allowed-signers file. Git only names a principal when the file matches, so they are the same fact. Keep one (FR-037's `signer`) and drop `principal` from FR-036. | spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:44-46,78-80; spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:30-31 |
| FND-009 | low | Pre-validating that each non-comment line "parses as principals, options and a key type and key" re-implements ssh-keygen's allowed-signers grammar (quoted principals, `namespaces=`, `valid-after=`, `cert-authority`) and can diverge from git. Keep the cheap file checks (regular file, 1 MiB, UTF-8) and let git/ssh-keygen reject a malformed line, mapped to the machine error. | spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:47-50,80 |
| FND-010 | low | FR-036 `relationships` omit FR-029 (cited for the exit-code convention) and FR-014 (Cargo feature discipline, which FR-032 cites for the same feature pattern). Add both, and FR-014 to Dependencies. | spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:5-11,112-117 |
| FND-011 | low | `quire validate` grammar warnings: two non-singular `shall` statements (FR-036 lines 68 and 94) and four agentless passives (FR-036:78 `shall be copied`; FR-037:45, 47, 60). Split and name the agent. | spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:68,78,94; spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:45,47,60 |

## Verdict

The design is right: git does the cryptography, the result is a closed classification, and attribution and meaning are reported, never inferred. The FR-032 command pattern is followed faithfully, nothing contradicts quoin FR-063/FR-065, and the public-repo hygiene is clean. Two high findings come from git's measured behaviour: the optional allowed-signers file and the "valid but unmapped" outcome cannot both exist under `git verify-commit` (FND-001, FND-002). Not mergeable until FND-001 through FND-007 are fixed; the lows are small edits in the same files.

## New findings (disposition pass 1)

Measured again in a throwaway repository with fresh fictional ed25519 keys, git 2.34.1, system and global config disabled, against fix commit 82c8e9f.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | high | Kind `tag` has no `%G?` source on the installed git. `%G?` is a `git log` placeholder for commits only; the ref-filter atom `%(signature:grade)` is refused by both `git for-each-ref` and `git verify-tag --format` (`fatal: unknown field name: signature:grade`), and FR-036 forbids classifying from git's message text. So FR-036-AC-1 and AC-2 cannot pass for tags on git 2.34.1. State how the adapter obtains a tag's grade and the minimum git release that provides it, and what an older git yields (for example `unverifiable` with a named reason), with an AC. | spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:38-41,80-83,127-128 |
| FND-013 | high | FR-037-AC-4 contradicts measured git and FR-036's no-message-text rule. git skips an allowed-signers line it cannot parse, prints `<file>:1: invalid key`, and grades from the rest: a file whose only line is bad yields `%G?` `U` (`signed_untrusted`), and a bad line plus the signer's valid line yields `G`, exit 0 (`signed_valid`). AC-4 allows only `signature_verify_allowed_signers_invalid` or `unrecognized_output` and never `signed_valid`, and "git names the file as the cause" can only be detected from message text. Drop the two "git rejects a line" bullets, state that a line git cannot parse authorizes no key, and rewrite AC-4 to those two measured outcomes. | spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:50-53,85 |
| FND-014 | medium | An allowed-signers entry whose `valid-before` has passed yields `%G?` `U` (`key has expired`, `No principal matched.`, exit 1), so an expired but named key is reported `signed_untrusted` and `unmapped`, while FR-036 defines `signed_untrusted` as a key "the file does not name" and puts "expired or revoked" under `signed_bad` via `X`/`Y`/`R`. Revocation needs `gpg.ssh.revocationFile` (then `B`), which the request cannot set. Define `signed_untrusted` as a good signature by a key the file does not accept at signing time (not named, or outside its validity window), drop the SSH expiry/revocation claim from `signed_bad` or add a revocation input, and add an expired-entry AC. | spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:57-60; spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:15-18,40-43 |

## New findings (disposition pass 3)

Measured against fix commit be91124 with fresh fictional ed25519 keys on git 2.34.1 (throwaway repository, deleted): a tag signed by a named key gives `git verify-tag` 0; by an unnamed key, `verify-tag` 1 and `ssh-keygen -Y check-novalidate -n git` 0; a tag altered after signing, `verify-tag` 1 and `check-novalidate` 255. The tag mechanism grades the tag object as specified.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-015 | high | The new tag order makes "no `-----BEGIN SSH SIGNATURE-----` block" mean `unsigned`, so a tag carrying an OpenPGP or X.509 signature block is reported `unsigned`. That contradicts the FR-036 rule that a non-SSH signature header yields `unverifiable` with `unsupported_signature_format` and never `unsigned`. Make the first step "no signature block of any format is `unsigned`", add "a non-SSH signature block is `unverifiable`/`unsupported_signature_format`" before `git verify-tag`, and extend FR-036-AC-10 to a tag. | spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:94-95,119-121,158 |
| FND-016 | medium | For a tag, no source is given for the signer principal or key fingerprint. FR-037 Outputs require `signer.principal` on every `signed_valid` (mapped) result, but `%GS`/`%GK` apply to commits only and git's and ssh-keygen's message text is excluded, so a `signed_valid` tag has no specified principal and no AC checks one. Name the source (for example `ssh-keygen -Y find-principals -s <sig> -f <allowed_signers>`, whose stdout is the principal list; measured to print the principal for a named key and exit 255 for an unnamed one), and either name a fingerprint source for tags or state that tag results carry no `key_fingerprint`; add a tag case to FR-037-AC-1. | spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:54-55,92-101; spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:30-36,79 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 82c8e9f: `allowed_signers` is required in the FR-036 request; a request without it is a machine error (FR-036-AC-8); the unmapped-without-file AC and bullet are gone. |
| FND-002 | fixed | 82c8e9f adds `signed_untrusted` (`%G?` `U`) with `mapping` `unmapped`; CON-1 requires exit 0, `G` and a named key; FR-036-AC-3 and FR-037-AC-2 agree. |
| FND-003 | fixed | 82c8e9f: the trailer is read over subject and body with the tag's trailing signature block removed first; FR-037-AC-8 asserts presence on a signed tag. |
| FND-004 | fixed | 82c8e9f: a non-SSH signature header yields `unverifiable` with `unsupported_signature_format`, never `signed_bad` or `unsigned`; FR-036-AC-9 and AC-10 cover it. |
| FND-005 | fixed | 82c8e9f states the 60 s timeout and 8 MiB capture bound; FR-036-AC-6 drives timeout and overflow through a simulated process outcome at the adapter boundary. |
| FND-006 | fixed | 82c8e9f declares `reason` for `unsigned`/`lightweight_tag` and `unverifiable` with a closed list of eight codes; FR-036-AC-5 names the exact reasons. |
| FND-007 | fixed | 82c8e9f classifies on `%G?` codes and exit status and forbids message-text classification. For commits this matches measured git; the tag path is FND-012. |
| FND-008 | fixed | 82c8e9f removes FR-036's `principal`; FR-037's `signer` is the only principal member. |
| FND-009 | fixed | 82c8e9f keeps only the regular-file, 1 MiB and UTF-8 checks and forbids re-implementing the grammar. The replacement rule for rejected lines is FND-013. |
| FND-010 | fixed | 82c8e9f adds FR-014 and FR-029 to FR-036 relationships and FR-014 to Dependencies. |
| FND-011 | fixed | `quire validate` over FR-036 and FR-037 at 82c8e9f exits 0 with no grammar warnings. |
| FND-012 | still-open | 27e6604 grades tags with `git for-each-ref --format=%(signature:grade)`. That atom exists from git 2.42.0 (release notes: for-each-ref "learned placeholders related to GPG signature verification"; merge of ks/ref-filter-signature is an ancestor of v2.42.0), but in git's ref-filter.c `grab_values` calls `grab_signature` only for `OBJ_COMMIT`; the `OBJ_TAG` branch never grabs a signature. On a tag ref the atom is therefore empty on every git release, and `%(*signature:grade)` would grade the tagged commit, not the tag (a CON-1 hazard). `git_too_old` detection is sound for git before 2.42 but never fires on newer git, where the empty value is unspecified. So the tag halves of FR-036-AC-1/2/3 and the tag part of FR-037-AC-9 cannot pass with the specified mechanism on any git, and the spec states no skip condition. Still missing: a tag grading mechanism that yields G/U/B/N for the tag object itself (for example: tag verified by `git verify-tag` exit status for G, with U versus B decided by a delegated `ssh-keygen -Y check-novalidate` over the tag payload and signature split at the signature header), or an explicit spec decision that kind `tag` is `unverifiable` until such a mechanism exists, with the tag ACs rewritten to that. |
| FND-013 | fixed | 27e6604 drops the "git names the file" rules and states that an unparsable line authorizes no key and git grades from the remaining lines; FR-037-AC-4 now asserts bad-only file gives `signed_untrusted` and bad plus valid line gives `signed_valid`, matching the round 1 measurement on git 2.34.1. |
| FND-014 | fixed | 27e6604 defines `signed_untrusted` as a key the file does not accept at signing time (not named, or outside its validity window), restricts `signed_bad` to grade `B`, maps `X`/`Y`/`R` to `unverifiable`/`unrecognized_output`, and adds FR-037-AC-9 for an expired entry. The tag half of AC-9 depends on FND-012. |
| FND-012 | fixed | be91124 grades the annotated tag object itself: no SSH signature block is `unsigned`, `git verify-tag` exit 0 is `signed_valid`, otherwise `ssh-keygen -Y check-novalidate -n git` over the extracted signature and the tag payload decides `signed_untrusted` or `signed_bad`; `ssh_keygen_not_started` replaces `git_too_old`. Measured on git 2.34.1: good 0, unnamed key 1 then 0, tampered 1 then 255. The non-SSH-block gap this order opens is FND-015. |
