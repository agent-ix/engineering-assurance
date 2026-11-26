---
id: FRR-001
title: "Engineering Assurance proposed high process-risk supplier assessment"
type: FeatureRiskRating
scope: supplier
rating: high_process_risk
status: proposed
relationships:
  - target: "ix://agent-ix/engineering-assurance/IU-001"
    type: uses_intended_use
  - target: "ix://agent-ix/engineering-assurance/FR-005"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-009"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-023"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-025"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-032"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-036"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-037"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-040"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-041"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-042"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-043"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-047"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-048"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-049"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-051"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-058"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-060"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-061"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-062"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-063"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-064"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-065"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-066"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-067"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-068"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-069"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-074"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-075"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-077"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-078"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-080"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-081"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-082"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-084"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-085"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-086"
    type: rates_feature
---
# Engineering Assurance proposed high process-risk supplier assessment

## Rationale

This is a proposed supplier-side rating for IU-001. The named human assigned to review and accept it is Peter Krenesky. The record is unsigned and unaccepted pending his decision through the signed-commit workflow. No score or product-safety classification is assigned.

The listed features can affect whether engineering evidence is complete, correctly attributed, validated, private, or accepted by the stated policy. A failure could therefore make an engineering record appear verified, traceable, published, or approved when its supporting evidence does not justify that result. These proposed ratings are human judgments for review, not outputs calculated from code, test, coverage, or performance data.

## Feature-specific false-claim reasons

| FR | Reason the feature can falsely claim assurance |
| --- | --- |
| FR-005 | a workflow can report a terminal human decision without the required human decision. |
| FR-009 | an evidence record can convert a producer failure or absent provenance into apparent successful evidence. |
| FR-023 | claim validation can accept a supported assurance claim without its required evidence references. |
| FR-025 | measurement promotion can accept a result that the required typed checker did not support. |
| FR-032 | change-scope verification can report a candidate within scope when its footprint exceeds the declaration. |
| FR-036 | SSH signature verification can report an invalid commit/tag signature as valid. |
| FR-037 | signer mapping can attribute an approval trailer to the wrong named person or meaning. |
| FR-040 | gitsign verification can report an invalid keyless signature as valid. |
| FR-041 | signed-tag verification can report a tag as a named person's approval when the tag/signature does not qualify. |
| FR-042 | approval-tag creation can issue an approval assertion for a different change-assurance record. |
| FR-043 | approval-trail output can include a signature as a verified approval when it does not verify. |
| FR-047 | release verification can accept a baseline without the corresponding valid named person's release tag. |
| FR-048 | a release workflow can record a terminal accepted decision without the human gate's accepted action. |
| FR-049 | release verification can report a release accepted when its baseline or approval evidence is invalid. |
| FR-051 | release verification can pass when the required evidence snapshot is absent from the release commit. |
| FR-058 | a change-approval statement can bind approval to the wrong pull request or merge commit. |
| FR-060 | approval binding can accept a pull-request head other than the one that merged. |
| FR-061 | role-policy verification can count one person's signature for multiple distinct required roles. |
| FR-062 | role-policy verification can accept an AI agent identity as a required human approver. |
| FR-063 | workflow admission can report an approval role satisfied when no staffed approver exists. |
| FR-064 | approval verification can accept a statement with the wrong workflow identity or role policy. |
| FR-065 | release workflow policy can mark review complete without the required human review before merge. |
| FR-066 | release attestation can state verified properties for artifacts that were not verified. |
| FR-067 | release attestation can be emitted despite an invalid release tag or unsatisfied role policy. |
| FR-068 | a signed verification summary can claim a release result not supported by its verification inputs. |
| FR-069 | Quoin Test Result ingestion can map failed or unusable evidence into a passing result. |
| FR-074 | a signed receipt can report a different result for a merged commit than the recorded gate outcome. |
| FR-075 | receipt generation can sign a pass for a commit without the corresponding verification result. |
| FR-077 | receipt verification can report an unreceipted or invalidly receipted commit as verified. |
| FR-078 | receipt verification can accept a receipt for a different commit or workflow identity. |
| FR-080 | proof-integrity verification can accept a statement bound to the wrong tag object or output. |
| FR-081 | tag receipt generation can attribute an approval/release tag to the wrong tag object. |
| FR-082 | tag comparison can report the tag set intact when a tag was changed, deleted, or lacks a valid receipt. |
| FR-084 | signature-gesture reporting can state a person intended approval when the signature evidences a different gesture. |
| FR-085 | approval-tag creation can encode an approval gesture the signer did not select. |
| FR-086 | approval verification/trail can report the wrong gesture as the signer's evidenced action. |

