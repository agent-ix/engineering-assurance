//! CampaignVerdict
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-003.

use serde::{Deserialize, Serialize};

/// CampaignVerdict
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-003.
///
/// Roles: engineering-assurance:campaign_enum.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CampaignVerdict {
    /// EN-003-accepted
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-003-accepted.
    #[serde(rename = "accepted")]
    Accepted,
    /// EN-003-inconclusive
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-003-inconclusive.
    #[serde(rename = "inconclusive")]
    Inconclusive,
    /// EN-003-rejected
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-003-rejected.
    #[serde(rename = "rejected")]
    Rejected,
}

impl CampaignVerdict {
    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}
