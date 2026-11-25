---
id: SR-2026112016223475846355056
title: "EARS review — Engineering Assurance producer identity"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/engineering-assurance; FR-036, FR-041, FR-043, FR-047 behavior statements"
review_set: subset
relationships:
  - { target: "ix://agent-ix/engineering-assurance/FR-036", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-041", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-043", type: references }
  - { target: "ix://agent-ix/engineering-assurance/FR-047", type: references }
---

## Summary

Reviewed the newly added behavior statements in the four changed functional requirements with the EARS engine and a semantic pass. The new obligations name their command or executable, specify concrete outputs and sampling events, and distinguish the host-clock observation from trusted signing times.

## Verdict

**PASS** — no EARS grammar or intent-pattern defects were found in the changed requirement statements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review Scope

- {id: FR-036 producer identity Behavior, path: "spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md", role: examined, excerpt: "The command SHALL sample the UTC host clock once when the `signature-verify` invocation starts. The command SHALL set `generated_at` on every verdict to that one sampled value. The command SHALL NOT use `generated_at` as a signing or trusted time or claim that the value was independently verified."}
- {id: FR-036 build identity Behavior, path: "spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md", role: examined, excerpt: "The executable SHALL report its Cargo package version as `ea_version`. The executable SHALL report the full commit id embedded for the Engineering Assurance source used to build it as `ea_source_revision`, or JSON `null` when the build has no embedded source commit. The executable SHALL NOT infer `ea_source_revision` from the repository being verified, the executable path, or a version string."}
- {id: FR-041 producer identity Behavior, path: "spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md", role: examined, excerpt: "The command SHALL capture `generated_at` once when the `approval-verify` invocation starts. The command SHALL use the same `generated_at` and producer identity as the nested signature verification. The command SHALL NOT read the local clock for `signed_at` or `trusted_at`."}
- {id: FR-043 producer identity Behavior, path: "spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md", role: examined, excerpt: "The command SHALL capture `generated_at` once when the `approval-trail` invocation starts. The command SHALL use that value and producer identity for the trail result and every entry it evaluates. The command SHALL NOT use that value as `signed_at` or `trusted_at`."}
- {id: FR-047 producer identity Behavior, path: "spec/functional/FR-047-release-record-and-signed-release-tag.md", role: examined, excerpt: "The command SHALL capture `generated_at` once when the `release-verify` invocation starts. The command SHALL use that value and producer identity for the nested `approval` result when it runs FR-041. The command SHALL NOT use that value as `signed_at` or `trusted_at`."}
- {id: FR-036-AC-16, path: "spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md", role: context_only, excerpt: "Every verdict for both commit and tag requests, including `unsigned` and `unverifiable`, carries the running package version, the build-embedded Engineering Assurance source commit or `null` when unavailable, and one UTC `generated_at` value with source `host_clock`; all verdicts produced by one invocation share that value, which is not used as signing or trusted time."}
- {id: FR-041-AC-37, path: "spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md", role: context_only, excerpt: "Every `approval-verify` verdict, including one whose trust inputs cannot be read and one whose status is not `approved`, carries the running `ea_version`, the build's `ea_source_revision` (or `null` when absent), and a UTC host-clock `generated_at` sampled once for the invocation with source `host_clock`; when `trust` is present, top-level `ea_version` equals `trust.ea_version`; the nested `signature` omits duplicate producer fields, and no producer field is presented as signing or trusted time."}
- {id: FR-043-AC-26, path: "spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md", role: context_only, excerpt: "The trail result and every entry carry `ea_version`, `ea_source_revision`, `generated_at` and `generated_at_source`; all entries share the trail's `generated_at`, including an empty or incomplete trail. When result `trust` is present, top-level `ea_version` equals `trust.ea_version`; `trust` carries `allowed_signers_sha256`, `ea_version` and `git_version`, and `allowed_signers_commit` when the request's `allowed_signers` is a `<rev>:<path>` that resolved."}
- {id: FR-047-AC-17, path: "spec/functional/FR-047-release-record-and-signed-release-tag.md", role: context_only, excerpt: "Every `released` or `not_released` verdict, including a verdict without `approval`, carries the executable's `ea_version`, build-embedded `ea_source_revision` (or `null` when unavailable), and one UTC `generated_at` value with source `host_clock`; when `trust` is present top-level `ea_version` equals `trust.ea_version`, and when `approval` is present its producer fields are identical; none of these fields is treated as signing or trusted time."}

## Coverage

- Engine: `quire validate --scope . 'spec/**/*.md' --summary`; the 24 grammar diagnostics are in unchanged requirements elsewhere in the repository. No changed statement in FR-036, FR-041, FR-043 or FR-047 received an EARS diagnostic.
- Semantic checks: trigger-vs-state fit, concrete observable response, and correct event pattern reviewed for every new behavior statement. The clock sample is tied to command invocation start, and producer identity is carried through the same run.
