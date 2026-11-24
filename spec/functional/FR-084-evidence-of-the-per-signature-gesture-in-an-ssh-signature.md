---
id: FR-084
title: "Report the per-signature gesture that an SSH signature evidences"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-036"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-037"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-040"
    type: "references"
---

# FR-084: Report the per-signature gesture that an SSH signature evidences

## Description

Tracked by Linear EA-113. A signature made by a key that any process with an unlocked
agent can use proves a key was used, not that a person was present. A FIDO security-key
SSH key (`ed25519-sk` or `ecdsa-sk`) differs: the key never leaves the device, and the
device records in each signature a flags byte, with bit 0x01 set when the user touched the
device (user presence) and bit 0x04 set when the user also verified, for example with a
PIN (user verification). `ssh-keygen -Y verify` does not enforce either bit; at debug
level `-vv` it prints them on standard error as `signature details: counter = N, flags =
0xNN`. A key created with `-O no-touch-required` signs with the 0x01 bit clear, and the
result for such a key is `none`, which states that no touch is shown.

For an accepted SSH signature on a tag, the `signature-verify` command
([FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md)) reports which gesture the
signature's own flags show, so that a reviewer can see whether the signature needed a
person at a device. The report is evidence from the signature alone. It does not say
whether that gesture suffices for any rule: Engineering Assurance assigns no score, class
or acceptability, and the person who accepts the risk decides.

No existing `ssh-keygen` run of FR-036 or FR-037 prints these flags, and FR-036 forbids
classifying from free-form message text. This requirement is the one scoped exception: it
adds one `ssh-keygen -Y verify -vv` run on the exact SSH signature block of the tag,
extracted from the tag object as a byte range with `git cat-file`, never interpreted. The
flags line is read only from that run's standard error.

Two other per-signature gestures leave no trace in a signature: `ssh-add -c` (the agent
asks for confirmation on each use) and an interactive `gitsign` OIDC login. They are
reported as `none` and never as a gesture. A caller may declare such a mode when it
creates an approval tag (FR-085), and FR-086 reports the declaration as a claim.

Out of scope: creating or configuring a signing key, the declared mode (FR-085, FR-086),
the OpenPGP signature kind (FR-036 does not verify it), a signed commit (the flags are
not read for a commit, and its result is `none`), and a customer-integrated signer reached
through a seam, which reports its own gesture.

## Inputs

- The request, the allowed-signers file, the namespace and the principal that FR-036 and
  FR-037 already use. No new request member.

## Outputs

- On a `signed_valid` verdict, one added member `gesture`, an object with exactly one
  member `evidenced`, whose value is exactly one of:
  - `device_touch_and_pin`: the flags line shows both the 0x01 and the 0x04 bits set;
  - `device_touch`: the flags line shows the 0x01 bit set and the 0x04 bit clear;
  - `none`: the flags line is absent or shows the 0x01 bit clear, the signature is by a
    key type that is not a security-key type, the signature is a gitsign signature, or
    the verdict is for a commit.
- No other status carries `gesture`: a signature that is not accepted evidences nothing.
- The protocol value `engineering-assurance.signature-verify-result/v1` and every
  existing member are unchanged.

## Behavior

- The command SHALL, for a `signed_valid` SSH tag, extract the armored SSH signature block
  from the tag object read with `git cat-file tag`, byte for byte, without interpreting it.
- The command SHALL make one `ssh-keygen -Y verify -vv` run on that block, with the
  allowed-signers file, the principal and the namespace of the verification FR-036 and
  FR-037 already made, under the process bounds of FR-019.
- The command SHALL take the flags only from the `signature details: counter = N, flags =
  0xNN` line on that run's standard error.
- The command SHALL report `none` when that run prints no flags line, when its exit status
  is nonzero, or when it cannot start.
- The command SHALL report `none` for a gitsign signature and for a commit.
- The command SHALL omit `gesture` for `signed_untrusted`, `signed_bad`, `unsigned` and
  `unverifiable`.
- The command SHALL NOT read the tag message text or any trailer to decide `evidenced`.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-084-CON-1 | The command SHALL contain no SSH signature, key-blob or certificate parsing; the armored block is passed to `ssh-keygen` as extracted bytes and every fact in `gesture` comes from `ssh-keygen` output. | Architecture | Inspection |
| FR-084-CON-2 | The command SHALL add no member that scores or classes a gesture, or states a verdict on it. | Safety | Inspection |

## Acceptance Criteria

Stand-in note: a stand-in `ssh-keygen` on `PATH` prints the `-vv` standard-error lines of
the real one, including `signature details: counter = 3, flags = 0x05`, for a
security-key signature. The real OpenSSH output on a software key, measured with 8.9p1,
carries no such line. Where no FIDO device is present, only the stand-in criteria run.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-084-AC-1 | A tag signed with a throwaway software ed25519 key and accepted reports `signed_valid` with `gesture` equal to `{"evidenced":"none"}`, from the real `ssh-keygen`. | Test |
| FR-084-AC-2 | With the stand-in printing `flags = 0x05`, an accepted tag signature reports `device_touch_and_pin`. | Test (stand-in) |
| FR-084-AC-3 | With the stand-in printing `flags = 0x01`, an accepted tag signature reports `device_touch`, and the verdict status stays `signed_valid`. | Test (stand-in) |
| FR-084-AC-4 | With the stand-in printing `flags = 0x00`, an accepted tag signature reports `none`; with the stand-in printing no flags line, or exiting nonzero, it reports `none`. | Test (stand-in) |
| FR-084-AC-5 | A gitsign tag signature accepted through the FR-040 stand-in, and a signed commit accepted with a security-key stand-in, each report `gesture` equal to `{"evidenced":"none"}`. | Test (stand-in) |
| FR-084-AC-6 | A `signed_untrusted`, `signed_bad`, `unsigned` and `unverifiable` verdict each carry no `gesture` member. | Test |
| FR-084-AC-7 | A tag signed by a software key whose message carries a `Signing-Gesture: device_touch` trailer reports `gesture` equal to `{"evidenced":"none"}`. | Test |
| FR-084-AC-8 | With a real FIDO security key present, a tag signed with the key touched reports `device_touch`, one signed with a key created with `-O verify-required` and the PIN given reports `device_touch_and_pin`, and one signed with a key created with `-O no-touch-required` reports `none`; where no device is present the test is reported not executed and is not counted as passed. | Test (hardware) |
| FR-084-AC-9 | Every pre-existing member and the protocol value of a `signed_valid` result are equal to the result without `gesture`. | Test |
| FR-084-AC-10 | The signature block passed to the stand-in `ssh-keygen` equals, byte for byte, the armored block of the tag object read with `git cat-file tag`, and inspection shows no signature-block, key-blob or certificate parsing and no member that scores or classes a gesture (CON-1, CON-2). | Test |

## Dependencies

- **Upstream**: [FR-036](./FR-036-ssh-signed-commit-and-tag-verification.md),
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
  [FR-019](./FR-019-bounded-producer-execution.md),
  [FR-040](./FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md).
- **Downstream**: FR-086 reports this evidence in the approval results.
