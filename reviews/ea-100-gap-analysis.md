---
id: SR-20261113124203
title: "Gap analysis of release-verify and Baseline resolution (engineering-assurance PR 228)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance@fe1c308; PR 228 (EA-100); spec FR-046, FR-047, FR-049, FR-050, FR-051, FR-053, NFR-007; src/release.rs; src/release_host.rs; tests/release.rs; tests/release_verify_cli.rs; spec/tests.md"
review_set: subset
---

# Gap analysis of release-verify and Baseline resolution (engineering-assurance PR 228)

## Summary

Ticket: EA-100. This is a planless gap analysis of PR agent-ix/engineering-assurance#228 at fe1c308. It covers the release-verify and resolution criteria this ticket delivers. The matrix comes from `quoin matrix --json` (quoin 0.30.1).

## Verdict

**FAIL**, on two medium coverage gaps. All other in-scope Test and Property criteria are tagged with tests that can fail. The two mutations run here were both killed (see SR-20261113124202).

FR-051-AC-7 is out of scope: it belongs to the release-decision workflow of FR-048. The Inspection criteria were inspected and hold: FR-047-AC-11, CON-2, CON-3, FR-049-AC-3/4, FR-050-AC-1..3, FR-051-AC-9 and FR-053-AC-3.

Reverse gap: none. Every new function has an owning statement in FR-046, FR-047 or FR-051, and there are no stubs or placeholders.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-046-AC-5 is untagged in the computed matrix (static_status untagged). The TC-390 row in spec/tests.md claims FR-046-AC-4 and FR-046-AC-5, and the test asserts refs, index and working tree unchanged. Its marker, though, is #[trace("TC-390", "FR-046-AC-4")], so AC-5 never binds. Add FR-046-AC-5 to the trace. | tests/release_verify_cli.rs:550 |
| FND-002 | medium | FR-047-AC-9 names a signed commit, a merge commit and a note carrying the record trailers. TC-407 covers the signed commit, a commit id and a note, but no merge commit carrying the trailers is ever passed as the tag. TC-395 tests a signed tag on a merge release commit, which is a different case. Add the merge-commit case to TC-407. | tests/release_verify_cli.rs:925 |
| FND-003 | low | NFR-007-M-1 ('Property test over the pure decision') is untagged, yet tc_415 is exactly that property test. Add NFR-007-M-1 to tc_415's trace so the metric binds. | tests/release.rs:81 |
| FND-004 | low | tc_416 (path rules) is tagged FR-047-AC-6, but AC-6 is about release-commit shape. The test exercises is_repository_path, which only guards the request's record_path (FR-047-AC-10 refusal); the snapshot path rule is enforced by the schema (FR-051-AC-4). Replace FR-047-AC-6 with FR-047-AC-10. | tests/release.rs:122 |

## Coverage

- Criteria examined and tagged with a failing-capable test: FR-046-AC-3, AC-4, AC-6; FR-047-AC-1..AC-10, AC-12, CON-1; FR-049-AC-1, AC-2; FR-051-AC-1..AC-6, AC-8; FR-053-AC-1, AC-2.
- Criteria examined and untagged: FR-046-AC-5 (FND-001), NFR-007-M-1 (FND-003). The FR-047-AC-9 merge branch is partly uncovered (FND-002).
- Inspection criteria examined by reading the code: FR-047-AC-11, CON-2, CON-3; FR-049-AC-3, AC-4; FR-050-AC-1..AC-3; FR-051-AC-9; FR-053-AC-3; NFR-007-M-2.
- Context only: FR-046-AC-1, AC-2 (EA-93); FR-050-AC-4 (spec.md, EA-92); FR-051-AC-7 (FR-048 workflow).
- Hostile-repository coverage is missing. No test uses replace refs or grafts. The defect is code-review SR-20261113124202 FND-001.
- Semantic review: skipped (reviewer subagent, not opted in).
- Plan completion: not assessed
