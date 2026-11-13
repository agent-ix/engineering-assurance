// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Adapter for the `release-verify` command (FR-047, FR-051) and the Baseline
//! resolution (FR-046).
//!
//! Everything about the tag's signature, trailers and meaning is the
//! `approval-verify` adapter's result, used unchanged. This adapter reads the
//! release record, the Baseline and the release commit's shape with the
//! `signature-verify` adapter's isolated git, hands the facts to the pure
//! decision, and renders the result. It starts no process of its own, writes
//! nothing and never reads the working tree. It computes no hash: every object
//! id is the one git reports. It checks that the snapshot is present and never
//! opens it.

use std::path::{Path, PathBuf};

use engineering_assurance::approval_verify::{ApprovalStatus, SigningTime};
use engineering_assurance::release::{
    BaselineState, Changes, Check, Facts, Reason, frontmatter, is_repository_path, reasons,
};
use engineering_assurance::signature_verify::is_plain_revision;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::approval_verify_host::{ResultLine as ApprovalLine, gitsign_identity};
use crate::process_host::CompletedProcess;
use crate::signature_verify_host::{GitsignRequest, check_allowed_signers, run_git};

const REQUEST_PROTOCOL: &str = "engineering-assurance.release-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.release-verify-result/v1";
/// The most items a Baseline may list, so that one git call tests every path.
const MAX_BASELINE_ITEMS: usize = 1024;
const RELEASE_SCHEMA: &str =
    include_str!("../engineering_assurance/schemas/release-frontmatter.schema.json");
const BASELINE_SCHEMA: &str =
    include_str!("../engineering_assurance/schemas/baseline-frontmatter.schema.json");

/// The kind of a configuration item.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ItemKind {
    SpecArtifact,
    SoupRecord,
    Evidence,
}

// The schema has already closed the document; only the members read are typed.
#[derive(Deserialize)]
struct BaselineDocument {
    name: String,
    items: Vec<ItemDocument>,
}

#[derive(Deserialize)]
struct ItemDocument {
    path: String,
    kind: ItemKind,
}

#[derive(Deserialize)]
struct BaselineRef {
    path: String,
    commit: String,
}

#[derive(Deserialize)]
struct ReleaseDocument {
    id: String,
    name: String,
    baseline: BaselineRef,
    snapshot_path: String,
    decision_owner: String,
    #[serde(default)]
    accepted_anomalies: Vec<String>,
}

/// One Baseline item at the resolved commit.
#[derive(Debug, Serialize)]
pub(crate) struct ResolvedItem {
    pub(crate) path: String,
    pub(crate) kind: ItemKind,
    pub(crate) present: bool,
}

/// A Baseline read at one commit.
#[derive(Debug)]
pub(crate) struct Resolution {
    pub(crate) name: String,
    /// The commit hash as git resolves it.
    pub(crate) commit: String,
    pub(crate) items: Vec<ResolvedItem>,
}

/// Why a Baseline gives no item list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BaselineError {
    /// The commit is not in the repository.
    CommitUnknown,
    /// The Baseline file is absent at the commit.
    FileAbsent,
    /// The Baseline file is invalid at the commit.
    FileInvalid,
    /// Git could not be run.
    Git,
}

/// Git could not be run or its answer could not be read.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GitFailure;

pub(crate) fn git(repository: &Path, arguments: &[&str]) -> Result<CompletedProcess, GitFailure> {
    run_git(repository, arguments).map_err(|_| GitFailure)
}

/// The object id git resolves `expression` to, or `None` when it names nothing.
pub(crate) fn object_id(repository: &Path, expression: &str) -> Result<Option<String>, GitFailure> {
    let output = git(
        repository,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            expression,
        ],
    )?;
    match output.status.code() {
        Some(0) => {}
        // `--verify --quiet` exits 1 when the expression names nothing.
        Some(1) => return Ok(None),
        _ => return Err(GitFailure),
    }
    let text = String::from_utf8(output.stdout).map_err(|_| GitFailure)?;
    let id = text.trim_end_matches('\n');
    Ok((!id.is_empty() && id.bytes().all(|b| b.is_ascii_hexdigit())).then(|| id.to_owned()))
}

