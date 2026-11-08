// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Declared change scope versus the candidate change footprint (FR-032).
//!
//! [`verify_scope`] is pure: it compares a declared scope with an observed set
//! of changed paths and classifies the declaration as complete, a false scope
//! or incomplete. It never reads a repository; the binary's git adapter
//! computes the [`Footprint`].

use std::collections::BTreeSet;

use serde::Serialize;
use thiserror::Error;

use crate::content_rights::is_normalized_relative_path;

/// Most scope entries a declaration may carry.
pub const MAX_SCOPE_ENTRIES: usize = 4096;
/// Most changed paths a footprint may carry.
pub const MAX_CHANGED_PATHS: usize = 100_000;
/// Longest accepted path in bytes.
pub const MAX_PATH_BYTES: usize = 4096;

/// The observed change footprint of a candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Footprint {
    /// Changed paths that were observed; `truncated` says the set is a prefix
    /// of the real one.
    Observed {
        /// Changed repository-relative paths, in any order.
        paths: Vec<String>,
        /// Whether the observation was cut short.
        truncated: bool,
    },
    /// The footprint could not be observed at all.
    Unavailable,
}

/// Outcome class of a scope verification.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopeStatus {
    /// Every changed path is covered and the footprint is fully observed.
    Complete,
    /// At least one changed path is not covered by the declaration.
    FalseScope,
    /// No uncovered path was seen, but the footprint is truncated or unavailable.
    Incomplete,
}

/// Result of comparing a declared scope with a footprint.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ScopeVerdict {
    /// Outcome class.
    pub status: ScopeStatus,
    /// Changed paths no scope entry covers, sorted and unique.
    pub uncovered_paths: Vec<String>,
    /// Scope entries that cover no changed path, sorted and unique; empty unless
    /// the footprint was fully observed.
    pub unused_scope: Vec<String>,
    /// Whether the observed footprint was cut short.
    pub truncated: bool,
    /// Whether a footprint could be observed at all.
    pub footprint_available: bool,
}

/// Typed refusal that produces no verdict.
#[derive(Debug, Eq, Error, PartialEq)]
pub enum ScopeError {
    /// A scope entry is not a normalized repository-relative path.
    #[error("scope entry {index} is not a normalized repository-relative path")]
    InvalidScopeEntry {
        /// Position of the entry in the declaration.
        index: usize,
    },
    /// Two scope entries are equal.
    #[error("scope entry {index} duplicates an earlier entry")]
    DuplicateScopeEntry {
        /// Position of the later entry.
        index: usize,
    },
    /// The declaration has more than [`MAX_SCOPE_ENTRIES`] entries.
    #[error("declared scope exceeds {MAX_SCOPE_ENTRIES} entries")]
    ScopeTooLarge,
    /// A changed path is not a normalized repository-relative path.
    #[error("changed path {index} is not a normalized repository-relative path")]
    InvalidChangedPath {
        /// Position of the path in the footprint.
        index: usize,
    },
    /// The footprint has more than [`MAX_CHANGED_PATHS`] paths.
    #[error("footprint exceeds {MAX_CHANGED_PATHS} paths")]
    FootprintTooLarge,
}

impl ScopeError {
    /// Stable machine error code of the command that surfaces this refusal.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidScopeEntry { .. }
            | Self::DuplicateScopeEntry { .. }
            | Self::ScopeTooLarge => "scope_footprint_scope_invalid",
            Self::InvalidChangedPath { .. } | Self::FootprintTooLarge => {
                "scope_footprint_changed_path_invalid"
            }
        }
    }
}

fn valid_path(path: &str) -> bool {
    path.len() <= MAX_PATH_BYTES && is_normalized_relative_path(path)
}

/// The declared entries that cover `path`: the path itself and each of its
/// whole-segment prefixes. A textual prefix that ends mid-segment never covers.
fn covering_entries<'a>(entries: &BTreeSet<&'a str>, path: &str) -> Vec<&'a str> {
    path.match_indices('/')
        .map(|(index, _)| &path[..index])
        .chain(std::iter::once(path))
        .filter_map(|candidate| entries.get(candidate).copied())
        .collect()
}

/// Compares `declared_scope` with `footprint` and classifies the declaration.
///
/// # Errors
///
/// Returns [`ScopeError`] without a verdict when a scope entry or changed path
/// is not a normalized repository-relative path, the scope has a duplicate
/// entry, or either input exceeds its bound.
pub fn verify_scope(
    declared_scope: &[String],
    footprint: &Footprint,
) -> Result<ScopeVerdict, ScopeError> {
    if declared_scope.len() > MAX_SCOPE_ENTRIES {
        return Err(ScopeError::ScopeTooLarge);
    }
    let mut entries = BTreeSet::new();
    for (index, entry) in declared_scope.iter().enumerate() {
        if !valid_path(entry) {
            return Err(ScopeError::InvalidScopeEntry { index });
        }
        if !entries.insert(entry.as_str()) {
            return Err(ScopeError::DuplicateScopeEntry { index });
        }
    }
    let Footprint::Observed { paths, truncated } = footprint else {
        return Ok(ScopeVerdict {
            status: ScopeStatus::Incomplete,
            uncovered_paths: Vec::new(),
            unused_scope: Vec::new(),
            truncated: false,
            footprint_available: false,
        });
    };
    if paths.len() > MAX_CHANGED_PATHS {
        return Err(ScopeError::FootprintTooLarge);
    }
    let mut changed = BTreeSet::new();
    for (index, path) in paths.iter().enumerate() {
        if !valid_path(path) {
            return Err(ScopeError::InvalidChangedPath { index });
        }
        changed.insert(path.as_str());
    }
    let mut used = BTreeSet::new();
    let mut uncovered_paths = Vec::new();
    for path in &changed {
        let covering = covering_entries(&entries, path);
        if covering.is_empty() {
            uncovered_paths.push((*path).to_owned());
        }
        used.extend(covering);
    }
    // Over-declaration is only knowable when every changed path was seen.
    let unused_scope = if *truncated {
        Vec::new()
    } else {
        entries
            .difference(&used)
            .map(|entry| (*entry).to_owned())
            .collect()
    };
    let status = if !uncovered_paths.is_empty() {
        ScopeStatus::FalseScope
    } else if *truncated {
        ScopeStatus::Incomplete
    } else {
        ScopeStatus::Complete
    };
    Ok(ScopeVerdict {
        status,
        uncovered_paths,
        unused_scope,
        truncated: *truncated,
        footprint_available: true,
    })
}
