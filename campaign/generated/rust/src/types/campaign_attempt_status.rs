//! CampaignAttemptStatus
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-004.

use serde::{Deserialize, Serialize};

/// CampaignAttemptStatus
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-004.
///
/// Roles: engineering-assurance:campaign_enum.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CampaignAttemptStatus {
    /// EN-004-cancelled
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-004-cancelled.
    #[serde(rename = "cancelled")]
    Cancelled,
    /// EN-004-completed
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-004-completed.
    #[serde(rename = "completed")]
    Completed,
    /// EN-004-containment-failure
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-004-containment-failure.
    #[serde(rename = "containment_failure")]
    ContainmentFailure,
    /// EN-004-failed
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-004-failed.
    #[serde(rename = "failed")]
    Failed,
    /// EN-004-invalid-request
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-004-invalid-request.
    #[serde(rename = "invalid_request")]
    InvalidRequest,
    /// EN-004-malformed-response
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-004-malformed-response.
    #[serde(rename = "malformed_response")]
    MalformedResponse,
    /// EN-004-refused
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-004-refused.
    #[serde(rename = "refused")]
    Refused,
    /// EN-004-timed-out
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-004-timed-out.
    #[serde(rename = "timed_out")]
    TimedOut,
    /// EN-004-unavailable
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-004-unavailable.
    #[serde(rename = "unavailable")]
    Unavailable,
}

impl CampaignAttemptStatus {
    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}
