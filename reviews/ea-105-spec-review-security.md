---
id: SR-20261113163615
title: "Security review of the fail-closed leak check, approval statements and workflow trust (engineering-assurance PR 231)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-054..FR-065, spec/non-functional/NFR-008..NFR-010, spec/spec.md (PR #231 diff, head dd5a124)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-056
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-057
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-059
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-061
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-062
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-064
    type: references
  - target: ix://agent-ix/engineering-assurance/NFR-010
    type: references
---

# SR-20261113163615: Security review of the fail-closed leak check, approval statements and workflow trust (engineering-assurance PR 231)

## Summary

Ticket: EA-105 and PLAT-1685. Examined the trust and disclosure surface: the leak check (FR-057, NFR-009), the record split (FR-056), sink defaults (FR-055), statement signing and verification (FR-058, FR-064, NFR-010), the meaning line (FR-059), the role and agent rules (FR-061, FR-062) and workflow roles (FR-065). The spec-security-analysis skill emits a spec.md section, not a SpecReview, and the SpecReview analysis enum has no security value, so this artifact carries `analysis: base`. Sound as specified: unknown visibility is private, `public_linear_teams` defaults to empty, the check runs on every publish before sending, a refusal sends nothing and reroutes nothing, the private form never enters the public signed bytes, verification trusts no identity named inside the statement, AI agents are excluded by account type plus the policy list, and the spec states that an agent driving a human account is not detectable (see FND-007 on its wording). Applicable practice: SLSA/in-toto attestation verification (signer identity bound to the trusted builder) and Sigstore keyless signing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Statements are signed with the CI workflow identity through Sigstore (FR-064 trusts certificate identity patterns and an OIDC issuer, i.e. keyless signing). Keyless signing against the public-good instance writes an entry with the signing certificate to the public Rekor log, and the certificate names the repository and workflow. For a private repository every approval statement therefore publishes the private repository's identity to a public log regardless of the sink selection and without the leak check. Fix: state that signing for a repository whose resolved sink is not `public_log` uses no public transparency log (private instance, or no log upload with the bundle stored privately), and add an AC. | spec/non-functional/NFR-010-no-cryptography-of-our-own.md:20 |
| FND-002 | high | The leak check walks only the record's authored `references`. Nothing requires the publishing repository itself, or any repository the record is about, to be in that list, and FR-055-AC-2 explicitly lets a private repository configure `public_log`. A private repository's record with an empty or partial `references` list passes the check and is published. Fix: the check also resolves the publishing repository and the record's subject repository, whatever `references` holds, and an AC covers a private repository configured `public_log` with no references. | spec/functional/FR-057-fail-closed-pre-publish-leak-check.md:18-23 |
| FND-003 | high | The closed schema closes member names only: `record_id`, `kind`, the names in `hashes` and `verdicts`, the verdict values and `timestamps` are free strings, so ticket text, a model name, or a private repository or Linear id can sit in a hash name, verdict value or `record_id` where the leak check (which reads `references` only) never looks. "Reject a record whose public members hold a model name, ticket text or criteria text" gives no detection rule, so FR-056-AC-4 cannot be implemented consistently. Fix: give every public member a closed value grammar (record_id format, hash algorithm enum and hex value, verdict name and value enums, RFC 3339 timestamps, `kind` enum), so the trimmed record can hold only IDs, hashes, verdicts, timestamps and the signature by construction. | spec/functional/FR-056-public-and-private-record-split.md:22-25, 38 |
| FND-004 | medium | The meaning line rules leave open: several `Signed-off-meaning:` lines (FR-041 keeps all values, here there is one `meaning`), leading whitespace and CR line endings, length and control-character bounds (FR-041 bounds its trailers at 256 bytes with no control character), and review bodies edited after submission. On GitHub a collaborator with write access can edit another person's review body, so a meaning line can be added to someone else's plain approval. Fix: define the line grammar and bounds, take the body as submitted (or refuse an approval whose body was edited by anyone but its author), and add ACs. | spec/functional/FR-059-source-adapter-interface-and-github-review-adapter.md:44-46 |
| FND-005 | medium | The workflow-identity policy lists "accepted certificate identity patterns" with no constraint on the workflow ref or trigger. A pattern that admits any ref also admits a workflow edited on a pull request branch and run on `refs/pull/N/merge`, which could sign arbitrary approval statements. Fix: require each trusted identity to pin the workflow path at the protected base branch ref, and add an AC where the same workflow at a pull request ref yields `untrusted_signer`. | spec/functional/FR-064-verify-approval-statement-with-workflow-identity-policy.md:22-26 |
| FND-006 | medium | Distinctness defaults to "different identity, different individual", so one person holding two accounts (two GitHub accounts, or a GitHub and later a Jira account) fills two roles unless the policy author equates them. That follows the ruling (judged on the identity the source reports), but unlike FR-062 the spec does not name the residual. Fix: state that distinctness rests on the base-branch policy's account lists, and prefer a shape that fails closed: the policy declares individuals, each with its accounts, roles list individuals, and an account under two individuals is `approval_policy_invalid`. | spec/functional/FR-061-role-policy-and-distinct-individuals.md:32-35 |
| FND-007 | low | The stated limitation is reversed: "cannot detect a person operating an agent's credentials". The undetectable case that matters is an AI agent operating a human-owned account, which the classifier counts as human. Fix: "cannot detect an agent operating a human account's credentials". | spec/functional/FR-062-ai-agents-are-never-approvers.md:21-22 |

## Verdict

Not mergeable: FND-001 to FND-003 are paths by which private content reaches a public log despite the leak check, which is the ticket's central acceptance criterion. FND-004 to FND-007 are for the same round.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | `signature` is up to 64 KiB of base64, which encodes any bytes, so "no member's grammar admits free text" is not true of it. The spec does not say what the bundle signs. If it is the approval statement's DSSE bundle, the bundle carries the statement payload (meaning, approver, pull request) inside the public form, and FR-058-AC-8's byte search passes vacuously because the payload is base64. Fix: state that the public `signature` is a bundle over the public form's own canonical bytes, that a bundle whose signed payload is not exactly the public form is rejected, and that a public-log entry for a statement signature carries hashes only, never the statement payload. | spec/functional/FR-056-public-and-private-record-split.md:40-41 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | The canonical bytes leave open whether member names are sorted in nested objects (`references`, `subject`, the maps), by which ordering, and how strings are escaped, so two implementations can produce different signed payloads and reject each other's bundles. FR-056-AC-8 is otherwise non-vacuous: it decodes the bundle and compares the payload. Fix: name RFC 8785 (JSON Canonicalization Scheme). | spec/functional/FR-056-public-and-private-record-split.md:44-45 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
| FND-008 | fixed | - |
