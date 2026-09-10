// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Typed protocol for the npm package-lifecycle host adapter.
//!
//! Filesystem traversal and package-manager invocation remain binary-host
//! responsibilities. This module only owns the closed machine result.

use serde::Serialize;
use thiserror::Error;

/// Protocol discriminator for npm package-lifecycle results.
pub const PACKAGE_LIFECYCLE_PROTOCOL: &str = "engineering-assurance.package-lifecycle-result/v1";

/// Closed npm package-lifecycle operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageLifecycleOperation {
    /// Stage the fixed module payload at the package root.
    Stage,
    /// Remove a staged payload after exact correspondence is proven.
    Clean,
    /// Refuse public package publication.
    RefusePublication,
}

/// Closed successful or policy-refused lifecycle outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PackageLifecycleOutcome {
    /// The fixed payload was copied and verified.
    Staged,
    /// The verified staged payload was removed or was already entirely absent.
    Cleaned,
    /// Public publication was refused without contacting a registry.
    PublicationRefused,
}

/// One versioned npm package-lifecycle result.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageLifecycleResult {
    /// Fixed result protocol discriminator.
    protocol: &'static str,
    /// Operation requested by the caller.
    operation: PackageLifecycleOperation,
    /// Closed outcome produced by that operation.
    outcome: PackageLifecycleOutcome,
}

impl PackageLifecycleResult {
    /// Construct a successful stage result.
    #[must_use]
    pub const fn staged() -> Self {
        Self {
            protocol: PACKAGE_LIFECYCLE_PROTOCOL,
            operation: PackageLifecycleOperation::Stage,
            outcome: PackageLifecycleOutcome::Staged,
        }
    }

    /// Construct a successful cleanup result.
    #[must_use]
    pub const fn cleaned() -> Self {
        Self {
            protocol: PACKAGE_LIFECYCLE_PROTOCOL,
            operation: PackageLifecycleOperation::Clean,
            outcome: PackageLifecycleOutcome::Cleaned,
        }
    }

    /// Construct the unconditional publication-refusal result.
    #[must_use]
    pub const fn publication_refused() -> Self {
        Self {
            protocol: PACKAGE_LIFECYCLE_PROTOCOL,
            operation: PackageLifecycleOperation::RefusePublication,
            outcome: PackageLifecycleOutcome::PublicationRefused,
        }
    }

    /// Return the fixed result protocol discriminator.
    #[must_use]
    pub const fn protocol(self) -> &'static str {
        self.protocol
    }

    /// Return the requested operation.
    #[must_use]
    pub const fn operation(self) -> PackageLifecycleOperation {
        self.operation
    }

    /// Return the closed operation outcome.
    #[must_use]
    pub const fn outcome(self) -> PackageLifecycleOutcome {
        self.outcome
    }

    /// Encode one newline-terminated machine result.
    ///
    /// # Errors
    ///
    /// Returns a typed error if serialization unexpectedly fails.
    pub fn to_json_line(self) -> Result<Vec<u8>, PackageLifecycleError> {
        let mut encoded =
            serde_json::to_vec(&self).map_err(PackageLifecycleError::Serialization)?;
        encoded.push(b'\n');
        Ok(encoded)
    }
}

/// Typed failure to encode a package-lifecycle result.
#[derive(Debug, Error)]
pub enum PackageLifecycleError {
    /// The closed result could not be serialized.
    #[error("package-lifecycle result serialization failed: {0}")]
    Serialization(serde_json::Error),
}

impl PackageLifecycleError {
    /// Return the stable machine category for this failure.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Serialization(_) => "package_lifecycle_result_serialization_failed",
        }
    }
}
