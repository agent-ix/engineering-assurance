// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Typed result contract for the binary package-audit adapter.
//!
//! Package construction, archive decoding, installation, filesystem access,
//! environment access, and child-process execution remain binary-host
//! responsibilities outside this reusable module.

use serde::Serialize;
use thiserror::Error;

/// Protocol discriminator for a successful package audit.
pub const PACKAGE_AUDIT_PROTOCOL: &str = "engineering-assurance.package-audit-result/v1";

/// Closed outcome for a complete package audit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageAuditOutcome {
    /// Both distributions and installed bundles passed every required check.
    Accepted,
}

/// Successful result from one complete wheel/npm package audit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageAuditResult {
    protocol: &'static str,
    outcome: PackageAuditOutcome,
    wheel_files: usize,
    npm_files: usize,
    installed_canonical_files: usize,
}

impl PackageAuditResult {
    /// Construct the accepted result after every adapter check has passed.
    #[must_use]
    pub const fn accepted(
        wheel_files: usize,
        npm_files: usize,
        installed_canonical_files: usize,
    ) -> Self {
        Self {
            protocol: PACKAGE_AUDIT_PROTOCOL,
            outcome: PackageAuditOutcome::Accepted,
            wheel_files,
            npm_files,
            installed_canonical_files,
        }
    }

    /// Return the fixed result protocol.
    #[must_use]
    pub const fn protocol(self) -> &'static str {
        self.protocol
    }

    /// Return the closed successful outcome.
    #[must_use]
    pub const fn outcome(self) -> PackageAuditOutcome {
        self.outcome
    }

    /// Return the audited wheel regular-file count.
    #[must_use]
    pub const fn wheel_files(self) -> usize {
        self.wheel_files
    }

    /// Return the audited npm regular-file count.
    #[must_use]
    pub const fn npm_files(self) -> usize {
        self.npm_files
    }

    /// Return the equal canonical-skill installed-file count.
    #[must_use]
    pub const fn installed_canonical_files(self) -> usize {
        self.installed_canonical_files
    }

    /// Encode one newline-terminated machine result.
    ///
    /// # Errors
    ///
    /// Returns a typed error if serialization unexpectedly fails.
    pub fn to_json_line(self) -> Result<Vec<u8>, PackageAuditError> {
        let mut encoded = serde_json::to_vec(&self).map_err(PackageAuditError::Serialization)?;
        encoded.push(b'\n');
        Ok(encoded)
    }
}

/// Typed failure to encode a package-audit result.
#[derive(Debug, Error)]
pub enum PackageAuditError {
    /// A closed package-audit result could not be serialized.
    #[error("package-audit result serialization failed: {0}")]
    Serialization(serde_json::Error),
}

impl PackageAuditError {
    /// Return the stable machine category for this failure.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Serialization(_) => "package_audit_result_serialization_failed",
        }
    }
}
