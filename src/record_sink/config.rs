// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Per-repository sink selection (FR-055).
//!
//! The configuration is one closed JSON document read from the protected base
//! branch. Visibility chooses only the sink type; the private store
//! destination is always configured.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::form::split_linear_issue;

/// The path of the sink configuration in the repository.
pub const SINK_CONFIG_PATH: &str = ".engineering-assurance/sinks.json";
/// The protocol literal of the sink configuration.
pub const SINK_CONFIG_PROTOCOL: &str = "engineering-assurance.sink-config/v1";

/// A named refusal. The code is the contract; [`ErrorCode::as_str`] is its text.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The configuration is malformed, carries an unknown member or a bad value.
    SinkConfigInvalid,
    /// The configuration is absent or lacks a destination or endpoint it needs.
    SinkConfigIncomplete,
    /// `public_log` was selected for a repository that is not public.
    SinkConfigPublicLogForNonPublicRepo,
    /// The base branch is not reported as protected.
    PolicyBranchUnprotected,
    /// The leak check found a private or undeterminable item.
    PublicSinkPrivateReference,
    /// The private store write failed.
    PrivateStoreWriteFailed,
    /// The public log rejected the record.
    PublicLogRejected,
    /// The private store could not store the public form.
    PrivateStorePublicFormFailed,
    /// The signing step failed.
    SigningFailed,
    /// The signature is not padded base64 of at most 16384 characters.
    SignatureInvalid,
    /// The bundle's signed payload is not the canonical bytes of the public form.
    SignaturePayloadMismatch,
}

impl ErrorCode {
    /// The code text.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SinkConfigInvalid => "sink_config_invalid",
            Self::SinkConfigIncomplete => "sink_config_incomplete",
            Self::SinkConfigPublicLogForNonPublicRepo => {
                "sink_config_public_log_for_non_public_repo"
            }
            Self::PolicyBranchUnprotected => "policy_branch_unprotected",
            Self::PublicSinkPrivateReference => "public_sink_private_reference",
            Self::PrivateStoreWriteFailed => "private_store_write_failed",
            Self::PublicLogRejected => "public_log_rejected",
            Self::PrivateStorePublicFormFailed => "private_store_public_form_failed",
            Self::SigningFailed => "signing_failed",
            Self::SignatureInvalid => "signature_invalid",
            Self::SignaturePayloadMismatch => "signature_payload_mismatch",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::error::Error for ErrorCode {}

/// A sink implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SinkKind {
    /// The public transparency log.
    PublicLog,
    /// The private store.
    PrivateStore,
    /// Keep nothing.
    None,
}

/// A repository's visibility as the hosting service reports it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Visibility {
    /// Public.
    Public,
    /// Private.
    Private,
    /// The lookup failed or was ambiguous.
    Unknown,
}

/// Where the resolved sink came from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionSource {
    /// The `sink` member.
    Configured,
    /// The repository's visibility.
    Default,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    protocol: String,
    #[serde(default, deserialize_with = "super::form::no_null")]
    sink: Option<SinkKind>,
    #[serde(default, deserialize_with = "super::form::no_null")]
    public_log: Option<RawPublicLog>,
    #[serde(default, deserialize_with = "super::form::no_null")]
    private_store: Option<RawPrivateStore>,
    #[serde(default)]
    public_linear_teams: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPublicLog {
    #[serde(default, deserialize_with = "super::form::no_null")]
    endpoint: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPrivateStore {
    #[serde(default, deserialize_with = "super::form::no_null")]
    destination: Option<String>,
}

/// A parsed sink configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SinkConfig {
    sink: Option<SinkKind>,
    public_log_endpoint: Option<String>,
    private_store_destination: Option<String>,
    public_linear_teams: Vec<String>,
}

fn is_https_url(text: &str) -> bool {
    text.strip_prefix("https://").is_some_and(|rest| {
        let host = rest.split(['/', '?', '#']).next().unwrap_or("");
        !host.is_empty()
            && !host.contains('@')
            && !text.chars().any(|c| c.is_whitespace() || c.is_control())
    })
}

fn is_team_key(text: &str) -> bool {
    split_linear_issue(&format!("{text}-1")).is_some()
}

