---
id: FR-083
title: "Document the adoption rulesets that protect tags and the private records branch, and state what cannot be protected"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-065"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-081"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-054"
    type: "references"
---

# FR-083: Document the adoption rulesets that protect tags and the private records branch, and state what cannot be protected

## Description

Tracked by Linear EA-137. Detection and protection are different things. What Engineering Assurance tests is detection:
[FR-082](./FR-082-detect-deleted-and-changed-tags-from-tag-receipts.md) lists a deleted or changed tag and
[FR-077](./FR-077-missing-receipt-check.md) lists a commit without a receipt. Protection is a platform setting that
Engineering Assurance documents and cannot enforce, and it neither applies nor checks the setting. This requirement
states what the adoption documentation SHALL tell an adopter to configure. The main-branch review gate ruleset stays
with [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md) and `docs/approval-gate.md`.

A GitHub repository ruleset has exactly one target type (`branch`, `tag` or `push`), and its `ref_name` condition names
branches or tags. The adoption documentation therefore describes two example ruleset payloads, one per target type,
beside the existing review-gate example:

- A tag ruleset (`target` `tag`, `enforcement` `active`) whose `conditions.ref_name.include` is `["refs/tags/*"]` and
  whose rules are `deletion` and `update`: no approval or release tag can be deleted or moved.
- A branch ruleset for the private store repository named in the sink configuration
  ([FR-054](./FR-054-record-sink-interface-and-implementations.md)) (`target` `branch`, `enforcement` `active`) whose
  `conditions.ref_name.include` is `["refs/heads/engineering-assurance-private-records"]` and whose rules are
  `deletion` and `non_fast_forward`. That ref lives in a separate repository, so this ruleset is applied there.

Both payloads have an empty `bypass_actors` list: the bypass list names no person who can sign approvals (no member of
any role of the role policy [FR-061](./FR-061-role-policy-and-distinct-individuals.md)) and no agent account.

Notes refs. The two notes refs, `refs/notes/assurance-receipts` and `refs/notes/assurance-tag-receipts`, are neither
branches nor tags, and a ruleset cannot target them. The documentation says so plainly: no ruleset protects the notes
refs, and for them detection and not protection is what exists. A deleted or rewritten note shows as `no_note`
([FR-077](./FR-077-missing-receipt-check.md)) for a commit, and a deleted tag note leaves its tag unreceipted
(`tag_no_receipt`, FR-082); a tag deleted at the same time as its receipt is not detectable by the tool, and the
documentation states that as a residual risk that a person accepts or addresses with platform controls it names
only in general terms (restricting who holds write access). The workflow identity that writes receipts needs only to create and
fast-forward the notes refs. A repository without these rulesets still gets detection from the tag receipts and gets no
protection.

The documentation is the deliverable; each ruleset is applied manually by the repository's operator, as the review gate
ruleset is. The example payloads are configuration data under the repository's own content rights; they hold no
organization-specific value and no copied text.

## Behavior

- The adoption documentation SHALL state that the tag ruleset protects `refs/tags/*` from deletion and update.
- The adoption documentation SHALL state that the private records branch in the private store repository is protected from deletion and non-fast-forward update by a branch ruleset of that repository.
- The adoption documentation SHALL state that a ruleset cannot target `refs/notes/assurance-receipts` or `refs/notes/assurance-tag-receipts` and that only detection covers them.
- The adoption documentation SHALL state that the bypass list names no person who can sign approvals and no agent account.
- The adoption documentation SHALL state that protection is platform setup that Engineering Assurance does not enforce and that detection is what the tool tests.
- The example ruleset payloads SHALL contain no organization-specific value.
- Engineering Assurance SHALL NOT apply, check or report on a ruleset.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-083-AC-1 | The adoption documentation names `refs/tags/*` (deletion and update) and `refs/heads/engineering-assurance-private-records` (deletion and non-fast-forward), and states that the two notes refs cannot be targeted by a ruleset and are covered by detection only. | Inspection |
| FR-083-AC-2 | A test reads each example payload and checks the fields this requirement names: the tag payload has `target` `tag`, `enforcement` `active`, `conditions.ref_name.include` exactly `["refs/tags/*"]`, rule types exactly `deletion` and `update` and an empty `bypass_actors`; the branch payload has `target` `branch`, `enforcement` `active`, `conditions.ref_name.include` exactly `["refs/heads/engineering-assurance-private-records"]`, rule types exactly `deletion` and `non_fast_forward` and an empty `bypass_actors`; neither payload names a `refs/notes/` ref. | Test |
| FR-083-AC-3 | The adoption documentation states that the bypass list names no approver and no agent account, and states in one sentence that protection is platform setup, not enforced by the tool, and that detection is what the tool tests. | Inspection |
| FR-083-AC-4 | No command or workflow of the repository applies, reads or reports a ruleset. | Inspection |

## Dependencies

- **Upstream**: [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md), [FR-054](./FR-054-record-sink-interface-and-implementations.md), [FR-081](./FR-081-tag-receipt-on-approval-and-release-tag-push.md), [FR-061](./FR-061-role-policy-and-distinct-individuals.md), [FR-077](./FR-077-missing-receipt-check.md), [FR-082](./FR-082-detect-deleted-and-changed-tags-from-tag-receipts.md).
- **Downstream**: the adoption documentation.
