---
id: SR-20261113192937
title: "gap-analysis of pluggable record sinks against FR-054..FR-057 and NFR-008..NFR-010 (engineering-assurance PR 233)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/engineering-assurance@94daf21; FR-054, FR-055, FR-056, FR-057, NFR-008, NFR-009, NFR-010; spec/tests.md; tests/record_sink.rs; src/record_sink/"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-055
    type: references
  - target: ix://agent-ix/engineering-assurance/NFR-010
    type: references
---

# Gap analysis of pluggable record sinks (engineering-assurance PR 233)

## Summary

Ticket: EA-105. Plan completion: not assessed. Computed with `quoin matrix --json` (quoin 0.30.1) at head 94daf21.
All 32 acceptance criteria of FR-054 (AC-1..6), FR-055 (AC-1..7), FR-056 (AC-1..8) and FR-057 (AC-1..11) are
tagged, each by a `tests/record_sink.rs` test. The six NFR metrics (NFR-008-M-1, NFR-009-M-1/M-2,
NFR-010-M-1/M-2/M-3) have zero binders. New TC rows TC-624..TC-651 each name a test that exists and passes
(29 passed at 94daf21); none of them is fabricated. No production code in the diff lacks an owning requirement.

Each AC was read against its test asking which source change would make the test fail. Behaviourally backed:
FR-054-AC-1..6, FR-055-AC-1..4 and AC-6, FR-056-AC-1..5 and AC-7, FR-057-AC-1..11 (TC-649 drives sends and
public-path signs over 500 generated visibility maps). Vacuous or partial bindings are listed below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-055-AC-5 / TC-633 cannot fail. The stand-in `Branches::read_from_base_branch` only reads `base`; the `pull_request` map is unreachable and `assert!(!source.pull_request.is_empty())` is a tautology. No production `PolicySource` exists, so nothing tests that a pull-request branch's `sinks.json` is ignored. Fix: implement the git-backed `PolicySource` (reads `<base>:<path>` and reports protection) and test it against a temp repository whose PR branch edits `sinks.json`; or, if the adapter belongs to a later ticket, set the FR-055-AC-5 / TC-633 rows to 🚧 naming that ticket instead of ✅. | tests/record_sink.rs:571-582; spec/tests.md (FR-055-AC-5 row) |
| FND-002 | medium | FR-055-AC-7 / TC-635 "the first publish is refused with nothing sent or stored" is asserted on a fresh, unused `World` (`assert!(World::new().nothing_sent_or_stored())`), which cannot fail. The sink-layer branch that refuses a selection with no private store destination (sink.rs:322-324) has no test at all. Fix: publish through a `SinkLayer` whose selection has `private_store_destination: None` for `public_log` and for `private_store`, and assert `refused` / `sink_config_incomplete` and `nothing_sent_or_stored()` on that same world. | tests/record_sink.rs:638-639; src/record_sink/sink.rs:321-324 |
| FND-003 | medium | The NFR-010 row is ✅ and says the delegated-library verification count (NFR-010-M-3: a stand-in library that rejects every signature) "is exercised by the signature-payload check of TC-642". It is not: no test substitutes a verifier that rejects every signature, and verification is FR-064's scope, not in this PR. Fix: reword the NFR-010 row to cover M-1 and M-2 only and mark M-3 🚧 (FR-064). | spec/tests.md (NFR-010 row) |
| FND-004 | low | NFR trace tags use bare ids (`"NFR-008"`, `"NFR-009"`, `"NFR-010"`), but the matrix binds metric ids, so every NFR-008..010 metric shows zero binders. Fix: tag TC-628/632/637 with `NFR-008-M-1`, TC-649 with `NFR-009-M-1`, TC-651 with `NFR-009-M-2` and `NFR-010-M-1`, TC-650 with `NFR-010-M-2`. | tests/record_sink.rs:382; tests/record_sink.rs:1085; tests/record_sink.rs:1132; tests/record_sink.rs:1160 |
| FND-005 | low | FR-056-AC-8 "a bundle over any other bytes is rejected": the only adverse signer signs a wholly different payload. No case signs the same public form non-canonically (`serde_json::to_vec` member order) or the canonical bytes with the `signature` member, the two near-misses a broken canonicaliser would produce. Fix: add `SignMode::NonCanonical` signing `serde_json::to_vec(public)` and assert `signature_payload_mismatch` with nothing sent. | tests/record_sink.rs:122-159; tests/record_sink.rs:892-903 |
| FND-006 | low | FR-056-AC-6 is a property over every public member, but TC-640 generates strings only for `hashes.statement`, `signature` and the repo id. `record_id`, `linear_issue` ids, `subject_commit` and timestamps are covered only by fixed examples in TC-638. Fix: extend the TC-640 loop with the same generated string checked against those four grammars. | tests/record_sink.rs:797-843 |

## Verdict

Every functional AC is tagged and most are behaviourally backed. Two ✅ rows (FR-055-AC-5, FR-055-AC-7) rest on
assertions that cannot fail, and the NFR-010 row overclaims a metric no test touches. Not mergeable until
FND-001..FND-003 are fixed; FND-004..FND-006 are small and should ride in the same fix round.
