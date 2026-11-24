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
runs git through the FR-019 process host with standard input closed and a 60 second
limit. Without `gesture`, git runs in a process group other than the caller's foreground
process group when a controlling terminal exists; this does not promise a detached session
or the absence of a controlling terminal. A signer that reads `/dev/tty` while its group
is in the background can be stopped by `SIGTTIN`. This requirement adds one optional
request member, `gesture`, that selects an interactive mode and records the mode the
caller says it used.

The three values name the per-signature gestures of the baseline: `device_touch` (a FIDO
security-key SSH key), `agent_confirm` (a key held by an agent that asks for
confirmation on each use, `ssh-add -c`) and `oidc_interactive` (a `gitsign` login a
person completes). The value is the caller's declaration. The command checks none of
them and signs nothing itself: git and the signer the caller configured still do. FR-084
reports what the signature evidences, and FR-086 reports the declaration beside it.

An interactive run does not prove a person is present: a process that starts the command
can also answer a prompt or an askpass program. What the mode gives is that a signer
which asks a person has the channel and the time to ask. The channels are the ones the
signers use: a FIDO PIN prompt of `ssh-keygen` reads `/dev/tty` or runs `SSH_ASKPASS`
(with `SSH_ASKPASS_REQUIRE` and `DISPLAY`); an `ssh-add -c` confirmation is asked by the
agent through its own askpass; `gitsign` opens a browser or prints a URL and a code on
standard error. The command's standard input is not a channel: with `--request -` the
request has already consumed it.

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

- The command SHALL, when `gesture` is present, ask the FR-019 process host to run git
  in the command's own session, so that `/dev/tty` opens in git and in the signer it
  starts.
- The command SHALL, when `gesture` is present, ask the process host to make git's
  invocation-owned process group the foreground process group of the controlling terminal
  for the run (`tcsetpgrp`), because a background group that reads `/dev/tty` is stopped
  by `SIGTTIN`.
- The process host SHALL hold git before exec until the foreground handoff succeeds, so
  git and its signer cannot race the handoff with a terminal read.
- If the controlling terminal cannot be opened, its foreground group cannot be read, or
  foreground acquisition fails, the process host SHALL terminate git before exec and
  report the run as failed; it SHALL NOT continue signing in the background.
- The command SHALL, when `gesture` is present, ask the process host to restore the
  previous foreground process group after the run, whether git exits, fails or times out.
  If restoration fails, the process host SHALL report the run as failed and retry the
  restoration during cleanup; approval-tag SHALL report `approval_tag_create_failed` and
  remove any tag ref written by that run.
- The command SHALL, when `gesture` is present, ask the process host to pass git's
  standard error to the command's own standard error and to leave the environment
  variables `SSH_ASKPASS`, `SSH_ASKPASS_REQUIRE`, `DISPLAY` and `SSH_AUTH_SOCK` in place.
- The command SHALL, when `gesture` is present, keep git's standard input closed and git's
  standard output captured, as the process host does for every other request.
- The command SHALL accept `gesture` with `--request -` as with `--request <json>`.
- The command SHALL write nothing but the one result line to its own standard output.
- The command SHALL, when `gesture` is present, give the process host a 300 second
  wall-clock deadline in place of the 60 second deadline of FR-042.
- The command SHALL, when `gesture` is absent, ask the process host to run git in its
  invocation-owned process group and give it the 60 second deadline of FR-042.
- When `gesture` is absent and the caller has a controlling terminal, git's process group
  SHALL differ from the terminal's foreground process group. Git's standard input remains
  closed as required by FR-019.
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
| FR-085-AC-1 | A request with no `gesture` creates a tag whose message holds exactly the three FR-042 trailers and whose result line holds no `gesture` member. Under a pseudo-terminal, git's process group differs from the terminal's foreground process group. | Test (pty, stand-in) |
| FR-085-AC-2 | A request with `gesture` `oidc_interactive`, passed with `--request -`, run under a pseudo-terminal with a stand-in signer that opens `/dev/tty`, writes a prompt there and reads a line from it, creates a tag; the signer was not stopped by `SIGTTIN` and the line was read from the pseudo-terminal and not from standard input, while the terminal's foreground process group during the run was git's process group; standard output is exactly one `created` line holding `"gesture":"oidc_interactive"`. | Test (pty, stand-in) |
| FR-085-AC-3 | The created tag message holds the trailers `Approved-Record-Id`, `Approved-Record-Digest`, `Signed-off-meaning`, `Signing-Gesture` in that order, and the FR-037 trailer reader returns the declared value for `Signing-Gesture`. | Test |
| FR-085-AC-4 | A `gesture` of `touch`, an empty string, a wrong case or a non-string is refused with `approval_tag_request_invalid`, exit status 2 and git not started. | Test |
| FR-085-AC-5 | With a key added by `ssh-add -c` to a test `ssh-agent` whose askpass script approves, two requests with `gesture` `agent_confirm` create two tags and the askpass script records exactly two calls, one per signature. | Test |
| FR-085-AC-6 | With the same agent and an askpass script that denies, the request yields `approval_tag_create_failed` at exit status 2 with the tag ref absent and no other ref changed. | Test |
| FR-085-AC-7 | A stand-in signer that does not answer is bounded by a 300 second deadline when `gesture` is present and by 60 seconds when it is absent, read from the deadline passed to the host that runs git. | Test |
| FR-085-AC-8 | The declared value is copied verbatim whichever signer ran: a request declaring `device_touch` with a software key creates the tag and the command reports no error. | Test |
| FR-085-AC-10 | With `gesture` present, the environment of the stand-in signer holds the `SSH_ASKPASS`, `SSH_ASKPASS_REQUIRE`, `DISPLAY` and `SSH_AUTH_SOCK` values of the command's environment, its standard error is the command's standard error, and its standard input is closed. | Test (stand-in) |
| FR-085-AC-11 | With `gesture` present, under a pseudo-terminal, after a run that completes, after one whose stand-in signer exits nonzero and after one that reaches its deadline, the terminal's foreground process group equals the one before the run, and git's process group no longer exists. | Test (pty, stand-in) |
| FR-085-AC-12 | If the controlling terminal cannot be acquired, the process host fails before executing git or its signer. | Test (process host) |
| FR-085-AC-13 | If foreground restoration fails after signing, the process host retries restoration and reports the run as failed; the command reports `approval_tag_create_failed` and removes the tag ref written by that run. | Test (process host, pty) |
| FR-085-AC-9 | Static inspection shows no terminal, prompt, PIN or askpass code in the approval-tag modules (CON-1, CON-2). | Inspection |

## Dependencies

- **Upstream**: [FR-042](./FR-042-create-signed-approval-tag-for-a-change-assurance-record.md),
  [FR-037](./FR-037-signer-mapping-and-signed-off-meaning.md),
  [FR-084](./FR-084-evidence-of-the-per-signature-gesture-in-an-ssh-signature.md).
- **Downstream**: FR-086 reports the declaration.
