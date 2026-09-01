---
id: TM-001
title: engineering-assurance Onboarding Test Matrix
type: TestMatrix
---

# Engineering-Assurance Onboarding Test Matrix

## Overview

This matrix records completed verification for the canonical assurance-onboarding
bundle. Every test case has an executable tracking tag and passing repository or
retained real-agent evidence. The matrix covers every stakeholder validation
criterion, user-story example, functional and non-functional acceptance criterion,
and declared constraint in this specification.

## Test Matrix Rules

1. Every criterion maps to at least one test case.
2. Local-source and repository-source installation are exercised across every
   supported agent surface.
3. Exact supported-agent set and thin-manifest constraints include rejecting
   values outside their declared boundary.
4. Documented malformed, unavailable, missing-input, missing-executable, and
   gate-override paths are explicit test cases.
5. Workflow interruption, resume, waiting, rejection, and invalid gate override
   are distinct transitions or transition failures.
6. Existing valid artifacts, absent justification, malformed output, unavailable
   producers, missing targets, and unexpected package members are explicit edge
   cases.
7. Agent evaluations cover seven concrete scenario variants on each of four hosts:
   existing profile, no profile, malformed producer, unavailable producer,
   interruption/resume, explicit acceptance, and explicit rejection.

## Requirements Traceability

### Stakeholder Requirement Coverage

| Stakeholder Req | Trace to US/FR | Test/Validation | Coverage Status |
|-----------------|----------------|-----------------|-----------------|
| StR-001 | FR-001, FR-004, FR-005 | StR-001-VC-1..VC-3 → TC-001..TC-003 | ✅ Passing |

### User Story Coverage

| User Story | Acceptance Criteria | Test Cases | Coverage Status |
|------------|---------------------|------------|-----------------|
| US-001 | US-001-EX-1 | TC-004 | ✅ Passing |
| US-001 | US-001-EX-2 | TC-005 | ✅ Passing |
| US-002 | US-002-EX-1 | TC-010 | ✅ Passing |
| US-002 | US-002-EX-2 | TC-011 | ✅ Passing |
| US-003 | US-003-EX-1 | TC-026 | ✅ Passing |
| US-003 | US-003-EX-2 | TC-028 | ✅ Passing |
| US-004 | US-004-EX-1 | TC-021 | ✅ Passing |
| US-004 | US-004-EX-2 | TC-020 | ✅ Passing |

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
|----------------|---------------------|------------|-----------------|
| FR-001 | FR-001-AC-1 | TC-004 | ✅ Passing |
| FR-001 | FR-001-AC-2 | TC-005 | ✅ Passing |
| FR-001 | FR-001-AC-3 | TC-006 | ✅ Passing |
| FR-001 | FR-001-AC-4 | TC-007 | ✅ Passing |
| FR-001 | FR-001-AC-5 | TC-008 | ✅ Passing |
| FR-001 | FR-001-AC-6 | TC-044 | ✅ Passing |
| FR-001 | FR-001-AC-7 | TC-045 | ✅ Passing |
| FR-002 | FR-002-AC-1 | TC-009 | ✅ Passing |
| FR-002 | FR-002-AC-2 | TC-010 | ✅ Passing |
| FR-002 | FR-002-AC-3 | TC-011 | ✅ Passing |
| FR-002 | FR-002-AC-4 | TC-012 | ✅ Passing |
| FR-002 | FR-002-AC-5 | TC-013 | ✅ Passing |
| FR-003 | FR-003-AC-1 | TC-014 | ✅ Passing |
| FR-003 | FR-003-AC-2 | TC-015 | ✅ Passing |
| FR-003 | FR-003-AC-3 | TC-016 | ✅ Passing |
| FR-003 | FR-003-AC-4 | TC-017 | ✅ Passing |
| FR-003 | FR-003-AC-5 | TC-018 | ✅ Passing |
| FR-003 | FR-003-AC-6 | TC-019 | ✅ Passing |
| FR-004 | FR-004-AC-1 | TC-020 | ✅ Passing |
| FR-004 | FR-004-AC-2 | TC-021 | ✅ Passing |
| FR-004 | FR-004-AC-3 | TC-022 | ✅ Passing |
| FR-004 | FR-004-AC-4 | TC-023 | ✅ Passing |
| FR-004 | FR-004-AC-5 | TC-024 | ✅ Passing |
| FR-004 | FR-004-AC-6 | TC-025 | ✅ Passing |
| FR-004 | FR-004-AC-7 | TC-046 | ✅ Passing |
| FR-005 | FR-005-AC-1 | TC-026 | ✅ Passing |
| FR-005 | FR-005-AC-2 | TC-027 | ✅ Passing |
| FR-005 | FR-005-AC-3 | TC-028 | ✅ Passing |
| FR-005 | FR-005-AC-4 | TC-029 | ✅ Passing |
| FR-005 | FR-005-AC-5 | TC-030 | ✅ Passing |
| FR-005 | FR-005-AC-6 | TC-047 | ✅ Passing |
| FR-005 | FR-005-AC-7 | TC-048 | ✅ Passing |
| FR-006 | FR-006-AC-1 | TC-031 | ✅ Passing |
| FR-006 | FR-006-AC-2 | TC-032 | ✅ Passing |
| FR-006 | FR-006-AC-3 | TC-033 | ✅ Passing |
| FR-006 | FR-006-AC-4 | TC-034 | ✅ Passing |
| FR-006 | FR-006-AC-5 | TC-049 | ✅ Passing |
| FR-006 | FR-006-AC-6 | TC-050 | ✅ Passing |
| FR-006 | FR-006-AC-7 | TC-051 | ✅ Passing |
| FR-007 | FR-007-AC-1 | TC-035 | ✅ Passing |
| FR-007 | FR-007-AC-2 | TC-036 | ✅ Passing |
| FR-007 | FR-007-AC-3 | TC-037 | ✅ Passing |

