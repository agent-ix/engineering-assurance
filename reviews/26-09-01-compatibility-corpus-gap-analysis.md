---
id: SR-025
title: "Gap analysis — accepted compatibility fixture corpus"
type: SpecReview
analysis: gap-analysis
scope: "FR-010, FR-011; spec/tests.md TC-069..TC-078; engineering_assurance/compatibility_corpus.py; tests/test_compatibility_corpus.py; the pinned qa-corpus submodule"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-011"
    type: "references"
---

# SR-025: Gap analysis — accepted compatibility fixture corpus

## Summary

Verification gate over FR-011 and over the deferral it closes in FR-010. Every
FR-011 acceptance criterion has a matrix row and a tracking-tagged test, every
Engineering Assurance #9 deliverable resolves to retained evidence or a stated
reference, and no code was added without an owning requirement.

## Verdict

**CONDITIONAL** — FR-011 has no coverage gap. Two findings record work that #9
names and this change does not fully discharge.

## Findings

| ID      | Severity | Summary                                                                | Refs                                          |
| ------- | -------- | ---------------------------------------------------------------------- | --------------------------------------------- |
| FND-001 | medium   | The chain is pinned to a Quoin source revision, not a released artifact | corpus/compatibility/corpus.json              |
| FND-002 | low      | Three artifacts are referenced by digest rather than retained            | corpus/compatibility/corpus.json              |

## Finding detail

### FND-001 — the chain's Quoin side is unreleased

Engineering Assurance #9 asks for the chain to be demonstrated "with exact
pinned artifacts". The Quire side is a released CLI (0.31.0, engine 0.46.0,
both with source revisions). The Quoin side is a build from source revision
`90a23b7`, because the `quoin change-assurance` surface the chain uses landed in
`agent-ix/quoin#325` and has not been released.

Failure scenario: a reader takes the corpus as evidence that a released
toolchain produces this receipt, when one side of it exists only as a commit.

Recorded rather than hidden: the corpus states in `chain.tools.quoin.note` that
it is "NOT a released artifact", and TC-074 asserts that sentence is present, so
the claim cannot quietly become stronger than the evidence. Turning it into a
released pin is `agent-ix/engineering-assurance#8`, which is the next gate after
this one.

### FND-002 — three artifacts are referenced, not republished

The external-engine result, the agent-evaluation result, and the Quire export
are pinned by digest but not copied into this repository, because each carries
content outside its publishable boundary (third-party advisory prose, an
absolute transcript path, an absolute module-manifest path).

Failure scenario: a reader without access to the source repositories cannot
verify those three digests locally.

Accepted: the alternative is republishing a machine path or third-party prose in
a public repository. The chain remains verifiable in structure — the retained
attestation and receipt bind the export's exact bytes — and TC-075 requires a
referenced case to name its producer, its digest, and why it is not retained, so
"referenced" cannot become a quiet way to list nothing.

## Coverage

FR-011, all backed by `tests/test_compatibility_corpus.py`:

| Criterion | Test case | Backing test |
| --- | --- | --- |
| FR-011-AC-1 | TC-069 | every retained artifact is the artifact recorded |
| FR-011-AC-2 | TC-070 | corpus covers every required state and labels constructions |
| FR-011-AC-3 | TC-071 | every legacy case maps to its recorded outcome |
| FR-011-AC-4 | TC-072 | no non-success case is read as a success |
| FR-011-AC-5 | TC-073 | real legacy records preserve identity, producer, and limits |
| FR-011-AC-6 | TC-074 | current-model receipt validates against the packaged schema |
| FR-011-AC-7 | TC-075 | producer cases name a real producer and a target concept |
| FR-011-AC-8 | TC-076 | the corpus is read-only and executes nothing |
| FR-011-AC-9 | TC-077 | the corpus reproduces from its recorded sources |
| FR-011-AC-10 | TC-078 | the checked-out corpus equals the reviewed gitlink |

Constraint coverage: CON-1 and CON-4 by TC-076, CON-2 by TC-069 (which matches
each real case against the digest its *source repository* recorded, so a
rewritten evidence tree fails here first), CON-3 by the corpus `limitations` and
by SR-024's notes.

Engineering Assurance #9 deliverables against what landed:

| Deliverable | State |
| --- | --- |
| Read-only PGM-01 v1/v2 mapping into the shared model | Pre-existing (FR-010); now exercised against real records rather than fictional ones |
| Explicit unmapped/lossy fields and limitations | Retained per case and asserted by TC-071 and TC-073 |
| Cross-language fixtures for the eight states | All eight kinds present; projected to Python, Rust, and TypeScript |
| A real governed `quire-code-rs` producer case | Retained at a pinned revision, asserted by TC-075 |
| Contract-conformance, ordinary test, measurement/proof, external-engine cases | Conformance, measurement, and diagnostic retained; external-engine referenced (FND-002) |

Acceptance criteria of #9:

| Criterion | State |
| --- | --- |
| Immutable evidence directories are never rewritten | Sources read through `git show origin/main:<path>`; TC-069 matches source-recorded digests |
| Views preserve revision, producer/config identity, result state, retained-output identity, limitations | TC-073 and TC-074 |
| Unreadable or ambiguous legacy is reported as such | TC-071 and TC-072 |
| Quire export → Quoin intake/audit → receipt with exact pinned artifacts | Demonstrated; Quoin side unreleased (FND-001) |
| The accepted fixture set becomes the migration implementation gate | FR-010's deferral removed; FR-011 enforcing |

Corpus location — the corpus is retained by the private `agent-ix/qa-corpus`
repository and pinned here as a submodule, because this repository is public and
its publication boundary permits fictional fixtures only. SR-024 FND-002 records
that the first implementation got this wrong and how it was corrected. The
practical consequence for coverage is that TC-069..TC-077 read real bytes that
this tree never holds, and TC-078 pins which bytes those are.

Underspecified code — none. `compatibility_corpus.py` is reached only from the
FR-011 tests and from `fixture_codegen`; `build_compatibility_corpus.py` now
lives beside the corpus in `qa-corpus`, and its `--check` mode is TC-077.

Semantic review — performed inline over FR-011's nine criteria. The tests
exercise the real mapper against real retained bytes rather than doubles, and
TC-077 additionally re-derives the whole corpus from its sources, so a fixture
edited to make a test pass would fail there. TC-076 is source inspection, which
is the right shape for a claim about what the reader does *not* do.
