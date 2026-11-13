---
id: FR-060
title: "Bind an approval to the pull request head that merged and survive a squash merge"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-058"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-059"
    type: "references"
---

# FR-060: Bind an approval to the pull request head that merged and survive a squash merge

## Description

Tracked by Linear PLAT-1685. An approval counts for a merge only when the reviewer approved the head commit that
merged. The statement writer compares the approval's commit with the pull request's head commit at the moment of
merge. If they differ (the author pushed after the review), the approval produces no statement. Because the
statement's subject is the merged commit and the reviewed head commit is a predicate member
([FR-058](./FR-058-approval-statement-and-predicate.md)), the statement still binds after a squash or rebase merge
rewrites the history: the pull request reference ties the two commits together.

The pull request's head at merge, its merged commit and its merge method are read from the source at the time the
pull request is closed as merged. An approval for a pull request that is not merged produces no statement.

## Behavior

- The statement writer SHALL compare the approval's commit with the pull request head commit at merge.
- The statement writer SHALL produce no statement when the approval's commit differs from the head commit at merge.
- The statement writer SHALL produce no statement for a pull request that is not merged.
- The statement writer SHALL set `merged_commit` to the commit the source reports as the merge result, for merge, squash
  and rebase methods alike.
- The statement writer SHALL set `head_commit` to the approval's commit.
- The statement writer SHALL NOT infer a head commit from tree or content similarity.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-060-AC-1 | A review on the head commit that merged, with the pull request merged by squash, yields a statement whose `merged_commit` is the squash commit and whose `head_commit` is the reviewed commit. | Test |
| FR-060-AC-2 | The same outcome holds for a merge commit and for a rebase merge, with the merge result commit as `merged_commit`. | Test |
| FR-060-AC-3 | A review on an earlier head, followed by a new push and a merge, yields no statement for that review. | Test |
| FR-060-AC-4 | A review on a pull request that is closed without merging, or still open, yields no statement. | Test |
| FR-060-AC-5 | A squash commit with a tree identical to the reviewed head's tree but a review on a different head commit yields no statement. | Test |

## Dependencies

- **Upstream**: [FR-058](./FR-058-approval-statement-and-predicate.md),
  [FR-059](./FR-059-source-adapter-interface-and-github-review-adapter.md).
- **Downstream**: [FR-061](./FR-061-role-policy-and-distinct-individuals.md).
