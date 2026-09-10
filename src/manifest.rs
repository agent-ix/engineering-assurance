// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure module-manifest, schema, and skeleton qualification.
//!
//! The caller supplies the authoritative schemas and all resource bytes. This
//! module does not discover schemas, traverse files, invoke Quire, fetch schema
//! references, select a package format, or access environment or clock state.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use thiserror::Error;

use crate::structured_yaml::parse_unambiguous_yaml_json;

/// Maximum bytes accepted for any one manifest, schema, or skeleton document.
pub const MAX_MANIFEST_DOCUMENT_BYTES: usize = 1_048_576;

/// Maximum artifact resource records accepted by one request.
pub const MAX_MANIFEST_ARTIFACTS: usize = 256;

/// Maximum combined schema and skeleton bytes across artifact resources.
pub const MAX_MANIFEST_RESOURCE_BYTES: usize = 33_554_432;

const MAX_SCHEMA_REFERENCE_BYTES: usize = 4_096;
const MAX_SAFE_LABEL_BYTES: usize = 256;

/// One caller-supplied artifact schema and skeleton pair.
#[derive(Clone, Copy, Debug)]
pub struct ManifestArtifactResource<'a> {
    /// Artifact type name this pair is intended to serve.
    pub artifact_name: &'a str,
    /// Manifest-relative schema reference this pair is intended to serve.
    pub schema_reference: &'a str,
    /// JSON Schema bytes for the artifact frontmatter.
    pub schema_json: &'a [u8],
    /// Markdown skeleton bytes carrying YAML frontmatter.
    pub skeleton_markdown: &'a [u8],
}

/// Complete caller-supplied input for pure module-manifest qualification.
#[derive(Clone, Copy, Debug)]
pub struct ManifestQualificationInput<'a> {
    /// Exact module name expected by the caller.
    pub expected_name: &'a str,
    /// Exact module version expected by the caller.
    pub expected_version: &'a str,
    /// Candidate module manifest YAML bytes.
    pub manifest_yaml: &'a [u8],
    /// Authoritative module-manifest JSON Schema bytes.
    pub manifest_schema_json: &'a [u8],
    /// YAML manifest whose edge registry owns admitted link verbs.
    pub edge_registry_yaml: &'a [u8],
    /// Caller-enumerated artifact schema and skeleton pairs.
    pub resources: &'a [ManifestArtifactResource<'a>],
}

/// Whether the supplied module bundle satisfies the manifest contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestQualificationOutcome {
    /// No qualification finding exists.
    Accepted,
    /// At least one typed qualification finding exists.
    Withheld,
}

/// Closed module-manifest finding taxonomy in canonical order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifestFindingCategory {
    /// The module manifest is not unambiguous YAML convertible to JSON.
    ManifestYamlInvalid,
    /// The authoritative module-manifest schema is not JSON.
    ManifestSchemaJsonInvalid,
    /// The authoritative module-manifest schema or one of its references is invalid.
    ManifestSchemaInvalid,
    /// The module manifest does not satisfy the authoritative schema.
    ManifestSchemaMismatch,
    /// Schema-valid manifest fields cannot form the required qualification projection.
    ManifestProjectionInvalid,
    /// The manifest module name differs from the caller's expected identity.
    ManifestNameMismatch,
    /// The manifest module version differs from the caller's expected identity.
    ManifestVersionMismatch,
    /// The edge-registry manifest is not an unambiguous required YAML mapping.
    EdgeRegistryInvalid,
    /// One artifact type name appears more than once in the manifest.
    ArtifactNameDuplicate,
    /// One declared allowed-link verb is absent from the edge registry.
    UnregisteredEdge,
    /// A declared frontmatter schema reference is unsafe or non-normalized.
    SchemaReferenceInvalid,
    /// No caller-supplied resource exists for one declared artifact.
    ResourceMissing,
    /// More than one caller-supplied resource names one declared artifact.
    ResourceDuplicate,
    /// A caller-supplied resource names no unique declared artifact.
    ResourceUnexpected,
    /// A resource's schema reference differs from its manifest declaration.
    ResourceReferenceMismatch,
    /// One artifact schema is not JSON.
    ArtifactSchemaJsonInvalid,
    /// One artifact schema, meta-schema, or offline reference is invalid.
    ArtifactSchemaInvalid,
    /// A skeleton lacks an exact Markdown YAML frontmatter envelope.
    SkeletonFrontmatterMissing,
    /// A skeleton frontmatter document is ambiguous or malformed YAML.
    SkeletonFrontmatterInvalid,
    /// A skeleton frontmatter document does not satisfy its artifact schema.
    SkeletonSchemaMismatch,
    /// A declared body locator is not required.
    LocatorNotRequired,
    /// A skeleton lacks one exact required level-two heading.
    RequiredHeadingMissing,
}

