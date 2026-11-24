---
id: SR-903
title: "Gap analysis — EA-136/138 and EA-145"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/engineering-assurance; origin/main...daf8522eaea699d4ee2d0d6725d49c8f973bc0ba; changed approval, trust, release, trail and signature code/tests; FR-040/FR-041"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/FR-040", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-041", type: references }
---

## Summary

The repository matrix and changed implementation/tests were reconciled for the trusted-time path and the combined EA-136/138 implementation. The central Rekor verification requirement has no exercising test: AC-28 is bound only to timestamp arithmetic and AC-29 has no binder. The source also DER-wraps the CMS signature where Sigstore expects raw octets.

## Verdict

**FAIL** — the new trusted-time acceptance and rejection behavior is not covered by its tagged tests, and the implemented bundle input has a representation defect.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-041-AC-28 is tagged only to a timestamp-formatting/skew unit that receives an already trusted integer; it does not exercise CMS extraction or Rekor verification. AC-29 is untagged and no test exercises invalid/tampered embedded entries. | src/approval_verify_host.rs:330-345; tests/approval_verify_gitsign.rs:1-6; FR-041-AC-28; FR-041-AC-29 | correct-requirement-no-evidence |
| FND-002 | low | Twelve newly added test trace annotations include TC-1049 through TC-1060, but those identifiers have no definitions in the repository; they are dangling matrix entries even where the same annotation also names valid AC ids. | tests/approval_verify_cli.rs:1135; tests/approval_trail_cli.rs:894; tests/release_verify_cli.rs:1625 | correct-requirement-no-evidence |

## Coverage

- Ran `quoin matrix --repo . --json` and `quire coverage --scope . --json`; the whole-repository matrix contains 814 FR-AC criteria among its 1,603 rows, with 1,336 backed rows overall. No run evidence was present in the matrix. Coverage emits duplicate-module and unrelated symbol-id collision diagnostics; these are baseline context, not findings attributed to this diff.
- Changed trusted-time criterion bindings: `FR-041-AC-28` is tagged on `approval_verify_host::tests::trusted_time_is_utc_and_time_skew_is_signed_minus_integrated_time`; the test only formats an integer timestamp and checks skew. `FR-041-AC-29` has no tagged test. The ignored gitsign live test is the pre-existing AC-26 path, uses external live inputs, and does not test the newly added Rekor extraction/verification cases.
- Reverse-gap review: the runtime path is implemented in `src/approval_rekor.rs` and `src/approval_verify_host.rs`, but no fixture/test reaches extraction, Bundle construction, or `Verifier::verify`; the run can silently degrade valid entries to `log_entry_unverified`.
- Stub review: no new production stub or placeholder was found in the reviewed behavior.
- EA-138 content-rights scope is part of this combined implementation review; coder reports `content-rights-tree` accepted 774.
- Focused verification: the six approval/trail/release/signature integration targets passed; the trusted-time arithmetic unit passed 1/1. The single live gitsign integration test remains ignored and was not run. No full suite or all-targets run.
- Plan completion: not assessed.

## Findings data

