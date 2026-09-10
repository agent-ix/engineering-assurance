// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Confined filesystem, environment, and Git adapter for content-rights policy.

use std::{
    collections::BTreeSet,
    env,
    ffi::{OsStr, OsString},
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use cap_std::{ambient_authority, fs::Dir};
use engineering_assurance::content_rights::{
    ContentEntryKind, ContentRightsCategory, ContentRightsError, ContentRightsFinding,
    ContentRightsTreeResult, inspect_content, split_protected_tokens,
};
use thiserror::Error;

use crate::process_host::{self, ProcessError, ProcessLimits};

const GIT_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_GIT_OUTPUT_BYTES: usize = 8_388_608;
const MAX_SELECTED_ENTRIES: usize = 65_536;
const MAX_PATH_BYTES: usize = 4_096;
const MAX_FILE_BYTES: usize = 8_388_608;
const MAX_TOTAL_BYTES: usize = 67_108_864;
const MAX_PROTECTED_TOKEN_BYTES: usize = 65_536;
const MAX_PROTECTED_TOKENS: usize = 256;
const MAX_PROTECTED_TOKEN_BYTES_EACH: usize = 4_096;

#[derive(Debug, Error)]
pub(crate) enum ContentRightsHostError {
    #[error("selected content-rights root is not a regular directory")]
    RootInvalid,
    #[error("selected content-rights root is not the Git worktree top level")]
    RootNotTopLevel,
    #[error("git is unavailable")]
    GitUnavailable,
    #[error("git did not terminate within {GIT_TIMEOUT:?}")]
    GitTimedOut,
    #[error("git output exceeded {MAX_GIT_OUTPUT_BYTES} bytes")]
    GitOutputTooLarge,
    #[error("git process observation failed")]
    GitObservationFailed,
    #[error("git command failed")]
    GitCommandFailed,
    #[error("git returned a malformed path population")]
    GitResponseInvalid,
    #[error("git selected more than {MAX_SELECTED_ENTRIES} entries")]
    EntryPopulationTooLarge,
    #[error("git selected a path longer than {MAX_PATH_BYTES} bytes")]
    EntryPathTooLong,
    #[error("git selected the same path more than once")]
    EntryDuplicate,
    #[error("a selected entry is missing, unstable, or has an unsupported file kind")]
    EntryInvalid,
    #[error("a selected file exceeds {MAX_FILE_BYTES} bytes")]
    FileTooLarge,
    #[error("the selected regular-file population exceeds {MAX_TOTAL_BYTES} bytes")]
    TotalBytesTooLarge,
    #[error("the protected-token environment value is not valid UTF-8")]
    ProtectedTokensNotUtf8,
    #[error("the protected-token environment value exceeds {MAX_PROTECTED_TOKEN_BYTES} bytes")]
    ProtectedTokensTooLarge,
    #[error("the protected-token population exceeds {MAX_PROTECTED_TOKENS} entries")]
    ProtectedTokenPopulationTooLarge,
    #[error("one protected token exceeds {MAX_PROTECTED_TOKEN_BYTES_EACH} bytes")]
    ProtectedTokenTooLarge,
    #[error("the content-rights policy could not classify the selected tree")]
    Policy,
}

impl ContentRightsHostError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::RootInvalid => "content_rights_root_invalid",
            Self::RootNotTopLevel => "content_rights_root_not_top_level",
            Self::GitUnavailable => "content_rights_git_unavailable",
            Self::GitTimedOut => "content_rights_git_timed_out",
            Self::GitOutputTooLarge => "content_rights_git_output_too_large",
            Self::GitObservationFailed => "content_rights_git_observation_failed",
            Self::GitCommandFailed => "content_rights_git_command_failed",
            Self::GitResponseInvalid => "content_rights_git_response_invalid",
            Self::EntryPopulationTooLarge => "content_rights_entry_population_too_large",
            Self::EntryPathTooLong => "content_rights_entry_path_too_long",
            Self::EntryDuplicate => "content_rights_entry_duplicate",
            Self::EntryInvalid => "content_rights_entry_invalid",
            Self::FileTooLarge => "content_rights_file_too_large",
            Self::TotalBytesTooLarge => "content_rights_total_bytes_too_large",
            Self::ProtectedTokensNotUtf8 => "content_rights_protected_tokens_not_utf8",
            Self::ProtectedTokensTooLarge => "content_rights_protected_tokens_too_large",
            Self::ProtectedTokenPopulationTooLarge => {
                "content_rights_protected_token_population_too_large"
            }
            Self::ProtectedTokenTooLarge => "content_rights_protected_token_too_large",
            Self::Policy => "content_rights_policy_invalid",
        }
    }
}

