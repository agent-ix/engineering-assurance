---
id: ADR-002
title: "Rust-native Engineering Assurance boundary"
type: ADR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "relates_to"
  - target: "ix://agent-ix/engineering-assurance/NFR-004"
    type: "relates_to"
---

# ADR 002: Rust-native Engineering Assurance boundary

**Status**: Accepted
**Date**: 2026-09-07
**Decision authority**: Peter Krenesky, repository owner (accepted 2026-09-08)

## Context

Engineering Assurance currently distributes first-party behavior across ten
Python package modules, ten Python qualification scripts, one canonical
JavaScript invariant module, one one-line JavaScript compatibility re-export,
two MJS evaluation modules, and package/build orchestration. The behavior is
cohesive, but its runtime boundaries are not: workflow assertions are loaded by
ix-flow, evaluation assertions are loaded by cli-agent-evals, and Python owns
onboarding, compatibility, projections, report aggregation, and qualification.

The earlier campaign migration centralized evidence retention and audit in
Quoin. It did not decide that Engineering Assurance should remain implemented
in Python or JavaScript. This decision changes the implementation boundary
without reversing that ownership: Quire still validates authored facts, Quoin
still retains and audits evidence, ix-flow still owns run and decision state,
portable verification contracts remain externally owned, and native producers
still own domain execution.

“Engineering Assurance ownership” means ownership of the domain-specific tools
and policy already allocated to this repository. It does not mean ownership of
every service, contract, or producer used during assurance. Those boundaries are
deliberate: qualification priorities differ, and placing unrelated code under
the strictest Engineering Assurance regime would enlarge both the trusted
computing base and the evidence population without transferring domain
ownership. Engineering Assurance consumes those components through versioned,
qualified interfaces and applies its stricter policy at its own boundary.

## Decision drivers

1. Put all first-party production and qualification semantics in Rust.
2. Preserve observable compatibility and every non-success/refusal path.
3. Keep one owner and one implementation for behavior shared by agents and
   qualification gates.
4. Avoid a new evidence model, generic producer runner, or universal stdout
   verdict adapter.
5. Migrate consumers without deleting a working path before parity is proven at
   the same candidate revision.
6. Isolate Engineering Assurance's strict qualification policy so it governs
   this repository's domain tools and integration boundary rather than
   transitively absorbing generic services, contracts, or native producers.

## Options considered

| Option | Disposition |
| --- | --- |
| Add a Rust library crate and native CLI to this repository | Selected. It preserves Engineering Assurance ownership and gives host integrations one versioned executable boundary. |
| Move the behavior into quire-verification | Rejected. Portable verification contracts and Engineering Assurance orchestration have different owners; combining them would make the contract package own workflow and qualification policy. |
| Add the behavior to Quoin | Rejected. Quoin remains the evidence retention, audit, and reporting authority and is not a home for producer execution or repository qualification. |
| Create another Rust repository | Rejected. It fragments the existing owner and requires separate repository governance before any technical advantage is established. |
| Keep Python/JavaScript behind a Rust wrapper | Rejected. The semantic and assertion logic would remain outside Rust and the wrapper would only conceal the boundary. |

## Capability and executable-path matrix

