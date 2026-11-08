---
id: IT-007
title: "A real consumer writes, publishes and reads an integrator package"
type: IT
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-030"
    type: "verifies"
  - target: "ix://agent-ix/engineering-assurance/FR-031"
    type: "verifies"
---

# IT-007: A real consumer writes, publishes and reads an integrator package

## Objective

Verify that a separate Rust crate can use the integrator-package writer, strict
reader and store through the public Engineering Assurance library alone, so a
source library such as tl-syntax adopts the shared contract without a local
format, reader or publisher.

## Target Integration

A Rust-to-Rust dependency from a synthetic consumer crate to the
`engineering_assurance::integrator_package` and `integrator_package_store`
APIs, selected with default features off and only `integrator-package`.

## Preconditions

The consumer is a temporary crate built offline against this checkout. The
store root is a fresh temporary directory. All identities and facts are
fictional.

## Inputs

One package with a fact of every kind, all stated incomplete, for one candidate
and attempt, published twice to the same store.

## Test Procedure

1. Build the consumer against the version under test.
   - IT-007-SC-01: the consumer compiles and resolves none of the CLI, archive,
     YAML, regex, schema or source-audit dependencies.
2. Run the consumer to publish, republish and read the package.
   - IT-007-SC-02: the first publish and the second report different outcomes
     (written, then already present), and the read-back equals the input.

## Expected Results

The consumer uses the shared contract only through the public library and
receives exactly the package it wrote.

## Metadata

- Priority: P0
- Target Integration: Engineering Assurance integrator package to a source-library adapter
- Automation: local-only automated contract test

## Dependencies

FR-030 and FR-031.