/// The commit `revision` peels to.
fn commit_of(repository: &Path, revision: &str) -> Result<Option<String>, GitFailure> {
    object_id(repository, &format!("{revision}^{{commit}}"))
}

/// The id of the object at `path` in `commit`.
fn object_at(repository: &Path, commit: &str, path: &str) -> Result<Option<String>, GitFailure> {
    object_id(repository, &format!("{commit}:{path}"))
}

/// The bytes of the blob `id`, or `None` when the object is not a blob.
fn blob(repository: &Path, id: &str) -> Result<Option<Vec<u8>>, GitFailure> {
    let kind = git(repository, &["cat-file", "-t", id])?;
    if !kind.status.success() {
        return Err(GitFailure);
    }
    if kind.stdout != b"blob\n" {
        return Ok(None);
    }
    let content = git(repository, &["cat-file", "blob", id])?;
    if content.status.success() {
        Ok(Some(content.stdout))
    } else {
        Err(GitFailure)
    }
}

/// The paths among `paths` that exist at `commit`, from one `ls-tree` call.
fn paths_present(
    repository: &Path,
    commit: &str,
    paths: &[&str],
) -> Result<std::collections::BTreeSet<Vec<u8>>, GitFailure> {
    let mut arguments = vec![
        "--literal-pathspecs",
        "ls-tree",
        "-z",
        "--full-tree",
        commit,
        "--",
    ];
    arguments.extend(paths);
    let output = git(repository, &arguments)?;
    if !output.status.success() {
        return Err(GitFailure);
    }
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let tab = entry
                .iter()
                .position(|byte| *byte == b'\t')
                .ok_or(GitFailure)?;
            Ok(entry[tab + 1..].to_vec())
        })
        .collect()
}

fn is_valid(schema: &str, instance: &Value) -> bool {
    let Ok(schema) = serde_json::from_str::<Value>(schema) else {
        return false;
    };
    jsonschema::options()
        .offline()
        .should_validate_formats(true)
        .build(&schema)
        .is_ok_and(|validator| validator.is_valid(instance))
}

/// The frontmatter of a Markdown document that satisfies `schema`, as typed.
fn document<T: for<'de> Deserialize<'de>>(bytes: &[u8], schema: &str) -> Option<T> {
    let text = std::str::from_utf8(bytes).ok()?;
    let value = engineering_assurance::structured_yaml::parse_unambiguous_yaml_json(
        frontmatter(text)?.as_bytes(),
    )?;
    if !is_valid(schema, &value) {
        return None;
    }
    serde_json::from_value(value).ok()
}

