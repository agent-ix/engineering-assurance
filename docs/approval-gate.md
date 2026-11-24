<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->
<!-- Copyright (C) 2026 Agent-IX -->

# Opt-in human review gate

The `review-gate` CLI evaluates approvals against a role policy from the pull
request's protected base revision. GitHub accounts are matched by numeric ID;
logins are display-only. Each required role must be covered by a distinct
eligible human. Policy-listed agents, GitHub bots, applications, unknown
accounts, and edited review bodies do not count. After an edited review, the
reviewer must submit a fresh review with its marker lines.

Copy [`examples/approval-policy.json`](../examples/approval-policy.json) to
`.engineering-assurance/approval-policy.json` and replace its fictional IDs
with the numeric GitHub user IDs for the repository's role holders. Every
policy individual must have at least one account, role names and individual
names use lowercase letters, digits, `_` and `-`, and each required role must
have at least one eligible individual. A person may hold multiple roles, but
one person can satisfy only one role in a single evaluation.

Review bodies must contain exactly one marker line of each form, at the start
of a line:

```text
Signed-off-meaning: reviewed the change for security impact
Role: security
```

The meaning is 1 to 256 UTF-8 bytes without control characters. A role must
appear in the reviewer's policy assignments. The review must be the reviewer's
latest submitted decision (`APPROVED`, `CHANGES_REQUESTED`, or `DISMISSED`) on
the current pull-request head.

## Opt in

The workflow is skipped unless repository variable
`EA_REVIEW_GATE_ENABLED` is exactly `true`. Set it only after installing the
workflow and policy and verifying the resulting check. The workflow requests
only `contents: read` and `pull-requests: read`, and declares no signing or
identity-token permission. Its status check is
`review-gate / review-gate` from the GitHub Actions integration. Reusable
workflow callers should pin the workflow ref to an approved commit or release;
the optional `tool_ref` input defaults to `main` and exists to run a reviewed
candidate ref in the isolated sandbox.

[`examples/main-review-gate-ruleset.json`](../examples/main-review-gate-ruleset.json)
is a manually applied ruleset payload. It targets `main`, requires the strict
`review-gate / review-gate` status check from GitHub Actions, and defines no
bypass actors. `scripts/install-review-gate-ruleset.sh` is restricted to a
private `agent-ix/ea-109-sandbox-*` repo, defaults to a dry run, and requires
`--apply` to create or update that sandbox's ruleset. Applying a ruleset is an
explicit repository administration step. This project does not apply it to any
active repository; use it only where the approval policy is intended to block
merges until distinct eligible humans approve.

## Detecting deleted tags and protecting refs

A git tag can be deleted or moved, and a deleted tag leaves nothing in the tag
set to show it was ever there. The optional `tag_receipts` member of the receipt
policy (`.engineering-assurance/receipt-policy.json`, read from the protected
base branch) lists literal tag-name `prefixes`; the reusable workflow
`tag-receipt.yml`, called by `tag-receipt-push.yml` on every tag push, signs one
receipt for each pushed tag that starts with a prefix and stores it as one
Sigstore bundle per line of a git note on the tagged commit under
`refs/notes/assurance-tag-receipts`. A receipt records only what was pushed:
the ref, the tag object id, the commit, the platform's push time and the
`Approved-Record-Id` trailer values. The identity that signs it is a
`receipt_writer` entry of the workflow-identity policy with `source_ref_prefix`
(`refs/tags/` followed by the tag prefix).

`receipt-missing` compares the receipts with the current tags and lists a
`missing_tag` finding (`tag_absent` or `tag_changed`), a `tag_no_receipt`
finding for a current prefix tag whose object has no receipt, and an
`invalid_receipt` finding for a receipt that does not verify; the scheduled
`missing-receipt-check.yml` fails on any of them. `approval-trail` applies the
same comparison to the receipts that belong to one record (it takes a
`tag_receipts` request member naming the two policy files). A person records
acceptance of a specific finding, such as a release tag that predates adoption,
in the `accepted_tags` list of the `tag_receipts` member: each entry names the
`ref`, the `tag_object` (null accepts only an absent tag), `accepted_by` and
`reason`. A matched finding is reported as `accepted` and does not fail the
workflow.

Protection is platform setup that Engineering Assurance does not enforce, and
detection is what the tool tests. Two example ruleset payloads are applied
manually by the repository's operator, as the review-gate ruleset is:

- [`examples/tag-protection-ruleset.json`](../examples/tag-protection-ruleset.json)
  is a `tag` ruleset over `refs/tags/*` that forbids `deletion` and `update`, so
  no approval or release tag can be deleted or moved.
- [`examples/private-records-branch-ruleset.json`](../examples/private-records-branch-ruleset.json)
  is a `branch` ruleset over `refs/heads/engineering-assurance-private-records`
  that forbids `deletion` and `non_fast_forward`. That ref lives in the private
  store repository named in the sink configuration, so this ruleset is applied
  there.

Both payloads have an empty `bypass_actors` list: the bypass list names no person
who can sign approvals (no member of any role of the role policy) and no agent
account. A GitHub ruleset has exactly one target type, which is why there are two
payloads.

The notes refs, `refs/notes/assurance-receipts` and
`refs/notes/assurance-tag-receipts`, are neither branches nor tags, and no ruleset
can target them; for them detection is what exists. A deleted or rewritten
change-receipt note shows as `no_note`, and a deleted tag note leaves its tag
unreceipted (`tag_no_receipt`). A tag deleted at the same time as its receipt is
not detectable by the tool: that is a residual risk that a person accepts or
addresses with platform controls, in general terms by restricting who holds write
access. The identity that writes receipts needs only to create and fast-forward
the notes refs. A repository without these rulesets still gets detection from the
tag receipts and gets no protection.

The workflow uses GraphQL `lastEditedAt` and review edit-history metadata. If
that metadata cannot be read completely, it returns `unavailable`; it never
uses the current edited body as the submitted body. An edited approval is
ignored and must be resubmitted.
