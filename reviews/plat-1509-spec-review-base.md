---
id: SR-142
title: "Base spec review of engineering-assurance PR 214 (FR-041 approval-verify)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@b957ab8; spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md, spec/spec.md"
review_set: subset
---

# Base spec review of engineering-assurance PR 214

## Summary

Ticket: PLAT-1509. Scoped to `git diff origin/main...HEAD` at b957ab8 (new FR-041 and
its spec.md index line). Reviewed together with agent-ix/quoin PR 836 as one unit. Tools:
quoin 0.30.1, quire 0.44.0 (engine 0.59.1), git 2.34.1. `quire validate` passes with
three EARS warnings (see the ears-conformance review). Assurance profiles: none in this
repository; disposition not applicable.

The ticket's design decisions are honoured: a signed annotated tag is the only approval
object, the tag message carries `Approved-Record-Id` and `Approved-Record-Digest`,
meaning comes from `Signed-off-meaning` with no default, and the signature rules are
delegated to FR-036/037/040. Real-behaviour checks on git 2.34.1 with fictional
ssh-keygen keys confirmed: the signature block sits directly after the trailer block, so
`git interpret-trailers --parse --no-divider` finds no trailer unless it is stripped
first; a value containing `:` (`sha256:<hex>`) survives intact; a repeated trailer is
listed twice; editing the digest inside a signed tag makes `git verify-tag` and
`ssh-keygen -Y check-novalidate` fail (signed_bad), so the digest is bound by the
signature; a foreign key yields verify-tag exit 1 plus check-novalidate exit 0
(signed_untrusted); lightweight and unsigned annotated tags are distinguishable. Every
one of the seven statuses is reachable and the order signature > binding > meaning
yields exactly one `approved` combination.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | A `signed_valid` tag whose trailer read fails has no `binding`; rule 2 ("binding other than match") then reports `wrong_record`, misreporting an infrastructure failure as a wrong record. Fix: a `signed_valid` verdict with no readable message yields `unverifiable`. | FR-041 Outputs status rule 2 | wrong-requirement |
| FND-002 | low | FR-041-AC-7 says a signed commit, merge commit and git note are "refused as an approval" without naming the status; a note cannot be named by `tag` at all. Fix: state the expected result (`unverifiable`, revision does not resolve to a tag) and drop or specify the note case. | FR-041-AC-7 | wrong-requirement |

## Verdict

Not mergeable until the failure-domain finding FND-001 (SR-144, medium) is fixed; the two
low findings here are wording fixes in the same PR.
