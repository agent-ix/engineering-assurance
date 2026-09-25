//! CampaignCompletionRule
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-002.

use serde::{Deserialize, Serialize};

/// CampaignCompletionRule
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-002.
///
/// Roles: engineering-assurance:campaign_enum.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CampaignCompletionRule {
    /// EN-002-all-required
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-002-all-required.
    #[serde(rename = "all_required")]
    AllRequired,
}

impl CampaignCompletionRule {
    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}
