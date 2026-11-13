---
id: SR-16419
title: "Spec review of the FR-041 signing-time amendment (engineering-assurance PR 218)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@fbb6e2f; spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md (description, Outputs, Behavior, CON-3, AC-16..AC-21); spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-041
    type: references
---

# Spec review of the FR-041 signing-time amendment (engineering-assurance PR 218)

## Summary

Ticket: PLAT-1592. `quire validate` accepts FR-041 and spec/tests.md (exit 0). Its two warnings are module-level and existed before this change.

The amendment states the time source precisely. It is the tagger line of the header block in the object read with `git cat-file tag`, never the message, with exactly one tagger header, a bounded epoch and a validated zone. It states that the zone never moves the instant, and it forbids defaulting the time to the verification time, the current time, the commit date or the author date. It confines the time to `approved` and gives a reason for that, and it leaves `signature-verify` unchanged.

The gitsign limit is stated plainly. I verified the claim against the sigstore/gitsign source at v0.18.0 (the latest release, 2026-10-20). `internal/commands/verify.PrintSummary`, which `verify-tag` also uses, prints only the tlog index, the certificate fingerprint and issuer, the "Good signature" SAN line and the claim lines. `gitsign show` prints an in-toto statement from `pkg/attest`. Neither prints the Rekor integrated time.

Reporting the tagger date with `signed_at_source: tagger_date` for gitsign is honest, not misleading, for two reasons:
- The source is named.
- In v0.18.0 the line is inside the raw payload that gitsign verifies (`pkg/git.SplitTag`).

Reserving `transparency_log` invents no output format. The CON-3 and AC-16..AC-21 statements are testable as written, apart from FND-003.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The amendment calls the time "the signer's own claim" but never says what follows from that. It does not state that a future date, epoch 0, or a date before the signing key existed or outside its validity is reported verbatim, with no plausibility check. A consumer cannot tell whether `signed_at` was checked. Two facts make the gap material: (a) for gitsign the tagger date is not checked against the short-lived certificate's validity window, because gitsign checks the certificate at the Rekor integrated time, so the date is entirely uncorroborated; (b) for SSH, git 2.35 and later passes the same claimed tagger date to ssh-keygen as `-Overify-time` (gpg-interface.c), so allowed-signers `valid-after`/`valid-before` are judged at the time the signer claims. Fix: add a Behavior SHALL that `signed_at` is reported exactly as claimed, with no comparison to the current time, the key's creation time or the certificate's validity, and that a future or epoch-0 date is reported as is. State (a) and (b). Optionally add an AC for an approved tag dated epoch 0 or in the future. | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:33-47,159-171 |
| FND-002 | medium | The spec asserts a fact that is not true. It says `transparency_log` stays reserved until a later amendment "(an owner decision recorded in the pull request)". The PR #218 body records the coder's decision "for owner confirmation" under "Spec point needing a call", and no owner decision exists. A normative document should not cite a decision that has not been made, or point to a PR thread for its authority. Fix: delete the parenthetical and keep the reservation as the requirement itself, or obtain the owner's decision before merge and still drop the PR pointer. | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:43-47 |
| FND-003 | low | Untestable clause: FR-041-AC-18 ends "and no member holds the verification time". The verification time is nondeterministic and appears in no member the spec defines, so no test can fail on this clause as worded. TC-337 checks only that `signed_at` and `signed_at_source` are absent. Fix: reword it to "and the result carries no time member other than `signed_at_missing`", which TC-337 can assert by checking the member set. | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:221 |

## Verdict

The amendment is precise about where the time comes from, the strict format, the rule that the time is never defaulted, confinement to `approved`, and the unchanged `signature-verify` output. The gitsign limit is correct and stated plainly. Two medium gaps must be fixed: FND-001 (what "signer's claim" means for plausibility and key validity) and FND-002 (a cited owner decision that does not exist). FND-003 is a wording fix.
