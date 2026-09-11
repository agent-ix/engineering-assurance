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
- For the package/archive capability, run the retained Python audit and the Rust
  adapter successfully at one candidate revision, record a reversible
  `package-audit` dispatch cutover, and then remove `scripts/audit_packages.py`,
  `tests/test_packages.py`, `scripts/check_content_rights.py`, and
  `tests/test_content_rights.py` together with all executable test imports or
  subprocess references to those paths and the two temporary Rust semantic-
  policy exemptions. Historical review references remain inert records.
- For the compatibility, accepted-corpus, and fixture-generation capability,
  keep the retained Python until native Rust compatibility classification,
  accepted-corpus access, corpus identity and integrity checking, and
  deterministic fixture generation all pass locally at one candidate revision;
  replace the Python-backed Rust differential with checked-in expected fixtures
  so no Python is executed as a test oracle after cutover; and then remove
  `engineering_assurance/compatibility.py`,
  `engineering_assurance/compatibility_corpus.py`,
  `engineering_assurance/fixture_codegen.py`,
  `tests/test_compatibility_matrix.py`, and
  `tests/test_compatibility_corpus.py` together with every executable import of
  those paths.
- For the verification-semantics capability, keep the retained Python until
  native Rust semantic-reference and fixture validation, ownership-registry
  validation, historical PGM-01 mapping, and bounded report rendering all pass
  locally at one candidate revision; replace every Python-backed Rust
  differential for that capability with expected fixtures captured once from the
  retained implementation and committed to this repository, so no Python is
  executed as a test oracle after cutover; carry every acceptance criterion the
  retired Python tests backed on a tracking-tagged Rust test that fails when its
  property is violated; and then remove
  `engineering_assurance/verification_semantics.py` and
  `tests/test_verification_semantics.py` together with every executable import
  or subprocess reference to those paths.
- Retain Python packaging support and unrelated Python onboarding and workflow
  code across that removal, and change no retained corpus byte.
- For the canonical-discovery and ix-flow workflow capability, port canonical
  bundle discovery to Rust before cutting over the workflow lane that reads its
  promoted workflow names, because the retained workflow module imports that
  name set from the retained discovery module. Keep the retained Python until
  Rust host-surface manifest validation, canonical skill and workflow inventory,
  bounded bundle-relative target resolution, and the ix-flow lifecycle adapter
  all pass locally at one candidate revision; restate every criterion the
  retired Python tests carried against a Rust assertion that fails when the
  property is violated, including interrupted-run resume from a completed phase,
  the human gate configured on every canonical terminal transition, and each
  binding field whose change must refuse a mismatched run; update the canonical
  skill's own invocation text to the Rust workflow-host capability; and then
  remove `engineering_assurance/discovery.py`,
  `engineering_assurance/workflow.py`, `tests/test_discovery.py`,
  `tests/test_workflow_resume.py`, and `tests/test_workflow_integration_gate.py`
  together with every executable import of those paths.
- Removal of a retained module SHALL NOT be recorded as complete while a
  criterion it carried is backed only by a test that asserts against a fixture
  the test itself wrote or by a check that passes over an empty population.
- Where a retired Python test carries a criterion with no native replacement,
  relocate that test rather than deleting it, and record which criterion it
  carries. The accepted corpus reproducing from its recorded sources is that
  case: the builder belongs to the pinned corpus repository, and invoking it is
  not the same as owning it.
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
