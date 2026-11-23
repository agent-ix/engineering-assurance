---
id: FR-080
title: "Verify a signed proof-integrity statement against one exact tag object"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-210"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-036"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-037"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-040"
    type: "references"
---

# FR-080: Verify a signed proof-integrity statement against one exact tag object

## Description

Engineering Assurance SHALL provide a `proof-checkpoint-verify` command that
verifies a signed annotated tag against the caller's expected exact tag object
id and the identity digest and scope of one `proof-integrity-statement/v1`
defined by Quoin FR-210. The signed checkpoint
states the purpose `proof-integrity-statement/v1`, the statement's SHA-256
identity digest, and its record id/revision, candidate revision, proof id, and
attestation id. The result SHALL identify the exact immutable tag object whose
checkpoint fields were parsed and whose signature was verified, and SHALL
return the statement digest and scope, signer, trust inputs, and any
unavailable/refusal cause.

This checkpoint establishes only that the configured verifier accepted a
signature over these checkpoint fields. It SHALL NOT establish human approval,
signer authorization for a role, proof correctness, or proof qualification.
Those decisions remain separate. In particular, this capability SHALL NOT
reuse or change the human-approval meaning of [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md).

Quoin owns creating and validating the exact statement and computing its
identity digest. This command accepts the expected digest and scope; it does
not read or retain statement bytes. Quoin FR-210 owns handling an absent
statement or a digest it cannot compute. Altering or substituting statement
content changes its identity digest, which SHALL fail to match a tag signed for
the original digest. Quoin also owns the selected retention service and proof
qualification. This command establishes neither retention nor proof validity.

Out of scope: creating or pushing a checkpoint tag, retaining statement or
proof objects, selecting a retention service, deciding whether a signer is
authorized to approve or qualify evidence, and deciding whether a proof is
valid.

## Inputs

- One closed request document `engineering-assurance.proof-checkpoint-verify-request/v1` with `protocol`, `repository` (absolute path), `tag` (a bare tag name or `refs/tags/<name>`), `expected` (the exact tag object id, statement digest and scope), `allowed_signers` (the FR-036 SSH trust file), and optional `gitsign` (the FR-040 identity, issuer, and principal). Unknown members are rejected.
- `expected` contains `purpose` (exactly `proof-integrity-statement/v1`), `statement_digest` (`sha256:` followed by 64 lowercase hexadecimal digits), `record_id`, `record_revision`, `candidate_revision`, `proof_id`, `attestation_id` and `tag_object_id` (the full lowercase hexadecimal id of the one annotated tag object the caller expects, 40 or 64 digits). The statement digest is the identity digest specified by Quoin FR-210. Identifiers are non-empty strings without control characters; `record_revision` is a positive integer.
- The signed tag message carries exactly one each of `Proof-Checkpoint-Purpose`, `Proof-Statement-Digest`, `Proof-Record-Id`, `Proof-Record-Revision`, `Proof-Candidate-Revision`, `Proof-Id`, and `Proof-Attestation-Id` trailers. Trailer keys are matched ASCII case-insensitively; values are compared exactly without normalization.
- The request SHALL include a well-formed expected tag object id, a well-formed statement digest and every scope field. A missing or malformed field is a request error and SHALL NOT start git or a signature verifier. The caller supplies the digest computed from the exact Quoin statement; this command does not accept or recompute statement bytes.

## Outputs