## Assurance activities

| Activity | Acceptance criterion |
|---|---|
| Interrupt and resume each supported workflow, checking completed phases and next action; reuse the id with changed repository, workflow version, or boundary and assert refusal without state mutation; test explicit accept and reject choices. | FR-005-AC-1–7 |
| Round-trip valid producer tuples and definition versions, then exercise each non-success state; verify projected values cite source record and field path and never claim a copied authority. | FR-009-AC-1–4 |
| Validate a supported claim with one valid ix:// reference, then remove it, empty the list, duplicate it, and use a non-ix URI; assert each unsupported shape is refused. | FR-023-AC-1–3 |
| Submit typed checker fixtures for each policy mode and stage; assert only the permitted result advances and verify retired historical profiles remain readable while active profiles require the current schema version. | FR-025-AC-1–8 |
| Compare exact-file and directory-prefix scopes against fixture change footprints; add an uncovered path and assert incomplete, then submit unreadable, malformed, invalid-scope, and option-like revisions and assert no Git process starts. | FR-032-AC-1–7 |
| Use real repository fixtures with allowed and disallowed keys for signed commits and annotated tags; assert signed_valid, unsigned, signed_bad, and signed_untrusted outcomes, including required ssh-keygen failure cases. | FR-036-AC-1–16 |
| Verify commit and tag fixtures against allowed-signers mappings and trailer blocks; alter principal, meaning, and trailer placement independently and assert mapping/status and returned trailers match the signed object. | FR-037-AC-1–25 |
| Use CMS-signed commit and tag fixtures with stand-in gitsign results for exact and mismatched identities/issuers; assert verification status, signature kind, certificate fields, and resolved identity-registry commit. | FR-040-AC-1–31 |
| Create annotated-tag fixtures with exact and altered record id/digest, principal, and Signed-off-meaning trailer; assert only the fully matching signature yields approved and every failure is reported. | FR-041-AC-1–37 |
| Create a fixture tag with a throwaway SSH signer and verify it with approval-verify; compare the caller-supplied digest bytes and trailers exactly, and inspect that no push or configuration change occurs. | FR-042-AC-1–13 |
| Create valid approved and revoked tags plus missing, unreadable, and invalid receipt fixtures; compare the complete oldest-first trail, tag object ids, statuses, truncation, and unreadable entries. | FR-043-AC-1–34 |
| Verify signed and unsigned annotated tag fixtures against matching and altered Baseline records; assert only the exact named signer, baseline, one-child release commit, and required snapshot yield released. | FR-047-AC-1–17 |
| Interrupt and resume release-decision fixtures; exercise accept, reject, missing decision, changed repository, changed boundary, and stale state, asserting only the explicit named-person action is terminal. | FR-048-AC-1–20 |
| Compare the emitted release-verification result with the declared Quoin input contract; inspect that the record is passed as data and no Quoin schema or record copy exists in this repository. | FR-049-AC-1–4 |
| Verify a release commit with exactly the record and snapshot against its Baseline; remove or alter the snapshot path and assert refusal, and inspect that snapshot members are never parsed. | FR-051-AC-1–9 |
| Create fixture approval statements and compare every predicate member with the approval source; recompute canonical JSON bytes and signature input, then vary the reviewed head and merged commit. | FR-058-AC-1–10 |
| Use fixture pull requests merged by squash and rebase; assert head_commit is the reviewed head and merged_commit is only the resulting merge/rebased tip. | FR-060-AC-1–6 |
| Evaluate a three-role policy with three eligible individuals, then reuse one individual across roles or list an ineligible role; assert satisfied only for distinct eligible people. | FR-061-AC-1–13 |
| Submit otherwise-valid APPROVED reviews from bot/agent accounts and assert no approval or statement is produced; generate approval sets and assert zero roles are covered by agents. | FR-062-AC-1–5 |
| Use policies with an empty role roster and with failed account-type lookups; assert the first is unsatisfied with approval_role_unstaffed and the second unavailable with approval_source_unavailable. | FR-063-AC-1–6 |
| Verify a statement from the protected workflow identity at the pinned ref for the expected commit/PR; vary identity, ref, policy, payload, and role coverage and assert refusal before publication. | FR-064-AC-1–25, FR-064-AC-27–29 |
| Parse the reusable workflow role/gate definitions; evaluate human and agent-authored pull requests and confirm unsigned release tags remain not_released even when approval statements exist. | FR-065-AC-1–10 |
| Generate a release with two fixture artifacts; assert exactly four statements and compare each subject digest with the artifact bytes and summary subject with release commit. | FR-066-AC-1–8 |
| Run release fixtures with valid and invalid signer tags and satisfied/unsatisfied role evidence; assert the full attestation set only follows both successful checks. | FR-067-AC-1–10 |
| Verify the fixture summary with the named reusable workflow and a stand-in gh verifier; add an unpinned signer or extra predicate field and assert refusal. | FR-068-AC-1–6 |
| Pass through authenticated PASSED and FAILED exports and compare bytes/status unchanged; invalidate the workflow-computed subject and assert rejection before attestation. | FR-069-AC-1–6 |
| Generate passing and failing change fixtures; assert each produces three signed statements, matching change-record status and PASSED/FAILED summary, with input digests matching the other statements. | FR-074-AC-1–7 |
| Parse the workflow and assert push-to-main trigger, no agent job, no repository checkout/code in signing job, and protected policy/approval inputs for valid and failed receipt cases. | FR-075-AC-1–9 |
| Use a five-commit fixture with missing, tampered, and unreadable notes; assert exact reason/status entries and grace-period handling for bypassed or absent workflow runs. | FR-077-AC-1–7 |
| Verify passing and failing receipt fixtures, then change signer identity, commit, policy, or summary; assert only the matching pinned workflow, commit, and role policy verifies. | FR-078-AC-1–7 |
| Verify annotated-tag fixtures with exact and one-field-mismatched purpose, statement digest, full scope, and tag object id; assert only exact matches return verified and malformed ids start no process. | FR-080-AC-1–10 |
| Push prefixed annotated-tag fixtures and verify one note on the peeled commit with the exact tag-object subject; make signer unavailable and assert no receipt or public-log call for private repos. | FR-081-AC-1–11 |
| Compare tag sets with verified, absent, modified, and missing receipt fixtures; assert exact missing_tag/tag_no_receipt/invalid_receipt findings and that untrusted refs are omitted where required. | FR-082-AC-1–14 |
| Verify a tag signed by a software key and assert gesture evidenced none; compare bytes passed to stand-in ssh-keygen with the original armored signature block and inspect for no signature-block rewriting. | FR-084-AC-1–10 |
| Create tags with and without gesture under a pseudo-terminal; assert process-group/terminal behavior, exact message trailers and result fields, and that signer cancellation/failure writes no tag. | FR-085-AC-1–13 |
| Verify software-key and declared-gesture tags through both commands; compare declared/evidenced fields and assert status is identical with and without the gesture declaration. | FR-086-AC-1–8 |