pub(crate) fn execute(root: &Path) -> Result<ContentRightsTreeResult, ContentRightsHostError> {
    let root = selected_root(root)?;
    require_git_top_level(&root)?;
    let selected = selected_paths(&root)?;
    let protected_tokens = protected_tokens()?;
    inspect_selected(&root, &selected, &protected_tokens)
}

fn selected_root(root: &Path) -> Result<PathBuf, ContentRightsHostError> {
    let metadata =
        std::fs::symlink_metadata(root).map_err(|_| ContentRightsHostError::RootInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ContentRightsHostError::RootInvalid);
    }
    std::fs::canonicalize(root).map_err(|_| ContentRightsHostError::RootInvalid)
}

fn require_git_top_level(root: &Path) -> Result<(), ContentRightsHostError> {
    let output = git(
        root,
        &[OsStr::new("rev-parse"), OsStr::new("--show-toplevel")],
    )?;
    if !output.status.success() {
        return Err(ContentRightsHostError::GitCommandFailed);
    }
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|_| ContentRightsHostError::GitResponseInvalid)?;
    let line = text
        .strip_suffix('\n')
        .ok_or(ContentRightsHostError::GitResponseInvalid)?;
    let top_level = line.strip_suffix('\r').unwrap_or(line);
    if top_level.is_empty() || top_level.contains(['\r', '\n']) {
        return Err(ContentRightsHostError::GitResponseInvalid);
    }
    let top_level =
        std::fs::canonicalize(top_level).map_err(|_| ContentRightsHostError::GitResponseInvalid)?;
    if top_level != root {
        return Err(ContentRightsHostError::RootNotTopLevel);
    }
    Ok(())
}

fn selected_paths(root: &Path) -> Result<Vec<String>, ContentRightsHostError> {
    let output = git(
        root,
        &[
            OsStr::new("ls-files"),
            OsStr::new("-z"),
            OsStr::new("--cached"),
            OsStr::new("--others"),
            OsStr::new("--exclude-standard"),
        ],
    )?;
    if !output.status.success() {
        return Err(ContentRightsHostError::GitCommandFailed);
    }
    parse_selected_paths(&output.stdout)
}

fn git(
    root: &Path,
    arguments: &[&OsStr],
) -> Result<process_host::CompletedProcess, ContentRightsHostError> {
    let mut complete = Vec::with_capacity(arguments.len().saturating_add(2));
    complete.push(OsStr::new("-C"));
    complete.push(root.as_os_str());
    complete.extend_from_slice(arguments);
    process_host::run(
        OsStr::new("git"),
        &complete,
        ProcessLimits {
            timeout: GIT_TIMEOUT,
            max_output_bytes: MAX_GIT_OUTPUT_BYTES,
        },
    )
    .map_err(|error| map_process_error(&error))
}

fn map_process_error(error: &ProcessError) -> ContentRightsHostError {
    match error {
        ProcessError::Unavailable { .. } => ContentRightsHostError::GitUnavailable,
        ProcessError::TimedOut { .. } => ContentRightsHostError::GitTimedOut,
        ProcessError::OutputTooLarge { .. } => ContentRightsHostError::GitOutputTooLarge,
        ProcessError::PipeUnavailable { .. } | ProcessError::OutputUnreadable { .. } => {
            ContentRightsHostError::GitObservationFailed
        }
        ProcessError::Observation { .. } => ContentRightsHostError::GitObservationFailed,
    }
}

fn parse_selected_paths(bytes: &[u8]) -> Result<Vec<String>, ContentRightsHostError> {
    let Some((terminator, population)) = bytes.split_last() else {
        return Ok(Vec::new());
    };
    if *terminator != 0 {
        return Err(ContentRightsHostError::GitResponseInvalid);
    }
    let mut selected = Vec::new();
    let mut unique = BTreeSet::new();
    for raw in population.split(|byte| *byte == 0) {
        if raw.is_empty() {
            return Err(ContentRightsHostError::GitResponseInvalid);
        }
        if selected.len() == MAX_SELECTED_ENTRIES {
            return Err(ContentRightsHostError::EntryPopulationTooLarge);
        }
        if raw.len() > MAX_PATH_BYTES {
            return Err(ContentRightsHostError::EntryPathTooLong);
        }
        let path = std::str::from_utf8(raw)
            .map_err(|_| ContentRightsHostError::GitResponseInvalid)?
            .to_owned();
        if !unique.insert(path.clone()) {
            return Err(ContentRightsHostError::EntryDuplicate);
        }
        selected.push(path);
    }
    selected.sort_unstable();
    Ok(selected)
}

