---
id: SR-20261124025459
title: "spec-review (base) of trusted signing time and reproducible trust identity (engineering-assurance PR 303)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@fe432b49 (PR 303, diff origin/main...HEAD); spec/functional/FR-036; spec/functional/FR-037; spec/functional/FR-040; spec/functional/FR-041; spec/functional/FR-043; spec/functional/FR-047; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-041
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-037
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-040
    type: references
---

# Spec review (base) of EA PR 303

## Summary

Ticket: EA-136 (trusted signing time from Rekor), EA-138 (reproducible trust identity and EA version). Review set: subset (base, integrity, ears-conformance); separate SpecReview files carry the integrity and EARS lenses. No AssuranceProfile in the repo requires an enforced spec-review run (the only AssuranceProfile file is the module skeleton), so this is a direct review. `quire validate --scope .` on each changed FR file (quire 0.52.0, engine 0.62.2) exits 0; the only output is the pre-existing `DuplicateModuleName` notice. No cargo, tests or make ci were run (spec-only).

Measured against the dependency the spec relies on: `sigstore-verify` 0.16.0 (Cargo.lock) keeps every transparency-log routine (`verify_set`, `verify_entry_inclusion`, `verify_tlog_entries`) in the private module `verify_impl`; its public surface is `Verifier::new(&TrustedRoot)` and `Verifier::verify(artifact, &Bundle, &policy)`. No `sigstore-*` 0.16.0 crate exposes a CMS parser (`sigstore-tsa`'s ASN.1 module is private). In this repo `sigstore-verify` is enabled only by the `change-record` and `attestation-verify` features (not `signature-verify` or `approval-verify`), with `default-features = false` and no `tuf` feature, and every current caller loads the compiled-in `SIGSTORE_PRODUCTION_TRUSTED_ROOT` / `SIGSTORE_GITHUB_TRUSTED_ROOT` constant.

Ids: every new id (FR-037-AC-23..25, FR-037-CON-3, FR-041-AC-27..35, FR-041-CON-5, FR-043-AC-26..27, FR-047-AC-15..16, TC-1101..1111) is absent from the full git history of its file and of the repo; no id is reused. No paywalled standard text is quoted (RFC 3161 is named only). No score, class or acceptability judgement is introduced: `time_skew` is a raw signed number.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The trusted-time path is unsatisfiable as constrained, and its feasibility is left open. Reading the Rekor entry from the tag's CMS unsigned attribute, and checking that the entry's certificate and signed-content digest equal the tag's, needs CMS/ASN.1 and certificate parsing. FR-041-CON-2 forbids signature or certificate parsing code. FR-040-CON-2 allows only code inside `sigstore-verify`. FR-041-AC-33 forbids certificate code. `sigstore-verify` 0.16.0 has no public CMS reader and no public tlog routine; only `Verifier::verify(artifact, Bundle, policy)` is public, so a Bundle would have to be assembled from parsed CMS parts. The fallback ("if the library offers no public way to verify ... every gitsign result is `log_entry_unverified`") covers only verification, not extraction, so `no_log_entry` still needs the parsing it forbids. Under that fallback FR-041-AC-28 (`trusted_at` present) and the distinct AC-29/AC-30 outcomes can never pass. Resolve feasibility before merge with a spike against a captured gitsign tag. Then do one of two things: name the exact library path, with any parse exception explicitly scoped; or reserve `transparency_log` like `rfc3161` and drop the gitsign trusted-time ACs. | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:246-259; FR-041-CON-2; FR-041-AC-28; FR-041-AC-29; FR-041-AC-30; FR-041-AC-33; FR-040-CON-2 |
| FND-002 | medium | The trusted-root source is ambiguous. "The verification library's own trusted-root support" could mean the compiled-in `SIGSTORE_PRODUCTION_TRUSTED_ROOT` every current caller uses, or a TUF fetch. The compiled-in root is offline and can never be "unavailable", so `trust_root_unavailable` is dead. It also goes stale when Rekor keys rotate, and it is an offline trust root, which FR-040-CON-2 keeps excluded. A TUF fetch needs the `tuf` feature that `sigstore-verify` is not built with here. AC-28/29 ("captured ... with its trusted root") and AC-30 (root unavailable, stand-in) also need the root injectable. State which source is used, how a test injects it, and when `trust_root_unavailable` occurs. | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:266-269; FR-041-AC-30; FR-040-CON-2; Cargo.toml:625 |
| FND-003 | medium | The trust identity is not reproducible across hosts. The spec states that the SSH verdict depends on the host git version: before 2.35, `valid-before` is judged at verification time; from 2.35, at the claimed tagger date. FR-037-AC-23 and FR-041-AC-31 make the same backdated tag `approved` on git 2.35 or later but `untrusted_signer` on 2.34. Yet `trust` records `ea_version` and the allowed-signers identity, not the git version that decided the verdict. A re-run on another host can disagree, and nothing in the result explains why. EA-138's goal is a result an auditor can reproduce. Report the git version, as run, in `trust`, or state why it is not needed. | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:104-112; FR-041-AC-31; FR-037-AC-23 |
| FND-004 | low | FR-041-AC-31 hard-codes a host fact ("git 2.34.1 on this host") into a requirement. The skip condition (older than 2.35) is already stated; drop the parenthesis. | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:333 |

## Verdict

FAIL for round 1 on FND-001 (high). The EA-138 half is sound and testable. That half covers the `<rev>:<path>` allowed-signers source through the shared trust-source reading, read-once into a private temp file, the single owner-approved `allowed_signers_sha256` content identity confined by FR-037-CON-3, `ea_version`, and the top-level `trust` on every release verdict. The FR-037-AC-9 rewrite holds on every git version. FR-037-AC-24/25, FR-043-AC-26/27 and FR-047-AC-15/16 are testable.

The EA-136 half is coherent for SSH. An SSH approval reports `trusted_at_missing` `ssh_signature`, and the backdated-tag-vs-expired-key behaviour on git 2.35 or later is reported as an unattested claim, not judged. That keeps the risk decision with people, as required, and RFC 3161 stays a reserved seam. The gitsign trusted-time path, however, is specified against a library surface that does not exist publicly in the pinned version, and the spec relies on a conditional instead of resolving the question.

On `trusted_at_missing`, which the lead asked about: it adds value and is not excess. It separates evidence absent (`no_log_entry`), evidence present but failing verification (`log_entry_unverified`, a possible tamper an auditor must see) and a check that could not run (`trust_root_unavailable`, retryable). `ssh_signature` is redundant with `signature_kind`, but it keeps the "exactly one of" shape. If FND-001 resolves to "not feasible", the reasons collapse and should shrink with it.

## New findings (disposition pass 1)

Reviewed fix commit 288c440e against fe432b49. `quire validate` exits 0 on all six changed FRs; its grammar warnings are judged in SR-20261124025461.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | FR-041-AC-30 keeps "in neither case" after its second case (`trust_root_unavailable`) was removed, so the sentence now refers to two cases when only one remains. | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:348 |
| FND-006 | low | New FR-041-AC-36 states both halves ("`approved` with a `git_version` of 2.35 or later and `untrusted_signer` with an earlier one"). A host has one git, so a run can observe only one half. Phrase it as a check against the host's git, the way AC-31 skips. | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:354 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Under the lead's ruling, FR-041-CON-2, FR-040-CON-2 and FR-041-AC-33 now allow a CMS/DER reader for extraction only (the Rekor entry with OID 1.3.6.1.4.1.57264.3.1, the certificate and the signed-content digest), with all verification in the public `Verifier::verify`. The design-time conditional is gone; outcomes are runtime ones (no entry gives `no_log_entry`; a bundle that cannot be built or is rejected gives `log_entry_unverified`), so AC-28/29/30 are each reachable. Whether gitsign's Rekor entry forms a bundle the Verifier accepts is still unproven. If it does not, AC-28 fails at implementation, not in the spec. |
| FND-002 | fixed | FR-041 Behavior now names the compiled-in `SIGSTORE_PRODUCTION_TRUSTED_ROOT`, makes it injectable for tests, forbids fetching, and maps a rotated-key failure to `log_entry_unverified`. `trust_root_unavailable` no longer appears anywhere in spec/. FR-040-CON-2 states the one offline root in use. |
| FND-003 | fixed | `trust.git_version` (as `git --version` prints it) was added to FR-041 Outputs and propagated to FR-043, FR-047 and FR-041-AC-35. New AC-36 ties the recorded version to the 2.34 / 2.35+ verdict difference. |
| FND-004 | fixed | FR-041-AC-31 now ends "skipped, not passed, on a git older than 2.35"; the host fact is gone. |
