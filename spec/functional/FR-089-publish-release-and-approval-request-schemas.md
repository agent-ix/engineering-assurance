---
id: FR-089
title: "Publish request schemas for release and approval commands"
type: FR
---
# FR-089: Publish request schemas for release and approval commands

## Description

Engineering Assurance SHALL make the exact request contract for each supported
release and approval command discoverable without executing the request.

## Inputs

- A CLI invocation of `release-history`, `release-decision`, `release-verify`,
  or `approval-tag` with `--schema request`.
- A CLI invocation that supplies a request document or invalid schema options.

## Outputs

- The command's published JSON Schema document on standard output.

## Behavior

- Each of the four commands SHALL accept `--schema request` without a request
  document and print a valid JSON Schema for its own request contract.
- Each published schema SHALL identify the command's actual request protocol.
- Each published schema SHALL describe its required and optional fields, field
  types, enumerated values, and closed object shape.
- A command SHALL reject a schema kind other than `request` before request
  execution.
- A command SHALL reject combining `--schema` with `--request` before request
  execution.
- A command SHALL reject `--schema` without a value.
- A command SHALL require `--request` when schema discovery is not selected.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-089-AC-1 | `release-history --schema request` prints a valid JSON Schema describing the `engineering-assurance.release-history-request/v1` contract, including the required nullable `previous_release_tag` member. | Test |
| FR-089-AC-2 | `release-decision --schema request` prints a valid JSON Schema describing the `engineering-assurance.release-decision-request/v1` contract, including its operation-specific optional members and `release`/`reject` choice values. | Test |
| FR-089-AC-3 | `release-verify --schema request` prints a valid JSON Schema describing the `engineering-assurance.release-verify-request/v1` contract, including the optional `gitsign` and `identity_registry` members. | Test |
| FR-089-AC-4 | `approval-tag --schema request` prints a valid JSON Schema describing the `engineering-assurance.approval-tag-request/v1` contract, including the defaultable `revision` and optional `gesture` members. | Test |
| FR-089-AC-5 | For each command, an unsupported schema value, a missing schema value, or simultaneous `--schema` and `--request` is rejected by the CLI without running the request; omitting both options continues to be rejected. | Test |

## Dependencies

- **Upstream**: [FR-042](./FR-042-create-signed-approval-tag-for-a-change-assurance-record.md), [FR-047](./FR-047-release-record-and-signed-release-tag.md), [FR-048](./FR-048-human-release-decision-workflow.md), [FR-052](./FR-052-release-history-since-the-previous-release.md) define the request contracts published here.