### Non-Functional Requirement Coverage

| Non-Functional Req | Verification Method | Evidence/Test Cases | Status |
|--------------------|---------------------|---------------------|--------|
| NFR-001 | Install, discovery, digest comparison, and static scan | TC-038 | ✅ Passing |
| NFR-002 | Agent evaluation against fixture-authorized outcomes | TC-039 | ✅ Passing |
| NFR-003 | Wheel/npm member audit plus compatibility invocation | TC-040 | ✅ Passing |

### Constraint Coverage

| Constraint | Test Cases | Status |
|------------|------------|-----------------|
| FR-002-CON-1 | TC-041 | ✅ Passing |
| FR-002-CON-2 | TC-042 | ✅ Passing |
| FR-007-CON-1 | TC-043 | ✅ Passing |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
|---------|-------|------|----------|-----------|--------|
| TC-001 | Inventory precedes every onboarding proposal | E2E | P0 | StR-001-VC-1 | ✅ |
| TC-002 | Unjustified profile is not scaffolded | E2E | P0 | StR-001-VC-2 | ✅ |
| TC-003 | Named human owns every terminal outcome | Integration | P0 | StR-001-VC-3 | ✅ |
| TC-004 | Existing valid profile is inventoried and reused | E2E | P0 | FR-001-AC-1, US-001-EX-1 | ✅ |
| TC-005 | No-profile repository creates no generic profile | E2E | P0 | FR-001-AC-2, US-001-EX-2 | ✅ |
| TC-006 | Justified artifact uses installed skeleton and Quire | Integration | P0 | FR-001-AC-3 | ✅ |
| TC-007 | Incomplete boundary requests input without writing | E2E | P0 | FR-001-AC-4 | ✅ |
| TC-008 | Inventory separates all required collections | Unit | P1 | FR-001-AC-5 | ✅ |
| TC-009 | Exactly one canonical onboarding skill exists | Static | P0 | FR-002-AC-1 | ✅ |
| TC-010 | Four host surfaces resolve one canonical skill | Integration | P0 | FR-002-AC-2, US-002-EX-1 | ✅ |
| TC-011 | Canonical skill exposes exactly four workflows | Integration | P0 | FR-002-AC-3, US-002-EX-2 | ✅ |
| TC-012 | Behavioral text in a host manifest is rejected | Static | P1 | FR-002-AC-4 | ✅ |
| TC-013 | Missing or escaping canonical target is rejected | Unit | P0 | FR-002-AC-5 | ✅ |
| TC-014 | Wheel includes explicit module and onboarding members | Integration | P0 | FR-003-AC-1 | ✅ |
| TC-015 | Npm archive includes explicit module and onboarding members | Integration | P0 | FR-003-AC-2 | ✅ |
| TC-016 | Local-source installation preserves discovery | Integration | P0 | FR-003-AC-3 | ✅ |
| TC-017 | Repository-source installation preserves discovery | Integration | P0 | FR-003-AC-4 | ✅ |
| TC-018 | Unexpected, missing, or escaping package content fails audit | Unit | P0 | FR-003-AC-5 | ✅ |
| TC-019 | Module/plugin and local/repository install procedures are distinct | Static | P1 | FR-003-AC-6 | ✅ |
| TC-020 | Observed producer retains the complete immutable governing-version tuple | E2E | P0 | FR-004-AC-1, US-004-EX-2 | ✅ |
| TC-021 | Invocation failure remains unavailable | E2E | P0 | FR-004-AC-2, US-004-EX-1 | ✅ |
| TC-022 | Deferred producer remains not computed | Unit | P1 | FR-004-AC-3 | ✅ |
| TC-023 | Excluded producer remains not applicable | Unit | P1 | FR-004-AC-4 | ✅ |
| TC-024 | Malformed output or missing/mutable provenance fails validation | E2E | P0 | FR-004-AC-5 | ✅ |
| TC-025 | Persisted evidence delegates to Quoin | Integration | P0 | FR-004-AC-6 | ✅ |
| TC-026 | Interrupted ix-flow run resumes without repetition | Integration | P0 | FR-005-AC-1, US-003-EX-1 | ✅ |
| TC-027 | All terminal transitions remain human gated | Static | P0 | FR-005-AC-2 | ✅ |
| TC-028 | Explicit rejection records one attributed event and no success state | Integration | P0 | FR-005-AC-3, US-003-EX-2 | ✅ |
| TC-029 | Missing human choice leaves run non-terminal | Integration | P0 | FR-005-AC-4 | ✅ |
| TC-030 | Automatic terminal-gate override fails closed | Unit | P0 | FR-005-AC-5 | ✅ |
| TC-031 | Evaluation suite executes all five classes and seven variants on four hosts | E2E | P0 | FR-006-AC-1 | ✅ |
| TC-032 | Evaluation envelopes retain versions, transcript digests, effort, and outcomes | Property | P0 | FR-006-AC-2 | ✅ |
| TC-033 | Missing executable fails aggregate gate | E2E | P0 | FR-006-AC-3 | ✅ |
| TC-034 | Aggregate passes only for complete passing scenarios | Property | P0 | FR-006-AC-4 | ✅ |
| TC-035 | Four pilot workflow invocations still load | Integration | P0 | FR-007-AC-1 | ✅ |
| TC-036 | Pilot and canonical workflows are equivalent | Property | P0 | FR-007-AC-2 | ✅ |
| TC-037 | Canonical install docs precede compatibility path | Static | P1 | FR-007-AC-3 | ✅ |
| TC-038 | Cross-agent canonical parity reaches all thresholds | Integration | P0 | NFR-001 | ✅ |
| TC-039 | Evaluation produces zero unsupported outcomes | E2E | P0 | NFR-002 | ✅ |
| TC-040 | Package contract stability reaches all thresholds | Integration | P0 | NFR-003 | ✅ |
| TC-041 | Supported-agent set rejects missing, extra, or duplicate hosts | Property | P1 | FR-002-CON-1 | ✅ |
| TC-042 | Thin manifests reject behavioral sections and copied workflows | Property | P1 | FR-002-CON-2 | ✅ |
| TC-043 | Compatibility inventory is exactly the four promoted pilots | Property | P1 | FR-007-CON-1 | ✅ |
| TC-044 | Malformed or conflicting existing artifacts remain unchanged and require human resolution | E2E | P0 | FR-001-AC-6 | ✅ |
| TC-045 | Artifact publication is staged, Quire-validated, atomic, and confined to the selected root | Integration | P0 | FR-001-AC-7 | ✅ |
| TC-046 | Exactly one availability state exists for every considered producer | Property | P0 | FR-004-AC-7 | ✅ |
| TC-047 | Explicit acceptance records one attributed event and no prior acceptance state | Integration | P0 | FR-005-AC-6 | ✅ |
| TC-048 | A run-id binding mismatch is refused without changing either run | Property | P0 | FR-005-AC-7 | ✅ |
| TC-049 | Equivalent runs retain explicit acceptance and rejection on every host | E2E | P0 | FR-006-AC-5 | ✅ |
| TC-050 | Evaluation agents and post-run verification use the snapshotted full ix-flow runtime package | Unit | P0 | FR-006-AC-6 | ✅ |
| TC-051 | Release verification rejects an aggregate retained for a different repository revision | Unit | P0 | FR-006-AC-7 | ✅ |
| TC-052 | Complete cross-component semantic fixture | Integration | P0 | StR-002-VC-1, IT-005 | ✅ |
| TC-053 | Ownership/type-fit registry is complete | Static | P0 | US-005-AC-1 | ✅ |
| TC-054 | Bounded report contains required sections and no trust score | Unit | P0 | US-005-AC-2 | ✅ |
| TC-055 | Legacy mapping is explicit and read-only | Integration | P0 | US-005-AC-3 | ✅ |
| TC-056 | Every concept has exactly one authority and link direction | Property | P0 | FR-008-AC-1 | ✅ |
| TC-057 | Definition, execution, result, evidence, and report ids stay distinct | Property | P0 | FR-008-AC-2 | ✅ |
| TC-058 | Missing semantic references fail validation | Property | P0 | FR-008-AC-3 | ✅ |
| TC-059 | Package has no execution, scraping, persistence, or decision path | Static | P0 | FR-008-AC-4, NFR-004-AC-2 | ✅ |
| TC-060 | Complete producer tuple and definition version survive projection | Unit | P0 | FR-009-AC-1 | ✅ |
| TC-061 | Non-success states survive every language projection | Property | P0 | FR-009-AC-2 | ✅ |
| TC-062 | Unknown versions and missing provenance fail explicitly | Property | P0 | FR-009-AC-3 | ✅ |
| TC-063 | Every projected value cites its source record and field path | Property | P0 | FR-009-AC-4 | ✅ |
| TC-064 | PGM-01 source bytes are unchanged after mapping | Integration | P0 | FR-010-AC-1 | ✅ |
| TC-065 | Legacy identity and limitations are preserved | Unit | P0 | FR-010-AC-2 | ✅ |
| TC-066 | Ambiguous, unreadable, malformed, stale, and tampered stay non-successful | Property | P0 | FR-010-AC-3 | ✅ |
| TC-067 | JSON and Markdown projections preserve bounded report semantics | Unit | P0 | FR-010-AC-4 | ✅ |
| TC-068 | Schema/ownership audit finds no duplicate record family | Static | P0 | NFR-004-AC-1 | ✅ |
| TC-069 | Every retained corpus artifact matches its recorded digest, and every real legacy case matches the digest its source repository recorded | Integration | P0 | FR-011-AC-1, FR-011-CON-2 | ✅ |
| TC-070 | The corpus covers all eight required states and every constructed case records its edit and reason | Static | P0 | FR-011-AC-2 | ✅ |
| TC-071 | Each legacy case maps to its recorded outcome with required mappings preserved and a stated limitation | Property | P0 | FR-011-AC-3 | ✅ |
| TC-072 | No failed, unavailable, not-computed, malformed, or tampered case reads as clean or reports a passed check | Property | P0 | FR-011-AC-4 | ✅ |
| TC-073 | A real legacy record preserves revision, repository, producer identity, and environment, keeps inconclusive distinct, and names what it could not carry | Unit | P0 | FR-011-AC-5 | ✅ |
| TC-074 | The retained receipt validates against Quoin's packaged schema and binds the exact chain digests under pinned tools | Integration | P0 | FR-011-AC-6 | ✅ |
| TC-075 | Every producer case names a real producer, source path, and shared-model concept across languages | Unit | P0 | FR-011-AC-7 | ✅ |
| TC-076 | Reading and mapping the corpus changes no byte, no artifact is executable, and the reader reaches for no subprocess, socket, or write | Static | P0 | FR-011-AC-8, FR-011-CON-1, FR-011-CON-4 | ✅ |
| TC-077 | The committed corpus reproduces from its recorded sources, and states plainly when it is skipped | Integration | P0 | FR-011-AC-9 | ✅ |
| TC-078 | The corpus is a gitlink whose checked-out commit equals the recorded pin, and an uninitialized corpus fails rather than passing quietly | Integration | P0 | FR-011-AC-10, FR-011-CON-5 | ✅ |
| TC-079 | Every matrix component pins a released version and names its release; no pin is a branch, latest, or HEAD | Static | P0 | FR-012-AC-1 | ✅ |
| TC-080 | Compatible, incompatible, and unknown are distinct with reasons, and neither incompatible nor unknown satisfies the gate | Unit | P0 | FR-012-AC-2 | ✅ |
| TC-081 | The gate requires every pinned component; one unobserved component withholds it | Property | P0 | FR-012-AC-3 | ✅ |
| TC-082 | Matrix acceptance is pending, unattributed, and documented as a human act | Static | P0 | FR-012-AC-4, FR-012-CON-2 | ✅ |
| TC-083 | Every recorded artifact digest matches this tree over at least the ten schema assets | Integration | P0 | FR-012-AC-5 | ✅ |
| TC-084 | Upgrade order and per-component rollback notes exist, no rollback is irreversible, and publication changes no CI posture | Static | P0 | FR-012-AC-6 | ✅ |
| TC-085 | An unknown matrix version and an unknown component name are refused | Unit | P0 | FR-012-AC-7 | ✅ |
| TC-086 | The classifier reaches for no subprocess, socket, or write, and the observing program is a separate file | Static | P0 | FR-012-AC-8, FR-012-CON-1 | ✅ |