/// One deterministic finding that never contains source bytes or an unsafe reference.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestFinding {
    /// Closed reason module acceptance is withheld.
    category: ManifestFindingCategory,
    /// Safe artifact label when one is applicable, otherwise absent.
    artifact: Option<String>,
    /// Safe normalized schema reference when one is applicable, otherwise absent.
    schema_reference: Option<String>,
    /// Safe registered-edge candidate when one is applicable, otherwise absent.
    edge: Option<String>,
}

impl ManifestFinding {
    /// Return the closed reason module acceptance is withheld.
    #[must_use]
    pub const fn category(&self) -> ManifestFindingCategory {
        self.category
    }

    /// Return the safe artifact label when one applies.
    #[must_use]
    pub fn artifact(&self) -> Option<&str> {
        self.artifact.as_deref()
    }

    /// Return the safe normalized schema reference when one applies.
    #[must_use]
    pub fn schema_reference(&self) -> Option<&str> {
        self.schema_reference.as_deref()
    }

    /// Return the safe edge label when one applies.
    #[must_use]
    pub fn edge(&self) -> Option<&str> {
        self.edge.as_deref()
    }

    fn global(category: ManifestFindingCategory) -> Self {
        Self {
            category,
            artifact: None,
            schema_reference: None,
            edge: None,
        }
    }

    fn for_artifact(category: ManifestFindingCategory, artifact: &str) -> Self {
        Self {
            category,
            artifact: safe_label(artifact),
            schema_reference: None,
            edge: None,
        }
    }

    fn for_reference(
        category: ManifestFindingCategory,
        artifact: &str,
        schema_reference: &str,
    ) -> Self {
        Self {
            category,
            artifact: safe_label(artifact),
            schema_reference: is_safe_schema_reference(schema_reference)
                .then(|| schema_reference.to_owned()),
            edge: None,
        }
    }

    fn for_edge(artifact: &str, edge: &str) -> Self {
        Self {
            category: ManifestFindingCategory::UnregisteredEdge,
            artifact: safe_label(artifact),
            schema_reference: None,
            edge: safe_label(edge),
        }
    }
}

/// Result of pure module-manifest qualification.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestQualificationResult {
    /// Accepted only when the finding list is empty.
    pub outcome: ManifestQualificationOutcome,
    /// Findings in canonical category, artifact, reference, and edge order.
    pub findings: Vec<ManifestFinding>,
}

/// Resource refusal raised before schema compilation begins.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ManifestQualificationError {
    /// The module manifest exceeds the per-document byte ceiling.
    #[error("module manifest exceeds the document byte limit")]
    ManifestTooLarge,
    /// The authoritative module-manifest schema exceeds the byte ceiling.
    #[error("module-manifest schema exceeds the document byte limit")]
    ManifestSchemaTooLarge,
    /// The edge-registry manifest exceeds the per-document byte ceiling.
    #[error("edge-registry manifest exceeds the document byte limit")]
    EdgeRegistryTooLarge,
    /// At least one artifact schema or skeleton exceeds the byte ceiling.
    #[error("artifact schema or skeleton exceeds the document byte limit")]
    ArtifactDocumentTooLarge,
    /// The artifact resource population exceeds the fixed ceiling.
    #[error("artifact resource population exceeds the resource limit")]
    ArtifactPopulationTooLarge,
    /// Combined artifact schema and skeleton bytes exceed the fixed ceiling.
    #[error("combined artifact resource bytes exceed the resource limit")]
    ArtifactBytesTooLarge,
}

impl ManifestQualificationError {
    /// Return the stable machine category for this refusal.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ManifestTooLarge => "module_manifest_too_large",
            Self::ManifestSchemaTooLarge => "module_manifest_schema_too_large",
            Self::EdgeRegistryTooLarge => "edge_registry_manifest_too_large",
            Self::ArtifactDocumentTooLarge => "artifact_document_too_large",
            Self::ArtifactPopulationTooLarge => "artifact_population_too_large",
            Self::ArtifactBytesTooLarge => "artifact_resource_bytes_too_large",
        }
    }
}

