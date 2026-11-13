---
id: SR-16419
title: "spec-review of FR-042 create a signed approval tag (engineering-assurance PR 219)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@a40b2fdb; spec/functional/FR-042-create-signed-approval-tag-for-a-change-assurance-record.md; spec/spec.md; spec/tests.md; README.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-042
    type: references
---

# Spec review of FR-042 create a signed approval tag (engineering-assurance PR 219)

## Summary

Ticket: PLAT-1591. The review covers the new FR-042, with EARS and integrity considerations applied inline, its spec index entry, the TC rows and the README row. The FR id is free on origin/main and on #218. Statements use SHALL and the ACs are atomic and testable. The record discriminators and digest form agree with quoin origin/main FR-063: `sha256:` plus lowercase hex of the exact stored bytes, with no digest member in the record. Out of scope is stated explicitly. README and spec index rows are present. #218 also edits the approval-verify README row next to this one, so expect a trivial merge conflict.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-042-AC-11 and the Behavior rule "report `approval_tag_create_failed` when `git tag -s` exits nonzero" assume git reports a signer failure in its exit status. On git 2.34.1 an SSH signing failure exits 0 and leaves an unsigned tag. The spec never requires the created tag to carry a signature, so as written it permits reporting `created` for an unsigned tag. Fix: require that a created tag carries a signature block, and that otherwise the command removes the tag it just wrote and reports `approval_tag_create_failed`. That needs a carve-out in FR-042-CON-1 ("never ... deletes one"). Extend AC-11 to an SSH signer failure. | spec/functional/FR-042-create-signed-approval-tag-for-a-change-assurance-record.md:124-125,148,170 |
| FND-002 | medium | The Behavior rule that the 120 s timeout "allows a passphrase prompt" is unsound. The FR-019 process host starts git in its own process group with stdin on /dev/null and stderr captured, so no prompt reaches the user. Fix: state that the signer must be non-interactive (ssh-agent, or a gitsign browser flow), or require the create step to run with the caller's terminal. | spec/functional/FR-042-create-signed-approval-tag-for-a-change-assurance-record.md:136-138 |
| FND-003 | low | The Description says x509 signing means `gpg.x509.program=gitsign`, but the signing rule accepts any `gpg.format=x509` with no program check, so a gpgsm-signed tag passes the precondition and never verifies under FR-040. Fix: make the rule and the Description agree, either by requiring `gpg.x509.program` or by saying it is not checked. | spec/functional/FR-042-create-signed-approval-tag-for-a-change-assurance-record.md:30-33,108-115 |
| FND-004 | low | FR-042-AC-8 defines a valid tag name only by `check-ref-format`. Names such as `HEAD` and `@` pass that check, are created, and are not resolvable by FR-041 `approval-verify`, which breaks the requirement's own goal. Fix: add to AC-8 that a name `approval-verify` would not resolve to `refs/tags/<name>`, such as one that already resolves as a revision, is `approval_tag_tag_invalid`. | spec/functional/FR-042-create-signed-approval-tag-for-a-change-assurance-record.md:120-123,167 |

## Verdict

FR-042 is well bounded and mostly testable. Two medium gaps, signer failure assumed visible in the exit status and the passphrase claim, plus two low consistency gaps, should be fixed in this PR together with the code findings.
