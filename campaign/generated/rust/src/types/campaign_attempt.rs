//! CampaignAttempt
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008.

use serde::{Deserialize, Serialize};

/// The fields of `CampaignAttempt`, in the order the contract declares them.
pub const FIELDS: &[crate::identity::FieldMeta] = &[
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-008/collectionId",
        name: "collectionId",
        rust_name: "collection_id",
        type_ref: "ix://quire/native/String",
        rust_type: "Option<String>",
        row: "field:single/non-null/optional",
        presence: "optional",
        nullable: false,
        multiplicity: crate::identity::MultiplicityMeta {
            lower: 0,
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
                path: "spec/functional/VO-008-campaign-attempt.md",
                start_line: 17,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-008/index",
        name: "index",
        rust_name: "index",
        type_ref: "ix://quire/native/Integer",
        rust_type: "i64",
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
                path: "spec/functional/VO-008-campaign-attempt.md",
                start_line: 16,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-008/member",
        name: "member",
        rust_name: "member",
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
                path: "spec/functional/VO-008-campaign-attempt.md",
                start_line: 15,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-008/rawArtifacts",
        name: "rawArtifacts",
        rust_name: "raw_artifacts",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/VO-009",
        rust_type: "Option<Vec<crate::CampaignRawArtifact>>",
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
                path: "spec/functional/VO-008-campaign-attempt.md",
                start_line: 18,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-008/reason",
        name: "reason",
        rust_name: "reason",
        type_ref: "ix://quire/native/String",
        rust_type: "Option<String>",
        row: "field:single/non-null/optional",
        presence: "optional",
        nullable: false,
        multiplicity: crate::identity::MultiplicityMeta {
            lower: 0,
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
                path: "spec/functional/VO-008-campaign-attempt.md",
                start_line: 20,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-008/status",
        name: "status",
        rust_name: "status",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/EN-004",
        rust_type: "crate::CampaignAttemptStatus",
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
                path: "spec/functional/VO-008-campaign-attempt.md",
                start_line: 19,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
];

/// CampaignAttempt
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008.
///
/// Roles: engineering-assurance:campaign_value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CampaignAttempt {
    /// collectionId
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/collectionId.
    #[serde(rename = "collectionId", skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<String>,
    /// index
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/index.
    pub index: i64,
    /// member
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/member.
    pub member: String,
    /// rawArtifacts
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/rawArtifacts.
    #[serde(rename = "rawArtifacts", skip_serializing_if = "Option::is_none")]
    pub raw_artifacts: Option<Vec<crate::CampaignRawArtifact>>,
    /// reason
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/reason.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// status
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/status.
    pub status: crate::CampaignAttemptStatus,
}

/// The deserialization shape of `CampaignAttempt`.
///
/// It exists so that `Deserialize` can route through `try_new`: serde has
/// no post-deserialization hook, and a value that skipped the constructor
/// would be a value the contract's constraints never saw.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignAttemptWire {
    #[serde(rename = "collectionId", default)]
    collection_id: Option<String>,
    index: i64,
    member: String,
    #[serde(rename = "rawArtifacts", default)]
    raw_artifacts: Option<Vec<crate::CampaignRawArtifact>>,
    #[serde(default)]
    reason: Option<String>,
    status: crate::CampaignAttemptStatus,
}

impl CampaignAttempt {
    /// Builds the record, enforcing every bound and uniqueness rule the
    /// contract declares on its members. Deserialization routes through
    /// this constructor.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        collection_id: Option<String>,
        index: i64,
        member: String,
        raw_artifacts: Option<Vec<crate::CampaignRawArtifact>>,
        reason: Option<String>,
        status: crate::CampaignAttemptStatus,
    ) -> Result<Self, crate::support::ValidationError> {
        Ok(Self {
            collection_id,
            index,
            member,
            raw_artifacts,
            reason,
            status,
        })
    }

    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}

impl<'de> Deserialize<'de> for CampaignAttempt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = CampaignAttemptWire::deserialize(deserializer)?;
        Self::try_new(
            wire.collection_id,
            wire.index,
            wire.member,
            wire.raw_artifacts,
            wire.reason,
            wire.status,
        )
        .map_err(serde::de::Error::custom)
    }
}
