---
id: SR-144
title: "Failure-domain review of engineering-assurance PR 214 (FR-041 trust and identity)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/engineering-assurance@b957ab8; spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md"
review_set: subset
---

# Failure-domain review of engineering-assurance PR 214

## Summary

Ticket: PLAT-1509. Examined FR-041's identity and trust claims and its trailer edge cases
against real git 2.34.1 behaviour with fictional keys. The signature-to-record binding
is real: altering the digest in a signed tag makes the signature fail. The weak point is
what the result says about *who* approved. A tag signed by a fictional "mallory" key and
checked against a caller-written allowed-signers file that maps that key to
`alice@example.test` gives `git verify-tag` exit 0 and `find-principals` prints
`alice@example.test`. That is a genuine `approved` result with `approver`
`alice@example.test`. The result does not echo the trust input it was checked against,
and for a tag FR-036 reports no key fingerprint. So a consumer that keeps the result
cannot tell which trust root named the approver, and cannot re-verify it.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-041 calls the result "a named person's approval", but `approver` is only the name a caller-chosen allowed-signers file (or, for gitsign, the caller-declared `gitsign.principal`) gives the key, and the result keeps neither the trust input nor a key identity. A consumer cannot tell which trust root named the approver, and cannot re-verify the approval. Fix: the result echoes its trust input (the `allowed_signers` path, plus `gitsign.certificate_identity`/`certificate_oidc_issuer` for gitsign), and the Description says `approver` is the principal that trust input names, not an established identity. | FR-041 Outputs; FR-041 Description | wrong-requirement |
| FND-002 | low | FR-041 does not say whether trailer keys match case-insensitively. git trailer keys are case-insensitive, and EA's FR-037 collector uses `eq_ignore_ascii_case`. Measured: `approved-record-id: rec-1` plus `APPROVED-RECORD-ID: rec-9` parse as two trailers, which is `repeated` under case-insensitive matching and `missing` under exact matching. Fix: state that keys match ASCII case-insensitively, as in FR-037, so every case variant counts toward `repeated`, and add the case to AC-3. | FR-041 Behavior (trailer read); FR-041-AC-3 | missing-requirement |
| FND-003 | low | A tag message with no subject paragraph yields no trailers at all. Measured: a message made only of the three trailer lines parses to nothing, because git takes its only paragraph as the subject. Such an approval reports `wrong_record` with both bindings `missing`, with no hint why. Fix: an AC or note fixing that the trailers must follow a subject paragraph, and that a trailers-only message reports `missing`. | FR-041 Behavior (trailer read) | missing-requirement |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 549939c: result echoes trust input; Description says approver is the principal the trust input names, not an established identity; AC-14 |
| FND-002 | fixed | 549939c: case-insensitive key rule; AC-3 covers case-variant repeats |
| FND-003 | fixed | 549939c: subject-paragraph rule and AC-13 |
