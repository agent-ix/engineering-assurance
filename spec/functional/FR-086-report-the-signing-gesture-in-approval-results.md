---
id: FR-086
title: "Report the signing gesture in approval-verify and approval-trail results"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-041"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-043"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-084"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-085"
    type: "references"
---

# FR-086: Report the signing gesture in approval-verify and approval-trail results

## Description

Tracked by Linear EA-113. A reviewer reading an approval result must see whether the
signature needed a person at a device, and what the signer claimed. This requirement adds
one member, `gesture`, to an `approved` result of `approval-verify`
([FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md)) and to each
`approved` entry of `approval-trail`
([FR-043](./FR-043-approval-trail-of-a-change-assurance-record.md)). It is a separate
additive member: it changes no existing member, status or sentence of FR-041 or FR-043.

The member holds the declaration only. What the signature evidences is already in the
`signature.gesture.evidenced` member of the same result (FR-084), which FR-041 embeds
unchanged. A declaration is the signer's claim: it is the only record of `ssh-add -c` and
of an interactive `gitsign` login, because no signature shows them, and it is never raised
to evidence. The result does not decide whether the gesture suffices for any rule: the
`status` is unchanged by it and no member scores, classes or accepts a gesture. A person
accepts the risk.

Out of scope: the status decision (FR-041), the signing mode (FR-085), the evidence itself
(FR-084), and customer-integrated signers, which report through their own seams.

## Outputs

- `gesture`, present on `approved` only, an object with exactly one member `declared`: the
  value of the single `Signing-Gesture` trailer when it is one of `device_touch`,
  `agent_confirm`, `oidc_interactive`; `unrecognized` when the trailer is present with
  another value or repeated; `none` when the tag has no such trailer.
- A status other than `approved` carries no `gesture`, as it carries no signing time.

## Behavior

- The command SHALL read the declaration only through the trailer reader of FR-037, asking
  for the key `Signing-Gesture`.
- The command SHALL NOT change `status`, `approver`, `meaning` or the binding because of
  `gesture`.
- The command SHALL add no member that scores, classes or accepts a gesture.
- The `approval-trail` command SHALL carry the `gesture` member of each approved entry
  exactly as `approval-verify` reports it for that tag.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-086-CON-1 | The commands SHALL add no signature, certificate or trailer-parsing code for `gesture`; the facts come from FR-084 and the FR-037 reader. | Architecture | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-086-AC-1 | An approval tag signed with a software key and made without `gesture` yields `approved` with `gesture` equal to `{"declared":"none"}` and `signature.gesture.evidenced` `none`. | Test |
| FR-086-AC-2 | A tag made with `gesture` `agent_confirm` by a software key, and one with `oidc_interactive` through the gitsign stand-in, yield `approved` with `declared` equal to the requested value and `signature.gesture.evidenced` `none`. | Test (stand-in) |
| FR-086-AC-3 | A software-key tag declaring `device_touch` yields `approved` with `declared` `device_touch`, `signature.gesture.evidenced` `none`, and the same `status`, `approver` and `meaning` as the tag without the declaration. | Test |
| FR-086-AC-4 | A tag with a `Signing-Gesture` trailer of another value, and a tag repeating the trailer, each yield `declared` `unrecognized`. | Test |
| FR-086-AC-5 | A tag with status `wrong_record`, `untrusted_signer`, `meaning_missing`, `signed_bad`, `unsigned` or `unverifiable` carries no `gesture` member. | Test |
| FR-086-AC-6 | An `approval-trail` run lists approved entries for an undeclared software-key tag and a declared software-key tag; for each tag, the entry's `gesture` equals the `approval-verify` result for that same tag. | Test |
| FR-086-AC-7 | With a real FIDO security key present, `approval-tag` with `gesture` `device_touch`, the key touched, yields a tag that `approval-verify` reports `approved` with `declared` `device_touch` and `signature.gesture.evidenced` `device_touch`; where no device is present the test is reported not executed and is not counted as passed. | Test (hardware) |
| FR-086-AC-8 | No result of either command holds a member that scores, classes or accepts a gesture, and `status` is equal with and without `gesture` for the same tag. | Test |

## Dependencies

- **Upstream**: [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md),
  [FR-043](./FR-043-approval-trail-of-a-change-assurance-record.md),
  [FR-084](./FR-084-evidence-of-the-per-signature-gesture-in-an-ssh-signature.md),
  [FR-085](./FR-085-approval-tag-gesture-signing-mode.md).
