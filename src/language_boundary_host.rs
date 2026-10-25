// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded Git revision selection and blob adapter for the language guard.

use std::{collections::BTreeSet, ffi::OsStr, path::Path, time::Duration};

use serde::Serialize;
use thiserror::Error;

use crate::{
    language_boundary::{self, Entry, Finding},
    process_host::{self, ProcessLimits},
};

const GIT_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_GIT_OUTPUT: usize = 32 * 1_048_576;
const MAX_ENTRIES: usize = 65_536;
const MAX_CANDIDATES: usize = 8_192;
const MAX_PATH: usize = 4_096;
const MAX_BLOB: usize = 2 * 1_048_576;
const MAX_TOTAL: usize = 64 * 1_048_576;

#[derive(Debug, Error)]
pub(crate) enum GuardError {
    #[error("repository root or identity is invalid")]
    RootInvalid,
    #[error("Git history is shallow, absent, or lacks a unique safe base")]
    BaseInvalid,
    #[error("Git command failed or exceeded its time or output limit")]
    GitFailure,
    #[error("Git tree response is malformed or exceeds a fixed bound")]
    TreeInvalid,
    #[error("selected executable population exceeds a fixed byte or entry bound")]
    PopulationTooLarge,
    #[error("language classification failed: {0}")]
    Classification(#[from] language_boundary::ClassificationError),
}

impl GuardError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::RootInvalid => "language_boundary_root_invalid",
            Self::BaseInvalid => "language_boundary_base_invalid",
            Self::GitFailure => "language_boundary_git_failed",
            Self::TreeInvalid => "language_boundary_tree_invalid",
            Self::PopulationTooLarge => "language_boundary_population_too_large",
            Self::Classification(_) => "language_boundary_classification_failed",
        }
    }
}

#[derive(Serialize)]
pub(crate) struct GuardResult {
    protocol: &'static str,
    repository: String,
    base_revision: String,
    head_revision: String,
    outcome: &'static str,
    findings: Vec<Finding>,
}

impl GuardResult {
    pub(crate) fn refused(&self) -> bool {
        !self.findings.is_empty()
    }
}

pub(crate) fn execute(
    root: &Path,
    repository: &str,
    explicit_base: Option<&str>,
    explicit_head: Option<&str>,
) -> Result<GuardResult, GuardError> {
    if repository == "build-chain"
        || repository.is_empty()
        || !repository
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(GuardError::RootInvalid);
    }
    let metadata = root
        .symlink_metadata()
        .map_err(|_| GuardError::RootInvalid)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(GuardError::RootInvalid);
    }
    let canonical = root.canonicalize().map_err(|_| GuardError::RootInvalid)?;
    if !root.is_dir()
        || git_text(&canonical, &["rev-parse", "--show-toplevel"])? != canonical.to_string_lossy()
    {
        return Err(GuardError::RootInvalid);
    }
    if git_text(&canonical, &["rev-parse", "--is-shallow-repository"])? != "false" {
        return Err(GuardError::BaseInvalid);
    }
    let head = if let Some(value) = explicit_head {
        if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(GuardError::BaseInvalid);
        }
        let selected = revision(&canonical, value)?;
        if selected != value.to_ascii_lowercase() {
            return Err(GuardError::BaseInvalid);
        }
        selected
    } else {
        revision(&canonical, "HEAD")?
    };
    let base = choose_base(&canonical, &head, explicit_base)?;
    let before = read_tree(&canonical, &base)?;
    let after = read_tree(&canonical, &head)?;
    let findings = language_boundary::classify_delta(repository, &before, &after)?;
    Ok(GuardResult {
        protocol: "engineering-assurance.language-boundary/v1",
        repository: repository.to_owned(),
        base_revision: base,
        head_revision: head,
        outcome: if findings.is_empty() {
            "accepted"
        } else {
            "refused"
        },
        findings,
    })
}

fn choose_base(root: &Path, head: &str, explicit: Option<&str>) -> Result<String, GuardError> {
    if let Some(value) = explicit {
        if value.len() != 40 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(GuardError::BaseInvalid);
        }
        let selected = revision(root, value)?;
        if selected != value.to_ascii_lowercase() {
            return Err(GuardError::BaseInvalid);
        }
        return merge_base(root, &selected, head);
    }
    let branch = git_text(root, &["symbolic-ref", "--quiet", "--short", "HEAD"])
        .map_err(|_| GuardError::BaseInvalid)?;
    if branch == "main" {
        return revision(root, "HEAD^");
    }
    let main = revision(root, "refs/remotes/origin/main")?;
    merge_base(root, &main, head)
}

