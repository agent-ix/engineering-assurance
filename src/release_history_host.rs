// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Adapter for the `release-history` command (FR-052).
//!
//! The history is read with the `signature-verify` adapter's isolated git and
//! object access only: no working tree, no record content, no ref written. The
//! pure request rules and the record path grammar come from the library; this
//! adapter walks the first-parent history and renders the one result line.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use engineering_assurance::release_history::{
    is_commit_hash, is_records_dir, is_tag_name, record_of_path,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::release_host::{GitFailure, git, object_id};

const REQUEST_PROTOCOL: &str = "engineering-assurance.release-history-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.release-history/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    repository: PathBuf,
    baseline_commit: String,
    previous_release_tag: Option<String>,
    records_dir: String,
}

/// Typed refusal that produces no list.
#[derive(Debug, Error)]
pub(crate) enum ReleaseHistoryError {
    /// The request is not readable JSON.
    #[error("release-history request is not readable JSON")]
    Unreadable,
    /// The request is not an exact release-history request.
    #[error("release-history request is not an exact release-history request")]
    Invalid,
    /// The baseline commit is not in the repository.
    #[error("release-history baseline commit is not in the repository")]
    BaselineUnknown,
    /// No tag of that name exists under `refs/tags/`.
    #[error("release-history previous release tag is not under refs/tags/")]
    PreviousTagUnknown,
    /// The tag does not peel to a commit with exactly one parent.
    #[error("release-history previous release tag does not name a one-parent commit")]
    PreviousNotReleaseCommit,
    /// The previous baseline commit is not an ancestor of the baseline commit.
    #[error("release-history previous baseline commit is not an ancestor of the baseline commit")]
    PreviousNotAncestor,
    /// The repository is a shallow clone.
    #[error("release-history repository is shallow")]
    Shallow,
    /// Git could not answer a read the list needs.
    #[error("release-history could not read the repository with git")]
    GitFailed,
    /// The result cannot be encoded.
    #[error("release-history result cannot be encoded")]
    ResultEncoding,
}

impl ReleaseHistoryError {
    /// Stable machine error code.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "release_history_request_unreadable",
            Self::Invalid => "release_history_request_invalid",
            Self::BaselineUnknown => "release_history_baseline_commit_unknown",
            Self::PreviousTagUnknown => "release_history_previous_tag_unknown",
            Self::PreviousNotReleaseCommit => "release_history_previous_not_a_release_commit",
            Self::PreviousNotAncestor => "release_history_previous_not_ancestor",
            Self::Shallow => "release_history_repository_shallow",
            Self::GitFailed => "release_history_git_failed",
            Self::ResultEncoding => "release_history_result_encoding_failed",
        }
    }
}

impl From<GitFailure> for ReleaseHistoryError {
    fn from(_: GitFailure) -> Self {
        Self::GitFailed
    }
}

#[derive(Serialize)]
struct PreviousRelease<'a> {
    tag: &'a str,
    commit: &'a str,
}

#[derive(Serialize)]
struct RecordLine {
    record_id: String,
    revision: u64,
    path: String,
    commit: String,
}

#[derive(Serialize)]
struct ResultLine<'a> {
    protocol: &'static str,
    baseline_commit: &'a str,
    previous_release: Option<PreviousRelease<'a>>,
    records: Vec<RecordLine>,
}

/// Runs one request document and returns the result line.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, ReleaseHistoryError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| ReleaseHistoryError::Unreadable)?;
    let request: Request =
        serde_json::from_value(value).map_err(|_| ReleaseHistoryError::Invalid)?;
    if request.protocol != REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !is_commit_hash(&request.baseline_commit)
        || !is_records_dir(&request.records_dir)
        || !request.previous_release_tag.as_deref().is_none_or(is_tag_name)
    {
        return Err(ReleaseHistoryError::Invalid);
    }
    let repository = request.repository.as_path();
    if is_shallow(repository)? {
        return Err(ReleaseHistoryError::Shallow);
    }
    let baseline = commit_of(repository, &request.baseline_commit)?
        .ok_or(ReleaseHistoryError::BaselineUnknown)?;
    let previous = match request.previous_release_tag.as_deref() {
        Some(tag) => Some((tag, previous_release(repository, tag)?)),
        None => None,
    };
    let previous_baseline = previous.as_ref().map(|(_, release)| release.baseline.as_str());
    let range = commits_in_range(repository, &baseline, previous_baseline)?;
    let records = records_added(repository, &baseline, &range, &request.records_dir)?;
    let line = ResultLine {
        protocol: RESULT_PROTOCOL,
        baseline_commit: &request.baseline_commit,
        previous_release: previous.as_ref().map(|(tag, release)| PreviousRelease {
            tag,
            commit: &release.commit,
        }),
        records,
    };
    let mut bytes = serde_json::to_vec(&line).map_err(|_| ReleaseHistoryError::ResultEncoding)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn is_shallow(repository: &Path) -> Result<bool, GitFailure> {
    let output = git(repository, &["rev-parse", "--is-shallow-repository"])?;
    if !output.status.success() {
        return Err(GitFailure);
    }
    match output.stdout.as_slice() {
        b"true\n" => Ok(true),
        b"false\n" => Ok(false),
        _ => Err(GitFailure),
    }
}

fn commit_of(repository: &Path, revision: &str) -> Result<Option<String>, GitFailure> {
    object_id(repository, &format!("{revision}^{{commit}}"))
}

/// A previous release: the release commit a tag peels to and its single parent.
struct PreviousCommit {
    commit: String,
    baseline: String,
}

