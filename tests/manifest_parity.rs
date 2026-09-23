// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure qualification and adverse coverage for module manifests.
//!
//! The pure classifier under test (`qualify_manifest`) never discovers or
//! copies a schema; FR-017-AC-7 requires only that it *consume* whatever
//! module-manifest schema and edge registry a caller supplies. This file
//! used to source that "caller-supplied" schema from an installed copy of
//! `spec-artifacts-iso`'s `module-manifest.schema.json` — a redistribution of
//! filament-core-service's FR-035 schema that `spec-artifacts-iso` retired in
//! PLAT-902 (agent-ix/spec-artifacts-iso#42, commit dffc449) specifically
//! because shipping that copy let downstream repositories compare their
//! manifests to bytes maintained by the redistributor instead of to
//! filament-core-service's real contract. EA does not restore, vendor, or
//! otherwise recreate that copy anywhere (repo policy: no exceptions).
//!
//! The fixtures below are a minimal, locally authored module-manifest schema
//! and edge registry that exist only to exercise `qualify_manifest`'s own
//! typed-finding behavior (malformed YAML, schema mismatch, resource
//! matching, skeleton frontmatter/heading rules, resource ceilings, and
//! result-order invariance). They are not authoritative, do not resemble the
//! FR-035 schema, and are not a copy of any other repository's schema or
//! registry.
//!
//! What this file therefore does NOT assert: that EA's real
//! `engineering_assurance/manifest.yaml` conforms to filament-core-service's
//! authoritative FR-035 schema, or that its link verbs appear in
//! spec-artifacts-iso's real edge registry. That real-conformance assertion
//! belongs to `tests/manifest_host_cli.rs`'s
//! `tc_121_real_manifest_conforms_to_the_authoritative_schema_and_edge_registry`
//! (PLAT-994), which runs the `manifest-validate` host adapter — the only
//! part of this repository with filesystem access — against explicit,
//! operator-supplied schema and registry roots. It is `#[ignore]`d by
//! default because no root is discovered or defaulted (FR-017 Behavior) and
//! the schema root is a private-repo checkout this public repository's
//! default CI cannot assume.
//! `tests/test_module.py::test_quire_accepts_every_skeleton_without_diagnostics`
//! runs the real `quire validate --module` engine, whose module load rejects
//! a mistyped manifest field but is not a full FR-035 schema check (it
//! accepts an unknown top-level key the schema forbids) and reports every
//! link verb as `UnknownEdgeType`. The `manifest-validate` host adapter
//! (FR-017-AC-8, `src/manifest_host.rs`) reads the schema and edge registry
//! from two explicit roots — since PLAT-902 no single upstream module root
//! holds both files, so the adapter takes them separately instead of
//! assembling one. This crate never searches a sibling checkout, an
//! installed cache, or the network for a schema (FR-017 Behavior).

use engineering_assurance::manifest::{
    MAX_MANIFEST_ARTIFACTS, MAX_MANIFEST_DOCUMENT_BYTES, MAX_MANIFEST_RESOURCE_BYTES,
    ManifestArtifactResource, ManifestFindingCategory, ManifestQualificationError,
    ManifestQualificationInput, ManifestQualificationOutcome, qualify_manifest,
};
use ix_trace_rs::trace;
use std::path::Path;

/// Minimal locally authored module-manifest schema fixture.
///
/// This is deliberately permissive (`{"type": "object"}`): the pure
/// classifier's identity, edge, and resource checks run against the parsed
/// manifest projection regardless of how strict the caller-supplied schema
/// is, so a permissive fixture is sufficient to exercise every typed finding
/// in this file while making no claim to be, or resemble, an authoritative
/// schema owned by another repository.
const MANIFEST_SCHEMA_FIXTURE: &[u8] = br#"{"type": "object"}"#;

/// Minimal locally authored edge registry fixture.
///
/// Lists exactly the link verbs `engineering_assurance/manifest.yaml`
/// declares today (`governs`, `references`, `measures`, `realizes`,
/// `supports`), not the full cross-ecosystem edge-type vocabulary that
/// `spec-artifacts-iso` FR-004 owns. Because it mirrors EA's own verbs, the
/// accepted-module test cannot detect an upstream registry that drops a verb
/// EA uses; that drift check is not performed in this repository.
const EDGE_REGISTRY_FIXTURE: &[u8] = b"edge_types:\n  governs: {}\n  references: {}\n  measures: {}\n  realizes: {}\n  supports: {}\n";

#[derive(Clone)]
struct OwnedResource {
    artifact_name: String,
    schema_reference: String,
    schema_json: Vec<u8>,
    skeleton_markdown: Vec<u8>,
}

