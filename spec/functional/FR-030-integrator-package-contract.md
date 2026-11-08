---
id: FR-030
title: "Write and strictly read the versioned integrator package"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/US-006"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
---

# FR-030: Write and strictly read the versioned integrator package

## Description

Engineering Assurance SHALL expose, behind the `integrator-package` Cargo
feature, a Rust-consumable writer and strict reader for the shared integrator
package: the source-release facts of one candidate plus the obligations an
adopter owns. The module performs no I/O. Candidate-bound publication is
[FR-031](./FR-031-integrator-package-publication.md).

## Inputs

- Writer: an `IntegratorPackage` holding a candidate identity, an attempt
  identity, source facts, and adopter subjects with their obligations.
- Reader: bytes.

## Outputs

- Writer: the canonical RFC 8785 bytes of the package, or a typed refusal.
- Reader: the `IntegratorPackage`, or a typed refusal naming whether the bytes
  were too large, not a package, a downgraded, newer or foreign protocol,
  malformed against the contract, not canonical, or in breach of a package rule.

## Behavior

- The package SHALL carry the protocol `engineering-assurance.integrator-package/v1`.
  Source facts SHALL each carry an identity, a kind, a state, the authority that
  stated them, a statement, an optional normalized value and their sources.
  The kinds are assumption, limitation, exception, negative result, review,
  decision, profile, corpus, configuration, supersession, license, reuse right,
  provenance and verification artifact. The states are established, open,
  negative, incomplete, conditional, conflict and refused; none implies another.
- A package SHALL contain at least one fact of every kind. A kind that has not been examined is stated
  as an incomplete fact, so absence is never a valid way to say "not known".
- An established fact SHALL carry at least one source, and an established
  license or reuse right SHALL carry its normalized value. A fact that is not
  established SHALL NOT carry a normalized value.
- Adopter subjects SHALL each carry an intended use, a configuration and at
  least one obligation. An obligation is open, or completed with the adopter
  party it is attributed to and its response. The package SHALL have no
  acceptance, approval or disposition field; a completed obligation is the
  adopter's own input, carried and never checked, accepted or promoted. The
  reader SHALL list open obligations separately from completed ones.
- The package SHALL state its fact and subject counts. Facts, subjects and
  obligations SHALL be ordered by identity with no duplicates.
- Identifiers SHALL be 1 to 128 bytes of ASCII lowercase letters, digits, `.`,
  `_` and `-`, neither starting nor ending with `.`, so each is a single path
  component that cannot alias another on a case-insensitive filesystem. Text SHALL be
  non-empty, free of leading and trailing whitespace and at most 16 KiB; a package SHALL hold at
  most 4096 facts, 256 subjects, 1024 obligations per subject and 64 sources per
  fact, and at most 8 MiB.
- The reader SHALL accept only the canonical encoding of a package of the one
  supported protocol version.
- The reader SHALL refuse unknown fields and variants, duplicate keys, other
  protocol versions (older as a downgrade, newer as unsupported, other names as
  unknown) and any breach of the rules above.
- The reader SHALL NOT default, repair, reorder or upgrade any content, and
  SHALL NOT use a digest: integrity is the byte equality of the input with the
  canonical re-encoding of what was parsed.
- The writer SHALL apply the same rules.
- The writer SHALL read its own output back through the reader, so it cannot
  emit bytes the reader refuses.
- The module SHALL be usable with only the `integrator-package` feature; it
  adds no dependency beyond serde, serde_json, serde_json_canonicalizer and
  thiserror (and the filesystem publisher of FR-027).

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-030-CON-1 | The contract SHALL NOT carry or derive an adopter acceptance, qualification, certification or release decision. | Security | Test (TC-220) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-030-AC-1 | A package with one fact of every kind and two adopter subjects reads back equal to what was written; the written bytes are deterministic and independent of the order the inputs were supplied in. | Test (TC-219) |
| FR-030-AC-2 | Open, negative, incomplete, conditional, conflict and refused facts, an open obligation and an attributed completed obligation survive the round trip unchanged; open obligations are listed apart from completed ones; an acceptance, approval or disposition member, or an "accepted" obligation state, is refused. | Test (TC-220) |
| FR-030-AC-3 | The reader refuses an unknown field at every level, an unknown kind or state, an invalid identifier, a duplicate key, extra whitespace or a trailing newline, unordered facts, input that is not JSON or not an object, and input over the size bound. | Test (TC-221) |
| FR-030-AC-4 | A protocol one version below, one above, of a malformed version, or of another name is refused as a downgrade, as unsupported, and as unknown respectively. | Test (TC-222) |
| FR-030-AC-5 | A deleted fact (with or without its count), a deleted subject, a duplicate identity, a fact made established without a source, an established license without its value, a value on a fact that is not established, blank or padded text, a completed obligation with blank attribution, and a subject with no obligations are each refused. | Test (TC-223) |
| FR-030-AC-6 | The writer refuses a package with a missing kind, a stale count, a protocol other than v1, or oversize text, and returns the bytes of a valid one only after the reader accepted them. | Test (TC-224) |
| FR-030-AC-7 | A downstream crate that enables only `integrator-package` with default features off compiles, writes, publishes and reads a package, and resolves none of the CLI, archive, YAML, regex, schema or source-audit dependencies. | Test (TC-231) |

## Dependencies

- **Upstream**: [FR-014](./FR-014-versioned-rust-boundary.md) for the Cargo
  feature discipline; [FR-027](./FR-027-atomic-publish.md) for publication.
