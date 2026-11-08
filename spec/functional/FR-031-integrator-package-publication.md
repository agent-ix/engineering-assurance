---
id: FR-031
title: "Publish an integrator package bound to its candidate, atomically and idempotently"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-006"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-030"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-027"
    type: "requires"
---

# FR-031: Publish an integrator package bound to its candidate, atomically and idempotently

## Description

Engineering Assurance SHALL publish an integrator package into a store root so
that the object is bound to its candidate and attempt, becomes visible only
after strict verification, is never replaced, and turns disagreement into a
conflict instead of a winner. It is a filesystem module in the same
`integrator-package` feature and, like FR-027, is exempt from the FR-014-AC-4
I/O-freedom audit.

## Inputs

- Publish: an existing store root directory and an `IntegratorPackage`.
- Read: the store root, a candidate identity and an attempt identity.

## Outputs

- Publish: the object path and `Written` or `AlreadyPresent`, or a typed error.
- Read: the verified `IntegratorPackage`, or a typed error that distinguishes a
  conflict, an unbound object, a non-regular object, a strict-reader refusal
  and a filesystem failure.

## Behavior

- The object path SHALL be `<root>/<candidate>/<attempt>.json`. The reader SHALL
  refuse an object whose own candidate and attempt differ from that path.
- Publish SHALL produce the bytes through the FR-030 writer, which has read them
  back strictly, before touching the store; an invalid package SHALL write
  nothing, not even the candidate directory. The bytes SHALL reach the
  destination only through the FR-027 no-replace publisher, and the committed
  object SHALL be read back from the store and equal the published package.
- Publishing identical bytes again SHALL succeed as `AlreadyPresent`.
- Different bytes for an attempt that already holds a valid package SHALL NOT
  be written. A link, directory or corrupt object at the path SHALL be reported
  as itself and SHALL NOT mark the attempt. The refused publisher SHALL record a conflict marker
  `<attempt>.conflict` beside the object. An attempt with a marker SHALL read
  and publish as a conflict for every caller, so neither version is presented
  as a timing-selected winner. A reader that runs between the winning commit
  and the losing writer's marker can still read the winner's object; this
  window is a documented limit, closed only by waiting for the known writers.
  The creating writer's own read-back reports the conflict if the marker is
  already present, so with non-identical writers at most one reports `Written`.
- Different attempts of one candidate, and different candidates, SHALL be
  independent objects.
- The reader SHALL read an object only as a regular file (not a link or
  directory) within the package size bound, and SHALL refuse a truncated or
  altered object through the strict reader.
- The module SHALL NOT spawn a process or follow links to an object.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-031-AC-1 | A valid package is published at its candidate and attempt path, reads back equal, and leaves no sidecar; an invalid package writes nothing; a missing store root is refused. | Test (TC-225) |
| FR-031-AC-2 | Publishing the same package again reports `AlreadyPresent`, resolves to the same path, and leaves the bytes and directory unchanged. | Test (TC-226) |
| FR-031-AC-3 | Reusing an attempt with a changed fact is a conflict; the original object is not replaced; after it the attempt reads as a conflict, and the original writer's identical retry is a conflict too. | Test (TC-227) |
| FR-031-AC-4 | Eight concurrent writers of non-identical packages for one attempt produce at most one `Written`, no outcome other than `Written` or conflict, and a conflict thereafter for every reader; eight identical writers produce one `Written`, no error and a readable object. | Test (TC-228) |
| FR-031-AC-5 | The reader refuses an object copied under another attempt or candidate, a truncated object, a directory, a missing object, and a symbolic link; a corrupt or directory occupant of an attempt's path is reported as such and leaves no conflict marker. | Test (TC-229) |
| FR-031-AC-6 | Two attempts of one candidate, and one attempt id under two candidates, coexist as independent verified objects. | Test (TC-230) |

## Dependencies

- **Upstream**: [FR-030](./FR-030-integrator-package-contract.md) and
  [FR-027](./FR-027-atomic-publish.md).