- One line of JSON `engineering-assurance.proof-checkpoint-verify-result/v1` at exit 0 for every verdict. The closed result includes `status`, `tag`, `tag_object_id`, `checkpoint`, `signer`, `trust`, and `signature`.
- `status` is `verified`, `wrong_checkpoint`, `untrusted_signer`, `signed_bad`, `unsigned`, or `unverifiable`. `checkpoint` returns the observed purpose, statement digest, and scope values, with each field classified as `match`, `mismatch`, `missing`, or `repeated`. `checkpoint.tag_object_id` is `match` or `mismatch` against `expected.tag_object_id`, and is `missing` when the top-level `tag_object_id` is `null` because the request did not resolve to an annotated tag object.
- `tag_object_id` is the canonical full object id of the annotated tag object whose message was parsed and whose signature was verified. It is `null` only when the request did not resolve to an annotated tag object.
- `signer` and `signature` preserve the corresponding FR-036, FR-037, and FR-040 verification results, except for an expected-object mismatch, where `signature` is `{"status":"unverifiable","reason":"expected_tag_object_id_mismatch","delegated":false}` and `signer` is `null`; `delegated` `false` marks that value as this command's own refusal and not a delegated signature verdict, and a delegated verdict carries no `delegated` member. `trust` echoes the allowed-signers path and, when supplied, the configured gitsign identity, OIDC issuer, and principal. A non-verified result never reports an accepted signer.
- A request that cannot yield a verdict writes the `engineering-assurance.error/v1` machine error for `proof-checkpoint-verify` and exits 2. An `unverifiable` result carries the refusal cause from the unavailable tag, object binding, verifier, or trust configuration.

## Behavior

