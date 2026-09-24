#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! Generated Rust/Serde declarations for `agent-ix/engineering-assurance-campaign`.
//!
//! Every declaration here is derived from the semantic contract by one
//! published mapping. Nothing in this crate is hand-written and nothing in
//! it should be hand-edited.

pub mod identity;
pub mod provenance;
pub mod support;
pub mod types;

pub use crate::identity::{FieldMeta, TypeMeta, TYPES};
pub use crate::types::campaign_attempt::CampaignAttempt;
pub use crate::types::campaign_attempt_status::CampaignAttemptStatus;
pub use crate::types::campaign_completion_rule::CampaignCompletionRule;
pub use crate::types::campaign_definition::CampaignDefinition;
pub use crate::types::campaign_member::CampaignMember;
pub use crate::types::campaign_raw_artifact::CampaignRawArtifact;
pub use crate::types::campaign_run::CampaignRun;
pub use crate::types::campaign_source::CampaignSource;
pub use crate::types::campaign_verdict::CampaignVerdict;
pub use crate::types::measurement_procedure::MeasurementProcedure;
pub use crate::types::procedure_argument::ProcedureArgument;
pub use crate::types::procedure_argument_kind::ProcedureArgumentKind;
pub use crate::types::procedure_artifact::ProcedureArtifact;
pub use crate::types::procedure_environment::ProcedureEnvironment;
pub use crate::types::procedure_environment_kind::ProcedureEnvironmentKind;
pub use crate::types::procedure_input_origin::ProcedureInputOrigin;
pub use crate::types::procedure_input_origin_kind::ProcedureInputOriginKind;
pub use crate::types::procedure_input_prefix::ProcedureInputPrefix;

/// One variant per generated type.
///
/// A consumer that matches exhaustively over this enum stops compiling when
/// the contract gains a type, which is the point: a contract addition is a
/// compile error in the consumer rather than a silent omission.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SemanticType {
    /// ProcedureArgumentKind
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-001.
    ///
    /// Roles: engineering-assurance:campaign_enum.
    ProcedureArgumentKind,
    /// CampaignCompletionRule
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-002.
    ///
    /// Roles: engineering-assurance:campaign_enum.
    CampaignCompletionRule,
    /// CampaignVerdict
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-003.
    ///
    /// Roles: engineering-assurance:campaign_enum.
    CampaignVerdict,
    /// CampaignAttemptStatus
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-004.
    ///
    /// Roles: engineering-assurance:campaign_enum.
    CampaignAttemptStatus,
    /// ProcedureEnvironmentKind
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-005.
    ///
    /// Roles: engineering-assurance:campaign_enum.
    ProcedureEnvironmentKind,
    /// ProcedureInputOriginKind
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/EN-006.
    ///
    /// Roles: engineering-assurance:campaign_enum.
    ProcedureInputOriginKind,
    /// MeasurementProcedure
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001.
    ///
    /// Roles: engineering-assurance:campaign_value.
    MeasurementProcedure,
    /// ProcedureArgument
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-002.
    ///
    /// Roles: engineering-assurance:campaign_value.
    ProcedureArgument,
    /// ProcedureArtifact
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-003.
    ///
    /// Roles: engineering-assurance:campaign_value.
    ProcedureArtifact,
    /// CampaignDefinition
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-004.
    ///
    /// Roles: engineering-assurance:campaign_value.
    CampaignDefinition,
    /// CampaignSource
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-005.
    ///
    /// Roles: engineering-assurance:campaign_value.
    CampaignSource,
    /// CampaignMember
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-006.
    ///
    /// Roles: engineering-assurance:campaign_value.
    CampaignMember,
    /// CampaignRun
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-007.
    ///
    /// Roles: engineering-assurance:campaign_value.
    CampaignRun,
    /// CampaignAttempt
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008.
    ///
    /// Roles: engineering-assurance:campaign_value.
    CampaignAttempt,
    /// CampaignRawArtifact
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-009.
    ///
    /// Roles: engineering-assurance:campaign_value.
    CampaignRawArtifact,
    /// ProcedureEnvironment
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-010.
    ///
    /// Roles: engineering-assurance:campaign_value.
    ProcedureEnvironment,
    /// ProcedureInputPrefix
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-011.
    ///
    /// Roles: engineering-assurance:campaign_value.
    ProcedureInputPrefix,
    /// ProcedureInputOrigin
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-012.
    ///
    /// Roles: engineering-assurance:campaign_value.
    ProcedureInputOrigin,
}

