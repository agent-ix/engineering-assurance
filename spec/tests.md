---
id: TM-001
title: engineering-assurance Onboarding Test Matrix
type: TestMatrix
---

# Engineering-Assurance Onboarding Test Matrix

## Overview

This matrix records completed verification for the canonical assurance-onboarding
and verification-semantics baseline and planned verification for the proposed
Rust port. TC-001..TC-095 have passing repository or retained real-agent
evidence. TC-096..TC-121 are staged: completed slices are identified explicitly,
while aggregate port cases remain pending until every named capability is
implemented and reviewed.

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
8. Every requirement-verifying Rust test uses the canonical ix-trace-rs import
   and bare trace attribute form that Quire reconciles.
9. Requirement Acceptance Criteria use broad verification-method categories
   such as `Test`, while this matrix's `Type` column names the concrete test
   level or evidence form (`Unit`, `Integration`, `Property`, `Static`, and so
   on). An AC's `Test` may therefore map to an `Integration` TC; the columns are
   related at different levels of specificity rather than one shared vocabulary.

## Requirements Traceability

### Stakeholder Requirement Coverage

| Stakeholder Req | Trace to US/FR | Test/Validation | Coverage Status |
|-----------------|----------------|-----------------|-----------------|
| StR-001 | FR-001, FR-004, FR-005 | StR-001-VC-1..VC-3 → TC-001..TC-003 | ✅ Passing |
| StR-003 | FR-014..FR-018, NFR-005 | StR-003-VC-1..VC-4 → TC-096, TC-097, TC-100, TC-115 | 🚧 Pending implementation |

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
| FR-014 | FR-014-AC-1 | TC-096 | ✅ Rust package foundation backed |
| FR-014 | FR-014-AC-2 | TC-098 | 🚧 Compatibility-command slice backed; aggregate pending |
| FR-014 | FR-014-AC-3 | TC-099 | 🚧 Compatibility-command slice backed; aggregate pending |
| FR-014 | FR-014-AC-4 | TC-101 | ✅ Parsed library-module audit, lexical-alias/comment adverse cases, and mutations passing |
| FR-015 | FR-015-AC-1 | TC-100 | 🚧 Compatibility, evidence, semantic-validation, report/PGM projection, and fixture-generation slices backed; aggregate pending |
| FR-015 | FR-015-AC-2 | TC-102 | 🚧 Evidence-availability and semantic-reference state vocabularies backed; aggregate pending |
| FR-015 | FR-015-AC-3 | TC-103 | 🚧 Evidence, semantic, and PGM refusal families backed; aggregate pending |
| FR-015 | FR-015-AC-4 | TC-104 | 🚧 Semantic ownership and inert-fixture static slice backed; aggregate executable-language audit pending |
| FR-016 | FR-016-AC-1 | TC-105 | ✅ Rust onboarding core, CLI, retained-reference parity, and confined publication backed |
| FR-016 | FR-016-AC-2 | TC-106 | ✅ Rust evaluator and CLI parity backed |
| FR-016 | FR-016-AC-3 | TC-107 | ✅ Rust lifecycle host and adverse cases pass against accepted ix-flow 0.2.3 |
| FR-016 | FR-016-AC-4 | TC-108 | 🚧 Pending implementation |
| FR-017 | FR-017-AC-1 | TC-109 | 🚧 Pending implementation |
| FR-017 | FR-017-AC-2 | TC-110 | 🚧 Pending implementation |
| FR-017 | FR-017-AC-3 | TC-111 | 🚧 Bounded Rust package/archive/install candidate and preceding qualification slices backed; full slice review and promotion pending |
| FR-017 | FR-017-AC-4 | TC-112 | 🚧 npm lifecycle, content-rights, and package-audit gates are declarative Rust dispatch; remaining host-configuration census pending |
| FR-017 | FR-017-AC-6 | TC-120 | ✅ Typed Rust policy, retained-Python differential, adverse boundaries, ordering, resource ceilings, and mutation probes passing |
| FR-017 | FR-017-AC-7 | TC-121 | ✅ Rust manifest classifier and retained-validator parity backed |
| FR-017 | FR-017-AC-1 | TC-129 | 🚧 Rust report/transcript adapter planned; live TC-109 remains pending |
| FR-018 | FR-018-AC-1 | TC-097 | 🚧 Pending implementation |
| FR-018 | FR-018-AC-2 | TC-113 | 🚧 Content-rights and package-audit same-revision cutovers plus final four-file deletion candidate backed; aggregate removal population pending |
| FR-018 | FR-018-AC-3 | TC-114 | 🚧 Content-rights and package-audit dispatch rollback/reapplication backed; aggregate rollback population pending |
| FR-018 | FR-018-AC-4 | TC-115 | 🚧 Pending implementation |

