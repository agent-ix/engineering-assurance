---
id: SR-20261115040626
title: "spec-review/ears-conformance review of EA-135 gitsign identity registry"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/engineering-assurance; spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md; spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md"
review_set: subset
---

## Summary

Ticket: EA-135. EARS grammar of new and edited requirement statements. The FR-040 Behavior
additions (exact-equality mapping, no pattern/prefix/case-fold, principal only from the
registry) are well-formed ubiquitous/negative statements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-037 Trust-source reading puts its shall-statements on the input rather than the system ("The entry SHALL be a regular file blob ...; the content SHALL be at most 1 MiB"), and the failure bullet has no SHALL for the unreadable outcome. Recast as unwanted-behaviour: "If the entry is not a regular blob, exceeds 1 MiB or is not UTF-8, then the command SHALL treat the source as unreadable." The FR-040 Behavior ordering sentence is also compound (validate, then refuse, then read, before git/gitsign) and is where FND-001 of the base review sits. | spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:83-93; spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md:216-219 |

## Verdict

PASS with one low finding.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The FR-037 reading rules are now system-subject "The command SHALL" and "If ..., then the command SHALL" statements, and the FR-040 ordering is split into four single-action SHALL statements. |
