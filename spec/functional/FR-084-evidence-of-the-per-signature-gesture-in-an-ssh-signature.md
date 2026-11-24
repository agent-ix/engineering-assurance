---
id: FR-084
title: "Report the per-signature gesture that an SSH signature evidences"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-036"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-037"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-040"
    type: "references"
---

# FR-084: Report the per-signature gesture that an SSH signature evidences

## Description

Tracked by Linear EA-113. A signature made by a key that any process with an unlocked
agent can use proves a key was used, not that a person was present. A FIDO security-key
SSH key (`ed25519-sk` or `ecdsa-sk`) differs: the key never leaves the device, and the
device signs only after a touch (user presence) and, when the key was created with
`-O verify-required`, a PIN (user verification). OpenSSH records both facts in the
signature it makes, and `ssh-keygen -Y verify` enforces user presence by default and user
verification when asked with `-O verify-required`.

The `signature-verify` command ([FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md))
therefore reports, for an accepted SSH signature, which gesture the signature evidences,
so that a reviewer can see whether the signature needed a person at a device. The report
is evidence from the signature alone. It does not say whether that gesture suffices for
any rule: Engineering Assurance assigns no score, class or acceptability, and the person
who accepts the risk decides.

Two other per-signature gestures leave no trace in a signature: `ssh-add -c` (the agent
asks for confirmation on each use) and an interactive `gitsign` OIDC login. A signature
made that way is byte-for-byte the shape of an unconfirmed one, so this requirement
reports them as `none` and never as a gesture. A caller may declare such a mode when it
creates an approval tag (FR-085), and FR-086 reports the declaration as a claim beside
this evidence.

Out of scope: creating or configuring a signing key, the declared mode (FR-085, FR-086),
the OpenPGP signature kind (FR-036 does not verify it), and a customer-integrated signer
reached through a seam, which reports its own gesture.

## Inputs

- The request, the allowed-signers file and the signature FR-036 and FR-037 already
  read. No new request member.

## Outputs

- On a `signed_valid` verdict, one added member `gesture`, an object with exactly one
  member `evidenced`, whose value is exactly one of:
  - `device_touch_and_pin`: an SSH signature by a FIDO security-key type whose user
    verification `ssh-keygen -Y verify -O verify-required` accepts;
  - `device_touch`: an SSH signature by a FIDO security-key type accepted without
    `-O verify-required` whose allowed-signers entry does not carry
    `no-touch-required`, so `ssh-keygen` enforced user presence;
  - `device_touch_unenforced`: an SSH signature by a FIDO security-key type whose
    allowed-signers entry carries `no-touch-required`, so user presence was not
    enforced and touch is not evidenced;
  - `none`: an SSH signature by any other key type, and every gitsign signature.
- No other status carries `gesture`: a signature that is not accepted evidences nothing.
- The protocol value `engineering-assurance.signature-verify-result/v1` and every
  existing member are unchanged.

## Behavior

- The command SHALL decide the key type from the output of the `ssh-keygen` run FR-036
  already makes, and SHALL NOT parse the signature block itself.
- The command SHALL, for a security-key type only, make one further `ssh-keygen -Y
  verify` run with `-O verify-required`, with the same bounds as FR-036, to decide
  `device_touch_and_pin`.
- The command SHALL report `none` when the key type is not a security-key type.
- The command SHALL report `device_touch_unenforced` for a security-key type whose
  matched allowed-signers entry carries the `no-touch-required` option.
- The command SHALL omit `gesture` for `signed_untrusted`, `signed_bad`, `unsigned` and
  `unverifiable`.
- The command SHALL NOT read the tag message or any trailer to decide `evidenced`.
- The command SHALL report `gesture.evidenced` equal to `none` for a gitsign signature,
  whichever OIDC issuer the certificate names.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-084-CON-1 | The command SHALL contain no SSH signature, key-blob or certificate parsing; every fact in `gesture` comes from `ssh-keygen` output or exit status. | Architecture | Inspection |
| FR-084-CON-2 | The command SHALL add no member that scores or classes a gesture, or states a verdict on it. | Safety | Inspection |

## Acceptance Criteria

Stand-in note: a stand-in `ssh-keygen` on `PATH` reproduces the documented output of the
real one for a security-key type; where a FIDO device is not present, only the stand-in
criteria run.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-084-AC-1 | A commit and a tag signed with a throwaway software ed25519 key and accepted report `signed_valid` with `gesture` equal to `{"evidenced":"none"}`. | Test |
| FR-084-AC-2 | With the stand-in `ssh-keygen` reporting an `ED25519-SK` key and accepting `-O verify-required`, an accepted signature reports `device_touch_and_pin`. | Test (stand-in) |
| FR-084-AC-3 | With the stand-in reporting an `ED25519-SK` key and rejecting `-O verify-required`, an accepted signature reports `device_touch`, and the verdict status stays `signed_valid`. | Test (stand-in) |
| FR-084-AC-4 | With the stand-in reporting an `ECDSA-SK` key and an allowed-signers entry carrying `no-touch-required`, the accepted signature reports `device_touch_unenforced`. | Test (stand-in) |
| FR-084-AC-5 | A gitsign signature accepted through the FR-040 stand-in reports `gesture` equal to `{"evidenced":"none"}`, for a certificate naming an interactive-login issuer and for one naming a workflow issuer. | Test (stand-in) |
| FR-084-AC-6 | A `signed_untrusted`, `signed_bad`, `unsigned` and `unverifiable` verdict each carry no `gesture` member. | Test |
| FR-084-AC-7 | A tag signed by a software key whose message carries a `Signing-Gesture: device_touch` trailer reports `gesture` equal to `{"evidenced":"none"}`. | Test |
| FR-084-AC-8 | With a real FIDO security key present, a commit signed with it, the key touched, reports `device_touch` and with a key created with `-O verify-required` and the PIN given reports `device_touch_and_pin`; where no device is present the test is reported not executed and is not counted as passed. | Test (hardware) |
| FR-084-AC-9 | Every pre-existing member and the protocol value of a `signed_valid` result are equal to the result without `gesture`. | Test |
| FR-084-AC-10 | Static inspection shows no signature-block, key-blob or certificate parsing for `gesture`, and no member that scores or classes a gesture (CON-1, CON-2). | Inspection |

## Dependencies

- **Upstream**: [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md),
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
  [FR-040](./FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md).
- **Downstream**: FR-086 reports this evidence in the approval results.
