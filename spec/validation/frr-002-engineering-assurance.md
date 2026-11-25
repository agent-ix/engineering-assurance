---
id: FRR-002
title: "Engineering Assurance proposed not-high process-risk supplier assessment"
type: FeatureRiskRating
scope: supplier
rating: not_high_process_risk
rated_by: Peter Krenesky
relationships:
  - target: "ix://agent-ix/engineering-assurance/IU-001"
    type: uses_intended_use
  - target: "ix://agent-ix/engineering-assurance/FR-001"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-002"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-003"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-006"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-007"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-013"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-018"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-028"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-083"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-087"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-088"
    type: rates_feature
---
# Engineering Assurance proposed not-high process-risk supplier assessment

## Feature assessments

| Requirement | Failure mode | Detection |
|---|---|---|
| FR-001 | A defect in “Inventory repository assurance context before proposing artifacts” could make the operator workflow incompatible or less clear, but it does not itself validate evidence or make a human decision. | Run each declared acceptance case for FR-001; verify output and invocation behavior and confirm that no evidence status or human decision changes. |
| FR-002 | A defect in “Expose one canonical onboarding and workflow bundle” could make the operator workflow incompatible or less clear, but it does not itself validate evidence or make a human decision. | Run each declared acceptance case for FR-002; verify output and invocation behavior and confirm that no evidence status or human decision changes. |
| FR-003 | A defect in “Package the canonical onboarding bundle” could make the operator workflow incompatible or less clear, but it does not itself validate evidence or make a human decision. | Run each declared acceptance case for FR-003; verify output and invocation behavior and confirm that no evidence status or human decision changes. |
| FR-006 | A defect in “Evaluate onboarding behavior through supported agents” could make the operator workflow incompatible or less clear, but it does not itself validate evidence or make a human decision. | Run each declared acceptance case for FR-006; verify output and invocation behavior and confirm that no evidence status or human decision changes. |
| FR-007 | A defect in “Preserve existing pilot workflow invocation” could make the operator workflow incompatible or less clear, but it does not itself validate evidence or make a human decision. | Run each declared acceptance case for FR-007; verify output and invocation behavior and confirm that no evidence status or human decision changes. |
| FR-013 | A defect in “Publish the reviewed campaign migration contract” could make the operator workflow incompatible or less clear, but it does not itself validate evidence or make a human decision. | Run each declared acceptance case for FR-013; verify output and invocation behavior and confirm that no evidence status or human decision changes. |
| FR-014 | A defect in “Expose a versioned Rust library and CLI boundary” could make the operator workflow incompatible or less clear, but it does not itself validate evidence or make a human decision. | Run each declared acceptance case for FR-014; verify output and invocation behavior and confirm that no evidence status or human decision changes. |
| FR-018 | A defect in “Retire replaced Engineering Assurance executable paths” could make the operator workflow incompatible or less clear, but it does not itself validate evidence or make a human decision. | Run each declared acceptance case for FR-018; verify output and invocation behavior and confirm that no evidence status or human decision changes. |
| FR-028 | A defect in “Consume shared CLI output utilities” could make the operator workflow incompatible or less clear, but it does not itself validate evidence or make a human decision. | Run each declared acceptance case for FR-028; verify output and invocation behavior and confirm that no evidence status or human decision changes. |
| FR-083 | A defect in “Document the adoption rulesets that protect tags and the private records branch, and state what cannot be protected” could make the operator workflow incompatible or less clear, but it does not itself validate evidence or make a human decision. | Run each declared acceptance case for FR-083; verify output and invocation behavior and confirm that no evidence status or human decision changes. |
| FR-087 | A defect in “Document the signing gestures and the shared-agent-session limit” could make the operator workflow incompatible or less clear, but it does not itself validate evidence or make a human decision. | Run each declared acceptance case for FR-087; verify output and invocation behavior and confirm that no evidence status or human decision changes. |
| FR-088 | A defect in “Document the private vulnerability reporting route” could make the operator workflow incompatible or less clear, but it does not itself validate evidence or make a human decision. | Run each declared acceptance case for FR-088; verify output and invocation behavior and confirm that no evidence status or human decision changes. |

## Rationale

This is a proposed supplier-side rating for IU-001. The named human assigned to review and accept it is Peter Krenesky. The record is unsigned and unaccepted pending his decision through the signed-commit workflow. No score or product-safety classification is assigned.

These features primarily provide discovery, packaging, compatibility, migration, or explanatory documentation. Their failure can disrupt use or reduce clarity, but does not itself verify evidence or make a human approval decision. These proposed ratings are human judgments for review, not outputs calculated from code, test, coverage, or performance data.

## Assurance activities

| Activity | Acceptance criterion |
|---|---|
| Exercise the stated behavior for FR-001; inspect the output and verify no assurance status, evidence state, or human decision changes. | FR-001-AC-1–9 |
| Exercise the stated behavior for FR-002; inspect the output and verify no assurance status, evidence state, or human decision changes. | FR-002-AC-1–5 |
| Exercise the stated behavior for FR-003; inspect the output and verify no assurance status, evidence state, or human decision changes. | FR-003-AC-1–8 |
| Exercise the stated behavior for FR-006; inspect the output and verify no assurance status, evidence state, or human decision changes. | FR-006-AC-1–7 |
| Exercise the stated behavior for FR-007; inspect the output and verify no assurance status, evidence state, or human decision changes. | FR-007-AC-1–4 |
| Exercise the stated behavior for FR-013; inspect the output and verify no assurance status, evidence state, or human decision changes. | FR-013-AC-1–8 |
| Exercise the stated behavior for FR-014; inspect the output and verify no assurance status, evidence state, or human decision changes. | FR-014-AC-1–9, FR-014-AC-11 |
| Exercise the stated behavior for FR-018; inspect the output and verify no assurance status, evidence state, or human decision changes. | FR-018-AC-1–4 |
| Exercise the stated behavior for FR-028; inspect the output and verify no assurance status, evidence state, or human decision changes. | FR-028-AC-1–4 |
| Exercise the stated behavior for FR-083; inspect the output and verify no assurance status, evidence state, or human decision changes. | FR-083-AC-1–4 |
| Exercise the stated behavior for FR-087; inspect the output and verify no assurance status, evidence state, or human decision changes. | FR-087-AC-1–5 |
| Exercise the stated behavior for FR-088; inspect the output and verify no assurance status, evidence state, or human decision changes. | FR-088-AC-1–3 |
