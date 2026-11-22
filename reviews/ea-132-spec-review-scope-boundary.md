---
id: SR-20261115040613
title: "spec-review/scope-boundary review of EA-132 signed proof checkpoint"
type: SpecReview
analysis: scope-boundary
scope: agent-ix/engineering-assurance; spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md; spec/spec.md
review_set: subset
---

## Summary

FAIL. Reviewed engineering-assurance PR #294 at 75729468a6f5653022afa109278bae7b9aae66e1 against EA-132. The FR uses a manifest-head/population ledger instead of the required signed proof-integrity statement identity digest.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-080 assigns a manifest-head/population ledger checkpoint to Engineering Assurance, while EA-132 explicitly excludes that ledger and assigns the signed statement plus retained proof objects to Quoin. The boundary is reversed. Keep EA verification limited to authenticating the exact tag-bound signed-statement digest. | spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md:33-36 |

## Verdict

Not mergeable until the FR binds the exact tag object to the SHA-256 identity digest of one signed `proof-integrity-statement/v1`, and removes manifest-head/sequence/ledger semantics. Verification of this digest must remain distinct from FR-041 human approval and from identity authorization, proof qualification, and retention enforcement.

## Dispositions

| ID | Status | Round 1 fix evidence |
| --- | --- | --- |
| FND-001 | fixed | Round 1: FR-080 now verifies the `proof-integrity-statement/v1` purpose, the expected SHA-256 statement identity digest, and the exact record/revision, candidate, proof, and attestation scope against the exact signed tag object. Missing or malformed digest/scope is rejected before git; a changed or substituted statement yields a different digest and does not match the tag. Quoin FR-210 owns statement bytes, canonicalization/digest computation, absence handling, and retention; FR-080 accepts only digest and scope. The SSH/gitsign signature path and the separation from FR-041 approval, signer authorization, and proof qualification remain explicit. The revised responsibility split leaves statement construction and independent retention with Quoin and exact tag-object/signature verification with Engineering Assurance. |

## Examined scope

```yaml
scope:
- id: FR-080
  path: spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md
  role: context_only
  excerpt: Engineering Assurance SHALL provide a `proof-checkpoint-verify` command
    that verifies a signed annotated tag as an integrity checkpoint for one proof
    manifest head and its declared population.
- id: FR-080-AC-1
  path: spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md
  role: examined
  excerpt: An annotated tag signed by an SSH key accepted by the supplied allowed-signers
    file, whose checkpoint fields and complete proof population exactly match the
    request, yields `verified` with the exact tag object id, mapped signer, and echoed
    trust input.
- id: FR-080-AC-2
  path: spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md
  role: examined
  excerpt: A gitsign-signed tag accepted for the requested certificate identity and
    OIDC issuer, with matching checkpoint fields, yields `verified` and reports the
    accepted identity, issuer, and configured principal.
- id: FR-080-AC-3
  path: spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md
  role: examined
  excerpt: Altering any checkpoint field, omitting or repeating one, or adding, removing,
    or changing a proof/attestation pair yields `wrong_checkpoint`, never `verified`.
- id: FR-080-AC-4
  path: spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md
  role: examined
  excerpt: A signature from a signer outside the configured trust, a bad signature,
    an unsigned annotated tag, and an unavailable required verifier yield respectively
    `untrusted_signer`, `signed_bad`, `unsigned`, and `unverifiable`, with no accepted
    signer on each refusal.
- id: FR-080-AC-5
  path: spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md
  role: examined
  excerpt: Replacing the tag ref after resolution cannot combine fields from one tag
    object with a successful signature verdict for another; the verifier observes
    the resolved object id or returns `unverifiable`.
- id: FR-080-AC-6
  path: spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md
  role: examined
  excerpt: A result with status `verified` contains no `approved`, role-authorization,
    or proof-qualification claim, and a consumer can distinguish checkpoint integrity
    from FR-041 approval.
- id: FR-080-AC-7
  path: spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md
  role: examined
  excerpt: A lightweight tag, commit, note, unresolved ref, malformed request, duplicate
    proof pair, malformed proof trailer, or unsupported/missing trust configuration
    never yields `verified`; malformed requests produce the declared machine error
    before invoking git.
- id: FR-080-CON-1
  path: spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md
  role: examined
  excerpt: A `verified` result SHALL bind checkpoint parsing and delegated signature
    verification to the same immutable annotated tag object identified by `tag_object_id`.
- id: FR-080-CON-2
  path: spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md
  role: examined
  excerpt: A `verified` result SHALL NOT imply human approval, role authorization,
    proof validity, or qualification.
- id: FR-080
  path: spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md
  role: examined
  excerpt: The command SHALL resolve the requested tag exactly once to a canonical
    annotated tag object id, then read the message and signature from that object.
    It SHALL bind signature verification to the same object id, using an isolated
    ref pinned to that id when a delegated verifier accepts only a ref.
- id: FR-080
  path: spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md
  role: examined
  excerpt: The command SHALL report `verified` only when the delegated signature verdict
    is `signed_valid` with a mapped signer and every required checkpoint field and
    proof/attestation pair occurs exactly once and matches `expected`.
- id: FR-080
  path: spec/functional/FR-080-signed-proof-manifest-checkpoint-verification.md
  role: examined
  excerpt: 'Downstream: Quoin consumes this result as data for the independently retained
    proof-manifest checkpoint. It SHALL NOT treat this result as human approval or
    proof qualification.'
- id: FR-041
  path: spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md
  role: context_only
  excerpt: Verify a signed annotated tag as a named person's approval of an exact
    change-assurance record
- id: FR-036
  path: spec/functional/FR-036-ssh-signed-commit-and-tag-verification.md
  role: context_only
  excerpt: Verify SSH signatures on commits and annotated tags by delegating to git
- id: FR-037
  path: spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md
  role: context_only
  excerpt: Map verified signers to named people and expose the signed-off meaning
- id: FR-040
  path: spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md
  role: context_only
  excerpt: Verify gitsign (Sigstore keyless) signatures on commits and annotated tags
    by delegating to gitsign
- id: US-005
  path: spec/usecase/US-005-correlate-definition-result-evidence.md
  role: context_only
  excerpt: Correlate a definition, result, evidence, and decision
- id: StR-002
  path: spec/stakeholder/StR-002-review-verification-evidence-without-semantic-collapse.md
  role: context_only
  excerpt: Review verification evidence without semantic collapse
- id: spec/spec.md
  path: spec/spec.md
  role: examined
  excerpt: Index entry for FR-080.
```

### Independent reviewer disposition round 1

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Independent round 1: fixed. EA scope is limited to verifying a signature over statement digest/scope and returning the exact tag object identity; Quoin owns statement bytes, digest computation, and independent retention. |
