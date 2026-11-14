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

The workflow uses GraphQL `lastEditedAt` and review edit-history metadata. If
that metadata cannot be read completely, it returns `unavailable`; it never
uses the current edited body as the submitted body. An edited approval is
ignored and must be resubmitted.
