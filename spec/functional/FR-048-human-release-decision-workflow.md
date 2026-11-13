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
- the evidence status of every acceptance criterion, in the Evidence Status
  vocabulary of `ix://agent-ix/quoin/FR-115` (the closed set `bound`, `stale`,
  `suspect`, `undischarged`, `no run evidence`), as Quoin reports it;
- the SOUP report: the SOUP records of the Baseline and the report the caller
  supplies for them;
- the unresolved anomalies: the list the caller supplies, each with its id and
  status;
- the snapshot file, if supplied, shown verbatim and unparsed with its `snapshot_path`. The owner's signature on the
  tag covers whatever bytes the release commit holds at `snapshot_path`; the workflow does not check that they are
  the bytes shown.

The evidence status, SOUP report and anomaly list are Quoin outputs, supplied
to the workflow as files in the formats that Quoin requirements define: the
evidence status of `ix://agent-ix/quoin/FR-115` (the Evidence Status of
`quoin matrix`), the SOUP report of `ix://agent-ix/quoin/FR-144` and the anomaly
report of `ix://agent-ix/quoin/FR-162`. Engineering Assurance reads only the members it
presents (per acceptance criterion the id and evidence status, the SOUP records and their report
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

Each supplied Quoin file is presented verbatim, as the JSON document it is, within a stated size bound, beside the
members the workflow reads from it; a member the workflow does not read is therefore still shown, and nothing in a
supplied report is dropped from what the person reviews. In the anomaly report every listed anomaly is carried, whatever
its status ([quoin FR-162](ix://agent-ix/quoin/FR-162)), so each must be named as accepted for a `release`; an entry of
the report's `unreadable` list (an anomaly document Quoin could not read, `{ path, code }`) is shown as such and is
named as accepted by its `path`, like an unresolved anomaly. A matrix document that carries a `reason` and no criteria is
shown verbatim with that reason.

Out of scope: computing evidence, SOUP or anomaly status; any automatic,
scheduled, or agent-made release choice; signing; pushing the tag.

## Inputs

- The FR-005 inputs: workflow name, stable run id and state directory, and the
  named decision owner with their explicit choice.
- The Baseline file path, the baseline commit (any revision git resolves to a commit; the workflow records the full
  hash git reports and binds the run to the string given), the repository directory and
  the repository-relative `snapshot_path` the owner is to commit the snapshot at.
- At presentation: the Release document `release_id` (`REL-` and digits, the id the Release schema requires) and the
  repository-relative `record_path` the owner is to commit the Release document at, which differs from `snapshot_path`.
- With the `release` choice: the `meaning`, the text of the `Signed-off-meaning:` trailer, which the owner states
  because the tag has no default meaning ([FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md)), and the
  accepted anomaly ids.
- Optional files, each a regular file of at most 1 MiB: evidence status, SOUP report, anomaly list, and the snapshot
  file Quoin wrote for the owner to commit ([FR-051](./FR-051-release-evidence-snapshot-placement.md)). The
  workflow reads these members: per criterion `id` and `evidence_status` (`requirements[].criteria[]`), per SOUP report
  row `purl` (`rows[]`), per anomaly `id` and `status` (`anomalies[]`, `status` possibly `null`), and per `unreadable`
  entry `path`.

## Outputs

- The review presentation above, retained with the run. Its size after JSON escaping is bounded (3 MiB) so that the
  run stays readable by every later command.
- On `release`: the Release document (decision owner, baseline commit,
  `snapshot_path` and accepted anomaly ids), the repository-relative `record_path` the owner is to
  commit it at, and the tag message text (subject paragraph, `Approved-Record-Id`,
  `Signed-off-meaning:` trailer, and the `Approved-Record-Digest` line to be
  completed with the `git-blob:` id of the record at the release commit). The Release document's `name` is the Baseline
  `name`, its `title` is "Release of" and that name, and the tag subject paragraph is "Release" and that name with
  whitespace collapsed. The Release document, `record_path` and tag message are retained with the run when the choice is
  acknowledged and a repeated `release` choice returns them unchanged. The
  workflow ends there. The owner commits the record in a release commit whose
  single parent is the baseline commit and whose only changes are the record and
  the release evidence snapshot ([FR-051](./FR-051-release-evidence-snapshot-placement.md)),
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
- If a Baseline item is absent at the baseline commit, then the workflow SHALL refuse the `release` choice with
  `baseline_item_absent` naming the paths and write no Release document, because `release-verify` could not report
  that release; the `reject` choice remains available.
- If `record_path` exists at the baseline commit, then the workflow SHALL refuse the `release` choice with
  `record_path_present_at_baseline` and write no Release document.
- If a repeated `release` choice names a `meaning` or accepted anomaly ids that differ from those acknowledged, then the
  workflow SHALL refuse it with `release_decision_changed` and change no state.
- If a `decide` request names a run that does not exist, then the workflow SHALL refuse it with
  `workflow_run_not_found` and create no run.
- The workflow SHALL NOT accept a release choice for a run that has no retained presentation, whether the run is driven
  by the workflow command or by the generic workflow host.
- If a supplied input file is not a regular file, is larger than 1 MiB, or would make the escaped presentation larger
  than its bound, then the workflow SHALL refuse with `release_input_unreadable` naming the file, or with
  `release_presentation_too_large`, before it changes the run.
- If `snapshot_path` exists at the baseline commit, then the workflow SHALL refuse the `release` choice with
  `snapshot_path_present_at_baseline` and write no Release document.
- If `snapshot_path` breaks the path rules of a Baseline item (FR-046 Inputs), then the workflow SHALL refuse the `release` choice with
  `release_snapshot_path_invalid` and write no Release document.
- The workflow SHALL NOT synthesize the `release` choice from evidence status,
  an empty anomaly list or a missing input.
- The workflow SHALL record an anomaly acceptance only from the owner's explicit
  naming of the anomaly id.
- If a run id is reused with a different repository, Baseline, baseline commit,
  `snapshot_path` or decision owner, then the workflow SHALL refuse the run without changing
  either run, as FR-005 does.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-048-AC-1 | The workflow's transitions into terminal phases are all `hitl`, and a requested automatic terminal-gate override fails closed. | Test |
| FR-048-AC-2 | With no owner choice the run stays non-terminal at the gate, no Release document exists and no tag exists. | Test |
| FR-048-AC-3 | The review presentation lists the baseline commit and items, each criterion's evidence status, the SOUP report and the anomalies; an input file not supplied is shown "not provided", and an empty supplied anomaly list is shown as supplied and empty. | Test |
| FR-048-AC-4 | A `release` choice with an unresolved anomaly not named as accepted writes no Release document; naming its id writes `accepted_anomalies` with exactly the named ids. | Test |
| FR-048-AC-5 | A `reject` choice retains the rejection outcome and writes no Release document and no tag. | Test |
| FR-048-AC-6 | A `release` choice emits one terminal event with run id, workflow version, owner, choice and timestamp, and the workflow runs no `git tag`, no `git commit`, no signer and no push. | Test |
| FR-048-AC-7 | Evidence in which every criterion is `bound`, an empty anomaly list or a failed input read never produces a `release` choice. | Test |
| FR-048-AC-8 | A run id reused with a different Baseline, baseline commit, repository, `snapshot_path` or owner is refused without changing either run; an interrupted run resumes at its last completed phase. | Test |
| FR-048-AC-9 | An owner-named accepted anomaly id absent from the supplied anomaly list is refused and no Release document is written; an unreadable input file, or one lacking a presented member, yields `release_input_unreadable` naming the file and leaves the run before the gate. | Test |
| FR-048-AC-10 | The `release` outcome carries the Release document, `record_path` and the tag message text with the digest line to complete; committing the record and the snapshot as a single child of the baseline commit and signing a tag with that message by `git tag -s` yields `released` from `release-verify`, with no workflow-run git write. | Test |
| FR-048-AC-11 | A `release` choice with a `snapshot_path` that exists at the baseline commit (which includes the Baseline file and any Baseline item) is refused with `snapshot_path_present_at_baseline`, and one that breaks the path rules of a Baseline item (absolute, `..`, glob, control character, space-leading segment) is refused with `release_snapshot_path_invalid`; neither writes a Release document. | Test |
| FR-048-AC-12 | A supplied snapshot file is presented verbatim and unparsed with its `snapshot_path`, and an unsupplied one is shown "not provided". | Test |
| FR-048-AC-13 | A repeated `release` choice returns the Release document, `record_path` and tag message retained when the choice was acknowledged, and a repeat whose `meaning` or accepted anomaly ids differ is refused with `release_decision_changed` and changes no state. | Test |
| FR-048-AC-14 | Neither the workflow command nor the generic workflow host takes a run with no retained presentation past `evidence_ready` or to `released`, and the Rust and JavaScript invariant evaluators agree on the presentation invariant. | Test |
| FR-048-AC-15 | A `decide` on a run id that does not exist creates no run; a supplied file that is a FIFO, not UTF-8 or over 1 MiB is refused naming the file; a presentation over its escaped bound is refused before the run changes, and one just under it stays decidable. | Test |
| FR-048-AC-16 | A `release` choice is refused, writing no Release document, when a Baseline item is absent at the baseline commit (`baseline_item_absent`) or `record_path` exists at it (`record_path_present_at_baseline`); `reject` stays available. | Test |
| FR-048-AC-17 | A non-empty `unreadable` list of the supplied anomaly report is shown, a `release` that does not name each entry's `path` as accepted is refused, and each supplied Quoin document is presented verbatim. | Test |

## Dependencies

- **Upstream**: [FR-005](./FR-005-resumable-human-decisions.md),
  [FR-046](./FR-046-baseline-archetype.md),
  [FR-047](./FR-047-release-record-and-signed-release-tag.md).
- **Downstream**: the owner's release commit and `git tag -s` signing, and `release-verify`.
