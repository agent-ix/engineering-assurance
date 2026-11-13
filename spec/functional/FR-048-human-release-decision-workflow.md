---
id: FR-048
title: "Decide a release through a human-gated, resumable release-decision workflow"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-005"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-042"
    type: "references"
---

# FR-048: Decide a release through a human-gated, resumable release-decision workflow

## Description

Engineering Assurance SHALL ship a canonical `release-decision` workflow that
reuses the resumable, human-gated decision mechanics of
[FR-005](./FR-005-resumable-human-decisions.md) and adds no second lifecycle.
The workflow gathers what a person needs to decide a release, presents it, and
stops at a `hitl` gate. No release is decided until the named decision owner
makes the explicit choice. A release exists only once the person has signed the
release tag ([FR-047](./FR-047-release-record-and-signed-release-tag.md)); the
workflow can neither sign nor choose for them.

What the person reviews, each shown as data with its source and as "not
provided" when the source was not supplied, never as "none" or "clean":

- the Baseline: its name, the commit hash, and every item path and kind with
  `present` or `absent` at that commit ([FR-046](./FR-046-baseline-archetype.md));
- the evidence status of every evidence item: the FR-004 states
  (`observed`, `unavailable`, `not_computed`, `not_applicable`) as retained;
- the SOUP report: the SOUP records of the Baseline and the report the caller
  supplies for them;
- the unresolved anomalies: the list the caller supplies, each with its id and
  status.

The SOUP report, evidence status and anomaly list are produced by Quoin or by
the caller and supplied to the workflow as files; Engineering Assurance reads
them and computes none of them
([FR-049](./FR-049-release-ownership-split-and-data-contract.md)).

The person's choice is `release` or `reject`. Choosing `release` with
unresolved anomalies is allowed only when the person names each such anomaly id
as accepted; the workflow records those ids in the release record's
`accepted_anomalies`. Accepting risk is the person's act and is never inferred.

Out of scope: computing evidence, SOUP or anomaly status; any automatic,
scheduled, or agent-made release choice; signing; pushing the tag.

## Inputs

- The FR-005 inputs: workflow name, stable run id and state directory, and the
  named decision owner with their explicit choice.
- The Baseline file path, the baseline commit hash, and the repository directory.
- Optional files: evidence status, SOUP report, anomaly list.

## Outputs

- The review presentation above, retained with the run.
- On `release`: the Release document with the decision owner, baseline commit
  and accepted anomaly ids, and the exact `approval-tag` request the person's
  own git configuration will sign (FR-042); the workflow does not run it.
- On `reject`: the rejection outcome and no Release document and no tag.
- The FR-005 terminal gate event: run id, workflow identity and version, owner,
  explicit choice, timestamp.

## Behavior

- The workflow SHALL leave every transition into a terminal phase at the
  `hitl` gate type, as FR-005 requires.
- If no named owner supplies a choice, then the workflow SHALL leave the run at
  the decision gate.
- The workflow SHALL refuse any requested automatic override of the gate.
- The workflow SHALL NOT create, sign, move or push a tag.
- The workflow SHALL present each input source as supplied or "not provided".
- While an unresolved anomaly is not named as accepted, the workflow SHALL NOT
  write a Release document for the `release` choice.
- The workflow SHALL NOT synthesize the `release` choice from evidence status,
  an empty anomaly list or a missing input.
- The workflow SHALL record an anomaly acceptance only from the owner's explicit
  naming of the anomaly id.
- A run id reused with a different repository, Baseline, baseline commit or
  decision owner SHALL be refused without changing either run, as FR-005 does.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-048-AC-1 | The workflow's transitions into terminal phases are all `hitl`, and a requested automatic terminal-gate override fails closed. | Test |
| FR-048-AC-2 | With no owner choice the run stays non-terminal at the gate, no Release document exists and no tag exists. | Test |
| FR-048-AC-3 | The review presentation lists the baseline commit and items, each evidence state, the SOUP report and the anomalies; an input file not supplied is shown "not provided", and an empty supplied anomaly list is shown as supplied and empty. | Test |
| FR-048-AC-4 | A `release` choice with an unresolved anomaly not named as accepted writes no Release document; naming its id writes `accepted_anomalies` with exactly the named ids. | Test |
| FR-048-AC-5 | A `reject` choice retains the rejection outcome and writes no Release document and no tag. | Test |
| FR-048-AC-6 | A `release` choice emits one terminal event with run id, workflow version, owner, choice and timestamp, and the workflow runs no `git tag`, no signer and no push. | Test |
| FR-048-AC-7 | Evidence that is all `observed`, an empty anomaly list or a failed input read never produces a `release` choice. | Test |
| FR-048-AC-8 | A run id reused with a different Baseline, baseline commit, repository or owner is refused without changing either run; an interrupted run resumes at its last completed phase. | Test |

## Dependencies

- **Upstream**: [FR-005](./FR-005-resumable-human-decisions.md),
  [FR-004](./FR-004-evidence-state-and-provenance.md),
  [FR-046](./FR-046-baseline-archetype.md),
  [FR-047](./FR-047-release-record-and-signed-release-tag.md),
  [FR-042](./FR-042-create-signed-approval-tag-for-a-change-assurance-record.md).
- **Downstream**: the owner's `approval-tag` signing and `release-verify`.
