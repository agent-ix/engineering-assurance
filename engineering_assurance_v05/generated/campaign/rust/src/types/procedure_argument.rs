//! ProcedureArgument
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-002.

use serde::{Deserialize, Serialize};

/// The fields of `ProcedureArgument`, in the order the contract declares them.
pub const FIELDS: &[crate::identity::FieldMeta] = &[
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-002/kind",
        name: "kind",
        rust_name: "kind",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/EN-001",
        rust_type: "crate::ProcedureArgumentKind",
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
                path: "spec/functional/VO-002-procedure-argument.md",
                start_line: 15,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-002/value",
        name: "value",
        rust_name: "value",
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
                path: "spec/functional/VO-002-procedure-argument.md",
                start_line: 16,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
];

/// ProcedureArgument
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-002.
///
/// Roles: engineering-assurance:campaign_value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ProcedureArgument {
    /// kind
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-002/kind.
    pub kind: crate::ProcedureArgumentKind,
    /// value
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-002/value.
    pub value: String,
}

/// The deserialization shape of `ProcedureArgument`.
///
/// It exists so that `Deserialize` can route through `try_new`: serde has
/// no post-deserialization hook, and a value that skipped the constructor
/// would be a value the contract's constraints never saw.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProcedureArgumentWire {
    kind: crate::ProcedureArgumentKind,
    value: String,
}

impl ProcedureArgument {
    /// Builds the record, enforcing every bound and uniqueness rule the
    /// contract declares on its members. Deserialization routes through
    /// this constructor.
    pub fn try_new(
        kind: crate::ProcedureArgumentKind,
        value: String,
    ) -> Result<Self, crate::support::ValidationError> {
        Ok(Self { kind, value })
    }

    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}

impl<'de> Deserialize<'de> for ProcedureArgument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ProcedureArgumentWire::deserialize(deserializer)?;
        Self::try_new(wire.kind, wire.value).map_err(serde::de::Error::custom)
    }
}
