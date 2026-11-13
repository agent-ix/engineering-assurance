---
id: FR-055
title: "Select a record sink per repository, defaulting from the repository's visibility"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-054"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-057"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-008"
    type: "references"
---

# FR-055: Select a record sink per repository, defaulting from the repository's visibility

## Description

Tracked by Linear EA-105. Each repository chooses where its records go. Without an explicit choice, the repository's
visibility decides: a public repository publishes to the public log and a private repository to the private store.
A repository may override the default: a public repository may choose the private store or `none`, and a private
repository may choose `none`. The public log is never available to a private repository, by default or by override
(the publishing repository is itself a checked reference, [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md)).
A repository whose visibility cannot be determined counts as private.

The protected base branch is the branch a change merges into; for a publish not tied to a pull request it is the
repository's default branch. Every policy document of this specification (this sink configuration, the role policy of
[FR-061](./FR-061-role-policy-and-distinct-individuals.md) and the workflow-identity policy of
[FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md)) is read from the protected base branch
and never from the branch or pull request under change. If the source does not report the base branch as protected,
the documents are refused with `policy_branch_unprotected` and the operation that needed them fails closed.

The sink configuration is one JSON document, `engineering-assurance.sink-config/v1`, at the path
`.engineering-assurance/sinks.json`. It is a closed schema ([NFR-008](../non-functional/NFR-008-closed-schemas.md)).

Out of scope: the sink behaviors ([FR-054](./FR-054-record-sink-interface-and-implementations.md)) and the leak check
([FR-057](./FR-057-fail-closed-pre-publish-leak-check.md)).

## Inputs

- The configuration document with the members `protocol`, optionally `sink` (`public_log`, `private_store` or
  `none`), `public_log` (`endpoint`, an https URL), `private_store` (`destination`, a non-empty string) and
  `public_linear_teams` (a list of Linear team keys, default empty, read by FR-057). Unknown members are rejected.
- The repository's visibility as the hosting service reports it: `public`, `private` or undeterminable.
- Whether the source reports the protected base branch as protected.

## Outputs

- The resolved selection: the sink, and whether it came from the `configured` member or the `default`.
- A configuration that selects `public_log` without a `public_log.endpoint`, or `private_store` without a
  `private_store.destination`, is refused with `sink_config_incomplete`.
- A configuration that resolves to `public_log` without a `private_store.destination` is refused with
  `sink_config_private_store_required`: the full private form always goes to a private store ([FR-054](./FR-054-record-sink-interface-and-implementations.md)).
- A configuration that selects `public_log` for a repository that is private or of undeterminable visibility is
  refused with `sink_config_public_log_for_non_public_repo`.

## Behavior

- Engineering Assurance SHALL resolve the sink from the `sink` member when the configuration holds one.
- Engineering Assurance SHALL resolve `public_log` when the `sink` member is absent and the repository is public.
- Engineering Assurance SHALL resolve `private_store` when the `sink` member is absent and the repository is private.
- Engineering Assurance SHALL resolve `private_store` when the `sink` member is absent and the visibility cannot be
  determined.
- Engineering Assurance SHALL refuse a configuration that selects `public_log` for a repository that is not public.
- Engineering Assurance SHALL refuse a configuration that resolves to `public_log` without a private store destination.
- Engineering Assurance SHALL refuse a configuration document that has an unknown member with `sink_config_invalid`.
- Engineering Assurance SHALL read each policy document from the protected base branch.
- Engineering Assurance SHALL refuse a policy document with `policy_branch_unprotected` when the base branch is not
  reported as protected.
- Engineering Assurance SHALL NOT infer a sink from any record content.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-055-AC-1 | With a configuration that holds only a private store destination, a public repository resolves `public_log` with source `default` and a private repository resolves `private_store` with source `default`; a public repository with no private store destination is refused with `sink_config_private_store_required`. | Test |
| FR-055-AC-2 | A public repository configured `private_store` or `none`, and a private repository configured `private_store` or `none`, resolve the configured sink with source `configured`; a private repository configured `public_log` is refused with `sink_config_public_log_for_non_public_repo` and nothing is published. | Test |
| FR-055-AC-3 | A repository whose visibility lookup fails or is ambiguous and which has no `sink` member resolves `private_store`; the same repository configured `public_log` is refused with `sink_config_public_log_for_non_public_repo`. | Test |
| FR-055-AC-4 | A configuration with an unknown member, an unknown `sink` value, a non-https `endpoint` or a missing endpoint or destination for the selected sink is refused with `sink_config_invalid` or `sink_config_incomplete`, and nothing is published. | Test |
| FR-055-AC-5 | A pull request branch that changes `.engineering-assurance/sinks.json`, the role policy or the workflow-identity policy does not change the document used for that pull request; the protected base branch's document is used. | Test |
| FR-055-AC-6 | When the source reports the base branch as unprotected, or does not report protection, each of the three documents is refused with `policy_branch_unprotected` and the operation that needed it fails. | Test |

## Dependencies

- **Upstream**: [FR-054](./FR-054-record-sink-interface-and-implementations.md),
  [NFR-008](../non-functional/NFR-008-closed-schemas.md).
- **Downstream**: [FR-057](./FR-057-fail-closed-pre-publish-leak-check.md),
  [FR-061](./FR-061-role-policy-and-distinct-individuals.md),
  [FR-064](./FR-064-verify-approval-statement-with-workflow-identity-policy.md); the EA-97 and EA-98 specifications.
