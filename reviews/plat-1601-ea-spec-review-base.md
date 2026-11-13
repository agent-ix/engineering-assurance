---
id: SR-20261113015353
title: "Base spec review of FR-044 Grype advisory-scan producer request and NFR-006 amendment (engineering-assurance PR 221)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-044-grype-advisory-scan-producer-request.md, spec/non-functional/NFR-006-single-sbom-tool-single-execution-path.md, spec/spec.md (PR #221 diff)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-044
    type: references
---

# SR-20261113015353: Base spec review of FR-044 and NFR-006 (engineering-assurance PR 221)

## Summary

Ticket: PLAT-1601. Review set `subset`: base checklist plus ears-conformance, integrity,
failure-domain and dependency, each in its own SpecReview (plat-1601-ea-spec-review-*). Not run:
evidence, risk-complexity (no change in verification-method catalogue or risk surface beyond
what failure-domain covers) and scope-boundary (its one question here, "no second SBOM, one
execution path", is checked in this base review). No AssuranceProfile document exists under
`spec/` on the branch (only the `engineering_assurance/skeletons/AssuranceProfile.md` template), so
no profile applies and this is a direct review.

Examined: FR-044 Description, Why Grype table, Inputs table, Outputs, Behavior, CON-1, AC-1 to
AC-10; NFR-006 Statement, Scope, Measurement, AC-1, AC-2; the spec/spec.md index line. Context:
FR-019 (input snapshot and argument binding semantics, `src/producer_execution.rs`
`ArgumentBinding`/`StdinBinding`), FR-029, FR-038, FR-039, quoin PR 846 FR-149 and FR-142.

Checked clean:
- No second SBOM: Grype is given only an `sbom:` selector, `dir:`/`docker:`/`oci-archive:`/
  `registry:` selectors are barred, and no SBOM output format is requested.
- Grype facts are grounded: `sbom:<file>` source, `-o json`, `--file`, `-c`, `GRYPE_DB_CACHE_DIR`,
  `GRYPE_DB_AUTO_UPDATE`, `GRYPE_DB_VALIDATE_AGE`, `GRYPE_CHECK_FOR_APP_UPDATE`, `grype db import`,
  and the JSON envelope (`matches`, `ignoredMatches`, `descriptor` with database status) are real
  Grype features.
- Failed and suppressed scans: exit policy admits 0 only, non-zero/signal/stream-bound map to
  `failed`, `-q`/`--only-fixed`/`--vex`/`--fail-on` etc. are barred, missing database is `failed`
  (AC-5), and quoin FR-149 refuses a report with non-empty `ignoredMatches`.
- `quire validate` reports no warning for the three changed files.
- No hash, digest, pin or tracking record is specified (`producer.version` is the FR-019 request
  identity field, as in FR-038). No paywalled standard is named or quoted. No roadmap or
  research-repo content.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The `inputs` row claims the declared `sbom` input binds the scan "to the exact SBOM it read", but the argument `sbom:in/sbom.cdx.json` is a `literal`, so Grype opens the pathname, not the FR-019 retained snapshot. FR-019 exposes a snapshot only through an `input_artifact` argument (resolved to the whole descriptor path, which cannot carry an `sbom:` prefix: the reason FR-038 declares no inputs) or an `input_artifact` stdin binding. Fix: bind `stdin` to the `sbom` input and let Grype read the SBOM from stdin (and restate the `sbom:` first-selector Behavior and AC-6 accordingly), or drop the binding claim. | spec/functional/FR-044-grype-advisory-scan-producer-request.md:62; spec/functional/FR-044-grype-advisory-scan-producer-request.md:64 |
| FND-002 | high | AC-2 (and AC-1) can pass with zero matches: "every match carries an `artifact.purl` of a component in that SBOM" is vacuously true for an empty `matches` array, and FR-039 fixtures are fictional, so nothing requires a match ever to occur. The purl mapping that quoin FR-142-AC-19 depends on is never exercised. Fix: require each fixture to pin at least one package version with a known advisory in the imported database and assert that match is present with that component's purl. | spec/functional/FR-044-grype-advisory-scan-producer-request.md:120 |
| FND-003 | low | The barred-option list ends with the open catch-all "or any option that suppresses diagnostics, hides a match or changes the exit code", which AC-6 cannot inspect; `--add-cpes-if-none` adds matches rather than hiding them. Fix: make AC-6 assert the `arguments` list equals the definition exactly, which bars every other option, and drop the catch-all. | spec/functional/FR-044-grype-advisory-scan-producer-request.md:93; spec/functional/FR-044-grype-advisory-scan-producer-request.md:124 |
| FND-004 | low | CON-1 and AC-9 say "an existing path of the right kind" without saying which kind. Fix: as FR-038-CON-1 does, state `EA_TEST_GRYPE` must name an executable regular file and `EA_TEST_GRYPE_DB` a directory. | spec/functional/FR-044-grype-advisory-scan-producer-request.md:113 |
| FND-005 | low | "Why Grype" justifies the choice against "the four properties the ticket requires" and says the caller "pins" the database. A spec must stand alone; the properties are already the table rows. Fix: say "the four properties below", and say the caller imports the database and disables auto-update rather than "pins". | spec/functional/FR-044-grype-advisory-scan-producer-request.md:36; spec/functional/FR-044-grype-advisory-scan-producer-request.md:42 |

## Verdict

FR-044 is a close, well-grounded mirror of FR-038 and keeps NFR-006's one-SBOM, one-path rule.
The Grype choice is justified on real Grype behaviour. Not mergeable until FND-001 (the input
binding claim is false against FR-019) and FND-002 (the match ACs are vacuous) are fixed; the low
findings are wording fixes for the same round.