- The command SHALL resolve the requested tag exactly once to a canonical annotated tag object id, then read the message and signature from that object. It SHALL bind signature verification to the same object id, using an isolated ref pinned to that id when a delegated verifier accepts only a ref. If that binding cannot be guaranteed, the result SHALL be `unverifiable`.
- The command SHALL obtain signature verdicts only through FR-036, FR-037, and FR-040. It SHALL NOT implement cryptography, alter their signature classification, or infer signer identity or trust beyond their returned data.
- The command SHALL decide `status` in this order, the first rule that applies winning: (1) a request error writes the machine error before any git or verifier process starts; (2) a request that resolves to no annotated tag object has `tag_object_id` `null` and `checkpoint.tag_object_id` `missing`, and its status is the delegated FR-036 and FR-040 verdict for it (`unsigned` with `lightweight_tag` for a lightweight tag, otherwise `unverifiable`); (3) an annotated tag object whose id differs from `expected.tag_object_id` is `wrong_checkpoint`; (4) otherwise the delegated signature verdict decides `unsigned`, `signed_bad`, `unverifiable` and `untrusted_signer` with their exact refusal states, and `signed_valid` with a mapped signer is `wrong_checkpoint` unless every field is `match`, in which case it is `verified`.
- The command SHALL compare the resolved annotated tag object id to `expected.tag_object_id` before it starts any delegated signature verifier.
- The command SHALL NOT start a delegated signature verifier for a resolved object whose id differs from `expected.tag_object_id`.
- When the resolved object id differs from `expected.tag_object_id`, the command SHALL report `wrong_checkpoint`, `checkpoint.tag_object_id` `mismatch`, the resolved id as `tag_object_id`, no accepted signer, and the `signature` refusal value defined in Outputs, however the object is signed or whether a verifier is available.
- When the resolved object id differs from `expected.tag_object_id`, the command SHALL still classify the other `checkpoint` fields from the resolved object's message, which needs no verifier.
- The command SHALL report `verified` only when the delegated signature verdict is `signed_valid` with a mapped signer, the resolved `tag_object_id` equals `expected.tag_object_id`, and the purpose, statement digest, and every scope field occur exactly once and match `expected`.
- A signed tag with a different purpose, statement digest, record id or revision, candidate revision, proof id, attestation id, missing field, or repeated field SHALL yield `wrong_checkpoint`.
- A good signature from an untrusted identity SHALL yield `untrusted_signer`. A signature classified bad by the delegated verifier SHALL yield `signed_bad`. An unsigned annotated tag SHALL yield `unsigned`.
- An unavailable repository, tag object, verifier, or required trust configuration SHALL yield `unverifiable`. The command SHALL NOT convert any of these cases into a positive result.
- The command SHALL NOT emit `approved`, an approval meaning, a role authorization, or a proof qualification result. A consumer SHALL treat `verified` only as authentication of the checkpoint fields under the returned trust configuration.
- A request that does not resolve to an annotated tag SHALL NOT verify a commit, lightweight tag, note, or mutable ref value as a checkpoint.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-080-CON-1 | A `verified` result SHALL bind checkpoint parsing and delegated signature verification to the same immutable annotated tag object identified by `tag_object_id`. | Safety | Test |
| FR-080-CON-2 | A `verified` result SHALL NOT imply human approval, role authorization, proof validity, or qualification. | Safety | Test |
| FR-080-CON-3 | A `verified` result SHALL NOT be reported unless the verified tag object id equals the caller's expected tag object id. | Safety | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-080-AC-1 | An annotated tag signed by an SSH key accepted by the supplied allowed-signers file, whose purpose, statement digest, and complete scope exactly match the request, yields `verified` with the exact tag object id, mapped signer, and echoed trust input. | Test |
| FR-080-AC-2 | A gitsign-signed tag accepted for the requested certificate identity and OIDC issuer, with matching purpose, statement digest, and scope, yields `verified` and reports the accepted identity, issuer, and configured principal. | Test |
| FR-080-AC-3 | A missing, malformed, or unknown expected statement digest never yields `verified`; a tag with a missing digest or a digest different from the exact statement's identity digest yields `wrong_checkpoint`. A changed or substituted statement produces a different expected digest and does not verify against a tag signed for the original digest. | Test |
| FR-080-AC-4 | A tag with an absent, altered, repeated, or substituted purpose or scope field yields `wrong_checkpoint`; changing record id/revision, candidate revision, proof id, or attestation id cannot yield `verified`. | Test |
| FR-080-AC-5 | A signature from a signer outside the configured trust, a wrong gitsign identity or issuer, a bad signature, an unsigned annotated tag, and an unavailable required verifier yield respectively `untrusted_signer`, `untrusted_signer`, `signed_bad`, `unsigned`, and `unverifiable`, with no accepted signer on each refusal. | Test |
| FR-080-AC-6 | A lightweight tag, commit, note, unresolved ref, malformed request, missing statement digest, or unsupported/missing trust configuration never yields `verified`; malformed requests produce the declared machine error before invoking git. | Test |
| FR-080-AC-7 | Replacing the tag ref after resolution cannot combine checkpoint fields from one tag object with a successful signature verdict for another; the verifier observes the resolved object id or returns `unverifiable`, and a deliberate removal of this binding makes the negative-control test fail. | Test |
| FR-080-AC-8 | A result with status `verified` contains no `approved`, role-authorization, or proof-qualification claim, and a consumer can distinguish statement integrity from FR-041 approval. | Test |
| FR-080-AC-9 | A request pins tag name `T` to object A. (a) When `T` has since been retargeted to a different annotated tag object B, validly signed by an accepted key and carrying identical purpose, statement digest and scope fields, the result is `wrong_checkpoint` with `checkpoint.tag_object_id` `mismatch`, `tag_object_id` equal to B, no accepted signer and `signature` `unverifiable` with reason `expected_tag_object_id_mismatch` and `delegated` `false`, and no verifier process is started. (b) The same request against two differently named tags is `verified` for A and `wrong_checkpoint` for B. (c) A mismatched object that is also unsigned, signed by an untrusted key, badly signed, or checked while the verifier is unavailable yields the same `wrong_checkpoint` result, with no verifier started. (d) A matching object that is unsigned, untrusted, badly signed or unverifiable keeps `unsigned`, `untrusted_signer`, `signed_bad` or `unverifiable`. (e) A request that resolves to a lightweight tag or no annotated tag reports `tag_object_id` `null` and `checkpoint.tag_object_id` `missing`. | Test |
| FR-080-AC-10 | A missing or malformed `expected.tag_object_id` (wrong length, uppercase or non-hexadecimal) is a request error that writes the machine error and starts neither git nor a verifier. | Test |

## Dependencies

- **Upstream**: [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md),
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md), and
  [FR-040](./FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md)
  define signature verification, signer mapping, and gitsign trust; Quoin
  [FR-210](ix://agent-ix/quoin/FR-210) defines the exact
  `proof-integrity-statement/v1`, its SHA-256 identity digest and scope.
- **Downstream**: Quoin FR-210 supplies the statement digest and scope and
  consumes this result as data. Quoin owns statement existence, canonical
  content, digest recomputation, and independent retention. It SHALL NOT treat
  this result as human approval, signer authorization, or proof qualification.
