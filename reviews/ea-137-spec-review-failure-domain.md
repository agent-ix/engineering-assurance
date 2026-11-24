---
id: SR-20261124025463
title: "Failure-domain review of EA-137 tag receipts and tag-deletion detection"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/engineering-assurance; PR #304 (git diff origin/main...HEAD at d5b0d7dd5): spec/functional/FR-043, FR-077, FR-081, FR-082, FR-083"
review_set: subset
---

## Summary

Ticket: EA-137. Examined the failure modes of the tag receipt flow: where push times come
from, what happens to tags that existed before adoption, how a standing finding is ever
cleared, and whether `complete` can be true while nothing was receipted. One high and two
medium findings.

Measured for FND-001 (2026-11-24, `gh api repos/<repo>/activity`): the GitHub repository
activity API lists 100 entries for agent-ix/engineering-assurance spanning
2026-11-15T06:57Z to 2026-11-24T02:33Z, all `refs/heads/*`; tag `v0.10.1`, created at
2026-11-15T14:16Z inside that window, has no entry, and `?ref=refs/tags/v0.10.1` returns
0 entries. agent-ix/quoin's latest 100 entries also contain no `refs/tags/*` ref. So on
GitHub "the platform's record of ref pushes" holds no push time for a tag.

## Verdict

FAIL. FR-081's own push time comes from the trigger context, which works; the
detection side (FR-082 `tag_no_receipt`) has no push-time source on GitHub.
Examined and clean: FR-082-AC-1 to AC-5 (comparison logic is total and pure),
FR-081-AC-5 (identical re-push adds nothing), FR-081-AC-6 (notes merge and retry follow
FR-076), FR-043-AC-20 to AC-23.

## Findings

| ID      | Severity | Summary | Refs | Escape Cause |
| ------- | -------- | ------- | ---- | ------------ |
| FND-001 | high | `tag_no_receipt` ages a tag from "the platform's record of ref pushes", but GitHub's activity record has no tag pushes (measured). Every current tag with a policy prefix and no receipt therefore has no push time: the `adopted_at` cutoff and `grace_minutes` cannot be applied, every pre-adoption tag (for example every existing `v*` release tag) is listed `status_unknown`, a tag whose receipt run is still running is listed at once, and by FR-082-AC-7 the scheduled workflow fails permanently from the day of adoption. FR-082-AC-6's "pushed after `adopted_at` and older than `grace_minutes` ... listed as `tag_no_receipt`" cannot be produced on GitHub. `tag_no_receipt` "with reason `status_unknown`" is also the only place a `tag_no_receipt` has a reason, and the shape of tag findings in the `receipt-missing` output (members, and how a tag `invalid_receipt` differs from a commit `invalid_receipt` of FR-077) is not defined. Needs a cutoff that does not depend on a tag push record (for example a backfill run on adoption, or listing unknown-time tags without failing the workflow), chosen by the author. | FR-082-AC-6, FR-082-AC-7, FR-082 Behavior, FR-081 | wrong-requirement |
| FND-002 | medium | A legitimate re-sign or deletion of a receipted tag leaves a permanent `missing_tag` finding with no way to clear it: receipts are never removed (FR-081), the notes refs are protected from rewrite (FR-083), and the commands never mark a finding resolved (FR-082-AC-9). The scheduled workflow then fails on every run from the first such event, so it can no longer signal a later deletion. People accept risk, so the tool needs a defined way for a person to record acceptance of a specific `(ref, tag_object)` finding (for example a reviewed list in the receipt policy on the protected branch), or the workflow must fail only on findings not yet accepted. | FR-082-AC-7, FR-082-AC-2, FR-082-AC-9, FR-043-AC-21, FR-083 | missing-requirement |
| FND-003 | medium | `tag_receipts.checked` is true whenever the request carries the member and the notes ref "was read", and `complete` ignores entries with `tag_receipt` `none`. A caller that passes `tag_receipts` for a repository that never wrote a receipt (no policy member, or a broken workflow) gets `complete` true with zero receipt coverage, while a caller that omits it always gets false. Decision (3) therefore does not achieve its purpose. Define `checked` against the repository's `tag_receipts` policy and say whether a present tag pushed after `tag_receipts.adopted_at` with `tag_receipt` `none` keeps `complete` false; also define `checked` when the notes ref does not exist. | FR-043-AC-24, FR-043 Outputs, FR-043-AC-21 | wrong-requirement |

## New findings (disposition pass 1)

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-004 | medium | An `accepted_tags` entry with `tag_object` null matches a `tag_no_receipt` finding "on a ref that has no receipt at all", whatever the ref's current object. Once a person accepts an existing unreceipted tag that way, the tag can later be moved or re-created at any object (still with no receipt) and it stays `accepted`: the move is never reported. A `tag_no_receipt` finding always has a current object id, so null is not needed there; restrict null to `tag_absent` and require the current object id for `tag_no_receipt`. | FR-082-AC-10, FR-082 Description, FR-081 (accepted_tags) |

## Dispositions

Round 1, fix commit 881b7852f (verified against `git diff d5b0d7dd5 881b7852f`).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Tag push time, `adopted_at` and `grace_minutes` for tags are deleted; `tag_no_receipt` is listed for every prefix-matching tag without a verified receipt for its current object; `status_unknown` is gone for tags; every `receipt-missing` finding carries `kind`, and the tag finding shape is listed (FR-082 AC-6, AC-7, AC-11). Pre-adoption tags are handled by `accepted_tags`. |
| FND-002 | fixed | `accepted_tags` {ref, tag_object or null, accepted_by, reason} on the protected base branch lets a person accept a specific finding; a matched finding is `accepted` and does not fail the workflow or `complete`; `invalid_receipt` cannot be accepted (FR-082-AC-10, FR-043-AC-29). This is people accepting risk, not a tool verdict, and is a real feature. |
| FND-003 | fixed | FR-043 takes `receipt_policy`; an absent notes ref is an empty set with `checked` true; a prefix-matching present tag with `tag_receipt` `none` makes `complete` false unless accepted (FR-043-AC-28). |
