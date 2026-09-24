# Measurement walkthrough: from a MeasurementPlan to a report

This walkthrough records one measurement against a `MeasurementPlan` and reads
it back. It uses Quoin's `quoin measurement record` and `quoin report`
commands. Engineering Assurance supplies the `MeasurementPlan` type; Quoin owns
the collection format and the measurement store.

Every file below was recorded and reported with quoin 0.24.1 (the version the
[compatibility matrix](../engineering_assurance/compatibility-matrix.json)
pins) and with quoin 0.23.1, and validated with quire-cli 0.33.0 against the
v0.4.0 and v0.4.1 modules. The service, harness, and digests are fictional. Run the
commands from the repository root, or pass `--repo <repo_root>`.

## 1. Write the plan

Quoin finds plans by walking every `*.md` file under `spec/assurance/` and
`assurance/` in the repository. A file counts as a plan when its frontmatter
has `type: MeasurementPlan`. The file name does not matter.

`spec/assurance/MP-001-request-retention.md`:

```markdown
---
id: MP-001
title: Juniper request-retention measurement
type: MeasurementPlan
status: active
owner: juniper-observability-owner
metric: request_retention_rate
definition_version: juniper.request-retention-v1
stage: baseline
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

## Population

Fictional valid requests accepted by the candidate service, fixed before
collection.

## Collection Procedure

Run the declared fixture five times with fixed software and configuration
identities. Keep each run's outcome.

## Interpretation

Report every run, the uncertainty interval, and any invalid runs.
```

Validate it:

```bash
quire validate --scope . 'spec/**/*.md'
```

This needs the module installed from v0.4.0 or later. A module installed from an older
commit refuses `statistical_design.decision_rule` and `minimum_population`;
`quoin module list` shows which `ref` you have.

The full field list, including `objective`, `protected_apparatus`, and
`negative_controls`, is in
[`engineering_assurance/skeletons/MeasurementPlan.md`](../engineering_assurance/skeletons/MeasurementPlan.md).

Keep one plan per `metric`. Quoin keys plans by metric, and when two plans
share one, the plan with the higher `id` is used, whatever its `status`.

## 2. Write the collection JSON

A collection is one complete run of the tool that produces the number.
`collection.json`:

```json
{
  "schemaVersion": 2,
  "collectionId": "juniper-retention-2026-09-23-run1",
  "subject": "juniper-classifier 2026.09.1",
  "scope": { "requestClasses": ["standard", "bulk"] },
  "toolIdentity": "juniper retention harness",
  "toolVersion": "1.0.0",
  "configDigest": "sha256:3333333333333333333333333333333333333333333333333333333333333333",
  "timestamp": "2026-09-23T12:00:00.000Z",
  "sourceRevision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "environment": { "runner": "local" },
  "verificationStack": {
    "schemaVersion": "verification-stack-attestation-v1",
    "lockDigest": "sha256:1111111111111111111111111111111111111111111111111111111111111111",
    "executableDigest": "sha256:2222222222222222222222222222222222222222222222222222222222222222",
    "buildProfile": "release",
    "toolchains": { "node": "22.12.0", "rust": "1.98.1", "python": "3.12.8" },
    "sources": {
      "juniper": {
        "revision": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "sourceState": "clean",
        "remote": "origin"
      }
    },
    "capabilities": ["juniper.retention"],
    "artifacts": {
      "harness-config": "sha256:3333333333333333333333333333333333333333333333333333333333333333"
    }
  },
  "observations": [
    {
      "metric": "request_retention_rate",
      "planId": "MP-001",
      "definitionVersion": "juniper.request-retention-v1",
      "state": "measured",
      "value": 0.994,
      "unit": "fraction",
      "shape": "ratio",
      "population": { "examined": 1000, "matched": 994, "complete": true, "repetitions": 5 }
    }
  ],
  "rawEvidence": { "runs": [0.995, 0.993, 0.994, 0.996, 0.992] }
}
```

### How an observation binds to the plan

Each observation must match a plan on three fields:

| Collection field | Plan field | Rule |
| --- | --- | --- |
| `observations[].metric` | `metric` | Must name a plan whose `status` is `active`. |
| `observations[].planId` | `id` | Must equal the plan's id. |
| `observations[].definitionVersion` | `definition_version` | Must be equal. |

From quoin 0.24.0, a `measured` observation is also checked against the plan's
`statistical_design`. This is why the example states `population.examined`
and `population.repetitions`:

| Plan field | Observation needs | Refusal |
| --- | --- | --- |
| `minimum_population: 200` | `population.examined` of at least 200 | `QM-POPULATION-UNSTATED` if absent, `QM-POPULATION-BELOW-MINIMUM` if smaller |
| `repetitions: 5` | `population.repetitions` of at least 5 (required whenever the plan's value is above 1) | `QM-POPULATION-UNSTATED` if absent, `QM-REPETITIONS-SHORT` if smaller |

Quoin owns the rest of the collection format and names the failing field when
it refuses a record. The onboarding report restates the full field contract:

```bash
node engineering_assurance/skills/assurance-onboarding/scripts/onboard.js --repo <repo_root>
```

Quoin 0.24.0 and later accept a `verificationStack.toolchains` that names any
one of `node`, `rust`, and `python`. The example names all three because quoin
0.23.1 requires all three and cannot read back a record that names fewer.

## 3. Record it

```bash
quoin measurement record --input collection.json
```

On success it prints the stored path:

```text
spec/evidence/measurements/juniper-retention-2026-09-23-run1.json
```

The printed path is relative to `--repo`, which defaults to `.`.

Records are write-once. Recording the same bytes again succeeds and changes
nothing. Different content under an existing `collectionId` is refused, so use
a new id for each run.

## 4. Read it back

```bash
quoin report
```

```text
# QA measurement report

## What this repository measures

| Metric | Plan | Stage | Current |
| --- | --- | --- | --- |
| request_retention_rate | MP-001 (spec/assurance/MP-001-request-retention.md) | baseline | 0.994 fraction |

## Current evidence

Corpus gaps: not_computed

- 2026-09-23T12:00:00.000Z — juniper retention harness 1.0.0; source aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa; corpus n/a; config sha256:3333333333333333333333333333333333333333333333333333333333333333

## Attention

No factual attention items.
```

Other views:

| Command | Shows |
| --- | --- |
| `quoin report --series request_retention_rate` | Every recorded value of one metric. |
| `quoin report --since <sourceRevision>` | The latest record compared with the record whose `sourceRevision` equals the argument. The string is matched as recorded, not resolved through Git, so `HEAD` or a short SHA finds nothing. |
| `quoin report --format json` | The same data as JSON. |

To decide whether a recorded measure moves the plan to its next `stage`, use
the `measurement-promotion` workflow through the `assurance-onboarding` skill.
When the governing profile's `measurement_policy` has `mode: require` and
lists that stage, the promotion also needs a verdict from
`quoin measurement verify`: check that `quoin measurement --help` lists
`verify`. Without a verdict the workflow refuses the promotion with
`promotion_checker_missing` unless the owner records a current exception.
