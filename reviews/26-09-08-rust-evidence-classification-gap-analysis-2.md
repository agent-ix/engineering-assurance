---
id: SR-045
title: "Gap analysis — Rust evidence classification remediation at bad4a55"
type: SpecReview
analysis: gap-analysis
scope: "PR #27 at bad4a55; FR-015-AC-1..AC-3; spec/tests.md TC-100, TC-102, TC-103; src/evidence.rs; engineering_assurance/evidence.py; Cargo.toml"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: "references"
---

## Summary

The remediation added four normative statements to FR-015 and three
correspondingly widened acceptance criteria, and the implementation stays inside
them almost everywhere. Four behaviours reach production without an acceptance
criterion that owns them, and three of the four were introduced by this PR
rather than inherited.

## Verdict

**CONDITIONAL** — no high findings; four unstated requirements, each closable by
an AC edit rather than a code change.

## Method

Every observable behaviour change between `main` and `bad4a55` was enumerated
from the diff and from executed comparison, then matched against FR-015's
statement list and AC table. Three sources:

1. Differential execution of `main`'s and `bad4a55`'s
   `engineering_assurance/evidence.py` over a 19-token version grid.
2. A 58-shape JSON identity probe against the retained canonicaliser (recorded
   in [SR-044](./26-09-08-rust-evidence-classification-rereview.md)).
3. A read of `Cargo.toml`'s feature change for effects outside the digest path.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Whitespace refusal in version tokens is implemented and prose-spec'd but no AC gates it | src/evidence.rs:129, engineering_assurance/evidence.py:48 | missing-requirement |
| FND-002 | medium | `arbitrary_precision` changes `serde_json::Value` equality repo-wide; two unequal outputs can mint one identity and no requirement says whether that is intended | Cargo.toml:24, src/evidence.rs:174 | missing-requirement |
| FND-003 | medium | No AC owns the traceability scanner's population boundary, so submodule symbols count against this repository | scripts/check_integration_evidence.py | missing-requirement |
| FND-004 | low | The five version tokens whose classification flipped have no pinned pre-change fixture, only the live Python reference | tests/evidence_parity.rs:184 | correct-requirement-no-evidence |
| FND-005 | low | spec/tests.md types TC-100/102/103 as Property while FR-015 now types their ACs as Test | spec/tests.md:271, spec/functional/FR-015-semantic-and-identity-parity.md:139 | wrong-requirement |

## Finding detail

### FND-001 — the whitespace class

Executed against both revisions:

| version | `main` | `bad4a55` |
| --- | --- | --- |
| `" 1.2.3"` | valid | `identity-version-invalid-character` |
| `"1.2.3 "` | valid | `identity-version-invalid-character` |
| `"1.2.3\n"` | valid | `identity-version-invalid-character` |
| `"1.2.3 rc"` | valid | `identity-version-invalid-character` |

FR-015 states it: "Each governing version identity SHALL use a non-empty token
composed only of printable ASCII characters without whitespace." FR-015-AC-3
does not: it enumerates "non-printable or non-ASCII version tokens", and an
ASCII space is neither. So the refusal is required by the FR body, implemented in
both languages, and verified by no criterion.

There is also an untracked migration consequence. A governing version read from
a file with a trailing newline was accepted before this PR and is refused after
it; nothing in the AC set, the error-conditions paragraph, or PLAN-003 covers
previously-valid identities that stop validating. Add the whitespace class to
AC-3, or decide deliberately to trim and keep the refusal for non-graphic
characters only.

### FND-002 — a feature flag with reach beyond the digest

`serde_json`'s `arbitrary_precision` was enabled to fix FND-084, and it does. It
also changes `serde_json::Value` equality across the crate: `Number` compares by
retained token rather than by value. Measured:

```
{"v":1e2}  and  {"v":100.0}   =>  Value == Value  is false
```

Both canonicalise to `{"v":100.0}` and therefore mint the *same* identity digest.
So `ProducerAttempt`/`EvidenceEnvelope` — which derive `PartialEq` — can hold two
values that compare unequal while carrying one identity. Before this PR both
parsed to the same `f64` and compared equal.

Many-to-one canonicalisation is the point of a canonicaliser, so this is
probably intended. But FR-015 says the implementation "SHALL NOT use a different
canonicalization algorithm to rewrite or silently reinterpret an existing
identity" and says nothing about whether input equality and identity equality may
diverge. TC-103 asserts `attempt.output` is unchanged after a refusal, which is
the closest existing check and does not reach this. One sentence in FR-015
settling it — equal digests do not imply equal inputs, and derived `PartialEq` on
these types is byte-level not semantic — closes the gap.

### FND-003 — the scanner's population boundary

`make integration-traceability` reports, alongside the declared 207/262:

```
untracked symbols: TC-999, TC-999, TC-1, FR-001-INV-1, FR-001-INV-1,
FR-001-INV-1, TC-999, TC-999, TC-999, TC-1598, TC-1598, FR-003-CON-1
```

`TC-1598` is a quire-rs identifier and `TC-999`/`FR-001-INV-1` are
`ix-trace-rs` fixtures. Both arrive through submodules — `vendor/ix-trace-rs`
and `corpus` — which are other repositories with their own matrices. The gate
counts them against this repository's traceability.

FND-094 recorded this as "pre-existing scanner-scope noise" and assigned it to
PLAN-003 TASK-023, which is the right disposition. The gap is that no
requirement states what the traceability population *is*. NFR-005-AC-3 and
TC-118 speak to marker reconciliation, not to which trees are in scope. Without
that, TASK-023 has to invent the boundary at the moment it enforces it, and any
new submodule silently enlarges the population again.

### FND-004 — the flipped tokens have no independent fixture

`1.2.3+linux-x86_64`, `1.2.3+x86_64`, `2.0.0-beta+exp.sha.5114f85` and
`1.2.3+X86` moved from refused to accepted; `1.*` moved from accepted to
refused. These are the declared FND-087/088 correction and TC-103 exercises the
new behaviour. What is not recorded anywhere is the *old* classification, so
nothing distinguishes "we deliberately changed these five" from "these five
drifted". The differential in `tests/evidence_parity.rs:184` shells out to the
live `engineering_assurance/evidence.py`, which this PR edited, so it cannot
supply the before-value either. A small checked-in table of the five tokens with
both classifications and a one-line reason makes the change auditable without
re-running `git show main:`.

### FND-005 — matrix and FR disagree on verification type

Recorded in [SR-044](./26-09-08-rust-evidence-classification-rereview.md)
FND-002; repeated here because it is a traceability defect as much as a review
one. FR-015-AC-1/2/3 now say `Test`; `spec/tests.md` still types TC-100, TC-102
and TC-103 as `Property`. One of the two is stale and the traceability gate does
not compare them.

## Coverage confirmed

Checked and found genuinely owned — not gaps:

- Arbitrary-precision integer preservation → FR-015 statement + AC-1 + TC-100,
  backed by the generated corpus and by independent probe.
- Non-finite and overflow-to-non-finite refusal, and the absence of a digest on
  the refusal path → FR-015 statement + AC-3 + TC-103.
- Wire spellings for availability states, exact-one untyped label validation,
  `is_valid()`, and `EvidenceValidationError::message` → FR-015 statement +
  AC-2/AC-3 + TC-102/TC-103. FND-093 is closed.
- Immutable metadata containing `x` remaining acceptable while true
  wildcards/ranges are refused → FR-015 statement + AC-3 + TC-103.
- The staged matrix statuses are honest: aggregate rows stay pending, backed
  subcases are named, and `make integration-traceability` refuses to report
  completion. FND-091 is closed.
