// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Adapter for the `approval-tag` command (FR-042).
//!
//! The request is checked, the record file is resolved, and `git tag -s` writes
//! the one tag through the caller's own git configuration. Git is the only
//! program started, from the absolute entries of `PATH` alone, and the only git
//! subcommands are `rev-parse`, `check-ref-format`, `config --get` and
//! `tag -s`: nothing is pushed, no configuration is written, and nothing signs
//! except the program the caller's configuration names.

use std::{
    ffi::OsStr,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use engineering_assurance::approval_tag::{
    SigningSetting, compose_message, is_meaning, is_record_digest, is_trailer_record_value,
    signing_refusal,
};
use engineering_assurance::signature_verify::is_plain_revision;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::process_host::{self, CompletedProcess, ProcessError, ProcessLimits};
use crate::signature_verify_host::{REMOVED_ENVIRONMENT, resolve_program};

const REQUEST_PROTOCOL: &str = "engineering-assurance.approval-tag-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.approval-tag-result/v1";
/// The only `record_type` a record may carry.
const RECORD_TYPE: &str = "change_assurance";
/// The only `schema_version` a record may carry.
const RECORD_SCHEMA_VERSION: u64 = 2;
/// Largest accepted record file in bytes.
const MAX_RECORD_BYTES: u64 = 8 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
/// Signing may wait on a passphrase prompt, so the bound is longer than the
/// 60 seconds of the verification adapters.
const PROCESS_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    repository: PathBuf,
    tag: String,
    #[serde(default = "head")]
    revision: String,
    record: PathBuf,
    record_id: String,
    record_digest: String,
    meaning: String,
}

fn head() -> String {
    "HEAD".to_owned()
}

#[derive(Serialize)]
struct ResultLine<'a> {
    protocol: &'static str,
    status: &'static str,
    tag: &'a str,
    tag_object: &'a str,
    target: &'a str,
    record_id: &'a str,
    record_digest: &'a str,
    meaning: &'a str,
}

/// Typed refusal that writes no tag.
#[derive(Debug, Error)]
pub(crate) enum ApprovalTagError {
    /// The request is not readable JSON.
    #[error("approval-tag request is not readable JSON")]
    Unreadable,
    /// The request is JSON that is not an exact approval-tag request.
    #[error("approval-tag request is not an exact approval-tag request")]
    Invalid,
    /// The record file does not resolve to the change-assurance record named.
    #[error(
        "approval-tag record is not a readable schema 2 change-assurance record file of at most 8 MiB with the requested record_id"
    )]
    RecordUnresolved,
    /// The repository directory is not a git repository.
    #[error("approval-tag repository is not a git repository")]
    RepositoryUnavailable,
    /// The tag name is not a valid bare tag name.
    #[error("approval-tag tag is not a valid bare tag name")]
    TagInvalid,
    /// A tag of that name exists.
    #[error("approval-tag tag already exists; it is never replaced")]
    TagExists,
    /// The revision to tag does not resolve to a commit.
    #[error("approval-tag revision does not resolve to a commit")]
    RevisionUnresolved,
    /// The git configuration cannot sign a verifiable tag.
    #[error(
        "approval-tag needs a git signing configuration: git config `{}` is missing or unsupported",
        .0.key()
    )]
    SigningUnconfigured(SigningSetting),
    /// `git tag -s` did not create the tag.
    #[error(
        "approval-tag: git tag -s did not create the tag (signing program, key or ref update failed)"
    )]
    CreateFailed,
    /// Git could not be started, timed out or produced too much output.
    #[error("approval-tag could not run git from an absolute PATH entry")]
    GitUnavailable,
    /// Git answered a query with a status or output the command does not know.
    #[error("approval-tag could not read git's answer")]
    GitFailed,
    /// The result cannot be encoded.
    #[error("approval-tag result cannot be encoded")]
    ResultEncoding,
}

impl ApprovalTagError {
    /// Stable machine error code.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "approval_tag_request_unreadable",
            Self::Invalid => "approval_tag_request_invalid",
            Self::RecordUnresolved => "approval_tag_record_unresolved",
            Self::RepositoryUnavailable => "approval_tag_repository_unavailable",
            Self::TagInvalid => "approval_tag_tag_invalid",
            Self::TagExists => "approval_tag_tag_exists",
            Self::RevisionUnresolved => "approval_tag_revision_unresolved",
            Self::SigningUnconfigured(_) => "approval_tag_signing_unconfigured",
            Self::CreateFailed => "approval_tag_create_failed",
            Self::GitUnavailable => "approval_tag_git_unavailable",
            Self::GitFailed => "approval_tag_git_failed",
            Self::ResultEncoding => "approval_tag_result_encoding_failed",
        }
    }
}

