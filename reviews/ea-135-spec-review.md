---
id: SR-20261115040623
title: "spec-review/base review of EA-135 gitsign identity registry"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance; README.md; spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md; spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md; spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md; spec/functional/FR-042-create-signed-approval-tag-for-a-change-assurance-record.md; spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md; spec/functional/FR-047-release-record-and-signed-release-tag.md; spec/functional/FR-080-signed-proof-integrity-statement-verification.md; spec/tests.md"
review_set: subset
---

## Summary

Ticket: EA-135. PR agent-ix/engineering-assurance#299 at 9ef3cad1e. Base checklist over the
diff `origin/main...HEAD`: the gitsign person mapping moves from `gitsign.principal` in the
request to a committed identity registry read as `<rev>:<path>`. The intent is carried through
FR-037/040/041/042/043/047/080 and README; no stale request-sourced mapping text remains in
spec, README, docs or schemas (grep over `gitsign.principal`, `source` `request`, "request's
principal", "configured principal"). Two ACs contradict the behaviour they sit under (FND-001,
FND-002); the remaining findings are cross-spec gaps.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-040-AC-26 requires `signature_verify_gitsign_principal_refused` for a `gitsign` member carrying `principal` "when `identity_registry` is absent", but FR-040 Behavior orders "validate the request, then refuse a `gitsign` member that carries `principal`", and Outputs/AC-11 make `gitsign` without `identity_registry` a request-validation failure (`signature_verify_request_invalid`). The same input has two required codes. Fix: either make the principal refusal precede all other `gitsign`/`identity_registry` validation, or drop the "absent" case from AC-26. | spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md:216-219,319,334 |
| FND-002 | high | FR-041-AC-10 ends "an unusable identity registry writes `approval_verify_identity_registry_invalid` ... none starts git", but FR-037 Trust-source reading resolves `<rev>` and reads the blob by git object, so detecting an unresolvable revision or absent path must start git. The AC cannot be met. Fix: restrict "none starts git" to the request-validation and principal-refusal cases, and say the registry refusal starts no signature verifier and does not touch the target (as FR-040-AC-27). | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:249; spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:79-93 |
| FND-003 | medium | The closed error-code lists were not amended: FR-036 Outputs ("The codes are ...") omits `signature_verify_gitsign_principal_refused` and `signature_verify_identity_registry_invalid`; FR-043 Outputs and FR-047 Outputs name their new codes only in Inputs, while their Outputs code lists stay closed without them; FR-043-AC-10 and FR-047's ACs never test them; FR-080 names no code for either refusal. A coder working from Outputs ships no such code. | spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md:90-96; spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md:57,104-109,184; spec/functional/FR-047-release-record-and-signed-release-tag.md:67,96-100; spec/functional/FR-080-signed-proof-integrity-statement-verification.md:63 |
| FND-004 | medium | FR-080 decision rule (4) assigns statuses to `unsigned`, `signed_bad`, `unverifiable`, `untrusted_signer` and "`signed_valid` with a mapped signer"; before this PR a gitsign `signed_valid` was always mapped, now FR-040 yields `signed_valid` + `unmapped` for an identity the registry does not name, and FR-080 gives it no status. FR-041 got the explicit `untrusted_signer` rule; FR-080 did not, nor an AC (FR-080-AC-5). | spec/functional/FR-080-signed-proof-integrity-statement-verification.md:69,100 |
| FND-005 | low | README `signature-verify` row still ends "with the `signer` principal the file maps the key to", which for gitsign is now the registry, not the allowed-signers file. | README.md:297 |

## Verdict

FAIL, not mergeable as is: fix FND-001 and FND-002 (both one-line AC/ordering edits) and FND-003/FND-004 (cross-spec amendments in the same PR). The code is implementable: `GitsignIdentity` drops `principal`, `classify_gitsign`/`probe_gitsign` take a registry lookup in place of `identity.principal` (src/signature_verify.rs), `SignerSource::Request` becomes a registry source, and the registry read can reuse the host git runner, which already sets `GIT_NO_REPLACE_OBJECTS` and a dead graft file (src/signature_verify_host.rs). FR-080 has no host yet (no proof-checkpoint code in src/), so its amendment is spec-only.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | FR-040-AC-31 requires "every verdict for a request with `gitsign`" to carry `identity_registry_commit`, but FR-040-AC-30 requires an `unverifiable` verdict, at exit 0, for a registry reading that is `git_failed`. In a directory that is not a repository, or with a git that cannot start, no commit was resolved, so no such id exists. FR-040 Outputs say the member is added only "whenever the identity registry was read", which fits AC-30. FR-041-AC-14 repeats the "any other status" form for `trust.identity_registry_commit`. Fix: in AC-31 and FR-041-AC-14, say "whenever the registry revision resolved", and have AC-30 assert the member is absent. | spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md:127-129,352-353; spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:254 |
| FND-007 | low | Test-plan rows the fix round did not update: TC-310 (FR-041-AC-9) still varies only signature status, bindings and meaning, not signer mapping; TC-325 (FR-041-AC-14) does not check `identity_registry_commit`; TC-1026 (FR-080-AC-5) lacks the registry-unnamed identity case that AC-5 now lists. | spec/tests.md:735,750,1049 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-040 Behavior now validates the request first and refuses `principal` only after it validates; AC-11 covers `gitsign` without `identity_registry` "with or without a `principal`", and AC-26 applies only to a request that otherwise validates. One code per input. |
| FND-002 | fixed | FR-041-AC-10 now says none of the refusals starts a signature verifier, only the registry refusal starts git, and a principal refusal comes before git. |
| FND-003 | fixed | FR-036, FR-043 and FR-047 Outputs code lists carry both new codes; FR-043-AC-10 and the new FR-047-AC-14 test them; FR-080 names `proof_checkpoint_verify_gitsign_principal_refused` and `proof_checkpoint_verify_identity_registry_invalid`. |
| FND-004 | fixed | FR-080 rule (4) makes `signed_valid` with an `unmapped` signer `untrusted_signer`, and FR-080-AC-5 tests it. |
| FND-005 | fixed | The README row now names the allowed-signers file for SSH and the identity registry for gitsign. |
