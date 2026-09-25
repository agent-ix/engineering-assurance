//! ProcedureInputOrigin
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-012.

use serde::{Deserialize, Serialize};

/// The fields of `ProcedureInputOrigin`, in the order the contract declares them.
pub const FIELDS: &[crate::identity::FieldMeta] = &[
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-012/dependencyArtifactRole",
        name: "dependencyArtifactRole",
        rust_name: "dependency_artifact_role",
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
                path: "spec/functional/VO-012-procedure-input-origin.md",
                start_line: 20,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-012/dependencyMember",
        name: "dependencyMember",
        rust_name: "dependency_member",
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
                path: "spec/functional/VO-012-procedure-input-origin.md",
                start_line: 19,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-012/kind",
        name: "kind",
        rust_name: "kind",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/EN-006",
        rust_type: "crate::ProcedureInputOriginKind",
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
                path: "spec/functional/VO-012-procedure-input-origin.md",
                start_line: 16,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-012/role",
        name: "role",
        rust_name: "role",
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
                path: "spec/functional/VO-012-procedure-input-origin.md",
                start_line: 15,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-012/sourcePath",
        name: "sourcePath",
        rust_name: "source_path",
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
                path: "spec/functional/VO-012-procedure-input-origin.md",
                start_line: 18,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-012/sourceRepository",
        name: "sourceRepository",
        rust_name: "source_repository",
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
                path: "spec/functional/VO-012-procedure-input-origin.md",
                start_line: 17,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
];

/// ProcedureInputOrigin
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-012.
///
/// Roles: engineering-assurance:campaign_value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProcedureInputOrigin {
    /// dependencyArtifactRole
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-012/dependencyArtifactRole.
    #[serde(
        rename = "dependencyArtifactRole",
        skip_serializing_if = "Option::is_none"
    )]
    pub dependency_artifact_role: Option<String>,
    /// dependencyMember
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-012/dependencyMember.
    #[serde(rename = "dependencyMember", skip_serializing_if = "Option::is_none")]
    pub dependency_member: Option<String>,
    /// kind
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-012/kind.
    pub kind: crate::ProcedureInputOriginKind,
    /// role
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-012/role.
    pub role: String,
    /// sourcePath
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-012/sourcePath.
    #[serde(rename = "sourcePath", skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    /// sourceRepository
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-012/sourceRepository.
    #[serde(rename = "sourceRepository", skip_serializing_if = "Option::is_none")]
    pub source_repository: Option<String>,
}

/// The deserialization shape of `ProcedureInputOrigin`.
///
/// It exists so that `Deserialize` can route through `try_new`: serde has
/// no post-deserialization hook, and a value that skipped the constructor
/// would be a value the contract's constraints never saw.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcedureInputOriginWire {
    #[serde(rename = "dependencyArtifactRole", default)]
    dependency_artifact_role: Option<String>,
    #[serde(rename = "dependencyMember", default)]
    dependency_member: Option<String>,
    kind: crate::ProcedureInputOriginKind,
    role: String,
    #[serde(rename = "sourcePath", default)]
    source_path: Option<String>,
    #[serde(rename = "sourceRepository", default)]
    source_repository: Option<String>,
}

impl ProcedureInputOrigin {
    /// Builds the record, enforcing every bound and uniqueness rule the
    /// contract declares on its members. Deserialization routes through
    /// this constructor.
    pub fn try_new(
        dependency_artifact_role: Option<String>,
        dependency_member: Option<String>,
        kind: crate::ProcedureInputOriginKind,
        role: String,
        source_path: Option<String>,
        source_repository: Option<String>,
    ) -> Result<Self, crate::support::ValidationError> {
        Ok(Self {
            dependency_artifact_role,
            dependency_member,
            kind,
            role,
            source_path,
            source_repository,
        })
    }

    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}

impl<'de> Deserialize<'de> for ProcedureInputOrigin {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ProcedureInputOriginWire::deserialize(deserializer)?;
        Self::try_new(
            wire.dependency_artifact_role,
            wire.dependency_member,
            wire.kind,
            wire.role,
            wire.source_path,
            wire.source_repository,
        )
        .map_err(serde::de::Error::custom)
    }
}
