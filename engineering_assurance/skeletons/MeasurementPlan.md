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
