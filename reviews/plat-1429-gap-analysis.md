---
id: SR-16414
title: "gap-analysis of the Syft producer request tests (engineering-assurance PR 216)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@e7d8da9; FR-038, FR-039, NFR-006; spec/tests.md; tests/syft_producer_cli.rs; src/"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-038
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-039
    type: references
  - target: ix://agent-ix/engineering-assurance/NFR-006
    type: references
---

# Gap analysis of the Syft producer request tests (engineering-assurance PR 216)

## Summary

Ticket: PLAT-1429. Plan completion: not assessed (planless). `quire matrix --scope .` shows every FR-038 and FR-039 acceptance criterion, FR-038-CON-1 and NFR-006-AC-2 tagged by `tests/syft_producer_cli.rs`. FR-038-AC-5 is tagged on TC-286 (method-without-symbol, Inspection). The Inspection criteria FR-039-AC-7 and NFR-006-AC-1 have no symbol. They were checked by hand: no file under `src/` or `engineering_assurance/` mentions Syft, CycloneDX or `bomFormat`, `Cargo.toml` has no quoin dependency, and the only launch site is the FR-019 executor. No production code, stub or coverage inflation was added. The tests.md rows are truthful: TC-283..TC-287 are ✅ and pass on Linux, and TC-278..TC-282 carry no ✅ and say they need `EA_TEST_SYFT`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The ticket's core acceptance criterion ("Rust, npm, Python and container fixtures each yield components", FR-038-AC-1/AC-2/AC-7, FR-039-AC-1..AC-4) has never been executed. The matrix shows "tagged", but no run with Syft exists, and reading Syft source predicts TC-279, TC-281 and TC-282 fail as written (SR-16413 FND-002..FND-004). Until a run against a real `EA_TEST_SYFT` passes, these criteria are unverified. | tests/syft_producer_cli.rs:353-421 |

## Verdict

Traceability is complete and honest. Verification is not: the Syft-dependent half of the ticket rests on unexecuted tests, three of which are predicted to fail. Mergeable once SR-16413 FND-002..FND-004 are fixed and TC-278..TC-282 pass against a real Syft, with the result recorded.
