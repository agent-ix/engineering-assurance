//! ProcedureInputOriginKind
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-006.

use serde::{Deserialize, Serialize};

/// ProcedureInputOriginKind
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-006.
///
/// Roles: engineering-assurance:campaign_enum.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProcedureInputOriginKind {
    /// EN-006-dependency
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-006-dependency.
    #[serde(rename = "dependency")]
    Dependency,
    /// EN-006-selected-bytes
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-006-selected-bytes.
    #[serde(rename = "selected_bytes")]
    SelectedBytes,
    /// EN-006-source-file
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-006-source-file.
    #[serde(rename = "source_file")]
    SourceFile,
}

impl ProcedureInputOriginKind {
    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}