/// Runs one request document and returns the result line.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, ApprovalTagError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| ApprovalTagError::Unreadable)?;
    let request: Request = serde_json::from_value(value).map_err(|_| ApprovalTagError::Invalid)?;
    if request.protocol != REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.record.is_absolute()
        || !is_plain_revision(&request.tag)
        || !is_plain_revision(&request.revision)
        || !is_trailer_record_value(&request.record_id)
        || !is_record_digest(&request.record_digest)
        || !is_meaning(&request.meaning)
    {
        return Err(ApprovalTagError::Invalid);
    }
    // A name `refs/...` would create `refs/tags/refs/...`, not the tag named.
    if request.tag.starts_with("refs/") {
        return Err(ApprovalTagError::TagInvalid);
    }
    resolve_record(&request.record, &request.record_id)?;
    let digest = request.record_digest.as_str();
    let git = Git {
        repository: &request.repository,
    };
    git.require_repository()?;
    let full_name = format!("refs/tags/{}", request.tag);
    if !git.succeeds(&["check-ref-format", &full_name])? {
        return Err(ApprovalTagError::TagInvalid);
    }
    if git.resolve(&full_name)?.is_some() {
        return Err(ApprovalTagError::TagExists);
    }
    let target = git
        .resolve(&format!("{}^{{commit}}", request.revision))?
        .ok_or(ApprovalTagError::RevisionUnresolved)?;
    git.require_signing()?;
    git.create(
        &request.tag,
        &target,
        &compose_message(&request.record_id, digest, &request.meaning),
    )?;
    let tag_object = git
        .resolve(&full_name)?
        .ok_or(ApprovalTagError::CreateFailed)?;
    let mut line = serde_json::to_vec(&ResultLine {
        protocol: RESULT_PROTOCOL,
        status: "created",
        tag: &request.tag,
        tag_object: &tag_object,
        target: &target,
        record_id: &request.record_id,
        record_digest: digest,
        meaning: &request.meaning,
    })
    .map_err(|_| ApprovalTagError::ResultEncoding)?;
    line.push(b'\n');
    Ok(line)
}

/// Checks that `path` is the change-assurance record `record_id`: a regular
/// UTF-8 JSON file of at most [`MAX_RECORD_BYTES`] holding an object with
/// `schema_version` 2, `record_type` `change_assurance`, `record_id` equal to
/// `record_id`, and a `review_workflow` object with a `signed_approval` object.
/// The record's digest is never computed: the caller supplies it and Quoin's
/// verification compares it with the stored bytes.
fn resolve_record(path: &Path, record_id: &str) -> Result<(), ApprovalTagError> {
    fn unresolved<E>(_: E) -> ApprovalTagError {
        ApprovalTagError::RecordUnresolved
    }
    // The type is checked before the open: opening a FIFO would block.
    let before = std::fs::metadata(path).map_err(unresolved)?;
    if !before.is_file() || before.len() > MAX_RECORD_BYTES {
        return Err(ApprovalTagError::RecordUnresolved);
    }
    let file = File::open(path).map_err(unresolved)?;
    let mut bytes = Vec::new();
    file.take(MAX_RECORD_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(unresolved)?;
    if u64::try_from(bytes.len()).map_or(true, |len| len > MAX_RECORD_BYTES) {
        return Err(ApprovalTagError::RecordUnresolved);
    }
    let record: serde_json::Value = serde_json::from_slice(&bytes).map_err(unresolved)?;
    let text = |name: &str| record.get(name).and_then(serde_json::Value::as_str);
    let has_policy = record
        .get("review_workflow")
        .and_then(|workflow| workflow.get("signed_approval"))
        .is_some_and(serde_json::Value::is_object);
    if record
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        == Some(RECORD_SCHEMA_VERSION)
        && text("record_type") == Some(RECORD_TYPE)
        && text("record_id") == Some(record_id)
        && has_policy
    {
        Ok(())
    } else {
        Err(ApprovalTagError::RecordUnresolved)
    }
}

struct Git<'a> {
    repository: &'a Path,
}

