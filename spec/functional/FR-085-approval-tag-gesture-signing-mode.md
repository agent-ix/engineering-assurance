---
id: FR-085
title: "Create an approval tag in an interactive signing mode that declares its gesture"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-037"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-042"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-084"
    type: "references"
---

# FR-085: Create an approval tag in an interactive signing mode that declares its gesture

## Description

Tracked by Linear EA-113. The `approval-tag` command ([FR-042](./FR-042-create-signed-approval-tag-for-a-change-assurance-record.md))
runs git with no terminal and a 60 second limit, so a signer that needs a person (a
security-key PIN prompt, an `ssh-add -c` confirmation, a `gitsign` browser or
out-of-band login) cannot finish. This requirement adds one optional request member,
`gesture`, that selects an interactive mode and records the mode the caller says it used.

The three values name the per-signature gestures of the baseline: `device_touch` (a FIDO
security-key SSH key), `agent_confirm` (a key held by an agent that asks for
confirmation on each use, `ssh-add -c`) and `oidc_interactive` (a `gitsign` login a
person completes). The value is the caller's declaration. The command checks none of
them and signs nothing itself: git and the signer the caller configured still do. FR-084
reports what the signature evidences, and FR-086 reports the declaration beside it.

An interactive run does not prove a person is present: a process that starts the command
can also answer a prompt or an askpass program. What the mode gives is that a signer
which asks a person has the stream and the time to ask.

Out of scope: choosing a key, configuring `ssh-add`, `ssh-agent`, an askpass program or
`gitsign`, deciding whether a gesture suffices, and verifying the tag (FR-041, FR-086).

## Inputs

- The FR-042 request, plus the optional member `gesture`, exactly one of
  `device_touch`, `agent_confirm`, `oidc_interactive`. When it is absent the command
  behaves exactly as FR-042 defines.

## Outputs

- With `gesture`, the tag message holds a fourth trailer `Signing-Gesture` after the three
  FR-042 trailers, whose value is the requested value, and the `created` line holds a
  member `gesture` equal to it. Without `gesture`, the message holds the three trailers
  and the line holds no `gesture` member.
- A `gesture` that is not one of the three values is refused with
  `approval_tag_request_invalid` and exit status 2, without starting git.

## Behavior

- The command SHALL, when `gesture` is present, give git the command's own standard input
  and standard error, so that a signer's prompt reaches the caller's terminal and the
  caller's answer reaches the signer.
- The command SHALL, when `gesture` is present, keep git's standard output captured.
- The command SHALL write nothing but the one result line to its own standard output.
- The command SHALL, when `gesture` is present, run git with a 300 second wall-clock
  timeout in place of the 60 second timeout of FR-042.
- The command SHALL, when `gesture` is absent, give git no terminal and keep the 60 second
  timeout.
- The command SHALL pass the same environment to git as FR-042 does, with the same
  variables removed, so `SSH_AUTH_SOCK`, `SSH_ASKPASS`, `SSH_ASKPASS_REQUIRE` and
  `DISPLAY` reach a signer.
- The command SHALL compose the message, when `gesture` is present, as a pure function
  with the trailers in the order `Approved-Record-Id`, `Approved-Record-Digest`,
  `Signed-off-meaning`, `Signing-Gesture`.
- The command SHALL report `approval_tag_create_failed` and leave no tag when the signer
  denies the gesture, exits nonzero or the timeout passes, as FR-042 does for a failed
  signer.
- The command SHALL NOT verify, infer or contradict the declared `gesture`.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-085-CON-1 | The command SHALL NOT open a terminal, prompt, read a PIN or run an askpass program itself. | Safety | Inspection |
| FR-085-CON-2 | The command SHALL still sign only through `git tag -s` and add no cryptography, as FR-042-CON-1 and FR-042-CON-2 require. | Architecture | Inspection |

## Acceptance Criteria

Stand-in note: a stand-in signing program configured as `gpg.x509.program` writes a
prompt to standard error, reads one line from standard input and emits a CMS-shaped
block; the `ssh-add -c` criterion uses a real `ssh-agent` and `ssh-add` started by the
test with a throwaway key and an askpass script that records each call.

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-085-AC-1 | A request with no `gesture` creates a tag whose message holds exactly the three FR-042 trailers, whose result line holds no `gesture` member, and whose git run had no terminal. | Test |
| FR-085-AC-2 | A request with `gesture` `oidc_interactive`, the stand-in signer and a line supplied on standard input creates a tag; the prompt text appears on standard error only; standard output is exactly one `created` line holding `"gesture":"oidc_interactive"`. | Test (stand-in) |
| FR-085-AC-3 | The created tag message holds the trailers `Approved-Record-Id`, `Approved-Record-Digest`, `Signed-off-meaning`, `Signing-Gesture` in that order, and the FR-037 trailer reader returns the declared value for `Signing-Gesture`. | Test |
| FR-085-AC-4 | A `gesture` of `touch`, an empty string, a wrong case or a non-string is refused with `approval_tag_request_invalid`, exit status 2 and and git not started. | Test |
| FR-085-AC-5 | With a key added by `ssh-add -c` to a test `ssh-agent` whose askpass script approves, two requests with `gesture` `agent_confirm` create two tags and the askpass script records exactly two calls, one per signature. | Test |
| FR-085-AC-6 | With the same agent and an askpass script that denies, the request yields `approval_tag_create_failed` at exit status 2 with the tag ref absent and no other ref changed. | Test |
| FR-085-AC-7 | A stand-in signer that does not answer is bounded by a 300 second timeout when `gesture` is present and by 60 seconds when it is absent, read from the timeout argument given to git when it is started. | Test |
| FR-085-AC-8 | The declared value is copied verbatim whichever signer ran: a request declaring `device_touch` with a software key creates the tag and the command reports no error. | Test |
| FR-085-AC-9 | Static inspection shows no terminal, prompt, PIN or askpass code in the approval-tag modules (CON-1, CON-2). | Inspection |

## Dependencies

- **Upstream**: [FR-042](./FR-042-create-signed-approval-tag-for-a-change-assurance-record.md),
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
  [FR-084](./FR-084-evidence-of-the-per-signature-gesture-in-an-ssh-signature.md).
- **Downstream**: FR-086 reports the declaration.
