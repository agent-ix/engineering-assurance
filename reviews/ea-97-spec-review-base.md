---
id: SR-20261113170514
title: "Base spec review of release attestations and per-change receipts (engineering-assurance PR 232)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-056, FR-066..FR-079, spec/non-functional/NFR-011, spec/spec.md, spec/tests.md (PR #232 diff, head 5bab3f1)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-070
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-072
    type: references
  - target: ix://agent-ix/engineering-assurance/NFR-011
    type: references
---

# SR-20261113170514: Base spec review of release attestations and per-change receipts (engineering-assurance PR 232)

## Summary

Ticket: EA-97 (with EA-98; EA-107, EA-108 and EA-109 scope). Review set `subset`: base checklist plus
ears-conformance, integrity, security, failure-domain and scope-boundary, each in its own SpecReview
(ea-97-spec-review-*). No AssuranceProfile document applies under `spec/` (only the skeleton template), so
this is a direct review.

Examined: every Behavior bullet and AC of FR-066 to FR-079 and NFR-011, the FR-056 amendment, the spec.md
index entries and the 72 new tests.md rows (TC-518 to TC-589). Context: FR-041, FR-047, FR-051, FR-053,
FR-054 to FR-065, NFR-007 to NFR-010.

Checked clean:
- `quire validate --scope .` over the 18 changed files: exit 0, zero warnings.
- tests.md: 70 AC rows map one to one to the 70 new ACs (FR-066 5, FR-067 6, FR-068 6, FR-069 3, FR-070 3,
  FR-071 5, FR-072 6, FR-073 5, FR-074 5, FR-075 6, FR-076 5, FR-077 5, FR-078 6, FR-079 4); TC text equals
  the AC text; the Property AC (FR-068-AC-2) maps to a Property TC and every Inspection AC to a Static TC;
  NFR-011 has TC-588 and TC-589. No TC id collides with existing rows.
- ID formats, relationships and Dependencies sections are well formed; every new FR is indexed in spec.md.
- No paywalled standard text: SLSA, in-toto, CycloneDX, Sigstore and RFC 8785 are open, and the CycloneDX
  schema comes from the library dependency, not a committed copy.
- No ticket text, model name or private-repository detail appears in the spec text.
- The digests in the change-record predicate and the verification-summary input attestations bind signed
  content to the statements that prove it; they are identity digests, not ceremony. The pinned workflow
  identity in the policy is load-bearing (it is the trust root of every signature).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | File-inventory check. FR-070 "SHALL add the five workflow files this requirement names and no other workflow file" with FR-070-AC-1 / TC-538 "the set of files added under `.github/workflows` equals the five named files" is a file-tracking assertion: it tests the shape of a diff, not behaviour, and passes for a broken workflow. Fix: delete the bullet, FR-070-AC-1 and TC-538; keep the five file descriptions as design and keep AC-2 (triggers and permissions, a real security property) and AC-3. | spec/functional/FR-070-release-and-receipt-workflow-files.md:35, 46; spec/tests.md:259, 767 |
| FND-002 | low | Process provenance in a public spec: FR-070 opens "Peter cleared edits under `.github/workflows` for these features" and "so the review can check the diff against it". Clearance is a ticket fact, not a requirement. Fix: delete both phrases. | spec/functional/FR-070-release-and-receipt-workflow-files.md:22 |
| FND-003 | low | Typos: "Tracked by Linear EA-98.." (double period) in FR-072 to FR-078 and "EA-97 and EA-98.." in FR-079; FR-072 repeats the tracking sentence twice. Fix: one sentence, one period. | spec/functional/FR-072-change-record-predicate.md:20; FR-073..FR-079 Description first line |

## Verdict

Not mergeable on the base checklist until FND-001 is removed. Structure, validation and AC-to-TC coverage are
clean. See the sibling analyses for the substantive findings.

## Dispositions

Round 1, head 428c2bd (fix commits 5bab3f1..428c2bd).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