## Option Permutation Matrix

| Test Case | Install Source | Host | Expected Behavior |
|-----------|----------------|------|-------------------|
| TC-010 | local | Claude Code | Resolves canonical onboarding skill |
| TC-010 | local | Codex | Resolves canonical onboarding skill |
| TC-010 | local | opencode | Resolves canonical onboarding skill |
| TC-010 | local | GitHub Copilot | Resolves canonical onboarding skill |
| TC-010 | repository | Claude Code | Resolves canonical onboarding skill |
| TC-010 | repository | Codex | Resolves canonical onboarding skill |
| TC-010 | repository | opencode | Resolves canonical onboarding skill |
| TC-010 | repository | GitHub Copilot | Resolves canonical onboarding skill |

## Agent Evaluation Permutation Matrix

| Scenario Class | Required Variants | Hosts | Required Cells |
|----------------|-------------------|-------|----------------|
| Existing repository | applicable valid profile | 4 | 4 |
| No applicable profile | bounded no-profile decision | 4 | 4 |
| Producer failure | malformed output; unavailable executable | 4 | 8 |
| Interruption | interrupt then resume | 4 | 4 |
| Human terminal decision | explicit acceptance; explicit rejection | 4 | 8 |

The aggregate gate therefore requires 28 of 28 host-scenario cells with complete
evaluation envelopes.