fn previous_release(repository: &Path, tag: &str) -> Result<PreviousCommit, ReleaseHistoryError> {
    let reference = format!("refs/tags/{tag}");
    let object = object_id(repository, &reference)?.ok_or(ReleaseHistoryError::PreviousTagUnknown)?;
    let commit = commit_of(repository, &object)?
        .ok_or(ReleaseHistoryError::PreviousNotReleaseCommit)?;
    let mut parents = commit_ids(
        repository,
        &["rev-list", "--parents", "-n", "1", "--end-of-options", &commit],
    )?
    .into_iter();
    // The first id is the commit itself; the release commit has exactly one parent.
    let baseline = match (parents.next(), parents.next(), parents.next()) {
        (Some(_), Some(parent), None) => parent,
        _ => return Err(ReleaseHistoryError::PreviousNotReleaseCommit),
    };
    Ok(PreviousCommit { commit, baseline })
}

/// The whitespace-separated ids git prints for `arguments`.
fn commit_ids(repository: &Path, arguments: &[&str]) -> Result<Vec<String>, GitFailure> {
    let output = git(repository, arguments)?;
    if !output.status.success() {
        return Err(GitFailure);
    }
    let text = String::from_utf8(output.stdout).map_err(|_| GitFailure)?;
    Ok(text.split_whitespace().map(str::to_owned).collect())
}

/// The first-parent commits of `baseline` after `previous`, newest first. With
/// no previous baseline the walk runs to the root.
fn commits_in_range(
    repository: &Path,
    baseline: &str,
    previous: Option<&str>,
) -> Result<Vec<String>, ReleaseHistoryError> {
    let chain = commit_ids(
        repository,
        &["rev-list", "--first-parent", "--end-of-options", baseline],
    )?;
    let Some(previous) = previous else {
        return Ok(chain);
    };
    if let Some(position) = chain.iter().position(|commit| commit == previous) {
        return Ok(chain[..position].to_vec());
    }
    let ancestor = git(
        repository,
        &["merge-base", "--is-ancestor", previous, baseline],
    )?;
    match ancestor.status.code() {
        Some(0) => {}
        Some(1) => return Err(ReleaseHistoryError::PreviousNotAncestor),
        _ => return Err(ReleaseHistoryError::GitFailed),
    }
    // The previous baseline joined by a merge: the range is the first-parent
    // commits that are not among its ancestors.
    let known: BTreeSet<String> = commit_ids(
        repository,
        &["rev-list", "--end-of-options", previous],
    )?
    .into_iter()
    .collect();
    Ok(chain
        .into_iter()
        .take_while(|commit| !known.contains(commit))
        .collect())
}

/// Records added by a commit of `range` and present at `baseline`, in result order.
fn records_added(
    repository: &Path,
    baseline: &str,
    range: &[String],
    records_dir: &str,
) -> Result<Vec<RecordLine>, ReleaseHistoryError> {
    if range.is_empty() {
        return Ok(Vec::new());
    }
    let count = range.len().to_string();
    let output = git(
        repository,
        &[
            "--literal-pathspecs",
            "log",
            "--first-parent",
            "--root",
            "--no-renames",
            "--diff-merges=first-parent",
            "--raw",
            "-z",
            "--diff-filter=A",
            "--format=%H",
            "-n",
            &count,
            "--end-of-options",
            baseline,
            "--",
            records_dir,
        ],
    )?;
    if !output.status.success() {
        return Err(ReleaseHistoryError::GitFailed);
    }
    let present = files_at(repository, baseline, records_dir)?;
    // `log` lists newest first, so an older commit replaces a newer one.
    let mut commit = String::new();
    let mut found: BTreeMap<(String, u64), RecordLine> = BTreeMap::new();
    let mut tokens = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|token| !token.is_empty());
    while let Some(token) = tokens.next() {
        if token.starts_with(b":") {
            let path = tokens.next().ok_or(GitFailure)?;
            if !present.contains(path) {
                continue;
            }
            if let Some((record_id, revision)) = record_of_path(records_dir, path) {
                found.insert(
                    (record_id.to_owned(), revision),
                    RecordLine {
                        record_id: record_id.to_owned(),
                        revision,
                        path: String::from_utf8_lossy(path).into_owned(),
                        commit: commit.clone(),
                    },
                );
            }
        } else {
            commit = String::from_utf8(token.to_vec()).map_err(|_| GitFailure)?;
        }
    }
    let mut records: Vec<RecordLine> = found.into_values().collect();
    records.sort_by(|a, b| {
        (&a.record_id, a.revision, &a.commit).cmp(&(&b.record_id, b.revision, &b.commit))
    });
    Ok(records)
}

/// The blob paths under `records_dir` at `commit`.
fn files_at(
    repository: &Path,
    commit: &str,
    records_dir: &str,
) -> Result<BTreeSet<Vec<u8>>, GitFailure> {
    let output = git(
        repository,
        &[
            "--literal-pathspecs",
            "ls-tree",
            "-r",
            "-z",
            "--full-tree",
            commit,
            "--",
            records_dir,
        ],
    )?;
    if !output.status.success() {
        return Err(GitFailure);
    }
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let Some(tab) = entry.iter().position(|byte| *byte == b'\t') else {
                return Some(Err(GitFailure));
            };
            entry[..tab]
                .windows(6)
                .any(|window| window == b" blob ")
                .then(|| Ok(entry[tab + 1..].to_vec()))
        })
        .collect()
}
