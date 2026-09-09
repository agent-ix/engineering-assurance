---
id: FR-011
title: "Accept a real compatibility fixture corpus as the migration gate"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-005"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-010"
    type: "extends"
---

# FR-011: Accept a real compatibility fixture corpus as the migration gate

## Description

An accepted corpus of real legacy records, real producer output, and one exact
Quire-to-Quoin receipt chain SHALL be retained by `agent-ix/qa-corpus` and
pinned by Engineering Assurance as a submodule read in place.

The corpus is held there rather than here because it retains real governance
evidence, and this repository's publication boundary permits fictional fixtures
only.

Engineering Assurance SHALL enforce that corpus as the implementation gate for
the eight repository migrations.

The gate SHALL read the corpus offline.

Each constructed case SHALL record the exact edit that produced it.

## Inputs

- Immutable PGM-01 v1 evidence retained by `agent-ix/quire-contract-ir`, read at
  a named `origin/main` revision and never written to.
- Real producer output from governed producers, including `agent-ix/quire-code-rs`,
  the contract conformance corpus, an external engine, an agent-evaluation
  measurement, and a static-scan diagnostic.
- One Quire static export, and the Quoin change-assurance record, proof
  attestation, retained output, decision history, audit report, and verification
  receipt produced from it.

## Outputs

- `corpus/compatibility/` in the pinned submodule, containing the retained
  bytes, a corpus index recording every digest, origin, derivation, and expected
  outcome, and the stated limitations of the set.
- A recorded gitlink naming the exact reviewed corpus commit.
- An enforcing test gate over that corpus, and cross-language projections of its
  case index committed here.

## Behavior

- Every retained artifact SHALL carry the SHA-256 of its own bytes, and every
  real legacy case SHALL additionally match the digest its source repository
  recorded for it.
- The corpus SHALL cover the legacy, current, malformed, unavailable,
  not-computed, failed, stale, and tampered states, naming any it lacks.
- The corpus author SHALL derive a case that does not exist in real history from
  real bytes by one named edit.
- Each derived case SHALL record that edit and the reason for it.
- The corpus SHALL state which states required derivation and why.
- No non-success case SHALL map to a clean read or to a passed check.
- A real legacy record's mapping SHALL preserve source revision, repository,
  producer identity and source revision, execution environment, and result
  states, and SHALL leave what the legacy record could not carry named as
  unmapped rather than filled in.
- The current-model receipt SHALL validate against Quoin's packaged receipt
  schema, as retained beside it, and SHALL bind the exact record, attestation,
  and retained-output digests the chain carried.
- The chain SHALL record the exact tool versions and source revisions it used,
  and SHALL state plainly which of them are source revisions rather than
  released artifacts.
- Reading the corpus SHALL change no byte of it, execute no producer, and
  contact no repository.
- The gate SHALL refuse an uninitialized corpus rather than skipping it.
- The checked-out corpus SHALL equal the gitlink recorded in this repository.

## Error Conditions

A retained artifact whose bytes no longer match its recorded digest, a real
legacy case that no longer matches the digest its source repository recorded, a
missing required state, a constructed case with no recorded derivation, and a
non-success case that reads as clean each fail the gate by name. The gate never
reports a count in place of the failing case.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-011-CON-1 | The corpus reader SHALL execute no subprocess, open no socket, and write no file. | Architecture | Test |
| FR-011-CON-2 | Source evidence trees SHALL be read at a named revision and never written to. | Integrity | Test |
| FR-011-CON-3 | The corpus SHALL make no claim about the live state of the source repositories. | Responsibility | Inspection |
| FR-011-CON-5 | This repository SHALL reference the corpus by pinned gitlink only, holding no retained operational evidence of its own. | Integrity | Test |
| FR-011-CON-4 | No retained artifact SHALL be executable. | Integrity | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-011-AC-1 | Every retained artifact matches its recorded digest, and every real legacy case matches the digest its source repository recorded (CON-2). | Test (TC-069) |
| FR-011-AC-2 | The corpus covers all eight required states, and every constructed case records its edit and its reason. | Test (TC-070) |
| FR-011-AC-3 | Every legacy case maps to the outcome the corpus records, with its required mappings preserved and a stated limitation. | Test (TC-071) |
| FR-011-AC-4 | No failed, unavailable, not-computed, malformed, or tampered case reads as clean or reports a passed check. | Test (TC-072) |
| FR-011-AC-5 | A real legacy record preserves revision, repository, producer identity and revision, and environment, keeps inconclusive distinct from passed, and names what it could not carry. | Test (TC-073) |
| FR-011-AC-6 | The retained receipt validates against Quoin's packaged schema and binds the exact record, attestation, and retained-output digests of the chain, whose tools are pinned and whose unreleased side is stated. | Test (TC-074) |
| FR-011-AC-7 | Every producer case names a real producer, a source path, and one shared-model concept, spanning at least two languages and four concepts, including the governed `quire-code-rs` case at a pinned revision. | Test (TC-075) |
| FR-011-AC-8 | Reading and mapping the whole corpus changes no byte, no artifact is executable, and the reader reaches for no subprocess, socket, or write (CON-1, CON-4). | Test (TC-076) |
| FR-011-AC-9 | The committed corpus reproduces from its recorded sources where those sources are checked out, and states plainly when it is skipped. | Test (TC-077) |
| FR-011-AC-10 | The corpus is tracked as a gitlink, the checked-out commit equals the recorded pin, and an uninitialized corpus fails rather than passing quietly. | Test (TC-078) |

## Dependencies

- **Upstream**: [FR-008](./FR-008-distinguish-verification-semantics.md),
  [FR-009](./FR-009-preserve-provenance-and-states.md), and
  [FR-010](./FR-010-read-only-compatibility-and-reporting.md); the Quoin
  producer-facing CLI (`agent-ix/quoin#322`).
- **Downstream**: `agent-ix/engineering-assurance#8` turns the chain's pinned
  source revisions into released versions; `agent-ix/engineering-assurance#10`
  cites this corpus as the migration implementation gate.
