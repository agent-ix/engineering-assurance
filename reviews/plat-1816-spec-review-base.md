---
id: SR-2026112016223475846355055
title: "Base spec review — Engineering Assurance producer identity"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance; FR-036, FR-041, FR-043, FR-047"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/FR-036", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-041", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-043", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-047", type: references }
---

## Summary

Reviewed the changed requirement clauses and acceptance criteria for producer version, source revision and generation-time identity across the four verifier commands. The criteria are observable, map to tests, and remain consistent with the shared FR-036 field contract.

## Verdict

**PASS** — no specification defects were found in the changed requirement text or criteria.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review Scope

- {id: FR-036 producer identity definition, path: "spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md", role: examined, excerpt: "Every verdict also carries `ea_version` (the running executable's Cargo package version), `ea_source_revision` (the full commit id of the Engineering Assurance source used to build the executable, or JSON `null` when the build did not embed a source commit; a non-null value is 40 or 64 lowercase ASCII hexadecimal digits), `generated_at` (a UTC instant in the form `YYYY-MM-DDTHH:MM:SSZ`), and `generated_at_source` with the value `host_clock`."}
- {id: FR-036-AC-16, path: "spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md", role: examined, excerpt: "Every verdict for both commit and tag requests, including `unsigned` and `unverifiable`, carries the running package version, the build-embedded Engineering Assurance source commit or `null` when unavailable, and one UTC `generated_at` value with source `host_clock`; all verdicts produced by one invocation share that value, which is not used as signing or trusted time."}
- {id: FR-041 producer identity result contract, path: "spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md", role: examined, excerpt: "These producer fields are present on every result with a verdict, including results without `trust`; the host-clock value is a claim and is not a signing or trusted time. They are additive and do not change the `engineering-assurance.approval-verify-result/v1` protocol value."}
- {id: FR-041-AC-37, path: "spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md", role: examined, excerpt: "Every `approval-verify` verdict, including one whose trust inputs cannot be read and one whose status is not `approved`, carries the running `ea_version`, the build's `ea_source_revision` (or `null` when absent), and a UTC host-clock `generated_at` sampled once for the invocation with source `host_clock`; when `trust` is present, top-level `ea_version` equals `trust.ea_version`; the nested `signature` omits duplicate producer fields, and no producer field is presented as signing or trusted time."}
- {id: FR-043 producer identity result contract, path: "spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md", role: examined, excerpt: "These producer fields appear on every trail result, including an empty or incomplete trail; `generated_at` is one host-clock claim sampled for the trail invocation, not a signing or trusted time. These fields are additive and do not change the `engineering-assurance.approval-trail-result/v1` protocol value. Every entry also carries these fields with the same values as the enclosing trail result."}
- {id: FR-043-AC-4, path: "spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md", role: examined, excerpt: "Each entry equals the `approval-verify` result for the same tag, request and trust input, less `protocol`, `record_id`, `record_digest`, `trust` and `generated_at`, and plus `claimed_at` and `object`; its `ea_version`, `ea_source_revision` and `generated_at_source` match the `approval-verify` result, while `generated_at` matches the enclosing trail result."}
- {id: FR-043-AC-26, path: "spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md", role: examined, excerpt: "The trail result and every entry carry `ea_version`, `ea_source_revision`, `generated_at` and `generated_at_source`; all entries share the trail's `generated_at`, including an empty or incomplete trail. When result `trust` is present, top-level `ea_version` equals `trust.ea_version`; `trust` carries `allowed_signers_sha256`, `ea_version` and `git_version`, and `allowed_signers_commit` when the request's `allowed_signers` is a `<rev>:<path>` that resolved."}
- {id: FR-047 producer identity result contract, path: "spec/functional/FR-047-release-record-and-signed-release-tag.md", role: examined, excerpt: "Every result with a verdict carries `ea_version`, `ea_source_revision`, `generated_at` and `generated_at_source`, with the meanings and formats defined by [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md). `generated_at` is sampled once for the release-verify invocation from the host clock, including when `approval` is absent; it is not a signing or trusted time."}
- {id: FR-047-AC-17, path: "spec/functional/FR-047-release-record-and-signed-release-tag.md", role: examined, excerpt: "Every `released` or `not_released` verdict, including a verdict without `approval`, carries the executable's `ea_version`, build-embedded `ea_source_revision` (or `null` when unavailable), and one UTC `generated_at` value with source `host_clock`; when `trust` is present top-level `ea_version` equals `trust.ea_version`, and when `approval` is present its producer fields are identical; none of these fields is treated as signing or trusted time."}
- {id: SECURITY.md, path: "SECURITY.md", role: context_only, excerpt: "If you believe you have found a security vulnerability in Engineering Assurance, use the repository Security tab and choose Report a vulnerability."}

## Coverage

- The four new acceptance criteria are all trace-bound to CLI tests. The updated FR-043 criterion also remains bound to the existing trail equivalence test.
- The command-specific meanings refer back to FR-036 rather than duplicating its field definitions. The host-clock observation is distinguished from signing and transparency-log trusted times.
- `quire validate --scope . 'spec/**/*.md' --summary` completed with existing grammar advisories elsewhere in the specification; none of the changed FR statements received an EARS diagnostic.