pub(crate) fn protected_tokens() -> Result<Vec<String>, ContentRightsHostError> {
    parse_protected_tokens(env::var_os("ASSURANCE_PROTECTED_TOKENS"))
}

fn parse_protected_tokens(raw: Option<OsString>) -> Result<Vec<String>, ContentRightsHostError> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    let raw = raw
        .into_string()
        .map_err(|_| ContentRightsHostError::ProtectedTokensNotUtf8)?;
    if raw.len() > MAX_PROTECTED_TOKEN_BYTES {
        return Err(ContentRightsHostError::ProtectedTokensTooLarge);
    }
    let tokens = split_protected_tokens(&raw);
    if tokens.len() > MAX_PROTECTED_TOKENS {
        return Err(ContentRightsHostError::ProtectedTokenPopulationTooLarge);
    }
    if tokens
        .iter()
        .any(|token| token.len() > MAX_PROTECTED_TOKEN_BYTES_EACH)
    {
        return Err(ContentRightsHostError::ProtectedTokenTooLarge);
    }
    Ok(tokens)
}

fn inspect_selected(
    root: &Path,
    selected: &[String],
    protected_tokens: &[String],
) -> Result<ContentRightsTreeResult, ContentRightsHostError> {
    let directory = Dir::open_ambient_dir(root, ambient_authority())
        .map_err(|_| ContentRightsHostError::RootInvalid)?;
    let mut findings = Vec::new();
    let mut inspected_entries = 0usize;
    let mut total_bytes = 0usize;
    for relative in selected {
        let path_preflight = classify(relative, ContentEntryKind::File, &[], protected_tokens)?;
        if path_preflight
            .iter()
            .any(|finding| finding.category == ContentRightsCategory::PathInvalid)
        {
            findings.extend(path_preflight);
            continue;
        }
        let metadata = directory
            .symlink_metadata(relative)
            .map_err(|_| ContentRightsHostError::EntryInvalid)?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            inspected_entries = inspected_entries
                .checked_add(1)
                .ok_or(ContentRightsHostError::EntryPopulationTooLarge)?;
            findings.extend(classify(
                relative,
                ContentEntryKind::SymbolicLink,
                &[],
                protected_tokens,
            )?);
            continue;
        }
        if file_type.is_dir() {
            continue;
        }
        if !file_type.is_file() {
            return Err(ContentRightsHostError::EntryInvalid);
        }
        let bytes = read_file(&directory, relative)?;
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .ok_or(ContentRightsHostError::TotalBytesTooLarge)?;
        if total_bytes > MAX_TOTAL_BYTES {
            return Err(ContentRightsHostError::TotalBytesTooLarge);
        }
        inspected_entries = inspected_entries
            .checked_add(1)
            .ok_or(ContentRightsHostError::EntryPopulationTooLarge)?;
        findings.extend(classify(
            relative,
            ContentEntryKind::File,
            &bytes,
            protected_tokens,
        )?);
    }
    Ok(ContentRightsTreeResult::new(inspected_entries, findings))
}

fn read_file(directory: &Dir, relative: &str) -> Result<Vec<u8>, ContentRightsHostError> {
    let file = directory
        .open(relative)
        .map_err(|_| ContentRightsHostError::EntryInvalid)?;
    let metadata = file
        .metadata()
        .map_err(|_| ContentRightsHostError::EntryInvalid)?;
    if !metadata.is_file() {
        return Err(ContentRightsHostError::EntryInvalid);
    }
    if metadata.len() > u64::try_from(MAX_FILE_BYTES).unwrap_or(u64::MAX) {
        return Err(ContentRightsHostError::FileTooLarge);
    }
    let limit = u64::try_from(MAX_FILE_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::new();
    file.take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| ContentRightsHostError::EntryInvalid)?;
    if bytes.len() > MAX_FILE_BYTES {
        return Err(ContentRightsHostError::FileTooLarge);
    }
    let observed_length =
        u64::try_from(bytes.len()).map_err(|_| ContentRightsHostError::EntryInvalid)?;
    if metadata.len() != observed_length {
        return Err(ContentRightsHostError::EntryInvalid);
    }
    Ok(bytes)
}

