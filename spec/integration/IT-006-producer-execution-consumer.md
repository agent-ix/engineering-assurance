---
id: IT-006
title: "A real consumer uses the bounded producer-execution library"
type: IT
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "verifies"
---

# IT-006: A real consumer uses the bounded producer-execution library

## Objective

Verify that `quire-verification` can execute a fictional producer through the
public Engineering Assurance Rust library and receive its own typed observation
without adding a local process runner, CLI-output parser, or evidence store.

## Target Integration

The boundary is a Rust-to-Rust dependency from a `quire-verification` synthetic
consumer to the accepted `engineering_assurance::producer_execution` API. The
producer is a deterministic fictional executable retained only by the test.

## Preconditions

- Both repositories select exact Rust 1.98.1.
- The consumer pins the accepted Engineering Assurance revision.
- A temporary capability root contains the fictional bounded input and output
  fixtures, while the exact producer executable is opened and executed through
  one retained no-follow descriptor.

## Inputs

- One valid qualification-case request/context identity projected into a
  producer-execution request with exact adapter and confinement bindings.
- One consumer-owned response adapter and typed observation.
- Null and retained-input stdin cases; one declared output artifact; and
  invalid digest, unavailable executable, timeout, malformed response,
  cancellation, admitted/rejected non-zero exit, and containment-breach
  variants of the same fictional case.

## Test Procedure

1. Compile the synthetic consumer against the accepted Engineering Assurance
   revision.
   - IT-006-SC-01: the consumer uses the public Rust library and contains no
     direct process launch or Engineering Assurance CLI-output parser.
2. Execute the valid fictional case through the shared executor.
   - IT-006-SC-02: exactly one typed completed result preserves the JCS request,
     caller, adapter and producer identities and carries bounded raw evidence,
     validated artifact references and the consumer-owned observation.
3. Execute each non-completion variant.
   - IT-006-SC-03: every variant returns its distinct executor state, only
     admitted exits reach the adapter, non-completed states carry no typed
     observation, and no ordinary descendant or invocation-owned state remains;
     the escaping-descendant case returns `containment_failure` without a
     full-tree-containment claim.
4. Inspect the consumer and provider boundaries.
   - IT-006-SC-04: neither repository introduces a duplicate evidence store,
     Quoin record schema, generic stdout verdict scraper, or domain oracle in
     Engineering Assurance.

## Expected Results

The real consumer compiles and executes the fictional producer only through the
shared Rust boundary. Completed and non-completed outcomes remain typed and
identity-bound, while domain interpretation and evidence retention stay with
their authoritative owners.

## Metadata

- Priority: P0
- Target Integration: Engineering Assurance producer execution to quire-verification qualification case
- Automation: local-only automated contract test

## Dependencies

The provider implementation and consumer request/result contracts must both be
available at accepted revisions. No hosted CI or public package publication is
required.

## Notes

This test does not qualify the fictional producer and does not standardize fuzz,
mutation, catalogue, advice, or other domain response semantics.

## Traceability

This integration verifies [FR-019](../functional/FR-019-bounded-producer-execution.md)
and preserves the ownership constraint in
[NFR-004](../non-functional/NFR-004-no-parallel-assurance-framework.md).
