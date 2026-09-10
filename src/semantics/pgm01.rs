// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Typed read-only projections of historical PGM-01 records.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use super::{
    PGM01_MAPPING_PROTOCOL, SemanticConcept, SemanticError, SemanticErrorKind, validate_digest,
};

/// One traceable field mapping from a historical PGM-01 record.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pgm01Mapping {
    /// Source JSON path.
    pub source_path: String,
    /// Target semantic concept.
    pub target_concept: SemanticConcept,
    /// Target field within that concept.
    pub target_field: String,
    /// Preserved source value.
    pub value: Value,
}

/// One deliberately unmapped historical field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pgm01Unmapped {
    /// Source JSON path.
    pub source_path: String,
    /// Reason the field cannot be mapped without inventing semantics.
    pub reason: String,
}

/// Classification of a read-only PGM-01 mapping.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Pgm01Outcome {
    /// Fully compatible mapping.
    Compatible,
    /// Readable mapping with explicit limitations.
    Lossy,
    /// Unsupported or identity-mismatched input.
    Incompatible,
    /// Malformed or unreadable input.
    Unreadable,
}

/// Read-only compatibility view over immutable PGM-01 bytes.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pgm01View {
    /// Mapping protocol discriminator.
    pub mapping_version: String,
    /// Source schema version or explicit unknown value.
    pub source_schema_version: String,
    /// Source record identity or explicit placeholder.
    pub source_record_id: String,
    /// SHA-256 digest of the exact source bytes.
    pub source_digest: String,
    /// Mapping outcome.
    pub outcome: Pgm01Outcome,
    /// Preserved field mappings.
    pub mappings: Vec<Pgm01Mapping>,
    /// Explicitly unmapped fields.
    pub unmapped_fields: Vec<Pgm01Unmapped>,
    /// Declared compatibility limitations.
    pub limitations: Vec<String>,
}

impl Pgm01View {
    fn base(raw: &[u8], schema_version: impl Into<String>, record_id: impl Into<String>) -> Self {
        Self {
            mapping_version: PGM01_MAPPING_PROTOCOL.to_owned(),
            source_schema_version: schema_version.into(),
            source_record_id: record_id.into(),
            source_digest: sha256_hex(raw),
            outcome: Pgm01Outcome::Lossy,
            mappings: Vec::new(),
            unmapped_fields: Vec::new(),
            limitations: Vec::new(),
        }
    }

    fn validate_contract(self, raw: &[u8]) -> Result<Self, SemanticError> {
        if self.mapping_version != PGM01_MAPPING_PROTOCOL {
            return Err(SemanticError::new(
                SemanticErrorKind::InvalidMappingVersion,
                "generated PGM-01 view has an invalid mapping version",
            ));
        }
        if self.source_schema_version.is_empty() || self.source_record_id.is_empty() {
            return Err(SemanticError::new(
                SemanticErrorKind::EmptySourceIdentity,
                "generated PGM-01 view has an empty source identity",
            ));
        }
        if self.source_digest != sha256_hex(raw)
            || validate_digest(&self.source_digest, "generated PGM-01 source digest").is_err()
        {
            return Err(SemanticError::new(
                SemanticErrorKind::ChangedSourceDigest,
                "generated PGM-01 view changed the source identity",
            ));
        }
        if self
            .mappings
            .iter()
            .any(|item| !item.source_path.starts_with('/') || item.target_field.is_empty())
        {
            return Err(SemanticError::new(
                SemanticErrorKind::InvalidFieldMapping,
                "generated PGM-01 view has an invalid field mapping",
            ));
        }
        if self
            .unmapped_fields
            .iter()
            .any(|item| !item.source_path.starts_with('/') || item.reason.is_empty())
        {
            return Err(SemanticError::new(
                SemanticErrorKind::InvalidUnmappedField,
                "generated PGM-01 view has an invalid unmapped field",
            ));
        }
        if self.limitations.iter().any(String::is_empty) {
            return Err(SemanticError::new(
                SemanticErrorKind::EmptyLimitation,
                "generated PGM-01 view has an empty limitation",
            ));
        }
        Ok(self)
    }
}

fn mapping(
    path: impl Into<String>,
    concept: SemanticConcept,
    field: &str,
    value: Value,
) -> Pgm01Mapping {
    Pgm01Mapping {
        source_path: path.into(),
        target_concept: concept,
        target_field: field.to_owned(),
        value,
    }
}

fn unmapped(path: &str, reason: impl Into<String>) -> Pgm01Unmapped {
    Pgm01Unmapped {
        source_path: path.to_owned(),
        reason: reason.into(),
    }
}