## Constraint Boundary Tests

| Constraint | Boundary Type | Test Value | Test Case | Expected |
|------------|---------------|------------|-----------|----------|
| FR-002-CON-1 | Min | All 4 declared hosts present | TC-041 | Pass |
| FR-002-CON-1 | Max | All 4 declared hosts present once | TC-041 | Pass |
| FR-002-CON-1 | Below Min | One declared host missing | TC-041 | Fail |
| FR-002-CON-1 | Above Max | Extra or duplicate host manifest | TC-041 | Fail |
| FR-002-CON-2 | Min | Discovery metadata plus one canonical target | TC-042 | Pass |
| FR-002-CON-2 | Above Max | Embedded behavioral section or workflow copy | TC-042 | Fail |
| FR-007-CON-1 | Min | Four compatible pilot names | TC-043 | Pass |
| FR-007-CON-1 | Below Min | One promoted pilot name missing | TC-043 | Fail |
| FR-007-CON-1 | Above Max | Undeclared compatibility alias added | TC-043 | Fail |

## State Transition Coverage

| Workflow Condition | Transition or Failure | Test Case | Expected |
|--------------------|-----------------------|-----------|----------|
| Persisted non-terminal run | Interrupted to resumed | TC-026 | Completed phase retained |
| Decision-ready run without choice | Remains decision-ready | TC-029 | Non-terminal |
| Decision-ready run with rejection | Decision-ready to rejection terminal | TC-028 | Rejected exactly once |
| Terminal transition configured automatic | Gate override attempt | TC-030 | Fails closed |

