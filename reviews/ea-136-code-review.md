---
id: SR-902
title: "Code review — EA-136/138 and EA-145 implementation"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance; origin/main...daf8522eaea699d4ee2d0d6725d49c8f973bc0ba; EA-136/138 Rust implementation and EA-145 trusted-time implementation, tests, and FR-040/FR-041 requirements"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/FR-040", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-041", type: references }
---

## Summary

Reviewed the full 8-commit, 18-path diff from the existing `origin/main` ref (`a04da1e13a353f4e3e7f8d9684643c2e45ab83ca`) through local head `daf8522eaea699d4ee2d0d6725d49c8f973bc0ba`. This combined Rust code review covers EA-136/138 and EA-145. The CMS signature bytes are DER-wrapped before being placed in a Sigstore Bundle field that expects the raw octets, so valid Rekor entries cannot verify.

## Verdict

**FAIL** — the new trusted-time path supplies the wrong signature representation to the verifier and consequently cannot return a trusted transparency-log time for a valid embedded entry.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | `signer.signature.to_der()` places the DER tag/length wrapper in `messageSignature.signature`, but Sigstore compares that field with Rekor's raw signature octets; the mismatch makes every otherwise valid embedded entry fail verification and suppresses `trusted_at`. | src/approval_rekor.rs:105-108 | implementation-bug-despite-evidence |

## Review evidence

- Base ref: existing `origin/main` at `a04da1e13a353f4e3e7f8d9684643c2e45ab83ca`; local head: `daf8522eaea699d4ee2d0d6725d49c8f973bc0ba`; 8 commits, 18 changed paths. A fetch was attempted but could not update read-only linked-worktree git metadata, so the review uses the existing local `origin/main` ref.
- Scope includes Cargo manifests, FR-040/FR-041, all changed Rust production modules and tests; EA-138 content-rights code is included in this combined EA-136 review.
- Focused evidence: `git diff --check` passed; `cargo +1.98.1 fmt --check` passed through `locked-build.sh`; focused six integration targets passed through the wrapper; the `trusted_time_is_utc_and_time_skew_is_signed_minus_integrated_time` binary unit passed (1/1). Coder reports `cargo check --locked --features full` passed, 8 gitsign tests passed with one live test ignored, and `content-rights-tree` accepted 774.
- No full suite or all-targets gate was run. The live Sigstore test was not run.
- Evidence for FND-001: `cms::signed_data::SignerInfo.signature` is a DER `OctetString`; `.to_der()` serializes its ASN.1 wrapper. `sigstore_types::SignatureBytes` is compared byte-for-byte with the Rekor signature bytes by `sigstore-verify`'s hashedrekord verifier. The raw octets are the `OctetString` contents. The path at `src/approval_verify_host.rs:232-253` converts bundle verification failure into `trusted_at_missing: log_entry_unverified`.
- Repo instructions: `AGENTS.md` inspected; `CLAUDE.md` and `CONTRIBUTING.md` absent. No CI workflow changed. No applicable `AssuranceProfile` found.

## Findings data

