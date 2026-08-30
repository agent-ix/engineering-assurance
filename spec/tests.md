---
id: TM-001
title: engineering-assurance Onboarding Test Matrix
type: TestMatrix
---

# Engineering-Assurance Onboarding Test Matrix

## Overview

This matrix defines pending verification for the canonical assurance-onboarding
bundle. Test cases remain `🚧` until implementation supplies executable tracking
tags and passing evidence. The matrix covers every stakeholder validation
criterion, user-story example, functional acceptance criterion, non-functional
requirement, and declared constraint in this specification.

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

## Requirements Traceability

### Stakeholder Requirement Coverage

| Stakeholder Req | Validation Criterion | Test Cases | Coverage Status |
|-----------------|----------------------|------------|-----------------|
| StR-001 | StR-001-VC-1 | TC-001 | 🚧 Pending |
| StR-001 | StR-001-VC-2 | TC-002 | 🚧 Pending |
| StR-001 | StR-001-VC-3 | TC-003 | 🚧 Pending |

### User Story Coverage

| User Story | Illustrative Example | Test Cases | Coverage Status |
|------------|----------------------|------------|-----------------|
| US-001 | US-001-EX-1 | TC-004 | 🚧 Pending |
| US-001 | US-001-EX-2 | TC-005 | 🚧 Pending |
| US-002 | US-002-EX-1 | TC-010 | 🚧 Pending |
| US-002 | US-002-EX-2 | TC-011 | 🚧 Pending |
| US-003 | US-003-EX-1 | TC-026 | 🚧 Pending |
| US-003 | US-003-EX-2 | TC-028 | 🚧 Pending |
| US-004 | US-004-EX-1 | TC-021 | 🚧 Pending |
| US-004 | US-004-EX-2 | TC-020 | 🚧 Pending |

### Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
|----------------|---------------------|------------|-----------------|
| FR-001 | FR-001-AC-1 | TC-004 | 🚧 Pending |
| FR-001 | FR-001-AC-2 | TC-005 | 🚧 Pending |
| FR-001 | FR-001-AC-3 | TC-006 | 🚧 Pending |
| FR-001 | FR-001-AC-4 | TC-007 | 🚧 Pending |
| FR-001 | FR-001-AC-5 | TC-008 | 🚧 Pending |
| FR-002 | FR-002-AC-1 | TC-009 | 🚧 Pending |
| FR-002 | FR-002-AC-2 | TC-010 | 🚧 Pending |
| FR-002 | FR-002-AC-3 | TC-011 | 🚧 Pending |
| FR-002 | FR-002-AC-4 | TC-012 | 🚧 Pending |
| FR-002 | FR-002-AC-5 | TC-013 | 🚧 Pending |
| FR-003 | FR-003-AC-1 | TC-014 | 🚧 Pending |
| FR-003 | FR-003-AC-2 | TC-015 | 🚧 Pending |
| FR-003 | FR-003-AC-3 | TC-016 | 🚧 Pending |
| FR-003 | FR-003-AC-4 | TC-017 | 🚧 Pending |
| FR-003 | FR-003-AC-5 | TC-018 | 🚧 Pending |
| FR-003 | FR-003-AC-6 | TC-019 | 🚧 Pending |
| FR-004 | FR-004-AC-1 | TC-020 | 🚧 Pending |
| FR-004 | FR-004-AC-2 | TC-021 | 🚧 Pending |
| FR-004 | FR-004-AC-3 | TC-022 | 🚧 Pending |
| FR-004 | FR-004-AC-4 | TC-023 | 🚧 Pending |
| FR-004 | FR-004-AC-5 | TC-024 | 🚧 Pending |
| FR-004 | FR-004-AC-6 | TC-025 | 🚧 Pending |
| FR-005 | FR-005-AC-1 | TC-026 | 🚧 Pending |
| FR-005 | FR-005-AC-2 | TC-027 | 🚧 Pending |
| FR-005 | FR-005-AC-3 | TC-028 | 🚧 Pending |
| FR-005 | FR-005-AC-4 | TC-029 | 🚧 Pending |
| FR-005 | FR-005-AC-5 | TC-030 | 🚧 Pending |
| FR-006 | FR-006-AC-1 | TC-031 | 🚧 Pending |
| FR-006 | FR-006-AC-2 | TC-032 | 🚧 Pending |
| FR-006 | FR-006-AC-3 | TC-033 | 🚧 Pending |
| FR-006 | FR-006-AC-4 | TC-034 | 🚧 Pending |
| FR-007 | FR-007-AC-1 | TC-035 | 🚧 Pending |
| FR-007 | FR-007-AC-2 | TC-036 | 🚧 Pending |
| FR-007 | FR-007-AC-3 | TC-037 | 🚧 Pending |