fn merge_base(root: &Path, base: &str, head: &str) -> Result<String, GuardError> {
    let output = git_text(root, &["merge-base", "--all", base, head])
        .map_err(|_| GuardError::BaseInvalid)?;
    if output.lines().count() != 1 || output.len() != 40 {
        return Err(GuardError::BaseInvalid);
    }
    Ok(output)
}

fn revision(root: &Path, value: &str) -> Result<String, GuardError> {
    let commit = format!("{value}^{{commit}}");
    let output =
        git_text(root, &["rev-parse", "--verify", &commit]).map_err(|_| GuardError::BaseInvalid)?;
    if output.len() != 40 || !output.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GuardError::BaseInvalid);
    }
    Ok(output)
}

fn read_tree(root: &Path, revision: &str) -> Result<Vec<Entry>, GuardError> {
    let raw = git_bytes(
        root,
        &["ls-tree", "-r", "-z", "--full-tree", revision],
        MAX_GIT_OUTPUT,
    )?;
    if !raw.is_empty() && raw.last() != Some(&0) {
        return Err(GuardError::TreeInvalid);
    }
    let mut entries = Vec::new();
    let mut paths = BTreeSet::new();
    let mut total = 0usize;
    let mut count = 0usize;
    for item in
        raw[..raw.len().saturating_sub(usize::from(!raw.is_empty()))].split(|byte| *byte == 0)
    {
        count += 1;
        if count > MAX_ENTRIES {
            return Err(GuardError::TreeInvalid);
        }
        let separator = item
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or(GuardError::TreeInvalid)?;
        let (header, raw_path) = item.split_at(separator);
        let path = &raw_path[1..];
        let header = std::str::from_utf8(header).map_err(|_| GuardError::TreeInvalid)?;
        let path = std::str::from_utf8(path).map_err(|_| GuardError::TreeInvalid)?;
        if path.is_empty()
            || path.len() > MAX_PATH
            || path.starts_with('/')
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || !paths.insert(path.to_owned())
        {
            return Err(GuardError::TreeInvalid);
        }
        let mut fields = header.split_whitespace();
        let (Some(mode), Some(kind), Some(oid), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(GuardError::TreeInvalid);
        };
        if oid.len() != 40 || !oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(GuardError::TreeInvalid);
        }
        if kind == "commit" && mode == "160000" {
            entries.push(Entry {
                path: path.to_owned(),
                mode: mode.to_owned(),
                bytes: Vec::new(),
            });
            continue;
        }
        if kind != "blob" || !matches!(mode, "100644" | "100755" | "120000") {
            return Err(GuardError::TreeInvalid);
        }
        if !language_boundary::needs_content(path, mode) {
            continue;
        }
        if entries.len() == MAX_CANDIDATES {
            return Err(GuardError::PopulationTooLarge);
        }
        let bytes = git_bytes(root, &["cat-file", "blob", oid], MAX_BLOB)?;
        total = total
            .checked_add(bytes.len())
            .ok_or(GuardError::PopulationTooLarge)?;
        if total > MAX_TOTAL {
            return Err(GuardError::PopulationTooLarge);
        }
        entries.push(Entry {
            path: path.to_owned(),
            mode: mode.to_owned(),
            bytes,
        });
    }
    Ok(entries)
}

fn git_text(root: &Path, args: &[&str]) -> Result<String, GuardError> {
    let bytes = git_bytes(root, args, 4096)?;
    let value = std::str::from_utf8(&bytes).map_err(|_| GuardError::GitFailure)?;
    Ok(value.trim_end_matches('\n').to_owned())
}

fn git_bytes(root: &Path, args: &[&str], limit: usize) -> Result<Vec<u8>, GuardError> {
    let args = args
        .iter()
        .map(|value| OsStr::new(*value))
        .collect::<Vec<_>>();
    let output = process_host::run_configured(
        OsStr::new("git"),
        &args,
        Some(root),
        &[],
        &[],
        ProcessLimits {
            timeout: GIT_TIMEOUT,
            max_output_bytes: limit,
        },
    )
    .map_err(|_| GuardError::GitFailure)?;
    if !output.status.success() {
        return Err(GuardError::GitFailure);
    }
    Ok(output.stdout)
}