struct OwnedBundle {
    manifest_yaml: Vec<u8>,
    manifest_schema_json: Vec<u8>,
    edge_registry_yaml: Vec<u8>,
    resources: Vec<OwnedResource>,
}

impl OwnedBundle {
    fn resource_views(&self) -> Vec<ManifestArtifactResource<'_>> {
        self.resources
            .iter()
            .map(|resource| ManifestArtifactResource {
                artifact_name: &resource.artifact_name,
                schema_reference: &resource.schema_reference,
                schema_json: &resource.schema_json,
                skeleton_markdown: &resource.skeleton_markdown,
            })
            .collect()
    }
}

fn retained_bundle() -> OwnedBundle {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let package = repository.join("engineering_assurance");
    let artifacts = [
        (
            "AssuranceProfile",
            "schemas/assurance-profile-frontmatter.schema.json",
        ),
        (
            "MeasurementPlan",
            "schemas/measurement-plan-frontmatter.schema.json",
        ),
        (
            "ArchitectureDescription",
            "schemas/architecture-description-frontmatter.schema.json",
        ),
        (
            "ComponentAssuranceContract",
            "schemas/component-assurance-contract-frontmatter.schema.json",
        ),
        (
            "AssuranceArgument",
            "schemas/assurance-argument-frontmatter.schema.json",
        ),
    ];
    OwnedBundle {
        manifest_yaml: std::fs::read(package.join("manifest.yaml"))
            .expect("retained manifest must be readable"),
        manifest_schema_json: MANIFEST_SCHEMA_FIXTURE.to_vec(),
        edge_registry_yaml: EDGE_REGISTRY_FIXTURE.to_vec(),
        resources: artifacts
            .into_iter()
            .map(|(name, reference)| OwnedResource {
                artifact_name: name.to_owned(),
                schema_reference: reference.to_owned(),
                schema_json: std::fs::read(package.join(reference))
                    .expect("artifact schema must be readable"),
                skeleton_markdown: std::fs::read(
                    package.join("skeletons").join(format!("{name}.md")),
                )
                .expect("artifact skeleton must be readable"),
            })
            .collect(),
    }
}

fn input<'a>(
    bundle: &'a OwnedBundle,
    resources: &'a [ManifestArtifactResource<'a>],
) -> ManifestQualificationInput<'a> {
    ManifestQualificationInput {
        expected_name: "engineering-assurance",
        expected_version: "0.3.1",
        manifest_yaml: &bundle.manifest_yaml,
        manifest_schema_json: &bundle.manifest_schema_json,
        edge_registry_yaml: &bundle.edge_registry_yaml,
        resources,
    }
}

