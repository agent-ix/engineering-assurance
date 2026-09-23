---
id: MP-001
title: Juniper request-retention measurement
type: MeasurementPlan
status: proposed
owner: juniper-observability-owner
metric: request_retention_rate
definition_version: juniper.request-retention-v1
stage: baseline
objective:
  direction: higher
  bound: 0.995
  weight: 1.0
  value_half_life: 30
  budget: 50
subject_identity:
  name: juniper-classifier
  version: 2026.09.1
statistical_design:
  population: fictional valid requests accepted by the candidate service
  minimum_population: 200
  sampling: deterministic seeded sample across declared request classes
  repetitions: 5
  estimator: proportion
  error_model: independent run variation and fixture selection
  uncertainty: report every run and a bootstrap interval
  decision_rule:
    comparator: ge
    threshold: 0.99
protected_apparatus:
  - evals/juniper/retention_harness.py
  - evals/juniper/checker.toml
  - fixtures/juniper/requests/**
  - fixtures/juniper/population.yaml
negative_controls:
  - kind: suppressed-observation
    description: >-
      the harness records every request in population.yaml, so a collection
      that examined fewer than the listed count is visible
  - kind: apparatus-edit
    description: >-
      the harness, checker configuration, request fixtures and population
      file are protected, so editing one changes the recorded digests
relationships:
  - target: ix://example/juniper/AP-001
    type: measures
---

# Juniper request-retention measurement

## Decision Use

The result informs whether the request-loss scenario needs more investigation;
it does not approve a release.

## Objective

`objective.direction` states which way the metric should move: `higher` or
`lower` when a larger or smaller value is better, `zero` when any non-zero
value is a deviation, and `target` when the metric should reach `bound`
(`bound` is then required). For the other directions `bound` is optional.
`bound` is the goal the metric should reach -- here, retention of 0.995. It is
informational and never evaluated: only `statistical_design.decision_rule` is
evaluated, and its threshold (0.99 here) may sit below the goal.

The objective is part of the measurement definition. Adding, removing, or
changing `direction` or `bound` requires a new `definition_version`, so
results under the old and new objective are never compared as one series.

## Steering Fields

`objective.weight`, `objective.value_half_life`, and `objective.budget` are
optional steering fields (FR-026), in units this plan's body states:

- `weight` -- this objective's relative value against the project's other
  objectives (here, 1.0 -- an arbitrary baseline weight; only meaningful
  relative to the weights of other objectives in the same project);
- `value_half_life` -- how quickly the value of improving this objective
  decays, so a sooner improvement ranks higher than an equally-sized later
  one (here, 30 days: an improvement realized after one half-life is worth
  about half of one realized immediately);
- `budget` -- time, tokens, or compute allowed per attempt at this objective
  (here, 50 -- e.g. agent-minutes), used only to normalize comparisons across
  objectives with different costs per attempt.

**All three are advisory only. They are never evaluated, never gate anything,
and never feed `statistical_design.decision_rule`** -- the rule in the next
section is the only part of this plan that is evaluated, and it is computed
without reading the objective at all. They are also excluded from the
measurement definition: changing only a steering field is not a definition
change and needs no `definition_version` bump, unlike a `direction` or
`bound` edit.

## Decision Rule

`statistical_design.decision_rule` is the only part of the plan that is
evaluated. `statistical_design.estimator` names how the metric is computed
from the population: `proportion`, `count`, `mean`, `median`, or `ratio`. The
rule is applied once to that estimate, computed over all `repetitions`: it
holds when `estimate <comparator> reference`. `comparator` is one of `gt`,
`ge`, `lt`, `le`, or `eq`; `eq` is exact equality, which for a `mean` or
`ratio` estimate means exact floating-point equality. The reference is exactly
one of:

- `threshold`: a fixed number stated in the plan, as here (retention of at
  least 0.99); or
- `baseline`: a value computed at evaluation time, one of
  `constant-predictor` (the per-family best constant answers combined as a
  size-weighted mean, see the worked example below; only with
  `estimator: proportion`), `prior-collection` (the collection this result is
  compared against), or `best-seen` (for a ratchet: over every collection the
  measurement intake admitted under this `definition_version`, the maximum
  under a `gt`/`ge` rule and the minimum under an `lt`/`le` rule; an `eq` rule
  cannot use it).

A baseline rule may add a `margin`, in the metric's own units and signed in
the direction of improvement: a positive margin means the result must beat the
baseline by at least that much, a negative one allows a regression of up to
that much. The reference is `baseline + margin` for `gt`/`ge` and
`baseline - margin` for `lt`/`le`; `eq` takes no margin. For example:

- higher is better -- beat the constant predictor by five percentage points of
  agreement rate: `{ comparator: gt, baseline: constant-predictor, margin: 0.05 }`;
- lower is better -- cut p50 latency by at least 20 ms against the compared
  collection: `{ comparator: le, baseline: prior-collection, margin: 20 }`, so
  a 200 ms prior needs 180 ms or less.

When `objective` is present the comparator agrees with it: `higher` takes `gt`
or `ge`, `lower` takes `lt` or `le`, `zero` takes `eq` or `le` against
threshold 0, and `target` takes any comparator.

The rule does not restate `metric`, `repetitions`, or `minimum_population`: it
applies to the plan's own `metric`, and a result over fewer than
`minimum_population` items is refused before the rule is read. `population`,
`sampling`, `error_model`, and `uncertainty` remain prose. The estimator and
rule are part of the measurement definition: changing either needs a new
`definition_version`, so a rule edited after results were seen starts a new
series rather than re-judging the old one.

## Protected Apparatus

`protected_apparatus` lists the files that produce the number: the harness, the
labels, corpus, or answer key, the file that selects the population, and the
checker configuration. A change that edits one of them changed the
measurement, not the thing measured, and earns no credit toward the objective.
A gate-stage plan must list them, and so must any plan that declares an
`apparatus-edit` negative control.

Each entry is either a repository-relative file path or a directory entry
ending in `/**`, which names every file under that directory, recursively
(`fixtures/juniper/requests/**` here). No other wildcard is allowed, and `**`
on its own is refused: the whole repository cannot be protected. Absolute
paths, `.` and `..` segments, empty segments, control characters, and
`\ ? [ ] { } :` are refused too.

Quoin's measurement intake resolves the entries when it writes a collection:
paths are case-sensitive, dotfiles are included, a symlink is refused rather
than followed, a directory entry must contain at least one file, and an entry
that names no file refuses the collection. It records the resolved set of
(path, digest) pairs. Any difference in that set -- a file edited, or a file
added or removed under a directory entry -- is an apparatus change.

The list is part of the measurement definition. Adding, removing, or changing
an entry needs a new `definition_version`, and so does any change to the
resolved files, which Quoin sees through their digests rather than through
this plan. Reordering the list is not a change.

The population is protected through the file that selects it
(`fixtures/juniper/population.yaml` here), not through
`statistical_design.population`: that field is prose a reader relies on, while
the file decides which items are counted.

## Negative Controls

`negative_controls` declares the gaming scenarios the plan says it guards
against, each a `kind` and a `description` of how. A gate-stage plan declares
at least one. The declaration is checked for shape only; Quoin's checker
(Linear PLAT-961) exercises the kinds it can detect -- `selective-reporting`
through its rerun-until-pass rule, and `apparatus-edit` through the digest
comparison (Linear PLAT-975). `kind` is one of:

- `suppressed-observation` -- an unfavourable item or run is left out;
- `gain-within-noise` -- an improvement smaller than the stated uncertainty is
  claimed as a gain;
- `stale-evidence` -- a result collected against an earlier subject version or
  apparatus is presented as current;
- `apparatus-edit` -- a protected file is edited alongside the change it
  grades (requires `protected_apparatus`);
- `selective-reporting` -- only a favourable run or variant is reported out of
  several tried.

## Population

The population and exclusions are fixed before collection and recorded with the
result.

## Collection Procedure

Run the declared fixture repeatedly with fixed software and configuration
identities. Preserve per-run outcomes instead of only an aggregate.

## Interpretation

Report uncertainty, invalid runs, environmental differences, and plausible
alternative explanations.

## Example: tracking two related quantities on one plan

A single MeasurementPlan can track more than one related quantity through its
recorded observations' `dimensions` field, instead of being split into one
plan per quantity. Two observations that differ only in `dimensions` are not
duplicates: the duplicate-observation check (`quoin-measurement`'s
`MeasurementObservation::identity()`) keys on the metric name *together with*
the sorted dimension entries, so two different dimension values never collide
even under the same metric and `planId`.

For example, one plan measuring both cost and latency for the same subject
records two observations under the same `metric` and `planId`, distinguished
only by `dimensions.quantity`:

```json
{
  "metric": "resource_usage",
  "planId": "mp-cost-latency",
  "dimensions": { "quantity": "cost" },
  "shape": "scalar",
  "unit": "usd",
  "state": "measured",
  "value": 4.12
}
```

```json
{
  "metric": "resource_usage",
  "planId": "mp-cost-latency",
  "dimensions": { "quantity": "latency" },
  "shape": "scalar",
  "unit": "ms",
  "state": "measured",
  "value": 812
}
```

Author one MeasurementPlan (`mp-cost-latency` above) whose Population,
Collection Procedure, and Interpretation cover both quantities, rather than
two separate plans that would duplicate everything except the quantity being
measured.

## Example: constant-predictor margin, not raw agreement, on a skewed population

A plan that grades a tool's output against recorded ground-truth labels
states its raw agreement/accuracy rate next to the **constant-predictor
baseline** — what a fixed, corpus-blind answer would score — rather than the
raw rate alone. A skewed population lets a tool that reads nothing score high
purely because one label dominates; the number that says whether the tool did
anything is the margin over this baseline, not the raw rate by itself.

**Formula.** Group the population into the answer-space families the plan's
own Population section already states — do not invent a new grouping here.
For family `f` with `n_f` items, and for each label `i` in that family's
answer space, let `n_{f,i}` count every item where label `i` is the primary
recorded reading *or* one of its recorded contested alternates (an item with
more than one defensible answer counts toward each). That family's best
constant score is `max_i(n_{f,i}) / n_f`. The whole population's baseline is
the size-weighted mean of the per-family rates — equivalently, total
best-constant agreements over total items:

```
baseline    = sum_f( max_i(n_{f,i}) ) / sum_f( n_f )     # a fraction in [0, 1]
margin      = observed_agreement_rate - baseline         # same units: a fraction, not percentage points
```

The observed agreement rate must credit contested alternates by the same
rule used to build `n_{f,i}` above; a strict observed rate compared against a
contested-inclusive baseline understates the margin.

Compute this from the corpus at measurement time. Never hard-code the
winning label or the resulting percentage: a corpus that changes moves the
baseline, and a written-down number goes stale exactly like a written-down
definition would.

**A single constant answer across every family, instead of one per family,
understates the baseline and lets a tool clear a bar it never actually beat.**
Worked example, from a real 15-item corpus split into two families with
different answer spaces (quoin's `criterion-strength-fixtures.json`, 11
`weakness_kind` items and 4 `adverse_case_coverage` items):

| family | items | best single global label (`sound`) | agrees | best per-family label |
| --- | --- | --- | --- | --- |
| `weakness_kind` | 11 | `sound` | 9 | `sound` (9/11) |
| `adverse_case_coverage` | 4 | `sound` (not in this label space) | 0 | coverage level `2` (3/4) |

A grader that answers `sound` to all 15 items scores 9/15 = **60.0%** — the
family whose answer space does not even contain `sound` contributes nothing,
and that gap is invisible in one blended global-constant number. Picking the
best label *per family* instead scores (9 + 3)/15 = **80.0%**. **80.0%, not
60.0%, is the bar a tool's own agreement rate has to clear** on this
population — the lower number was an artifact of grading one family against
another family's answer.

State the observed rate, the baseline, and the margin together, and gate on
the margin (not the raw rate) whenever the population is skewed enough that
a constant answer scores high on its own. A gate plan that requires this
margin to clear *together with* other metrics (recall, calibration, and
similar) expresses each as a `relationships` entry of `type: references` in
its own frontmatter — one per constituent MeasurementPlan — rather than only
describing the dependency in prose.