## Edge Cases

| ID | Description | Related Req | Test Case | Risk if Untested |
|----|-------------|-------------|-----------|------------------|
| EC-001 | Valid applicable profile already exists | FR-001 | TC-004 | Duplicate or conflicting profile |
| EC-002 | Decision boundary justifies no profile | FR-001 | TC-005 | Generic assurance posture is invented |
| EC-003 | Canonical target is absent or escapes package | FR-002 | TC-013 | Host discovers stale or unsafe content |
| EC-004 | Package has an unexpected or missing member | FR-003 | TC-018 | Rights or install contract silently changes |
| EC-005 | Producer output is malformed | FR-004 | TC-024 | Invalid output counted as evidence |
| EC-006 | Selected producer executable is missing | FR-004 | TC-021 | Tooling failure becomes missing evidence |
| EC-007 | Agent stops after a persisted phase | FR-005 | TC-026 | Repeated work or lost decision context |
| EC-008 | Required evaluation executable is absent | FR-006 | TC-033 | Incomplete evaluation appears passing |
| EC-009 | Existing applicable artifacts conflict or are malformed | FR-001 | TC-044 | Onboarding silently selects or overwrites an artifact |
| EC-010 | Artifact target or manifest reference escapes its selected root | FR-001, FR-003 | TC-018, TC-045 | Installation or onboarding writes/loads unowned content |
| EC-011 | Run id is reused for a different repository or workflow | FR-005 | TC-048 | One run contaminates another decision boundary |
| EC-012 | Valid output lacks an immutable governing version | FR-004 | TC-024 | Unreproducible evidence is admitted as observed |

