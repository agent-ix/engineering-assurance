// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure comparison of expected and observed package file-member names.
//!
//! Archive decoding, member-kind validation, package construction,
//! installation, filesystem traversal, and distribution-format selection
//! belong to explicit adapters outside this module.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::Serialize;
use thiserror::Error;

/// Maximum number of expected or observed members accepted by one comparison.
pub const MAX_PACKAGE_MEMBERS: usize = 65_536;

/// Maximum UTF-8 byte length of one package member name.
pub const MAX_PACKAGE_MEMBER_PATH_BYTES: usize = 4_096;

/// Whether an observed package has exactly the expected file-member names.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageMembershipOutcome {
    /// Every expected member occurs exactly once and no other member occurs.
    Accepted,
    /// At least one observed membership defect exists.
    Withheld,
}

/// Closed package-membership finding taxonomy in canonical output order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageMembershipCategory {
    /// An observed member name is unsafe or not lexically normalized.
    ActualPathInvalid,
    /// A safe observed member name occurs more than once.
    ActualPathDuplicate,
    /// A safe observed member is absent from the expected allowlist.
    UnexpectedMember,
    /// An expected member is absent from the observed population.
    MissingMember,
}

impl PackageMembershipCategory {
    /// Return the stable machine spelling for this finding category.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ActualPathInvalid => "actual_path_invalid",
            Self::ActualPathDuplicate => "actual_path_duplicate",
            Self::UnexpectedMember => "unexpected_member",
            Self::MissingMember => "missing_member",
        }
    }
}

impl fmt::Display for PackageMembershipCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One deterministic package-membership finding.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageMembershipFinding {
    /// Safe normalized member name, absent when an observed name is unsafe.
    pub path: Option<String>,
    /// Closed reason package acceptance is withheld.
    pub category: PackageMembershipCategory,
}

/// Result of comparing one observed file-member population with a policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageMembershipResult {
    /// Accepted only when the finding list is empty.
    pub outcome: PackageMembershipOutcome,
    /// Findings in canonical category and lexical path order.
    pub findings: Vec<PackageMembershipFinding>,
}

/// Typed refusal raised before a package-membership result can be produced.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum PackageMembershipError {
    /// The expected allowlist exceeds the fixed population ceiling.
    #[error("expected package member population exceeds {MAX_PACKAGE_MEMBERS}")]
    ExpectedPopulationTooLarge,
    /// The observed population exceeds the fixed population ceiling.
    #[error("observed package member population exceeds {MAX_PACKAGE_MEMBERS}")]
    ObservedPopulationTooLarge,
    /// One expected member name is unsafe or not lexically normalized.
    #[error("expected package member path is invalid")]
    ExpectedPathInvalid,
    /// One safe expected member name occurs more than once.
    #[error("expected package member path is duplicated: {path}")]
    ExpectedPathDuplicate {
        /// Duplicated safe normalized expected member name.
        path: String,
    },
}

impl PackageMembershipError {
    /// Return the stable machine category for this refusal.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ExpectedPopulationTooLarge => "expected_package_population_too_large",
            Self::ObservedPopulationTooLarge => "observed_package_population_too_large",
            Self::ExpectedPathInvalid => "expected_package_path_invalid",
            Self::ExpectedPathDuplicate { .. } => "expected_package_path_duplicate",
        }
    }
}

/// Validated expected package file-member policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageMembershipPolicy {
    expected: BTreeSet<String>,
}

impl PackageMembershipPolicy {
    /// Validate and retain an exact expected file-member allowlist.
    ///
    /// # Errors
    ///
    /// Refuses an over-limit population, an unsafe/non-normalized name, or a
    /// duplicate expected member. An unsafe input name is never returned by the
    /// error.
    pub fn new(expected: &[String]) -> Result<Self, PackageMembershipError> {
        if expected.len() > MAX_PACKAGE_MEMBERS {
            return Err(PackageMembershipError::ExpectedPopulationTooLarge);
        }

        let mut normalized = BTreeSet::new();
        for path in expected {
            if !is_safe_member_path(path) {
                return Err(PackageMembershipError::ExpectedPathInvalid);
            }
            if !normalized.insert(path.clone()) {
                return Err(PackageMembershipError::ExpectedPathDuplicate { path: path.clone() });
            }
        }
        Ok(Self {
            expected: normalized,
        })
    }

    /// Compare observed file-member names with this exact policy.
    ///
    /// # Errors
    ///
    /// Refuses an observed population larger than [`MAX_PACKAGE_MEMBERS`]
    /// before building comparison state.
    pub fn compare(
        &self,
        observed: &[String],
    ) -> Result<PackageMembershipResult, PackageMembershipError> {
        if observed.len() > MAX_PACKAGE_MEMBERS {
            return Err(PackageMembershipError::ObservedPopulationTooLarge);
        }

        let mut invalid_count = 0_usize;
        let mut safe_counts = BTreeMap::<String, usize>::new();
        for path in observed {
            if is_safe_member_path(path) {
                safe_counts
                    .entry(path.clone())
                    .and_modify(|count| *count += 1)
                    .or_insert(1);
            } else {
                invalid_count += 1;
            }
        }

        let mut findings = Vec::new();
        findings.extend((0..invalid_count).map(|_| PackageMembershipFinding {
            path: None,
            category: PackageMembershipCategory::ActualPathInvalid,
        }));
        findings.extend(
            safe_counts
                .iter()
                .filter(|(_, count)| **count > 1)
                .map(|(path, _)| PackageMembershipFinding {
                    path: Some(path.clone()),
                    category: PackageMembershipCategory::ActualPathDuplicate,
                }),
        );
        findings.extend(
            safe_counts
                .keys()
                .filter(|path| !self.expected.contains(*path))
                .map(|path| PackageMembershipFinding {
                    path: Some(path.clone()),
                    category: PackageMembershipCategory::UnexpectedMember,
                }),
        );
        findings.extend(
            self.expected
                .iter()
                .filter(|path| !safe_counts.contains_key(*path))
                .map(|path| PackageMembershipFinding {
                    path: Some(path.clone()),
                    category: PackageMembershipCategory::MissingMember,
                }),
        );

        let outcome = if findings.is_empty() {
            PackageMembershipOutcome::Accepted
        } else {
            PackageMembershipOutcome::Withheld
        };
        Ok(PackageMembershipResult { outcome, findings })
    }
}

fn is_safe_member_path(path: &str) -> bool {
    if path.is_empty()
        || path.len() > MAX_PACKAGE_MEMBER_PATH_BYTES
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || path.chars().any(char::is_control)
        || has_windows_drive_root(path)
    {
        return false;
    }

    !path
        .split('/')
        .any(|component| component.is_empty() || component == "." || component == "..")
}

fn has_windows_drive_root(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/'
}