impl SinkConfig {
    /// Parses the document `engineering-assurance.sink-config/v1`.
    ///
    /// # Errors
    ///
    /// [`ErrorCode::SinkConfigInvalid`] for malformed JSON, an unknown member,
    /// an unknown `sink` value, a wrong protocol, a non-https endpoint, an empty
    /// destination or a malformed team key.
    pub fn parse(bytes: &[u8]) -> Result<Self, ErrorCode> {
        let raw: RawConfig =
            serde_json::from_slice(bytes).map_err(|_| ErrorCode::SinkConfigInvalid)?;
        let endpoint = raw.public_log.and_then(|p| p.endpoint);
        let destination = raw.private_store.and_then(|p| p.destination);
        let valid = raw.protocol == SINK_CONFIG_PROTOCOL
            && endpoint.as_deref().is_none_or(is_https_url)
            && destination.as_deref().is_none_or(|d| !d.is_empty())
            && raw.public_linear_teams.iter().all(|t| is_team_key(t));
        if !valid {
            return Err(ErrorCode::SinkConfigInvalid);
        }
        Ok(Self {
            sink: raw.sink,
            public_log_endpoint: endpoint,
            private_store_destination: destination,
            public_linear_teams: raw.public_linear_teams,
        })
    }
}

/// The resolved sink and the settings it needs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SinkSelection {
    /// The resolved sink.
    pub sink: SinkKind,
    /// Whether it came from the `sink` member or the visibility default.
    pub source: SelectionSource,
    /// The public log endpoint, present when the sink is `public_log`.
    pub public_log_endpoint: Option<String>,
    /// The private store destination, present unless the sink is `none`.
    pub private_store_destination: Option<String>,
    /// Linear team keys whose issues count as public (read by the leak check).
    pub public_linear_teams: Vec<String>,
}

/// Resolves the sink for a repository.
///
/// # Errors
///
/// [`ErrorCode::SinkConfigIncomplete`] for an absent document, a configured
/// `public_log` without an endpoint, or a missing destination the sink needs,
/// and
/// [`ErrorCode::SinkConfigPublicLogForNonPublicRepo`] for `public_log` on a
/// repository that is not public.
pub fn resolve_sink(
    config: Option<&SinkConfig>,
    visibility: Visibility,
) -> Result<SinkSelection, ErrorCode> {
    let config = config.ok_or(ErrorCode::SinkConfigIncomplete)?;
    let (sink, source) = match config.sink {
        Some(sink) => (sink, SelectionSource::Configured),
        None if visibility == Visibility::Public => (SinkKind::PublicLog, SelectionSource::Default),
        None => (SinkKind::PrivateStore, SelectionSource::Default),
    };
    if sink == SinkKind::PublicLog && visibility != Visibility::Public {
        return Err(ErrorCode::SinkConfigPublicLogForNonPublicRepo);
    }
    if config.sink == Some(SinkKind::PublicLog) && config.public_log_endpoint.is_none() {
        return Err(ErrorCode::SinkConfigIncomplete);
    }
    if sink != SinkKind::None && config.private_store_destination.is_none() {
        return Err(ErrorCode::SinkConfigIncomplete);
    }
    Ok(SinkSelection {
        sink,
        source,
        public_log_endpoint: config.public_log_endpoint.clone(),
        private_store_destination: config.private_store_destination.clone(),
        public_linear_teams: config.public_linear_teams.clone(),
    })
}

/// A source of policy documents. It can read only from the protected base
/// branch: the branch or pull request under change is not reachable through it.
pub trait PolicySource {
    /// Whether the source reports the base branch as protected, or `None` when
    /// it does not report protection.
    fn base_branch_protected(&self) -> Option<bool>;

    /// The bytes of `path` on the protected base branch, or `None` when the
    /// file is absent or unreadable.
    fn read_from_base_branch(&self, path: &str) -> Option<Vec<u8>>;
}

/// Reads one policy document (the sink configuration, the role policy or the
/// workflow-identity policy) from the protected base branch.
///
/// # Errors
///
/// [`ErrorCode::PolicyBranchUnprotected`] when the base branch is not reported
/// as protected; nothing is read.
pub fn read_policy_document(
    source: &dyn PolicySource,
    path: &str,
) -> Result<Option<Vec<u8>>, ErrorCode> {
    if source.base_branch_protected() != Some(true) {
        return Err(ErrorCode::PolicyBranchUnprotected);
    }
    Ok(source.read_from_base_branch(path))
}

/// Reads the sink configuration from the protected base branch and resolves
/// the repository's sink.
///
/// # Errors
///
/// Any refusal of [`read_policy_document`], [`SinkConfig::parse`] or
/// [`resolve_sink`].
pub fn resolve_from_source(
    source: &dyn PolicySource,
    visibility: Visibility,
) -> Result<SinkSelection, ErrorCode> {
    let config = read_policy_document(source, SINK_CONFIG_PATH)?
        .map(|bytes| SinkConfig::parse(&bytes))
        .transpose()?;
    resolve_sink(config.as_ref(), visibility)
}
