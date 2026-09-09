---
id: FR-019
title: "Orchestrate Quoin change assurance through Rust"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: "requires"
---

# FR-019: Orchestrate Quoin change assurance through Rust

## Description

Engineering Assurance SHALL replace the eight repository-local
`assurance_chain.py` programs with one native Rust orchestration capability
that consumes declarative repository inputs and invokes only the reviewed
Quoin change-assurance operations. Quoin remains the authority for record,
attestation, intake, audit, receipt, and verification schemas and storage.

## Inputs

- A request carrying the
  `engineering-assurance.quoin-chain/v1` discriminator.
- An explicit repository root, candidate revision, Quoin store root, and
  declarative chain definition whose exact bytes and digest are identified.
- Already-produced result files with explicit path, digest, media type,
  producer identity/version, proof identity, and declared result state.
- Explicit decision and audit inputs when the requested Quoin receipt requires
  them.
- The accepted Quoin component version and packaged contract digests from the
  Engineering Assurance compatibility boundary.

## Outputs

- One bounded `engineering-assurance.quoin-chain-result/v1` machine result.
- An ordered record of every attempted Quoin operation, exit status, accepted
  structured response identity, and any Quoin-owned outcome without renaming
  or aggregation.
- A distinguishable refusal result naming the last completed operation and the
  operation that was not attempted.

## Behavior

- The capability SHALL complete preflight of the request, path confinement,
  candidate revision, input digests, allowed operation sequence, and accepted
  Quoin identity prior to invoking Quoin or permitting a write.
- The capability SHALL invoke only the accepted `quoin` binary.
- The capability SHALL accept only the required ordered subsequences of
  `change-assurance seal-record`, `seal-attestation`, `intake`, `receipt`, and
  `verify-receipt`.
- The capability SHALL treat decision and audit reports as explicit,
  pre-produced receipt inputs rather than executable chain operations.
- The capability SHALL NOT run a producer.
- The capability SHALL require every producer output to exist and match its
  declared digest immediately prior to the consuming Quoin operation.
- The capability SHALL parse the declared Quoin machine response for each
  operation, verify its expected schema/version and candidate or digest
  binding, and carry Quoin outcome tokens unchanged.
- The capability SHALL report `incomplete`, `invalid`, `unavailable`, stale,
  malformed, and tampered states distinctly.
- The capability SHALL NOT infer a human decision or an overall assurance
  verdict.
- When an operation fails, times out, is cancelled, returns a malformed or
  mismatched response, or observes changed input bytes, the capability SHALL
  terminate the child.
- When such a failure occurs, the capability SHALL invoke no later operation
  and report every operation already completed.
- The capability SHALL NOT claim atomic rollback of a write Quoin completed
  prior to the failure.
- The capability SHALL restrict Quoin writes to the explicit store root.
- The capability SHALL leave historical repository evidence and corpus bytes
  read-only.

## Operation Contract

| Operation | Required prior state | Multiplicity |
| --- | --- | --- |
| `seal-record` | Complete preflight; no operation attempted | Exactly one |
| `seal-attestation` | Accepted sealed record; declared pre-produced proof result present and digest-matched | Exactly one per declared proof |
| `intake` | Corresponding accepted sealed attestation and unchanged proof-result bytes | Exactly one per declared proof |
| `receipt` | Accepted sealed record, every declared selection retained, and explicit decision/audit inputs digest-matched | Exactly one |
| `verify-receipt` | Receipt response accepted and bound to the candidate plus selected digests | Exactly one |

The capability SHALL reject a skipped prerequisite, an undeclared proof,
selection, or input, an extra operation, and any repetition outside the
per-proof attestation/intake rows.

## Error Conditions

An unknown protocol, an escaping or aliased path, a dirty or mismatched
candidate revision, an absent or changed input, an unaccepted Quoin version or
contract digest, an unsupported/reordered/repeated operation, arbitrary
arguments or executables, a malformed response, a response bound to another
candidate or digest, timeout, cancellation, signal, and a changed historical
byte each produce a distinguishable refusal.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-019-CON-1 | Engineering Assurance SHALL NOT define, persist, or reinterpret a Quoin evidence, audit, receipt, or decision schema. | Responsibility | Test |
| FR-019-CON-2 | The capability SHALL NOT execute a producer or recover a verdict from arbitrary stdout or stderr. | Responsibility | Test |
| FR-019-CON-3 | The capability SHALL NOT create a repository-local generic evidence store, history, envelope, manifest, or canonical form. | Architecture | Test |
| FR-019-CON-4 | The capability SHALL NOT infer, synthesize, or rename a human decision or Quoin-owned outcome. | Responsibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-019-AC-1 | A valid declarative request over already-produced inputs invokes the fixed Quoin operation sequence, validates every structured response, and emits one bounded result carrying the exact Quoin outcomes and completed-operation order. | Integration (TC-130) |
| FR-019-AC-2 | Unknown protocols, escaping or aliased paths, mismatched candidate revisions, missing or changed input bytes, unaccepted Quoin identities, arbitrary executables/arguments, and unsupported operation graphs fail before the first invocation or write. | Property (TC-131) |
| FR-019-AC-3 | Operation failure, timeout, cancellation, signal, malformed response, and response-binding mismatch terminate the child, emit no success result, invoke no later operation, and emit one bounded refusal identifying every operation Quoin completed before refusal. | Property (TC-132) |
| FR-019-AC-4 | The Rust capability matches the retained success and adverse-case observations of all eight current `assurance_chain.py` consumers at pinned revisions, preserving pass, fail, unavailable, not-computed, malformed, stale, tampered, and incomplete without rewriting historical bytes. | Integration (TC-133) |
| FR-019-AC-5 | Static ownership and call-surface audits find only the accepted Quoin executable and the five declared change-assurance subcommands, no audit-report producer invocation, no other producer invocation or stdout verdict recovery, no copied Quoin schema/store/canonicalization, and no human-decision inference (CON-1..CON-4). | Test (TC-134) |

## Dependencies

- **Upstream**: FR-014 supplies the bounded CLI protocol and child lifecycle;
  FR-015 supplies identity, compatibility, state, and canonical-byte behavior;
  the accepted compatibility matrix identifies Quoin and its contract bytes.
- **External authority**: Quoin owns all change-assurance schemas, persistence,
  audit, receipt construction, verification, and outcome semantics.
- **Downstream**: FR-018 and quire-research #60 may migrate the eight consumer
  scripts only after this capability and the registry snapshot pass at their
  exact candidate revisions.
