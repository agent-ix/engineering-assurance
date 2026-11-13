// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Decision of whether a signed release tag is a named person's release of one
//! Baseline (FR-047, FR-051).
//!
//! The module is pure. It takes the `approval-verify` status of the tag
//! (FR-041), which owns every signature, trailer and meaning rule, and the
//! facts the git adapter read about the baseline and the release commit, and
//! lists every failed check. It holds no signature, certificate, trailer or
//! hashing code, and it never opens the evidence snapshot.

use serde::Serialize;

use crate::approval_verify::ApprovalStatus;

/// Whether `path` is a repository-relative path of a Baseline item, a release
/// record or a release evidence snapshot: not empty, no leading `/`, no empty,
/// `.` or `..` segment, no glob character, no control character and no segment
/// starting with a space.
#[must_use]
pub fn is_repository_path(path: &str) -> bool {
    !path.is_empty()
        && path.split('/').all(|segment| {
            !matches!(segment, "" | "." | "..")
                && !segment.starts_with(' ')
                && !segment.chars().any(|c| {
                    c.is_control() || matches!(c, '*' | '?' | '[' | ']' | '{' | '}' | '\\')
                })
        })
}

/// The YAML frontmatter of a Markdown document: the text between an opening
/// `---` line at the very start and the next `---` line.
#[must_use]
pub fn frontmatter(document: &str) -> Option<&str> {
    let body = document
        .strip_prefix("---\n")
        .or_else(|| document.strip_prefix("---\r\n"))?;
    let mut offset = 0;
    for line in body.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return Some(&body[..offset]);
        }
        offset += line.len();
    }
    None
}

/// A failed check of a release other than the FR-041 status.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    /// The release record cannot be read at the release commit.
    RecordUnreadable,
    /// The release record is not a valid Release document.
    RecordInvalid,
    /// The baseline commit is not in the repository.
    BaselineCommitUnknown,
    /// The Baseline file is absent at the baseline commit.
    BaselineFileAbsent,
    /// The Baseline file is invalid at the baseline commit.
    BaselineFileInvalid,
    /// The release commit has no single parent equal to the baseline commit.
    ReleaseCommitNotChildOfBaseline,
    /// The release commit changes a path other than the record and the snapshot.
    ReleaseCommitChangesOtherPaths,
    /// The release commit does not add the record.
    ReleaseCommitRecordNotAdded,
    /// The release commit does not add the snapshot.
    ReleaseCommitSnapshotNotAdded,
    /// The snapshot path exists at the baseline commit.
    SnapshotPathPresentAtBaseline,
    /// A Baseline item path is absent at the baseline commit.
    BaselineItemAbsent,
    /// The approver is not the record's `decision_owner`.
    DecisionOwnerMismatch,
}

/// One reason a release is not a release.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Reason {
    /// The FR-041 status of the tag, when it is not `approved`.
    Approval(ApprovalStatus),
    /// Another failed check.
    Check(Check),
}

/// How the Baseline resolved at the baseline commit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BaselineState {
    /// The baseline commit is not in the repository.
    CommitUnknown,
    /// The Baseline file is absent at the baseline commit.
    FileAbsent,
    /// The Baseline file is invalid at the baseline commit.
    FileInvalid,
    /// The Baseline resolved.
    Resolved {
        /// Whether every item path is present at the baseline commit.
        all_items_present: bool,
    },
}

/// The facts of a release whose record was readable and valid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Facts {
    /// The FR-041 status of the tag.
    pub approval: ApprovalStatus,
    /// Whether the FR-041 approver equals `decision_owner` byte for byte; only
    /// read when the approval is `approved`.
    pub approver_is_decision_owner: bool,
    /// How the Baseline resolved.
    pub baseline: BaselineState,
    /// Whether the release commit has exactly one parent, the baseline commit;
    /// `None` when the baseline commit is unknown.
    pub child_of_baseline: Option<bool>,
    /// Whether the release commit changes only the record and the snapshot.
    pub changes_only_record_and_snapshot: bool,
    /// Whether the release commit adds the record.
    pub record_added: bool,
    /// Whether the release commit adds the snapshot.
    pub snapshot_added: bool,
    /// Whether the snapshot path exists at the baseline commit; `None` when the
    /// baseline commit is unknown.
    pub snapshot_present_at_baseline: Option<bool>,
}

/// Every failed check, in a fixed order. The release is `released` exactly when
/// the list is empty.
#[must_use]
pub fn reasons(facts: &Facts) -> Vec<Reason> {
    let mut found = Vec::new();
    let approved = facts.approval == ApprovalStatus::Approved;
    if !approved {
        found.push(Reason::Approval(facts.approval));
    }
    let mut check = |failed: bool, reason: Check| {
        if failed {
            found.push(Reason::Check(reason));
        }
    };
    match facts.baseline {
        BaselineState::CommitUnknown => check(true, Check::BaselineCommitUnknown),
        BaselineState::FileAbsent => check(true, Check::BaselineFileAbsent),
        BaselineState::FileInvalid => check(true, Check::BaselineFileInvalid),
        BaselineState::Resolved { all_items_present } => {
            check(!all_items_present, Check::BaselineItemAbsent);
        }
    }
    check(
        facts.child_of_baseline == Some(false),
        Check::ReleaseCommitNotChildOfBaseline,
    );
    check(
        !facts.changes_only_record_and_snapshot,
        Check::ReleaseCommitChangesOtherPaths,
    );
    check(!facts.record_added, Check::ReleaseCommitRecordNotAdded);
    check(!facts.snapshot_added, Check::ReleaseCommitSnapshotNotAdded);
    check(
        facts.snapshot_present_at_baseline == Some(true),
        Check::SnapshotPathPresentAtBaseline,
    );
    check(
        approved && !facts.approver_is_decision_owner,
        Check::DecisionOwnerMismatch,
    );
    found
}
