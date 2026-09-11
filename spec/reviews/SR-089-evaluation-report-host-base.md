---
id: SR-089
title: "Base review — Rust evaluation report host adapter"
type: SpecReview
analysis: base
scope: "FR-017 report/transcript adapter and aggregate-command cutover; TC-129"
review_set: base
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

# Base review — Rust evaluation report host adapter

## Summary

Reviewed the additive Rust report/transcript adapter and aggregate-command
cutover against the QUOIN base checklist. The slice consumes the exact
`cli-agent-evals.report/v1` source contract, preserves the retained aggregate
behavior, confines transcript authority to an explicit workspace root, and
does not claim the live 28-cell or external scenario-provider gates.

## Verdict

**PASS** — the bounded slice is implementable after incorporating every
finding below into FR-017 and TC-129.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | A report-controlled absolute `workDir` could make the adapter hash an arbitrary workstation file | FR-017 Behavior; TC-129 | missing-requirement |
| FND-002 | high | Treating a digest/path pair as retained without checking `transcriptRetention` would admit default-cleanup reports that cannot be reopened | FR-017 Behavior; TC-129 | missing-requirement |
| FND-003 | high | Parsing only the fields consumed by EA would let unknown or contradictory host fields bypass the v1 contract | FR-017 Behavior; TC-129 | missing-requirement |
| FND-004 | medium | Report, result, collection, and transcript populations had no resource ceilings | FR-017 Behavior; TC-129 | missing-requirement |
| FND-005 | medium | Multiple reports could silently combine different model selections for one host or change diagnostics with input order | FR-017 Behavior; TC-129 | missing-requirement |
| FND-006 | high | A passing synthetic report adapter could be misreported as the live 28-cell host/provider acceptance gate | FR-017 Dependencies; TC-109; TC-129 | wrong-requirement |
| FND-007 | medium | Replacing the Python aggregate command without same-revision parity and rollback evidence would make removal irreversible | FR-017 Behavior; TC-129 | missing-requirement |

## Dispositions

- **FND-001 resolved**: one explicit canonical workspace root bounds every
  work directory and transcript; links, special files, and escapes refuse.
- **FND-002 resolved**: only `retained` successful samples with a path and
  digest can produce an envelope.
- **FND-003 resolved**: closed Rust structures own report, result, sample,
  token-usage, and EA result shapes; opaque maps remain host-owned only where
  cli-agent-evals explicitly leaves them extensible.
- **FND-004 resolved**: collection, report-byte, result-count, path, and
  transcript-byte ceilings are exact and covered at equality and one over.
- **FND-005 resolved**: host models must agree and retained outputs are sorted
  independently of supplied report order.
- **FND-006 resolved**: TC-129 is additive adapter evidence only; TC-109 stays
  pending and no live agent evaluation is authorized by this review.
- **FND-007 resolved**: parity, cutover, rollback, and deletion remain ordered
  gates rather than one irreversible change.

## Boundary

This review authorizes synthetic/local Rust adapter work only. It does not
authorize a live agent invocation, hosted CI, new Quire language work, npm
publication, a consumer-local runner, or completion of TC-109.
