// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Git adapter for the `scope-footprint` command (FR-032).
//!
//! The adapter computes the candidate change footprint with `git diff
//! --name-status -z` between two resolved commits and hands it to the pure
//! [`verify_scope`]. It owns no coverage semantics and never writes to the
//! repository.

use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    time::Duration,
};

use engineering_assurance::scope_footprint::{
    Footprint, MAX_CHANGED_PATHS, ScopeError, ScopeVerdict, verify_scope,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::process_host::{self, ProcessError, ProcessLimits};

const REQUEST_PROTOCOL: &str = "engineering-assurance.scope-footprint-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.scope-footprint-result/v1";
const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const GIT_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_REVISION_BYTES: usize = 256;

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
    base: String,
    candidate: String,
    scope: Vec<String>,
}

#[derive(Serialize)]
struct ResultLine<'a> {
    protocol: &'static str,
    #[serde(flatten)]
    verdict: &'a ScopeVerdict,
}

/// Typed refusal that produces no verdict.
#[derive(Debug, Error)]
pub(crate) enum ScopeFootprintError {
    /// The request is not readable JSON.
    #[error("scope-footprint request is not readable JSON")]
    Unreadable,
    /// The request is JSON that is not an exact request document.
    #[error("scope-footprint request is not an exact scope-footprint request")]
    Invalid,
    /// A revision is empty, option-like or contains whitespace or controls.
    #[error("scope-footprint revision is not a plain revision name")]
    Revision,
    /// The declared scope or the observed footprint breaks a rule.
    #[error("{0}")]
    Scope(#[from] ScopeError),
    /// The verdict cannot be encoded.
    #[error("scope-footprint result cannot be encoded")]
    ResultEncoding,
}

impl ScopeFootprintError {
    /// Stable machine error code.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "scope_footprint_request_unreadable",
            Self::Invalid => "scope_footprint_request_invalid",
            Self::Revision => "scope_footprint_revision_invalid",
            Self::Scope(error) => error.code(),
            Self::ResultEncoding => "scope_footprint_result_encoding_failed",
        }
    }
}

/// Runs one request document and returns the result line.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, ScopeFootprintError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| ScopeFootprintError::Unreadable)?;
    let request: Request =
        serde_json::from_value(value).map_err(|_| ScopeFootprintError::Invalid)?;
    if request.protocol != REQUEST_PROTOCOL || !request.repository.is_absolute() {
        return Err(ScopeFootprintError::Invalid);
    }
    if !plain_revision(&request.base) || !plain_revision(&request.candidate) {
        return Err(ScopeFootprintError::Revision);
    }
    // An invalid declaration is refused before git starts.
    verify_scope(&request.scope, &Footprint::Unavailable)?;
    let footprint = observe(&request.repository, &request.base, &request.candidate);
    let verdict = verify_scope(&request.scope, &footprint)?;
    let mut line = serde_json::to_vec(&ResultLine {
        protocol: RESULT_PROTOCOL,
        verdict: &verdict,
    })
    .map_err(|_| ScopeFootprintError::ResultEncoding)?;
    line.push(b'\n');
    Ok(line)
}

fn plain_revision(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_REVISION_BYTES
        && !value.starts_with('-')
        && !value.chars().any(|c| c.is_control() || c.is_whitespace())
}

/// Observes the footprint; any failure to do so is `Unavailable`.
fn observe(repository: &Path, base: &str, candidate: &str) -> Footprint {
    let Some(base_commit) = resolve(repository, base) else {
        return Footprint::Unavailable;
    };
    let Some(candidate_commit) = resolve(repository, candidate) else {
        return Footprint::Unavailable;
    };
    let arguments = [
        "diff",
        "--name-status",
        "-z",
        "--find-renames",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        base_commit.as_str(),
        candidate_commit.as_str(),
        "--",
    ];
    footprint_from(git(repository, &arguments))
}

/// Maps the diff invocation to a footprint: output over the capture bound is a
/// truncated footprint, every other failure an unavailable one.
fn footprint_from(result: Result<process_host::CompletedProcess, ProcessError>) -> Footprint {
    match result {
        Ok(output) if output.status.success() => parse_name_status(&output.stdout),
        Err(ProcessError::OutputTooLarge { .. }) => Footprint::Observed {
            paths: Vec::new(),
            truncated: true,
        },
        Ok(_) | Err(_) => Footprint::Unavailable,
    }
}

