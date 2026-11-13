---
id: SR-20261113055253
title: "Base spec review of the FR-039 licence fixture and FR-039-AC-8 (engineering-assurance PR 223)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-039-syft-sbom-evidence-ingestion.md (PR #223 diff, head 9ca41138)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-039
    type: references
---

# SR-20261113055253: Base spec review of the FR-039 licence fixture (engineering-assurance PR 223)

## Summary

Ticket: PLAT-1671. Review set `subset`: base checklist plus ears-conformance and integrity, each in its
own SpecReview (plat-1671-ea-spec-review-*). No AssuranceProfile document exists under `spec/` (only the
`engineering_assurance/skeletons/AssuranceProfile.md` template), so no profile applies and this is a
direct review. Paired with quoin PR 847 (FR-150, FR-150-AC-1 and FR-150-AC-12 read as context).

Examined: FR-039 Inputs licence-fixture paragraph, the two new Behavior bullets, FR-039-AC-8, and the
new `references` edge to quoin FR-150. Context: FR-039-AC-1 to AC-7, FR-038, FR-044 (its FR-039 fixture
statement), NFR-006, quoin FR-150.

Checked clean:
- Licences are specified as SPDX identifiers and an SPDX expression on a fictional npm package with no
  real package name; no licence text, standard text or paywalled material is named or reproduced.
- No second SBOM tool and no new execution path: the fixture is scanned by the existing FR-038 Syft
  request, and AC-5 (Syft acquisition) covers it. NFR-006 bars production component readers only; the
  AC-8 reader is test code.
- FR-044's statement that the FR-039 fixtures pin no vulnerable package stays true.
- No hash, digest, pin or tracking record ("pinned" is lock-file dependency pinning). No roadmap or
  research-repo content. `quire validate` reports no error on FR-039.
- The expected-result-file pattern matches FR-044's fixtures.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The four fixture entries (permissive id, copyleft id, two-id expression, unlicensed) cannot supply the five roles quoin FR-150-AC-1 needs from captured output (allowed, denied, not-listed, unlicensed, banned): with `allow` holding the permissive id and `deny` the copyleft id, no single SPDX id is left to be `licence-not-allowed`, and FR-150-AC-12's hand-written fallback is only for a case Syft cannot produce, which this is not. Fix: add a fifth entry with a second single SPDX identifier (meant to be in neither list), and state the entry count. | spec/functional/FR-039-syft-sbom-evidence-ingestion.md:35-38 |
| FND-002 | medium | AC-8 says the `licenses` entry's "value matches the expected-result file", but a CycloneDX licence entry has three shapes (`license.id`, `license.name`, `expression`) and Syft emits a single SPDX id as `license.id` and an expression as `expression`; FR-150 classes licences by that shape. Two implementers would compare different fields. Fix: the expected-result file records, per purl, the CycloneDX field (`id` or `expression`) and its text, and AC-8 asserts both. | spec/functional/FR-039-syft-sbom-evidence-ingestion.md:75 |
| FND-003 | low | The licence-fixture paragraph does not say the fixture is placed under `scan/` as the other four are, does not require a lock file version whose `packages` entries carry `license` (a v1 lock has none for Syft to read), and says the entries "declare licences" while one has no `license` field. Fix: "placed under `scan/`, with a `lockfileVersion` 3 lock file; three of its four pinned entries declare a licence". | spec/functional/FR-039-syft-sbom-evidence-ingestion.md:35-39 |

## Verdict

The fixture is the right, minimal change: fictional, SPDX identifiers only, the existing Syft path, no
new tool and no ceremony. Not mergeable until FND-001 (the fixture cannot feed FR-150-AC-1's
not-allowed case) and FND-002 (which licence field is compared) are fixed; FND-003 is wording for the
same round.
