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
- A temporary working root contains the fictional producer and bounded input
  fixture with their exact recorded SHA-256 digests.

## Inputs

- One valid qualification-case request projected into a producer-execution
  request.
- One consumer-owned response adapter and typed observation.
- Invalid digest, unavailable executable, timeout, malformed response,
  cancellation, and non-zero-exit variants of the same fictional case.

## Test Procedure

1. Compile the synthetic consumer against the accepted Engineering Assurance
   revision.
   - IT-006-SC-01: the consumer uses the public Rust library and contains no
     direct process launch or Engineering Assurance CLI-output parser.
2. Execute the valid fictional case through the shared executor.
   - IT-006-SC-02: exactly one typed completed result preserves the request and
     producer identities and carries the consumer-owned observation.
3. Execute each non-completion variant.
   - IT-006-SC-03: every variant returns its distinct executor state, carries no
     typed observation, and leaves no descendant or invocation-owned state.
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