impl Git<'_> {
    /// Runs git in the repository with the caller's own configuration: the
    /// system, global and repository files are read as `git tag -s` run by hand
    /// reads them. Only the repository-selecting variables are removed, and
    /// `PATH` is cut to its absolute entries so that neither git nor the signing
    /// program it starts is found inside the repository.
    fn run(&self, arguments: &[&OsStr]) -> Result<CompletedProcess, ApprovalTagError> {
        let (git, path) = resolve_program("git").map_err(|_| ApprovalTagError::GitUnavailable)?;
        let removed: Vec<&OsStr> = REMOVED_ENVIRONMENT.iter().map(OsStr::new).collect();
        process_host::run_configured(
            git.as_os_str(),
            arguments,
            Some(self.repository),
            &[
                (OsStr::new("PATH"), path.as_os_str()),
                (OsStr::new("GIT_OPTIONAL_LOCKS"), OsStr::new("0")),
            ],
            &removed,
            ProcessLimits {
                timeout: PROCESS_TIMEOUT,
                max_output_bytes: MAX_OUTPUT_BYTES,
            },
        )
        .map_err(|error| match error {
            ProcessError::Unavailable { .. }
            | ProcessError::TimedOut { .. }
            | ProcessError::OutputTooLarge { .. } => ApprovalTagError::GitUnavailable,
            ProcessError::PipeUnavailable { .. }
            | ProcessError::Observation { .. }
            | ProcessError::OutputUnreadable { .. } => ApprovalTagError::GitFailed,
        })
    }

    /// Whether `git <arguments>` exits 0; exit 1 is "no", anything else is
    /// unreadable.
    fn succeeds(&self, arguments: &[&str]) -> Result<bool, ApprovalTagError> {
        let arguments: Vec<&OsStr> = arguments.iter().map(OsStr::new).collect();
        match self.run(&arguments)?.status.code() {
            Some(0) => Ok(true),
            Some(1 | 128) => Ok(false),
            _ => Err(ApprovalTagError::GitFailed),
        }
    }

    fn require_repository(&self) -> Result<(), ApprovalTagError> {
        if !self.repository.is_dir() {
            return Err(ApprovalTagError::RepositoryUnavailable);
        }
        if self.succeeds(&["rev-parse", "--git-dir"])? {
            Ok(())
        } else {
            Err(ApprovalTagError::RepositoryUnavailable)
        }
    }

    /// The full object id `expression` names, or `None` when it names nothing.
    fn resolve(&self, expression: &str) -> Result<Option<String>, ApprovalTagError> {
        let arguments = [
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            expression,
        ]
        .map(OsStr::new);
        let output = self.run(&arguments)?;
        match output.status.code() {
            Some(0) => String::from_utf8(output.stdout)
                .ok()
                .map(|text| text.trim_end_matches('\n').to_owned())
                .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_hexdigit()))
                .map(Some)
                .ok_or(ApprovalTagError::GitFailed),
            Some(1) => Ok(None),
            _ => Err(ApprovalTagError::GitFailed),
        }
    }

    /// The value of `key` in the configuration git itself would read, or `None`
    /// when it is unset.
    fn config(&self, key: &str) -> Result<Option<String>, ApprovalTagError> {
        let arguments = ["config", "--get", key].map(OsStr::new);
        let output = self.run(&arguments)?;
        match output.status.code() {
            Some(0) => String::from_utf8(output.stdout)
                .map(|text| Some(text.trim_end_matches('\n').to_owned()))
                .map_err(|_| ApprovalTagError::GitFailed),
            Some(1) => Ok(None),
            _ => Err(ApprovalTagError::GitFailed),
        }
    }

    /// Refuses before any tag is written when git's configuration cannot sign a
    /// tag that `approval-verify` can verify.
    fn require_signing(&self) -> Result<(), ApprovalTagError> {
        let format = self.config("gpg.format")?;
        let (key, command) = if format.as_deref() == Some("ssh") {
            (
                self.config("user.signingkey")?,
                self.config("gpg.ssh.defaultKeyCommand")?,
            )
        } else {
            (None, None)
        };
        signing_refusal(format.as_deref(), key.as_deref(), command.as_deref())
            .map_or(Ok(()), |setting| {
                Err(ApprovalTagError::SigningUnconfigured(setting))
            })
    }

    /// `git tag -s` of `target` as `name` with `message`. Signing goes through
    /// the caller's configuration alone: no key, program or format is passed,
    /// and `-f` is never passed, so an existing tag is never replaced.
    fn create(&self, name: &str, target: &str, message: &str) -> Result<(), ApprovalTagError> {
        let directory = tempfile::tempdir().map_err(|_| ApprovalTagError::GitFailed)?;
        let path = directory.path().join("message");
        std::fs::write(&path, message).map_err(|_| ApprovalTagError::GitFailed)?;
        let arguments: [&OsStr; 8] = [
            OsStr::new("tag"),
            OsStr::new("-s"),
            OsStr::new("--cleanup=verbatim"),
            OsStr::new("-F"),
            path.as_os_str(),
            OsStr::new("--"),
            OsStr::new(name),
            OsStr::new(target),
        ];
        if self.run(&arguments)?.status.success() {
            Ok(())
        } else {
            Err(ApprovalTagError::CreateFailed)
        }
    }
}
