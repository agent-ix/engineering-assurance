//! CampaignRun
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-007.

use serde::{Deserialize, Serialize};

/// The fields of `CampaignRun`, in the order the contract declares them.
pub const FIELDS: &[crate::identity::FieldMeta] = &[
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-007/attempts",
        name: "attempts",
        rust_name: "attempts",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/VO-008",
        rust_type: "Option<Vec<crate::CampaignAttempt>>",
        row: "field:collection/non-null/optional",
        presence: "optional",
        nullable: false,
        multiplicity: crate::identity::MultiplicityMeta {
            lower: 0,
            upper: None,
            ordered: Some(false),
            unique: Some(false),
        },
        unit: None,
        default_kind: "none",
        default_value: None,
        origin: crate::identity::OriginMeta {
            source: Some(crate::identity::SourceLocusMeta {
                source_identity: "ix://agent-ix/engineering-assurance-campaign/spec",
                path: "spec/functional/VO-007-campaign-run.md",
                start_line: 19,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-007/definitionDigest",
        name: "definitionDigest",
        rust_name: "definition_digest",
        type_ref: "ix://quire/native/String",
        rust_type: "String",
        row: "field:single/non-null/required",
        presence: "required",
        nullable: false,
        multiplicity: crate::identity::MultiplicityMeta {
            lower: 1,
            upper: Some(1),
            ordered: Some(false),
            unique: Some(false),
        },
        unit: None,
        default_kind: "none",
        default_value: None,
        origin: crate::identity::OriginMeta {
            source: Some(crate::identity::SourceLocusMeta {
                source_identity: "ix://agent-ix/engineering-assurance-campaign/spec",
                path: "spec/functional/VO-007-campaign-run.md",
                start_line: 17,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-007/id",
        name: "id",
        rust_name: "id",
        type_ref: "ix://quire/native/String",
        rust_type: "String",
        row: "field:single/non-null/required",
        presence: "required",
        nullable: false,
        multiplicity: crate::identity::MultiplicityMeta {
            lower: 1,
            upper: Some(1),
            ordered: Some(false),
            unique: Some(false),
        },
        unit: None,
        default_kind: "none",
        default_value: None,
        origin: crate::identity::OriginMeta {
            source: Some(crate::identity::SourceLocusMeta {
                source_identity: "ix://agent-ix/engineering-assurance-campaign/spec",
                path: "spec/functional/VO-007-campaign-run.md",
                start_line: 16,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-007/schemaVersion",
        name: "schemaVersion",
        rust_name: "schema_version",
        type_ref: "ix://quire/native/String",
        rust_type: "String",
        row: "field:single/non-null/required",
        presence: "required",
        nullable: false,
        multiplicity: crate::identity::MultiplicityMeta {
            lower: 1,
            upper: Some(1),
            ordered: Some(false),
            unique: Some(false),
        },
        unit: None,
        default_kind: "none",
        default_value: None,
        origin: crate::identity::OriginMeta {
            source: Some(crate::identity::SourceLocusMeta {
                source_identity: "ix://agent-ix/engineering-assurance-campaign/spec",
                path: "spec/functional/VO-007-campaign-run.md",
                start_line: 15,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-007/sourceGraphDigest",
        name: "sourceGraphDigest",
        rust_name: "source_graph_digest",
        type_ref: "ix://quire/native/String",
        rust_type: "String",
        row: "field:single/non-null/required",
        presence: "required",
        nullable: false,
        multiplicity: crate::identity::MultiplicityMeta {
            lower: 1,
            upper: Some(1),
            ordered: Some(false),
            unique: Some(false),
        },
        unit: None,
        default_kind: "none",
        default_value: None,
        origin: crate::identity::OriginMeta {
            source: Some(crate::identity::SourceLocusMeta {
                source_identity: "ix://agent-ix/engineering-assurance-campaign/spec",
                path: "spec/functional/VO-007-campaign-run.md",
                start_line: 18,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-007/verdict",
        name: "verdict",
        rust_name: "verdict",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/EN-003",
        rust_type: "crate::CampaignVerdict",
        row: "field:single/non-null/required",
        presence: "required",
        nullable: false,
        multiplicity: crate::identity::MultiplicityMeta {
            lower: 1,
            upper: Some(1),
            ordered: Some(false),
            unique: Some(false),
        },
        unit: None,
        default_kind: "none",
        default_value: None,
        origin: crate::identity::OriginMeta {
            source: Some(crate::identity::SourceLocusMeta {
                source_identity: "ix://agent-ix/engineering-assurance-campaign/spec",
                path: "spec/functional/VO-007-campaign-run.md",
                start_line: 20,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
];

/// CampaignRun
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-007.
///
/// Roles: engineering-assurance:campaign_value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CampaignRun {
    /// attempts
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-007/attempts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempts: Option<Vec<crate::CampaignAttempt>>,
    /// definitionDigest
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-007/definitionDigest.
    #[serde(rename = "definitionDigest")]
    pub definition_digest: String,
    /// id
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-007/id.
    pub id: String,
    /// schemaVersion
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-007/schemaVersion.
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    /// sourceGraphDigest
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-007/sourceGraphDigest.
    #[serde(rename = "sourceGraphDigest")]
    pub source_graph_digest: String,
    /// verdict
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-007/verdict.
    pub verdict: crate::CampaignVerdict,
}

/// The deserialization shape of `CampaignRun`.
///
/// It exists so that `Deserialize` can route through `try_new`: serde has
/// no post-deserialization hook, and a value that skipped the constructor
/// would be a value the contract's constraints never saw.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignRunWire {
    #[serde(default)]
    attempts: Option<Vec<crate::CampaignAttempt>>,
    #[serde(rename = "definitionDigest")]
    definition_digest: String,
    id: String,
    #[serde(rename = "schemaVersion")]
    schema_version: String,
    #[serde(rename = "sourceGraphDigest")]
    source_graph_digest: String,
    verdict: crate::CampaignVerdict,
}

impl CampaignRun {
    /// Builds the record, enforcing every bound and uniqueness rule the
    /// contract declares on its members. Deserialization routes through
    /// this constructor.
    pub fn try_new(
        attempts: Option<Vec<crate::CampaignAttempt>>,
        definition_digest: String,
        id: String,
        schema_version: String,
        source_graph_digest: String,
        verdict: crate::CampaignVerdict,
    ) -> Result<Self, crate::support::ValidationError> {
        Ok(Self {
            attempts,
            definition_digest,
            id,
            schema_version,
            source_graph_digest,
            verdict,
        })
    }

    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}

impl<'de> Deserialize<'de> for CampaignRun {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = CampaignRunWire::deserialize(deserializer)?;
        Self::try_new(
            wire.attempts,
            wire.definition_digest,
            wire.id,
            wire.schema_version,
            wire.source_graph_digest,
            wire.verdict,
        )
        .map_err(serde::de::Error::custom)
    }
}
