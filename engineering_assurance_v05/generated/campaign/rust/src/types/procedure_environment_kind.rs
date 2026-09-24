//! ProcedureEnvironmentKind
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-005.

use serde::{Deserialize, Serialize};

/// ProcedureEnvironmentKind
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-005.
///
/// Roles: engineering-assurance:campaign_enum.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ProcedureEnvironmentKind {
    /// EN-005-literal
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-005-literal.
    #[serde(rename = "literal")]
    Literal,
    /// EN-005-runtime
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/variant/EN-005-runtime.
    #[serde(rename = "runtime")]
    Runtime,
}

impl ProcedureEnvironmentKind {
    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}