fn categories(
    bundle: &OwnedBundle,
    resources: &[ManifestArtifactResource<'_>],
) -> Vec<ManifestFindingCategory> {
    qualify_manifest(&input(bundle, resources))
        .expect("bounded fixture")
        .findings
        .into_iter()
        .map(|finding| finding.category())
        .collect()
}

fn replace(bytes: &[u8], old: &str, new: &str) -> Vec<u8> {
    let text = String::from_utf8(bytes.to_vec()).expect("text fixture");
    assert!(
        text.contains(old),
        "fixture does not contain mutation target"
    );
    text.replacen(old, new, 1).into_bytes()
}

#[test]
#[trace("TC-121", "FR-017-AC-7", "FR-017-CON-3")]
fn retained_valid_module_is_accepted_by_the_pure_qualifier() {
    let bundle = retained_bundle();
    let resources = bundle.resource_views();
    let result = qualify_manifest(&input(&bundle, &resources)).expect("bounded retained bundle");
    assert_eq!(result.outcome, ManifestQualificationOutcome::Accepted);
    assert!(result.findings.is_empty());
}

#[test]
#[trace("TC-121", "FR-017-AC-7", "FR-017-CON-3")]
fn manifest_schema_identity_and_registry_failures_are_typed() {
    let baseline = retained_bundle();

    let mut malformed = retained_bundle();
    malformed.manifest_yaml = b"[not: yaml".to_vec();
    let views = malformed.resource_views();
    assert_eq!(
        categories(&malformed, &views),
        [ManifestFindingCategory::ManifestYamlInvalid]
    );

    let mut duplicate = retained_bundle();
    duplicate.manifest_yaml = replace(
        &duplicate.manifest_yaml,
        "name: engineering-assurance",
        "name: engineering-assurance\nname: replacement",
    );
    let views = duplicate.resource_views();
    assert_eq!(
        categories(&duplicate, &views),
        [ManifestFindingCategory::ManifestYamlInvalid]
    );

    let mut merged = retained_bundle();
    merged.manifest_yaml = replace(
        &merged.manifest_yaml,
        "name: engineering-assurance",
        "defaults: &defaults\n  name: engineering-assurance\n<<: *defaults",
    );
    let views = merged.resource_views();
    assert_eq!(
        categories(&merged, &views),
        [ManifestFindingCategory::ManifestYamlInvalid]
    );

    let mut invalid_json = retained_bundle();
    invalid_json.manifest_schema_json = b"{not json".to_vec();
    let views = invalid_json.resource_views();
    assert_eq!(
        categories(&invalid_json, &views),
        [ManifestFindingCategory::ManifestSchemaJsonInvalid]
    );

    let mut invalid_schema = retained_bundle();
    invalid_schema.manifest_schema_json = br#"{
        "$schema":"http://json-schema.org/draft-07/schema#",
        "type":7
    }"#
    .to_vec();
    let views = invalid_schema.resource_views();
    assert_eq!(
        categories(&invalid_schema, &views),
        [ManifestFindingCategory::ManifestSchemaInvalid]
    );

    let mut mismatch = retained_bundle();
    mismatch.manifest_schema_json = br#"{
        "$schema":"http://json-schema.org/draft-07/schema#",
        "type":"string"
    }"#
    .to_vec();
    let views = mismatch.resource_views();
    assert_eq!(
        categories(&mismatch, &views),
        [ManifestFindingCategory::ManifestSchemaMismatch]
    );

    let mut identity = retained_bundle();
    identity.manifest_yaml = replace(
        &identity.manifest_yaml,
        "name: engineering-assurance\nversion: 0.3.1",
        "name: changed\nversion: 9.9.9",
    );
    let views = identity.resource_views();
    assert!(categories(&identity, &views).starts_with(&[
        ManifestFindingCategory::ManifestNameMismatch,
        ManifestFindingCategory::ManifestVersionMismatch,
    ]));

    let mut registry = baseline;
    registry.edge_registry_yaml = b"edge_types: {}\n".to_vec();
    let views = registry.resource_views();
    let result = qualify_manifest(&input(&registry, &views)).expect("bounded fixture");
    assert_eq!(result.outcome, ManifestQualificationOutcome::Withheld);
    assert!(
        result
            .findings
            .iter()
            .any(|finding| finding.category() == ManifestFindingCategory::UnregisteredEdge)
    );

    let mut duplicate_registry = retained_bundle();
    duplicate_registry.edge_registry_yaml = b"edge_types: {}\nedge_types: {}\n".to_vec();
    let views = duplicate_registry.resource_views();
    assert!(
        categories(&duplicate_registry, &views)
            .contains(&ManifestFindingCategory::EdgeRegistryInvalid)
    );

    let mut merged_registry = retained_bundle();
    merged_registry.edge_registry_yaml =
        b"defaults: &defaults\n  edge_types: {}\n<<: *defaults\n".to_vec();
    let views = merged_registry.resource_views();
    assert!(
        categories(&merged_registry, &views)
            .contains(&ManifestFindingCategory::EdgeRegistryInvalid)
    );
}

#[test]
#[trace("TC-121", "FR-017-AC-7", "FR-017-CON-3")]
fn artifact_resource_matching_failures_are_typed() {
    let mut missing = retained_bundle();
    missing.resources.pop();
    let views = missing.resource_views();
    assert!(categories(&missing, &views).contains(&ManifestFindingCategory::ResourceMissing));

    let mut duplicate = retained_bundle();
    duplicate.resources.push(duplicate.resources[0].clone());
    let views = duplicate.resource_views();
    assert!(categories(&duplicate, &views).contains(&ManifestFindingCategory::ResourceDuplicate));

    let mut unexpected = retained_bundle();
    let mut extra = unexpected.resources[0].clone();
    extra.artifact_name = "UndeclaredArtifact".to_owned();
    unexpected.resources.push(extra);
    let views = unexpected.resource_views();
    assert!(categories(&unexpected, &views).contains(&ManifestFindingCategory::ResourceUnexpected));

    let mut reference_mismatch = retained_bundle();
    reference_mismatch.resources[0].schema_reference = "schemas/other.json".to_owned();
    let views = reference_mismatch.resource_views();
    assert!(
        categories(&reference_mismatch, &views)
            .contains(&ManifestFindingCategory::ResourceReferenceMismatch)
    );

    let mut unsafe_reference = retained_bundle();
    unsafe_reference.manifest_yaml = replace(
        &unsafe_reference.manifest_yaml,
        "schemas/assurance-profile-frontmatter.schema.json",
        "../private.schema.json",
    );
    unsafe_reference.resources[0].schema_reference = "../private.schema.json".to_owned();
    let views = unsafe_reference.resource_views();
    let result = qualify_manifest(&input(&unsafe_reference, &views)).expect("bounded fixture");
    assert!(
        result.findings.iter().any(|finding| {
            finding.category() == ManifestFindingCategory::SchemaReferenceInvalid
        })
    );
    assert!(
        !serde_json::to_string(&result)
            .expect("result serializes")
            .contains("../private")
    );
}

