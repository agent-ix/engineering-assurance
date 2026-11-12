// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Git and `ssh-keygen` adapter for the `signature-verify` command (FR-036).
//!
//! The adapter runs `git verify-commit`, `git log --format=%G?%x00%GF`,
//! `git verify-tag` and, for a tag git rejected, `ssh-keygen -Y
//! check-novalidate`, through the shared process host. It reads exit statuses
//! and fixed signals only, hands them to the pure classifier, and never writes
//! to the repository.

use std::{
    ffi::{OsStr, OsString},
    fs::File,
    path::{Path, PathBuf},
    time::Duration,
};

use engineering_assurance::signature_verify::{
    CommitObservation, GitGrade, Reason, SignatureFormat, SshCheck, TagObservation, TargetKind,
    Verdict, classify_commit, classify_tag, commit_signature_format, is_plain_revision,
    split_tag_object,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::process_host::{self, CompletedProcess, ProcessError, ProcessLimits};

const REQUEST_PROTOCOL: &str = "engineering-assurance.signature-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.signature-verify-result/v1";
const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(60);

/// Repository-selecting and configuration variables git must not inherit.
const REMOVED_ENVIRONMENT: [&str; 12] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_EXTERNAL_DIFF",
    "GIT_DIFF_OPTS",
    "GIT_NAMESPACE",
    "GIT_COMMON_DIR",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_REPLACE_REF_BASE",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    repository: PathBuf,
    revision: String,
    kind: TargetKind,
    allowed_signers: PathBuf,
}

#[derive(Serialize)]
struct ResultLine<'a> {
    protocol: &'static str,
    #[serde(flatten)]
    verdict: &'a Verdict,
}

/// Typed refusal that produces no verdict.
#[derive(Debug, Error)]
pub(crate) enum SignatureVerifyError {
    /// The request is not readable JSON.
    #[error("signature-verify request is not readable JSON")]
    Unreadable,
    /// The request is JSON that is not an exact request document.
    #[error("signature-verify request is not an exact signature-verify request")]
    Invalid,
    /// The revision is empty, too long, option-like or contains whitespace or controls.
    #[error("signature-verify revision is not a plain revision name")]
    Revision,
    /// The verdict cannot be encoded.
    #[error("signature-verify result cannot be encoded")]
    ResultEncoding,
}

impl SignatureVerifyError {
    /// Stable machine error code.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "signature_verify_request_unreadable",
            Self::Invalid => "signature_verify_request_invalid",
            Self::Revision => "signature_verify_revision_invalid",
            Self::ResultEncoding => "signature_verify_result_encoding_failed",
        }
    }
}

/// Runs one request document and returns the result line.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, SignatureVerifyError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| SignatureVerifyError::Unreadable)?;
    let request: Request =
        serde_json::from_value(value).map_err(|_| SignatureVerifyError::Invalid)?;
    if request.protocol != REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.allowed_signers.is_absolute()
    {
        return Err(SignatureVerifyError::Invalid);
    }
    if !is_plain_revision(&request.revision) {
        return Err(SignatureVerifyError::Revision);
    }
    let git = Git {
        repository: &request.repository,
        allowed_signers: &request.allowed_signers,
    };
    let observed = match request.kind {
        TargetKind::Commit => git.commit(&request.revision),
        TargetKind::Tag => git.tag(&request.revision),
    };
    let verdict = observed.unwrap_or_else(|reason| Verdict::unverifiable(request.kind, reason));
    let mut line = serde_json::to_vec(&ResultLine {
        protocol: RESULT_PROTOCOL,
        verdict: &verdict,
    })
    .map_err(|_| SignatureVerifyError::ResultEncoding)?;
    line.push(b'\n');
    Ok(line)
}

/// Maps a process-mechanics failure to the reason it makes the object
/// unverifiable; `not_started` is the reason for a program that cannot start.
const fn mechanics_reason(error: &ProcessError, not_started: Reason) -> Reason {
    match error {
        ProcessError::Unavailable { .. } => not_started,
        ProcessError::TimedOut { .. } => Reason::Timeout,
        ProcessError::OutputTooLarge { .. } => Reason::OutputExceeded,
        ProcessError::PipeUnavailable { .. }
        | ProcessError::Observation { .. }
        | ProcessError::OutputUnreadable { .. } => Reason::UnrecognizedOutput,
    }
}

