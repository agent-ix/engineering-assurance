// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Contract tests for the pure release decision (FR-047, FR-051).

use engineering_assurance::approval_verify::ApprovalStatus;
use engineering_assurance::release::{
    BaselineState, Changes, Check, Facts, Reason, frontmatter, is_repository_path, reasons,
};
use ix_trace_rs::trace;

const STATUSES: [ApprovalStatus; 7] = [
    ApprovalStatus::Approved,
    ApprovalStatus::WrongRecord,
    ApprovalStatus::UntrustedSigner,
    ApprovalStatus::MeaningMissing,
    ApprovalStatus::SignedBad,
    ApprovalStatus::Unsigned,
    ApprovalStatus::Unverifiable,
];

const BASELINES: [BaselineState; 5] = [
    BaselineState::CommitUnknown,
    BaselineState::FileAbsent,
    BaselineState::FileInvalid,
    BaselineState::Resolved {
        all_items_present: false,
    },
    BaselineState::Resolved {
        all_items_present: true,
    },
];

const TRI: [Option<bool>; 3] = [None, Some(false), Some(true)];
const BOOL: [bool; 2] = [false, true];

fn all_facts() -> Vec<Facts> {
    let mut all = Vec::new();
    for approval in STATUSES {
        for approver_is_decision_owner in BOOL {
            for baseline in BASELINES {
                for child_of_baseline in TRI {
                    for changes_only_record_and_snapshot in BOOL {
                        for record_added in BOOL {
                            for snapshot_added in BOOL {
                                for snapshot_present_at_baseline in TRI {
                                    // Both baseline-dependent facts are unknown
                                    // exactly when the baseline commit is.
                                    let unknown = baseline == BaselineState::CommitUnknown;
                                    if unknown != child_of_baseline.is_none()
                                        || unknown != snapshot_present_at_baseline.is_none()
                                    {
                                        continue;
                                    }
                                    all.push(Facts {
                                        approval,
                                        approver_is_decision_owner,
                                        baseline,
                                        child_of_baseline,
                                        changes: Changes {
                                            only_record_and_snapshot:
                                                changes_only_record_and_snapshot,
                                            record_added,
                                            snapshot_added,
                                        },
                                        snapshot_present_at_baseline,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    all
}

#[test]
#[trace("TC-415", "FR-047-AC-12", "FR-047-CON-1", "NFR-007-M-1")]
fn tc_415_released_is_returned_for_exactly_one_combination_of_the_facts() {
    let released: Vec<Facts> = all_facts()
        .into_iter()
        .filter(|facts| reasons(facts).is_empty())
        .collect();
    assert_eq!(released.len(), 1, "{released:?}");
    let [only] = released.as_slice() else {
        unreachable!("one passing combination")
    };
    assert_eq!(only.approval, ApprovalStatus::Approved);
    assert!(only.approver_is_decision_owner);
    assert_eq!(
        only.baseline,
        BaselineState::Resolved {
            all_items_present: true
        }
    );
    assert_eq!(only.child_of_baseline, Some(true));
    assert_eq!(only.snapshot_present_at_baseline, Some(false));
    assert!(
        only.changes.only_record_and_snapshot
            && only.changes.record_added
            && only.changes.snapshot_added
    );
}

#[test]
#[trace("TC-419", "FR-047-AC-12", "FR-047-CON-1")]
fn tc_419_a_status_other_than_approved_is_always_listed_and_never_released() {
    for facts in all_facts() {
        let found = reasons(&facts);
        if facts.approval == ApprovalStatus::Approved {
            assert!(!found.contains(&Reason::Approval(ApprovalStatus::Approved)));
        } else {
            assert_eq!(found.first(), Some(&Reason::Approval(facts.approval)));
        }
    }
}

#[test]
#[trace("TC-416", "FR-047-AC-10")]
fn tc_416_path_rules_refuse_every_unsafe_spelling() {
    for good in [
        "a",
        "spec/a.md",
        ".hidden/a",
        "a b/c d",
        "a..b",
        "release/REL-1.md",
    ] {
        assert!(is_repository_path(good), "{good:?}");
    }
    for bad in [
        "", "/a", "a/", "a//b", ".", "a/.", "a/./b", "..", "../a", "a/..", "a/*", "a?", "a[b]",
        "{a}", "a\\b", "a\u{7}", "a\nb", " a", "a/ b",
    ] {
        assert!(!is_repository_path(bad), "{bad:?}");
    }
}

#[test]
#[trace("TC-417", "FR-047-AC-7")]
fn tc_417_frontmatter_is_the_block_between_the_first_two_fences() {
    assert_eq!(frontmatter("---\na: 1\n---\nbody\n"), Some("a: 1\n"));
    assert_eq!(frontmatter("---\r\na: 1\r\n---\r\n"), Some("a: 1\r\n"));
    assert_eq!(frontmatter("---\n---\n"), Some(""));
    assert_eq!(frontmatter("a: 1\n---\n"), None);
    assert_eq!(frontmatter("\n---\na: 1\n---\n"), None);
    assert_eq!(frontmatter("---\na: 1\n"), None);
}

#[test]
#[trace("TC-418", "FR-047-AC-5", "FR-047-AC-6")]
fn tc_418_each_failed_fact_gives_its_own_reason_and_no_other() {
    let passing = Facts {
        approval: ApprovalStatus::Approved,
        approver_is_decision_owner: true,
        baseline: BaselineState::Resolved {
            all_items_present: true,
        },
        child_of_baseline: Some(true),
        changes: Changes {
            only_record_and_snapshot: true,
            record_added: true,
            snapshot_added: true,
        },
        snapshot_present_at_baseline: Some(false),
    };
    assert!(reasons(&passing).is_empty());
    let with_changes = |changes: Changes| Facts { changes, ..passing };
    let with_baseline = |baseline| Facts {
        baseline,
        ..passing
    };
    let cases = [
        (
            Facts {
                baseline: BaselineState::CommitUnknown,
                child_of_baseline: None,
                snapshot_present_at_baseline: None,
                ..passing
            },
            Check::BaselineCommitUnknown,
        ),
        (
            with_baseline(BaselineState::FileAbsent),
            Check::BaselineFileAbsent,
        ),
        (
            with_baseline(BaselineState::FileInvalid),
            Check::BaselineFileInvalid,
        ),
        (
            with_baseline(BaselineState::Resolved {
                all_items_present: false,
            }),
            Check::BaselineItemAbsent,
        ),
        (
            Facts {
                child_of_baseline: Some(false),
                ..passing
            },
            Check::ReleaseCommitNotChildOfBaseline,
        ),
        (
            with_changes(Changes {
                only_record_and_snapshot: false,
                ..passing.changes
            }),
            Check::ReleaseCommitChangesOtherPaths,
        ),
        (
            with_changes(Changes {
                record_added: false,
                ..passing.changes
            }),
            Check::ReleaseCommitRecordNotAdded,
        ),
        (
            with_changes(Changes {
                snapshot_added: false,
                ..passing.changes
            }),
            Check::ReleaseCommitSnapshotNotAdded,
        ),
        (
            Facts {
                snapshot_present_at_baseline: Some(true),
                ..passing
            },
            Check::SnapshotPathPresentAtBaseline,
        ),
        (
            Facts {
                approver_is_decision_owner: false,
                ..passing
            },
            Check::DecisionOwnerMismatch,
        ),
    ];
    for (facts, check) in cases {
        assert_eq!(reasons(&facts), vec![Reason::Check(check)], "{check:?}");
    }
}
