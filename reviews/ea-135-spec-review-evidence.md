---
id: SR-20261115040627
title: "spec-review/evidence review of EA-135 gitsign identity registry"
type: SpecReview
analysis: evidence
scope: "agent-ix/engineering-assurance; spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md; spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md; spec/tests.md"
review_set: subset
---

## Summary

Ticket: EA-135. Verification method and value of the new ACs. All new ACs are Test or
Test (stand-in) and can be tested with the existing stand-in gitsign and throwaway repos.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-037-AC-21 makes the reading yield and test a blob id, and asserts that a second read of an unchanged commit gives the same three values. Nothing consumes the blob id: no output carries it, and commit id plus path already determine it. The repeat-read check tests that git is deterministic, not any behaviour of this command. This is read-provenance ceremony. Fix: drop the blob id and the repeat-read clause, and keep only the resolved commit id if it is echoed (failure-domain FND-001); otherwise drop AC-21. | spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:86-88,174 |
| FND-002 | low | spec/tests.md has no TC row for FR-037-AC-19..21, FR-040-AC-25..29 or FR-040-CON-3, although every earlier FR-040 AC has one, and TC-286, TC-287, TC-327 and TC-361 still describe the old request shape (TC-327 repeats the "without starting git" claim of base FND-002). | spec/tests.md:707-708,752,786 |

## Verdict

PASS with findings once FND-001 is resolved.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The blob id and the repeat-read clause are gone. FR-037-AC-21 now tests branch resolution and tag peeling, and the reading yields only the content and the commit id, which `identity_registry_commit` consumes. |
| FND-002 | fixed | TC-1037 to TC-1048 cover FR-037-AC-19..22, FR-040-AC-25..31, CON-3 and FR-047-AC-14, and TC-286, TC-287, TC-327 and TC-361 are updated. The rows still stale are recorded as base FND-007. |
