---
id: SR-905
title: "Spec review EARS conformance — EA-145 trusted-time requirements"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/engineering-assurance; changed requirement statements in FR-040 and FR-041 in diff origin/main...daf8522eaea699d4ee2d0d6725d49c8f973bc0ba; EA-145"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/FR-040", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-041", type: references }
---

## Summary

Quire's grammar check reports eight findings on the changed requirements: two on FR-040-CON-2, five across changed FR-041 behavior/constraints, and one on FR-041-CON-2. Most are compound obligations; one trigger is non-canonical.

## Verdict

**FAIL** — split compound obligations and state the actors/triggers in canonical, testable forms. The intended technical behavior is discernible, but the current statements weaken traceability and consistent interpretation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-040-CON-2 contains multiple `SHALL` obligations and mixes `shall` with `may`, preventing one-to-one requirement traceability. | FR-040-CON-2 |
| FND-002 | medium | FR-041's extraction requirement uses agentless passive wording (“reader SHALL be used”), leaving the responsible actor unallocated. | FR-041 |
| FND-003 | low | FR-041 uses “On the gitsign and transparency-log path” as a trigger; use a canonical `When`/`While` trigger. | FR-041 |
| FND-004 | medium | FR-041's root-injection statement is unclassifiable and omits a subject before `SHALL`, making the actor and condition unclear. | FR-041 |
| FND-005 | medium | FR-041-CON-2 combines multiple `SHALL` obligations and mixes modal strengths in one constraint. | FR-041-CON-2 |

## Review evidence

- Ran `quire validate --scope .` on FR-040 and FR-041; exit 0 with 8 grammar warnings: non-singular 2, mixed-modal 2, non-canonical-trigger 1, unclassifiable 1, missing-subject 1, agentless-passive 1. These are advisory diagnostics, not validation failures.
- Semantic review found the changed technical responses concrete; the problems are statement structure and allocation, not missing behavior definitions.

## Findings data

```yaml
clean: false
scope:
  - {id: FR-040-CON-2, path: spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md, role: examined, excerpt: "The implementation SHALL contain no cryptographic, certificate-parsing or Sigstore client code of its own, and add no crate dependency that performs signature verification, with one exception: transparency-log time verification..."}
  - {id: FR-041, path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md, role: examined, excerpt: "The command SHALL take a trusted time only for an approved result whose signature_kind is gitsign... The CMS/DER reader SHALL be used only to extract..."}
  - {id: FR-041-CON-2, path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md, role: examined, excerpt: "The command SHALL contain no signature, certificate or trailer-parsing code of its own... a CMS/DER reader crate may be used only to extract... the library SHALL compute the artifact digest..."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: spec-review/ears-conformance
    check: compound
    artifact_id: FR-040-CON-2
    related_ids: []
    path: spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md
    lines: "307"
    excerpt: "The artifact supplied to that verifier SHALL be the CMS signed attributes re-encoded as DER with the explicit universal SET OF tag, not the signed-content digest; the library SHALL compute the artifact digest from those bytes. A CMS/DER reader crate may be used only to extract that signed-attributes DER..."
    finding: "Quire reports non-singular and mixed-modal at FR-040-CON-2: multiple SHALL clauses and a mix of SHALL/may place separate obligation strengths in one constraint. Split into atomic requirements."
  - fnd: FND-002
    severity: medium
    confidence: high
    method: spec-review/ears-conformance
    check: ambiguous
    artifact_id: FR-041
    related_ids: []
    path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md
    lines: "288"
    excerpt: "The CMS/DER reader SHALL be used only to extract the re-encoded signed-attributes DER, CMS signature bytes, signing certificate and Rekor TransparencyLogEntry..."
    finding: "Quire reports agentless passive wording at FR-041 line 288 (`SHALL be used`); name the actor that uses the reader so responsibility is allocated."
  - fnd: FND-003
    severity: low
    confidence: high
    method: spec-review/ears-conformance
    check: other
    artifact_id: FR-041
    related_ids: []
    path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md
    lines: "296"
    excerpt: "On the gitsign and transparency-log path the command SHALL NOT judge any certificate at the local clock; the SSH path delegates its validity judgement to git, as above."
    finding: "Quire reports a non-canonical trigger `On`; state the event/state with a canonical When/While form."
  - fnd: FND-004
    severity: medium
    confidence: high
    method: spec-review/ears-conformance
    check: ambiguous
    artifact_id: FR-041
    related_ids: []
    path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md
    lines: "304"
    excerpt: "SHALL make the root injectable so a test supplies its own. The compiled-in root is offline; the command SHALL NOT fetch a root..."
    finding: "Quire reports unclassifiable and missing-subject at FR-041 line 304; the root-injection obligation has no named subject and no canonical EARS pattern. State who makes the root injectable and under what test condition."
  - fnd: FND-005
    severity: medium
    confidence: high
    method: spec-review/ears-conformance
    check: compound
    artifact_id: FR-041-CON-2
    related_ids: []
    path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md
    lines: "328"
    excerpt: "The command SHALL contain no signature, certificate or trailer-parsing code of its own... the library SHALL compute the artifact digest... Engineering Assurance SHALL write no SET, inclusion-proof, certificate-window or payload-binding logic of its own."
    finding: "Quire reports non-singular and mixed-modal at FR-041-CON-2: multiple SHALL clauses and a mix of SHALL/may in one constraint. Split these obligations."
bindings: []
```
