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
  bound: 0.99
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
(`bound` is then required). For the other directions `bound` is optional and
names the threshold measured against; here, a retention rate of at least 0.99.

The objective is part of the measurement definition. Adding, removing, or
changing it requires a new `definition_version`, so results under the old and
new objective are never compared as one series.

## Decision Rule

`statistical_design.estimator` names how the metric is computed from the
population: `proportion`, `count`, `mean`, `median`, or `ratio`.
`statistical_design.decision_rule` is the rule a checker applies to that
estimate, computed over all `repetitions`: it holds when
`estimate <comparator> reference`. `comparator` is one of `gt`, `ge`, `lt`,
`le`, or `eq`. The reference is exactly one of:

- `threshold`: a fixed number stated in the plan, as here (retention of at
  least 0.99); or
- `baseline`: a value computed at evaluation time, one of
  `constant-predictor` (the best constant answer per answer family, see the
  worked example below), `prior-collection` (the collection this result is
  compared against), or `best-seen` (for a ratchet: the maximum accepted value
  so far under a `gt`/`ge` rule, the minimum under an `lt`/`le` rule; an `eq`
  rule cannot use it), plus an optional signed `margin` added to it.

For example, a plan that must beat the constant predictor by five percentage
points states `decision_rule: { comparator: gt, baseline: constant-predictor,
margin: 0.05 }`. The rule does not restate `metric`, `repetitions`, or
`minimum_population`: it applies to the plan's own `metric`, and a result over
fewer than `minimum_population` items is refused before the rule is read.
`population`, `sampling`, `error_model`, and `uncertainty` remain prose.

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
baseline    = sum_f( max_i(n_{f,i}) ) / sum_f( n_f )     # both rates below are fractions in [0, 1]
margin_pp   = 100 * (observed_agreement_rate - baseline)
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