#[test]
#[trace("TC-121", "FR-017-AC-7", "FR-017-CON-3")]
fn artifact_schema_frontmatter_and_heading_failures_are_typed() {
    let mut schema_json = retained_bundle();
    schema_json.resources[0].schema_json = b"{not json".to_vec();
    let views = schema_json.resource_views();
    assert!(
        categories(&schema_json, &views)
            .contains(&ManifestFindingCategory::ArtifactSchemaJsonInvalid)
    );

    let mut schema_invalid = retained_bundle();
    schema_invalid.resources[0].schema_json = br#"{
        "$schema":"http://json-schema.org/draft-07/schema#",
        "type":7
    }"#
    .to_vec();
    let views = schema_invalid.resource_views();
    assert!(
        categories(&schema_invalid, &views)
            .contains(&ManifestFindingCategory::ArtifactSchemaInvalid)
    );

    let mut unavailable_reference = retained_bundle();
    let unavailable_url = ["https:", "//example.invalid/unavailable.schema.json"].concat();
    unavailable_reference.resources[0].schema_json = format!(
        r#"{{
        "$schema":"http://json-schema.org/draft-07/schema#",
        "$ref":"{unavailable_url}"
    }}"#
    )
    .into_bytes();
    let views = unavailable_reference.resource_views();
    assert!(
        categories(&unavailable_reference, &views)
            .contains(&ManifestFindingCategory::ArtifactSchemaInvalid)
    );

    let mut invalid_known_format = retained_bundle();
    invalid_known_format.resources[0].schema_json = replace(
        &invalid_known_format.resources[0].schema_json,
        "\"title\": { \"type\": \"string\", \"minLength\": 1 }",
        "\"title\": { \"type\": \"string\", \"format\": \"date-time\" }",
    );
    let views = invalid_known_format.resource_views();
    assert!(
        categories(&invalid_known_format, &views)
            .contains(&ManifestFindingCategory::SkeletonSchemaMismatch)
    );
}

#[test]
#[trace("TC-121", "FR-017-AC-7", "FR-017-CON-3")]
fn artifact_frontmatter_and_heading_failures_are_typed() {
    let mut no_frontmatter = retained_bundle();
    no_frontmatter.resources[0].skeleton_markdown = b"# No frontmatter\n".to_vec();
    let views = no_frontmatter.resource_views();
    assert!(
        categories(&no_frontmatter, &views)
            .contains(&ManifestFindingCategory::SkeletonFrontmatterMissing)
    );

    let mut duplicate_frontmatter = retained_bundle();
    duplicate_frontmatter.resources[0].skeleton_markdown = replace(
        &duplicate_frontmatter.resources[0].skeleton_markdown,
        "type: AssuranceProfile",
        "type: AssuranceProfile\ntype: MeasurementPlan",
    );
    let views = duplicate_frontmatter.resource_views();
    assert!(
        categories(&duplicate_frontmatter, &views)
            .contains(&ManifestFindingCategory::SkeletonFrontmatterInvalid)
    );

    let mut merged_frontmatter = retained_bundle();
    merged_frontmatter.resources[0].skeleton_markdown = replace(
        &merged_frontmatter.resources[0].skeleton_markdown,
        "type: AssuranceProfile",
        "defaults: &defaults\n  type: AssuranceProfile\n<<: *defaults",
    );
    let views = merged_frontmatter.resource_views();
    assert!(
        categories(&merged_frontmatter, &views)
            .contains(&ManifestFindingCategory::SkeletonFrontmatterInvalid)
    );

    let mut schema_mismatch = retained_bundle();
    schema_mismatch.resources[0].skeleton_markdown = replace(
        &schema_mismatch.resources[0].skeleton_markdown,
        "type: AssuranceProfile",
        "type: MeasurementPlan",
    );
    let views = schema_mismatch.resource_views();
    assert!(
        categories(&schema_mismatch, &views)
            .contains(&ManifestFindingCategory::SkeletonSchemaMismatch)
    );

    let mut optional_locator = retained_bundle();
    optional_locator.manifest_yaml = replace(
        &optional_locator.manifest_yaml,
        "required: true",
        "required: false",
    );
    let views = optional_locator.resource_views();
    assert!(
        categories(&optional_locator, &views)
            .contains(&ManifestFindingCategory::LocatorNotRequired)
    );

    let mut missing_heading = retained_bundle();
    missing_heading.resources[0].skeleton_markdown = replace(
        &missing_heading.resources[0].skeleton_markdown,
        "## Decision Boundary",
        "## Different Boundary",
    );
    let views = missing_heading.resource_views();
    assert!(
        categories(&missing_heading, &views)
            .contains(&ManifestFindingCategory::RequiredHeadingMissing)
    );

    let mut inexact_heading = retained_bundle();
    inexact_heading.resources[0].skeleton_markdown = replace(
        &inexact_heading.resources[0].skeleton_markdown,
        "## Decision Boundary",
        "## Decision Boundary appendix",
    );
    let views = inexact_heading.resource_views();
    assert!(
        categories(&inexact_heading, &views)
            .contains(&ManifestFindingCategory::RequiredHeadingMissing)
    );
}

