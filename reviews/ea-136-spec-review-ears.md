---
id: SR-20261124025461
title: "spec-review (ears-conformance) of trusted signing time and reproducible trust identity (engineering-assurance PR 303)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/engineering-assurance@fe432b49 (PR 303, diff origin/main...HEAD); new or edited SHALL statements in spec/functional/FR-036, FR-037, FR-040, FR-041, FR-047"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-041
    type: references
---

# EARS review of EA PR 303

## Summary

Ticket: EA-136, EA-138. This lens examined each added or edited requirement statement for EARS form: a clear trigger or state, one responsible system and one response. Clean: FR-036 Behavior (verify against a private file), FR-037 temp-file statement, FR-040-CON-2, FR-041-CON-5, FR-041 "take a trusted time only for", the trusted-root statement, the no-log-field statement, FR-047 "report `trust` for every verdict", FR-037-CON-3, and FR-047-CON-3.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | "If the library offers no public way to verify an embedded entry, every gitsign result is `trusted_at_missing` `log_entry_unverified`" uses a design-time fact, not a runtime trigger or state, as its condition. No test can observe it, and it leaves the requirement with two contradictory behaviours. Resolve the fact and state one behaviour (see SR-20261124025459 FND-001). | spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:256-258 |
| FND-002 | low | "The allowed-signers reading yields `allowed_signers_sha256` ... and ... `allowed_signers_commit`" is a new obligation with no SHALL, unlike its neighbours. Rewrite as "The reading SHALL yield ...". | spec/functional/FR-037-signer-mapping-and-signed-off-meaning.md:91-95 |

## Verdict

Mostly conformant. FND-001 is the EARS face of the base review's feasibility finding, and is fixed together with it.

## New findings (disposition pass 1)

Reviewed fix commit 288c440e against fe432b49. `quire validate` (quire 0.52.0) exits 0 with grammar warnings on the new text, confirmed at FR-040:307 and FR-041:267, 274, 282 and 306.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | The new statements carry quire grammar warnings. FR-040-CON-2 (line 307) and FR-041-CON-2 (line 306) mix SHALL with a "may" exception. FR-041:267 "The CMS/DER reader SHALL be used only to ..." is an agentless passive. FR-041:274 "On the gitsign and transparency-log path ..." uses a non-EARS trigger. FR-041:282 "The compiled-in root is offline; the command SHALL NOT fetch a root ..." is unclassifiable and has no subject. Each meaning is unambiguous and no AC becomes untestable, so this is low. Possible rewrites: "The command SHALL use the CMS/DER reader only to ...", and "While on the gitsign path ...". | spec/functional/FR-040-gitsign-sigstore-keyless-commit-and-tag-verification.md:307; spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:267,274,282,306 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The design-time conditional is replaced by runtime behaviour: "When the tag carries no embedded entry the result is `trusted_at_missing` `no_log_entry`; when the bundle cannot be built from the tag or the library does not accept it, the result is `trusted_at_missing` `log_entry_unverified`". |
| FND-002 | fixed | It now reads "The allowed-signers reading SHALL yield `allowed_signers_sha256` ...". |
