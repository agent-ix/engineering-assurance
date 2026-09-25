//! ProcedureArtifact
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-003.

use serde::{Deserialize, Serialize};

/// The fields of `ProcedureArtifact`, in the order the contract declares them.
pub const FIELDS: &[crate::identity::FieldMeta] = &[
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-003/required",
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
                path: "spec/functional/VO-003-procedure-artifact.md",
                start_line: 16,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-003/role",
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
                path: "spec/functional/VO-003-procedure-artifact.md",
                start_line: 15,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
];

/// ProcedureArtifact
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-003.
///
/// Roles: engineering-assurance:campaign_value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProcedureArtifact {
    /// required
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-003/required.
    pub required: bool,
    /// role
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-003/role.
    pub role: String,
}

/// The deserialization shape of `ProcedureArtifact`.
///
/// It exists so that `Deserialize` can route through `try_new`: serde has
/// no post-deserialization hook, and a value that skipped the constructor
/// would be a value the contract's constraints never saw.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcedureArtifactWire {
    required: bool,
    role: String,
}

impl ProcedureArtifact {
    /// Builds the record, enforcing every bound and uniqueness rule the
    /// contract declares on its members. Deserialization routes through
    /// this constructor.
    pub fn try_new(required: bool, role: String) -> Result<Self, crate::support::ValidationError> {
        Ok(Self { required, role })
    }

    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}

impl<'de> Deserialize<'de> for ProcedureArtifact {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ProcedureArtifactWire::deserialize(deserializer)?;
        Self::try_new(wire.required, wire.role).map_err(serde::de::Error::custom)
    }
}
