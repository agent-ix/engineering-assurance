---
id: SR-027
title: "Gap analysis — pinned shared-assurance compatibility matrix"
type: SpecReview
analysis: gap-analysis
scope: "FR-012; spec/tests.md TC-079..TC-086; engineering_assurance/compatibility.py; scripts/check_compatibility_matrix.py; the released component set"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-012"
    type: "references"
---

# SR-027: Gap analysis — pinned shared-assurance compatibility matrix

## Summary

Verification gate over FR-012 after #8. Every acceptance criterion has a matrix
row and a tracking-tagged test, and every #8 acceptance criterion resolves to
something checked or to a stated open gate.

## Verdict

**CONDITIONAL** — no FR-012 coverage gap. Two findings record what the matrix
cannot claim on its own.

## Findings

| ID      | Severity | Summary                                                              | Refs                                                |
| ------- | -------- | -------------------------------------------------------------------- | --------------------------------------------------- |
| FND-001 | medium   | The gate is not satisfied until this repository's own tag exists      | engineering_assurance/compatibility-matrix.json:69   |
| FND-002 | low      | "Cross-component fixtures pass" rests on one chain, not a matrix of them | engineering_assurance/compatibility-matrix.json:121  |

## Finding detail

### FND-001 — the matrix pins a version this commit does not yet carry

The matrix pins `engineering-assurance 0.2.0`, and the tag is cut from the
commit that contains the matrix — so at review time the classifier reports
`unknown` for this component and the gate is correctly not satisfied.

Failure scenario: a reader runs the checker, sees `gate: NOT satisfied`, and
concludes the released components are wrong, when the only missing thing is
this repository's own tag.

Accepted and self-correcting: the tag is cut immediately after this lands, and
the checker then reports four `compatible` components. The ordering is stated in
the matrix's own upgrade note — "this repository's tag" is the third step, not
the first.

### FND-002 — one chain, not a fixture matrix

`#8` asks that "cross-component fixtures pass against the exact pinned
artifacts". What exists is one chain: a Quire export carried through Quoin seal,
intake, and audit to a valid receipt, retained artifact by artifact in the
pinned corpus, plus the twelve PGM-01 compatibility cases FR-011 enforces.

Failure scenario: a reader takes "cross-component fixtures pass" to mean a
combinatorial matrix of component versions has been exercised. It has not. One
version tuple has.

Accepted: the campaign pins exactly one version tuple, so a second tuple would
be evidence about a configuration nobody is allowed to run. The claim the
matrix makes is the claim it can support — that these pinned artifacts produce
this receipt — and the chain reproduced byte for byte across the change from a
source build to the released binary, which is the strongest form that claim has.

## Coverage

FR-012, all backed by `tests/test_compatibility_matrix.py`:

| Criterion | Test case | Backing test |
| --- | --- | --- |
| FR-012-AC-1 | TC-079 | every pin is a released artifact |
| FR-012-AC-2 | TC-080 | unknown and incompatible are distinct and neither passes |
| FR-012-AC-3 | TC-081 | the gate requires every component and says so |
| FR-012-AC-4 | TC-082 | human acceptance is pending and an agent cannot grant it |
| FR-012-AC-5 | TC-083 | pinned artifact digests match this tree |
| FR-012-AC-6 | TC-084 | upgrade and rollback are stated per component |
| FR-012-AC-7 | TC-085 | an unknown matrix version is refused |
| FR-012-AC-8 | TC-086 | the classifier executes nothing |

Constraint coverage: CON-1 by TC-086, which asserts both that the classifier
reaches for no subprocess or write and that the observing program does; CON-2
by TC-082; CON-3 by the rollback notes, every one of which is an install or a
checkout.

#8's acceptance criteria:

| Criterion | State |
| --- | --- |
| Every pin refers to a released artifact, not an ambient branch head | TC-079; quoin 0.23.1 was released during this work |
| Cross-component fixtures pass against the exact pinned artifacts | FR-011's corpus and chain, re-run against the released binary (FND-002 bounds the claim) |
| Unknown or incompatible versions fail explicitly | TC-080 and TC-081, with unknown kept distinct from incompatible |
| No enforcing migration begins before a human accepts the matrix | TC-082; acceptance is unset and an agent filling it in fails the gate |
| Publication does not enable automatic hosted CI | TC-084; the release workflow that published 0.23.1 is itself manual-dispatch |

Deliverables:

| Deliverable | State |
| --- | --- |
| Versioned EA semantic schemas and compatibility fixtures | Ten schema digests pinned and re-hashed by TC-083; fixtures are the FR-011 corpus |
| A Quire/quire-cli release with the assurance export surface | 0.31.0, already released (quire-cli#74) |
| A Quoin release with evidence, measurement, attestation, intake, audit, receipt surfaces | 0.23.1, released during this work |
| An explicit compatibility matrix with exact versions and digests | `compatibility-matrix.json` and `docs/compatibility-matrix.md` |
| Upgrade and rollback notes | Per component, asserted by TC-084 |

Underspecified code — none. `compatibility.py` is reached from the FR-012 tests
and from `scripts/check_compatibility_matrix.py`; the script is the documented
verification step in the matrix's own upgrade note.

Semantic review — performed inline over FR-012's eight criteria. The tests
exercise the real classifier against the real matrix, and TC-081 varies each
component in turn rather than asserting one happy tuple, so a rule that only
worked for `quoin` would fail. TC-083 additionally asserts the digest check
examined at least ten artifacts, because a digest check over an empty set
passes trivially — the tautology that would otherwise make this criterion
worthless.
