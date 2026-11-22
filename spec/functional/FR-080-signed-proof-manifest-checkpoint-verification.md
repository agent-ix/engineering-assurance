---
id: FR-080
title: "Verify a signed proof-manifest checkpoint against one exact tag object"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-036"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-037"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-040"
    type: "references"
---

# FR-080: Verify a signed proof-manifest checkpoint against one exact tag object

## Description

Engineering Assurance SHALL provide a `proof-checkpoint-verify` command that
verifies a signed annotated tag as an integrity checkpoint for one proof
manifest head and its declared population. The checkpoint SHALL state its
versioned purpose, manifest-head digest, record id and revision, candidate
revision, and the proof and attestation identifiers it covers. The result SHALL
identify the exact immutable tag object whose checkpoint fields were parsed and
whose signature was verified, and SHALL return the signer and trust inputs used
for that verification.

This checkpoint establishes only that the configured verifier accepted a
signature over these checkpoint fields. It SHALL NOT establish human approval,
signer authorization for a role, proof correctness, or proof qualification.
Those decisions remain separate. In particular, this capability SHALL NOT
reuse or change the human-approval meaning of [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md).

Out of scope: creating or pushing a checkpoint tag, retaining manifest objects,
selecting a retention service, deciding whether a signer is authorized to
approve or qualify evidence, and deciding whether the referenced proofs are
valid.

## Inputs

- One closed request document `engineering-assurance.proof-checkpoint-verify-request/v1` with `protocol`, `repository` (absolute path), `tag` (a bare tag name or `refs/tags/<name>`), `expected` (the exact checkpoint fields the caller requires), `allowed_signers` (the FR-036 SSH trust file), and optional `gitsign` (the FR-040 identity, issuer, and principal). Unknown members are rejected.
- `expected` contains `purpose` (exactly `proof-manifest-integrity/v1`), `manifest_head` (`sha256:` followed by 64 lowercase hexadecimal digits), `record_id`, `record_revision`, `candidate_revision`, and a non-empty set of `proofs`; each proof entry contains one `proof_id` and one `attestation_id`. Identifiers are non-empty strings without control characters. Proof entries are unique by the pair `(proof_id, attestation_id)`.
- The signed tag message carries one each of `Proof-Checkpoint-Purpose`, `Proof-Manifest-Head`, `Proof-Record-Id`, `Proof-Record-Revision`, and `Proof-Candidate-Revision` trailers, and one `Proof-Checkpoint-Proof` trailer for each expected proof entry. Each proof trailer value is a closed JSON object with exactly `proof_id` and `attestation_id`. Trailer keys are matched ASCII case-insensitively; values and the expected field set are compared exactly without normalization.

## Outputs

- One line of JSON `engineering-assurance.proof-checkpoint-verify-result/v1` at exit 0 for every verdict. The closed result includes `status`, `tag`, `tag_object_id`, `checkpoint`, `signer`, `trust`, and `signature`.
- `status` is `verified`, `wrong_checkpoint`, `untrusted_signer`, `signed_bad`, `unsigned`, or `unverifiable`. `checkpoint` reports each parsed field as `match`, `mismatch`, `missing`, or `repeated`, and reports the observed proof population as exact proof/attestation pairs when readable.
- `tag_object_id` is the canonical full object id of the annotated tag object whose message was parsed and whose signature was verified. It is `null` only when the request did not resolve to an annotated tag object.
- `signer` and `signature` preserve the corresponding FR-036, FR-037, and FR-040 verification results. `trust` echoes the allowed-signers path and, when supplied, the configured gitsign identity, OIDC issuer, and principal. A non-verified result never reports an accepted signer.
- A request that cannot yield a verdict writes the `engineering-assurance.error/v1` machine error for `proof-checkpoint-verify` and exits 2.

## Behavior

- The command SHALL resolve the requested tag exactly once to a canonical annotated tag object id, then read the message and signature from that object. It SHALL bind signature verification to the same object id, using an isolated ref pinned to that id when a delegated verifier accepts only a ref. If that binding cannot be guaranteed, the result SHALL be `unverifiable`.
- The command SHALL obtain signature verdicts only through FR-036, FR-037, and FR-040. It SHALL NOT implement cryptography, alter their signature classification, or infer signer identity or trust beyond their returned data.
- The command SHALL report `verified` only when the delegated signature verdict is `signed_valid` with a mapped signer and every required checkpoint field and proof/attestation pair occurs exactly once and matches `expected`.
- A signed tag with a different purpose, digest, record or revision, candidate revision, missing or repeated field, or different proof population SHALL yield `wrong_checkpoint`.
- A good signature from an untrusted identity SHALL yield `untrusted_signer`. A signature classified bad by the delegated verifier SHALL yield `signed_bad`. An unsigned annotated tag SHALL yield `unsigned`.
- An unavailable repository, tag object, verifier, or required trust configuration SHALL yield `unverifiable`. The command SHALL NOT convert any of these cases into a positive result.
- The command SHALL NOT emit `approved`, an approval meaning, a role authorization, or a proof qualification result. A consumer SHALL treat `verified` only as authentication of the checkpoint fields under the returned trust configuration.
- A request that does not resolve to an annotated tag SHALL NOT verify a commit, lightweight tag, note, or mutable ref value as a checkpoint.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-080-CON-1 | A `verified` result SHALL bind checkpoint parsing and delegated signature verification to the same immutable annotated tag object identified by `tag_object_id`. | Safety | Test |
| FR-080-CON-2 | A `verified` result SHALL NOT imply human approval, role authorization, proof validity, or qualification. | Safety | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-080-AC-1 | An annotated tag signed by an SSH key accepted by the supplied allowed-signers file, whose checkpoint fields and complete proof population exactly match the request, yields `verified` with the exact tag object id, mapped signer, and echoed trust input. | Test |
| FR-080-AC-2 | A gitsign-signed tag accepted for the requested certificate identity and OIDC issuer, with matching checkpoint fields, yields `verified` and reports the accepted identity, issuer, and configured principal. | Test |
| FR-080-AC-3 | Altering any checkpoint field, omitting or repeating one, or adding, removing, or changing a proof/attestation pair yields `wrong_checkpoint`, never `verified`. | Test |
| FR-080-AC-4 | A signature from a signer outside the configured trust, a bad signature, an unsigned annotated tag, and an unavailable required verifier yield respectively `untrusted_signer`, `signed_bad`, `unsigned`, and `unverifiable`, with no accepted signer on each refusal. | Test |
| FR-080-AC-5 | Replacing the tag ref after resolution cannot combine fields from one tag object with a successful signature verdict for another; the verifier observes the resolved object id or returns `unverifiable`. | Test |
| FR-080-AC-6 | A result with status `verified` contains no `approved`, role-authorization, or proof-qualification claim, and a consumer can distinguish checkpoint integrity from FR-041 approval. | Test |
| FR-080-AC-7 | A lightweight tag, commit, note, unresolved ref, malformed request, duplicate proof pair, malformed proof trailer, or unsupported/missing trust configuration never yields `verified`; malformed requests produce the declared machine error before invoking git. | Test |

## Dependencies

- **Upstream**: [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md),
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md), and
  [FR-040](./FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md)
  define signature verification, signer mapping, and gitsign trust.
- **Downstream**: Quoin consumes this result as data for the independently
  retained proof-manifest checkpoint. It SHALL NOT treat this result as human
  approval or proof qualification.