### Non-Functional Requirement Coverage

| Non-Functional Req | Verification Method | Evidence/Test Cases | Status |
|--------------------|---------------------|---------------------|--------|
| NFR-001 | Install, discovery, digest comparison, and static scan | TC-038 | ✅ Passing |
| NFR-002 | Agent evaluation against fixture-authorized outcomes | TC-039 | ✅ Passing |
| NFR-003 | Wheel/npm member audit plus compatibility invocation | TC-040 | ✅ Passing |
| NFR-005 | Exact toolchain build, unsafe audit, executable-path audit, and Quire reconciliation | TC-115..TC-118 | 🚧 Pending implementation |

### Constraint Coverage

| Constraint | Test Cases | Status |
|------------|------------|-----------------|
| FR-002-CON-1 | TC-041 | ✅ Passing |
| FR-002-CON-2 | TC-042 | ✅ Passing |
| FR-007-CON-1 | TC-043 | ✅ Passing |
| FR-012-CON-2 | TC-082 | ✅ Passing |
| FR-012-CON-4 | TC-082 | ✅ Passing |
| FR-012-CON-5 | TC-082 | ✅ Passing |
| FR-014-CON-1 | TC-101 | ✅ Parsed library-module containment audit passing |
| FR-014-CON-2 | TC-101 | ✅ Parsed library-module containment audit passing |
| FR-014-CON-3 | TC-096 | ✅ Rust package foundation backed |
| FR-015-CON-1 | TC-103 | 🚧 Read-only evidence and semantic/PGM slices backed; aggregate pending |
| FR-015-CON-2 | TC-104 | 🚧 Inert generated-fixture slice backed; aggregate pending |
| FR-015-CON-3 | TC-104 | 🚧 Semantic-contract slice backed; aggregate pending |
| FR-016-CON-1 | TC-107 | ✅ Delegated ix-flow state/chain behavior passes against accepted ix-flow 0.2.3 |
| FR-016-CON-2 | TC-115 | 🚧 Pending owner disposition |
| FR-016-CON-3 | TC-106 | ✅ Deterministic I/O-free Rust evaluator backed |
| FR-016-CON-4 | TC-105 | ✅ Pure library and binary-only host adapter backed |
| FR-016-CON-5 | TC-107 | ✅ Supported ix-flow CLI-only lifecycle path passes against accepted ix-flow 0.2.3 |
| FR-016-CON-6 | TC-107 | ✅ State-file isolation and delegated chain verification pass against accepted ix-flow 0.2.3 |
| FR-016-CON-7 | TC-107 | ✅ Direct argv and display-only next-action guards pass against accepted ix-flow 0.2.3 |
| FR-017-CON-1 | TC-109 | 🚧 Pending implementation |
| FR-017-CON-2 | TC-112 | 🚧 npm lifecycle hooks backed; remaining host-configuration census pending |
| FR-018-CON-1 | TC-113 | 🚧 Pending implementation |
| FR-018-CON-2 | TC-114 | 🚧 Pending implementation |
| FR-018-CON-3 | TC-115 | 🚧 Pending implementation |

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
| TC-082 | Matrix acceptance is pending-and-unattributed or accepted-with-name-and-date, never half-recorded, and documented as a human act | Static | P0 | FR-012-AC-4, FR-012-CON-2, FR-012-CON-4, FR-012-CON-5 | ✅ |
| TC-083 | Every recorded artifact digest matches this tree over at least the ten schema assets | Integration | P0 | FR-012-AC-5 | ✅ |
| TC-084 | Upgrade order and per-component rollback notes exist, no rollback is irreversible, and publication changes no CI posture | Static | P0 | FR-012-AC-6 | ✅ |
| TC-085 | An unknown matrix version and an unknown component name are refused | Unit | P0 | FR-012-AC-7 | ✅ |
| TC-086 | The classifier reaches for no subprocess, socket, or write, and the observing program is a separate file | Static | P0 | FR-012-AC-8, FR-012-CON-1 | ✅ |
| TC-087 | Every family in the decision table carries exactly one of keep, delete, or replace | Static | P0 | FR-013-AC-1 | ✅ |
| TC-088 | The decision table accounts for every recurring script family present in the eight repositories, and states when the sources cannot be read | Integration | P0 | FR-013-AC-2 | ✅ |
| TC-089 | Repository-local generic evidence schemas and stdout-derived verdicts are forbidden by name, and a domain-output schema is permitted | Static | P0 | FR-013-AC-3 | ✅ |
| TC-090 | Domain output validation, evidence intake, audit, and human decision each name a distinct owner | Static | P0 | FR-013-AC-4 | ✅ |
| TC-091 | Rollback is defined per failure mode, legacy history is never rewritten, and deletion is last | Static | P0 | FR-013-AC-5 | ✅ |
| TC-092 | The review checklist covers inventory, both prohibitions, byte-identical legacy evidence, every non-success state, and manual dispatch | Static | P0 | FR-013-AC-6 | ✅ |
| TC-093 | All eight repositories appear exactly once in the Agent A/B/C allocation | Static | P0 | FR-013-AC-7 | ✅ |
| TC-094 | The contract waits on matrix acceptance, changes no trigger, and makes no qualification claim | Unit | P0 | FR-013-AC-8, FR-013-CON-1, FR-013-CON-2, FR-013-CON-3 | ✅ |
| TC-095 | A fully pinned toolchain does not open an unaccepted gate; any state but `accepted`, and any half-record missing a name or date, withholds | Unit | P0 | FR-012-AC-9 | ✅ |
| TC-096 | Existing repository builds the named Rust library and CLI | Compile | P0 | StR-003-VC-1, FR-014-AC-1, FR-014-CON-3 | ✅ Rust package foundation backed |
| TC-097 | Every first-party executable path in this repository has one current state and final disposition | Static | P0 | StR-003-VC-2, FR-018-AC-1 | 🚧 pending implementation |
| TC-098 | Machine CLI output obeys the versioned stdout/stderr contract | Property | P0 | FR-014-AC-2 | 🚧 compatibility-command slice backed; aggregate pending |
| TC-099 | Invalid protocol, input, root, host, and host response fail before side effects | Property | P0 | FR-014-AC-3 | 🚧 compatibility-command slice backed; aggregate pending |
| TC-100 | For each ADR-002 compatibility/semantic/projection/fixture capability row, Rust agrees byte-for-byte with the accepted identity and report reference over the named corpus and focused adverse cases, including generated arbitrary-precision integer boundaries | Integration | P0 | StR-003-VC-3, FR-015-AC-1 | 🚧 compatibility, evidence, semantic validation, report/PGM projections, and fixture generation backed; aggregate pending |
| TC-101 | Parsed Rust syntax for every reusable-library module rejects filesystem, environment, child-program, network, persistence, and arbitrary-stdout capability paths, including lexical aliases declared in that source, without treating comments or literals as code or claiming cross-document compiler resolution | Static | P0 | FR-014-AC-4, FR-014-CON-1, FR-014-CON-2 | ✅ AST audit, closed findings, adverse scope/alias cases, and mutation evidence passing |
| TC-102 | Every successful and non-successful semantic state remains distinct; typed state spellings and exact-one untyped-label validation remain callable without string parsing | Integration | P0 | FR-015-AC-2 | 🚧 evidence-availability and semantic-reference state vocabularies backed; aggregate pending |
| TC-103 | Invalid semantic inputs, including non-finite/retained-domain-overflow numbers, invalid/mutable version tokens, and structured PGM-01 identities, fail or classify read-only without an identity digest or source-byte change; independent fixture classes retain leading/trailing and interior ASCII space plus lowercase/uppercase immutable `x` metadata boundaries; every exercised semantic and PGM-01 refusal exposes a typed reason without diagnostic-prose matching | Integration | P0 | FR-015-AC-3, FR-015-CON-1 | 🚧 evidence, semantic, and PGM refusals backed; aggregate pending |
| TC-104 | Contract ownership and inert foreign-language fixtures remain bounded | Static | P0 | FR-015-AC-4, FR-015-CON-2, FR-015-CON-3 | 🚧 semantic ownership and inert-fixture slice backed; aggregate audit pending |
| TC-105 | Rust onboarding matches the retained complete sorted inventory, status, recommendation, and artifact path for existing, absent, conflicting, malformed, unavailable, unjustified, incomplete-boundary, and valid-authoring cases; duplicate-key and merge-key artifact identities remain malformed; newly authored artifacts retain equivalent parsed frontmatter and an identical Markdown body and pass Quire, while malformed requests, unsupported types, absolute or parent-traversing targets, symlink escapes, existing destinations, and invalid staged artifacts publish nothing | Property | P0 | FR-016-AC-1, FR-016-CON-4 | ✅ Rust core/CLI parity, fail-closed YAML identity, real Quire validation, no-replace publication, and boundary refusals passing |
| TC-106 | Rust and the retained invariant provider return the same ordered typed outcomes for all eleven canonical invariants and valid boundary fixtures at one explicit evaluation instant; unknown invariant names and malformed Rust requests fail before an outcome | Integration | P0 | FR-016-AC-2, FR-016-CON-3 | ✅ Rust evaluator, retained-reference parity, fail-closed probes, and CLI boundary passing |
| TC-107 | Against the exact accepted ix-flow pin, Rust start/resume and explicit decision coordination preserves ix-flow state ownership, recovers only pristine interrupted initialization, remains idempotent across gate interruption windows, requires ix-flow to verify an intact event chain, refuses incompatible hosts and binding/transition/decision conflicts before mutation, reports ambiguous post-mutation host failures as indeterminate for status reconciliation, bounds malformed/oversized/timed-out responses, and never supplies an automatic gate override or executes next-action text | Integration | P0 | FR-016-AC-3, FR-016-CON-1, FR-016-CON-5, FR-016-CON-6, FR-016-CON-7 | ✅ 8 local lifecycle cases pass against accepted ix-flow 0.2.3 |
| TC-108 | Canonical and pilot workflows pass through the Rust invariant provider before removal | Integration | P0 | FR-016-AC-4 | 🚧 pending host interface |
| TC-109 | Rust evaluation completes the 28-cell matrix without inferred decisions | E2E | P0 | FR-017-AC-1, FR-017-CON-1 | 🚧 pending host interface |
| TC-110 | The pure typed Rust evaluator preserves the exact 28-cell contract and deterministically withholds aggregation for missing, duplicate, malformed, unsupported, unavailable, failed, stale-revision, changed-governing-identity, changed-workflow, unsupported-addition, invalid-transcript-reference, invalid-count, outcome-mismatch, and terminal-pair cases; input permutation cannot change the result, oversized input refuses before decoding, and the reusable boundary performs no I/O | Property | P0 | FR-017-AC-2, FR-017-CON-1, FR-017-CON-3 | ✅ typed Rust aggregation, retained-Python differential cases, adverse cases, portable path/refusal rules, resource ceiling, and mutation probes passing |
| TC-111 | Each package, rights, manifest, integration, and publication-refusal capability has independent positive and negative subcases matching the retained gate; npm stage/cleanup covers missing, linked, special, non-portable, oversized, excessive, pre-existing, changed, exact-copy, rollback, idempotent-absence, direct-JSON, and hook-empty-stdout states; the content-rights tree adapter covers exact Git selection, tracked/untracked/ignored entries, gitlinks, links, special files, protected tokens, repository-root identity, deterministic findings, and every exact/over resource boundary; the package-audit adapter covers pre-build linked/special/unsafe/over-limit source refusal including wheel build configuration, direct bounded builder/installer processes and descendant groups, fail-closed hosts without process-group containment, invocation-owned package-manager cache/temporary paths, bounded top-level output correspondence, preflight-absent and exact post-process npm staging cleanup on success and failure, exact archive selection, validated regular/zero-payload-directory kinds, raw tar extension-header refusal before preprocessing, safe unique member names, exact/over archive bytes, total entries, member bytes, and aggregate expanded bytes, independent report/archive allowlist agreement, every content-rights category without disclosure, exact license/private metadata, offline installs, thin host discovery, canonical/pilot YAML equivalence, installed links/special files/escapes, and cross-format canonical byte identity | Property | P0 | FR-017-AC-3 | ✅ bounded Rust package, rights, manifest, integration, publication-refusal, archive, install, correspondence, and adverse-case slices reviewed and passing |
| TC-112 | Package-manager and host files contain declarative dispatch only; npm lifecycle hooks invoke the pinned local Rust CLI in explicit hook mode without embedded staging/refusal semantics or contamination of npm output; `make test` invokes the Rust content-rights tree adapter and `make package-audit` invokes the Rust package-audit adapter after their respective same-revision correspondence; local qualification, real-agent evaluation, publication, and release operations remain explicit manual actions | Static | P0 | FR-017-AC-4, FR-017-CON-2 | 🚧 npm lifecycle, content-rights, and package-audit Rust dispatch backed; remaining host-configuration census pending |
| TC-113 | Removal refuses mismatched revisions, incomplete parity, and direct invocations that still use the old path; content-rights removal additionally requires exact full-tree status/finding correspondence and an independently passing Rust tree and package-audit dispatch at one candidate revision, followed by removal of every executable import or subprocess reference to the deleted Python paths | Property | P0 | FR-018-AC-2, FR-018-CON-1 | 🚧 content-rights and package-audit same-revision cutovers plus final four-file deletion candidate backed; aggregate removal population pending |
| TC-114 | A failed cutover can restore the previous invocation without changing historical bytes; the content-rights and package-audit dispatches are separately reverted while their retained implementations remain present before the final deletion step | Property | P0 | FR-018-AC-3, FR-018-CON-2 | 🚧 content-rights and package-audit revert/reapply evidence backed; aggregate rollback population pending |
| TC-115 | Final audit finds no unapproved non-Rust semantic or assertion logic | Static | P0 | StR-003-VC-4, FR-016-CON-2, FR-018-AC-4, FR-018-CON-3, NFR-005-AC-4 | 🚧 pending implementation |
| TC-116 | Exact Rust 1.98.1 builds and tests every target with unsafe code forbidden | Compile | P0 | NFR-005-AC-1 | ✅ Rust package foundation; exact all-target gates recorded with implementation review |
| TC-117 | Parsed first-party Rust tests under `src/` and `tests/` use an exact unaliased `ix_trace_rs::trace` import and bare trace attributes carrying both TC and AC literals; absent, aliased, path-qualified, malformed, invalid-source, and over-limit cases fail closed | Static | P0 | NFR-005-AC-2 | ✅ Parsed repository census, canonical-form adverse cases, and mutation evidence passing |
| TC-118 | Quire reconciles every Rust test marker without missing, orphaned, or duplicate bindings | Integration | P0 | NFR-005-AC-3 | 🚧 pending implementation |
| TC-119 | The pure Rust content-rights classifier matches retained finding/exception behavior, rejects unsafe paths without echoing them, preserves Unicode protected-token matching, leaks no matched content, and remains deterministic and I/O-free | Property | P0 | FR-017-AC-5, FR-017-CON-3 | ✅ typed Rust classifier, retained-Python differential, exhaustive boundary cases, and mutation probes passing |
| TC-120 | The pure Rust package-membership classifier matches retained extra/missing behavior for safe unique names; rejects invalid expected policies and population limits; withholds on invalid, duplicate, unexpected, or missing observed members without echoing unsafe paths; remains permutation-invariant and I/O-free; and does not select or decode a package format | Property | P0 | FR-017-AC-6, FR-017-CON-3 | ✅ Typed Rust policy, retained-Python differential, adverse boundaries, ordering, resource ceilings, and mutation probes passing |
| TC-121 | The pure Rust module-manifest classifier accepts the retained valid module; consumes the authoritative module and artifact schemas offline; and deterministically withholds for every malformed manifest/schema/registry/resource/frontmatter/heading case without unsafe-reference or source-byte disclosure, I/O, schema discovery, or copied manifest grammar | Property | P0 | FR-017-AC-7, FR-017-CON-3 | ✅ `tests/manifest_parity.rs` |
| TC-129 | The Rust report adapter strictly decodes `cli-agent-evals.report/v1`, admits only retained successful single-run samples beneath the explicit workspace root, verifies exact transcript bytes and source identity, preserves failed-attempt and host-model behavior, refuses every malformed/version/path/link/kind/digest/resource/population case, remains input-order invariant, and matches the retained Python aggregate artifact before Rust command cutover | Integration | P0 | FR-017-AC-1, FR-017-CON-1, FR-017-CON-3 | 🚧 planned; live 28-cell execution remains TC-109 |

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

