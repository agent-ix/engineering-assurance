---
type: master-requirements
name: engineering-assurance
org: agent-ix
component_type: rust-lib
implementation_language: rust
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
- Versioned reference contracts, deterministic
  cross-language fixtures.
- A Rust library crate and native CLI that own this repository's production,
  validation, generation, evaluation, and qualification behavior.
- Supported structured host interfaces for ix-flow and agent evaluations.
- One reusable bounded Rust producer-execution boundary for declared native
  tools and caller-owned typed response adapters.
- A parity-gated Rust port and cutover of this repository's current Python,
  JavaScript, and MJS executable paths.

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
- Selecting or qualifying native verification producers, scraping arbitrary
  stdout for a verdict, persisting a parallel evidence store, or inferring
  human decisions.
- Replacing Quire static facts, Quoin evidence/audit/report records, native
  domain result formats, or ix-flow decision history.
- Moving Engineering Assurance behavior into another repository or creating a
  new repository for the Rust port.
- Treating a Rust wrapper around non-Rust semantic or assertion logic as a
  completed port.

## System Overview

Engineering-assurance is a Rust library and native CLI accompanied by artifact
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
- [StR-003](./stakeholder/StR-003-rust-native-engineering-assurance.md) — keep
  first-party Engineering Assurance production and qualification behavior in a
  centralized Rust boundary.

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
- [US-006](./usecase/US-006-hand-off-source-facts-to-an-integrator.md) — hand source
  facts to an integrator without closing the integrator's validation.

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
- FR-010 — withdrawn (read-only PGM-01 compatibility and bounded reports). The
  identifier and FR-010-AC-1..4 are not reused.
- FR-011 — withdrawn (compatibility corpus gate). The identifier, FR-011-CON-1..4 and
  FR-011-AC-1..10 are not reused.
- [FR-013](./functional/FR-013-migration-contract.md) — publish the reviewed
  migration contract the eight repositories are migrated against.
- [FR-014](./functional/FR-014-versioned-rust-boundary.md) — expose the shared
  library and CLI through versioned structured interfaces.
- [FR-015](./functional/FR-015-semantic-and-identity-parity.md) — preserve
  verification semantics, identity, and canonicalization in Rust.
- [FR-016](./functional/FR-016-rust-onboarding-and-workflow.md) — move onboarding,
  workflow coordination, and invariants behind the Rust boundary.
- [FR-017](./functional/FR-017-rust-evaluation-and-qualification.md) — move agent
  evaluation and repository qualification behavior behind the Rust boundary.
- [FR-018](./functional/FR-018-staged-runtime-migration.md) — retire replaced
  executable paths after local parity and direct-invocation cutover.
- [FR-019](./functional/FR-019-bounded-producer-execution.md) — execute an exact
  caller-declared producer through one bounded public Rust library boundary.
- [FR-020](./functional/FR-020-measurement-plan-objective.md) — declare a
  MeasurementPlan's objective direction and bound as part of its definition.
- [FR-021](./functional/FR-021-measurement-plan-decision-rule.md) — declare a
  MeasurementPlan's estimator and decision rule as closed, checkable data.
- [FR-022](./functional/FR-022-claim-strength-vocabulary.md) — own the unordered
  claim-strength vocabulary for advanced verification results.
- [FR-023](./functional/FR-023-claim-evidence-references.md) — require a
  `supported` assurance claim to reference its evidence records.
- [FR-024](./functional/FR-024-measurement-plan-protected-apparatus.md) — declare
  a MeasurementPlan's protected apparatus and the negative controls a gate
  plan must catch.
- [FR-025](./functional/FR-025-measurement-promotion-checker-policy.md) — read
  the independent checker's result when promoting a measurement, advisory or
  required per AssuranceProfile `measurement_policy`.
- [FR-026](./functional/FR-026-measurement-plan-objective-steering-fields.md) —
  declare optional `weight`, `value_half_life`, and `budget` steering fields
  on a MeasurementPlan objective, advisory only and never a gate.
