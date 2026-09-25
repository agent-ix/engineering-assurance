//! MeasurementProcedure
//!
//! Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001.

use serde::{Deserialize, Serialize};

/// The fields of `MeasurementProcedure`, in the order the contract declares them.
pub const FIELDS: &[crate::identity::FieldMeta] = &[
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/arguments",
        name: "arguments",
        rust_name: "arguments",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/VO-002",
        rust_type: "Option<Vec<crate::ProcedureArgument>>",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 19,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/environment",
        name: "environment",
        rust_name: "environment",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/VO-010",
        rust_type: "Option<Vec<crate::ProcedureEnvironment>>",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 20,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/inputOrigins",
        name: "inputOrigins",
        rust_name: "input_origins",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/VO-012",
        rust_type: "Option<Vec<crate::ProcedureInputOrigin>>",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 22,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/inputRolePrefixes",
        name: "inputRolePrefixes",
        rust_name: "input_role_prefixes",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/VO-011",
        rust_type: "Option<Vec<crate::ProcedureInputPrefix>>",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 23,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/inputs",
        name: "inputs",
        rust_name: "inputs",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/VO-003",
        rust_type: "Option<Vec<crate::ProcedureArtifact>>",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 21,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/outputTrees",
        name: "outputTrees",
        rust_name: "output_trees",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/VO-003",
        rust_type: "Option<Vec<crate::ProcedureArtifact>>",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 25,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/outputs",
        name: "outputs",
        rust_name: "outputs",
        type_ref: "ix://agent-ix/engineering-assurance-campaign/VO-003",
        rust_type: "Option<Vec<crate::ProcedureArtifact>>",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 24,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/producerName",
        name: "producerName",
        rust_name: "producer_name",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 16,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/producerVersion",
        name: "producerVersion",
        rust_name: "producer_version",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 17,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/repetitions",
        name: "repetitions",
        rust_name: "repetitions",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 29,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/responseAdapter",
        name: "responseAdapter",
        rust_name: "response_adapter",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 27,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/responseAdapterVersion",
        name: "responseAdapterVersion",
        rust_name: "response_adapter_version",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 28,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/responseProtocol",
        name: "responseProtocol",
        rust_name: "response_protocol",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 26,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/schemaVersion",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 15,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/sourceRepository",
        name: "sourceRepository",
        rust_name: "source_repository",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 18,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
    crate::identity::FieldMeta {
        identity: "ix://agent-ix/engineering-assurance-campaign/VO-001/timeoutMillis",
        name: "timeoutMillis",
        rust_name: "timeout_millis",
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
                path: "spec/functional/VO-001-measurement-procedure.md",
                start_line: 30,
                start_column: 3,
                end_line: None,
                end_column: None,
            }),
            generated: None,
        },
    },
];

/// MeasurementProcedure
///
/// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001.
///
/// Roles: engineering-assurance:campaign_value.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MeasurementProcedure {
    /// arguments
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/arguments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<crate::ProcedureArgument>>,
    /// environment
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/environment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<Vec<crate::ProcedureEnvironment>>,
    /// inputOrigins
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/inputOrigins.
    #[serde(rename = "inputOrigins", skip_serializing_if = "Option::is_none")]
    pub input_origins: Option<Vec<crate::ProcedureInputOrigin>>,
    /// inputRolePrefixes
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/inputRolePrefixes.
    #[serde(rename = "inputRolePrefixes", skip_serializing_if = "Option::is_none")]
    pub input_role_prefixes: Option<Vec<crate::ProcedureInputPrefix>>,
    /// inputs
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/inputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<Vec<crate::ProcedureArtifact>>,
    /// outputTrees
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/outputTrees.
    #[serde(rename = "outputTrees", skip_serializing_if = "Option::is_none")]
    pub output_trees: Option<Vec<crate::ProcedureArtifact>>,
    /// outputs
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/outputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<crate::ProcedureArtifact>>,
    /// producerName
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/producerName.
    #[serde(rename = "producerName")]
    pub producer_name: String,
    /// producerVersion
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/producerVersion.
    #[serde(rename = "producerVersion")]
    pub producer_version: String,
    /// repetitions
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/repetitions.
    pub repetitions: i64,
    /// responseAdapter
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/responseAdapter.
    #[serde(rename = "responseAdapter")]
    pub response_adapter: String,
    /// responseAdapterVersion
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/responseAdapterVersion.
    #[serde(rename = "responseAdapterVersion")]
    pub response_adapter_version: String,
    /// responseProtocol
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/responseProtocol.
    #[serde(rename = "responseProtocol")]
    pub response_protocol: String,
    /// schemaVersion
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/schemaVersion.
    #[serde(rename = "schemaVersion")]
    pub schema_version: String,
    /// sourceRepository
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/sourceRepository.
    #[serde(rename = "sourceRepository")]
    pub source_repository: String,
    /// timeoutMillis
    ///
    /// Semantic identity: ix://agent-ix/engineering-assurance-campaign/VO-001/timeoutMillis.
    #[serde(rename = "timeoutMillis")]
    pub timeout_millis: i64,
}

