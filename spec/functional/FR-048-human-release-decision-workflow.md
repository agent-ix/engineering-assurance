---
id: FR-048
title: "Decide a release through a human-gated, resumable release-decision workflow"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-005"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: "requires"
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

The evidence status, SOUP report and anomaly list are the output of Quoin's
evidence, SOUP and anomaly reporting, supplied to the workflow as files in the
format Quoin publishes; Engineering Assurance reads only the members it
presents (per evidence item the id and state, the SOUP records and their report
entries, per anomaly the id and status) and computes none of them
([FR-049](./FR-049-release-ownership-split-and-data-contract.md)). A supplied
file that cannot be read, or lacks a member the presentation needs, is refused
with `release_input_unreadable` naming the file, and the run stays before the
gate.

The person's choice is `release` or `reject`. Choosing `release` with
unresolved anomalies is allowed only when the person names each such anomaly id
as accepted, and an id the person names that is not in the supplied anomaly
list is refused; the workflow records the named ids in the release record's
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
- On `release`: the Release document (decision owner, baseline commit and
  accepted anomaly ids), the repository-relative `record_path` the owner is to
  commit it at, and the tag message text (subject paragraph, `Approved-Record-Id`,
  `Signed-off-meaning:` trailer, and the `Approved-Record-Digest` line to be
  completed with the `git-blob:` id of the record at the release commit). The
  workflow ends there. The owner commits the record in a release commit whose
  single parent is the baseline commit and whose only change is the record,
  then signs the annotated tag with `git tag -s` and their own signing
  configuration; `release-verify` ([FR-047](./FR-047-release-record-and-signed-release-tag.md))
  later reports whether that tag is the release.
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
- The workflow SHALL NOT create a commit.
- The workflow SHALL present each input source as supplied or "not provided".
- While an unresolved anomaly is not named as accepted, the workflow SHALL NOT
  write a Release document for the `release` choice.
- If the owner names an accepted anomaly id that is not in the supplied anomaly
  list, then the workflow SHALL refuse the choice and write no Release document.
- If a supplied input file cannot be read or lacks a member the presentation
  needs, then the workflow SHALL refuse with `release_input_unreadable` and
  leave the run before the gate.
- The workflow SHALL NOT synthesize the `release` choice from evidence status,
  an empty anomaly list or a missing input.
- The workflow SHALL record an anomaly acceptance only from the owner's explicit
  naming of the anomaly id.
- If a run id is reused with a different repository, Baseline, baseline commit
  or decision owner, then the workflow SHALL refuse the run without changing
  either run, as FR-005 does.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-048-AC-1 | The workflow's transitions into terminal phases are all `hitl`, and a requested automatic terminal-gate override fails closed. | Test |
| FR-048-AC-2 | With no owner choice the run stays non-terminal at the gate, no Release document exists and no tag exists. | Test |
| FR-048-AC-3 | The review presentation lists the baseline commit and items, each evidence state, the SOUP report and the anomalies; an input file not supplied is shown "not provided", and an empty supplied anomaly list is shown as supplied and empty. | Test |
| FR-048-AC-4 | A `release` choice with an unresolved anomaly not named as accepted writes no Release document; naming its id writes `accepted_anomalies` with exactly the named ids. | Test |
| FR-048-AC-5 | A `reject` choice retains the rejection outcome and writes no Release document and no tag. | Test |
| FR-048-AC-6 | A `release` choice emits one terminal event with run id, workflow version, owner, choice and timestamp, and the workflow runs no `git tag`, no `git commit`, no signer and no push. | Test |
| FR-048-AC-7 | Evidence that is all `observed`, an empty anomaly list or a failed input read never produces a `release` choice. | Test |
| FR-048-AC-9 | An owner-named accepted anomaly id absent from the supplied anomaly list is refused and no Release document is written; an unreadable input file, or one lacking a presented member, yields `release_input_unreadable` naming the file and leaves the run before the gate. | Test |
| FR-048-AC-10 | The `release` outcome carries the Release document, `record_path` and the tag message text with the digest line to complete; committing the record as a single child of the baseline commit and signing a tag with that message by `git tag -s` yields `released` from `release-verify`, with no workflow-run git write. | Test |
| FR-048-AC-8 | A run id reused with a different Baseline, baseline commit, repository or owner is refused without changing either run; an interrupted run resumes at its last completed phase. | Test |

## Dependencies

- **Upstream**: [FR-005](./FR-005-resumable-human-decisions.md),
  [FR-004](./FR-004-evidence-state-and-provenance.md),
  [FR-046](./FR-046-baseline-archetype.md),
  [FR-047](./FR-047-release-record-and-signed-release-tag.md).
- **Downstream**: the owner's release commit and `git tag -s` signing, and `release-verify`.
