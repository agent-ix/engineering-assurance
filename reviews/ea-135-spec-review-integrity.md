---
id: SR-20261115040624
title: "spec-review/integrity review of EA-135 gitsign identity registry"
type: SpecReview
analysis: integrity
scope: "agent-ix/engineering-assurance; spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md; spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md; spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md; spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md; spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md; spec/functional/FR-047-release-record-and-signed-release-tag.md; spec/functional/FR-080-signed-proof-integrity-statement-verification.md"
review_set: subset
---

## Summary

Ticket: EA-135. Consistency and exit/error taxonomy across FR-036/037/040/041/042/043/047/080
after the registry change. Signer model, `source` value and FR-041 `untrusted_signer` rule are
consistent; the error taxonomy is not.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Operational git failures while reading the registry are classed as an invalid registry. FR-037 makes "a git failure" (not started, timeout, output bound) and a non-repository path "unreadable", which FR-040 maps to `*_identity_registry_invalid` exit 2. The same failures on the target are `unverifiable` with `git_not_started`/`timeout`/`repository_unavailable` at exit 0 (FR-036), and FR-043/FR-047 have their own `approval_trail_repository_unavailable` / `release_verify_git_failed` codes. Scenario: `approval-trail` with `gitsign` on a path that is not a git repository; FR-043-AC-10 requires `approval_trail_repository_unavailable`, the FR-040 ordering (registry read first) yields `approval_trail_identity_registry_invalid`. Fix: separate "registry content invalid / absent at revision" from "git could not answer", and state per command which code wins. | spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:89-93; spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md:144-147,216-219; spec/functional/FR-043-approval-trail-of-a-change-assurance-record.md:184; spec/functional/FR-047-release-record-and-signed-release-tag.md:104-108 |
| FND-002 | low | FR-041-AC-9 (Property) still enumerates "verdict status, binding member and meaning presence"; the decision now also depends on signer mapping (`signed_valid` + `unmapped` is `untrusted_signer`), so the property can pass while that branch returns `approved`. Add mapping as a dimension. | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:118-120,248 |

## Verdict

FAIL on FND-001 (medium); see the base review for the code-list and FR-080 gaps. FR-037-CON-1, FR-040-CON-3 and FR-041-CON-1 agree that `mapped` requires a registry entry and that no principal comes from a request.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-037 now has three reading outcomes (read, `invalid`, `git_failed`). `git_failed` takes each capability's existing git-failure outcome: `unverifiable` at exit 0 for signature-verify and approval-verify (FR-040-AC-30), `approval_trail_repository_unavailable` (FR-043-AC-10, never the registry code), `release_verify_git_failed` (FR-047-AC-14), and `unverifiable` for FR-080. |
| FND-002 | fixed | FR-041-AC-9 now covers signer mapping as a dimension of the property. |
