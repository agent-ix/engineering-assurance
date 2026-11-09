---
id: SR-16312
title: "spec-review of the EA-84 withdrawals and matrix edits"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@ea-84-traceability; spec/spec.md; spec/functional/FR-016-rust-onboarding-and-workflow.md; spec/functional/FR-018-staged-runtime-migration.md; spec/stakeholder/StR-003-rust-native-engineering-assurance.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-016
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-018
    type: references
---

# Spec review of the EA-84 withdrawals and matrix edits

## Summary

Ticket: EA-84. `quire validate` accepts every edited spec file. Outside
historical review records, no live artifact references a withdrawn id
(FR-016-AC-4, FR-018-AC-1/3, FR-018-CON-2/3, TC-097, TC-108, TC-114). The PR adds
no ledger, pin, count assertion or workstation path.

The withdrawals were judged on their merits:

- **FR-018-AC-1 / TC-097** (a per-path state and disposition matrix) is a
  ledger. StR-003-VC-2 moves to TC-115, whose allowlist is the disposition.
  Withdrawing it is sound.
- **FR-018-AC-3 / TC-114 and FR-018-CON-2/CON-3** (rollback and reporting of
  coexistence) are rollback and reporting ceremony for prerelease software.
  Withdrawing them is sound. The stated reason is incomplete (FND-002).
- **FR-016-AC-4 / TC-108.** SR-125 kept TC-108 because it guarded removal of
  the JavaScript invariant providers. That guard is still in place without it.
  `workflow_invariants_parity` (TC-106) runs the canonical `invariants.js`,
  TC-036 resolves the pilot re-export, the package audit lists the pilot
  provider as a required member, and the new TC-115 test requires both files to
  be present. Deleting either provider before a Rust provider replaces it
  therefore still fails. TC-108 also depends on an ix-flow interface owned
  elsewhere, which is the same reason FR-019-AC-6 was retired. Withdrawal is
  sound, but the rationale names the wrong guard (FND-001).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The FR-016-AC-4 withdrawal note says "FR-018-AC-2 and FR-018-AC-4 guard removal of the JavaScript providers". FR-018-AC-2's only binder (TC-113, `content_rights_tree_cli.rs`) never touches the JS providers. The guards that actually stop their removal are TC-106 parity, TC-036 and TC-115. Name those, so the next reader sees that SR-125's concern is answered. | spec/spec.md:157-160 | wrong-requirement |
| FND-002 | low | The FR-018 withdrawal reason says "the Python cutovers are complete and no rollback ... remains to verify". FR-018 still governs the pending JavaScript invariant cutover (FR-016 Downstream). The actual reason is that rollback is not required before release, and the note should say so. | spec/spec.md:165-169 | wrong-requirement |
| FND-003 | low | Repository convention (FR-006, FR-014, FR-024) records "X is withdrawn. Its row is removed and the identifier is not reused." in the owning FR file. FR-016 and FR-018 carry no such line; the note exists only in spec.md. FR-018 `## Outputs` is also left with an empty line where the removed bullet was. | spec/functional/FR-016-rust-onboarding-and-workflow.md:177; spec/functional/FR-018-staged-runtime-migration.md:32-35 | wrong-requirement |
| FND-004 | low | INT-006 keeps its id but changes meaning, from "Load Rust workflow invariants through a structured host interface" to "Drive the ix-flow run lifecycle through the Rust workflow host". This re-points the id instead of withdrawing it, against the withdrawn-not-reused convention. Withdraw INT-006 with FR-016-AC-4 and give the lifecycle row a new id. | spec/tests.md:567 | wrong-requirement |

## Verdict

The withdrawals are justified and the edited documents are structurally valid.
FND-001 should be fixed before merge, because it records a false reason for
dropping a guard that a previous review kept on purpose. FND-002 to FND-004 are
small wording and convention fixes.
