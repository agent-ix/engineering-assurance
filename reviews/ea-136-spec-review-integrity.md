---
id: SR-906
title: "Spec review integrity — EA-145 trusted-time requirements"
type: SpecReview
analysis: integrity
scope: "agent-ix/engineering-assurance; changed FR-040/FR-041 requirements in diff origin/main...daf8522eaea699d4ee2d0d6725d49c8f973bc0ba; EA-145"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/FR-040", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-041", type: references }
---

## Summary

Integrity review of the changed trusted-time requirements found no internal contradiction or hidden assumption that changes the intended design. FR-040 and FR-041 consistently assign Rekor verification to `sigstore-verify`, specify the artifact and CMS signature inputs, and define missing/unverified outcomes. Test coverage and EARS issues are recorded in their separate method artifacts.

## Verdict

**PASS** — no integrity finding in the changed requirements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review evidence

- Checked requirement completeness, linked FR/US/StR/verification relations, internal consistency, hidden assumptions around external tools and services, atomicity, observability, and testability for the changed FR-040/FR-041 text.
- The trusted-time design consistently takes signed-attributes DER, raw CMS signature bytes, certificate and Rekor entry as inputs to the existing verifier; it specifies that the library computes the artifact digest and that FR-040 verifies the signed attributes against the same immutable tag payload.
- Absence, invalid bundle and failed verifier outcomes are distinct and fail closed; status and approval fields remain unchanged.
- The FR-041 acceptance criteria include real live cases with explicit unavailable/not-executed semantics. Their missing execution coverage is reported separately in gap-analysis.
- No applicable `AssuranceProfile` was present under `spec/`; no full failure-domain analysis was selected or run.

## Findings data

```yaml
clean: true
scope:
  - {id: FR-040-CON-2, path: spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md, role: examined, excerpt: "The artifact supplied to that verifier SHALL be the CMS signed attributes re-encoded as DER with the explicit universal SET OF tag..."}
  - {id: FR-041-AC-28, path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md, role: examined, excerpt: "Against a genuine gitsign-signed annotated tag carrying an embedded Rekor entry... when sigstore-verify verifies the signed-attributes DER artifact, the Rekor entry and signature, and the certificate window..."}
  - {id: FR-041-AC-29, path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md, role: examined, excerpt: "Starting from the genuine live tag and entry used for AC-28, changing integratedTime, the SET, the inclusion proof, substituting a genuine entry from a different tag..."}
  - {id: FR-041-AC-33, path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md, role: examined, excerpt: "The trusted-time code uses a CMS/DER reader only to extract the re-encoded signed-attributes DER, CMS signature bytes, signing certificate and log entry..."}
findings: []
bindings: []
```

## Dispositions

### Round 2 — HEAD 9fa56daa0196f4c62bbd8788df5279726b81b98d

The changed requirements remain internally consistent. The generated offline
fixtures exercise the verifier boundary under a test-only root, and the spec
continues to distinguish them from genuine gitsign/production-root evidence.
No integrity finding was added.

```yaml
dispositions: []
```

### Round 3 — HEAD cec9ee8626c9ce65696040d2e4172bf7f67b584a

The latest change is limited to trace-attribute spelling. The requirements
remain internally consistent; the clean verdict and empty findings set stand.

```yaml
dispositions: []
```
