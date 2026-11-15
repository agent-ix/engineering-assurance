---
id: SR-20261114084861
title: "Code review of campaign type metadata alignment (engineering-assurance PR 278)"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance@61c528ec; campaign/contract/spec/functional/EN-001..EN-006, VO-001..VO-012; engineering_assurance/manifest.yaml; FCD extraction-frontend bundle.rs at dd348507"
review_set: subset
---

# Code review of campaign type metadata alignment (engineering-assurance PR 278)

## Summary

Ticket: EA-125. Reviewed frozen head `61c528ec55a0f3784377ee6797bfda145195ec01` against `origin/main` `bfce1e4e618f2cc3f36db745c8c58fc75229b5db`. The 18 source records now use the declared object kind as their `type`; no generated files, implementation code, tests, test tags, or gates changed. The current FCD extraction code and live lift confirm that this changes them from composed requirements into model declarations.

The corrected lift emits 18 types (12 `campaign_value`, 6 `campaign_enum`) at FCD main `dd3485073737e6a892b96042123fe2ad4acfe1e9`. I independently reran `cargo test --locked --features campaign --test campaign` (11/11) and `cargo test --locked --features full --test semantic_exports` (11/11), each through `locked-build.sh`. Rights check is reported accepted at 709 entries / 0 findings. No applicable AssuranceProfile exists under `spec/`. The observed Quire 0.45.0 / engine 0.60.0 validator mismatch is a separate residual: the CLI validates the enumeration schema without the Values table extraction, while FCD successfully extracts the authored values. It does not contradict the FCD object classification in this change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS** — the metadata change matches the module’s declared object types and current FCD classification. The lift and both requested campaign-focused test suites pass; the diff is limited to the 18 source documents.

## Reviewed scope

All 18 changed records were checked for `type`/`object` agreement, stable ids and names, and unchanged model bodies: EN-001 through EN-006 and VO-001 through VO-012. Context checked: `engineering_assurance/manifest.yaml` exports `campaign_value` and `campaign_enum`; FCD extraction-frontend `Bundle::load` treats a document as composed only when its `type` is neither a declared object type nor equal to its `object` field. The seven FCD `DECLARED_LOSS` diagnostics for optional collection presence are unchanged semantics and outside this metadata diff.

## Verification

- FCD live lift on `dd3485073737e6a892b96042123fe2ad4acfe1e9`: pass; IR contains 18 declarations.
- `locked-build.sh ... cargo test --locked --features campaign --test campaign`: 11 passed, 0 failed.
- `locked-build.sh ... cargo test --locked --features full --test semantic_exports`: 11 passed, 0 failed.
- Content-rights: coder reports 709 entries, 0 findings accepted.
- `git diff --check`: pass; PR diff contains only the 18 intended source docs.
- Quire CLI residual independently reproduced with `quire 0.45.0 (engine 0.60.0)`: VO-001 warns about unresolved references under standalone validation; EN-001 exits 3 with `values` required although the `## Values` table exists. This command is not the FCD extraction path and the direct lift emits the values successfully.