#[derive(Debug, Deserialize)]
struct ManifestProjection {
    name: String,
    version: String,
    artifact_types: Vec<ArtifactProjection>,
}

#[derive(Debug, Deserialize)]
struct ArtifactProjection {
    name: String,
    frontmatter_schema_ref: String,
    allowed_links: Vec<String>,
    body_extraction: BodyExtractionProjection,
}

#[derive(Debug, Deserialize)]
struct BodyExtractionProjection {
    yield_pattern: YieldPatternProjection,
}

#[derive(Debug, Deserialize)]
struct YieldPatternProjection {
    #[serde(rename = "match")]
    locators: BTreeMap<String, LocatorProjection>,
}

#[derive(Debug, Deserialize)]
struct LocatorProjection {
    after_heading: String,
    required: bool,
}

#[derive(Debug, Deserialize)]
struct EdgeRegistryProjection {
    edge_types: BTreeMap<String, JsonValue>,
}

/// Validate one complete caller-supplied module-manifest bundle without I/O.
///
/// # Errors
///
/// Refuses resource populations and documents above the declared ceilings
/// before compiling any JSON Schema.
pub fn qualify_manifest(
    input: &ManifestQualificationInput<'_>,
) -> Result<ManifestQualificationResult, ManifestQualificationError> {
    validate_resource_limits(input)?;

    let mut findings = Vec::new();
    let Some(manifest_json) = parse_unambiguous_yaml_json(input.manifest_yaml) else {
        findings.push(ManifestFinding::global(
            ManifestFindingCategory::ManifestYamlInvalid,
        ));
        return Ok(finish(findings));
    };
    let Some(manifest_validator) = compile_schema(
        input.manifest_schema_json,
        ManifestFindingCategory::ManifestSchemaJsonInvalid,
        ManifestFindingCategory::ManifestSchemaInvalid,
        &mut findings,
        None,
    ) else {
        return Ok(finish(findings));
    };
    if manifest_validator.validate(&manifest_json).is_err() {
        findings.push(ManifestFinding::global(
            ManifestFindingCategory::ManifestSchemaMismatch,
        ));
        return Ok(finish(findings));
    }
    let Ok(manifest) = serde_json::from_value::<ManifestProjection>(manifest_json) else {
        findings.push(ManifestFinding::global(
            ManifestFindingCategory::ManifestProjectionInvalid,
        ));
        return Ok(finish(findings));
    };

    validate_identity(input, &manifest, &mut findings);
    let registry = parse_edge_registry(input.edge_registry_yaml, &mut findings);
    let artifacts = unique_artifacts(&manifest.artifact_types, &mut findings);
    if let Some(registry) = registry.as_ref() {
        validate_edges(&artifacts, registry, &mut findings);
    }
    validate_resources(&artifacts, input.resources, &mut findings);

    Ok(finish(findings))
}

fn validate_resource_limits(
    input: &ManifestQualificationInput<'_>,
) -> Result<(), ManifestQualificationError> {
    if input.manifest_yaml.len() > MAX_MANIFEST_DOCUMENT_BYTES {
        return Err(ManifestQualificationError::ManifestTooLarge);
    }
    if input.manifest_schema_json.len() > MAX_MANIFEST_DOCUMENT_BYTES {
        return Err(ManifestQualificationError::ManifestSchemaTooLarge);
    }
    if input.edge_registry_yaml.len() > MAX_MANIFEST_DOCUMENT_BYTES {
        return Err(ManifestQualificationError::EdgeRegistryTooLarge);
    }
    if input.resources.len() > MAX_MANIFEST_ARTIFACTS {
        return Err(ManifestQualificationError::ArtifactPopulationTooLarge);
    }

    let mut combined = 0_usize;
    for resource in input.resources {
        if resource.schema_json.len() > MAX_MANIFEST_DOCUMENT_BYTES
            || resource.skeleton_markdown.len() > MAX_MANIFEST_DOCUMENT_BYTES
        {
            return Err(ManifestQualificationError::ArtifactDocumentTooLarge);
        }
        combined = combined
            .checked_add(resource.schema_json.len())
            .and_then(|size| size.checked_add(resource.skeleton_markdown.len()))
            .ok_or(ManifestQualificationError::ArtifactBytesTooLarge)?;
        if combined > MAX_MANIFEST_RESOURCE_BYTES {
            return Err(ManifestQualificationError::ArtifactBytesTooLarge);
        }
    }
    Ok(())
}