/// Reads the Baseline file `path` at `commit` and tests each item path there,
/// with git object access only (FR-046).
pub(crate) fn resolve_baseline(
    repository: &Path,
    commit: &str,
    path: &str,
) -> Result<Resolution, BaselineError> {
    let git_error = |_: GitFailure| BaselineError::Git;
    let commit = commit_of(repository, commit)
        .map_err(git_error)?
        .ok_or(BaselineError::CommitUnknown)?;
    let file = object_at(repository, &commit, path)
        .map_err(git_error)?
        .and_then(|id| blob(repository, &id).transpose())
        .transpose()
        .map_err(git_error)?
        .ok_or(BaselineError::FileAbsent)?;
    let baseline: BaselineDocument =
        document(&file, BASELINE_SCHEMA).ok_or(BaselineError::FileInvalid)?;
    if baseline.items.len() > MAX_BASELINE_ITEMS {
        return Err(BaselineError::FileInvalid);
    }
    let mut seen = std::collections::BTreeSet::new();
    if !baseline
        .items
        .iter()
        .all(|item| seen.insert(item.path.as_str()))
    {
        return Err(BaselineError::FileInvalid);
    }
    let paths: Vec<&str> = baseline
        .items
        .iter()
        .map(|item| item.path.as_str())
        .collect();
    let present = paths_present(repository, &commit, &paths).map_err(git_error)?;
    let items = baseline
        .items
        .into_iter()
        .map(|item| ResolvedItem {
            present: present.contains(item.path.as_bytes()),
            path: item.path,
            kind: item.kind,
        })
        .collect();
    Ok(Resolution {
        name: baseline.name,
        commit,
        items,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    repository: PathBuf,
    tag: String,
    record_path: String,
    allowed_signers: PathBuf,
    gitsign: Option<GitsignRequest>,
}

/// Typed refusal that produces no verdict.
#[derive(Debug, Error)]
pub(crate) enum ReleaseVerifyError {
    /// The request is not readable JSON.
    #[error("release-verify request is not readable JSON")]
    Unreadable,
    /// The request is JSON that is not an exact release-verify request.
    #[error("release-verify request is not an exact release-verify request")]
    Invalid,
    /// The allowed-signers file is not a readable regular UTF-8 file of at most 1 MiB.
    #[error(
        "release-verify allowed-signers file is not a readable UTF-8 regular file of at most 1 MiB"
    )]
    AllowedSigners,
    /// Git could not answer a read the verdict needs.
    #[error("release-verify could not read the repository with git")]
    GitFailed,
    /// The result cannot be encoded.
    #[error("release-verify result cannot be encoded")]
    ResultEncoding,
}

impl ReleaseVerifyError {
    /// Stable machine error code.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "release_verify_request_unreadable",
            Self::Invalid => "release_verify_request_invalid",
            Self::AllowedSigners => "release_verify_allowed_signers_invalid",
            Self::GitFailed => "release_verify_git_failed",
            Self::ResultEncoding => "release_verify_result_encoding_failed",
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Released,
    NotReleased,
}

#[derive(Serialize)]
struct ReleaseInfo<'a> {
    id: &'a str,
    name: &'a str,
    snapshot_path: &'a str,
}

#[derive(Serialize)]
struct BaselineInfo<'a> {
    name: &'a str,
    commit: &'a str,
    items: Vec<ItemInfo<'a>>,
}

#[derive(Serialize)]
struct ItemInfo<'a> {
    path: &'a str,
    kind: ItemKind,
}

/// The person, meaning and signing time of a released decision.
#[derive(Serialize)]
struct DecisionInfo<'a> {
    approver: &'a str,
    meaning: &'a [String],
    #[serde(flatten)]
    signing_time: &'a SigningTime,
}

#[derive(Serialize)]
struct ResultLine<'a> {
    protocol: &'static str,
    status: Status,
    tag: &'a str,
    reasons: Vec<Reason>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    absent_items: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    approval: Option<ApprovalLine<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    release: Option<ReleaseInfo<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    baseline: Option<BaselineInfo<'a>>,
    #[serde(flatten)]
    decision: Option<DecisionInfo<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accepted_anomalies: Option<&'a [String]>,
}

impl<'a> ResultLine<'a> {
    fn not_released(tag: &'a str, reasons: Vec<Reason>) -> Self {
        Self {
            protocol: RESULT_PROTOCOL,
            status: Status::NotReleased,
            tag,
            reasons,
            absent_items: Vec::new(),
            approval: None,
            release: None,
            baseline: None,
            decision: None,
            accepted_anomalies: None,
        }
    }
}

