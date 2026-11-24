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

The member holds two things kept apart: what the signature evidences (FR-084) and what the
tag declares (FR-085). A declaration is the signer's claim, so a declaration the signature
does not support is shown as unsupported and is never raised to evidence. The result does
not decide whether the gesture suffices for any rule: the `status` is unchanged by it and
no member scores, classes or accepts a gesture. A person accepts the risk.

Out of scope: the status decision (FR-041), the signing mode (FR-085), the evidence itself
(FR-084), and customer-integrated signers, which report through their own seams.

## Outputs

- `gesture`, present on `approved` only, an object with:
  - `evidenced`: the value of `signature.gesture.evidenced` (FR-084);
  - `declared`: the value of the single `Signing-Gesture` trailer when it is one of
    `device_touch`, `agent_confirm`, `oidc_interactive`; `unrecognized` when the trailer is
    present with another value or repeated; `none` when the tag has no such trailer;
  - `declared_status`: `undeclared` when `declared` is `none`; `evidenced` when `declared`
    is `device_touch` and `evidenced` is `device_touch` or `device_touch_and_pin`;
    `not_evidenced` in every other case, including every `agent_confirm` and
    `oidc_interactive` declaration, because no signature evidences them.
- A status other than `approved` carries no `gesture`, as it carries no signing time.

## Behavior

- The command SHALL read the declaration only through the trailer reader of FR-037, asking
  for the key `Signing-Gesture`.
- The command SHALL copy `evidenced` from the FR-084 verdict of the same run.
- The command SHALL NOT decide `evidenced` again.
- The command SHALL report `declared_status` `not_evidenced` for a declaration of
  `device_touch` made on a tag whose `evidenced` is `none` or `device_touch_unenforced`.
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
| FR-086-AC-1 | An approval tag signed with a software key and made without `gesture` yields `approved` with `gesture` equal to `{"evidenced":"none","declared":"none","declared_status":"undeclared"}`. | Test |
| FR-086-AC-2 | A tag made with `gesture` `agent_confirm` by a software key yields `approved`, `evidenced` `none`, `declared` `agent_confirm` and `declared_status` `not_evidenced`; the same for `oidc_interactive` through the gitsign stand-in. | Test (stand-in) |
| FR-086-AC-3 | With the FR-084 stand-in reporting a security-key type, a tag declaring `device_touch` yields `evidenced` `device_touch`, `declared_status` `evidenced`; with `device_touch_and_pin` evidenced it is `evidenced` too. | Test (stand-in) |
| FR-086-AC-4 | A software-key tag declaring `device_touch` yields `approved` with `declared_status` `not_evidenced` and the same `status`, `approver` and `meaning` as the tag without the declaration. | Test |
| FR-086-AC-5 | A tag with a `Signing-Gesture` trailer of another value, and a tag repeating the trailer, each yield `declared` `unrecognized` and `declared_status` `not_evidenced`. | Test |
| FR-086-AC-6 | A tag with status `wrong_record`, `untrusted_signer`, `meaning_missing`, `signed_bad`, `unsigned` or `unverifiable` carries no `gesture` member. | Test |
| FR-086-AC-7 | An `approval-trail` run lists an approved entry whose `gesture` equals the `approval-verify` result for the same tag, for a software-key tag and a declared one. | Test |
| FR-086-AC-8 | With a real FIDO security key present, `approval-tag` with `gesture` `device_touch`, the key touched, yields a tag that `approval-verify` reports `approved` with `evidenced` `device_touch` and `declared_status` `evidenced`; where no device is present the test is reported not executed and is not counted as passed. | Test (hardware) |
| FR-086-AC-9 | No result of either command holds a member that scores, classes or accepts a gesture, and `status` is equal with and without `gesture` for the same tag. | Test |

## Dependencies

- **Upstream**: [FR-041](./FR-041-signed-tag-approval-of-a-change-assurance-record.md),
  [FR-043](./FR-043-approval-trail-of-a-change-assurance-record.md),
  [FR-084](./FR-084-evidence-of-the-per-signature-gesture-in-an-ssh-signature.md),
  [FR-085](./FR-085-approval-tag-gesture-signing-mode.md).