fn validate_identity(
    input: &ManifestQualificationInput<'_>,
    manifest: &ManifestProjection,
    findings: &mut Vec<ManifestFinding>,
) {
    if manifest.name != input.expected_name {
        findings.push(ManifestFinding::global(
            ManifestFindingCategory::ManifestNameMismatch,
        ));
    }
    if manifest.version != input.expected_version {
        findings.push(ManifestFinding::global(
            ManifestFindingCategory::ManifestVersionMismatch,
        ));
    }
}

fn parse_edge_registry(
    bytes: &[u8],
    findings: &mut Vec<ManifestFinding>,
) -> Option<EdgeRegistryProjection> {
    let Some(json) = parse_unambiguous_yaml_json(bytes) else {
        findings.push(ManifestFinding::global(
            ManifestFindingCategory::EdgeRegistryInvalid,
        ));
        return None;
    };
    let Ok(registry) = serde_json::from_value(json) else {
        findings.push(ManifestFinding::global(
            ManifestFindingCategory::EdgeRegistryInvalid,
        ));
        return None;
    };
    Some(registry)
}

fn unique_artifacts<'a>(
    declared: &'a [ArtifactProjection],
    findings: &mut Vec<ManifestFinding>,
) -> BTreeMap<&'a str, &'a ArtifactProjection> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for artifact in declared {
        counts
            .entry(&artifact.name)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    let mut unique = BTreeMap::new();
    for artifact in declared {
        if counts[artifact.name.as_str()] == 1 {
            unique.insert(artifact.name.as_str(), artifact);
        }
    }
    findings.extend(
        counts
            .iter()
            .filter(|(_, count)| **count > 1)
            .map(|(name, _)| {
                ManifestFinding::for_artifact(ManifestFindingCategory::ArtifactNameDuplicate, name)
            }),
    );
    unique
}

fn validate_edges(
    artifacts: &BTreeMap<&str, &ArtifactProjection>,
    registry: &EdgeRegistryProjection,
    findings: &mut Vec<ManifestFinding>,
) {
    for artifact in artifacts.values() {
        let unique_edges: BTreeSet<_> = artifact.allowed_links.iter().map(String::as_str).collect();
        findings.extend(
            unique_edges
                .into_iter()
                .filter(|edge| !registry.edge_types.contains_key(*edge))
                .map(|edge| ManifestFinding::for_edge(&artifact.name, edge)),
        );
    }
}

