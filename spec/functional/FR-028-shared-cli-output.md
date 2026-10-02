---
id: FR-028
title: Consume shared CLI output utilities
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: constrains
---

# FR-028: Consume shared CLI output utilities

## Description

The native CLI SHALL consume the authoritative ix-cli-kit output writer and
JSON encoder instead of maintaining duplicate implementations.

## Inputs

- Complete domain response bytes prepared by the selected capability.
- The typed machine error document prepared by the CLI boundary.

## Outputs

- The existing complete primary response on stdout.
- The existing human diagnostic on stderr.

## Behavior

The CLI SHALL delegate stdout write and flush to the shared writer. The CLI
SHALL encode its machine error document through the shared compact JSON encoder
and append its existing newline. Domain documents, error codes, exit statuses
and stream selection SHALL remain unchanged. Minimal library consumers SHALL
NOT enable the shared CLI dependency unless the full CLI feature is requested.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-028-AC-1 | Successful and rejected fictional workflow requests retain their exact primary document bytes, stderr role and exit status through the real CLI process. | Test |
| FR-028-AC-2 | A malformed fictional stdin request emits the existing typed machine error document with exactly one final newline. | Test |
| FR-028-AC-3 | The empty default and each individual library feature continue to compile without enabling the shared CLI dependency. | Test |

## Dependencies

- The authoritative ix-cli-kit stream writer and JSON encoder.
- The existing domain response encoders and CLI process acceptance suite.