### Non-Functional Requirement Coverage

| Non-Functional Req | Verification Method | Evidence/Test Cases | Status |
|--------------------|---------------------|---------------------|--------|
| NFR-001 | Install, discovery, digest comparison, and static scan | TC-038 | 🚧 Pending |
| NFR-002 | Agent evaluation against fixture-authorized outcomes | TC-039 | 🚧 Pending |
| NFR-003 | Wheel/npm member audit plus compatibility invocation | TC-040 | 🚧 Pending |

### Constraint Coverage

| Constraint | Test Cases | Coverage Status |
|------------|------------|-----------------|
| FR-002-CON-1 | TC-041 | 🚧 Pending |
| FR-002-CON-2 | TC-042 | 🚧 Pending |
| FR-007-CON-1 | TC-043 | 🚧 Pending |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
|---------|-------|------|----------|-----------|--------|
| TC-001 | Inventory precedes every onboarding proposal | E2E | P0 | StR-001-VC-1 | 🚧 |
| TC-002 | Unjustified profile is not scaffolded | E2E | P0 | StR-001-VC-2 | 🚧 |
| TC-003 | Named human owns every terminal outcome | Integration | P0 | StR-001-VC-3 | 🚧 |
| TC-004 | Existing valid profile is inventoried and reused | E2E | P0 | FR-001-AC-1, US-001-EX-1 | 🚧 |
| TC-005 | No-profile repository creates no generic profile | E2E | P0 | FR-001-AC-2, US-001-EX-2 | 🚧 |
| TC-006 | Justified artifact uses installed skeleton and Quire | Integration | P0 | FR-001-AC-3 | 🚧 |
| TC-007 | Incomplete boundary requests input without writing | E2E | P0 | FR-001-AC-4 | 🚧 |
| TC-008 | Inventory separates all required collections | Unit | P1 | FR-001-AC-5 | 🚧 |
| TC-009 | Exactly one canonical onboarding skill exists | Static | P0 | FR-002-AC-1 | 🚧 |
| TC-010 | Four host surfaces resolve one canonical skill | Integration | P0 | FR-002-AC-2, US-002-EX-1 | 🚧 |
| TC-011 | Canonical skill exposes exactly four workflows | Integration | P0 | FR-002-AC-3, US-002-EX-2 | 🚧 |
| TC-012 | Behavioral text in a host manifest is rejected | Static | P1 | FR-002-AC-4 | 🚧 |
| TC-013 | Missing or escaping canonical target is rejected | Unit | P0 | FR-002-AC-5 | 🚧 |
| TC-014 | Wheel includes explicit module and onboarding members | Integration | P0 | FR-003-AC-1 | 🚧 |
| TC-015 | Npm archive includes explicit module and onboarding members | Integration | P0 | FR-003-AC-2 | 🚧 |
| TC-016 | Local-source installation preserves discovery | Integration | P0 | FR-003-AC-3 | 🚧 |
| TC-017 | Repository-source installation preserves discovery | Integration | P0 | FR-003-AC-4 | 🚧 |
| TC-018 | Unexpected or missing package member fails audit | Unit | P0 | FR-003-AC-5 | 🚧 |
| TC-019 | Install source procedures are distinct | Static | P1 | FR-003-AC-6 | 🚧 |
| TC-020 | Observed producer retains exact provenance | E2E | P0 | FR-004-AC-1, US-004-EX-2 | 🚧 |
| TC-021 | Invocation failure remains unavailable | E2E | P0 | FR-004-AC-2, US-004-EX-1 | 🚧 |
| TC-022 | Deferred producer remains not computed | Unit | P1 | FR-004-AC-3 | 🚧 |
| TC-023 | Excluded producer remains not applicable | Unit | P1 | FR-004-AC-4 | 🚧 |
| TC-024 | Malformed producer output fails validation | E2E | P0 | FR-004-AC-5 | 🚧 |
| TC-025 | Persisted evidence delegates to Quoin | Integration | P0 | FR-004-AC-6 | 🚧 |
| TC-026 | Interrupted ix-flow run resumes without repetition | Integration | P0 | FR-005-AC-1, US-003-EX-1 | 🚧 |
| TC-027 | All terminal transitions remain human gated | Static | P0 | FR-005-AC-2 | 🚧 |
| TC-028 | Explicit rejection records no success state | Integration | P0 | FR-005-AC-3, US-003-EX-2 | 🚧 |
| TC-029 | Missing human choice leaves run non-terminal | Integration | P0 | FR-005-AC-4 | 🚧 |
| TC-030 | Automatic terminal-gate override fails closed | Unit | P0 | FR-005-AC-5 | 🚧 |
| TC-031 | Evaluation suite executes five required classes | E2E | P0 | FR-006-AC-1 | 🚧 |
| TC-032 | Evaluation observations are complete | Property | P0 | FR-006-AC-2 | 🚧 |
| TC-033 | Missing executable fails aggregate gate | E2E | P0 | FR-006-AC-3 | 🚧 |
| TC-034 | Aggregate passes only for complete passing scenarios | Property | P0 | FR-006-AC-4 | 🚧 |
| TC-035 | Four pilot workflow invocations still load | Integration | P0 | FR-007-AC-1 | 🚧 |
| TC-036 | Pilot and canonical workflows are equivalent | Property | P0 | FR-007-AC-2 | 🚧 |
| TC-037 | Canonical install docs precede compatibility path | Static | P1 | FR-007-AC-3 | 🚧 |
| TC-038 | Cross-agent canonical parity reaches all thresholds | Integration | P0 | NFR-001 | 🚧 |
| TC-039 | Evaluation produces zero unsupported outcomes | E2E | P0 | NFR-002 | 🚧 |
| TC-040 | Package contract stability reaches all thresholds | Integration | P0 | NFR-003 | 🚧 |
| TC-041 | Supported-agent set rejects missing, extra, or duplicate hosts | Property | P1 | FR-002-CON-1 | 🚧 |
| TC-042 | Thin manifests reject behavioral sections and copied workflows | Property | P1 | FR-002-CON-2 | 🚧 |
| TC-043 | Compatibility inventory is exactly the four promoted pilots | Property | P1 | FR-007-CON-1 | 🚧 |

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