/// Runs one request document and returns the result line.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, ReleaseVerifyError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| ReleaseVerifyError::Unreadable)?;
    let request: Request =
        serde_json::from_value(value).map_err(|_| ReleaseVerifyError::Invalid)?;
    if request.protocol != REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.allowed_signers.is_absolute()
        || !is_plain_revision(&request.tag)
        || !is_repository_path(&request.record_path)
        || request.record_path.starts_with('-')
    {
        return Err(ReleaseVerifyError::Invalid);
    }
    let identity =
        gitsign_identity(request.gitsign.as_ref()).map_err(|()| ReleaseVerifyError::Invalid)?;
    check_allowed_signers(&request.allowed_signers)
        .map_err(|_| ReleaseVerifyError::AllowedSigners)?;
    let mut line = match verify(&request, identity.as_ref()) {
        Ok(line) => line,
        Err(VerifyError::Git) => return Err(ReleaseVerifyError::GitFailed),
        Err(VerifyError::Encoding) => return Err(ReleaseVerifyError::ResultEncoding),
    };
    line.push(b'\n');
    Ok(line)
}

enum VerifyError {
    Git,
    Encoding,
}

impl From<GitFailure> for VerifyError {
    fn from(_: GitFailure) -> Self {
        Self::Git
    }
}

fn encode(result: &ResultLine<'_>) -> Result<Vec<u8>, VerifyError> {
    serde_json::to_vec(result).map_err(|_| VerifyError::Encoding)
}

/// The changes a commit makes against its first parent, as `(status, path bytes)`.
fn changes(
    repository: &Path,
    commit: &str,
    first_parent: Option<&str>,
) -> Result<Vec<(u8, Vec<u8>)>, GitFailure> {
    let mut arguments = vec![
        "diff-tree",
        "-r",
        "-z",
        "--raw",
        "--no-renames",
        "--no-commit-id",
    ];
    match first_parent {
        Some(parent) => arguments.extend([parent, commit]),
        None => arguments.extend(["--root", commit]),
    }
    let output = git(repository, &arguments)?;
    if !output.status.success() {
        return Err(GitFailure);
    }
    let mut parts = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty());
    let mut found = Vec::new();
    while let Some(meta) = parts.next() {
        let status = *meta.last().ok_or(GitFailure)?;
        let path = parts.next().ok_or(GitFailure)?;
        found.push((status, path.to_vec()));
    }
    Ok(found)
}

/// How the Baseline of the record resolves at the baseline commit.
fn baseline_state(
    repository: &Path,
    baseline_commit: Option<&str>,
    record: &ReleaseDocument,
) -> Result<(BaselineState, Option<Resolution>), VerifyError> {
    let Some(commit) = baseline_commit else {
        return Ok((BaselineState::CommitUnknown, None));
    };
    match resolve_baseline(repository, commit, &record.baseline.path) {
        Ok(resolution) => Ok((
            BaselineState::Resolved {
                all_items_present: resolution.items.iter().all(|item| item.present),
            },
            Some(resolution),
        )),
        Err(BaselineError::FileAbsent) => Ok((BaselineState::FileAbsent, None)),
        Err(BaselineError::FileInvalid) => Ok((BaselineState::FileInvalid, None)),
        Err(BaselineError::CommitUnknown) => Ok((BaselineState::CommitUnknown, None)),
        Err(BaselineError::Git) => Err(VerifyError::Git),
    }
}

/// What git says about the release commit against the baseline commit.
struct Shape {
    child_of_baseline: Option<bool>,
    changes: Changes,
    snapshot_present_at_baseline: Option<bool>,
}

fn commit_shape(
    repository: &Path,
    release_commit: &str,
    baseline_commit: Option<&str>,
    (record_path, snapshot_path): (&str, &str),
) -> Result<Shape, VerifyError> {
    let parents = git(
        repository,
        &[
            "rev-list",
            "--parents",
            "-n",
            "1",
            "--end-of-options",
            release_commit,
        ],
    )?;
    if !parents.status.success() {
        return Err(VerifyError::Git);
    }
    let parents = String::from_utf8(parents.stdout).map_err(|_| GitFailure)?;
    let parents: Vec<&str> = parents.split_whitespace().skip(1).collect();
    let changed = changes(repository, release_commit, parents.first().copied())?;
    let added = |path: &str| {
        changed
            .iter()
            .any(|(status, name)| *status == b'A' && name == path.as_bytes())
    };
    let snapshot_present_at_baseline = baseline_commit
        .map(|commit| object_at(repository, commit, snapshot_path).map(|id| id.is_some()))
        .transpose()?;
    Ok(Shape {
        child_of_baseline: baseline_commit.map(|commit| parents == [commit]),
        changes: Changes {
            only_record_and_snapshot: changed.iter().all(|(_, name)| {
                name == record_path.as_bytes() || name == snapshot_path.as_bytes()
            }),
            record_added: added(record_path),
            snapshot_added: added(snapshot_path),
        },
        snapshot_present_at_baseline,
    })
}