fn classify(
    path: &str,
    kind: ContentEntryKind,
    bytes: &[u8],
    protected_tokens: &[String],
) -> Result<Vec<ContentRightsFinding>, ContentRightsHostError> {
    inspect_content(path, kind, bytes, protected_tokens).map_err(map_policy_error)
}

fn map_policy_error(_: ContentRightsError) -> ContentRightsHostError {
    ContentRightsHostError::Policy
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use ix_trace_rs::trace;

    use super::*;

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn tc_111_git_path_population_boundaries_fail_closed() {
        let exact = (0..MAX_SELECTED_ENTRIES)
            .flat_map(|index| format!("p{index}\0").into_bytes())
            .collect::<Vec<_>>();
        assert_eq!(
            parse_selected_paths(&exact)
                .expect("exact selected-entry limit must be admitted")
                .len(),
            MAX_SELECTED_ENTRIES
        );

        let mut over = exact;
        over.extend_from_slice(b"over\0");
        assert!(matches!(
            parse_selected_paths(&over),
            Err(ContentRightsHostError::EntryPopulationTooLarge)
        ));
        assert!(matches!(
            parse_selected_paths(b"duplicate\0duplicate\0"),
            Err(ContentRightsHostError::EntryDuplicate)
        ));
        assert!(matches!(
            parse_selected_paths(b"unterminated"),
            Err(ContentRightsHostError::GitResponseInvalid)
        ));
        assert!(matches!(
            parse_selected_paths(b"first\0\0"),
            Err(ContentRightsHostError::GitResponseInvalid)
        ));
        assert!(matches!(
            parse_selected_paths(&[0xff, 0]),
            Err(ContentRightsHostError::GitResponseInvalid)
        ));

        let mut long = vec![b'a'; MAX_PATH_BYTES.saturating_add(1)];
        long.push(0);
        assert!(matches!(
            parse_selected_paths(&long),
            Err(ContentRightsHostError::EntryPathTooLong)
        ));
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn tc_111_protected_token_boundaries_preserve_logical_lines() {
        let exact = "x".repeat(MAX_PROTECTED_TOKEN_BYTES_EACH);
        assert_eq!(
            parse_protected_tokens(Some(OsString::from(&exact)))
                .expect("exact token limit must be admitted"),
            vec![exact]
        );
        assert!(matches!(
            parse_protected_tokens(Some(OsString::from(
                "x".repeat(MAX_PROTECTED_TOKEN_BYTES_EACH.saturating_add(1))
            ))),
            Err(ContentRightsHostError::ProtectedTokenTooLarge)
        ));

        let exact_population = (0..MAX_PROTECTED_TOKENS)
            .map(|index| format!("t{index}"))
            .collect::<Vec<_>>()
            .join("\u{2028}");
        assert_eq!(
            parse_protected_tokens(Some(OsString::from(&exact_population)))
                .expect("exact token population must be admitted")
                .len(),
            MAX_PROTECTED_TOKENS
        );
        let over_population = format!("{exact_population}\ntoo-many");
        assert!(matches!(
            parse_protected_tokens(Some(OsString::from(over_population))),
            Err(ContentRightsHostError::ProtectedTokenPopulationTooLarge)
        ));
        assert!(matches!(
            parse_protected_tokens(Some(OsString::from(
                "x".repeat(MAX_PROTECTED_TOKEN_BYTES.saturating_add(1))
            ))),
            Err(ContentRightsHostError::ProtectedTokensTooLarge)
        ));
    }

    #[cfg(unix)]
    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn tc_111_non_utf8_protected_tokens_fail_closed() {
        use std::os::unix::ffi::OsStringExt;

        assert!(matches!(
            parse_protected_tokens(Some(OsString::from_vec(vec![0xff]))),
            Err(ContentRightsHostError::ProtectedTokensNotUtf8)
        ));
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn tc_111_bounded_process_failures_keep_distinct_tree_codes() {
        let cases = [
            (
                ProcessError::Unavailable {
                    detail: "fixture".to_owned(),
                },
                "content_rights_git_unavailable",
            ),
            (
                ProcessError::TimedOut {
                    timeout: Duration::from_secs(1),
                },
                "content_rights_git_timed_out",
            ),
            (
                ProcessError::OutputTooLarge {
                    stream: "stdout",
                    limit: 1,
                },
                "content_rights_git_output_too_large",
            ),
            (
                ProcessError::Observation {
                    detail: "fixture".to_owned(),
                },
                "content_rights_git_observation_failed",
            ),
        ];
        for (failure, expected) in cases {
            assert_eq!(map_process_error(&failure).code(), expected);
        }
    }
}
