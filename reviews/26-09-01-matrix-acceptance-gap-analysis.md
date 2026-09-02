---
id: SR-031
title: "Gap analysis — compatibility matrix human acceptance"
type: SpecReview
analysis: gap-analysis
scope: "engineering_assurance/compatibility-matrix.json, scripts/check_compatibility_matrix.py, docs/migration-contract.md, spec/functional/FR-012, spec/functional/FR-013, spec/tests.md, tests/test_compatibility_matrix.py, tests/test_migration_contract.py"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-012"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-013"
    type: "references"
---

# SR-031: Gap analysis — compatibility matrix human acceptance

## Summary

Verifies the change recording human acceptance of the FR-012 compatibility
matrix. The Test Matrix reconciles fully — 189 of 189 rows backed, no unbacked
rows, no status lies — and yet the two most important findings here are things
that full coverage did not catch. One is a documented precondition that nothing
mechanically enforces; the other is a constraint clause that binds by tag and
not by meaning.

## Verdict

**FAIL at analysis, PASS after remediation.** Three high findings and one low.
None was a missing test row; all three highs were places where a green signal
did not mean what a reader would take it to mean. All are fixed in-branch and
proven by mutation probe; see Disposition.

## Findings

| ID      | Severity | Summary                                                                     | Refs                                       |
| ------- | -------- | ---------------------------------------------------------------------------- | ------------------------------------------ |
| FND-001 | high     | The gate script's exit code ignores acceptance state entirely                 | scripts/check_compatibility_matrix.py:111  |
| FND-002 | high     | CON-2's operative clause binds to TC-082 by tag but is unverified             | spec/functional/FR-012-pinned-compatibility-matrix.md:70 |
| FND-003 | high     | The migration contract states the acceptance fields are null; they are not    | docs/migration-contract.md:9               |
| FND-004 | low      | No plan bundle targets this change                                            | plan/                                      |

## Disposition

| ID | Disposition | How |
| --- | --- | --- |
| FND-001 | **FIXED** | Acceptance is now a gate condition, decided in the pure half. New `human_acceptance_recorded()` in `engineering_assurance/compatibility.py`; `check_compatibility_matrix.py` exits non-zero while acceptance is unrecorded. New FR-012-AC-9 / TC-095 cover it. |
| FND-002 | **FIXED** | TC-082 asserts the attribution names neither an agent nor a bot. The clause now constrains. |
| FND-003 | **FIXED** | The contract paragraph no longer asserts field values it cannot keep current. Cross-referenced `SR-030` FND-001. |
| FND-004 | **ACCEPTED** | No plan bundle. Recording one decision is not a plan's worth of tasks; Step 1 has no subject rather than a failing one. |

### Mutation probe

Each acceptance failure mode was written into the matrix, both gates run, and
the file restored. Recorded because a gate that has never been seen red is a
gate nobody has tested:

| Mutation | Gate script | Test suite |
| --- | --- | --- |
| `pending_human_acceptance` | exit 1 | red |
| `accepted`, `accepted_by: null` | exit 1 | red |
| `accepted`, `accepted_at: null` | exit 1 | red |
| `accepted`, whitespace-only name | exit 1 | red |
| unrecognised state (`rubber-stamped`) | exit 1 | red |
| attribution names an agent | exit 0 | red |

The last row is a deliberate division, not a hole. Whether a record is
*complete* is a structural question the gate answers; whether the named party is
a human is a judgement, and a substring denylist is too weak a thing to put in
the runtime path. It lives in TC-082, where a static record-integrity assertion
belongs. Anything gating on the script alone inherits that limit and should
know it.

## FND-001 — the gate reports acceptance but does not gate on it

```python
ok = accepted(classifications) and not mismatches   # :111
...
return 0 if ok else 1                               # :146
```

`accepted()` is `all(item.verdict == "compatible" for item in classifications)`.
It takes classifications and nothing else. `matrix["accepted"]["state"]` reaches
the output — as a JSON field at `:118` and a printed line at `:144` — and never
reaches `ok`.

