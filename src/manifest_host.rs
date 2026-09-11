// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Explicit-root filesystem adapter for pure module-manifest qualification.

use std::{
    io::Read,
    path::{Component, Path, PathBuf},
    process::ExitCode,
};

use cap_std::{ambient_authority, fs::Dir};
use engineering_assurance::{
    manifest::{
        MAX_MANIFEST_ARTIFACTS, MAX_MANIFEST_DOCUMENT_BYTES, MAX_MANIFEST_RESOURCE_BYTES,
        ManifestArtifactResource, ManifestQualificationInput, ManifestQualificationOutcome,
        ManifestQualificationResult, qualify_manifest,
    },
    structured_yaml::parse_unambiguous_yaml_json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const CAPABILITY: &str = "manifest-validate";
const PROTOCOL: &str = "engineering-assurance.manifest-validate/v1";
const EXPECTED_MODULE_NAME: &str = "engineering-assurance";
const EXPECTED_MODULE_VERSION: &str = "0.3.0";
const PACKAGE_DIRECTORY: &str = "engineering_assurance";
const MANIFEST_PATH: &str = "engineering_assurance/manifest.yaml";
const MODULE_SCHEMA_PATH: &str = "module-manifest.schema.json";
const EDGE_REGISTRY_PATH: &str = "manifest.yaml";
const MAX_ARTIFACT_NAME_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(crate) enum ManifestHostError {
    #[error("manifest repository root is invalid")]
    RepositoryRootInvalid,
    #[error("authoritative module root is invalid")]
    ModuleRootInvalid,
    #[error("manifest resource is invalid")]
    ResourceInvalid,
    #[error("manifest resource is missing or unreadable")]
    ResourceUnavailable,
    #[error("manifest resource exceeds its fixed byte limit")]
    ResourceTooLarge,
    #[error("manifest document cannot form the required resource inventory")]
    ManifestProjectionInvalid,
    #[error("manifest validation result could not be encoded")]
    ResultEncodingFailed,
}

impl ManifestHostError {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::RepositoryRootInvalid => "manifest_repository_root_invalid",
            Self::ModuleRootInvalid => "manifest_module_root_invalid",
            Self::ResourceInvalid => "manifest_resource_invalid",
            Self::ResourceUnavailable => "manifest_resource_unavailable",
            Self::ResourceTooLarge => "manifest_resource_too_large",
            Self::ManifestProjectionInvalid => "manifest_projection_invalid",
            Self::ResultEncodingFailed => "manifest_result_encoding_failed",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManifestHostResult {
    protocol: &'static str,
    capability: &'static str,
    #[serde(flatten)]
    qualification: ManifestQualificationResult,
}

impl ManifestHostResult {
    pub(crate) fn exit_code(&self) -> ExitCode {
        match self.qualification.outcome {
            ManifestQualificationOutcome::Accepted => ExitCode::SUCCESS,
            ManifestQualificationOutcome::Withheld => ExitCode::from(1),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ModuleManifestProjection {
    artifact_types: Vec<ArtifactProjection>,
}

#[derive(Debug, Deserialize)]
struct ArtifactProjection {
    name: String,
    frontmatter_schema_ref: String,
}

struct LoadedResource {
    artifact_name: String,
    schema_reference: String,
    schema_json: Vec<u8>,
    skeleton_markdown: Vec<u8>,
}

/// Read the fixed module bundle through explicit roots and delegate its meaning to the pure library.
pub(crate) fn execute(
    repository_root: &Path,
    module_root: &Path,
) -> Result<ManifestHostResult, ManifestHostError> {
    let repository = open_root(repository_root, ManifestHostError::RepositoryRootInvalid)?;
    let module = open_root(module_root, ManifestHostError::ModuleRootInvalid)?;
    let manifest_yaml = read_file(
        &repository,
        Path::new(MANIFEST_PATH),
        MAX_MANIFEST_DOCUMENT_BYTES,
    )?;
    let manifest_schema_json = read_file(
        &module,
        Path::new(MODULE_SCHEMA_PATH),
        MAX_MANIFEST_DOCUMENT_BYTES,
    )?;
    let edge_registry_yaml = read_file(
        &module,
        Path::new(EDGE_REGISTRY_PATH),
        MAX_MANIFEST_DOCUMENT_BYTES,
    )?;
    let resources = load_resources(&repository, &manifest_yaml)?;
    let resources = resources
        .iter()
        .map(|resource| ManifestArtifactResource {
            artifact_name: &resource.artifact_name,
            schema_reference: &resource.schema_reference,
            schema_json: &resource.schema_json,
            skeleton_markdown: &resource.skeleton_markdown,
        })
        .collect::<Vec<_>>();
    let qualification = qualify_manifest(&ManifestQualificationInput {
        expected_name: EXPECTED_MODULE_NAME,
        expected_version: EXPECTED_MODULE_VERSION,
        manifest_yaml: &manifest_yaml,
        manifest_schema_json: &manifest_schema_json,
        edge_registry_yaml: &edge_registry_yaml,
        resources: &resources,
    })
    .map_err(|error| match error {
        engineering_assurance::manifest::ManifestQualificationError::ManifestTooLarge
        | engineering_assurance::manifest::ManifestQualificationError::ManifestSchemaTooLarge
        | engineering_assurance::manifest::ManifestQualificationError::EdgeRegistryTooLarge
        | engineering_assurance::manifest::ManifestQualificationError::ArtifactDocumentTooLarge
        | engineering_assurance::manifest::ManifestQualificationError::ArtifactPopulationTooLarge
        | engineering_assurance::manifest::ManifestQualificationError::ArtifactBytesTooLarge => {
            ManifestHostError::ResourceTooLarge
        }
    })?;
    Ok(ManifestHostResult {
        protocol: PROTOCOL,
        capability: CAPABILITY,
        qualification,
    })
}

pub(crate) fn to_json_line(result: &ManifestHostResult) -> Result<Vec<u8>, ManifestHostError> {
    let mut bytes =
        serde_json::to_vec(result).map_err(|_| ManifestHostError::ResultEncodingFailed)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn load_resources(
    repository: &Dir,
    manifest_yaml: &[u8],
) -> Result<Vec<LoadedResource>, ManifestHostError> {
    let value = parse_unambiguous_yaml_json(manifest_yaml)
        .ok_or(ManifestHostError::ManifestProjectionInvalid)?;
    let projection: ModuleManifestProjection =
        serde_json::from_value(value).map_err(|_| ManifestHostError::ManifestProjectionInvalid)?;
    if projection.artifact_types.len() > MAX_MANIFEST_ARTIFACTS {
        return Err(ManifestHostError::ResourceTooLarge);
    }
    let mut total_resource_bytes = 0_usize;
    let mut resources = Vec::with_capacity(projection.artifact_types.len());
    for artifact in projection.artifact_types {
        let schema_path = resource_path(&artifact.frontmatter_schema_ref)?;
        let skeleton_path = skeleton_path(&artifact.name)?;
        let schema_json = read_file(repository, &schema_path, MAX_MANIFEST_DOCUMENT_BYTES)?;
        let skeleton_markdown = read_file(repository, &skeleton_path, MAX_MANIFEST_DOCUMENT_BYTES)?;
        total_resource_bytes = total_resource_bytes
            .checked_add(schema_json.len())
            .and_then(|value| value.checked_add(skeleton_markdown.len()))
            .ok_or(ManifestHostError::ResourceTooLarge)?;
        if total_resource_bytes > MAX_MANIFEST_RESOURCE_BYTES {
            return Err(ManifestHostError::ResourceTooLarge);
        }
        resources.push(LoadedResource {
            artifact_name: artifact.name,
            schema_reference: artifact.frontmatter_schema_ref,
            schema_json,
            skeleton_markdown,
        });
    }
    Ok(resources)
}

fn resource_path(reference: &str) -> Result<PathBuf, ManifestHostError> {
    if reference.is_empty()
        || reference.starts_with('/')
        || reference.ends_with('/')
        || reference.contains('\\')
        || reference.chars().any(char::is_control)
        || has_windows_drive_root(reference)
    {
        return Err(ManifestHostError::ResourceInvalid);
    }
    let relative = Path::new(reference);
    normal_components(relative).ok_or(ManifestHostError::ResourceInvalid)?;
    Ok(Path::new(PACKAGE_DIRECTORY).join(relative))
}

fn skeleton_path(name: &str) -> Result<PathBuf, ManifestHostError> {
    if name.is_empty()
        || name.len() > MAX_ARTIFACT_NAME_BYTES
        || name.contains(['/', '\\'])
        || name.chars().any(char::is_control)
    {
        return Err(ManifestHostError::ResourceInvalid);
    }
    let path = Path::new(PACKAGE_DIRECTORY)
        .join("skeletons")
        .join(format!("{name}.md"));
    normal_components(&path).ok_or(ManifestHostError::ResourceInvalid)?;
    Ok(path)
}

fn has_windows_drive_root(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/'
}

fn open_root(path: &Path, error: ManifestHostError) -> Result<Dir, ManifestHostError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| error)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(error);
    }
    let canonical = std::fs::canonicalize(path).map_err(|_| error)?;
    Dir::open_ambient_dir(&canonical, ambient_authority()).map_err(|_| error)
}

fn read_file(root: &Dir, relative: &Path, maximum: usize) -> Result<Vec<u8>, ManifestHostError> {
    require_file_path(root, relative)?;
    let file = root
        .open(relative)
        .map_err(|_| ManifestHostError::ResourceUnavailable)?;
    let metadata = file
        .metadata()
        .map_err(|_| ManifestHostError::ResourceUnavailable)?;
    if !metadata.is_file() {
        return Err(ManifestHostError::ResourceInvalid);
    }
    if metadata.len() > u64::try_from(maximum).unwrap_or(u64::MAX) {
        return Err(ManifestHostError::ResourceTooLarge);
    }
    let mut bytes = Vec::new();
    file.take(u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| ManifestHostError::ResourceUnavailable)?;
    if bytes.len() > maximum {
        Err(ManifestHostError::ResourceTooLarge)
    } else {
        Ok(bytes)
    }
}

fn require_file_path(root: &Dir, relative: &Path) -> Result<(), ManifestHostError> {
    let components = normal_components(relative).ok_or(ManifestHostError::ResourceInvalid)?;
    if components.is_empty() {
        return Err(ManifestHostError::ResourceInvalid);
    }
    let mut current = PathBuf::new();
    for (index, component) in components.iter().enumerate() {
        current.push(component);
        let metadata = root
            .symlink_metadata(&current)
            .map_err(|_| ManifestHostError::ResourceUnavailable)?;
        if metadata.file_type().is_symlink() {
            return Err(ManifestHostError::ResourceInvalid);
        }
        if index + 1 == components.len() {
            if !metadata.is_file() {
                return Err(ManifestHostError::ResourceInvalid);
            }
        } else if !metadata.is_dir() {
            return Err(ManifestHostError::ResourceInvalid);
        }
    }
    Ok(())
}

fn normal_components(path: &Path) -> Option<Vec<&std::ffi::OsStr>> {
    path.components()
        .map(|component| match component {
            Component::Normal(value) => Some(value),
            Component::RootDir
            | Component::CurDir
            | Component::ParentDir
            | Component::Prefix(_) => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use ix_trace_rs::trace;
    use tempfile::TempDir;

    use super::*;

    fn fixture() -> (TempDir, TempDir) {
        let repository = TempDir::new().expect("repository fixture must be creatable");
        let module = TempDir::new().expect("module fixture must be creatable");
        let package = repository.path().join(PACKAGE_DIRECTORY);
        fs::create_dir_all(package.join("schemas"))
            .expect("schema fixture directory must be creatable");
        fs::create_dir_all(package.join("skeletons"))
            .expect("skeleton fixture directory must be creatable");
        fs::write(
            package.join("manifest.yaml"),
            "name: engineering-assurance\nversion: 0.3.0\nartifact_types:\n  - name: sample\n    frontmatter_schema_ref: schemas/sample.schema.json\n    allowed_links: [supports]\n    body_extraction:\n      yield_pattern:\n        match:\n          body:\n            after_heading: Required\n            required: true\n",
        )
        .expect("manifest fixture must be writable");
        fs::write(
            package.join("schemas/sample.schema.json"),
            b"{\"type\":\"object\"}",
        )
        .expect("artifact schema fixture must be writable");
        fs::write(
            package.join("skeletons/sample.md"),
            "---\nkind: sample\n---\n## Required\n",
        )
        .expect("skeleton fixture must be writable");
        fs::write(
            module.path().join(MODULE_SCHEMA_PATH),
            b"{\"type\":\"object\"}",
        )
        .expect("module schema fixture must be writable");
        fs::write(
            module.path().join(EDGE_REGISTRY_PATH),
            "edge_types:\n  supports: {}\n",
        )
        .expect("edge registry fixture must be writable");
        (repository, module)
    }

    #[test]
    #[trace("TC-131", "FR-017-AC-8", "FR-017-CON-3", "FR-014-AC-2")]
    fn tc_131_qualifies_the_explicit_module_bundle() {
        let (repository, module) = fixture();
        let result = execute(repository.path(), module.path()).expect("fixture must qualify");
        assert_eq!(
            result.qualification.outcome,
            ManifestQualificationOutcome::Accepted
        );
        assert_eq!(result.protocol, PROTOCOL);
        assert_eq!(
            to_json_line(&result).expect("result must encode").last(),
            Some(&b'\n')
        );
    }

    #[test]
    #[trace("TC-131", "FR-017-AC-8", "FR-017-CON-3")]
    fn tc_131_refuses_an_escaping_declared_resource() {
        let (repository, module) = fixture();
        let manifest = repository.path().join(MANIFEST_PATH);
        let text = fs::read_to_string(&manifest).expect("manifest must be readable");
        fs::write(
            &manifest,
            text.replacen("schemas/sample.schema.json", "../outside.schema.json", 1),
        )
        .expect("manifest fixture must be writable");
        assert!(matches!(
            execute(repository.path(), module.path()),
            Err(ManifestHostError::ResourceInvalid)
        ));
    }

    #[test]
    #[trace("TC-131", "FR-017-AC-8", "FR-017-CON-3")]
    fn tc_131_refuses_missing_special_and_over_limit_resources() {
        let (repository, module) = fixture();
        let schema = repository
            .path()
            .join(PACKAGE_DIRECTORY)
            .join("schemas/sample.schema.json");
        fs::remove_file(&schema).expect("fixture schema must be removable");
        assert!(matches!(
            execute(repository.path(), module.path()),
            Err(ManifestHostError::ResourceUnavailable)
        ));

        fs::create_dir(&schema).expect("fixture special resource must be creatable");
        assert!(matches!(
            execute(repository.path(), module.path()),
            Err(ManifestHostError::ResourceInvalid)
        ));

        fs::remove_dir(&schema).expect("fixture special resource must be removable");
        fs::write(
            &schema,
            vec![b'x'; MAX_MANIFEST_DOCUMENT_BYTES.saturating_add(1)],
        )
        .expect("over-limit fixture resource must be writable");
        assert!(matches!(
            execute(repository.path(), module.path()),
            Err(ManifestHostError::ResourceTooLarge)
        ));
    }

    #[cfg(unix)]
    #[test]
    #[trace("TC-131", "FR-017-AC-8", "FR-017-CON-3")]
    fn tc_131_refuses_a_linked_schema_before_opening_it() {
        use std::os::unix::fs::symlink;

        let (repository, module) = fixture();
        let schema = repository
            .path()
            .join(PACKAGE_DIRECTORY)
            .join("schemas/sample.schema.json");
        fs::remove_file(&schema).expect("fixture schema must be removable");
        symlink("/dev/null", schema).expect("fixture schema link must be creatable");
        assert!(matches!(
            execute(repository.path(), module.path()),
            Err(ManifestHostError::ResourceInvalid)
        ));
    }

    #[cfg(unix)]
    #[test]
    #[trace("TC-131", "FR-017-AC-8", "FR-017-CON-3")]
    fn tc_131_refuses_linked_repository_and_module_roots() {
        use std::os::unix::fs::symlink;

        let (repository, module) = fixture();
        let aliases = TempDir::new().expect("alias fixture directory must be creatable");
        let root_alias = aliases.path().join("repository");
        let module_alias = aliases.path().join("module");
        symlink(repository.path(), &root_alias).expect("repository alias must be creatable");
        symlink(module.path(), &module_alias).expect("module alias must be creatable");
        assert!(matches!(
            execute(&root_alias, module.path()),
            Err(ManifestHostError::RepositoryRootInvalid)
        ));
        assert!(matches!(
            execute(repository.path(), &module_alias),
            Err(ManifestHostError::ModuleRootInvalid)
        ));
    }
}