#[test]
#[trace("TC-121", "FR-017-AC-7", "FR-017-CON-3")]
fn resource_limits_accept_exact_boundaries_and_refuse_over_limit() {
    let baseline = retained_bundle();
    let exact_document = vec![b' '; MAX_MANIFEST_DOCUMENT_BYTES];
    let mut document = retained_bundle();
    document.manifest_yaml = exact_document;
    let views = document.resource_views();
    assert!(qualify_manifest(&input(&document, &views)).is_ok());

    let mut oversized = retained_bundle();
    oversized.manifest_yaml = vec![b' '; MAX_MANIFEST_DOCUMENT_BYTES + 1];
    let views = oversized.resource_views();
    assert_eq!(
        qualify_manifest(&input(&oversized, &views)),
        Err(ManifestQualificationError::ManifestTooLarge)
    );

    let tiny = b"{}";
    let skeleton = b"---\n{}\n---\n";
    let artifact_names: Vec<_> = (0..MAX_MANIFEST_ARTIFACTS)
        .map(|index| format!("Artifact{index}"))
        .collect();
    let exact_resources: Vec<_> = artifact_names
        .iter()
        .map(|artifact_name| ManifestArtifactResource {
            artifact_name,
            schema_reference: "schemas/a.json",
            schema_json: tiny,
            skeleton_markdown: skeleton,
        })
        .collect();
    let exact_input = ManifestQualificationInput {
        resources: &exact_resources,
        ..input(&baseline, &[])
    };
    assert!(qualify_manifest(&exact_input).is_ok());

    let mut too_many = exact_resources;
    too_many.push(ManifestArtifactResource {
        artifact_name: "Overflow",
        schema_reference: "schemas/a.json",
        schema_json: tiny,
        skeleton_markdown: skeleton,
    });
    let over_input = ManifestQualificationInput {
        resources: &too_many,
        ..input(&baseline, &[])
    };
    assert_eq!(
        qualify_manifest(&over_input),
        Err(ManifestQualificationError::ArtifactPopulationTooLarge)
    );

    let shared = vec![b' '; MAX_MANIFEST_RESOURCE_BYTES / MAX_MANIFEST_ARTIFACTS];
    let exact_combined: Vec<_> = (0..MAX_MANIFEST_ARTIFACTS)
        .map(|_| ManifestArtifactResource {
            artifact_name: "extra",
            schema_reference: "schemas/a.json",
            schema_json: &shared,
            skeleton_markdown: &[],
        })
        .collect();
    let exact_input = ManifestQualificationInput {
        resources: &exact_combined,
        ..input(&baseline, &[])
    };
    assert!(qualify_manifest(&exact_input).is_ok());
    let one_byte = *b" ";
    let mut over_combined = exact_combined;
    over_combined[0].skeleton_markdown = &one_byte;
    let over_input = ManifestQualificationInput {
        resources: &over_combined,
        ..input(&baseline, &[])
    };
    assert_eq!(
        qualify_manifest(&over_input),
        Err(ManifestQualificationError::ArtifactBytesTooLarge)
    );
}

#[test]
#[trace("TC-121", "FR-017-AC-7", "FR-017-CON-3")]
fn resource_order_does_not_change_the_qualified_result() {
    let bundle = retained_bundle();
    let resources = bundle.resource_views();
    let first = qualify_manifest(&input(&bundle, &resources)).expect("bounded fixture");
    let reversed: Vec<_> = resources.iter().rev().copied().collect();
    let second = qualify_manifest(&input(&bundle, &reversed)).expect("bounded fixture");
    assert_eq!(first, second);
}
