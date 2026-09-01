---
type: master-requirements
name: engineering-assurance
org: agent-ix
component_type: configuration-module
implementation_language: python
title: "engineering-assurance Master Requirements Specification"
depends_on:
  - quire
  - quoin
  - ix-flow
relationships:
  - target: "ix://agent-ix/quire-rs"
    type: "depends_on"
  - target: "ix://agent-ix/quoin"
    type: "depends_on"
  - target: "ix://agent-ix/ix-flow"
    type: "depends_on"
---

# Master Requirements Specification

## Purpose

This specification defines the repository-owned onboarding capability and the
shared verification vocabulary/type-fit contract. Onboarding helps an agent
inspect an existing repository and enter a governed workflow without inventing
policy, evidence, or terminal decisions. The verification contract keeps
definitions, executions, results, evidence, measurements, diagnostics, reports,
and human decisions distinct while reusing their authoritative Quire, Quoin,
native-producer, and ix-flow representations.

It is the top-level requirements artifact for the engineering-assurance module.
The discrete requirement files under `spec/` are authoritative; this document
defines their shared scope and indexes them.

## Scope

### In Scope

- A canonical `assurance-onboarding` skill and workflow bundle owned by this
  repository.
- Promotion of the existing `assurance-intake`, `architecture-evaluation`,
  `measurement-promotion`, and `change-assurance` pilots into that bundle.
- Thin discovery manifests for Claude Code, Codex, opencode, and GitHub Copilot.
- Local-source and repository-source installation, package membership, and
  preservation of the module-root contract already consumed by Quire.
- Existing-repository intake that inventories decisions and measurements before
  proposing an AssuranceProfile or MeasurementPlan.
- Explicit evidence availability, producer provenance, operator observations,
  resumable workflow state, and named human terminal decisions.
- Smoke tests and agent evaluations for supported discovery and onboarding paths.
- Semantic ownership and link-direction definitions for verification concepts.
- Versioned reference and report projection contracts, deterministic
  cross-language fixtures, and read-only historical PGM-01 compatibility.

### Out of Scope

- Defining assurance policy for a repository being inspected.
- Generating generic AssuranceProfile or MeasurementPlan artifacts when the
  repository does not justify them.
- Reimplementing Quire validation, Quoin evidence or policy semantics, or ix-flow
  state and gate mechanics.
- Automating acceptance, approval, rejection, promotion, or other terminal human
  decisions.
- Publishing this private module to a public package registry.
- Adding agent-specific copies of canonical skill or workflow content.
- Executing native verification producers, scraping arbitrary stdout for a
  verdict, persisting a parallel evidence store, or inferring human decisions.
- Replacing Quire static facts, Quoin evidence/audit/report records, native
  domain result formats, or ix-flow decision history.

## System Overview

Engineering-assurance is a private configuration module containing artifact
schemas, skeletons, and shared semantic contracts. The onboarding capability
adds one repository-owned entry point that inventories declared decisions,
measurements, artifacts, and producer availability before proposing the smallest
justified work. The verification-semantics capability publishes only definitions,
reference validation, read-only compatibility, and bounded projections.

Quire remains the validator for authored assurance artifacts. Quoin remains the
owner of evidence records and policy-facing reports. ix-flow remains the owner of
workflow run state, resume behavior, and human gates. Agent manifests expose the
same canonical bundle and do not redefine these responsibilities.

## Requirements Architecture

### Stakeholder Requirements

- [StR-001](./stakeholder/StR-001-bounded-existing-repository-onboarding.md) —
  onboard an existing repository without invented assurance claims.
- [StR-002](./stakeholder/StR-002-review-verification-evidence-without-semantic-collapse.md) —
  review linked verification evidence without collapsing semantic boundaries.

### User Stories

- [US-001](./usecase/US-001-assess-existing-repository.md) — assess an existing
  repository before proposing assurance work.
- [US-002](./usecase/US-002-discover-onboarding-across-agents.md) — discover the
  same onboarding capability in each supported coding agent.
- [US-003](./usecase/US-003-resume-and-decide-workflow.md) — resume interrupted
  work and retain explicit human decisions.
- [US-004](./usecase/US-004-understand-evidence-availability.md) — distinguish
  evidence states and inspect producer provenance.
- [US-005](./usecase/US-005-correlate-definition-result-evidence.md) — correlate
  definitions, executions, results, evidence, reports, and decisions.

### Functional Requirements

- [FR-001](./functional/FR-001-inventory-before-proposal.md) — inventory existing
  decisions and measurements before proposing artifacts.
- [FR-002](./functional/FR-002-canonical-discovery-bundle.md) — expose one canonical
  skill and workflow tree through thin agent manifests.
- [FR-003](./functional/FR-003-package-installable-bundle.md) — include the
  onboarding bundle in installable packages without breaking the module root.
- [FR-004](./functional/FR-004-evidence-state-and-provenance.md) — preserve evidence
  availability, producer versions, and operator observations.
