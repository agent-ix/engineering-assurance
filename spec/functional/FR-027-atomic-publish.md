---
id: FR-027
title: "Publish files without replacement"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "requires"
---

# FR-027: Publish files without replacement

## Description

Engineering Assurance SHALL expose one small filesystem capability, behind its
own Cargo feature, that a consumer uses to retain evidence files: an atomic
publish that never replaces an existing file.

## Inputs

- Publish: a destination path, the bytes to publish, and a caller-supplied
  collision value returned when a different file already occupies the
  destination.

## Outputs

- Publish: success, or a typed `PublishError` that is either a collision
  carrying the destination path and the caller's collision value, or an I/O
  failure naming the failed operation and path.

## Behavior

- Publish SHALL write the bytes to a uniquely named sidecar beside the
  destination, created exclusively, and commit it with a hard link. It SHALL
  NOT use rename, because rename replaces an existing destination silently.
- When the destination already exists as a regular file with identical bytes,
  publish SHALL succeed and leave it untouched. When it holds different bytes,
  or is not a regular file, publish SHALL return the collision and leave it
  untouched.
- Publish SHALL remove its sidecar on every path, success or failure.
- The commit step SHALL be a separate step over an already-written sidecar, so
  a destination that appears between the caller's check and the commit is
  testable without threads or sleeps. It is not public API.
- Publish SHALL retry a taken sidecar name a bounded number of times, and SHALL
  compare an existing destination through one descriptor opened without
  following links.
- The module is a filesystem module, like `producer_execution`, and is
  exempt from the I/O-freedom audit of [FR-014](./FR-014-versioned-rust-boundary.md).

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-027-CON-1 | The capability SHALL NOT spawn a process. | Security | Test (TC-127) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-027-AC-1 | Publishing to an absent destination creates it with exactly the given bytes and leaves no sidecar in the directory. | Test (TC-206) |
| FR-027-AC-2 | Publishing over an identical existing file succeeds; over a different file, a directory or a symbolic link it returns the typed collision with the destination unchanged; a destination that appears before the commit step is handled the same way; the sidecar is removed in every case. | Test (TC-207) |

FR-027-AC-3 is withdrawn. Its row is removed and the identifier is not reused.

## Dependencies

- **Upstream**: [FR-014](./FR-014-versioned-rust-boundary.md) for the Cargo feature discipline.