```yaml
clean: false
scope:
  - {id: Cargo.toml, path: Cargo.toml, role: examined, excerpt: "Feature/dependency changes for the trusted-time path."}
  - {id: Cargo.lock, path: Cargo.lock, role: examined, excerpt: "Locked CMS, DER, and Sigstore support dependencies."}
  - {id: FR-040-CON-2, path: spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md, role: examined, excerpt: "The artifact supplied to that verifier SHALL be the CMS signed attributes re-encoded as DER with the explicit universal SET OF tag... A CMS/DER reader crate may be used only to extract that signed-attributes DER, the CMS signature bytes..."}
  - {id: FR-041-AC-28, path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md, role: examined, excerpt: "Against a genuine gitsign-signed annotated tag carrying an embedded Rekor entry... when sigstore-verify verifies the signed-attributes DER artifact, the Rekor entry and signature... the approved result carries trusted_at equal to integratedTime in UTC..."}
  - {id: FR-041-AC-29, path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md, role: examined, excerpt: "Starting from the genuine live tag and entry used for AC-28, changing integratedTime, the SET, the inclusion proof, substituting a genuine entry from a different tag, or using a trusted root that does not verify the entry..."}
  - {id: src/approval_rekor.rs, path: src/approval_rekor.rs, role: examined, excerpt: "Extracts signed attributes, CMS signature, signer certificate and embedded Rekor entry; constructs a Sigstore Bundle."}
  - {id: src/approval_verify_host.rs, path: src/approval_verify_host.rs, role: examined, excerpt: "Calls extraction and verification for an approved gitsign tag; failed verification becomes log_entry_unverified."}
  - {id: src/approval_verify.rs, path: src/approval_verify.rs, role: examined, excerpt: "Trusted-time state and serialization types."}
  - {id: src/approval_trail_host.rs, path: src/approval_trail_host.rs, role: examined, excerpt: "Approval-trail host uses approval results and carries trusted-time fields."}
  - {id: src/signature_verify_host.rs, path: src/signature_verify_host.rs, role: examined, excerpt: "gitsign request, executable loading, isolated tag verification and trust capture."}
  - {id: src/release_host.rs, path: src/release_host.rs, role: examined, excerpt: "Release output carries approval/trust result data."}
  - {id: src/main.rs, path: src/main.rs, role: examined, excerpt: "Registers the approval Rekor module."}
  - {id: tests/approval_verify.rs, path: tests/approval_verify.rs, role: examined, excerpt: "Pure approval decision and signing-time tests."}
  - {id: tests/approval_verify_cli.rs, path: tests/approval_verify_cli.rs, role: examined, excerpt: "Approval CLI cases for request, trust, and output behavior."}
  - {id: tests/approval_verify_gitsign.rs, path: tests/approval_verify_gitsign.rs, role: examined, excerpt: "Stand-in gitsign process-boundary tests; live test is ignored by default."}
  - {id: tests/approval_trail_cli.rs, path: tests/approval_trail_cli.rs, role: examined, excerpt: "Approval trail CLI tests."}
  - {id: tests/release_verify_cli.rs, path: tests/release_verify_cli.rs, role: examined, excerpt: "Release approval and trust-output tests."}
  - {id: tests/signature_verify_cli.rs, path: tests/signature_verify_cli.rs, role: examined, excerpt: "Signature CLI regression tests, including committed trust-source behavior."}
  - {id: tests/source_audit.rs, path: tests/source_audit.rs, role: examined, excerpt: "Source-level audit constraints."}
findings:
  - fnd: FND-001
    severity: high
    confidence: high
    method: code-review
    check: code-bug
    artifact_id: FR-041-AC-28
    related_ids: [FR-040-CON-2, FR-041-AC-29]
    path: src/approval_rekor.rs
    lines: "105-108"
    excerpt: "Against a genuine gitsign-signed annotated tag carrying an embedded Rekor entry, with real gitsign v0.18.0 and available Sigstore trust services, when sigstore-verify verifies the signed-attributes DER artifact, the Rekor entry and signature, and the certificate window, the approved result carries trusted_at equal to integratedTime in UTC, trusted_at_source transparency_log, signed_at with tagger_date, and time_skew equal to the difference in seconds when the two differ and no time_skew when they are equal."
    finding: "The CMS signature is encoded with OctetString::to_der() before it is put in messageSignature.signature. That bundle field contains raw signature octets; sigstore-verify compares it byte-for-byte with Rekor's raw signature. The DER wrapper therefore makes verification fail for valid entries, and the caller maps every such failure to trusted_at_missing=log_entry_unverified. Extract the raw OctetString contents for the bundle field."
bindings: []
```

## Deferred finding (disposition pass 4)