fn validate_resources(
    artifacts: &BTreeMap<&str, &ArtifactProjection>,
    resources: &[ManifestArtifactResource<'_>],
    findings: &mut Vec<ManifestFinding>,
) {
    let mut by_artifact = BTreeMap::<&str, Vec<&ManifestArtifactResource<'_>>>::new();
    for resource in resources {
        by_artifact
            .entry(resource.artifact_name)
            .or_default()
            .push(resource);
    }

    for name in by_artifact.keys() {
        if !artifacts.contains_key(name) {
            findings.push(ManifestFinding::for_artifact(
                ManifestFindingCategory::ResourceUnexpected,
                name,
            ));
        }
    }

    for (name, artifact) in artifacts {
        let Some(supplied) = by_artifact.get(name) else {
            findings.push(ManifestFinding::for_artifact(
                ManifestFindingCategory::ResourceMissing,
                name,
            ));
            continue;
        };
        if supplied.len() != 1 {
            findings.push(ManifestFinding::for_artifact(
                ManifestFindingCategory::ResourceDuplicate,
                name,
            ));
            continue;
        }
        validate_artifact_resource(artifact, supplied[0], findings);
    }
}

fn validate_artifact_resource(
    artifact: &ArtifactProjection,
    resource: &ManifestArtifactResource<'_>,
    findings: &mut Vec<ManifestFinding>,
) {
    if !is_safe_schema_reference(&artifact.frontmatter_schema_ref) {
        findings.push(ManifestFinding::for_artifact(
            ManifestFindingCategory::SchemaReferenceInvalid,
            &artifact.name,
        ));
        return;
    }
    if resource.schema_reference != artifact.frontmatter_schema_ref {
        findings.push(ManifestFinding::for_reference(
            ManifestFindingCategory::ResourceReferenceMismatch,
            &artifact.name,
            resource.schema_reference,
        ));
        return;
    }

    let Some(validator) = compile_schema(
        resource.schema_json,
        ManifestFindingCategory::ArtifactSchemaJsonInvalid,
        ManifestFindingCategory::ArtifactSchemaInvalid,
        findings,
        Some((&artifact.name, &artifact.frontmatter_schema_ref)),
    ) else {
        return;
    };
    let Some(frontmatter) = split_frontmatter(resource.skeleton_markdown) else {
        findings.push(ManifestFinding::for_reference(
            ManifestFindingCategory::SkeletonFrontmatterMissing,
            &artifact.name,
            &artifact.frontmatter_schema_ref,
        ));
        return;
    };
    let Some(frontmatter_json) = parse_unambiguous_yaml_json(frontmatter) else {
        findings.push(ManifestFinding::for_reference(
            ManifestFindingCategory::SkeletonFrontmatterInvalid,
            &artifact.name,
            &artifact.frontmatter_schema_ref,
        ));
        return;
    };
    if validator.validate(&frontmatter_json).is_err() {
        findings.push(ManifestFinding::for_reference(
            ManifestFindingCategory::SkeletonSchemaMismatch,
            &artifact.name,
            &artifact.frontmatter_schema_ref,
        ));
    }
    validate_headings(artifact, resource.skeleton_markdown, findings);
}

fn validate_headings(
    artifact: &ArtifactProjection,
    skeleton: &[u8],
    findings: &mut Vec<ManifestFinding>,
) {
    let Ok(text) = std::str::from_utf8(skeleton) else {
        return;
    };
    for locator in artifact.body_extraction.yield_pattern.locators.values() {
        if !locator.required {
            findings.push(ManifestFinding::for_artifact(
                ManifestFindingCategory::LocatorNotRequired,
                &artifact.name,
            ));
            continue;
        }
        let expected = format!("## {}", locator.after_heading);
        if !text.lines().any(|line| line == expected) {
            findings.push(ManifestFinding::for_artifact(
                ManifestFindingCategory::RequiredHeadingMissing,
                &artifact.name,
            ));
        }
    }
}

fn compile_schema(
    bytes: &[u8],
    json_invalid: ManifestFindingCategory,
    schema_invalid: ManifestFindingCategory,
    findings: &mut Vec<ManifestFinding>,
    artifact: Option<(&str, &str)>,
) -> Option<jsonschema::Validator> {
    let Ok(schema) = serde_json::from_slice::<JsonValue>(bytes) else {
        findings.push(schema_finding(json_invalid, artifact));
        return None;
    };
    if jsonschema::meta::options().validate(&schema).is_err() {
        findings.push(schema_finding(schema_invalid, artifact));
        return None;
    }
    let Ok(validator) = jsonschema::options()
        .offline()
        .should_validate_formats(true)
        .build(&schema)
    else {
        findings.push(schema_finding(schema_invalid, artifact));
        return None;
    };
    Some(validator)
}

fn schema_finding(
    category: ManifestFindingCategory,
    artifact: Option<(&str, &str)>,
) -> ManifestFinding {
    artifact.map_or_else(
        || ManifestFinding::global(category),
        |(name, reference)| ManifestFinding::for_reference(category, name, reference),
    )
}

fn split_frontmatter(bytes: &[u8]) -> Option<&[u8]> {
    let remainder = bytes.strip_prefix(b"---\n")?;
    if let Some(index) = remainder.windows(5).position(|window| window == b"\n---\n") {
        return Some(&remainder[..index]);
    }
    remainder.strip_suffix(b"\n---")
}

fn is_safe_schema_reference(reference: &str) -> bool {
    if reference.is_empty()
        || reference.len() > MAX_SCHEMA_REFERENCE_BYTES
        || reference.starts_with('/')
        || reference.ends_with('/')
        || reference.contains('\\')
        || reference.chars().any(char::is_control)
        || has_windows_drive_root(reference)
    {
        return false;
    }
    !reference
        .split('/')
        .any(|component| component.is_empty() || component == "." || component == "..")
}

fn has_windows_drive_root(reference: &str) -> bool {
    let bytes = reference.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/'
}

fn safe_label(label: &str) -> Option<String> {
    (!label.is_empty()
        && label.len() <= MAX_SAFE_LABEL_BYTES
        && !label.chars().any(char::is_control))
    .then(|| label.to_owned())
}

fn finish(mut findings: Vec<ManifestFinding>) -> ManifestQualificationResult {
    findings.sort_unstable();
    findings.dedup();
    let outcome = if findings.is_empty() {
        ManifestQualificationOutcome::Accepted
    } else {
        ManifestQualificationOutcome::Withheld
    };
    ManifestQualificationResult { outcome, findings }
}
