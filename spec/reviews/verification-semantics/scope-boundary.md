---
id: SR-018
title: "Scope and boundary review of verification semantics"
type: SpecReview
analysis: scope-boundary
scope: "ADR-001; FR-008..FR-010; NFR-004"
review_set: all
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-008"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/NFR-004"
    type: reviews
---

# Scope and boundary review of verification semantics

## Summary

Engineering Assurance owns names and type fit only. Quire remains non-executing
and owns static facts; Quoin remains non-executing and owns transcription,
retention, integrity, audit, and reports; native tools own execution and domain
results; ix-flow owns human decisions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Resolved: the original ticket wording could be read as owning new evidence records; ADR-001 and NFR-004 explicitly prohibit that interpretation. | ADR-001; NFR-004 |

## Out of Scope

Universal execution, standalone `quire-evidence`, generic stdout verdict
scraping, a new evidence envelope/store, repository migrations, and automated
certification or sufficiency decisions.