| Capability | Current paths and consumers | Classification | Rust disposition or gate |
| --- | --- | --- | --- |
| Compatibility classification and accepted corpus access | `engineering_assurance/compatibility.py`, `compatibility_corpus.py`; compatibility and campaign gates | Missing reusable implementation | Implement in the library; preserve read-only corpus and matrix behavior. |
| AssuranceProfile and MeasurementPlan module contracts | `engineering_assurance/schemas/`, `skeletons/`, and `manifest.yaml`; Quire validates the pinned package corpus and registered consumer revisions | Published cross-repository contract with an observed compatibility defect: the installed 0.2 module rejects legacy AssuranceProfile frontmatter and legacy MeasurementPlan body sections in the package corpus and `quire-rs`, while newer consumer shapes are accepted | Version the accepted legacy/current shapes, commit an owner-reviewed consumer registry and revision-bound compatibility snapshot, add real consumer fixtures, and complete owner-approved consumer migration before replacing a schema. An invalid profile cannot govern review selection. |
| Verification vocabulary, reference validation, projection, and fixture generation | `verification_semantics.py`, `evidence.py`, `fixture_codegen.py`; repository tests, generated fixture consumers, and historical PGM-01 views | Existing shared contracts plus missing Rust implementation | Consume accepted portable contract versions; implement Engineering Assurance mapping and validation without copying their ownership or creating persisted records. |
| Discovery, onboarding, and run coordination | `discovery.py`, `onboarding.py`, `workflow.py`; canonical skill, supported agent manifests, ix-flow | Engineering Assurance domain logic | Implement in the library and CLI; keep Quire and ix-flow as authoritative external services. |
| Workflow invariants | `engineering_assurance/skills/assurance-onboarding/scripts/invariants.js`; ix-flow | Engineering Assurance domain logic | Implement in Rust. Removal waits for an ix-flow external structured-provider interface. |
| Pilot invariant path | `pilots/assurance-workflows/scripts/invariants.js`; compatibility workflow loads | Compatibility re-export, not a second implementation | Retain only until canonical Rust host loading proves pilot parity, then remove with the pilot path. |
| Evaluation model and report aggregation | `evaluation.py`, `eval_reports.py`, `evals/result-contract.mjs`; repository and release gates | Engineering Assurance domain logic | Implement in the library and CLI with versioned result JSON and fail-closed validation. |
| Agent evaluation suite | `evals/cli-agent-evals.config.mjs`, `scripts/run_agent_evals.py`, `aggregate_agent_eval_reports.py`; cli-agent-evals and supported hosts | Engineering Assurance scenarios over an external host | Implement scenarios, assertions, and aggregation in Rust. Removal waits for a cli-agent-evals external structured-suite interface. |
| Repository integration and readiness audits | `check_compatibility_matrix.py`, `check_eval_readiness.py`, `check_integration_evidence.py`, `validate_manifest.py`; Make and release gates | Missing reusable implementation over Engineering Assurance contracts | Implement as CLI subcommands; preserve exit status, structured output, refusal, and failure paths. |
| Package and content-rights audits | `audit_packages.py`, `check_content_rights.py`, `stage-npm.mjs`, `refuse-publication.mjs`; Python wheel, npm package, and registry refusal | Repository-specific qualification and packaging | Implement as CLI subcommands; package managers may dispatch the binary but may not contain semantic assertions. |
| Generated `.py`, `.ts`, and `.rs` fixture files | `engineering_assurance/fixtures/verification-semantics/generated/`; compatibility tests and external-language readers | Foreign-language data fixtures | Keep as inert generated samples when required for parity; generate and compare them in Rust and never execute them as qualification logic. |
| Makefile, package manifests, and workflow YAML | `Makefile`, `package.json`, `pyproject.toml`, `.github/workflows/ci.yml`; developer and hosted gates | External host configuration | Convert semantic branches and assertions to Rust subcommands; retain only declarative dispatch allowed by the host. |

No row grants approval for residual Python, JavaScript, MJS, or inline CI
semantic logic. A host bridge that cannot be removed requires a separate,
explicit owner disposition for its exact file and behavior.

## Assurance-contract census baseline

The specification investigation used Quire 0.31.0 with the installed
`engineering-assurance` module at 0.2.0. The installed manifest, AssuranceProfile
schema, and MeasurementPlan schema SHA-256 digests were respectively
`80e4f2c8753bf541c5049272eada8d5b4792888bc128193b383a903c5fc31f72`,
`070f8802e62b8c38701b03445f86a008f77191baa8c0c4358147fb5126cf3535`,
and `aae266f2cefd706548dd25d7c232f0cf5d1ac04fe99ad540244be5f72bcfd05a`.
Those bytes match the package at revision `ae50e13154f9`.

The point-in-time census established two live contract generations:

- The package-pinned `qa-corpus` revision `ea4ac7a227bd` and the `quire-rs`
  specification at `8b8020e665c6` with corpus revision `7442f2770880` use the
  legacy AP/MP generation and fail different layers of the installed 0.2
  structural contract. The legacy AssuranceProfile frontmatter fails the
  closed frontmatter schema. The MeasurementPlan frontmatter passes its schema,
  but the documents fail the module's required `Decision Use`, `Population`,
  `Collection Procedure`, and `Interpretation` body-section contract.
- Current AP/MP frontmatter in Quoin, `quire-analyze`,
  `quire-contract-codegen`, `quire-contract-ir`, `quire-contract-runtime`,
  `quire-verification`, and the observed `tl-mltl`, `tl-parse`, `tl-rewrite`,
  and `tl-syntax` qualification revisions is accepted by the installed 0.2
  schemas. Quoin still has a separate duplicate-heading structural finding,
  which is not evidence of schema incompatibility.

This local census is diagnostic evidence, not the release inventory. The Rust
migration SHALL replace it with a committed owner-reviewed registry of canonical
consumer repositories and a candidate-revision snapshot. Workstation directory
discovery, stale clones, skeletons, quarantined evidence, and duplicated
submodule checkouts cannot silently add or remove a release consumer.

