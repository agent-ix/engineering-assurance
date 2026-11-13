# Solo-maintainer controls

This procedure applies when one human maintains a repository. A solo-maintained
repository has one required maintainer role, staffed by that human. The role
policy must not claim that one person fills multiple required roles
([FR-061](../../spec/functional/FR-061-role-policy-and-distinct-individuals.md),
[FR-063](../../spec/functional/FR-063-unstaffed-required-role-fails-visibly.md)).
For higher-risk changes, a distinct external reviewer must review under their
own role in the repository's policy. That reviewer is not the maintainer.

The human maintainer is the only person who approves changes or signs human
decisions. Workflow identities may sign automated statements and receipts only
in signing jobs that run no agent, execute no repository code, and hold no
credential reachable by any agent. Agents may draft and verify work, but they
do not approve changes or sign records. The controls below distinguish
automated enforcement from steps the maintainer performs and records. No
criticality or software-safety class is inferred or assigned automatically;
the human owner assigns those values under the repository's policy.

| Control | Enforcement / procedure | Performer | Interval or deadline | Evidence and command |
| --- | --- | --- | --- | --- |
| Human-only approval and person-signing | The review gate counts human approvals only; bot, app, unknown-type, and policy-listed agent accounts cannot approve ([FR-062](../../spec/functional/FR-062-ai-agents-are-never-approvers.md), [FR-065](../../spec/functional/FR-065-workflow-roles-and-relation-to-signed-release-tags.md)). The human alone signs human decisions; workflow identities sign automated statements and receipts only in jobs that run no agent, execute no repository code, and hold no credential reachable by any agent ([NFR-011](../../spec/non-functional/NFR-011-signing-jobs-run-no-agent.md)). | Human maintainer (Peter Krenesky) approves and signs human decisions; agents may author or verify. | Every change and every signing operation. | Required review-gate result and post-merge receipt (EA-109, EA-108); inspect with `gh pr checks <number>` and `engineering-assurance receipt-missing`. Person-signed records are checked with `git verify-commit <commit>` or `git verify-tag <tag>`. |
| No self-approval; dismiss stale approvals | Main-branch ruleset requires an approval from someone other than the author and dismisses approvals when new commits are pushed (EA-109). | GitHub ruleset enforces; maintainer verifies its configuration. | Every pull request; inspect after ruleset changes. | Ruleset definition and API response: `gh api repos/<owner>/<repo>/rulesets`. |
| Review ruleset bypasses | Procedural review of the repository's ruleset insights/bypass log and missing-receipt report. Explain or revert every bypassed commit. Record the review as a dated, human-signed entry with `Signed-off-meaning: reviewed`, such as a signed commit to a review log. | Human maintainer (Peter Krenesky). | Proposed interval: every 90 calendar days. | Review-log entry and ruleset Insights/bypass log; run `engineering-assurance receipt-missing` and retain its output with the entry. FR-077 defines the missing-receipt check; EA-108 tracks its receipt implementation. |
| Post-hoc review of emergency changes | An emergency change is one merged by ruleset bypass, or merged while the review gate failed, to restore service or fix a security issue. The maintainer obtains and records a human approval for the merged commit after the fact. | Human maintainer (Peter Krenesky) obtains review from a distinct human reviewer. | Proposed deadline: within 5 working days after merge; pending Peter's confirmation. A missed deadline is listed at the next periodic bypass review. | Approval statement or signed review entry that names the merged commit and review date; inspect with `gh pr view <number> --json reviews,commits` and `git verify-commit <commit>`. |
| External review for higher-risk changes | A higher-risk change is proposed here as any change to: (1) signing, attestation, or release-verification code (`src/signature_verify*.rs`, `src/approval_*.rs`, `src/release*.rs`, or attestation and receipt modules); (2) a workflow with `id-token: write`; (3) the ruleset, role policy, or allowed-signers file; or (4) a requirement whose human-assigned criticality or software-safety class is the highest value in use. These triggers are proposals pending Peter's confirmation. The external reviewer is a named person other than the maintainer, listed in the policy under their own role. Until policy can vary per change, the maintainer checks this procedurally at the periodic review. | Human maintainer (Peter Krenesky) identifies items using the proposal and obtains review from a distinct external reviewer. The reviewer approves under their own role. | For each proposed higher-risk change, before merge where feasible; otherwise at the next periodic review. | External reviewer's approval statement for the merged commit; inspect with `gh pr view <number> --json reviews,commits` and `engineering-assurance receipt-missing`. See [FR-061](../../spec/functional/FR-061-role-policy-and-distinct-individuals.md), [FR-065](../../spec/functional/FR-065-workflow-roles-and-relation-to-signed-release-tags.md), and EA-109. |
| One-role policy for a solo repository | Configure one required maintainer role. FR-061 permits one-role policies. Do not make the policy appear to satisfy more roles than there are distinct people. A higher-risk change requires a distinct external reviewer under their own policy role; while policy is not per-change, apply that control procedurally at the periodic review. | Human maintainer (Peter Krenesky) owns and verifies the policy; the approval gate evaluates it. | On repository setup and whenever the role policy changes; check higher-risk exceptions at each 90-day review. | Policy at `.engineering-assurance/approval-policy.json`; inspect with `gh api repos/<owner>/<repo>/contents/.engineering-assurance/approval-policy.json` and review-gate results. See [FR-061](../../spec/functional/FR-061-role-policy-and-distinct-individuals.md), [FR-063](../../spec/functional/FR-063-unstaffed-required-role-fails-visibly.md), and EA-109. |

## Open operational proposals

The 5-working-day post-hoc review deadline and the four higher-risk triggers
above are proposed values awaiting Peter's confirmation. The 90-calendar-day
periodic bypass-review interval is this SOP's proposed operational choice. Its
quarterly cadence gives the maintainer a predictable review point while
keeping bypass and missing-receipt evidence from accumulating for long.
