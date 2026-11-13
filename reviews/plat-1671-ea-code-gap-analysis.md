---
id: SR-20261113055257
title: "Gap analysis of engineering-assurance PR 224 (FR-039-AC-8 licence fixture)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@870c8d19; spec/functional/FR-039-syft-sbom-evidence-ingestion.md; spec/functional/FR-038-syft-sbom-producer-request.md; spec/functional/FR-044-grype-advisory-scan-producer-request.md; spec/non-functional/NFR-006-single-sbom-tool-single-execution-path.md; tests/syft_producer_cli.rs; tests/fixtures/licence-npm/; spec/tests.md; agent-ix/quoin@origin/main spec/functional/FR-150-licence-and-ban-policy-findings.md (AC-12, context)"
review_set: subset
---

# Gap analysis of engineering-assurance PR 224

## Summary

Ticket: PLAT-1671. Plan completion: not assessed. Measured at 870c8d19.

`quoin matrix --repo . --json` (quoin 0.30.1) reports FR-039-AC-8 as `tagged-by-ignored-test`. That is the same status as its Syft-dependent siblings FR-039-AC-1..AC-4. The tag is `#[trace("TC-377", "FR-039-AC-8")]` on `tc_377_licence_fixture_components_carry_the_expected_licences`, and the test asserts what the AC says:

- the declared package count with `pkg:npm/` purls;
- for each purl, the `license.id` or `expression` and the text the expected-result file names;
- no `licenses` on the unlicensed entry.

The FR-039 Behavior clause "if no component carries `licenses`, the test SHALL fail" is asserted explicitly. Stripping every licence from the lock made the test fail.

The fixture matches the FR-039 Inputs paragraph:

- `lockfileVersion` 3 and five third-party entries;
- a permissive id (MIT), a copyleft id (GPL-3.0-only) and a second permissive id (Apache-2.0);
- a two-id expression (`MIT OR BSD-3-Clause`) and one entry with no licence;
- fictional names, with an expected-result file beside the fixture.

This is what quoin FR-150-AC-12 needs for its captured fixtures: five entries, four with a licence, one permissive id to allow, the copyleft id to deny, two not listed, and any one purl to ban. Real Syft 1.57.1 emits `licenses[].license.id` for the three ids, `licenses[].expression` for the expression (with the parentheses removed), and no `licenses` key for the unlicensed entry.

FR-038: the test uses the unchanged FR-038 request through `producer-execute`. NFR-006: there is no new Syft launch path and no production component reader. The only reader is test code, which FR-039-AC-7 allows for AC-8. FR-044 (Grype) is not touched. The diff adds no production code, so there is no unowned code, stub or coverage inflation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. FR-039-AC-8 is covered by a test that runs real Syft through `producer-execute` and can fail. The fixture meets the FR-039 Inputs contract and the needs of quoin FR-150-AC-12.