The structural result above was reproduced with Quire 0.31.0 against immutable
consumer revisions. From the `qa-corpus` root at
`ea4ac7a227bd2a393c29f722f9b331e1f97aa999`,
`quire validate --scope . "assurance/*.md" --summary` reports the AP
frontmatter/body failures and four missing required sections on each of
MP-201 and MP-202. From the `quire-rs` root at
`967eafe468c3b234a20856aaa761c71f55207735`,
`quire validate --scope . "spec/assurance/*.md" --summary` reports the same AP
failure classes and the same four missing sections on each of MP-201 through
MP-208. Both runs report every selected document grammar-clean. This evidence
must not be restated as a symmetric frontmatter-schema failure: MP frontmatter
passes the installed schema, and only its body-section contract rejects these
documents.

## Decision

The existing `engineering-assurance` repository remains the owner of the
Engineering Assurance-specific tools and policy enumerated in the capability
matrix. Their executable implementations will converge on one Cargo package
named `engineering-assurance`. That package exports the
`engineering_assurance` Rust library crate and the `engineering-assurance` CLI
binary. The underscore is Rust's import-name normalization; these are library
and binary targets of one package, not two independently governed products.

The library owns reusable types and deterministic Engineering Assurance domain
behavior. The CLI owns filesystem, process, and external-host boundaries and
exposes versioned JSON input and output for automation. “One Cargo package” is
an implementation and release boundary; it does not require every repository
artifact to become Rust or collapse all behavior into one monolithic binary.
Initial source consumption may use a pinned repository revision; this decision
does not authorize registry publication.

Schemas, module manifests, skeletons, skills, Markdown specifications,
controlled corpora, and generated cross-language fixtures remain declarative
content or inert test data in suitable standardized formats such as JSON, YAML,
Markdown, or a fixture consumer's source representation. A `.py`, `.ts`, or
`.rs` fixture is retained only as data to parse or compare; its file extension
does not authorize executable assurance behavior in that language. Makefiles,
package manifests, and workflow YAML may remain as thin host dispatch. They
cannot own semantic branches or assertions that duplicate the Rust
implementation. No executable Engineering Assurance policy may remain
implemented in Python, JavaScript, or MJS after its governed removal gate
passes.

Quire validation, Quoin evidence retention/audit, ix-flow run and decision
state, portable verification contracts, cli-agent-evals host mechanics, and
native producer execution remain outside this package and under their current
owners. Engineering Assurance qualifies the versions and observable interfaces
it consumes, but does not thereby take ownership of their implementations or
impose its complete internal qualification process on those repositories.

Host protocols use an `engineering-assurance.<capability>/v1` discriminator,
read one declared input, write one structured result to stdout, and write only
diagnostics to stderr. Unknown versions and malformed inputs fail closed. The
CLI never derives a verdict from arbitrary producer stdout or creates an
authoritative evidence record.

The current ix-flow custom-invariant interface loads JavaScript, and the current
cli-agent-evals suite interface loads JavaScript/MJS. Full removal therefore
depends on reviewed external-provider interfaces in those hosts. This repository
will implement no unapproved shim to evade those gates.

## Migration order

1. Establish the Rust package, versioned types, errors, and canonicalization.
2. Port pure compatibility, semantic validation, projection, and fixture logic.
3. Port discovery, onboarding, run coordination, and workflow invariants.
4. Port evaluations, report aggregation, package checks, rights checks, and
   integration gates.
5. Migrate each consumer and external host against parity evidence.
6. Remove legacy executable paths only after the replacement passes at the same
   candidate revision.

## Consequences

- The repository gains a native build artifact while retaining its skill,
  schema, fixture, and discovery content.
- Python and JavaScript remain temporarily during an additive parity phase; that
  coexistence is migration state, not an approved final architecture.
- ix-flow and cli-agent-evals host work must be separately reviewed by their
  owners before this migration can remove the current modules.
- Engineering Assurance's strictest qualification obligations remain focused on
  its Rust domain package, CLI, and integration adapters. External dependencies
  provide separately governed contracts and evidence rather than being absorbed
  into the Engineering Assurance implementation or evidence population.
- The accepted corpus and historical evidence remain byte-identical and
  read-only throughout the migration.
- Assurance artifact schemas remain versioned public contracts. A Rust rewrite
  cannot preserve compatibility merely by reproducing the current validator if
  that validator rejects the active artifacts shipped by its own corpus or a
  recorded Quire consumer.

## Revisit triggers

Reopen this decision if a required host cannot accept a structured external
provider, an accepted portable contract cannot be consumed without copying its
ownership, Rust parity changes an existing digest domain, or a required
dependency cannot support the repository's reviewed MSRV and rights boundary.