```yaml
clean: false
scope:
  - {id: FR-041-AC-28, path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md, role: examined, excerpt: "Against a genuine gitsign-signed annotated tag carrying an embedded Rekor entry... when sigstore-verify verifies the signed-attributes DER artifact, the Rekor entry and signature, and the certificate window, the approved result carries trusted_at equal to integratedTime in UTC..."}
  - {id: FR-041-AC-29, path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md, role: examined, excerpt: "Starting from the genuine live tag and entry used for AC-28, changing integratedTime, the SET, the inclusion proof, substituting a genuine entry from a different tag, or using a trusted root that does not verify the entry..."}
  - {id: src/approval_rekor.rs, path: src/approval_rekor.rs, role: examined, excerpt: "Extracts CMS signed attributes, signature, certificate and Rekor entry; constructs and verifies a Sigstore Bundle."}
  - {id: src/approval_verify_host.rs, path: src/approval_verify_host.rs, role: examined, excerpt: "trusted_time unit test is bound to AC-28 but checks only UTC formatting and signed-minus-integrated time arithmetic."}
  - {id: tests/approval_verify_gitsign.rs, path: tests/approval_verify_gitsign.rs, role: examined, excerpt: "Stand-in gitsign process tests; the ignored live test is for AC-26 and requires externally supplied live tag/service inputs."}
  - {id: tests/approval_verify_cli.rs, path: tests/approval_verify_cli.rs, role: examined, excerpt: "Contains new trust and approval CLI trace annotations, including unresolvable TC ids."}
  - {id: tests/approval_trail_cli.rs, path: tests/approval_trail_cli.rs, role: examined, excerpt: "New trail test trace annotation includes an unresolvable TC id."}
  - {id: tests/release_verify_cli.rs, path: tests/release_verify_cli.rs, role: examined, excerpt: "New release trust test trace annotation includes an unresolvable TC id."}
  - {id: tests/signature_verify_cli.rs, path: tests/signature_verify_cli.rs, role: examined, excerpt: "New signature tests include direct FR-AC links and TC ids with no declared artifacts."}
  - {id: FR-040-CON-2, path: spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md, role: context_only, excerpt: "The artifact supplied to that verifier SHALL be the CMS signed attributes re-encoded as DER with the explicit universal SET OF tag..."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-041-AC-28
    related_ids: [FR-041-AC-29]
    path: src/approval_verify_host.rs
    lines: "319-345"
    excerpt: "Against a genuine gitsign-signed annotated tag carrying an embedded Rekor entry... when sigstore-verify verifies the signed-attributes DER artifact, the Rekor entry and signature, and the certificate window, the approved result carries trusted_at equal to integratedTime in UTC..."
    finding: "The only binder for AC-28 is a pure trusted_time test that takes an integer and checks formatting/skew; it never parses a CMS tag, constructs a Rekor Bundle, or calls Verifier::verify. AC-29 has no binder and no changed test exercises tampering, entry substitution, inclusion failure, or wrong-root rejection. Thus the core acceptance and fail-closed behavior can regress without a focused test detecting it."
  - fnd: FND-002
    severity: low
    confidence: high
    method: gap-analysis
    check: other
    artifact_id: tests/approval_verify_cli.rs
    related_ids: []
    path: tests/approval_verify_cli.rs
    lines: "1135-1364"
    excerpt: "#[trace(\"TC-1049\", ..., \"FR-041-AC-34\")]"
    finding: "TC-1049 through TC-1060 appear in new trace annotations but are not defined in spec artifacts; quoin reports them as unbacked matrix rows. Direct FR-AC tags in the same annotations do bind, but the extra TC ids are dangling references."
bindings:
  - {test_id: trusted_time_is_utc_and_time_skew_is_signed_minus_integrated_time, ac_id: FR-041-AC-28, trace: correct}
  - {test_id: tests/approval_verify_gitsign.rs::tc_1017_real_gitsign_verifies_and_binds_a_public_approval_tag, ac_id: FR-041-AC-26, trace: correct}
```

## Dispositions

- **FND-001 — fixed for offline extraction and verifier coverage.** The new
  locally generated fixtures exercise the production CMS extraction and Bundle
  construction, then `Verifier::verify` with a test-only trusted root. The
  accepted case verifies the signature, Rekor SET, inclusion proof, checkpoint,
  certificate chain and integrated time. Five fail-closed cases cover changed
  integrated time, changed SET, changed inclusion proof, substitution of an
  entry from a second valid tag, and a trusted root with an unrelated Rekor
  key. Evidence: the focused extraction test passed 1/1 and the filtered
  rejection tests passed 5/5 through `locked-build.sh`.

  The fixtures use fictional locally generated CMS/Rekor material and are not
  gitsign-produced. They do not establish production-root trust or replace the
  ignored live AC-26 test; that live test remains unavailable/not executed.
  The synthetic verifier policy skips SCT validation because the fixture has no
  CT proof, while retaining certificate-chain, Rekor, artifact and signature
  verification.
