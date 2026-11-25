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
| FR-001 | Onboarding could recommend creating new artifacts even when an applicable valid artifact already exists, or proceed despite missing decision-boundary input. | Use valid, absent, conflicting, malformed, unavailable, and incomplete repository fixtures; assert reuse/no-authoring recommendations, bounded paths, and summary statuses for each case. |
| FR-002 | Discovery could select a duplicate or out-of-bundle skill/workflow as canonical, or silently succeed when the canonical target is missing. | Inspect the packaged repository for exactly one canonical skill and its declared workflows; remove the target and move it outside the bundle in separate fixtures and assert discovery fails. |
| FR-003 | The installed wheel could omit or alter a skill, workflow, manifest, schema, or guide needed for onboarding. | Build the wheel in a temporary directory and compare its member set with the required bundle; validate each included schema against its skeleton and the documented manifest contracts. |
| FR-006 | A supported host/scenario combination could be omitted or could infer acceptance/rejection from an incomplete producer result. | Execute the five required scenario classes on all four supported hosts, including malformed and unavailable producer cases; assert both explicit terminal choices per host and no inferred decision. |
| FR-007 | A migration could rename or remove a pilot workflow, breaking existing callers. | Load each of the four original pilot invocations with ix-flow and compare the exported workflow names with the pre-promotion set; assert added or missing names fail. |
| FR-013 | Campaign migration could omit a family or prescribe incompatible keep/delete/replace actions, causing consumers to migrate data inconsistently. | Walk every family in the migration decision table and assert exactly one action with a reason; verify the contract makes no qualification claim. |
| FR-014 | The published Rust crate or CLI boundary could expose an unintended optional dependency or fail for a minimal downstream consumer. | Build a minimal downstream consumer against the root library and CLI; verify the default feature set is empty and feature-specific consumers resolve only the declared dependencies, including no serde_json for default-off consumers. |
| FR-018 | Retiring a legacy executable path could remove behavior before the Rust path agrees, or leave old semantic/assertion code active. | Attempt retirement with differing old/new outputs and assert refusal; verify direct invocations use the Rust interface, then scan executable paths for unapproved non-Rust semantics while excluding inert fixtures. |
| FR-028 | The shared CLI wrapper could alter primary document bytes, stderr role, exit status, or admit an unintended dependency URL. | Run successful and rejected fictional workflow requests through the real CLI and compare bytes, stderr, and status; place the shared dependency URL in root metadata, nested metadata, prose, and lookalike forms and assert only the allowed root entry passes. |
| FR-083 | Adopters could believe tag or private-record protections cover notes refs that the organization rulesets cannot protect. | Inspect the adoption guide for the exact tag and private-record branch patterns, deletion/update restrictions, and explicit notes-ref limits; confirm no command or workflow claims to apply or inspect a ruleset. |
| FR-087 | Readers could mistake a caller-declared gesture for device evidence or miss the documented limit of a shared agent session. | Inspect README for all four gesture names, whether each is declared or evidenced, and the shared-session limit; assert the document contains no paywalled standard designator. |
| FR-088 | A vulnerability reporter could be sent to a public channel or given response/security promises the maintainers have not adopted. | Inspect SECURITY.md for a direct private GitHub reporting link, an explicit exclusion of public issues/discussions/PRs, and absence of time, version, safe-harbor, severity, or disclosure promises. |

## Rationale

This is a proposed supplier-side rating for IU-001. The named human assigned to review and accept it is Peter Krenesky. The record is unsigned and unaccepted pending his decision through the signed-commit workflow. No score or product-safety classification is assigned.

These features primarily provide discovery, packaging, compatibility, migration, or explanatory documentation. Their failure can disrupt use or reduce clarity, but does not itself verify evidence or make a human approval decision. These proposed ratings are human judgments for review, not outputs calculated from code, test, coverage, or performance data.

## Assurance activities

| Activity | Acceptance criterion |
|---|---|
| Use valid, absent, conflicting, malformed, unavailable, and incomplete repository fixtures; assert reuse/no-authoring recommendations, bounded paths, and summary statuses for each case. | FR-001-AC-1–9 |
| Inspect the packaged repository for exactly one canonical skill and its declared workflows; remove the target and move it outside the bundle in separate fixtures and assert discovery fails. | FR-002-AC-1–5 |
| Build the wheel in a temporary directory and compare its member set with the required bundle; validate each included schema against its skeleton and the documented manifest contracts. | FR-003-AC-1–8 |
| Execute the five required scenario classes on all four supported hosts, including malformed and unavailable producer cases; assert both explicit terminal choices per host and no inferred decision. | FR-006-AC-1–7 |
| Load each of the four original pilot invocations with ix-flow and compare the exported workflow names with the pre-promotion set; assert added or missing names fail. | FR-007-AC-1–4 |
| Walk every family in the migration decision table and assert exactly one action with a reason; verify the contract makes no qualification claim. | FR-013-AC-1–8 |
| Build a minimal downstream consumer against the root library and CLI; verify the default feature set is empty and feature-specific consumers resolve only the declared dependencies, including no serde_json for default-off consumers. | FR-014-AC-1–9, FR-014-AC-11 |
| Attempt retirement with differing old/new outputs and assert refusal; verify direct invocations use the Rust interface, then scan executable paths for unapproved non-Rust semantics while excluding inert fixtures. | FR-018-AC-1–4 |
| Run successful and rejected fictional workflow requests through the real CLI and compare bytes, stderr, and status; place the shared dependency URL in root metadata, nested metadata, prose, and lookalike forms and assert only the allowed root entry passes. | FR-028-AC-1–4 |
| Inspect the adoption guide for the exact tag and private-record branch patterns, deletion/update restrictions, and explicit notes-ref limits; confirm no command or workflow claims to apply or inspect a ruleset. | FR-083-AC-1–4 |
| Inspect README for all four gesture names, whether each is declared or evidenced, and the shared-session limit; assert the document contains no paywalled standard designator. | FR-087-AC-1–5 |
| Inspect SECURITY.md for a direct private GitHub reporting link, an explicit exclusion of public issues/discussions/PRs, and absence of time, version, safe-harbor, severity, or disclosure promises. | FR-088-AC-1–3 |