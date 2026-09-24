# Measurement walkthrough: from a MeasurementPlan to a report

This walkthrough records one measurement against a `MeasurementPlan` and reads
it back. It uses Quoin's `quoin measurement record` and `quoin report`
commands. Engineering Assurance supplies the `MeasurementPlan` type; Quoin owns
the collection format and the measurement store.

Every file below was run with quoin 0.24.1 and quire-cli 0.33.0, and both
commands succeeded. The service, harness, and digests are fictional.

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

The full field list, including `objective`, `protected_apparatus`, and
`negative_controls`, is in
[`engineering_assurance/skeletons/MeasurementPlan.md`](../engineering_assurance/skeletons/MeasurementPlan.md).

Keep one plan per `metric`. Quoin keys plans by metric, so a second plan with
the same metric replaces the first when records are matched.

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
    "toolchains": { "rust": "1.98.1" },
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

The plan's `statistical_design` is also checked. `repetitions: 5` in the plan
requires `population.repetitions: 5` in the observation. Without it the record
is refused with `QM-POPULATION-UNSTATED`.

### Rules that catch most first attempts

- `schemaVersion` is `2`. Version 1 is read-only history.
- `collectionId` becomes the file name. Use letters, digits, `.`, `_`, and `-`.
- `configDigest`, `lockDigest`, `executableDigest`, and every `artifacts`
  value are `sha256:` followed by 64 lowercase hex characters.
- `verificationStack.buildProfile` is `release`.
- `verificationStack.toolchains` names at least one of `node`, `rust`, or
  `python`.
- Each `sources` entry has a 40-character lowercase hex `revision` and
  `sourceState: clean`.
- `state` is `measured` or `not_computed`. With `not_computed`, write
  `"value": null`. Leaving `value` out is refused.
- `shape` is `scalar`, `ratio`, or `count`.
- Two observations with the same `metric` and `dimensions` are refused.
- If an `artifacts` key names a file that exists in the repository, its digest
  must match that file. `quoin measurement record --digest-from-file
  artifacts.<name>=<path>` fills or checks a digest from a file.

The onboarding report lists the full field contract:

```bash
node engineering_assurance/skills/assurance-onboarding/scripts/onboard.js --repo <repo_root>
```

## 3. Record it

```bash
quoin measurement record --input collection.json
```

On success it prints the stored path:

```text
<repo_root>/spec/evidence/measurements/juniper-retention-2026-09-23-run1.json
```

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
```

Other views:

| Command | Shows |
| --- | --- |
| `quoin report --series request_retention_rate` | Every recorded value of one metric. |
| `quoin report --since <revision>` | Changes since a Git revision. |
| `quoin report --format json` | The same data as JSON. |

To decide whether a recorded measure moves the plan to its next `stage`, use
the `measurement-promotion` workflow through the `assurance-onboarding` skill.
