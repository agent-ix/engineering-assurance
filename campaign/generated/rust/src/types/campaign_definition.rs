//! CampaignDefinition
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-004.

use serde::{Deserialize, Serialize};

/// The fields of `CampaignDefinition`, in the order the contract declares them.
pub const FIELDS: &[crate::identity::FieldMeta] = &[
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-004/completionRule",
        name: "completionRule",
        rust_name: "completion_rule",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/EN-002",
        rust_type: "crate::CampaignCompletionRule",
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
                path: "spec/functional/VO-004-campaign-definition.md",
                start_line: 21,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-004/id",
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
                path: "spec/functional/VO-004-campaign-definition.md",
                start_line: 16,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-004/members",
        name: "members",
        rust_name: "members",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/VO-006",
        rust_type: "Vec<crate::CampaignMember>",
        row: "field:collection/non-null/required",
        presence: "required",
        nullable: false,
        multiplicity: crate::identity::MultiplicityMeta {
            lower: 1,
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
                path: "spec/functional/VO-004-campaign-definition.md",
                start_line: 20,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-004/schemaVersion",
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
                path: "spec/functional/VO-004-campaign-definition.md",
                start_line: 15,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-004/sourceGraph",
        name: "sourceGraph",
        rust_name: "source_graph",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/VO-005",
        rust_type: "Vec<crate::CampaignSource>",
        row: "field:collection/non-null/required",
        presence: "required",
        nullable: false,
        multiplicity: crate::identity::MultiplicityMeta {
            lower: 1,
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
                path: "spec/functional/VO-004-campaign-definition.md",
                start_line: 19,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-004/subjectName",
        name: "subjectName",
        rust_name: "subject_name",
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
                path: "spec/functional/VO-004-campaign-definition.md",
                start_line: 17,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-004/subjectVersion",
        name: "subjectVersion",
        rust_name: "subject_version",
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
                path: "spec/functional/VO-004-campaign-definition.md",
                start_line: 18,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
];

/// CampaignDefinition
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-004.
///
/// Roles: engineering-assurance:campaign_value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CampaignDefinition {
    /// completionRule
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-004/completionRule.
    #[serde(rename = "completionRule")]
    pub completion_rule: crate::CampaignCompletionRule,
    /// id
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-004/id.
    pub id: String,
    /// members
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-004/members.
    pub members: Vec<crate::CampaignMember>,
    /// schemaVersion
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-004/schemaVersion.
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    /// sourceGraph
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-004/sourceGraph.
    #[serde(rename = "sourceGraph")]
    pub source_graph: Vec<crate::CampaignSource>,
    /// subjectName
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-004/subjectName.
    #[serde(rename = "subjectName")]
    pub subject_name: String,
    /// subjectVersion
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-004/subjectVersion.
    #[serde(rename = "subjectVersion")]
    pub subject_version: String,
}

/// The deserialization shape of `CampaignDefinition`.
///
/// It exists so that `Deserialize` can route through `try_new`: serde has
/// no post-deserialization hook, and a value that skipped the constructor
/// would be a value the contract's constraints never saw.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignDefinitionWire {
    #[serde(rename = "completionRule")]
    completion_rule: crate::CampaignCompletionRule,
    id: String,
    members: Vec<crate::CampaignMember>,
    #[serde(rename = "schemaVersion")]
    schema_version: String,
    #[serde(rename = "sourceGraph")]
    source_graph: Vec<crate::CampaignSource>,
    #[serde(rename = "subjectName")]
    subject_name: String,
    #[serde(rename = "subjectVersion")]
    subject_version: String,
}

impl CampaignDefinition {
    /// Builds the record, enforcing every bound and uniqueness rule the
    /// contract declares on its members. Deserialization routes through
    /// this constructor.
    pub fn try_new(
        completion_rule: crate::CampaignCompletionRule,
        id: String,
        members: Vec<crate::CampaignMember>,
        schema_version: String,
        source_graph: Vec<crate::CampaignSource>,
        subject_name: String,
        subject_version: String,
    ) -> Result<Self, crate::support::ValidationError> {
        {
            let items = &members;
            if items.is_empty() {
                return Err(crate::support::ValidationError::new(
                    "ix://agent-ix/engineering-assurance-campaign/VO-004/members",
                    "multiplicity.lower",
                    "members",
                    "1",
                ));
            }
        }
        {
            let items = &source_graph;
            if items.is_empty() {
                return Err(crate::support::ValidationError::new(
                    "ix://agent-ix/engineering-assurance-campaign/VO-004/sourceGraph",
                    "multiplicity.lower",
                    "sourceGraph",
                    "1",
                ));
            }
        }
        Ok(Self {
            completion_rule,
            id,
            members,
            schema_version,
            source_graph,
            subject_name,
            subject_version,
        })
    }

    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}

impl<'de> Deserialize<'de> for CampaignDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = CampaignDefinitionWire::deserialize(deserializer)?;
        Self::try_new(
            wire.completion_rule,
            wire.id,
            wire.members,
            wire.schema_version,
            wire.source_graph,
            wire.subject_name,
            wire.subject_version,
        )
        .map_err(serde::de::Error::custom)
    }
}
