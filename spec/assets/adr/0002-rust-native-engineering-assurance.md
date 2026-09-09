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

Engineering Assurance currently implements its first-party behavior across
Python, JavaScript, and MJS modules. This decision changes that implementation
boundary without changing the architecture or taking ownership from Quire,
Quoin, ix-flow, cli-agent-evals, portable contract providers, or native domain
producers.

Engineering Assurance owns its domain-specific tools and policy. It does not
own every service, contract, producer, project, or artifact that those tools
may operate with. Each project remains independent and owns its own assurance
configuration, artifacts, and qualification priorities.

## Decision drivers

1. Put first-party Engineering Assurance production and qualification semantics
   in Rust.
2. Preserve observable behavior, including non-success and refusal paths.
3. Keep shared Engineering Assurance behavior in one repository-owned package.
4. Avoid a new evidence model, producer runner, consumer registry, or ecosystem
   coordination service.
5. Replace old implementations only after equivalent Rust behavior is locally
   demonstrated.
6. Keep Engineering Assurance qualification isolated from the repositories and
   services it consumes.

## Options considered

| Option | Disposition |
| --- | --- |
| Add a Rust library crate and native CLI to this repository | Selected. It preserves the existing owner and supplies one implementation boundary. |
| Move the behavior into quire-verification | Rejected. Portable contracts and Engineering Assurance tooling have different owners. |
| Add the behavior to Quoin | Rejected. Quoin owns evidence retention, audit, and reporting, not Engineering Assurance execution. |
| Create another Rust repository | Rejected. No distinct ownership or release boundary justifies it. |
| Keep Python or JavaScript behind a Rust wrapper | Rejected. That would conceal rather than replace the old implementation. |

## Capability boundary

| Capability | Rust disposition |
| --- | --- |
| Compatibility classification and accepted-corpus access | Port the existing behavior into the library; preserve read-only corpus and matrix behavior. |
| Verification vocabulary, reference validation, projection, and fixture generation | Port the existing Engineering Assurance mapping and validation behavior without copying external ownership or creating persisted records. |
| Discovery, onboarding, and run coordination | Port repository-owned behavior; keep Quire and ix-flow authoritative for their domains. |
| Workflow invariants | Port the canonical invariant behavior; integrate through an interface supported by the ix-flow owner. |
| Evaluation model and report aggregation | Port the existing result validation and aggregation behavior. |
| Agent-evaluation scenarios | Port Engineering Assurance scenarios and assertions; keep cli-agent-evals host mechanics external. |
| Repository qualification, package, and rights checks | Port existing checks; host files may remain declarative dispatch. |
| Schemas, manifests, skeletons, specifications, corpora, and generated fixtures | Keep in their suitable data or documentation formats. Generated source fixtures are inert test data. |

## Decision

The existing `engineering-assurance` repository remains the owner of its tools
and policy. Their executable implementations will converge on one Cargo package
named `engineering-assurance`, exporting the `engineering_assurance` Rust
library crate and the `engineering-assurance` CLI binary. These are two targets
of one package, not independently governed products.

The library owns reusable deterministic domain behavior. The CLI owns explicit
filesystem, process, and external-host boundaries and exposes versioned JSON for
machine-facing operations. It does not infer verdicts from arbitrary producer
stdout or persist authoritative evidence.

Schemas, manifests, skeletons, skills, Markdown specifications, corpora, and
generated cross-language fixtures remain data or documentation in their native
formats. Makefiles, package manifests, and workflow files may remain thin host
dispatch. They do not own semantic branches or assertions duplicated from Rust.

Quire validation, Quoin evidence retention and audit, ix-flow run and decision
state, cli-agent-evals host mechanics, portable verification contracts, and
native producer execution remain outside this package. Integration with an
external host uses a supported versioned interface; Engineering Assurance does
not copy or redesign that host.

## Port and cutover order

1. Establish the Rust package, typed errors, versioned interfaces, and required
   canonicalization behavior.
2. Port compatibility, semantic validation, projection, and fixture logic.
3. Port discovery, onboarding, run coordination, and workflow invariants.
4. Port evaluations, aggregation, package checks, rights checks, and repository
   qualification.
5. Update direct Engineering Assurance invocations and host configuration.
6. Delete each replaced implementation after equivalent behavior passes locally
   at the same candidate revision.

## Consequences

- The repository gains a native implementation while retaining its schemas,
  fixtures, skills, and other declarative content.
- Python, JavaScript, and MJS coexist only during the temporary parity phase.
- External host changes remain separately owned and reviewed by those hosts.
- No consumer registry, ecosystem snapshot, cross-repository CI checkout, or
  centralized migration ledger is created.
- Accepted corpus and historical evidence bytes remain unchanged.

## Revisit triggers

Reopen this decision if a required host cannot expose a usable structured
interface, an external contract cannot be consumed without copying its
ownership, canonicalization would change an existing identity domain, or a
required tool demonstrably cannot run on Rust 1.98.1. Formatting differences
and repairable lint findings are not tool incompatibilities.
