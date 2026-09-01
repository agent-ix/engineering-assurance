---
id: SR-029
title: "Gap analysis — campaign migration contract"
type: SpecReview
analysis: gap-analysis
scope: "FR-013; spec/tests.md TC-087..TC-094; docs/migration-contract.md; the eight campaign repositories at origin/main"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-013"
    type: "references"
---

# SR-029: Gap analysis — campaign migration contract

## Summary

Verification gate over FR-013 after #10. Every acceptance criterion has a
matrix row and a tracking-tagged test, every #10 acceptance criterion resolves
to something checked, and the decision table is verified against the eight
repositories rather than asserted.

## Verdict

**CONDITIONAL** — no FR-013 coverage gap. Two findings bound what a published
playbook can be evidence for.

## Findings

| ID      | Severity | Summary                                                                | Refs                              |
| ------- | -------- | ---------------------------------------------------------------------- | --------------------------------- |
| FND-001 | medium   | A reviewed playbook is not a migrated repository                        | docs/migration-contract.md:1      |
| FND-002 | low      | Two decision rows rest on reading a script name, not its contents       | docs/migration-contract.md:41     |

## Finding detail

### FND-001 — the playbook is the deliverable, and it is not evidence of a migration

`#10` asks for one reviewed migration contract, and that is what landed. No
repository has been migrated, and none may be until the compatibility matrix
records human acceptance.

Failure scenario: the closure of `#10` is read as "the campaign is migrated",
when what it means is "the procedure is written and reviewed". The eight
migrations are separate work with separate review.

Accepted, and the contract says so in its own first paragraph. `FR-013`'s
downstream dependency names the eight migrations as the consumers of this
document, not as part of it.

### FND-002 — two rows were classified by name and purpose, not by full reading

`validate_governance.py` and `validate_matrix_status.py` in `quire-contract-ir`
are classified **KEEP** as governance-specific rather than generic. That
judgement came from their names, their location, and the surrounding repository
— not from reading both files end to end.

Failure scenario: one of them turns out to contain a generic evidence collector,
and the table tells its migrating agent to keep it.

Accepted with a stated mitigation: the PR checklist requires the migrating agent
to produce the inventory in the PR body, so both files are read by someone
before anything is kept on the strength of this row. The other eleven rows were
classified after reading the file or its head — `build_evidence_envelope.py`
and `collect_evidence.sh` in particular, which is where the generic-envelope and
stdout-retention conclusions came from.

## Coverage

FR-013, all backed by `tests/test_migration_contract.py`:

| Criterion | Test case | Backing test |
| --- | --- | --- |
| FR-013-AC-1 | TC-087 | every family carries exactly one decision |
| FR-013-AC-2 | TC-088 | the table accounts for every recurring family |
| FR-013-AC-3 | TC-089 | both prohibitions are stated by name |
| FR-013-AC-4 | TC-090 | domain validation and evidence intake have distinct owners |
| FR-013-AC-5 | TC-091 | rollback is per failure mode and never rewrites history |
| FR-013-AC-6 | TC-092 | the review checklist covers every required question |
| FR-013-AC-7 | TC-093 | the allocation covers all eight repositories once |
| FR-013-AC-8 | TC-094 | migration waits on acceptance and claims no qualification |

Constraint coverage: CON-1 and CON-3 by TC-094, which also reads the matrix to
confirm the gate it points at is still closed; CON-2 by TC-094's assertions that
the contract changes no trigger.

#10's required procedure, step by step:

| Step | Where |
| --- | --- |
| Inventory generic machinery separately from domain logic | Decision table; procedure step 1 |
| Preserve domain runners, oracles, corpora, schemas, formats, failure behaviour | Procedure step 2; the KEEP rows |
| Register static obligations through Quire, dynamic results through Quoin | Procedure steps 3 and 4 |
| Replace local envelope, manifest, identity, retention, audit, history, traceability, anchors | Procedure step 5; the DELETE and REPLACE rows |
| Preserve legacy history through the compatibility view | Procedure step 6; rollback table |
| Simplify Makefiles to readable native orchestration | Procedure step 7 |
| Demonstrate pass, fail, unavailable, not-computed, malformed, stale, tampered | Procedure step 8; checklist |
| Delete old generic code only after the shared path passes at the same revision | Procedure step 9; TC-091 |

#10's acceptance criteria:

| Criterion | State |
| --- | --- |
| Mechanical delete/keep/replace decision for every recurring script family | TC-087 and TC-088 |
| Forbids repository-local generic evidence schemas and universal stdout corroboration | TC-089 |
| Distinguishes domain output validation from evidence intake | TC-090 |
| Includes rollback and legacy-history handling | TC-091 |
| Defines a common migration PR review checklist | TC-092 |
| Preserves the existing Agent A/B/C allocation | TC-093 |
| Hosted CI remains manual-only and is not dispatched | TC-094 |

Underspecified code — none. The contract is a document; its only executable
surface is its test module, which reads it and the eight repositories.

Semantic review — performed inline over FR-013's eight criteria. TC-088 is the
one that carries weight: it derives its expectation from the repositories
rather than from the document, so a table that omitted a family would fail even
though every prose assertion still passed. The remaining seven are document
assertions, which is the right shape for a document — with the exception of
TC-094, which additionally reads the compatibility matrix so the contract
cannot claim to wait on a gate that has already opened.