- [FR-005](./functional/FR-005-resumable-human-decisions.md) — delegate resumable
  state and terminal decisions to ix-flow.
- [FR-006](./functional/FR-006-agent-evaluation-suite.md) — exercise the required
  onboarding and failure scenarios with real agent evaluations.
- [FR-007](./functional/FR-007-pilot-compatibility.md) — retain compatibility for
  the existing pilot invocation while promoting canonical paths.
- [FR-008](./functional/FR-008-distinguish-verification-semantics.md) — distinguish
  verification concepts and map each to its authoritative owner.
- [FR-009](./functional/FR-009-preserve-provenance-and-states.md) — preserve the
  producer tuple, definition version, and every non-success state.
- [FR-010](./functional/FR-010-read-only-compatibility-and-reporting.md) — map
  historical PGM-01 records read-only and define bounded reports.
- [FR-011](./functional/FR-011-accepted-compatibility-corpus.md) — retain the
  accepted real-record corpus and enforce it as the migration gate.
- [FR-012](./functional/FR-012-pinned-compatibility-matrix.md) — pin the exact
  released shared-assurance versions and classify an observed toolchain.
- [FR-013](./functional/FR-013-migration-contract.md) — publish the reviewed
  migration contract the eight repositories are migrated against.

### Non-Functional Requirements

- [NFR-001](./non-functional/NFR-001-cross-agent-parity.md) — all supported agents
  resolve the same canonical content.
- [NFR-002](./non-functional/NFR-002-non-inventing-onboarding.md) — evaluations
  introduce no unsupported assurance artifacts or conclusions.
- [NFR-003](./non-functional/NFR-003-package-contract-stability.md) — package audits
  preserve the module payload and canonical onboarding bundle.
- [NFR-004](./non-functional/NFR-004-no-parallel-assurance-framework.md) — prevent
  a parallel executor, evidence framework, generic scraper, or trust score.

### Integration Tests

- [IT-001](./integration/IT-001-local-source-agent-discovery.md) — discover the
  canonical bundle after a real local-source install.
- [IT-002](./integration/IT-002-package-workflow-discovery.md) — discover module and
  workflow content from real package archives.
- [IT-003](./integration/IT-003-existing-repository-onboarding.md) — run intake for
  existing, absent, malformed, and unavailable inputs.
- [IT-004](./integration/IT-004-interruption-resume-and-rejection.md) — resume a real
  workflow and retain an explicit rejection.
- [IT-005](./integration/IT-005-verification-semantics-compatibility.md) — validate
  current and historical semantic fixtures without running producers.

## Ownership Boundaries

| Boundary | Owner | This Module's Responsibility |
|----------|-------|------------------------------|
| Assurance artifact validation | Quire | Invoke the installed validator and preserve its result without reinterpretation. |
| Evidence and policy records | Quoin | Request or reference records and preserve their declared states and provenance. |
| Workflow lifecycle | ix-flow | Supply canonical definitions and use its run, resume, transition, and human-gate behavior. |
| Agent discovery | engineering-assurance | Expose one canonical bundle through thin host manifests. |
| Onboarding judgment | engineering-assurance plus named human | Inventory, propose bounded work, and leave terminal choices to the human owner. |
| Shared verification vocabulary | engineering-assurance | Define semantic distinctions and type-fit mappings without owning a second persisted record family. |
| Native verification execution and domain result | campaign repository/domain tool | Execute checks and preserve the domain result schema, oracle, and failure behavior. |

## Error and Failure Model

Onboarding distinguishes absence from failure. A producer that cannot run is
`unavailable`; a producer that was not invoked is `not_computed`; and a producer
outside the selected decision boundary is `not_applicable`. Malformed producer
output remains an explicit validation failure. None of these conditions is
converted into successful evidence or a fabricated artifact.

Interrupted workflow runs retain their run identifier and last completed phase.
Terminal transitions remain human-gated, including explicit rejection paths.

## Verification Strategy

Static contract tests verify canonical ownership, manifest thinness, package
membership, and compatibility paths. Integration tests install from real local
and package sources, invoke real filesystem discovery, validate with Quire, and
run real ix-flow definitions. Agent evaluations exercise the five required
scenario classes across all supported hosts and record immutable module, plugin,
skill, workflow, executable, schema, and producer versions; transcript digests;
commands; elapsed time; human-interaction counts; outcomes; and explicit positive
and negative human decisions.

## Change Management

Changes to canonical skill or workflow content, discovery manifests, package
membership, or module-root layout require corresponding matrix and package-audit
updates. Agent manifests may point to canonical content but may not acquire
independent behavioral text.

## References

- `engineering_assurance/manifest.yaml` — installed module-root contract.
- `pilots/assurance-workflows/` — workflow definitions promoted by this scope.
- `CONTENT_RIGHTS.md` — repository content and package rights boundary.