| ID | Severity | Summary | Refs | Disposition |
| --- | --- | --- | --- | --- |
| FND-002 | high | A policy-prefix tag that does not become an approval entry is skipped as `TagNoReceipt`; it is absent from the trail and does not make `complete` false, even when no receipt exists. | src/approval_trail_host.rs:589-595; FR-043-AC-28 | deferred to EA-146; unchanged from `origin/main`, outside EA-306 scope |

This is reproducible by a `tag_receipts` request for record R with an absent
notes ref and a current policy-prefix tag whose readable trailers do not name R.
`examine` returns `Other`, `compare_tags` emits `TagNoReceipt`, and the trail
loop drops that finding before updating `complete`. FR-043-AC-28 requires the
prefix tag to have `tag_receipt` `none` and the result to be incomplete.

This behavior is present in `origin/main` unchanged; EA-146 now tracks the missing-receipt regression acceptance. It is not a new EA-306 finding. Review of the seven named merge-resolution files found no separate defect introduced by the EA-306 changes.

```yaml
deferred_findings:
  - fnd: FND-002
    severity: high
    confidence: high
    method: code-review
    check: deferred
    artifact_id: FR-043-AC-28
    related_ids: []
    path: src/approval_trail_host.rs
    lines: "589-595"
    excerpt: "A current tag whose name starts with a policy prefix and has no verified receipt for its current object id carries `tag_receipt` `none` and counts against `complete` unless accepted."
    finding: "`examine` drops tags whose readable trailers do not name the requested record. `compare_tags` then emits `TagNoReceipt` for a current policy-prefix tag with no note, but `tag_section` skips every such finding on the assumption that it is represented by an approval entry. In this case no entry exists, so the tag is omitted and `complete` can remain true. This violates FR-043-AC-28 and the complete-result rule. The behavior is unchanged from origin/main and is tracked by EA-146, so it is deferred and out of EA-306 scope."
    deferred_to: EA-146
    scope: unchanged_from_origin_main
```

## Dispositions

### Round 2 — HEAD 9fa56daa0196f4c62bbd8788df5279726b81b98d

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The production bundle now base64-encodes `OctetString::as_bytes()` directly. The generated CMS/Rekor fixture exercises extraction, bundle construction and successful verifier acceptance; the focused fixture filter passed 6/6, including tamper and substitution rejection cases. |

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      fn bundle_signature(signature: &OctetString) -> String {
          STANDARD.encode(signature.as_bytes())
      }
```

### Round 3 — HEAD cec9ee8626c9ce65696040d2e4172bf7f67b584a

The latest commit changes the trace attribute import to the form recognized by
Quoin. The focused offline fixture tests passed 6/6; no new code defect was
found.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Raw CMS signature octets still enter the bundle unchanged, and the positive fixture plus five rejection cases pass. |

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      fn bundle_signature(signature: &OctetString) -> String {
          STANDARD.encode(signature.as_bytes())
      }
```

### Round 4 — requested HEAD eb390a6839adccbdd857475011f9e6d7e94aa759 (checked-out HEAD f819ab8c1271d245beae1cfcc0ff728f1c550e59; identical tree)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The CMS/Rekor bundle signature continues to use raw signature octets; the offline fixture binders remain present in the computed matrix. |
| FND-002 | deferred to EA-146 | This behavior is unchanged from `origin/main`; the missing-receipt regression acceptance is tracked by EA-146 and is outside EA-306 scope. |

No separate EA-306 defect was found in the seven merge-resolution files.

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    reason: "The corrected raw-octet bundle mapping remains in place; Quoin matrix still binds the positive offline verifier fixture and five rejection cases."
  - fnd: FND-002
    outcome: deferred
    deferred_to: EA-146
    scope: unchanged_from_origin_main
    reason: "The TagNoReceipt skip is unchanged from origin/main; EA-146 now carries the missing-receipt regression acceptance, so this is outside EA-306 scope."
```
