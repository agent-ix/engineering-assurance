---
id: FR-055
title: "Select a record sink per repository, defaulting from the repository's visibility"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-054"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-008"
    type: "references"
---

# FR-055: Select a record sink per repository, defaulting from the repository's visibility

## Description

Tracked by Linear EA-105. Each repository chooses where its records go. Without an explicit choice, the repository's
visibility decides: a public repository publishes to the public log and a private repository to the private store. A
repository may override the default in either direction, so an open-standard repository keeps the public default and a
public repository may choose the private store or `none`.

The sink configuration is one JSON document, `engineering-assurance.sink-config/v1`, at the path
`.engineering-assurance/sinks.json`, read from the repository's protected default branch and never from the branch or
pull request under change. It is a closed schema ([NFR-008](../non-functional/NFR-008-closed-schemas.md)).

A repository whose visibility cannot be determined takes the private store as its default: the default never falls
toward the public log.

Out of scope: the sink behaviors ([FR-054](./FR-054-record-sink-interface-and-implementations.md)) and the leak check
that still applies to an explicit public-log choice ([FR-057](./FR-057-fail-closed-pre-publish-leak-check.md)).

## Inputs

- The configuration document with the members `protocol`, optionally `sink` (`public_log`, `private_store` or
  `none`), `public_log` (`endpoint`, an https URL) and `private_store` (`destination`, a non-empty string) and `public_linear_teams` (a list of Linear team keys, default
  empty, read by [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md)). Unknown
  members are rejected.
- The repository's visibility as the hosting service reports it: `public`, `private` or undeterminable.

## Outputs

- The resolved selection: the sink, and whether it came from the `configured` member or the `default`.
- A configuration that selects `public_log` without a `public_log.endpoint`, or `private_store` without a
  `private_store.destination`, is refused with `sink_config_incomplete`.

## Behavior

- Engineering Assurance SHALL resolve the sink from the `sink` member when the configuration holds one.
- Engineering Assurance SHALL resolve `public_log` when the `sink` member is absent and the repository is public.
- Engineering Assurance SHALL resolve `private_store` when the `sink` member is absent and the repository is private.
- Engineering Assurance SHALL resolve `private_store` when the `sink` member is absent and the visibility cannot be
  determined.
- Engineering Assurance SHALL read the configuration from the protected default branch.
- Engineering Assurance SHALL refuse a configuration document that has an unknown member with `sink_config_invalid`.
- Engineering Assurance SHALL NOT infer a sink from any record content.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-055-AC-1 | With no configuration, a public repository resolves `public_log` with source `default` and a private repository resolves `private_store` with source `default`. | Test |
| FR-055-AC-2 | A public repository configured `private_store` or `none`, and a private repository configured `public_log` or `none`, resolve the configured sink with source `configured`. | Test |
| FR-055-AC-3 | A repository whose visibility lookup fails or is ambiguous and which has no `sink` member resolves `private_store`. | Test |
| FR-055-AC-4 | A configuration with an unknown member, an unknown `sink` value, a non-https `endpoint` or a missing endpoint or destination for the selected sink is refused with `sink_config_invalid` or `sink_config_incomplete`, and nothing is published. | Test |
| FR-055-AC-5 | A pull request branch that changes `.engineering-assurance/sinks.json` does not change the sink resolved for that pull request; the default branch's document is used. | Test |

## Dependencies

- **Upstream**: [FR-054](./FR-054-record-sink-interface-and-implementations.md),
  [NFR-008](../non-functional/NFR-008-closed-schemas.md).
- **Downstream**: [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md); the EA-97 and EA-98 specifications.
