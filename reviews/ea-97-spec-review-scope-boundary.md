---
id: SR-20261113170519
title: "Scope-boundary review of release attestations and per-change receipts (engineering-assurance PR 232)"
type: SpecReview
analysis: scope-boundary
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-065..FR-079 (PR #232 diff, head 5bab3f1); cross-repo boundary with quoin"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-069
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-065
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-077
    type: references
---

# SR-20261113170519: Scope-boundary review of release attestations and per-change receipts (engineering-assurance PR 232)

## Summary

Ticket: EA-97 (EA-98, EA-107, EA-108, EA-109). Allocation between Engineering Assurance, Quoin, the CI
workflows and the related tickets. Ticket text was read as data.

The author's numbered discrepancy list (decisions 1 to 11) was not in the PR body, the tickets or the
brief; only decisions 3, 4, 5, 8 and 11 were described, in the dispatching brief. Those five are assessed
here and in the failure-domain and security reviews. Decisions 1, 2, 6, 7, 9 and 10 were not assessed
individually.

- Decision 3 (FR-065's review gate in place of EA-109's "approval attestation present and valid"
  check): sound. A pre-merge check cannot verify a statement whose subject, the merged commit, does not
  exist yet; FR-065 already says this. The rest of EA-109 is not covered; see FND-002.
- Decision 8 (direct Sigstore signing for receipts): sound, because the bundles must be held for git notes
  and NFR-010 allows the delegated libraries. It is conditional on the visibility-dependent signing path
  (security FND-003).
- Decision 11 (the Quoin test-result export has no Quoin FR): confirmed as a cross-repo spec gap; FND-001
  gives the requirement text.

Checked clean: Engineering Assurance owns signing, verification and storage. Quoin is limited to a pure
export with no git, no network and no signing, consistent with NFR-010. Syft stays the only SBOM tool
(NFR-006). The CycloneDX schema is taken from the library dependency, not copied. The release decision stays
with FR-047, FR-048 and NFR-007.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Cross-repo spec gap (decision 11): FR-069 depends on a Quoin export that no Quoin requirement specifies. Fix: the quoin lane adds a requirement with this text. "Quoin SHALL provide a command that writes, from the retained evidence store and to a caller-named file, one JSON document that is an in-toto Test Result v0.1 predicate (the in-toto Test Result v0.1 predicate type) with exactly the members `result`, `configuration`, `passedTests`, `warnedTests` and `failedTests`; each test entry is the criterion or test id the evidence store records; a failed or stale result is listed in `failedTests` and is never in `passedTests`; `result` is `FAILED` when any entry is failed or stale, `WARNED` when any is warned, and `PASSED` otherwise; the output is the RFC 8785 canonical form, byte-identical for an identical store; the command runs no git, no network call and no signing; an unreadable or invalid store exits non-zero with a named error and writes no file." Track it as a blocking edge on the EA-97 code ticket and cite the Quoin FR id in FR-069 once it exists. | spec/functional/FR-069-test-result-export-is-consumed-as-data.md:16 |
| FND-002 | medium | EA-109's remaining scope is allocated nowhere. FR-070 says the review gate "is a separate workflow outside this requirement", but no requirement names that workflow file or the checked-in ruleset (required check, up to date before merge, stale approvals dismissed, author may not approve, bypass list), and FR-077-AC-3 depends on a ruleset bypass existing. Fix: state in FR-065 or FR-070 that the gate workflow and the ruleset definition are specified under EA-109 in a later change, so the boundary is explicit, or specify them here. | spec/functional/FR-070-release-and-receipt-workflow-files.md:30; spec/functional/FR-077-missing-receipt-check.md:20 |
| FND-003 | low | EA-98's acceptance text (untrusted ticket data) still asks for "a merge-time check" that fails a PR with a missing receipt. The spec, following the no-merge-queue ruling, makes the receipt post-merge, with the scheduled missing-receipt check as the detector. Nothing in the spec says it replaces the merge-time check, so a later acceptance check against EA-98 would fail. Fix: one sentence in FR-077 stating that the post-merge check takes the place of a merge-time check because there is no merge queue, and update the ticket's acceptance to match. | spec/functional/FR-077-missing-receipt-check.md:20; spec/functional/FR-075-push-to-main-receipt-workflow.md:22 |
| FND-004 | low | FR-079's bullet "The implementing change SHALL write the literal ... URIs only after the check admits the prefix" allocates a sequencing step to a change, not to a system component. Fix: drop it; the description already says this, and FR-079-AC-1 and AC-4 test the outcome. | spec/functional/FR-079-content-rights-admits-the-attestation-schema-prefix.md:24 |

## Verdict

Mergeable on scope once FND-001 is turned into a Quoin requirement request with a blocking edge (the EA spec
itself only needs the citation) and FND-002 states the EA-109 boundary.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-069's quoted Quoin capability and PLAT-1687 (the Quoin ticket that blocks EA-97 and EA-108) disagree. PLAT-1687 exports a full Test Result statement with a caller-supplied subject and lists only passed and failed entries, with no stale or WARNED rule; FR-069 consumes a predicate file and adds the subjects itself. FR-069 does not name PLAT-1687, so the dependency cannot be followed. Fix: cite PLAT-1687 in FR-069, and later the Quoin FR id that replaces the quoted text; ask the quoin lane to align PLAT-1687's scope with the quoted capability (predicate only, stale in `failedTests`, the `result` rule). | spec/functional/FR-069-test-result-export-is-consumed-as-data.md:18 |

## Dispositions

Round 1 rows (findings of the review pass) at head 428c2bd; round 2 rows (findings of disposition pass 1, listed after them) at head a6895e7.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | The EA side now states the required Quoin capability in FR-069; the Quoin requirement itself is tracked by PLAT-1687, which blocks EA-97 and EA-108 in Linear. Its scope does not yet match FR-069 (new FND-005). |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