- [FR-027](./functional/FR-027-atomic-publish.md) —
  publish evidence files without replacement.
- [FR-029](./functional/FR-029-producer-execute-command.md) — expose the FR-019 executor as a
  versioned `producer-execute` CLI command with published request and result schemas.
- [FR-030](./functional/FR-030-integrator-package-contract.md) — write and
  strictly read the versioned integrator package, preserving source facts and
  leaving adopter validation open.
- [FR-031](./functional/FR-031-integrator-package-publication.md) — publish an
  integrator package bound to its candidate, atomically and idempotently, with
  conflicts instead of winners.
- [FR-032](./functional/FR-032-declared-scope-footprint-verification.md) — verify a declared
  change scope against the candidate change footprint: complete, false scope or incomplete.
- [FR-033](./functional/FR-033-measurement-plan-cross-profile-comparison.md) — let a
  MeasurementPlan opt in to comparing collections built under different build profiles.
- [FR-034](./functional/FR-034-measurement-plan-paired-run.md) — let a MeasurementPlan state how
  two runs of the same measurement relate.
- [FR-035](./functional/FR-035-measurement-plan-population-set.md) — let a
  MeasurementPlan declare an enumerated population set or an exemption reason.
- [FR-036](./functional/FR-036-ssh-signed-commit-and-tag-verification.md) — verify SSH
  signatures on commits and annotated tags by delegating to git.
- [FR-037](./functional/FR-037-signer-mapping-and-signed-off-meaning.md) — map verified
  signers to named people and expose the signed-off meaning.
- [FR-038](./functional/FR-038-syft-sbom-producer-request.md) — define the Syft producer as one
  `producer-execute` request shape emitting CycloneDX JSON, with existing result states for failure.
- [FR-039](./functional/FR-039-syft-sbom-evidence-ingestion.md) — end Engineering Assurance's SBOM work at the retained
  CycloneDX artifact the Quoin SBOM adapter accepts, with no parser or quoin launch here.
- [FR-044](./functional/FR-044-osv-scanner-advisory-scan-producer-request.md) — define the OSV-Scanner advisory scan as one
  `producer-execute` request shape reading the retained Syft SBOM offline, producing a JSON report, with no second SBOM.
- [FR-040](./functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md) — verify
  gitsign (Sigstore keyless) signatures on commits and annotated tags by
  delegating to gitsign.
- [FR-041](./functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md) — verify a
  signed annotated tag as a named person's approval of an exact change-assurance record, reporting the
  signer, signing time and meaning together.
- [FR-042](./functional/FR-042-create-signed-approval-tag-for-a-change-assurance-record.md) — create a
  signed annotated tag that approves a change-assurance record, signed through the caller's git configuration.
- [FR-043](./functional/FR-043-approval-trail-of-a-change-assurance-record.md) — list the audit trail of signed
  approval tags that name a change-assurance record, oldest first by claimed signing time, with every verdict shown.
- [FR-045](./functional/FR-045-document-converter-producer-requests.md) — run the document converters pandoc and
  typst as `producer-execute` requests, with existing result states for failure.
- [FR-046](./functional/FR-046-baseline-archetype.md) — declare a Baseline as a named set of configuration
  items at one git commit, identified by the commit hash alone.
- [FR-047](./functional/FR-047-release-record-and-signed-release-tag.md) — verify a Release as a Baseline plus a
  named person's signed release tag, reusing the approval binding of FR-041.
- [FR-048](./functional/FR-048-human-release-decision-workflow.md) — decide a release through a human-gated,
  resumable workflow; no automatic decision.
- [FR-049](./functional/FR-049-release-ownership-split-and-data-contract.md) — Engineering Assurance owns the
  release record and verification; Quoin consumes the result as data and runs no git.
- [FR-050](./functional/FR-050-record-named-persons-release-decision-stance.md) — state that Engineering Assurance
  records a named person's release decision and never decides one.
