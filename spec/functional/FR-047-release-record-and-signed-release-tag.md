---
id: FR-047
title: "Verify a Release as a Baseline plus a named person's signed release tag"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-046"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-036"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-037"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-041"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-042"
    type: "references"
---

# FR-047: Verify a Release as a Baseline plus a named person's signed release tag

## Description

Engineering Assurance SHALL provide a `Release` archetype and a `release-verify`
command. A Release is a Baseline ([FR-046](./FR-046-baseline-archetype.md)) read
at one commit plus a named person's release decision. The decision is a signed
annotated git tag. The `Release` document is the release record: it names the
Baseline, the baseline commit hash, the person who is to decide
(`decision_owner`), and the anomalies that person accepted (`accepted_anomalies`).

The tag is the approval object of
[FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md) and is
written by `approval-tag`
([FR-042](./FR-042-create-signed-approval-tag-for-a-change-assurance-record.md)):
its message names the release record by `Approved-Record-Id` (the record `id`),
carries `Approved-Record-Digest`, and carries a `Signed-off-meaning:` trailer
that states what the person decided (FR-037). `release-verify` adds no
cryptography and restates no signature rule. It runs the `approval-verify`
verification of FR-041 and adds only what a release needs: the record is read
from the tagged commit, the baseline commit it names is that commit or an
ancestor of it, every baseline item exists at that commit, and the verified
approver is the named decision owner.

The release record is committed after the baseline commit, so the tag points at
the commit that holds the record, and the record names the earlier baseline
commit. The record digest that the tag carries is `sha256:` followed by the
lowercase hexadecimal SHA-256 of the record file's bytes at the tagged commit,
which is the digest form `approval-tag` accepts; it binds the signature to the
exact record and is not a digest of the baseline. The baseline stays identified
by its commit hash alone.

Out of scope: creating the tag (FR-042 does it), deciding who may release or
which meaning suffices, and any release without a person's signature.

## Inputs

- One request document `engineering-assurance.release-verify-request/v1` with
  `protocol`, `repository` (absolute directory), `tag`, `record_path`
  (repository-relative path of the release record), `allowed_signers` (absolute
  file) and optionally `gitsign`, with the bounds FR-041 sets for them. Unknown
  members are rejected.
- One Release document with frontmatter `id`, `title`, `type: Release`, `name`,
  `baseline` (an object with `path`, the repository path of the Baseline file,
  and `commit`, the full commit hash), `decision_owner` (a non-empty name that
  must equal the allowed-signers principal or the gitsign principal), and
  optional `accepted_anomalies` (a list of strings).

## Outputs

- One line of JSON, protocol `engineering-assurance.release-verify-result/v1`,
  exit status 0 for every status, with `status` one of `released`,
  `not_released`, and for `not_released` a `reasons` list, each one of the
  FR-041 statuses other than `approved`, `record_unreadable`, `record_invalid`,
  `baseline_commit_unknown`, `baseline_not_ancestor`, `baseline_item_absent`,
  `decision_owner_mismatch`.
- Always: the `approval` member, the unchanged FR-041 result for the tag.
- On `released` only: `release` (record `id`, `name`), `baseline` (`name`,
  `commit`, and each item `path`, `kind`), `approver`, `meaning`, `signed_at`
  with its source, and `accepted_anomalies`.
- A request that yields no verdict writes the `engineering-assurance.error/v1`
  machine error and exits 2.

## Behavior

- The command SHALL read the release record from the commit the tag peels to,
  with git object access, and SHALL NOT read the working tree.
- The command SHALL compute the record digest as stated and pass the record
  `id` and that digest as `record_id` and `record_digest` to the FR-041
  verification.
- The command SHALL report `released` only when the FR-041 status is `approved`
  and every other check passes.
- The command SHALL report all failing checks it can evaluate, not only the
  first.
- The command SHALL report `decision_owner_mismatch` when the FR-041 `approver`
  differs from `decision_owner` by any byte.
- The command SHALL derive the person only from the FR-041 `approver` and SHALL
  NOT derive one from the record, the tagger, the author or an email.
- The command SHALL require `meaning` to be present, by the FR-041 rule, and
  SHALL NOT decide what a meaning permits.
- The command SHALL NOT record, compute or accept a digest, hash or manifest
  of the baseline; the baseline is identified by `baseline.commit` alone.
- The command SHALL NOT create, move, push or delete any ref.
- The command SHALL NOT report `released` from a record, a commit signature, a
  merge commit, a note or a lightweight tag.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-047-CON-1 | A result SHALL NOT be `released` unless the FR-041 status for the tag is `approved`. | Safety | Test |
| FR-047-CON-2 | The command SHALL contain no signature, certificate or trailer-parsing code of its own and no rule of FR-036, FR-037, FR-040 or FR-041 restated. | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-047-AC-1 | A tag signed by the key the allowed-signers file maps to the record's `decision_owner`, whose message carries the record id, the record digest and a `Signed-off-meaning` trailer, on a commit holding the record that names an ancestor baseline commit whose items all exist, yields `released` with `approver`, `meaning`, `signed_at` and the baseline items. | Test |
| FR-047-AC-2 | Changing one byte of the release record after the tag was signed yields `not_released` with FR-041 status `wrong_record`. | Test |
| FR-047-AC-3 | A tag signed by a mapped principal that differs from `decision_owner` yields `not_released` with reason `decision_owner_mismatch` and no `release` member. | Test |
| FR-047-AC-4 | An unsigned annotated tag, a lightweight tag, a bad signature, an unmapped key and a tag with no `Signed-off-meaning` each yield `not_released` carrying the corresponding FR-041 status. | Test |
| FR-047-AC-5 | A baseline commit that is not in the repository yields `baseline_commit_unknown`; one that exists but is not the tagged commit or an ancestor of it yields `baseline_not_ancestor`; a baseline item path absent at the baseline commit yields `baseline_item_absent` naming the path. | Test |
| FR-047-AC-6 | A record whose frontmatter is invalid, or that carries a digest, hash or manifest member for the baseline, yields `record_invalid`; a missing record path at the tagged commit yields `record_unreadable`. | Test |
| FR-047-AC-7 | A record edited only in the working tree after tagging does not change the verdict, which is read from the tagged commit. | Test |
| FR-047-AC-8 | A signed commit, a merge commit or a note carrying the record trailers yields `not_released` and never `released`. | Test |
| FR-047-AC-9 | The `approval` member equals the output of `approval-verify` for the same tag, record id and digest, and the command refuses a request with an unknown member, a relative path or a `-`-leading value with exit 2 before starting git. | Test |
| FR-047-AC-10 | Static inspection shows no signature, certificate or trailer-parsing code in the release module (CON-2). | Inspection |
| FR-047-AC-11 | The decision from the FR-041 status and the record checks is exercised without any process, over every combination, and `released` is returned for exactly one. | Property |

## Dependencies

- **Upstream**: [FR-046](./FR-046-baseline-archetype.md),
  [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md),
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
  [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md),
  [FR-042](./FR-042-create-signed-approval-tag-for-a-change-assurance-record.md).
- **Downstream**: [FR-048](./FR-048-human-release-decision-workflow.md),
  [FR-049](./FR-049-release-ownership-split-and-data-contract.md).
