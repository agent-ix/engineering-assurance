---
id: SR-904
title: "Spec review base — EA-145 trusted-time requirements"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance; changed requirements in FR-040 and FR-041 in diff origin/main...daf8522eaea699d4ee2d0d6725d49c8f973bc0ba; EA-145"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/FR-040", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-041", type: references }
---

## Summary

Base checklist review of the changed trusted-time requirements and their criterion links. The new live AC-28/29 acceptance criteria require version, object, mutation, identity, issuer, digest and service-outcome records; AC-28 also pins an exact gitsign version. This creates a tracking/provenance record and tool-version pin outside the checklist's narrow identity-digest exception.

## Verdict

**FAIL** — remove the tool pin and extra tracking/provenance recording obligations before implementation; retain only evidence needed to test trusted-time behavior and bind the signed proof to its exact artifact.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | AC-28 requires exact `gitsign v0.18.0` and a tracking record containing version, tag ref/object id, identity, issuer, artifact digest and service outcome; AC-29 requires source object ids and each mutation to be recorded. This is an unasked-for provenance ledger/tool-version pin, not the narrow digest needed to bind a signed proof to its artifact. | FR-041-AC-28; FR-041-AC-29 |

## Review evidence

- Reviewed changed FR-040 constraint FR-040-CON-2 and FR-041 behavior/constraints/acceptance criteria, including AC-28/29 and their links.
- ID formats in changed scope are valid; no duplicate or missing IDs were introduced in the changed criteria. Criterion references resolve to FR-040/FR-041.
- An applicable `AssuranceProfile` was not present under `spec/`.
- The narrow identity digest used to verify the Rekor hashedrekord artifact is allowed by the checklist. The additional records of exact tool version, tag object identifiers, identity/issuer, trust-service outcome and each mutation are not needed to bind that proof and create tracking/provenance records.
- The original FR-041-AC-34/35/36 trust-source digest and tool-version requirements were not changed by this diff; this finding is limited to newly changed AC-28/29.

## Findings data

```yaml
clean: false
scope:
  - {id: FR-040-CON-2, path: spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md, role: examined, excerpt: "The artifact supplied to that verifier SHALL be the CMS signed attributes re-encoded as DER with the explicit universal SET OF tag, not the signed-content digest; the library SHALL compute the artifact digest from those bytes."}
  - {id: FR-041-AC-28, path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md, role: examined, excerpt: "Against a genuine gitsign-signed annotated tag carrying an embedded Rekor entry, with real gitsign v0.18.0 and available Sigstore trust services... The test records the gitsign version, tag reference and resolved object id, signed-attributes artifact digest, identity, issuer, and network/trust-service outcome."}
  - {id: FR-041-AC-29, path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md, role: examined, excerpt: "Starting from the genuine live tag and entry used for AC-28... The test records its source tag object ids and each mutation."}
findings:
  - fnd: FND-001
    severity: high
    confidence: high
    method: spec-review/base
    check: other
    artifact_id: FR-041-AC-28
    related_ids: [FR-041-AC-29]
    path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md
    lines: "368-369"
    excerpt: "Against a genuine gitsign-signed annotated tag carrying an embedded Rekor entry, with real gitsign v0.18.0 and available Sigstore trust services... The test records the gitsign version, tag reference and resolved object id, signed-attributes artifact digest, identity, issuer, and network/trust-service outcome."
    finding: "The checklist prohibits exact tool-version pins and tracking/provenance records except a narrow identity digest that binds a signed statement/proof to its exact artifact. AC-28 requires gitsign v0.18.0 and records version, ref/object id, identity, issuer and service outcome; AC-29 requires source object ids and each mutation be recorded. Remove those pin and tracking-record obligations. The signed-attributes artifact digest itself may remain only insofar as it is the identity digest needed for verification."
bindings: []
```