## Integration Test Matrix

### Cross-Project Integrations

| Integration ID | Purpose | Target Project | Type | Test Cases | Status |
|----------------|---------|----------------|------|------------|--------|
| INT-001 | Validate justified assurance artifacts | quire-rs | service | TC-006, TC-017 | 🚧 |
| INT-002 | Persist and render evidence without redefining policy | quoin | service | TC-025 | 🚧 |
| INT-003 | Load workflows, persist runs, resume, and gate decisions | ix-flow | service | TC-026, TC-028, TC-029, TC-030, TC-035 | 🚧 |
| INT-004 | Discover canonical onboarding through supported hosts | agent discovery adapters | service | TC-010, TC-011, TC-038 | 🚧 |

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
| TC-010 | INT-004 | Host discovery permutations | Two sources by four hosts | Same canonical skill | P0 |
| TC-011 | INT-004 | Workflow discovery | Canonical skill | Exactly four workflow definitions | P0 |

## Coverage Gaps

| Gap ID | Description | Risk Level | Mitigation |
|--------|-------------|------------|------------|
| GAP-001 | All mapped tests are pending implementation. | High | Implement tracking-tagged tests and retain `🚧` status until each passes. |
| GAP-002 | Exact supported-agent harness commands and versions are not selected by this requirements stage. | Medium | Resolve them in the implementation plan and record exact versions in evaluation evidence. |

## Test Execution Summary

| Category | Total | Passed | Failed | Blocked | Coverage |
|----------|-------|--------|--------|---------|----------|
| Unit | 6 | 0 | 0 | 6 | 0% |
| Integration | 15 | 0 | 0 | 15 | 0% |
| E2E | 11 | 0 | 0 | 11 | 0% |
| Property | 6 | 0 | 0 | 6 | 0% |
| Static | 5 | 0 | 0 | 5 | 0% |