## Rust Port Permutation Matrix

| Test Case | Axis | Variants | Expected |
|---|---|---|---|
| TC-098, TC-099 | Protocol/result | supported v1; unknown version; malformed input; escaping root; unavailable or invalid host | one versioned result for success; every adverse case fails before a write or downstream action |
| TC-100, TC-102, TC-103 | Semantic state | success; unavailable; not-computed; not-applicable; failed; inconclusive; malformed; stale; tampered; lossy; unreadable; arbitrary-precision integer; NaN/infinity/overflow-to-non-finite; scalar and structured PGM-01 identities; exact metadata containing `x`; wildcard/range/invalid-character version | byte-identical success on the shared supported domain or one explicit declared structured-identity divergence; each non-success outcome and typed refusal reason remains distinct; invalid numeric/version inputs mint no digest |
| TC-107 | ix-flow lifecycle host | new run; pristine unbound recovery; interrupted resume; missing/accept/reject/repeated/opposite choice; invalid phase; binding drift; intact/broken chain; absent/wrong-version/malformed/oversized/timed-out host; automatic-gate evidence; hostile next-action text | preserve ix-flow ownership, exact binding, verified event integrity, one attributed decision, bounded typed output, pre-mutation history on refusal, and status-reconciled state after an indeterminate mutation; pass no gate override and execute no returned command text |
| TC-108, TC-109, TC-110 | Remaining host integration | supported request/result; absent host; malformed response; invalid transition; incomplete scenario | preserve host ownership and fail unsupported inputs explicitly |
| TC-113, TC-114 | Cutover | old only; additive parity; direct invocation updated; parity failure | delete only after local parity and restore the prior invocation on failure |
| TC-100 | Pure capability | compatibility classification; semantic validation; bounded projection; fixture generation; canonical identity | each ADR-002 row has an independently reported old/new corpus and adverse-case result |
| TC-111 | Qualification capability | package members; rights; manifest; npm stage/clean; integration; publication refusal | each gate has one retained passing case and every named refusal case; cleanup deletes nothing unless every present staged byte corresponds |
| TC-120 | Package membership | exact match; empty expected/actual; extra; missing; duplicate expected/actual; absolute, backslash, trailing-slash, empty, dot, parent, control, non-normalized, and overlong path; 65,536 and 65,537 entries; permuted inputs | exact safe membership is accepted; invalid policy or resource limits refuse without a result; every observed mismatch withholds with deterministic typed findings and no unsafe-path disclosure |
| TC-121 | Module manifest | retained valid bundle; malformed/duplicate/merge-key manifest; invalid or mismatching authoritative schema; identity drift; duplicate artifact; unregistered edge; missing/duplicate/extra resource; unsafe schema reference; malformed/invalid artifact schema; missing/malformed/duplicate/merge-key/schema-invalid skeleton frontmatter; optional or absent required locator heading; exact/over document, artifact-count, and combined-byte ceilings; permuted resources | valid input is accepted; ceiling violations refuse without a result; every semantic/input defect withholds through deterministic typed findings without source-byte or unsafe-reference disclosure |
| TC-129 | Retained report adapter | exact/unknown report version; complete/failed run; retained/not-retained/unavailable transcript; safe/absolute/backslash/dot/parent/control path; matching/changed/missing/linked/special/oversized transcript; one/two model identities; exact/over report, result, and collection ceilings; permuted report paths | exact retained samples map to typed envelopes and the retained aggregate; every adverse case refuses or withholds without reading outside the explicit roots; permutation cannot alter output |
| TC-106 | Invariant request | eleven canonical names; ordered multi-name request; unknown name; malformed Rust request; expired/current exception at one explicit instant | exact ordered Rust/reference parity for the valid shared domain or a typed pre-outcome refusal |
| TC-105 | Onboarding request | retained inventory/status cases; absent Quire; malformed, duplicate-key, or merge-key frontmatter; duplicate artifact; unsupported type; absolute, parent, symlink, and existing targets; valid/invalid staged artifact | exact Rust/reference inventory and decision parity on the supported domain, explicit fail-closed YAML identity handling, equivalent parsed frontmatter plus identical Markdown body for valid publication, or a typed refusal with no published bytes |

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
| New ix-flow run interrupted before binding | Pristine unbound to bound resume | TC-107 | Add exactly one matching binding, then resume |
| Decision-ready run interrupted after gate defer or acknowledgement | Retry same explicit choice | TC-107 | Complete exactly one attributed terminal decision |
| Terminal run receives opposite choice | Accepted/rejected conflict | TC-107 | Refuse without changing history |
| Legacy path only | Additive Rust path enabled | TC-113 | Both remain available; no completion claim |
| Old and Rust paths pass at the same revision | Direct invocation updated | TC-097, TC-113 | Rust path becomes the repository-owned invocation |
| Direct invocation and parity checks pass | Legacy path removed | TC-113 | Removal allowed once |
| Rust cutover fails | Previous invocation restored | TC-114 | Historical evidence and corpus bytes remain unchanged |

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
| EC-013 | Existing digest domain is silently changed to another canonicalization | FR-015 | TC-100, TC-103 | Historical identities change without a version boundary |
| EC-014 | ix-flow or cli-agent-evals cannot load a structured external provider | FR-016, FR-017 | TC-108, TC-109 | JavaScript remains an undeclared permanent dependency |
| EC-015 | A legacy path is deleted before its Rust replacement and direct invocation pass | FR-018 | TC-113 | Qualification or onboarding becomes unavailable |
| EC-016 | Path-qualified ix-trace-rs macro compiles but Quire cannot bind it | NFR-005 | TC-117, TC-118 | Passing Rust tests provide no requirement evidence |
| EC-017 | Content path or protected token attempts to escape or disclose selected bytes | FR-017 | TC-119 | Qualification reads unowned content or leaks the material it is meant to reject |

