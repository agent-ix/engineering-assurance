---
id: SR-20261124033409
title: "spec-review (ears-conformance) of per-signature gesture evidence and interactive approval-tag mode (engineering-assurance PR 305)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/engineering-assurance@18293e76 (PR 305, diff origin/main...HEAD); new or edited SHALL statements in spec/functional/FR-042, FR-084, FR-085, FR-086, FR-087"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-084
    type: references
---

# EARS review of EA PR 305

## Summary

Ticket: EA-113. This lens checked each added or edited requirement statement for EARS form: one trigger or state, one responsible system, one response, and no pronoun reference. Most statements were clean: FR-084 Behavior bullets 2-7 and CON-1/2; FR-085 Behavior bullets 1-7 and 9, plus CON-1/2; FR-086 Behavior and CON-1; FR-087 Behavior. No statement uses "it" as its subject. Two statements are not in EARS form; both findings are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-084 Behavior bullet 1 holds two SHALL clauses ("SHALL decide the key type ... and SHALL NOT parse the signature block itself"). Split them; the second already duplicates FR-084-CON-1. | spec/functional/FR-084-evidence-of-the-per-signature-gesture-in-an-ssh-signature.md:68-69 |
| FND-002 | low | FR-087's Description holds a requirement ("The README SHALL explain them ..."), outside Behavior, and duplicates Behavior bullet 1. Make the Description prose descriptive. | spec/functional/FR-087-document-signing-gestures-and-shared-agent-sessions.md:22 |

## Verdict

PASS with two low findings. The new statements are atomic, name the command as the actor and are free of pronoun references. The FR-042 edited timeout bullet has a consistency problem, which the integrity lens records.
