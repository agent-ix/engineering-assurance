---
id: SR-20261113015354
title: "Failure-domain review of FR-044 Grype advisory-scan producer request (engineering-assurance PR 221)"
type: SpecReview
analysis: failure-domain
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-044-grype-advisory-scan-producer-request.md (PR #221 diff)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-044
    type: references
---

# SR-20261113015354: Failure-domain review of FR-044 (engineering-assurance PR 221)

## Summary

Ticket: PLAT-1601. Examined the FR-044 failure paths: unavailable executable (AC-3), non-zero
exit and timeout (AC-4), missing database (AC-5), stale output removal (AC-7), offline run (AC-8),
the barred options, and the database-age rule. Context: FR-039 container fixture definition.

Clean: every non-`completed` state maps to an existing FR-019 state and none presents a report;
an empty database is `failed`, never a clean scan; the stale output file is removed before launch
by FR-019; network use is closed by `GRYPE_DB_AUTO_UPDATE=false` and
`GRYPE_CHECK_FOR_APP_UPDATE=false`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Behavior says "a missing, unreadable or stale database makes Grype exit non-zero and is therefore `failed`", but the environment sets `GRYPE_DB_VALIDATE_AGE=false`, which turns Grype's age check off, and the Inputs text says recency stays with the consumer's gate. A stale database therefore completes. Fix: drop "stale" from the Behavior sentence. | spec/functional/FR-044-grype-advisory-scan-producer-request.md:98; spec/functional/FR-044-grype-advisory-scan-producer-request.md:75 |
| FND-002 | medium | The container case cannot produce OS-package matches. The FR-039 container fixture has no `os-release`, so the Syft SBOM carries no `operating-system` component and Grype cannot identify the distribution to match `deb`/`apk`/`rpm` packages against distribution advisories. The "Why Grype" rationale ("matches OS packages ..., which the container fixtures need") and AC-2's container case rest on a fixture that yields an empty scan. Fix: give the container fixture an `os-release` (amending FR-039-AC-4) or a FR-044-specific container fixture, and require at least one OS-package match. | spec/functional/FR-044-grype-advisory-scan-producer-request.md:47; spec/functional/FR-044-grype-advisory-scan-producer-request.md:120 |

## Verdict

Failure paths are complete and conservative. Fix FND-001 (self-contradiction) and FND-002 (the
container case is empty by construction) in the same round as the base findings.