struct Git<'a> {
    repository: &'a Path,
    allowed_signers: &'a Path,
}

impl Git<'_> {
    fn commit(&self, revision: &str) -> Result<Verdict, Reason> {
        self.require_repository()?;
        let id = self.resolve(&format!("{revision}^{{commit}}"))?;
        let object = self.successful(&["cat-file", "commit", &id])?;
        let format = commit_signature_format(&object);
        let git = if format == Some(SignatureFormat::Other) {
            None
        } else {
            Some(self.grade(&id)?)
        };
        Ok(classify_commit(&CommitObservation { format, git }))
    }

    fn grade(&self, id: &str) -> Result<GitGrade, Reason> {
        let verify_accepted = self.run(&["verify-commit", id])?.status.success();
        let log = self.successful(&["log", "-1", "--format=%G?%x00%GF", id])?;
        let text = String::from_utf8(log).map_err(|_| Reason::UnrecognizedOutput)?;
        let (grade, fingerprint) = text
            .trim_end_matches('\n')
            .split_once('\0')
            .ok_or(Reason::UnrecognizedOutput)?;
        let mut codes = grade.chars();
        let (Some(grade), None) = (codes.next(), codes.next()) else {
            return Err(Reason::UnrecognizedOutput);
        };
        Ok(GitGrade {
            verify_accepted,
            grade,
            fingerprint: (!fingerprint.is_empty()).then(|| fingerprint.to_owned()),
        })
    }

    fn tag(&self, revision: &str) -> Result<Verdict, Reason> {
        self.require_repository()?;
        let id = self.resolve(revision)?;
        let kind = self.successful(&["cat-file", "-t", &id])?;
        let observation = match kind.as_slice() {
            b"commit\n" => TagObservation::Lightweight,
            b"tag\n" => self.annotated_tag(&id)?,
            _ => return Err(Reason::RevisionUnresolved),
        };
        Ok(classify_tag(&observation))
    }

    fn annotated_tag(&self, id: &str) -> Result<TagObservation, Reason> {
        let object = self.successful(&["cat-file", "tag", id])?;
        let Some(split) = split_tag_object(&object) else {
            return Ok(TagObservation::NoSignature);
        };
        if split.format == SignatureFormat::Other {
            return Ok(TagObservation::Signature {
                format: SignatureFormat::Other,
                verify_accepted: false,
                check: None,
            });
        }
        let verify_accepted = self.run(&["verify-tag", id])?.status.success();
        let check = if verify_accepted {
            None
        } else {
            Some(ssh_check(split.signature, split.payload)?)
        };
        Ok(TagObservation::Signature {
            format: SignatureFormat::Ssh,
            verify_accepted,
            check,
        })
    }

    fn require_repository(&self) -> Result<(), Reason> {
        if !self.repository.is_dir() {
            return Err(Reason::RepositoryUnavailable);
        }
        if self.run(&["rev-parse", "--git-dir"])?.status.success() {
            Ok(())
        } else {
            Err(Reason::RepositoryUnavailable)
        }
    }

    /// Resolves `expression` to a full object id.
    fn resolve(&self, expression: &str) -> Result<String, Reason> {
        let output = self.run(&[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            expression,
        ])?;
        let id = output
            .status
            .success()
            .then(|| String::from_utf8(output.stdout).ok())
            .flatten()
            .map(|text| text.trim_end_matches('\n').to_owned())
            .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_hexdigit()));
        id.ok_or(Reason::RevisionUnresolved)
    }

    /// Standard output of a git command that must exit with status 0.
    fn successful(&self, arguments: &[&str]) -> Result<Vec<u8>, Reason> {
        let output = self.run(arguments)?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(Reason::UnrecognizedOutput)
        }
    }

    /// Runs git with only the allowed-signers setting from the request.
    fn run(&self, arguments: &[&str]) -> Result<CompletedProcess, Reason> {
        let mut setting = OsString::from("gpg.ssh.allowedSignersFile=");
        setting.push(self.allowed_signers);
        let mut all: Vec<&OsStr> = vec![
            OsStr::new("-c"),
            &setting,
            OsStr::new("-c"),
            OsStr::new("gpg.ssh.program=ssh-keygen"),
        ];
        all.extend(arguments.iter().map(OsStr::new));
        let removed: Vec<&OsStr> = REMOVED_ENVIRONMENT.iter().map(OsStr::new).collect();
        let environment: [(&OsStr, &OsStr); 3] = [
            (OsStr::new("GIT_CONFIG_NOSYSTEM"), OsStr::new("1")),
            (OsStr::new("GIT_CONFIG_GLOBAL"), OsStr::new("/dev/null")),
            (OsStr::new("GIT_OPTIONAL_LOCKS"), OsStr::new("0")),
        ];
        process_host::run_configured(
            OsStr::new("git"),
            &all,
            Some(self.repository),
            &environment,
            &removed,
            limits(),
        )
        .map_err(|error| mechanics_reason(&error, Reason::GitNotStarted))
    }
}

