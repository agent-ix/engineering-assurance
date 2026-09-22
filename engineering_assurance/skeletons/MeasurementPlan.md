---
id: MP-001
title: Juniper request-retention measurement
type: MeasurementPlan
status: proposed
owner: juniper-observability-owner
metric: request_retention_rate
definition_version: juniper.request-retention-v1
stage: baseline
statistical_design:
  population: fictional valid requests accepted by the candidate service
  sampling: deterministic seeded sample across declared request classes
  repetitions: 5
  estimator: retained-result proportion
  error_model: independent run variation and fixture selection
  uncertainty: report every run and a bootstrap interval
  decision_rule: escalate when the lower interval bound is below the owned threshold
relationships:
  - target: ix://example/juniper/AP-001
    type: measures
---

# Juniper request-retention measurement

## Decision Use

The result informs whether the request-loss scenario needs more investigation;
it does not approve a release.

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