## Integration Test Matrix

### Cross-Project Integrations

| Integration ID | Purpose | Target Project | Type | Test Cases | Status |
|----------------|---------|----------------|------|------------|--------|
| INT-001 | Validate justified assurance artifacts | quire-rs | service | TC-006, TC-017, TC-045 | ✅ |
| INT-002 | Persist and render evidence without redefining policy | quoin | service | TC-025, TC-046 | ✅ |
| INT-003 | Load workflows, persist runs, resume, and gate decisions | ix-flow | service | TC-026, TC-028, TC-029, TC-030, TC-035, TC-047, TC-048 | ✅ |
| INT-004 | Discover canonical onboarding through supported hosts | agent discovery adapters | service | TC-010, TC-011, TC-038 | ✅ |
| INT-005 | Validate shared semantic references and historical compatibility | Quire, Quoin, ix-flow, native producer fixtures | library | TC-052..TC-068 | ✅ |
| INT-006 | Load Rust workflow invariants through a structured host interface | ix-flow | service | TC-107, TC-108 | 🚧 |
| INT-007 | Load Rust-owned scenarios and assertions through a structured host interface | cli-agent-evals | service | TC-109, TC-110 | 🚧 |
| INT-009 | Decode and verify retained cli-agent-evals reports through Rust | cli-agent-evals | service | TC-129 | 🚧 |

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
| TC-107 | INT-006 | Run lifecycle and human gates | Versioned Rust provider and fictional runs | Existing ix-flow state behavior is preserved | P0 |
| TC-108 | INT-006 | Canonical and pilot invariant loading | Canonical and compatibility workflows | Both resolve the same Rust invariant provider | P0 |
| TC-109 | INT-007 | Complete supported-host evaluation | Seven scenarios on four hosts | 28 valid result cells with explicit decisions | P0 |
| TC-129 | INT-009 | Retained report ingestion | Versioned reports and transcripts under an explicit workspace root | Typed aggregate input or deterministic refusal without out-of-root reads | P0 |
| TC-110 | INT-007 | Invalid or incomplete evaluation | Missing and malformed host results | Aggregate gate remains withheld | P0 |

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
| FR-012 | FR-012-AC-9 | TC-095 | ✅ Passing |
| FR-013 | FR-013-AC-1 | TC-087 | ✅ Passing |
| FR-013 | FR-013-AC-2 | TC-088 | ✅ Passing |
| FR-013 | FR-013-AC-3 | TC-089 | ✅ Passing |
| FR-013 | FR-013-AC-4 | TC-090 | ✅ Passing |
| FR-013 | FR-013-AC-5 | TC-091 | ✅ Passing |
| FR-013 | FR-013-AC-6 | TC-092 | ✅ Passing |
| FR-013 | FR-013-AC-7 | TC-093 | ✅ Passing |
| FR-013 | FR-013-AC-8 | TC-094 | ✅ Passing |
| NFR-004 | NFR-004-AC-1 | TC-068 | ✅ Passing |
| NFR-004 | NFR-004-AC-2 | TC-059 | ✅ Passing |

