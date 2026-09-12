---
id: SR-125
title: "Review of the retirement of FR-019-AC-6 as a consumer adoption claim"
type: SpecReview
analysis: code-review
scope: "FR-019-AC-6, TC-126; producer-execution consumer boundary"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: reviews
---

## Summary

FR-019-AC-6 required a *real* `quire-verification` synthetic consumer to compile
against the accepted revision and distinguish completed observations from every
non-completion state without a local runner or stdout adapter. TC-126 carried
it, and could never be discharged here: the test's subject is a crate in another
repository.

The question is not whether the boundary works. It is whether this criterion
says anything about Engineering Assurance that the requirement does not already
prove.

### Every technical claim was already proven

| AC-6 asserted | Discharged by |
| --- | --- |
| compiles with default features off and only `producer-execution` | FR-019-AC-7 / TC-128 — minimal consumer plus activated direct-dependency census |
| no local runner, no stdout parsing | FR-019-AC-5 / TC-127 — caller-owned typed adapter receives bound terminal evidence |
| completed distinguishable from every non-completion state | FR-019-AC-3 / TC-124 — all eight states distinct, canonically serializable |

What remained once those are subtracted is the consumer's *identity*: that the
adopting crate is the real `quire-verification` rather than a minimal one. That
is adoption by another repository. It is a fact about someone else's roadmap,
not a property of this codebase, and no test here can observe it.

### Why a permanently-open row is worse than no row

`validate_matrix` refuses any `🚧`, so an acceptance criterion that cannot be
discharged locally does not merely sit there — it holds the release gate closed
on evidence this repository is not able to produce. The row reported as
outstanding work when the outstanding work was not ours, which is the same
confusion the coverage gate exists to prevent: a gap that cannot be closed and a
gap nobody has closed are different things and must not look alike.

The dependency is real and is not being discarded. FR-019's Dependencies section
already named IT-006 and the consumer-owned qualification cases; it now also
records what the retired criterion asserted and where each of its claims is
proven, so a reader meeting the numbering gap finds the reason rather than an
absence.

### Numbering is deliberately left alone

AC-7 is not renumbered to AC-6. Renumbering would silently re-point every
existing reference — matrix rows, reviews, trace tags — at a different
criterion, and each would still parse. A gap in the sequence is visible; a
silent re-pointing is not.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-233 | medium | An acceptance criterion asserted adoption by another repository, so it could not be discharged locally and held the release gate closed on evidence this repository cannot produce | FR-019-AC-6 | wrong-requirement |
| FND-234 | low | Its three technical claims were already discharged by FR-019-AC-3, AC-5 and AC-7, so the row added no coverage | FR-019-AC-6 | wrong-requirement |

## Disposition

FR-019-AC-6 and TC-126 are retired. `REQUIRED_TEST_CASES` moves 133 to 132 with
the population it pins.

Coverage moves `281/312` to `281/310`: the denominator drops by the retired
criterion and its test case, and the backed count is unchanged because neither
was ever backed. No row changed status and no evidence was claimed.

TC-108 is deliberately **not** retired alongside it. It guards removal of the
JavaScript invariant paths, and unlike AC-6 its claim is not proven elsewhere —
retiring it would delete the check that stops those files being deleted before
the Rust provider is wired in.
