---
type: index
title: "PLAN-003 — Rust-native Engineering Assurance"
description: "Contents of the PLAN-003 Rust-native migration bundle."
okf_version: "0.1"
---
# PLAN-003 — Rust-native Engineering Assurance

## Contents

* [PLAN-003: Rust-native Engineering Assurance](./plan.md) - Migration overview, dependency graph, tests, tracks, and gates.
* [TASK-012: Establish the Rust package foundation](./tasks/TASK-012-rust-package-foundation.md) - Existing package and exact-toolchain foundation.
* [TASK-013: Port compatibility classification](./tasks/TASK-013-compatibility-classification.md) - Existing read-only compatibility slice.
* [TASK-014: Port evidence availability and identity](./tasks/TASK-014-evidence-availability-identity.md) - Current evidence-classification slice and review remediation.
* [TASK-015: Implement the versioned CLI boundary](./tasks/TASK-015-versioned-cli-boundary.md) - Protocol, limits, process, and refusal behavior.
* [TASK-016: Port semantic validation, projections, and fixtures](./tasks/TASK-016-semantic-projections-fixtures.md) - Remaining pure shared semantics.
* [TASK-017: Implement the assurance-contract registry](./tasks/TASK-017-assurance-contract-registry.md) - Registry, snapshot, and artifact compatibility.
* [TASK-018: Port onboarding and invariant evaluation](./tasks/TASK-018-onboarding-invariant-core.md) - Host-independent onboarding and invariant core.
* [TASK-019: Integrate the accepted ix-flow host boundary](./tasks/TASK-019-ix-flow-host-integration.md) - Blocked external-provider gate and integration.
* [TASK-020: Port evaluation and repository qualification](./tasks/TASK-020-evaluation-qualification-core.md) - Host-independent evaluation and qualification logic.
* [TASK-021: Integrate the accepted evaluation host boundary](./tasks/TASK-021-evaluation-host-integration.md) - Blocked cli-agent-evals gate and integration.
* [TASK-022: Migrate consumers and retire legacy paths](./tasks/TASK-022-consumer-migration-removal.md) - Same-revision consumer migration and removal.
* [TASK-023: Complete final qualification](./tasks/TASK-023-final-qualification.md) - Containment, traceability, performance, and hosted gates.
* [TASK-024: Implement Quoin chain orchestration](./tasks/TASK-024-quoin-chain-orchestration.md) - Replace eight local chain drivers without absorbing Quoin ownership.
