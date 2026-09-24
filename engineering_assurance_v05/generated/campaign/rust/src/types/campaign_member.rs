//! CampaignMember
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-006.

use serde::{Deserialize, Serialize};

/// The fields of `CampaignMember`, in the order the contract declares them.
pub const FIELDS: &[crate::identity::FieldMeta] = &[
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-006/checkerProcedure",
        name: "checkerProcedure",
        rust_name: "checker_procedure",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/VO-001",
        rust_type: "Option<crate::MeasurementProcedure>",
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
                path: "spec/functional/VO-006-campaign-member.md",
                start_line: 21,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-006/definitionVersion",
        name: "definitionVersion",
        rust_name: "definition_version",
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
                path: "spec/functional/VO-006-campaign-member.md",
                start_line: 18,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-006/dependsOn",
        name: "dependsOn",
        rust_name: "depends_on",
        type_ref: "ix://quire/native/String",
        rust_type: "Option<Vec<String>>",
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
                path: "spec/functional/VO-006-campaign-member.md",
                start_line: 19,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-006/group",
        name: "group",
        rust_name: "group",
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
                path: "spec/functional/VO-006-campaign-member.md",
                start_line: 16,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-006/name",
        name: "name",
        rust_name: "name",
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
                path: "spec/functional/VO-006-campaign-member.md",
                start_line: 15,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-006/planId",
        name: "planId",
        rust_name: "plan_id",
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
                path: "spec/functional/VO-006-campaign-member.md",
                start_line: 17,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-006/required",
        name: "required",
        rust_name: "required",
        type_ref: "ix://quire/native/Boolean",
        rust_type: "bool",
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
                path: "spec/functional/VO-006-campaign-member.md",
                start_line: 20,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
];

/// CampaignMember
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-006.
///
/// Roles: engineering-assurance:campaign_value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CampaignMember {
    /// checkerProcedure
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-006/checkerProcedure.
    #[serde(rename = "checkerProcedure", skip_serializing_if = "Option::is_none")]
    pub checker_procedure: Option<crate::MeasurementProcedure>,
    /// definitionVersion
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-006/definitionVersion.
    #[serde(rename = "definitionVersion")]
    pub definition_version: String,
    /// dependsOn
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-006/dependsOn.
    #[serde(rename = "dependsOn", skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<Vec<String>>,
    /// group
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-006/group.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// name
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-006/name.
    pub name: String,
    /// planId
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-006/planId.
    #[serde(rename = "planId")]
    pub plan_id: String,
    /// required
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-006/required.
    pub required: bool,
}

/// The deserialization shape of `CampaignMember`.
///
/// It exists so that `Deserialize` can route through `try_new`: serde has
/// no post-deserialization hook, and a value that skipped the constructor
/// would be a value the contract's constraints never saw.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignMemberWire {
    #[serde(rename = "checkerProcedure", default)]
    checker_procedure: Option<crate::MeasurementProcedure>,
    #[serde(rename = "definitionVersion")]
    definition_version: String,
    #[serde(rename = "dependsOn", default)]
    depends_on: Option<Vec<String>>,
    #[serde(default)]
    group: Option<String>,
    name: String,
    #[serde(rename = "planId")]
    plan_id: String,
    required: bool,
}

impl CampaignMember {
    /// Builds the record, enforcing every bound and uniqueness rule the
    /// contract declares on its members. Deserialization routes through
    /// this constructor.
    pub fn try_new(
        checker_procedure: Option<crate::MeasurementProcedure>,
        definition_version: String,
        depends_on: Option<Vec<String>>,
        group: Option<String>,
        name: String,
        plan_id: String,
        required: bool,
    ) -> Result<Self, crate::support::ValidationError> {
        Ok(Self {
            checker_procedure,
            definition_version,
            depends_on,
            group,
            name,
            plan_id,
            required,
        })
    }

    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}

impl<'de> Deserialize<'de> for CampaignMember {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = CampaignMemberWire::deserialize(deserializer)?;
        Self::try_new(
            wire.checker_procedure,
            wire.definition_version,
            wire.depends_on,
            wire.group,
            wire.name,
            wire.plan_id,
            wire.required,
        )
        .map_err(serde::de::Error::custom)
    }
}