impl SemanticType {
    /// The semantic identity of the type the variant names.
    pub fn identity(&self) -> &'static str {
        match self {
            SemanticType::ProcedureArgumentKind => {
                "ix://agent-ix/engineering-assurance-campaign/EN-001"
            }
            SemanticType::CampaignCompletionRule => {
                "ix://agent-ix/engineering-assurance-campaign/EN-002"
            }
            SemanticType::CampaignVerdict => "ix://agent-ix/engineering-assurance-campaign/EN-003",
            SemanticType::CampaignAttemptStatus => {
                "ix://agent-ix/engineering-assurance-campaign/EN-004"
            }
            SemanticType::ProcedureEnvironmentKind => {
                "ix://agent-ix/engineering-assurance-campaign/EN-005"
            }
            SemanticType::ProcedureInputOriginKind => {
                "ix://agent-ix/engineering-assurance-campaign/EN-006"
            }
            SemanticType::MeasurementProcedure => {
                "ix://agent-ix/engineering-assurance-campaign/VO-001"
            }
            SemanticType::ProcedureArgument => {
                "ix://agent-ix/engineering-assurance-campaign/VO-002"
            }
            SemanticType::ProcedureArtifact => {
                "ix://agent-ix/engineering-assurance-campaign/VO-003"
            }
            SemanticType::CampaignDefinition => {
                "ix://agent-ix/engineering-assurance-campaign/VO-004"
            }
            SemanticType::CampaignSource => "ix://agent-ix/engineering-assurance-campaign/VO-005",
            SemanticType::CampaignMember => "ix://agent-ix/engineering-assurance-campaign/VO-006",
            SemanticType::CampaignRun => "ix://agent-ix/engineering-assurance-campaign/VO-007",
            SemanticType::CampaignAttempt => "ix://agent-ix/engineering-assurance-campaign/VO-008",
            SemanticType::CampaignRawArtifact => {
                "ix://agent-ix/engineering-assurance-campaign/VO-009"
            }
            SemanticType::ProcedureEnvironment => {
                "ix://agent-ix/engineering-assurance-campaign/VO-010"
            }
            SemanticType::ProcedureInputPrefix => {
                "ix://agent-ix/engineering-assurance-campaign/VO-011"
            }
            SemanticType::ProcedureInputOrigin => {
                "ix://agent-ix/engineering-assurance-campaign/VO-012"
            }
        }
    }
}

/// Every extension identity the contract this crate was generated from
/// declares, ordered by code point.
pub const DECLARED_EXTENSION_IDENTITIES: &[&str] = &[];

/// The capabilities this crate admits.
///
/// Empty, and empty is a stated decision rather than an omission.
/// `consumer-policy.schema.json` is sealed and carries no capability
/// member, and the published `rust` target contract declares no capability
/// list, so there is no published input a non-empty set could be read from.
/// That is GAP-007 in `conformance/contract-gaps.json`, owned by issue #9.
/// A crate that claimed to admit a capability nobody published would be
/// inventing the rule the gap records as missing.
pub const ADMITTED_CAPABILITIES: &[&str] = &[];

/// Decides one extension against this crate's declared set.
///
/// An empty result is acceptance; a blocking diagnostic is rejection. A
/// `required: false` extension is always preserved, whatever its identity.
pub fn decide_extension(extension: &support::Extension) -> Vec<support::Diagnostic> {
    extension.decide(DECLARED_EXTENSION_IDENTITIES, ADMITTED_CAPABILITIES)
}
