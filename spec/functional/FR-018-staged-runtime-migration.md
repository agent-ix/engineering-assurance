---
id: FR-018
title: "Retire replaced Engineering Assurance executable paths"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: "requires"
---

# FR-018: Retire replaced Engineering Assurance executable paths

## Description

Engineering Assurance SHALL retain each old Python, JavaScript, or MJS
implementation until its Rust replacement demonstrates equivalent behavior,
then update this repository's direct invocation and remove the old path.

## Inputs

- The executable-path matrix in ADR-002.
- The reviewed Rust library, CLI, and supported host interfaces.
- Same-revision differential, package, rights, and integration evidence.

## Outputs

- A disposition for each first-party executable path in this repository.
- A final inventory of retained configuration and inert foreign-language data.

## Behavior

- Follow ADR-002's port and cutover order.
- Keep an old path until its Rust replacement passes locally at the same
  candidate revision.
- Update direct Engineering Assurance invocations and supported host
  configuration before deleting the replaced path.
- For the content-rights tree capability, compare the retained Python and Rust
  gates at one candidate revision over the same Git-selected population and
  record identical accepted/refused status plus exact path, line, and category
  tuples before changing the direct `make test` invocation.
- Keep the content-rights checker and its tests recoverable while proving that
  reverting the dispatch cutover restores the retained invocation. Remove the
  checker, its Python tests, and their temporary Rust policy exemptions only
  after the Rust dispatch passes independently and the package/archive audit no
  longer imports the retained classifier.
- Preserve historical corpus and evidence bytes.
- Return changed interfaces or compatibility promises to specification before
  implementation continues.

## Error Conditions

Missing parity evidence, mismatched candidate revisions, unavailable replacement
hosts, changed historical bytes, and unresolved path dispositions block removal.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-018-CON-1 | Deletion SHALL be the final step for each replaced capability. | Lifecycle | Test |
| FR-018-CON-2 | Cutover SHALL NOT rewrite historical evidence or corpus bytes. | Data Integrity | Test |
| FR-018-CON-3 | Temporary coexistence SHALL NOT be reported as completed remediation. | Reporting | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-018-AC-1 | Every first-party executable-path matrix row records one current state and one final disposition. | Test (TC-097) |
| FR-018-AC-2 | Removal is refused unless old and new paths pass at the same candidate revision and direct invocations use the Rust interface. | Property (TC-113) |
| FR-018-AC-3 | Before deletion, a failed Rust cutover can restore the previous invocation without rewriting historical evidence or corpus bytes. | Test (TC-114) |
| FR-018-AC-4 | Final inventory and static scans find no unapproved first-party non-Rust semantic or assertion logic and exclude inert fixture samples from executable debt. | Test (TC-115) |

## Dependencies

- **Upstream**: accepted ADR-002 and completed FR-014 through FR-017.
- **Downstream**: none.
