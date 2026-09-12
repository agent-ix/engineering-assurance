---
id: SR-124
title: "Review of the declared evaluation qualification population"
type: SpecReview
analysis: code-review
scope: "FR-017-AC-2, NFR-002-AC-3, TC-039, TC-109, TC-110; evaluation host population"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

`REQUIRED_CELL_COUNT` was `EvaluationHost::ALL.len() * EvaluationScenario::ALL.len()`
— every host the enum can name, times every scenario. Twenty-eight cells, each
one a live token-bearing session on a commercial agent CLI, and the aggregate
refused unless all twenty-eight were present, passing, and carrying a non-empty
model string per host.

That made the gate demand evidence for hosts no release claims. The failure it
produced — an incomplete aggregate — is indistinguishable from a run that was
attempted and fell short, which is a different and more serious thing.

### `ALL` and `QUALIFIED` are different questions

`ALL` answers "what can Engineering Assurance evaluate". `QUALIFIED` answers
"what does a release qualify against". Conflating them is what made the
population unstatable: there was nowhere to say that a host is supported but
unqualified.

`EvaluationHost::QUALIFIED` now names the declared population and the
requirement states it. Completeness is untouched — every declared host must
retain every scenario, `ok` must hold, `failures` must be empty, and each
declared host must carry a model. Only the population is smaller, and it is
smaller by declaration rather than by omission.

### Narrowing removes a real check, and says so

`append_scenario_workflow_failures` compares governing workflow identity across
hosts within one scenario. The aggregator drops cells outside the required
matrix before that check runs, so with one declared host there is exactly one
cell per scenario and `ScenarioWorkflowVersionMismatch` cannot fire on any
input.

The variant is retained, not deleted: it becomes reachable the moment a second
host is declared. Its parity case is removed **as a pair with its expected
code**, because that assertion list is positionally zipped — removing one side
silently re-pairs every case after it with the wrong expectation, which is a
test that still passes while checking something nobody intended. Both the
absence and the reason are recorded at the assertion site and in the matrix.

### Fixtures moved to a qualified host

Several report-adapter fixtures were built on `codex`. With codex unqualified
those cells fall outside the required matrix, so the fixtures proved nothing
about completeness. They are rebuilt on `claude`. One assertion named
`missing:claude:existing-profile` — true only while codex supplied that cell —
and now names a cell that is genuinely still absent.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-230 | high | The required cell count derived from every nameable host, so the gate demanded evidence for hosts no release claims, and reported that as an incomplete run | FR-017-AC-2, NFR-002-AC-3 | wrong-requirement |
| FND-231 | medium | Narrowing the population makes `ScenarioWorkflowVersionMismatch` unreachable; without a recorded rationale its absent test case would read as an oversight | TC-110 | missing-requirement |
| FND-232 | low | Report-adapter fixtures were built on an unqualified host, so they exercised cells outside the required matrix | TC-129 | correct-requirement-no-evidence |

## Disposition

This narrows what a release claims, so it is a specification change rather than
a constant edit: FR-017-AC-2, NFR-002-AC-3 and the Agent Evaluation Permutation
Matrix all now speak of the declared population, and the matrix names Claude
Code as that population with the other three hosts recorded as supported but
unqualified.

Repository coverage is unchanged at `281/312` with zero status lies — this adds
no bindings. What it changes is that the aggregate is now achievable at all,
which is what twenty-two of the matrix's pending rows are waiting on.

No status was promoted, and no host is claimed as evidenced.
