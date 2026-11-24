---
id: FR-087
title: "Document the signing gestures and the shared-agent-session limit"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-084"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-085"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-086"
    type: "references"
---

# FR-087: Document the signing gestures and the shared-agent-session limit

## Description

Tracked by Linear EA-113. Signing through `approval-tag` is non-interactive by default,
so any process that can reach an unlocked `ssh-agent`, including an AI agent in the same
session, can sign as a person. The baseline tier (what a GitHub user has with git,
`ssh-keygen` including security keys, and public Sigstore) offers three per-signature
gestures. The README SHALL explain them so a person can choose, and a reviewer can read
the result. The regulated tier (customer integrations through the signer seams of this
tool) reports through those seams and is not described as a gesture of this tool.

The text states facts and the person decides. It cites the public rule a regulated user
checks, 21 CFR 11.200(a), by section only, and quotes no paywalled standard.

## Behavior

- The README SHALL describe, under one heading, the three gestures, naming each as the
  command and result values do: a FIDO security-key SSH key (`device_touch`,
  `device_touch_and_pin`), `ssh-add -c` (`agent_confirm`) and interactive `gitsign`
  (`oidc_interactive`).
- The README SHALL state, for each gesture, whether the signature evidences it
  (`evidenced`) or only the signer declares it (`declared`), as FR-084 and FR-086 define.
- The README SHALL state that a key unlocked in an agent whose socket other processes of
  the session can reach lets each such process, including an AI agent, sign without a
  person, and that the result then reports `evidenced` `none`.
- The README SHALL state that Engineering Assurance assigns no score, class or
  acceptability to a gesture, and that whether a gesture suffices for a rule the person is
  bound by, such as 21 CFR 11.200(a), is that person's decision.
- The README SHALL show the `gesture` request member of `approval-tag` and the `gesture`
  member of `approval-verify` and `approval-trail`.
- The README SHALL replace the statement that `approval-tag` requires a signer without a terminal
  prompt with the rule of FR-085.
- The README SHALL name no paywalled standard.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-087-AC-1 | The README holds one heading that names `device_touch`, `device_touch_and_pin`, `agent_confirm` and `oidc_interactive`, and states for each whether it is evidenced or declared. | Inspection |
| FR-087-AC-2 | The README states that a key unlocked in a shared agent session lets another process, including an AI agent, sign without a person and that the result then shows `none`. | Inspection |
| FR-087-AC-3 | The README states that no score, class or acceptability is assigned and that the person decides, and cites 21 CFR 11.200(a) by section only. | Inspection |
| FR-087-AC-4 | The `approval-tag` row of the README no longer says the signer must work without a terminal prompt, and states the `gesture` mode of FR-085. | Inspection |
| FR-087-AC-5 | The README contains no paywalled standard designator, checked by the repository's content-rights check. | Test |

## Dependencies

- **Upstream**: [FR-084](./FR-084-evidence-of-the-per-signature-gesture-in-an-ssh-signature.md),
  [FR-085](./FR-085-approval-tag-gesture-signing-mode.md),
  [FR-086](./FR-086-report-the-signing-gesture-in-approval-results.md).
