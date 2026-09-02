---
id: SR-030
title: "Code review — compatibility matrix human acceptance"
type: SpecReview
analysis: code-review
scope: "engineering_assurance/compatibility-matrix.json, docs/compatibility-matrix.md, spec/functional/FR-012, spec/tests.md, tests/test_compatibility_matrix.py, tests/test_migration_contract.py"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-012"
    type: "references"
---

# SR-030: Code review — compatibility matrix human acceptance

## Summary

Reviews the change that records human acceptance of the FR-012 compatibility
matrix, opening the gate that `agent-ix/engineering-assurance#8` held closed and
that `#10` waits on. The change flips one data field, and because two test gates
and one published document asserted that field's previous value, it necessarily
touches those too. That blast radius is the review's real subject: a change to a
gate's state is exactly the change most likely to leave stale claims behind.

## Verdict

**FAIL at review, PASS after remediation.** Two high findings and one low, all
three now fixed in-branch and re-verified. Dispositions are recorded below;
the finding text is left as written so the record shows what was found, not
only what survived.

## Gates

Run against branch `accept-compatibility-matrix` at head `5252e09`:

- `make lint` (ruff) — clean.
- `make test` — 182 passed, 0 failed. One pre-existing `jsonschema`
  DeprecationWarning, unrelated and unmodified.
- `make package-audit` — passes.
- `scripts/check_content_rights.py --tree` — passes.
- `python3 scripts/check_compatibility_matrix.py` — exit 0, all four components
  `compatible`, `human acceptance: accepted`.
- The `pre-push` hook independently re-ran lint, test, and package-audit
  against this head.

## Findings

| ID      | Severity | Summary                                                                    | Refs                                  |
| ------- | -------- | -------------------------------------------------------------------------- | ------------------------------------- |
| FND-001 | high     | The migration contract still states `accepted_by` and `accepted_at` are null | docs/migration-contract.md:9           |
| FND-002 | high     | CON-2's new "names the human rather than the agent" clause has no test       | spec/functional/FR-012-pinned-compatibility-matrix.md:70 |
| FND-003 | low      | Date check leans on the truthiness of an always-truthy `date`                | tests/test_compatibility_matrix.py:124 |

## Disposition

| ID | Disposition | How |
| --- | --- | --- |
| FND-001 | **FIXED** | `docs/migration-contract.md` no longer states the field values. It directs the reader to the file or the gate script, which is the only claim that stays true as the state changes. |
| FND-002 | **FIXED** | TC-082 now asserts the attribution names neither an agent nor a bot, so `"Agent IX"` fails the constraint that permits transcription. |
| FND-003 | **FIXED** | The `fromisoformat` call stands on its own with a comment naming the `ValueError` as the check; the misleading `assert` is gone. |

Re-verified at the remediated head: `make lint`, `make test` (183 passed),
`make package-audit` all clean.

## FND-001 — a published document asserts the gate is closed

`docs/migration-contract.md:7-11` reads:

> **Nothing in this playbook may begin until the compatibility matrix records
> human acceptance.** `accepted_by` and `accepted_at` in
> `engineering_assurance/compatibility-matrix.json` are null, and
> `python3 scripts/check_compatibility_matrix.py` reports the gate. An agent
> cannot grant that acceptance; TC-082 fails if one tries.

Two sentences are now false. The fields are not null. TC-082 no longer fails
when acceptance is filled in — that is the entire point of this change. A reader
opening the contract to find out whether migration may begin is told, in the
document's most prominent paragraph, that it may not.

The reason nothing caught this is instructive. TC-094 asserts two substrings of
this paragraph — `"may begin until the compatibility matrix records"` and
`"An agent cannot grant that acceptance"` — and both survive. The false sentence
sits between them and is asserted by nothing. A substring test over prose pins
the sentences someone thought to quote, not the paragraph's truth.

**Severity: high.** This is the document the eight migration agents are told to
read first, and it misstates the one precondition they check.

## FND-002 — the widened constraint outran its test

FR-012-CON-2 previously read *"An agent SHALL NOT record human acceptance of the
matrix."* This change rewords it to permit transcription, and adds a clause:

> ...and the recorded attribution SHALL name that human rather than the agent.

That clause is the load-bearing half of the new constraint. It is what stops the
reworded CON-2 from being a blanket permission. The spec marks CON-2's validation
as `Test`, and `spec/tests.md:204` traces it to TC-082.

TC-082 asserts `accepted_by` is a non-empty string. `"Agent IX"` is a non-empty
string. So is `"an agent"`. The clause is stated and unenforced.

This matters more than a normal traceability gap because of who wrote it. The
constraint that governs agent behavior was rewritten by an agent, in the commit
that first relied on the rewrite, and the part that constrains was left
untested. Whether or not the rewording is accepted on its merits — and the PR
flags it for exactly that scrutiny — it should not ship with the enforcing half
unverified.

**Severity: high.** A `Responsibility` constraint validated by `Test` with no
test for its operative clause.

## FND-003 — truthiness where the check is really "does not raise"

`tests/test_compatibility_matrix.py:124`:

```python
assert datetime.date.fromisoformat(acceptance["accepted_at"])
```

`datetime.date` has no falsy value, so the `assert` never fails on its operand.
The check that actually runs is `fromisoformat` raising `ValueError` on a
malformed date. That works, but the code reads as a value assertion and is one
refactor away from someone "simplifying" it into nothing.

**Severity: low.** Correct behavior, misleading form.

## What was checked and found clean

- **Test conventions.** This repo's tests are module-level functions with a
  `"""Trace: ..."""` docstring, not classes. Both changed tests keep that form
  and their trace lines. The skill's default "tests should use classes" does not
  apply here; the repo idiom outranks it.
- **Mock compliance.** No mocks added or changed. Both tests read real data.
- **Completeness.** No `TODO`, `FIXME`, `pass` placeholder, skip marker, or
  empty test body in the diff.
- **Integrity.** No coverage threshold lowered, no warning suppressed, no
  pragma added.
- **The rewritten gates are still load-bearing.** Setting `state: "accepted"`
  with a null `accepted_by` fails TC-082 on the `isinstance` assertion, and
  fails TC-094 on the "nobody on record" assertion. The half-recorded state both
  tests were rewritten to catch is genuinely caught.
- **TC-094's narrowing is within spec.** Its traced criterion FR-013-AC-8 is
  about what the contract document *states*, not about live gate state. The
  removed `assert state == "pending_human_acceptance"` was beyond that
  criterion — it froze the test at the moment it was written. Removing it is a
  correction, not a weakening.
- **The `ix-flow` release-string fix is right.** The unscoped `ix-flow@0.0.4`
  404s on the public registry; `@agent-ix/ix-flow@0.0.4` resolves. Version and
  gate behavior were always correct — only the string a reader would copy was
  wrong. Verified against `registry.npmjs.org`, not the `npm.ix` mirror.
- **Attribution carries no email.** The repository is public and the criterion
  asks for a name and a date. Recording the name alone satisfies FR-012-AC-4
  without publishing a personal address.
