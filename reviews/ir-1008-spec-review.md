---
id: SR-16301
title: "spec-review of NFR-005 npm release transition section (engineering-assurance PR 187)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@1b7ff8c3; spec/non-functional/NFR-005-rust-containment-and-traceability.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/NFR-005
    type: references
---

# Spec review of the NFR-005 npm release transition section (engineering-assurance PR 187)

## Summary

Ticket: IR-1008. Scope: the 30 lines this PR adds to NFR-005, read against the unchanged NFR-005-AC-6 and AC-7. Sub-analyses applied by changed scope: integrity (consistency between the new statement and the existing ACs) and EARS (the new SHALL statements). No relationships, domain objects or dependencies changed, so the other sub-analyses do not apply. `quire validate` on NFR-005 passes. The new statements are atomic enough, testable, and agree with the code at 1b7ff8c3. Their wording that authority comes from the base and that no publication is attested is precise. The one inconsistency is with AC-7, recorded below. The missing dedicated AC is recorded in SR-16300 FND-001 and not repeated here.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | NFR-005-AC-7 still says that newly added or expanded dependencies produce typed refusal findings, with no exception. The new section now sanctions changing existing optional-dependency versions with no finding, and the accepting test `platform_release_updates_only_existing_owned_versions` is tagged to AC-7. A reader of AC-7 alone could treat a version change as an expanded dependency and judge the accepting test a violation. The criterion and the statement disagree in the same document. Fix: qualify AC-7 with "except the sanctioned owned-release transition in Existing npm package release transitions" (or add a dedicated AC, see SR-16300 FND-001). | spec/non-functional/NFR-005-rust-containment-and-traceability.md:79; spec/non-functional/NFR-005-rust-containment-and-traceability.md:27-55 | wrong-requirement |

## Verdict

Not mergeable as is, because of FND-001 (medium). The fix is a one-line AC-7 qualification or a new AC, with no change to code or spec/tests.md.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Head 56bed14d qualifies NFR-005-AC-7 with "except the exact version findings for the sanctioned existing-family release transition in AC-9", and adds AC-9. The accepting test is now tagged AC-9 and AC-6, not AC-7. |
