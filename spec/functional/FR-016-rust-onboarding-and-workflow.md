---
id: FR-016
title: "Run onboarding and workflow invariants through Rust"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/US-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-001"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-005"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: "requires"
---

# FR-016: Run onboarding and workflow invariants through Rust

## Description

Engineering Assurance SHALL perform repository discovery, bounded onboarding,
ix-flow coordination, and canonical workflow-invariant evaluation through the
Rust library and CLI.

## Inputs

- A selected repository root and onboarding boundary.
- An `engineering-assurance.onboarding/v1` request containing an absolute
  repository root, an absolute installed Engineering Assurance module root, a
  Quire executable name or absolute path, optional decision-boundary and owner
  strings, and an optional requested artifact, justification, confined target,
  and frontmatter object.
- Canonical skill and workflow definitions.
- Versioned ix-flow run state.
- An `engineering-assurance.workflow-host/v1` request containing one
  `start_or_resume` or `decide` operation, absolute state and canonical-skill
  directories, an ix-flow executable name or absolute path, and a binding with
  run, repository, workflow, workflow-version, decision-boundary, and
  decision-owner identities. A `decide` request additionally carries an absent,
  `accept`, or `reject` choice.
- An `engineering-assurance.workflow-invariants/v1` request containing an
  ordered non-empty invariant-name list, one ix-flow instance snapshot, and an
  explicit RFC 3339 evaluation instant.
- Explicit human decision actions.

## Outputs

- The existing inventory and bounded onboarding proposal.
- An `engineering-assurance.onboarding-result/v1` result containing the complete
  sorted inventory, one of `needs-input`, `no-applicable-work`,
  `needs-human-selection`, `reuse`, or `authored`, the retained recommendation,
  and an artifact path only for `reuse` or `authored`.
- An `engineering-assurance.workflow-invariants-result/v1` result containing
  one ordered typed outcome per requested invariant. Each failed outcome carries
  the invariant name, its stable failure code, and structured details.
- An `engineering-assurance.workflow-host-result/v1` result containing either a
  typed snapshot with run, workflow, workflow-version, phase, state-version,
  next actions, and open gates, or one typed terminal decision event with run,
  workflow, workflow-version, owner, choice, outcome, and timestamp.
- Resumable ix-flow state with attributed human terminal events.

## Behavior

- Preserve FR-001 inventory-before-proposal behavior.
- Inventory regular `.md`, `.json`, `.yaml`, and `.yml` files without following
  symlinks; exclude `.git`, `node_modules`, and `__pycache__`; keep decisions,
  measurements, assurance artifacts, evidence references, producer
  configurations, and unresolved inputs as separate lexically sorted
  collections.
- Parse Markdown frontmatter with the maintained Rust YAML implementation and
  validate recognized Engineering Assurance artifacts through Quire using the
  caller-selected installed module root. Quire remains the artifact validator;
  Rust does not copy its grammar or acceptance rules.
- Engineering Assurance SHALL require artifact identity to be one explicit
  top-level string `type` value and treat duplicate mapping keys or YAML merge
  keys as malformed inventory inputs that cannot select, validate, reuse, or
  author an artifact.
- Preserve existing validated artifacts for reuse; preserve malformed,
  conflicting, or duplicate applicable artifacts for explicit human selection;
  and author no generic artifact without a non-empty justification, confined
  target, and frontmatter object.
- Render an authored artifact only from the requested installed skeleton,
  validate a same-directory staged file through Quire, sync it, and publish the
  complete inode atomically without replacing an existing destination. A
  refusal removes the staged file and leaves existing repository bytes
  unchanged.
- Delegate run state, transitions, and terminal decision history to ix-flow.
- Before any workflow mutation, read the exact ix-flow release pin from the
  compatibility matrix and require `ix-flow --version` to match it. Parse
  ix-flow's JSON `ok`, `state`, `error`, run data, gates, and events rather than
  treating process exit status or diagnostic prose as the contract.
- For a new run, invoke ix-flow with the canonical skill and no gate-mode or
  per-transition override, then add exactly one complete `run_binding` item.
  If interruption leaves a newly created run without that item, repair it only
  when ix-flow reports the requested workflow/version, its initial phase,
  state-version zero, no other item, and only the creation event; otherwise
  refuse the mismatch without mutating the run.
- For an existing run, require the workflow name, workflow version, and exactly
  one complete `run_binding` item to equal the request before invoking resume.
- Before mutating an existing run and before emitting a successful snapshot or
  terminal decision, invoke ix-flow's `verify` command and require its event
  chain result to be intact. Engineering Assurance SHALL NOT recompute the
  chain or inspect the state file itself.
- When no terminal choice is supplied, return the current decision-ready
  snapshot without acknowledging or advancing a terminal gate.
- When an explicit choice is supplied at `decision_ready`, require ix-flow to
  report the selected terminal transition as `hitl`, defer it to exactly one
  human gate, acknowledge that gate as the bound owner with the selected choice,
  and then ask ix-flow to advance. A retry after interruption at creation, gate deferral,
  acknowledgement, or terminal advance SHALL return or complete the same bound
  result without inventing a second binding or terminal decision event.
- Refuse an opposite terminal outcome, a non-decision-ready phase, an ambiguous
  gate, an automatic gate event, or a terminal event not attributed to the
  bound owner and selected choice without attempting a replacement decision.
- Bound each ix-flow invocation to 60 seconds and at most 16 MiB each of stdout
  and stderr. On timeout or overflow, terminate the child and refuse its
  response; do not parse a truncated envelope.
- If a read-only invocation fails before mutation, leave prior history
  unchanged. If a mutating ix-flow invocation times out, overflows, or returns
  malformed output after it may have committed, report an indeterminate host
  outcome without claiming rollback; on retry, reconcile from `status` before
  issuing another mutation.