## Integration Test Matrix

### Cross-Project Integrations

| Integration ID | Purpose | Target Project | Type | Test Cases | Status |
|----------------|---------|----------------|------|------------|--------|
| INT-001 | Validate justified assurance artifacts | quire-rs | service | TC-006, TC-017, TC-045 | ✅ |
| INT-002 | Persist and render evidence without redefining policy | quoin | service | TC-025, TC-046 | ✅ |
| INT-003 | Load workflows, persist runs, resume, and gate decisions | ix-flow | service | TC-026, TC-028, TC-029, TC-030, TC-035, TC-047, TC-048 | ✅ |
| INT-004 | Discover canonical onboarding through supported hosts | agent discovery adapters | service | TC-010, TC-011, TC-038 | ✅ |
| INT-005 | Validate shared semantic references and historical compatibility | Quire, Quoin, ix-flow, native producer fixtures | library | TC-052..TC-068 | ✅ |

### Integration Test Details

| Test Case | Integration | Scenario | Input | Expected | Priority |
|-----------|-------------|----------|-------|----------|----------|
| TC-006 | INT-001 | Justified artifact validation | Installed skeleton and fictional bounded inputs | Quire accepts artifact before recommendation | P0 |
| TC-017 | INT-001 | Repository-source module validation | Installed archive and fictional document | Existing module root validates | P0 |
| TC-025 | INT-002 | Evidence delegation | Valid observed producer result | Quoin owns retained record and report | P0 |
| TC-026 | INT-003 | Interrupted workflow resume | Persisted non-terminal run | Next action without repeated phase | P0 |
| TC-028 | INT-003 | Human rejection | Named owner rejection action | Rejection terminal only | P0 |
| TC-029 | INT-003 | Missing terminal choice | Decision-ready run | Remains non-terminal | P0 |
| TC-030 | INT-003 | Invalid gate override | Automatic terminal gate request | Fails closed | P0 |
| TC-035 | INT-003 | Compatibility discovery | Four pilot names | Every pilot loads | P0 |
| TC-045 | INT-001 | Validated publication | Staged valid/invalid artifacts and escaping target | Only valid in-root artifact is atomically published | P0 |
| TC-046 | INT-002 | Evidence-state exclusivity | Generated producer-state combinations | Exactly one catalogued state per producer | P0 |
| TC-047 | INT-003 | Human acceptance | Named owner acceptance action | One attributed acceptance terminal event | P0 |
| TC-048 | INT-003 | Run identity mismatch | Existing id with changed binding | Refused with both runs unchanged | P0 |
| TC-010 | INT-004 | Host discovery permutations | Two sources by four hosts | Same canonical skill | P0 |
| TC-011 | INT-004 | Workflow discovery | Canonical skill | Exactly four workflow definitions | P0 |

## Engineering Assurance #5 Coverage