/// The deserialization shape of `MeasurementProcedure`.
///
/// It exists so that `Deserialize` can route through `try_new`: serde has
/// no post-deserialization hook, and a value that skipped the constructor
/// would be a value the contract's constraints never saw.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MeasurementProcedureWire {
    #[serde(default)]
    arguments: Option<Vec<crate::ProcedureArgument>>,
    #[serde(default)]
    environment: Option<Vec<crate::ProcedureEnvironment>>,
    #[serde(rename = "inputOrigins", default)]
    input_origins: Option<Vec<crate::ProcedureInputOrigin>>,
    #[serde(rename = "inputRolePrefixes", default)]
    input_role_prefixes: Option<Vec<crate::ProcedureInputPrefix>>,
    #[serde(default)]
    inputs: Option<Vec<crate::ProcedureArtifact>>,
    #[serde(rename = "outputTrees", default)]
    output_trees: Option<Vec<crate::ProcedureArtifact>>,
    #[serde(default)]
    outputs: Option<Vec<crate::ProcedureArtifact>>,
    #[serde(rename = "producerName")]
    producer_name: String,
    #[serde(rename = "producerVersion")]
    producer_version: String,
    repetitions: i64,
    #[serde(rename = "responseAdapter")]
    response_adapter: String,
    #[serde(rename = "responseAdapterVersion")]
    response_adapter_version: String,
    #[serde(rename = "responseProtocol")]
    response_protocol: String,
    #[serde(rename = "schemaVersion")]
    schema_version: String,
    #[serde(rename = "sourceRepository")]
    source_repository: String,
    #[serde(rename = "timeoutMillis")]
    timeout_millis: i64,
}

impl MeasurementProcedure {
    /// Builds the record, enforcing every bound and uniqueness rule the
    /// contract declares on its members. Deserialization routes through
    /// this constructor.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        arguments: Option<Vec<crate::ProcedureArgument>>,
        environment: Option<Vec<crate::ProcedureEnvironment>>,
        input_origins: Option<Vec<crate::ProcedureInputOrigin>>,
        input_role_prefixes: Option<Vec<crate::ProcedureInputPrefix>>,
        inputs: Option<Vec<crate::ProcedureArtifact>>,
        output_trees: Option<Vec<crate::ProcedureArtifact>>,
        outputs: Option<Vec<crate::ProcedureArtifact>>,
        producer_name: String,
        producer_version: String,
        repetitions: i64,
        response_adapter: String,
        response_adapter_version: String,
        response_protocol: String,
        schema_version: String,
        source_repository: String,
        timeout_millis: i64,
    ) -> Result<Self, crate::support::ValidationError> {
        Ok(Self {
            arguments,
            environment,
            input_origins,
            input_role_prefixes,
            inputs,
            output_trees,
            outputs,
            producer_name,
            producer_version,
            repetitions,
            response_adapter,
            response_adapter_version,
            response_protocol,
            schema_version,
            source_repository,
            timeout_millis,
        })
    }

    /// The non-blocking diagnostics this value carries.
    pub fn validate(&self) -> Vec<crate::support::Diagnostic> {
        Vec::new()
    }
}

impl<'de> Deserialize<'de> for MeasurementProcedure {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = MeasurementProcedureWire::deserialize(deserializer)?;
        Self::try_new(
            wire.arguments,
            wire.environment,
            wire.input_origins,
            wire.input_role_prefixes,
            wire.inputs,
            wire.output_trees,
            wire.outputs,
            wire.producer_name,
            wire.producer_version,
            wire.repetitions,
            wire.response_adapter,
            wire.response_adapter_version,
            wire.response_protocol,
            wire.schema_version,
            wire.source_repository,
            wire.timeout_millis,
        )
        .map_err(serde::de::Error::custom)
    }
}