fn resolve(repository: &Path, revision: &str) -> Option<String> {
    let peeled = format!("{revision}^{{commit}}");
    let output = git(
        repository,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &peeled,
        ],
    )
    .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let id = text.trim_end_matches('\n');
    (!id.is_empty() && id.bytes().all(|b| b.is_ascii_hexdigit())).then(|| id.to_owned())
}

fn git(
    repository: &Path,
    arguments: &[&str],
) -> Result<process_host::CompletedProcess, ProcessError> {
    let arguments: Vec<&OsStr> = arguments.iter().map(OsStr::new).collect();
    let removed: Vec<&OsStr> = REMOVED_ENVIRONMENT.iter().map(OsStr::new).collect();
    let environment: [(OsString, OsString); 3] = [
        ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
        ("GIT_CONFIG_GLOBAL".into(), "/dev/null".into()),
        ("GIT_OPTIONAL_LOCKS".into(), "0".into()),
    ];
    let environment: Vec<(&OsStr, &OsStr)> = environment
        .iter()
        .map(|(key, value)| (key.as_os_str(), value.as_os_str()))
        .collect();
    process_host::run_configured(
        OsStr::new("git"),
        &arguments,
        Some(repository),
        &environment,
        &removed,
        ProcessLimits {
            timeout: GIT_TIMEOUT,
            max_output_bytes: MAX_OUTPUT_BYTES,
        },
    )
}

/// Parses NUL-separated `--name-status` records; a rename or copy lists both
/// paths. Output that does not parse yields an unavailable footprint.
fn parse_name_status(bytes: &[u8]) -> Footprint {
    // Every field, the last included, ends with a NUL.
    let Some(body) = bytes.strip_suffix(&[0]) else {
        return if bytes.is_empty() {
            Footprint::Observed {
                paths: Vec::new(),
                truncated: false,
            }
        } else {
            Footprint::Unavailable
        };
    };
    let mut fields = body.split(|b| *b == 0);
    let mut paths = BTreeSet::new();
    while let Some(status) = fields.next() {
        let count = match status.first() {
            Some(b'R' | b'C') => 2,
            Some(b'A' | b'D' | b'M' | b'T' | b'U' | b'X' | b'B') => 1,
            _ => return Footprint::Unavailable,
        };
        for _ in 0..count {
            let Some(path) = fields.next().and_then(|raw| std::str::from_utf8(raw).ok()) else {
                return Footprint::Unavailable;
            };
            paths.insert(path.to_owned());
        }
    }
    let truncated = paths.len() > MAX_CHANGED_PATHS;
    Footprint::Observed {
        paths: paths.into_iter().take(MAX_CHANGED_PATHS).collect(),
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    #[test]
    #[trace("TC-234", "FR-032-AC-3", "FR-032-CON-1")]
    fn oversized_git_output_is_a_truncated_footprint() {
        let footprint = footprint_from(Err(ProcessError::OutputTooLarge {
            stream: "stdout",
            limit: MAX_OUTPUT_BYTES,
        }));
        assert_eq!(
            footprint,
            Footprint::Observed {
                paths: Vec::new(),
                truncated: true
            }
        );
        let timed_out = footprint_from(Err(ProcessError::TimedOut {
            timeout: GIT_TIMEOUT,
        }));
        assert_eq!(timed_out, Footprint::Unavailable);
    }

    #[test]
    #[trace("TC-234", "FR-032-AC-3", "FR-032-CON-1")]
    fn path_count_over_the_bound_is_truncated_and_garbage_is_unavailable() {
        let mut bytes = Vec::new();
        for index in 0..=MAX_CHANGED_PATHS {
            bytes.extend_from_slice(format!("M\0f{index:06}\0").as_bytes());
        }
        let Footprint::Observed { paths, truncated } = parse_name_status(&bytes) else {
            panic!("a well-formed listing is observed");
        };
        assert!(truncated);
        assert_eq!(paths.len(), MAX_CHANGED_PATHS);
        assert_eq!(
            parse_name_status(b"M\0unterminated"),
            Footprint::Unavailable
        );
        assert_eq!(parse_name_status(b"Q\0x\0"), Footprint::Unavailable);
        assert_eq!(
            parse_name_status(b"R100\0only-one\0"),
            Footprint::Unavailable
        );
        assert_eq!(
            parse_name_status(b"R100\0old\0new\0A\0added\0"),
            Footprint::Observed {
                paths: vec!["added".into(), "new".into(), "old".into()],
                truncated: false
            }
        );
    }
}