- [FR-051](./functional/FR-051-release-evidence-snapshot-placement.md) — place the Quoin-written release evidence
  snapshot in the release commit beside the record, and check it is present there.
- [FR-052](./functional/FR-052-release-history-since-the-previous-release.md) — list the change-assurance records
  merged since the previous release, as data for the snapshot.
- [FR-053](./functional/FR-053-snapshot-and-attestations-never-replace-the-decision.md) — attestations may carry the
  snapshot and never replace the signed release decision.
- [FR-054](./functional/FR-054-record-sink-interface-and-implementations.md) — publish a record through a sink interface with a public log, a private store and a none implementation.
- [FR-055](./functional/FR-055-per-repo-sink-configuration-and-visibility-default.md) — select a sink per repository, defaulting from visibility.
- [FR-056](./functional/FR-056-public-and-private-record-split.md) — split a record into a trimmed public form and a full private form linked by id.
- [FR-057](./functional/FR-057-fail-closed-pre-publish-leak-check.md) — refuse a public publish and public-log signing when the publishing repo or a reference is private or undeterminable.
- [FR-058](./functional/FR-058-approval-statement-and-predicate.md) — record a change approval as a CI-signed in-toto statement bound to the pull request and merged commit.
- [FR-059](./functional/FR-059-source-adapter-interface-and-github-review-adapter.md) — read approvals through a source adapter interface, GitHub review adapter first.
- [FR-060](./functional/FR-060-approval-binds-to-the-merged-head-commit.md) — bind an approval to the head that merged and survive a squash merge.
- [FR-061](./functional/FR-061-role-policy-and-distinct-individuals.md) — require every policy role to be signed by a distinct individual.
- [FR-062](./functional/FR-062-ai-agents-are-never-approvers.md) — never accept an AI agent identity as an approver.
- [FR-063](./functional/FR-063-unstaffed-required-role-fails-visibly.md) — fail with a named error when a required role has no staffed approver.
- [FR-064](./functional/FR-064-verify-approval-statement-with-workflow-identity-policy.md) — verify approval statements against the trusted CI workflow identity and the role policy.
- [FR-065](./functional/FR-065-workflow-roles-and-relation-to-signed-release-tags.md) — name the CI workflow roles, gate on reviews before merge; release decisions stay person-signed tags.
- [FR-066](./functional/FR-066-release-attestation-set-and-subjects.md) — emit a release as a fixed set of standard attestations over the release artifacts.
- [FR-067](./functional/FR-067-release-attestations-bind-to-the-signed-release-tag.md) — attest a release only after the signed tag and the role policy verify.
- [FR-068](./functional/FR-068-verification-summary-signed-by-the-pinned-workflow.md) — sign one verification summary with a pinned reusable workflow.
- [FR-069](./functional/FR-069-test-result-export-is-consumed-as-data.md) — consume the Quoin test-result export as data.
- [FR-070](./functional/FR-070-release-and-receipt-workflow-files.md) — the attestation and receipt workflows: files, jobs, triggers, permissions and untrusted-input handling.
- [FR-071](./functional/FR-071-release-attestations-publish-through-the-sinks.md) — publish release attestations through the sinks; never write a private repository to a public log.
- [FR-072](./functional/FR-072-change-record-predicate.md) — the change-record/v1 predicate: identifiers, hashes, verdicts, per-role approval references.
- [FR-073](./functional/FR-073-agent-bom-in-cyclonedx-1-7.md) — the agent bill of materials as a producer-reported CycloneDX 1.7 document.
- [FR-074](./functional/FR-074-per-change-receipt-bundle-records-failures-too.md) — one CI-signed receipt per merged commit, failures recorded too.
- [FR-075](./functional/FR-075-push-to-main-receipt-workflow.md) — write the receipt in a push-to-main workflow whose signing job runs no repository code.
- [FR-076](./functional/FR-076-receipt-storage-in-git-notes.md) — store the receipt in git notes; publish only the trimmed record.
- [FR-077](./functional/FR-077-missing-receipt-check.md) — list commits with no valid receipt; unknown status counts as missing.
- [FR-078](./functional/FR-078-receipt-verify.md) — verify a receipt against the pinned workflow identity, the commit and the role policy.
- [FR-079](./functional/FR-079-content-rights-admits-the-attestation-schema-prefix.md) — admit the attestation schema prefix in the content-rights URL check.

