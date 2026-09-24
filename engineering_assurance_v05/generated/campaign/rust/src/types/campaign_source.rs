//! CampaignSource
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-005.

use serde::{Deserialize, Serialize};

/// The fields of `CampaignSource`, in the order the contract declares them.
pub const FIELDS: &[crate::identity::FieldMeta] = &[
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-005/digest",
        name: "digest",
        rust_name: "digest",
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
                path: "spec/functional/VO-005-campaign-source.md",
                start_line: 25,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-005/repository",
        name: "repository",
        rust_name: "repository",
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
                path: "spec/functional/VO-005-campaign-source.md",
                start_line: 23,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-005/revision",
        name: "revision",
        rust_name: "revision",
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
                path: "spec/functional/VO-005-campaign-source.md",
                start_line: 24,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
];

/// CampaignSource
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-005.
///
/// Roles: engineering-assurance:campaign_value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CampaignSource {
    /// digest
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-005/digest.
    pub digest: String,
    /// repository
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-005/repository.
    pub repository: String,
    /// revision
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-005/revision.
    pub revision: String,
}

/// The deserialization shape of `CampaignSource`.
///
/// It exists so that `Deserialize` can route through `try_new`: serde has
/// no post-deserialization hook, and a value that skipped the constructor
/// would be a value the contract's constraints never saw.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CampaignSourceWire {
    digest: String,
    repository: String,
    revision: String,
}

impl CampaignSource {
    /// Builds the record, enforcing every bound and uniqueness rule the
    /// contract declares on its members. Deserialization routes through
    /// this constructor.
    pub fn try_new(
        digest: String,
        repository: String,
        revision: String,
    ) -> Result<Self, crate::support::ValidationError> {
        Ok(Self {
            digest,
            repository,
            revision,
        })
    }

    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}

impl<'de> Deserialize<'de> for CampaignSource {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = CampaignSourceWire::deserialize(deserializer)?;
        Self::try_new(wire.digest, wire.repository, wire.revision).map_err(serde::de::Error::custom)
    }
}
