---
id: FR-083
title: "Document the adoption ruleset that protects tags, receipt notes and the private records branch"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-065"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-081"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-054"
    type: "references"
---

# FR-083: Document the adoption ruleset that protects tags, receipt notes and the private records branch

## Description

Tracked by Linear EA-137. Detection and protection are different things. What Engineering Assurance tests is detection:
[FR-082](./FR-082-detect-deleted-and-changed-tags-from-tag-receipts.md) lists a deleted or changed tag and
[FR-077](./FR-077-missing-receipt-check.md) lists a commit without a receipt. Protection is a platform setting that
Engineering Assurance documents and cannot enforce, and it neither applies nor checks the setting. This requirement
states what the adoption documentation SHALL tell an adopter to configure, so that the evidence stays hard to remove.
The main-branch review gate ruleset stays with [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md)
and its documentation (`docs/approval-gate.md`).

The adoption documentation SHALL describe one ruleset payload, with a concrete example file beside the existing
review-gate example, covering these refs:

- `refs/tags/*` of the repository: no deletion and no update (so an approval or release tag cannot be removed or moved);
- `refs/notes/assurance-receipts` and `refs/notes/assurance-tag-receipts` of the repository: no deletion and no
  non-fast-forward update (so receipts cannot be removed or rewritten);
- `refs/heads/engineering-assurance-private-records` of the private store repository named in the sink configuration
  ([FR-054](./FR-054-record-sink-interface-and-implementations.md)): no deletion and no non-fast-forward update. This
  ref lives in a separate repository, so it is a second ruleset on that repository.

The bypass list of every such ruleset names no person who can sign approvals (no member of any role of the role policy
[FR-061](./FR-061-role-policy-and-distinct-individuals.md)) and no agent account. The documentation says plainly that the
workflow identity that writes receipts needs only to create and fast-forward the two notes refs, that tag creation by a
person is allowed by the ruleset (only deletion and update are blocked), and that a repository without this ruleset
still gets detection from the tag receipts but gets no protection.

The documentation is the deliverable; the ruleset is applied manually by the repository's operator, as the review gate
ruleset is. The example payload is configuration data under the repository's own content rights; it holds no
organization-specific value and no copied text.

## Behavior

- The adoption documentation SHALL state that the ruleset protects `refs/tags/*` from deletion and update.
- The adoption documentation SHALL state that the ruleset protects `refs/notes/assurance-receipts` and `refs/notes/assurance-tag-receipts` from deletion and non-fast-forward update.
- The adoption documentation SHALL state that the private records branch in the private store repository is protected from deletion and non-fast-forward update by a ruleset of that repository.
- The adoption documentation SHALL state that the bypass list names no person who can sign approvals and no agent account.
- The adoption documentation SHALL state that protection is platform setup that Engineering Assurance does not enforce and that detection is what the tool tests.
- The example ruleset payload SHALL contain no organization-specific value.
- Engineering Assurance SHALL NOT apply, check or report on the ruleset.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-083-AC-1 | The adoption documentation names `refs/tags/*`, `refs/notes/assurance-receipts`, `refs/notes/assurance-tag-receipts` and `refs/heads/engineering-assurance-private-records` with the blocked operations of this requirement for each. | Inspection |
| FR-083-AC-2 | The example ruleset payload parses as JSON, targets exactly the repository refs `refs/tags/*`, `refs/notes/assurance-receipts` and `refs/notes/assurance-tag-receipts` with deletion and non-fast-forward rules, and has an empty bypass actor list. | Test |
| FR-083-AC-3 | The adoption documentation states that the bypass list names no approver and no agent account, and states in one sentence that protection is platform setup, not enforced by the tool, and that detection is what the tool tests. | Inspection |
| FR-083-AC-4 | No command or workflow of the repository applies, reads or reports a ruleset for these refs. | Inspection |

## Dependencies

- **Upstream**: [FR-065](./FR-065-workflow-roles-and-relation-to-signed-release-tags.md), [FR-054](./FR-054-record-sink-interface-and-implementations.md), [FR-081](./FR-081-tag-receipt-on-approval-and-release-tag-push.md), [FR-061](./FR-061-role-policy-and-distinct-individuals.md).
- **Downstream**: the adoption documentation.