### Non-Functional Requirements

- [NFR-001](./non-functional/NFR-001-cross-agent-parity.md) — all supported agents
  resolve the same canonical content.
- [NFR-002](./non-functional/NFR-002-non-inventing-onboarding.md) — evaluations
  introduce no unsupported assurance artifacts or conclusions.
- [NFR-003](./non-functional/NFR-003-package-contract-stability.md) — package audits
  preserve the module payload and canonical onboarding bundle.
- [NFR-004](./non-functional/NFR-004-no-parallel-assurance-framework.md) — prevent
  a parallel executor, evidence framework, generic scraper, or trust score.
- [NFR-005](./non-functional/NFR-005-rust-containment-and-traceability.md) — enforce
  Rust containment, current-stable qualification, unsafe-code, and canonical
  ix-trace-rs test tracing.
- [NFR-006](./non-functional/NFR-006-single-sbom-tool-single-execution-path.md) — Syft is the only
  SBOM generator, run only through the FR-019 executor, read only through the Quoin adapter.
- [NFR-007](./non-functional/NFR-007-no-release-without-a-person.md) — a release is reported only for a verified
  person's signed decision.
- [NFR-008](./non-functional/NFR-008-closed-schemas.md) — records, configuration, statements and policies use closed schemas.
- [NFR-009](./non-functional/NFR-009-no-private-reference-reaches-a-public-log.md) — no private reference reaches a public log.
- [NFR-010](./non-functional/NFR-010-no-cryptography-of-our-own.md) — no cryptography of our own.
- [NFR-011](./non-functional/NFR-011-signing-jobs-run-no-agent.md) — signing jobs run no agent, no repository code and hold no agent-reachable credential.

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
- [IT-006](./integration/IT-006-producer-execution-consumer.md) — verify that a
  real Rust consumer uses the bounded producer-execution library directly.
- [IT-007](./integration/IT-007-integrator-package-consumer.md) — verify that a
  real Rust consumer writes, publishes and reads an integrator package.

## Ownership Boundaries

| Boundary | Owner | This Module's Responsibility |
|----------|-------|------------------------------|
| Assurance artifact validation | Quire | Invoke the installed validator and preserve its result without reinterpretation. |
| Evidence and policy records | Quoin | Request or reference records and preserve their declared states and provenance. |
| Workflow lifecycle | ix-flow | Supply canonical definitions and use its run, resume, transition, and human-gate behavior. |
| Agent discovery | engineering-assurance | Expose one canonical bundle through thin host manifests. |
| Onboarding judgment | engineering-assurance plus named human | Inventory, propose bounded work, and leave terminal choices to the human owner. |
| Shared verification vocabulary | engineering-assurance | Define semantic distinctions and type-fit mappings without owning a second persisted record family. |
| Bounded native producer execution | engineering-assurance | Execute only an exact caller-declared tool through FR-019 and return a typed state without interpreting domain success. |
| Native producer selection, response semantics, and domain result | campaign repository/domain tool | Select checks and preserve the domain result schema, oracle, and failure behavior through a caller-owned response adapter. |
| Engineering Assurance executable behavior | engineering-assurance Rust crate | Implement repository-owned validation, generation, orchestration, evaluation, and qualification behavior. |
| Portable verification contracts | quire-verification | Publish versioned shared contract types; this module consumes rather than copies accepted definitions. |

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
scenario classes across all supported hosts and record module, plugin,
skill, workflow, executable, schema, and producer versions; transcript paths;
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