| Requirement | Criterion | Test Case | Status |
| --- | --- | --- | --- |
| StR-002 | StR-002-VC-1 | TC-052 | ✅ Passing |
| US-005 | US-005-AC-1 | TC-053 | ✅ Passing |
| US-005 | US-005-AC-2 | TC-054 | ✅ Passing |
| US-005 | US-005-AC-3 | TC-055 | ✅ Passing |
| FR-008 | FR-008-AC-1 | TC-056 | ✅ Passing |
| FR-008 | FR-008-AC-2 | TC-057 | ✅ Passing |
| FR-008 | FR-008-AC-3 | TC-058 | ✅ Passing |
| FR-008 | FR-008-AC-4 | TC-059 | ✅ Passing |
| FR-009 | FR-009-AC-1 | TC-060 | ✅ Passing |
| FR-009 | FR-009-AC-2 | TC-061 | ✅ Passing |
| FR-009 | FR-009-AC-3 | TC-062 | ✅ Passing |
| FR-009 | FR-009-AC-4 | TC-063 | ✅ Passing |
| FR-010 | FR-010-AC-1 | TC-064 | ✅ Passing |
| FR-010 | FR-010-AC-2 | TC-065 | ✅ Passing |
| FR-010 | FR-010-AC-3 | TC-066 | ✅ Passing |
| FR-010 | FR-010-AC-4 | TC-067 | ✅ Passing |
| FR-011 | FR-011-AC-1 | TC-069 | ✅ Passing |
| FR-011 | FR-011-AC-2 | TC-070 | ✅ Passing |
| FR-011 | FR-011-AC-3 | TC-071 | ✅ Passing |
| FR-011 | FR-011-AC-4 | TC-072 | ✅ Passing |
| FR-011 | FR-011-AC-5 | TC-073 | ✅ Passing |
| FR-011 | FR-011-AC-6 | TC-074 | ✅ Passing |
| FR-011 | FR-011-AC-7 | TC-075 | ✅ Passing |
| FR-011 | FR-011-AC-8 | TC-076 | ✅ Passing |
| FR-011 | FR-011-AC-9 | TC-077 | ✅ Passing |
| FR-011 | FR-011-AC-10 | TC-078 | ✅ Passing |
| FR-012 | FR-012-AC-1 | TC-079 | ✅ Passing |
| FR-012 | FR-012-AC-2 | TC-080 | ✅ Passing |
| FR-012 | FR-012-AC-3 | TC-081 | ✅ Passing |
| FR-012 | FR-012-AC-4 | TC-082 | ✅ Passing |
| FR-012 | FR-012-AC-5 | TC-083 | ✅ Passing |
| FR-012 | FR-012-AC-6 | TC-084 | ✅ Passing |
| FR-012 | FR-012-AC-7 | TC-085 | ✅ Passing |
| FR-012 | FR-012-AC-8 | TC-086 | ✅ Passing |
| NFR-004 | NFR-004-AC-1 | TC-068 | ✅ Passing |
| NFR-004 | NFR-004-AC-2 | TC-059 | ✅ Passing |

## Coverage Gaps

The completed onboarding scope has no open gap: Quire reconciles TC-001..TC-051
to real tracking-tagged symbols, and the retained 28-cell aggregate records the
selected host commands, models, governing versions, transcripts, and outcomes.
TC-052..TC-068 now have tracking-tagged implementations in
`tests/test_verification_semantics.py`, and TC-069..TC-086 in
`tests/test_compatibility_corpus.py` and `tests/test_compatibility_matrix.py`,
which enforce the accepted compatibility corpus that FR-010 previously deferred
and the pinned release matrix that gates the migrations; SR-022 and SR-023 retain the completed
code-review and gap-analysis closure gates.

## Test Execution Summary

| Category | Total | Passed | Failed | Blocked | Coverage |
|----------|-------|--------|--------|---------|----------|
| Unit | 12 | 12 | 0 | 0 | 100% |
| Integration | 20 | 20 | 0 | 0 | 100% |
| E2E | 13 | 13 | 0 | 0 | 100% |
| Property | 15 | 15 | 0 | 0 | 100% |
| Static | 8 | 8 | 0 | 0 | 100% |

The table combines the completed onboarding baseline with the 17 implemented
Engineering Assurance #5 rows.