## Coverage Gaps

The completed baseline has no open gap: Quire reconciles TC-001..TC-051
to real tracking-tagged symbols, and the retained 28-cell aggregate records the
selected host commands, models, governing versions, transcripts, and outcomes.
TC-052..TC-068 now have tracking-tagged implementations in
`tests/test_verification_semantics.py`, and TC-069..TC-094 in
`tests/test_compatibility_corpus.py` and `tests/test_compatibility_matrix.py`,
which enforce the accepted compatibility corpus that FR-010 previously deferred
and the pinned release matrix used by the existing campaign contract.

The Rust port has an intentional open gap: TC-096..TC-118 are mapped, with
completed slices identified in the matrix. TC-108 depends on an ix-flow
interface supported by its owner, and TC-109 depends on a cli-agent-evals
interface supported by its owner. No implementation or removal is complete
while its applicable dependency or test row remains open.

## Test Execution Summary

| Category | Total | Passed | Failed | Blocked | Coverage |
|----------|-------|--------|--------|---------|----------|
| Unit | 12 | 12 | 0 | 0 | 100% |
| Integration | 20 | 20 | 0 | 0 | 100% |
| E2E | 13 | 13 | 0 | 0 | 100% |
| Property | 15 | 15 | 0 | 0 | 100% |
| Static | 8 | 8 | 0 | 0 | 100% |

The table reports only the completed baseline and does not count the 23 staged
Rust-port rows.
