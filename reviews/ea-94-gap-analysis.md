---
id: SR-20261113144002
title: "gap-analysis of the human release-decision workflow (engineering-assurance PR 230)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@d67d1c9; FR-048, FR-005, FR-047, FR-049, FR-053, NFR-007; spec/tests.md TC-450..TC-461; tests/release_decision_cli.rs; src/release_decision_host.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-048
    type: references
---

# Gap analysis of the human release-decision workflow (engineering-assurance PR 230)

## Summary

Ticket: EA-94. Plan completion: not assessed. `quoin matrix --repo .` shows FR-048-AC-1..AC-12 all `tagged`, each by exactly one test, TC-450..TC-461 in tests/release_decision_cli.rs, and every binding is to the matching AC. Two mutations were killed: TC-453 caught the removed anomaly refusal, and TC-450/TC-455 caught an `auto` terminal gate. The ticket's own criteria ("records the release decision by creating a signed release tag", "Decision record is written") contradict the merged FR-048, which forbids the workflow from creating a tag or a commit. The merged spec wins and the code follows it.

The input member names were checked against quoin origin/main sources. The evidence status is `requirements[].criteria[].{id,evidence_status}` with the closed set of quoin-assurance `matrix.rs`. The anomaly report is `anomalies[].{id,status}`, and FR-162's "carried" set makes the rule that every supplied anomaly must be accepted faithful. The SOUP report rows are `rows[].{purl,id}` (quoin-cli `soup/report/render.rs`). The member names are right. What is presented is the problem, as follows.

FR-005 reuse holds, and so do FR-049 (files only, Quoin not invoked) and FR-053 (no attestation read). NFR-007 holds for the release-decision command. The generic-host path is recorded in the code-review artifact.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The SOUP presentation drops every member a reviewer decides on. Per row only `purl`/`id` is kept, which loses `unresolved_advisory`, `advisories`, `past_end_of_support`, `support_status`, `policy_status` and `known_anomalies`. The report-level `unmatched`, `scans` (not run / vacuous) and `unreadable_scans` are dropped too. Rows are not joined to the Baseline `soup_record` paths, although each FR-144 row carries `path`. A report with grype not run therefore reads like clean rows, against FR-048's "never as none or clean". FR-048's "their report entries" is ambiguous; it should name the presented members or present rows verbatim. | src/release_decision_host.rs:374-384 |
| FND-002 | medium | Incompleteness signals in the supplied Quoin outputs are discarded. A FR-162 anomaly report with a non-empty `unreadable` (quoin exits non-zero, so anomalies are missing) is presented as a complete list, and `release` then needs no acceptance of the unread anomalies. A FR-115 matrix with `reason` (no criteria) is presented as supplied with zero criteria, which looks the same as an empty pass. FR-048 should state a refusal (`release_input_unreadable`) or the presentation of these members. | src/release_decision_host.rs:385-401; src/release_decision_host.rs:482-499 |
| FND-003 | low | `record_path` is not checked against the baseline commit, and `release` is allowed while Baseline items are `absent`. Either way the emitted record can never verify (`release_commit_record_not_added`, `baseline_item_absent`). FR-048 is silent on this; it should refuse or state the consequence. | src/release_decision_host.rs:251-259; src/release_decision_host.rs:529-556 |
| FND-004 | low | Coverage edges: TC-458 has no unreadable `snapshot_file` case (for example non-UTF-8), and TC-457 covers resume only from `capture`, not from `evidence_ready` (between `add_item` and the advance to `decision_ready`). | tests/release_decision_cli.rs:528-609 |

## Verdict

All twelve FR-048 ACs are tagged, bound correctly and able to fail. The gaps are in the presentation's faithfulness to the Quoin inputs (FND-001 and FND-002, medium, each needing an FR-048 amendment plus code) and two lows.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified on head 46b9e93:     let soup_report = supplied(request.soup_report.as_deref(), \|source, document\| {         for row in array_member(source, &document, "rows")? {             string_member(source, row, "purl")?;         }         Ok(json!({"document": document}))     })?; |
| FND-002 | fixed | Verified on head 46b9e93:         let unreadable_entries = match document.get("unreadable") {             None => Vec::new(),             Some(entries) => entries                 .as_array()                 .ok_or_else(\|\| unreadable(source, "the unreadable member is not a list"))?                 .iter()                 .map(\|entry\| {                     let path = string_member(source, entry, "path")?;                     Ok(json!({"path": path, "code": entry.get("code")}))                 })                 .collect::<Result<Vec<_>, ReleaseDecisionError>>()?,         };         Ok(json!({"items": items, "unreadable": |
| FND-003 | fixed | Verified on head 46b9e93:     let absent: Vec<String> = presentation["baseline"]["items"]         .as_array()         .into_iter()         .flatten()         .filter(\|item\| item["present"] == false)         .filter_map(\|item\| item["path"].as_str().map(str::to_owned))         .collect();     if !absent.is_empty() {         return Err(ReleaseDecisionError::BaselineItemAbsent { paths: absent });     }     let stored = run         .items("release_request")?         .first()         .ok_or(ReleaseDecisionError::PresentationMissing)?;     let release_id = text_at(stored, &["release_id"]);     let record_path = text_at(stored |
| FND-004 | fixed | Verified on head 46b9e93: tc_465_a_fifo_or_non_utf8_input_is_unreadable_and_never_hangs; tc_468_an_interrupted_run_resumes_from_evidence_ready |