const fn limits() -> ProcessLimits {
    ProcessLimits {
        timeout: PROCESS_TIMEOUT,
        max_output_bytes: MAX_OUTPUT_BYTES,
    }
}

/// Checks `signature` over `payload` with `ssh-keygen -Y check-novalidate`,
/// passing both parts unchanged.
fn ssh_check(signature: &[u8], payload: &[u8]) -> Result<SshCheck, Reason> {
    let directory = tempfile::tempdir().map_err(|_| Reason::UnrecognizedOutput)?;
    let signature_path = directory.path().join("signature");
    let payload_path = directory.path().join("payload");
    std::fs::write(&signature_path, signature).map_err(|_| Reason::UnrecognizedOutput)?;
    std::fs::write(&payload_path, payload).map_err(|_| Reason::UnrecognizedOutput)?;
    let stdin = File::open(&payload_path).map_err(|_| Reason::UnrecognizedOutput)?;
    let arguments = [
        OsStr::new("-Y"),
        OsStr::new("check-novalidate"),
        OsStr::new("-n"),
        OsStr::new("git"),
        OsStr::new("-s"),
        signature_path.as_os_str(),
    ];
    let result = process_host::run_configured_stdin(
        OsStr::new("ssh-keygen"),
        &arguments,
        Some(directory.path()),
        &[],
        &[],
        stdin,
        limits(),
    );
    match result {
        Ok(output) if output.status.success() => Ok(SshCheck::Accepted),
        Ok(_) => Ok(SshCheck::Rejected),
        Err(ProcessError::Unavailable { .. }) => Ok(SshCheck::NotStarted),
        Err(error) => Err(mechanics_reason(&error, Reason::UnrecognizedOutput)),
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    #[test]
    #[trace("TC-255", "FR-036-AC-6")]
    fn timeout_and_oversized_output_are_unverifiable_reasons() {
        let timed_out = mechanics_reason(
            &ProcessError::TimedOut {
                timeout: PROCESS_TIMEOUT,
            },
            Reason::GitNotStarted,
        );
        assert_eq!(timed_out, Reason::Timeout);
        let oversized = mechanics_reason(
            &ProcessError::OutputTooLarge {
                stream: "stdout",
                limit: MAX_OUTPUT_BYTES,
            },
            Reason::GitNotStarted,
        );
        assert_eq!(oversized, Reason::OutputExceeded);
        let not_started = mechanics_reason(
            &ProcessError::Unavailable {
                detail: "missing".to_owned(),
            },
            Reason::GitNotStarted,
        );
        assert_eq!(not_started, Reason::GitNotStarted);
        let verdict = Verdict::unverifiable(TargetKind::Commit, timed_out);
        assert_ne!(
            verdict.status,
            engineering_assurance::signature_verify::Status::SignedValid
        );
    }
}