The consequence: **`check_compatibility_matrix.py` exits 0 whether or not a
human has accepted the matrix.** It would have exited 0 yesterday, with
`accepted_by` null, because all four components were already the pinned
versions. Exit status answers "are the versions right", not "may migration
begin".

This is not a regression — the script is faithful to its own documented contract
(`docs/compatibility-matrix.md`: *"It exits non-zero unless every component is
`compatible`"*). The gap is between that contract and the role the script is
given elsewhere. `docs/migration-contract.md:10` points at this script as the
thing that "reports the gate", and the natural next step for any agent or
automation is to shell out to it and branch on the exit code. That check passes
on an unaccepted matrix.

Until this change, the acceptance precondition did have one mechanical
enforcer: TC-082 asserted the fields were unset, so the test suite went red if
anyone filled them in. That enforcement was aimed at the wrong direction — it
caught acceptance rather than its absence — and this change necessarily removes
it. What remains is prose.

**Recommendation:** decide deliberately whether acceptance belongs in the exit
code. Either gate on it (`ok = accepted(...) and not mismatches and
matrix["accepted"]["state"] == "accepted"`) and say so in the doc, or state
plainly in both the script's help text and the migration contract that exit 0
means versions-match-only and the acceptance precondition is verified by
reading. Both are defensible; the current silence is not.

## FND-002 — 189/189 backed, and the clause is still unverified

`quire coverage` reports every matrix row backed. FR-012-CON-2 is among them,
bound to TC-082 via `spec/tests.md:204`. The binding is real: the tag resolves,
the test exists, it runs, it passes.

CON-2 now reads:

> An agent SHALL NOT decide acceptance of the matrix. It MAY transcribe an
> acceptance a named human has explicitly directed it to record, and the
> recorded attribution SHALL name that human rather than the agent.

TC-082 asserts `accepted_by` is a non-empty string. `"Agent IX"` is a non-empty
string. The clause that does the constraining — *names that human rather than
the agent* — has no assertion anywhere.

This is the coverage binding trap in its cleanest form: a tag greps fine, the
row goes green, and the semantics behind it are unexamined. It is worth
recording precisely because the mechanical steps of this analysis returned a
perfect score over it. Step 2 cannot see this; only reading the constraint
against its test can.

It also has an aggravating factor. CON-2 governs agent behavior, it was
rewritten by an agent, and the rewrite shipped in the same commit that first
depended on it. That is the configuration in which an unverified clause matters
most.

## FND-003 — the contract asserts a state that no longer holds

`docs/migration-contract.md:7-11` tells its readers that `accepted_by` and
`accepted_at` "are null" and that "TC-082 fails if one tries" to grant
acceptance. Neither is true after this change.

TC-094 asserts two substrings of that same paragraph and both still match, so
the suite stays green while the paragraph is wrong. Substring assertions over
prose pin the sentences someone thought to quote; the sentence between them
carried the factual claim and was asserted by nothing.

This is the document the eight campaign migrations are directed to read first,
and the false claim is in its opening paragraph. Cross-referenced as FND-001 in
`SR-030`.

## FND-004 — no plan bundle

`plan/` holds `PLAN-001-assurance-onboarding` and
`PLAN-002-verification-semantics`; neither targets this work. Recording an
acceptance is a single decision, not a plan's worth of tasks, so Step 1's
completion check has no subject rather than a failing one. Recorded for
completeness, not as an objection.

## Coverage

`quire coverage --scope . --json`:

| Measure | Value |
| --- | --- |
| Matrix rows backed | 189 / 189 |
| Unbacked rows | 0 |
| Status lies | 0 |
| Untracked symbols outside `corpus/` | 0 |
| Acceptance criteria reconciled | 95 |

All `untracked_symbols` entries fall inside the `corpus/` submodule, which is
the FR-011 fixture set of deliberately-broken detection cases. They are the
corpus doing its job, not drift in this repository.

**Semantic review (Step 4): performed inline rather than fanned out.** The
change touches two test functions and one data file, so a per-FR subagent fan-out
would cost more than reading them. That pass is what produced FND-002, which the
mechanical steps scored as fully covered.
