---
id: SR-20261115040623
title: "code-review of the risk management presentation in the release decision (engineering-assurance PR 300)"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance@feb71ac91; src/release_decision_host.rs; tests/release_decision_cli.rs; engineering_assurance/skills/assurance-onboarding/workflows/release-decision/def.yaml"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-048
    type: references
---

# Code review of the risk management presentation in the release decision (engineering-assurance PR 300)

## Summary

Ticket: EA-140 (part 2, FR-048). This review covers `git diff origin/main...HEAD` at head feb71ac91, with the rust-review lane and rust-style folded in. The branch already contains origin/main 6ea599215 (EA-134). No other open PR touches `src/release_decision_host.rs`, so there is no overlap with the hardening-r2 lane to resolve.

The change adds three optional present-only inputs: `risk_summary`, `overall_evaluation` and `risk_review`. They pass the existing absolute-path check and the present-only check. Each file goes through `read_text`: regular file only, at most 1 MiB, UTF-8. They are added under a new `risk` member and are counted in the 2 MiB printed-presentation bound, which is checked before any ix-flow mutation. Hostile text ends up as a JSON string, so it cannot inject structure. `decide` and `release_outcome` never read `presentation.risk`. The gate outcome therefore cannot depend on these files. The tests confirm this (see the gap-analysis artifact).

The new production code has no `unwrap`, `expect`, indexing or slicing. `quoin risk-summary --format json` exists on quoin origin/main 681af611 (`rust/crates/quoin-cli/src/risk.rs`, `--format markdown|json`). It prints an object with a `rows` list (FR-169, the snapshot at `quoin-risk-advice/tests/snapshots/risk_summary.json`), so the input is not a phantom. `overall_residual_risk_evaluation` is declared on spec-objects-safety main. `risk_management_review` is declared only on spec-objects-safety#51, which is still open, so the stated merge order (#51 first) matters.

The `def.yaml` `required` list is consistent with its other consumers. The Rust `release_presentation_ready` invariant, the JS `invariants.js` invariant and `tests/workflow_invariants_parity.rs` read only `id` and `baseline`.

Gates: `cargo test --features full --test release_decision_cli` and `cargo clippy --features full --test release_decision_cli --bin engineering-assurance -- -D warnings` were run through `locked-build.sh`. Tests: 26 passed, 0 failed. Clippy: clean (rc 0).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-048-AC-19 oracle for "the presentation names no verdict of its own" is a substring denylist (`acceptable`, `unacceptable`, `risk_accepted`) over the whole serialized presentation. A host-added verdict member such as `"acceptability": "accepted"`, `"risk_ok": false` or `"verdict": "not_accepted"` contains none of those strings, so the test passes it. Reproduced: a temporary mutation adding `"acceptability": "accepted"` to `presentation.risk` left all three risk tests passing. Assert instead the exact key set of `presentation.risk` (`summary`, `overall_evaluation`, `review`) and of each child (`state`, `source`, plus `document` or `text`). | tests/release_decision_cli.rs:1262-1265 |
| FND-002 | low | `frontmatter_type` is a hand-rolled line scan that duplicates `engineering_assurance::release::frontmatter` (the same `release` module is already imported) and the YAML-parsed `onboarding::frontmatter_type`. It diverges from both. An unclosed envelope (`---\ntype: overall_residual_risk_evaluation\n` with no closing `---`, so `take_while` runs into the body) is accepted. Mismatched quotes (`'x"`) are stripped. The first of two `type:` keys wins. Reuse the existing frontmatter split. | src/release_decision_host.rs:547-557 |
| FND-003 | low | `supplied_document` recovers the file name with `path.unwrap_or_default()` inside the `supplied_text` closure. `None` cannot occur there, and if it did the error would name an empty file. Pass the path into the closure, as `supplied` already does for JSON inputs, so the unreachable default goes away (rust-style: prefer pattern matching to check-then-unwrap, no placeholder values). | src/release_decision_host.rs:536-541 |
| FND-004 | low | The user-facing description of `present` in README.md still lists only the evidence status, SOUP report, anomaly list and snapshot as supplied files. The three new risk inputs, and the fact that they are shown and never judged, are undocumented outside FR-048. | README.md:303 |

## Verdict

The changes are correct, and the code meets the ticket's core constraint: supplied risk files cannot change the gate outcome. There are no panics, sizes are bounded and hostile content is escaped. One medium finding: the "no verdict of its own" test oracle is too weak to catch the defect it names. Three low findings. Mergeable after FND-001, provided spec-objects-safety#51 merges first.
