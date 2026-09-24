//! CampaignAttempt
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008.

use serde::{Deserialize, Serialize};

/// The fields of `CampaignAttempt`, in the order the contract declares them.
pub const FIELDS: &[crate::identity::FieldMeta] = &[
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-008/checkerRequestDigest",
        name: "checkerRequestDigest",
        rust_name: "checker_request_digest",
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
                start_line: 23,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-008/checkerResultDigest",
        name: "checkerResultDigest",
        rust_name: "checker_result_digest",
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
                start_line: 24,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-008/collectionDigest",
        name: "collectionDigest",
        rust_name: "collection_digest",
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
                start_line: 19,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-008/domainVerdictDigest",
        name: "domainVerdictDigest",
        rust_name: "domain_verdict_digest",
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
                start_line: 22,
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
                start_line: 25,
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
                start_line: 27,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-008/requestDigest",
        name: "requestDigest",
        rust_name: "request_digest",
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
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-008/resultDigest",
        name: "resultDigest",
        rust_name: "result_digest",
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
                start_line: 18,
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
                start_line: 26,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-008/verdictDigest",
        name: "verdictDigest",
        rust_name: "verdict_digest",
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
                start_line: 21,
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
    /// checkerRequestDigest
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/checkerRequestDigest.
    #[serde(
        rename = "checkerRequestDigest",
        skip_serializing_if = "Option::is_none"
    )]
    pub checker_request_digest: Option<String>,
    /// checkerResultDigest
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/checkerResultDigest.
    #[serde(
        rename = "checkerResultDigest",
        skip_serializing_if = "Option::is_none"
    )]
    pub checker_result_digest: Option<String>,
    /// collectionDigest
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/collectionDigest.
    #[serde(rename = "collectionDigest", skip_serializing_if = "Option::is_none")]
    pub collection_digest: Option<String>,
    /// collectionId
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/collectionId.
    #[serde(rename = "collectionId", skip_serializing_if = "Option::is_none")]
    pub collection_id: Option<String>,
    /// domainVerdictDigest
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/domainVerdictDigest.
    #[serde(
        rename = "domainVerdictDigest",
        skip_serializing_if = "Option::is_none"
    )]
    pub domain_verdict_digest: Option<String>,
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
    /// requestDigest
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/requestDigest.
    #[serde(rename = "requestDigest", skip_serializing_if = "Option::is_none")]
    pub request_digest: Option<String>,
    /// resultDigest
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/resultDigest.
    #[serde(rename = "resultDigest", skip_serializing_if = "Option::is_none")]
    pub result_digest: Option<String>,
    /// status
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/status.
    pub status: crate::CampaignAttemptStatus,
    /// verdictDigest
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-008/verdictDigest.
    #[serde(rename = "verdictDigest", skip_serializing_if = "Option::is_none")]
    pub verdict_digest: Option<String>,
}

/// The deserialization shape of `CampaignAttempt`.
///
/// It exists so that `Deserialize` can route through `try_new`: serde has
/// no post-deserialization hook, and a value that skipped the constructor
/// would be a value the contract's constraints never saw.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignAttemptWire {
    #[serde(rename = "checkerRequestDigest", default)]
    checker_request_digest: Option<String>,
    #[serde(rename = "checkerResultDigest", default)]
    checker_result_digest: Option<String>,
    #[serde(rename = "collectionDigest", default)]
    collection_digest: Option<String>,
    #[serde(rename = "collectionId", default)]
    collection_id: Option<String>,
    #[serde(rename = "domainVerdictDigest", default)]
    domain_verdict_digest: Option<String>,
    index: i64,
    member: String,
    #[serde(rename = "rawArtifacts", default)]
    raw_artifacts: Option<Vec<crate::CampaignRawArtifact>>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(rename = "requestDigest", default)]
    request_digest: Option<String>,
    #[serde(rename = "resultDigest", default)]
    result_digest: Option<String>,
    status: crate::CampaignAttemptStatus,
    #[serde(rename = "verdictDigest", default)]
    verdict_digest: Option<String>,
}

impl CampaignAttempt {
    /// Builds the record, enforcing every bound and uniqueness rule the
    /// contract declares on its members. Deserialization routes through
    /// this constructor.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        checker_request_digest: Option<String>,
        checker_result_digest: Option<String>,
        collection_digest: Option<String>,
        collection_id: Option<String>,
        domain_verdict_digest: Option<String>,
        index: i64,
        member: String,
        raw_artifacts: Option<Vec<crate::CampaignRawArtifact>>,
        reason: Option<String>,
        request_digest: Option<String>,
        result_digest: Option<String>,
        status: crate::CampaignAttemptStatus,
        verdict_digest: Option<String>,
    ) -> Result<Self, crate::support::ValidationError> {
        Ok(Self {
            checker_request_digest,
            checker_result_digest,
            collection_digest,
            collection_id,
            domain_verdict_digest,
            index,
            member,
            raw_artifacts,
            reason,
            request_digest,
            result_digest,
            status,
            verdict_digest,
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
            wire.checker_request_digest,
            wire.checker_result_digest,
            wire.collection_digest,
            wire.collection_id,
            wire.domain_verdict_digest,
            wire.index,
            wire.member,
            wire.raw_artifacts,
            wire.reason,
            wire.request_digest,
            wire.result_digest,
            wire.status,
            wire.verdict_digest,
        )
        .map_err(serde::de::Error::custom)
    }
}
