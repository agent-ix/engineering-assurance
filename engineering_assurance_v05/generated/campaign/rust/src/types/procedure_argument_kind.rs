//! ProcedureArgumentKind
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-001.

use serde::{Deserialize, Serialize};

/// ProcedureArgumentKind
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-001.
///
/// Roles: engineering-assurance:campaign_enum.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProcedureArgumentKind {
    /// EN-001-input-artifact
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-001-input-artifact.
    #[serde(rename = "input_artifact")]
    InputArtifact,
    /// EN-001-literal
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-001-literal.
    #[serde(rename = "literal")]
    Literal,
    /// EN-001-output-artifact
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-001-output-artifact.
    #[serde(rename = "output_artifact")]
    OutputArtifact,
}

impl ProcedureArgumentKind {
    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}
