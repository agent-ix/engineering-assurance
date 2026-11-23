---
id: SR-20261115040625
title: "spec-review/failure-domain review of EA-135 gitsign identity registry"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/engineering-assurance; spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md; spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md; spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md"
review_set: subset
---

## Summary

Ticket: EA-135. Failure modes, identity confusion and trust-root edge cases of the committed
identity registry, including the lead-accepted caller choice of `<rev>`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The output does not say which commit the registry came from, and FR-040 overclaims. The caller picks `<rev>` in its own repository, so it can commit a registry naming any person for its own identity on a local branch or a detached commit and point `<rev>` at it. That is acceptable only if a consumer can see and check which commit was used, but FR-041/FR-080 `trust` echo only the request string `<rev>:<path>`. A branch name moves, so a consumer can neither check that the commit is on protected history nor re-run against the same bytes, which is the stated purpose of the echo. FR-040 Description says the mapping is "evidence a caller cannot supply by itself", which is false under this design. Fix: echo the resolved full commit id with the `identity_registry` trust input (this has a real consumer: an ancestry check and a re-run), and reword the Description to say the registry is only as trustworthy as the commit the consumer accepts. | spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md:29-43,161-165; spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:91-94; spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:86-88 |
| FND-002 | low | Trust-source address edge cases are unstated: an empty `<rev>` (`:path`, which `git cat-file` reads from the index) is not listed as invalid; whether a `<rev>` naming an annotated tag is peeled to its commit or refused is open; and splitting at the first `:` excludes valid revision syntax containing `:` (`HEAD@{12:00}`, `:/text`) without saying so. The section also does not say replace refs and grafts are ignored. The host runner already does that, but EA-138 is meant to reuse this section. | spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:75-88 |

## Verdict

PASS with findings for the failure-domain lens. Exact string equality, closed registry members, the duplicate rule, the 4096/1 MiB bounds, read-once and no working tree/index are all sound, and the unmapped-but-`signed_valid` result fails closed for approval (FR-041). Security judgement on the lead-accepted caller-chosen `<rev>`: it is not a defect in itself, because it is the same caller-controlled trust model as the SSH allowed-signers path. It becomes one only because the resolved commit is not reported (FND-001). The FR-037 reading section is reusable for EA-138 as written, apart from the FND-002 gaps.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-040 Description now says the mapping is only as trustworthy as the commit the consumer accepts. The verdict carries `identity_registry_commit`, which FR-041, FR-043 and FR-080 echo in `trust`, so a consumer can run an ancestry check and re-run. One echo case still contradicts AC-30; it is recorded as base FND-006. |
| FND-002 | fixed | FR-037 now refuses an empty `<rev>` or path before git starts, peels an annotated tag to its commit, says a revision containing `:` cannot be addressed, and ignores replace refs and grafts (AC-19, AC-20, AC-21). |