/// Historical PGM records contain fields that this read-only view deliberately
/// does not interpret. Each typed input therefore captures those fields in an
/// explicit opaque map instead of silently treating them as semantic input.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pgm01V1 {
    record_id: String,
    subject_revision: String,
    repository: String,
    collector: Pgm01V1Collector,
    environment: Map<String, Value>,
    checks: Vec<Pgm01V1Check>,
    outputs: Vec<String>,
    limitations: Vec<String>,
    #[serde(flatten)]
    _unmapped: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pgm01V1Collector {
    implementation: String,
    implementation_revision: String,
    #[serde(flatten)]
    _unmapped: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct Pgm01V1Check {
    status: Pgm01State,
    #[serde(flatten)]
    _unmapped: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Pgm01V2 {
    record_id: String,
    subject_revision: String,
    repository: String,
    collector: Pgm01V2Collector,
    parameters: DigestObject,
    profile: DigestObject,
    commands: Vec<Pgm01V2Command>,
    overall_status: Pgm01State,
    historical_disposition: HistoricalDisposition,
    limitations: Vec<String>,
    #[serde(flatten)]
    _unmapped: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct Pgm01V2Collector {
    id: String,
    version: String,
    sha256: String,
    #[serde(flatten)]
    _unmapped: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct DigestObject {
    sha256: String,
    #[serde(flatten)]
    _unmapped: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct Pgm01V2Command {
    status: Pgm01State,
    stdout: RetainedStream,
    stderr: RetainedStream,
    #[serde(flatten)]
    _unmapped: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
struct RetainedStream {
    path: String,
    sha256: String,
    bytes: Value,
    #[serde(flatten)]
    _unmapped: Map<String, Value>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Pgm01State {
    #[serde(alias = "pass")]
    Passed,
    #[serde(alias = "fail")]
    Failed,
    Error,
    Skipped,
    Inconclusive,
    Unavailable,
}

impl Pgm01State {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Error => "error",
            Self::Skipped => "skipped",
            Self::Inconclusive => "inconclusive",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum HistoricalDisposition {
    Active,
    Retracted,
}

fn required_legacy_string<'a>(value: &'a str, path: &str) -> Result<&'a str, SemanticError> {
    if value.is_empty() {
        Err(SemanticError::new(
            SemanticErrorKind::InvalidLegacyField,
            format!("legacy field {path} must be a string"),
        ))
    } else {
        Ok(value)
    }
}

fn compatibility_identity(value: Option<&Value>, fallback: &str) -> String {
    match value {
        Some(Value::String(value)) if !value.is_empty() => value.clone(),
        Some(Value::Number(value)) if value.as_f64() != Some(0.0) => value.to_string(),
        Some(Value::Bool(true)) => "True".to_owned(),
        _ => fallback.to_owned(),
    }
}

fn required_legacy_digest<'a>(value: &'a str, path: &str) -> Result<&'a str, SemanticError> {
    let value = required_legacy_string(value, path)?;
    validate_digest(value, &format!("legacy field {path}"))?;
    Ok(value)
}

fn required_non_negative_integer(value: &Value, path: &str) -> Result<Value, SemanticError> {
    let token = value.as_number().map(ToString::to_string);
    match token.as_deref() {
        Some("-0") => Ok(Value::from(0)),
        Some(token) if token.bytes().all(|byte| byte.is_ascii_digit()) => Ok(value.clone()),
        _ => Err(SemanticError::new(
            SemanticErrorKind::InvalidLegacyInteger,
            format!("legacy field {path} must be a non-negative integer"),
        )),
    }
}

fn map_pgm01_v1(raw: &[u8], record: Pgm01V1) -> Result<Pgm01View, SemanticError> {
    let record_id = required_legacy_string(&record.record_id, "/recordId")?;
    let mut view = Pgm01View::base(raw, "quire.pgm01-evidence/v1", record_id);
    view.mappings.extend([
        mapping(
            "/recordId",
            SemanticConcept::Report,
            "legacy_record_id",
            Value::String(record_id.to_owned()),
        ),
        mapping(
            "/subjectRevision",
            SemanticConcept::VerificationExecution,
            "source_revision",
            Value::String(
                required_legacy_string(&record.subject_revision, "/subjectRevision")?.to_owned(),
            ),
        ),
        mapping(
            "/repository",
            SemanticConcept::VerificationExecution,
            "repository",
            Value::String(required_legacy_string(&record.repository, "/repository")?.to_owned()),
        ),
        mapping(
            "/collector/implementation",
            SemanticConcept::VerificationExecution,
            "producer.identity",
            Value::String(
                required_legacy_string(
                    &record.collector.implementation,
                    "/collector/implementation",
                )?
                .to_owned(),
            ),
        ),
        mapping(
            "/collector/implementationRevision",
            SemanticConcept::VerificationExecution,
            "producer.source_revision",
            Value::String(
                required_legacy_string(
                    &record.collector.implementation_revision,
                    "/collector/implementationRevision",
                )?
                .to_owned(),
            ),
        ),
        mapping(
            "/environment",
            SemanticConcept::VerificationExecution,
            "producer.environment",
            Value::Object(record.environment),
        ),
    ]);
    for (index, check) in record.checks.into_iter().enumerate() {
        view.mappings.push(mapping(
            format!("/checks/{index}/status"),
            SemanticConcept::CheckResult,
            "state",
            Value::String(check.status.as_str().to_owned()),
        ));
    }
    for (index, output) in record.outputs.into_iter().enumerate() {
        let output = required_legacy_string(&output, &format!("/outputs/{index}"))?;
        view.mappings.push(mapping(
            format!("/outputs/{index}"),
            SemanticConcept::Evidence,
            "retained_output.path",
            Value::String(output.to_owned()),
        ));
    }
    for (index, limitation) in record.limitations.into_iter().enumerate() {
        let limitation = required_legacy_string(&limitation, &format!("/limitations/{index}"))?;
        view.mappings.push(mapping(
            format!("/limitations/{index}"),
            SemanticConcept::Report,
            "limitation",
            Value::String(limitation.to_owned()),
        ));
    }
    finish_pgm01_v1(&mut view);
    Ok(view)
}

fn finish_pgm01_v1(view: &mut Pgm01View) {
    view.unmapped_fields.extend([
        unmapped(
            "/collector/version",
            "PGM-01 v1 did not record a distinct producer version",
        ),
        unmapped(
            "/configurationDigest",
            "PGM-01 v1 did not record a configuration digest",
        ),
        unmapped(
            "/definitionVersion",
            "PGM-01 v1 did not record a governing definition version",
        ),
        unmapped(
            "/outputs/*/digest",
            "output digests live in a separate checksum file, not this manifest",
        ),
        unmapped(
            "/decision",
            "the legacy merge-readiness label is not an ix-flow human decision event",
        ),
    ]);
    view.limitations.push(
        "PGM-01 v1 is readable only as a lossy view; missing identities remain missing.".to_owned(),
    );
}

fn map_pgm01_v2(raw: &[u8], record: Pgm01V2) -> Result<Pgm01View, SemanticError> {
    let record_id = required_legacy_string(&record.record_id, "/recordId")?;
    let mut view = Pgm01View::base(raw, "quire.pgm01-evidence/v2", record_id);
    view.mappings.extend([
        mapping(
            "/recordId",
            SemanticConcept::Report,
            "legacy_record_id",
            Value::String(record_id.to_owned()),
        ),
        mapping(
            "/subjectRevision",
            SemanticConcept::VerificationExecution,
            "source_revision",
            Value::String(
                required_legacy_string(&record.subject_revision, "/subjectRevision")?.to_owned(),
            ),
        ),
        mapping(
            "/repository",
            SemanticConcept::VerificationExecution,
            "repository",
            Value::String(required_legacy_string(&record.repository, "/repository")?.to_owned()),
        ),
        mapping(
            "/collector/id",
            SemanticConcept::VerificationExecution,
            "producer.identity",
            Value::String(
                required_legacy_string(&record.collector.id, "/collector/id")?.to_owned(),
            ),
        ),
        mapping(
            "/collector/version",
            SemanticConcept::VerificationExecution,
            "producer.version",
            Value::String(
                required_legacy_string(&record.collector.version, "/collector/version")?.to_owned(),
            ),
        ),
        mapping(
            "/collector/sha256",
            SemanticConcept::VerificationExecution,
            "producer.executable_digest",
            Value::String(
                required_legacy_digest(&record.collector.sha256, "/collector/sha256")?.to_owned(),
            ),
        ),
        mapping(
            "/parameters/sha256",
            SemanticConcept::VerificationExecution,
            "producer.configuration_digest",
            Value::String(
                required_legacy_digest(&record.parameters.sha256, "/parameters/sha256")?.to_owned(),
            ),
        ),
        mapping(
            "/profile/sha256",
            SemanticConcept::VerificationDefinition,
            "definition_version",
            Value::String(
                required_legacy_digest(&record.profile.sha256, "/profile/sha256")?.to_owned(),
            ),
        ),
        mapping(
            "/overallStatus",
            SemanticConcept::CheckResult,
            "state",
            Value::String(record.overall_status.as_str().to_owned()),
        ),
    ]);
    map_pgm01_v2_commands(&mut view, record.commands)?;
    for (index, limitation) in record.limitations.into_iter().enumerate() {
        let limitation = required_legacy_string(&limitation, &format!("/limitations/{index}"))?;
        view.mappings.push(mapping(
            format!("/limitations/{index}"),
            SemanticConcept::Report,
            "limitation",
            Value::String(limitation.to_owned()),
        ));
    }
    if matches!(
        record.historical_disposition,
        HistoricalDisposition::Retracted
    ) {
        view.mappings.push(mapping(
            "/historicalDisposition",
            SemanticConcept::Evidence,
            "state",
            Value::String("stale".to_owned()),
        ));
    }
    finish_pgm01_v2(&mut view);
    Ok(view)
}

fn map_pgm01_v2_commands(
    view: &mut Pgm01View,
    commands: Vec<Pgm01V2Command>,
) -> Result<(), SemanticError> {
    for (index, command) in commands.into_iter().enumerate() {
        view.mappings.push(mapping(
            format!("/commands/{index}/status"),
            SemanticConcept::CheckResult,
            "state",
            Value::String(command.status.as_str().to_owned()),
        ));
        map_retained_stream(view, index, "stdout", &command.stdout)?;
        map_retained_stream(view, index, "stderr", &command.stderr)?;
    }
    Ok(())
}

fn map_retained_stream(
    view: &mut Pgm01View,
    command_index: usize,
    stream_name: &str,
    stream: &RetainedStream,
) -> Result<(), SemanticError> {
    let path_prefix = format!("/commands/{command_index}/{stream_name}");
    let path = required_legacy_string(&stream.path, &format!("{path_prefix}/path"))?;
    let digest = required_legacy_digest(&stream.sha256, &format!("{path_prefix}/sha256"))?;
    let bytes = required_non_negative_integer(&stream.bytes, &format!("{path_prefix}/bytes"))?;
    view.mappings.extend([
        mapping(
            format!("{path_prefix}/path"),
            SemanticConcept::Evidence,
            "retained_output.path",
            Value::String(path.to_owned()),
        ),
        mapping(
            format!("{path_prefix}/sha256"),
            SemanticConcept::Evidence,
            "retained_output.sha256",
            Value::String(digest.to_owned()),
        ),
        mapping(
            format!("{path_prefix}/bytes"),
            SemanticConcept::Evidence,
            "retained_output.bytes",
            bytes,
        ),
    ]);
    Ok(())
}

fn finish_pgm01_v2(view: &mut Pgm01View) {
    view.unmapped_fields.extend([
        unmapped(
            "/environment",
            "PGM-01 v2 did not record a complete execution environment",
        ),
        unmapped(
            "/commands/*/corroboration",
            "generic transcript corroboration is not imported as a verdict",
        ),
        unmapped(
            "/quoin/status",
            "legacy intake status is not evidence sufficiency or a check result",
        ),
    ]);
    view.limitations.push("PGM-01 v2 is readable only as a lossy view; corroboration and intake status do not establish success.".to_owned());
}

/// Map immutable PGM-01 v1/v2 bytes without writing or synthesizing fields.
///
/// # Errors
///
/// Returns [`SemanticError`] only when the caller supplies an invalid expected
/// digest. Malformed, unknown, and tampered source bytes receive explicit views.
pub fn map_pgm01_bytes(
    raw: &[u8],
    expected_digest: Option<&str>,
) -> Result<Pgm01View, SemanticError> {
    let digest = sha256_hex(raw);
    if let Some(expected) = expected_digest {
        if validate_digest(expected, "expected digest").is_err() {
            return Err(SemanticError::new(
                SemanticErrorKind::InvalidExpectedDigest,
                "expected digest must be a SHA-256 digest",
            ));
        }
        if digest != expected {
            let mut view = Pgm01View::base(raw, "unknown", "tampered-source");
            view.outcome = Pgm01Outcome::Incompatible;
            view.unmapped_fields.push(unmapped(
                "/",
                "tampered source digest differs from expected identity",
            ));
            view.limitations
                .push("No field from the altered source was interpreted.".to_owned());
            return view.validate_contract(raw);
        }
    }

    let decoded: Value = match serde_json::from_slice(raw) {
        Ok(decoded) => decoded,
        Err(error) => {
            return unreadable_view(raw, format!("invalid JSON: {error}")).validate_contract(raw);
        }
    };
    let Some(record) = decoded.as_object() else {
        return unreadable_view(raw, "legacy record must be a JSON object").validate_contract(raw);
    };
    let schema_version_value = record.get("schemaVersion");
    let record_id_value = record.get("recordId");
    let schema_version = schema_version_value
        .and_then(Value::as_str)
        .map(str::to_owned);
    let source_schema_version = compatibility_identity(schema_version_value, "unknown");
    let source_record_id = compatibility_identity(record_id_value, "incompatible-source");
    let unreadable_record_id = compatibility_identity(record_id_value, "unreadable-source");

    let mapped = match schema_version.as_deref() {
        Some("quire.pgm01-evidence/v1") => serde_json::from_value::<Pgm01V1>(decoded)
            .map_err(|error| {
                SemanticError::new(
                    SemanticErrorKind::InvalidLegacyField,
                    format!("invalid PGM-01 v1 record: {error}"),
                )
            })
            .and_then(|record| map_pgm01_v1(raw, record)),
        Some("quire.pgm01-evidence/v2") => serde_json::from_value::<Pgm01V2>(decoded)
            .map_err(|error| {
                SemanticError::new(
                    SemanticErrorKind::InvalidLegacyField,
                    format!("invalid PGM-01 v2 record: {error}"),
                )
            })
            .and_then(|record| map_pgm01_v2(raw, record)),
        _ => {
            let mut view = Pgm01View::base(raw, source_schema_version, source_record_id);
            view.outcome = Pgm01Outcome::Incompatible;
            view.unmapped_fields
                .push(unmapped("/schemaVersion", "unknown PGM-01 schema version"));
            view.limitations
                .push("No unknown schema was treated as empty or current.".to_owned());
            return view.validate_contract(raw);
        }
    };

    match mapped {
        Ok(view) => view,
        Err(error) => malformed_view(raw, &source_schema_version, &unreadable_record_id, &error),
    }
    .validate_contract(raw)
}

fn unreadable_view(raw: &[u8], reason: impl Into<String>) -> Pgm01View {
    let mut view = Pgm01View::base(raw, "unknown", "unreadable-source");
    view.outcome = Pgm01Outcome::Unreadable;
    view.unmapped_fields.push(unmapped("/", reason));
    view.limitations
        .push("No legacy field was interpreted.".to_owned());
    view
}

fn malformed_view(
    raw: &[u8],
    schema_version: &str,
    record_id: &str,
    error: &SemanticError,
) -> Pgm01View {
    let mut view = Pgm01View::base(raw, schema_version, record_id);
    view.outcome = Pgm01Outcome::Unreadable;
    view.unmapped_fields.push(unmapped("/", error.to_string()));
    view.limitations
        .push("The malformed legacy record was not accepted.".to_owned());
    view
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    const RAW: &[u8] = br#"{"schemaVersion":"test","recordId":"record-1"}"#;

    fn valid_view() -> Pgm01View {
        Pgm01View::base(RAW, "test", "record-1")
    }

    fn assert_contract_error(view: Pgm01View, expected: SemanticErrorKind) {
        let error = view
            .validate_contract(RAW)
            .expect_err("invalid generated view must be refused");
        assert_eq!(error.code(), "invalid_semantic_contract");
        assert_eq!(error.kind(), expected);
    }

    #[trace("TC-103", "FR-015-AC-3")]
    #[test]
    fn tc_103_generated_pgm01_view_refuses_each_invalid_field_family() {
        valid_view()
            .validate_contract(RAW)
            .expect("baseline generated view must satisfy its contract");

        let mut view = valid_view();
        view.mapping_version = "wrong".to_owned();
        assert_contract_error(view, SemanticErrorKind::InvalidMappingVersion);

        let mut view = valid_view();
        view.source_record_id.clear();
        assert_contract_error(view, SemanticErrorKind::EmptySourceIdentity);

        let mut view = valid_view();
        view.source_digest = "0".repeat(64);
        assert_contract_error(view, SemanticErrorKind::ChangedSourceDigest);

        let mut view = valid_view();
        view.mappings.push(Pgm01Mapping {
            source_path: "not-a-json-pointer".to_owned(),
            target_concept: SemanticConcept::CheckResult,
            target_field: "result".to_owned(),
            value: Value::Null,
        });
        assert_contract_error(view, SemanticErrorKind::InvalidFieldMapping);

        let mut view = valid_view();
        view.unmapped_fields.push(Pgm01Unmapped {
            source_path: "/opaque".to_owned(),
            reason: String::new(),
        });
        assert_contract_error(view, SemanticErrorKind::InvalidUnmappedField);

        let mut view = valid_view();
        view.limitations.push(String::new());
        assert_contract_error(view, SemanticErrorKind::EmptyLimitation);
    }
}
