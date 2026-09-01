---
id: NFR-004
title: "Do not introduce a parallel assurance framework"
type: NFR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-002"
    type: "constrains"
---

# NFR-004: Do not introduce a parallel assurance framework

## Statement

The verification-semantics package SHALL remain a definition, mapping,
validation, and projection boundary. It SHALL NOT execute producers, scrape
arbitrary stdout for verdicts, persist evidence, define a second generic
envelope or manifest, or infer a human decision.

## Scope

The constraint applies to schemas, mapping code, projections, compatibility
fixtures, packaging, CLI integrations, and reports introduced by issue #5.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Duplicate authoritative record families | 0 | 0 | static ownership/schema audit |
| Producer execution or generic stdout scraping paths | 0 | 0 | static package audit |

## Rationale

The shared migration succeeds only if repositories reuse Quire, Quoin, and
ix-flow instead of replacing eight local frameworks with a ninth one.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-004-AC-1 | The issue #5 package adds no duplicate authoritative record family. | Test (TC-068) |
| NFR-004-AC-2 | The package contains no producer execution, generic stdout verdict scraper, evidence persistence, or automatic human decision path. | Test (TC-059) |

## Verification

Inspect the ownership registry, schema titles and record discriminators, and
the complete package dependency/call surface.

## Dependencies

FR-008 through FR-010 define the constrained implementation boundary.
