---
id: SR-065
title: "Base review of the Rust onboarding slice"
type: SpecReview
analysis: base
scope: "FR-016-AC-1, FR-016-CON-4, TC-105"
review_set: base
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: reviews
---

# SR-065: Base review of the Rust onboarding slice

## Summary

This owner-selected base review covers the behavior-preserving Rust port of
repository inventory, onboarding recommendation, and validated artifact
publication. It does not review ix-flow lifecycle, workflow-invariant hosting,
evaluation qualification, consumer migration, or legacy deletion.

The result is **PASS after fixes**. The changed requirement now defines the
versioned request/result boundary, exact inventory populations and ordering,
decision states, Quire ownership, publication transaction, refusal categories,
I/O seam, and adverse test population needed by TC-105.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | **Closed through `/specify`.** “Matches retained onboarding” did not define the request/result fields, inventory collections, ordering, recommendation states, or artifact-path presence rules, so a partial inventory could satisfy it. These are now explicit. | FR-016 Inputs, Outputs, Behavior; FR-016-AC-1; TC-105 |
| FND-002 | high | **Closed through `/specify`.** The prior requirement allowed reusable library code to walk repositories and spawn Quire despite ADR-002 assigning I/O and external hosts to the CLI. FR-016-CON-4 now fixes that seam and TC-105 verifies it. | ADR-002; FR-016-CON-4; TC-105 |
| FND-003 | high | **Closed through `/specify`.** Artifact publication named no transaction boundary, so validation failure, destination races, or an escaping/symlinked target could leave staged, overwritten, or partial bytes. Same-directory staging, Quire validation, sync, atomic no-replace publication, cleanup, and no-write refusals are now required. | FR-016 Behavior; FR-016-AC-1; TC-105 |
| FND-004 | medium | **Closed through `/specify`.** Quire unavailability was not allocated between inventory and publication. The retained inventory outcome remains an invalid validation with `quire-unavailable`; publication refuses because it cannot establish validity. | FR-016 Error Conditions; TC-105 |
| FND-005 | medium | **Closed through `/specify`.** TC-105 did not enumerate duplicate/conflicting artifacts, unsupported types, malformed frontmatter/request input, existing destinations, or absolute/parent/symlink target escapes. The matrix now requires each case and its no-publication oracle. | FR-016-AC-1; TC-105 |
| FND-006 | medium | **Closed through `/specify`.** Byte-identical serialization would make retired PyYAML line-wrapping and scalar-quoting choices normative for newly authored artifacts even though no pre-existing artifact is rewritten. TC-105 now requires equivalent parsed frontmatter, identical Markdown body, and successful Quire validation; preservation of existing bytes remains exact. | FR-016 Behavior; FR-016-AC-1; TC-105 |
| FND-007 | medium | **Closed through `/specify`.** FR-016 required unknown protocol versions to refuse but omitted their stable machine code from the otherwise exhaustive onboarding refusal list. `unsupported_onboarding_protocol` now distinguishes version skew from malformed request structure. | FR-014-AC-3; FR-016 Error Conditions; TC-105 |
| FND-008 | high | **Closed through `/specify`.** Differential probing showed that changing YAML engines silently changed duplicate-key and merge-key artifact identity behavior. Retaining PyYAML's last-key-wins result would admit ambiguous identity, while accepting merge-provided identity would make selection depend on engine-specific expansion. FR-016 now requires one explicit top-level string `type` and fail-closed handling for either construct; TC-105 retains the old/new observations while enforcing the selected Rust behavior. | FR-016 Behavior; FR-016-AC-1; TC-105 |

The maintained Rust YAML implementation is an implementation dependency, not a
new semantic authority. Differential tests remain responsible for showing that
the supported frontmatter and skeleton population preserves retained behavior.