fn verify(
    request: &Request,
    identity: Option<&engineering_assurance::signature_verify::GitsignIdentity>,
) -> Result<Vec<u8>, VerifyError> {
    let repository = request.repository.as_path();
    let tag = request
        .tag
        .strip_prefix("refs/tags/")
        .unwrap_or(&request.tag);
    let Some(release_commit) = commit_of(repository, &request.tag)? else {
        let reasons = vec![Reason::Check(Check::RecordUnreadable)];
        return encode(&ResultLine::not_released(tag, reasons));
    };
    let early = |check| {
        let reasons = vec![Reason::Check(check)];
        encode(&ResultLine::not_released(tag, reasons))
    };
    let Some(record_object) = object_at(repository, &release_commit, &request.record_path)? else {
        return early(Check::RecordUnreadable);
    };
    let Some(record_bytes) = blob(repository, &record_object)? else {
        return early(Check::RecordUnreadable);
    };
    let Some(record) = document::<ReleaseDocument>(&record_bytes, RELEASE_SCHEMA)
        .filter(|record| record.snapshot_path != request.record_path)
    else {
        return early(Check::RecordInvalid);
    };

    let digest = format!("git-blob:{record_object}");
    let approval = ApprovalLine::verify(
        repository,
        &request.tag,
        (&record.id, &digest),
        &request.allowed_signers,
        request.gitsign.as_ref(),
        identity,
    );
    let status = approval.assessment().status();
    let approved = approval.assessment().approval().cloned();

    let baseline_commit = commit_of(repository, &record.baseline.commit)?;
    let (state, resolution) = baseline_state(repository, baseline_commit.as_deref(), &record)?;
    let shape = commit_shape(
        repository,
        &release_commit,
        baseline_commit.as_deref(),
        (&request.record_path, &record.snapshot_path),
    )?;
    let facts = Facts {
        approval: status,
        approver_is_decision_owner: approved
            .as_ref()
            .is_some_and(|approval| approval.approver == record.decision_owner),
        baseline: state,
        child_of_baseline: shape.child_of_baseline,
        changes: shape.changes,
        snapshot_present_at_baseline: shape.snapshot_present_at_baseline,
    };
    let failed = reasons(&facts);
    let absent_items = resolution
        .as_ref()
        .map(|resolved| {
            resolved
                .items
                .iter()
                .filter(|item| !item.present)
                .map(|item| item.path.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let released = failed.is_empty() && status == ApprovalStatus::Approved;
    let mut result = ResultLine::not_released(tag, failed);
    result.absent_items = absent_items;
    if released && let (Some(approved), Some(resolution)) = (approved.as_ref(), resolution.as_ref())
    {
        result.status = Status::Released;
        result.release = Some(ReleaseInfo {
            id: &record.id,
            name: &record.name,
            snapshot_path: &record.snapshot_path,
        });
        result.baseline = Some(BaselineInfo {
            name: &resolution.name,
            commit: &resolution.commit,
            items: resolution
                .items
                .iter()
                .map(|item| ItemInfo {
                    path: &item.path,
                    kind: item.kind,
                })
                .collect(),
        });
        result.decision = Some(DecisionInfo {
            approver: &approved.approver,
            meaning: &approved.meaning,
            signing_time: &approved.signing_time,
        });
        result.accepted_anomalies = Some(&record.accepted_anomalies);
    }
    result.approval = Some(approval);
    encode(&result)
}