- Pass ix-flow arguments directly to the executable without a shell. Treat
  next-action command strings returned by ix-flow as display data and never
  execute them.
- Return all applicable invariant failures in deterministic order.
- Evaluate exception expiry against the request's explicit evaluation instant;
  the reusable Rust library SHALL NOT read the system clock.
- Reject an unknown invariant name rather than treating it as passed or
  omitting it from the result.
- Reject unknown workflow, phase, or protocol versions.
- Leave a decision-ready run non-terminal when human input is absent.
- Retain the pilot path only as a temporary compatibility alias until the
  canonical Rust path passes locally at the same candidate revision.

## Error Conditions

Unavailable Quire or ix-flow tools, incompatible ix-flow versions, malformed or
oversized host responses, host timeouts, run-binding mismatches, invalid
transitions, automatic or conflicting terminal decisions, escaping
artifact targets, unknown invariant names, malformed instance snapshots, and
invalid evaluation instants fail closed without overwriting artifacts or run
history. Onboarding machine refusals use `onboarding_request_invalid`,
`unsupported_onboarding_protocol`,
`onboarding_root_invalid`, `onboarding_module_root_invalid`,
`onboarding_target_invalid`, `onboarding_inventory_failed`, or
`onboarding_publication_failed`. Workflow-host refusals use
`workflow_host_request_invalid`, `workflow_binding_invalid`,
`workflow_transition_invalid`, `workflow_decision_conflict`,
`ix_flow_unavailable`, `ix_flow_version_incompatible`,
`ix_flow_command_failed`, `ix_flow_response_invalid`, or
`ix_flow_outcome_indeterminate`; invariant-evaluation refusals use
`workflow_invariant_request_invalid`, `workflow_invariant_unknown`, or
`workflow_binding_invalid` as applicable. Quire unavailability during inventory
is retained as an invalid artifact validation where the artifact type is known;
Quire unavailability during staged publication refuses publication.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-016-CON-1 | Engineering Assurance SHALL NOT reimplement ix-flow run state or human-gate mechanics. | Responsibility | Test |
| FR-016-CON-2 | A foreign-language host bridge requires an explicit owner disposition before inclusion. | Architecture | Review |
| FR-016-CON-3 | The reusable Rust invariant evaluator SHALL NOT access the filesystem, environment, process table, network, or system clock. | Determinism | Test |
| FR-016-CON-4 | Reusable Rust onboarding parsing, rendering, and recommendation logic SHALL be I/O-free; the CLI adapter exclusively owns selected-root traversal, Quire invocation, staging, synchronization, and artifact publication. | Responsibility | Test |
| FR-016-CON-5 | The Rust workflow-host adapter SHALL invoke only the supported ix-flow CLI for workflow lifecycle operations. | Responsibility | Test |
| FR-016-CON-6 | The Rust workflow-host adapter SHALL NOT parse, edit, replace, or independently persist ix-flow state files. | Responsibility | Test |
| FR-016-CON-7 | The Rust workflow-host adapter SHALL NOT invoke a shell or execute ix-flow-provided next-action text. | Security | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-016-AC-1 | For the accepted onboarding fixtures, Rust returns the declared complete sorted inventory, status, recommendation, and artifact path for existing, absent, conflicting, malformed, unavailable, unjustified, incomplete-boundary, and valid-authoring cases without executing another implementation; duplicate-key or merge-key artifact identities remain malformed; absolute or parent-traversing targets, symlink escapes, existing destinations, invalid staged artifacts, malformed requests, and unsupported artifact types refuse without publishing bytes. | Test (TC-105) |
| FR-016-AC-2 | For every canonical invariant and focused valid boundary fixture, Rust and the retained reference produce the same complete ordered outcome set at the same explicit evaluation instant; unknown names and malformed Rust requests are refused before an outcome is emitted. | Test (TC-106) |
| FR-016-AC-3 | Against the exact accepted ix-flow pin, start, pristine unbound-run recovery, interruption/resume, explicit acceptance, explicit rejection, missing choice, repeated same choice, opposite choice, invalid transition, run-binding mismatch, broken event chain, unavailable/incompatible host, malformed/oversized/timed-out response, and automatic-gate evidence preserve ix-flow-owned state and human-gate behavior; every successful Rust result is typed and follows an intact ix-flow chain verification, every pre-mutation refusal leaves prior history unchanged, and every ambiguous post-mutation outcome is reported as indeterminate and reconciled on retry. | Test (TC-107) |
| FR-016-AC-4 | Canonical and pilot invocations pass through Rust before either legacy JavaScript path is removed. | Test (TC-108) |

## Dependencies

- **Upstream**: FR-001 through FR-005, FR-014, FR-015, and an ix-flow interface
  supported by the ix-flow owner.
- **Compatibility gate**: ix-flow 0.2.3 is the accepted released pin and
  retains the required JSON envelope, optimistic concurrency, resume, and
  human-gate contracts. [FR-012](./FR-012-pinned-compatibility-matrix.md)
  records Peter Krenesky's 2026-09-10 acceptance after TC-107 passed locally;
  any enforcing migration or legacy removal still requires its own acceptance
  evidence.
- **Downstream**: FR-018 governs removal of the old paths.
- **Sequence**: FR-016-AC-2 is an independently reviewable additive slice. It
  does not authorize the host bridge or JavaScript removal required by
  FR-016-AC-4.
- **Sequence**: FR-016-AC-1 is independently reviewable from ix-flow lifecycle
  and invariant-host integration. It does not authorize removal until FR-018's
  same-revision gate passes.
- **Sequence**: FR-016-AC-3 ports the existing lifecycle adapter and does not
  approve the JavaScript invariant-provider bridge needed by FR-016-AC-4.
