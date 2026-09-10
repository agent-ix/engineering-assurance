// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded installed-bundle validation for the package-audit binary.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use engineering_assurance::{
    package_membership::is_safe_package_member_path, structured_yaml::parse_unambiguous_yaml_json,
};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::package_archive::{
    MAX_ARCHIVE_ENTRIES, MAX_ARCHIVE_EXPANDED_BYTES, MAX_ARCHIVE_FILE_BYTES,
};

const CANONICAL_SKILL: &str = "engineering_assurance/skills/assurance-onboarding";
const CANONICAL_SKILLS: &str = "engineering_assurance/skills";
const WORKFLOWS: [&str; 4] = [
    "architecture-evaluation",
    "assurance-intake",
    "change-assurance",
    "measurement-promotion",
];
const REQUIRED_MODULE_ROOTS: [&str; 5] = [
    "contracts",
    "fixtures",
    "manifest.yaml",
    "schemas",
    "skeletons",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InstalledBundle {
    pub(crate) canonical_files: BTreeMap<String, Vec<u8>>,
}

impl InstalledBundle {
    pub(crate) fn canonical_file_count(&self) -> usize {
        self.canonical_files.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(crate) enum InstalledError {
    #[error("installed bundle root is invalid")]
    RootInvalid,
    #[error("installed bundle exceeds the entry limit")]
    EntryPopulationTooLarge,
    #[error("installed bundle contains an invalid path")]
    PathInvalid,
    #[error("installed bundle contains a linked or special entry")]
    EntryKindInvalid,
    #[error("installed bundle contains an oversized file")]
    FileTooLarge,
    #[error("installed bundle exceeds the aggregate byte limit")]
    TotalBytesTooLarge,
    #[error("installed bundle cannot be read")]
    InspectionFailed,
    #[error("installed module root is incomplete")]
    ModuleRootIncomplete,
    #[error("installed canonical skill population is invalid")]
    CanonicalSkillInvalid,
    #[error("installed workflow population is invalid")]
    WorkflowPopulationInvalid,
    #[error("installed host manifest is invalid")]
    HostManifestInvalid,
    #[error("installed host manifest does not resolve the canonical skill")]
    HostTargetInvalid,
    #[error("installed canonical and pilot workflows differ")]
    WorkflowMismatch,
}

pub(crate) fn inspect(
    bundle_root: &Path,
    module_root: &Path,
) -> Result<InstalledBundle, InstalledError> {
    let root = selected_root(bundle_root)?;
    let module = selected_child(&root, module_root)?;
    let tree = inspect_tree(&root)?;
    validate_module_root(&module)?;
    validate_discovery(&root, &tree)?;
    validate_workflows(&tree)?;
    let prefix = format!("{CANONICAL_SKILL}/");
    let canonical_files = tree
        .files
        .iter()
        .filter_map(|(path, bytes)| {
            path.strip_prefix(&prefix)
                .map(|relative| (relative.to_owned(), bytes.clone()))
        })
        .collect();
    Ok(InstalledBundle { canonical_files })
}

fn selected_root(root: &Path) -> Result<PathBuf, InstalledError> {
    let metadata = fs::symlink_metadata(root).map_err(|_| InstalledError::RootInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(InstalledError::RootInvalid);
    }
    fs::canonicalize(root).map_err(|_| InstalledError::RootInvalid)
}

fn selected_child(root: &Path, child: &Path) -> Result<PathBuf, InstalledError> {
    let child = fs::canonicalize(child).map_err(|_| InstalledError::ModuleRootIncomplete)?;
    if !child.starts_with(root) {
        return Err(InstalledError::ModuleRootIncomplete);
    }
    Ok(child)
}

fn validate_module_root(module: &Path) -> Result<(), InstalledError> {
    for (required, directory) in REQUIRED_MODULE_ROOTS
        .into_iter()
        .map(|path| (path, path != "manifest.yaml"))
    {
        let metadata = fs::symlink_metadata(module.join(required))
            .map_err(|_| InstalledError::ModuleRootIncomplete)?;
        if metadata.file_type().is_symlink()
            || (directory && !metadata.is_dir())
            || (!directory && !metadata.is_file())
        {
            return Err(InstalledError::ModuleRootIncomplete);
        }
    }
    Ok(())
}

fn validate_discovery(root: &Path, tree: &TreeSnapshot) -> Result<(), InstalledError> {
    let skill_path = format!("{CANONICAL_SKILL}/SKILL.md");
    if !tree.files.contains_key(&skill_path) {
        return Err(InstalledError::CanonicalSkillInvalid);
    }
    let skills_prefix = format!("{CANONICAL_SKILLS}/");
    let skill_directories = tree
        .directories
        .iter()
        .filter_map(|path| path.strip_prefix(&skills_prefix))
        .filter(|relative| !relative.contains('/'))
        .collect::<BTreeSet<_>>();
    if skill_directories != BTreeSet::from(["assurance-onboarding"]) {
        return Err(InstalledError::CanonicalSkillInvalid);
    }

    let surfaces = [
        HostSurface::plugin(".claude-plugin/plugin.json"),
        HostSurface::plugin(".codex-plugin/plugin.json"),
        HostSurface::plugin(".github/plugin/plugin.json"),
        HostSurface::opencode("opencode.json"),
    ];
    let canonical_root = root.join(CANONICAL_SKILLS);
    for surface in surfaces {
        let bytes = tree
            .files
            .get(surface.path)
            .ok_or(InstalledError::HostManifestInvalid)?;
        let value: Value =
            serde_json::from_slice(bytes).map_err(|_| InstalledError::HostManifestInvalid)?;
        let object = value
            .as_object()
            .ok_or(InstalledError::HostManifestInvalid)?;
        surface.validate_keys(object)?;
        let source = surface.skill_source(object)?;
        let target = normalized_host_target(source)?;
        let resolved =
            fs::canonicalize(root.join(target)).map_err(|_| InstalledError::HostTargetInvalid)?;
        if resolved != canonical_root {
            return Err(InstalledError::HostTargetInvalid);
        }
    }
    Ok(())
}

fn validate_workflows(tree: &TreeSnapshot) -> Result<(), InstalledError> {
    let canonical_prefix = format!("{CANONICAL_SKILL}/workflows/");
    let canonical = tree
        .directories
        .iter()
        .filter_map(|path| path.strip_prefix(&canonical_prefix))
        .filter(|relative| !relative.contains('/'))
        .collect::<BTreeSet<_>>();
    if canonical != BTreeSet::from(WORKFLOWS) {
        return Err(InstalledError::WorkflowPopulationInvalid);
    }
    for workflow in WORKFLOWS {
        let canonical_path = format!("{CANONICAL_SKILL}/workflows/{workflow}/def.yaml");
        let pilot_path = format!("pilots/assurance-workflows/workflows/{workflow}/def.yaml");
        let canonical_bytes = tree
            .files
            .get(&canonical_path)
            .ok_or(InstalledError::WorkflowPopulationInvalid)?;
        let pilot_bytes = tree
            .files
            .get(&pilot_path)
            .ok_or(InstalledError::WorkflowPopulationInvalid)?;
        let canonical_value =
            parse_unambiguous_yaml_json(canonical_bytes).ok_or(InstalledError::WorkflowMismatch)?;
        let pilot_value =
            parse_unambiguous_yaml_json(pilot_bytes).ok_or(InstalledError::WorkflowMismatch)?;
        if canonical_value != pilot_value {
            return Err(InstalledError::WorkflowMismatch);
        }
    }
    Ok(())
}

struct HostSurface {
    path: &'static str,
    kind: HostKind,
}

impl HostSurface {
    const fn plugin(path: &'static str) -> Self {
        Self {
            path,
            kind: HostKind::Plugin,
        }
    }

    const fn opencode(path: &'static str) -> Self {
        Self {
            path,
            kind: HostKind::Opencode,
        }
    }

    fn validate_keys(&self, object: &Map<String, Value>) -> Result<(), InstalledError> {
        let allowed = match self.kind {
            HostKind::Plugin => ["name", "version", "description", "skills"].as_slice(),
            HostKind::Opencode => ["$schema", "skills"].as_slice(),
        };
        if object.keys().any(|key| !allowed.contains(&key.as_str())) {
            return Err(InstalledError::HostManifestInvalid);
        }
        Ok(())
    }

    fn skill_source<'a>(&self, object: &'a Map<String, Value>) -> Result<&'a str, InstalledError> {
        let skills = object
            .get("skills")
            .ok_or(InstalledError::HostManifestInvalid)?;
        let source = match self.kind {
            HostKind::Plugin => skills.as_str(),
            HostKind::Opencode => skills
                .as_array()
                .filter(|values| values.len() == 1)
                .and_then(|values| values[0].as_str()),
        }
        .ok_or(InstalledError::HostManifestInvalid)?;
        if source.trim().is_empty() {
            return Err(InstalledError::HostManifestInvalid);
        }
        Ok(source)
    }
}

#[derive(Clone, Copy)]
enum HostKind {
    Plugin,
    Opencode,
}

fn normalized_host_target(source: &str) -> Result<&str, InstalledError> {
    let source = source
        .strip_prefix("./")
        .ok_or(InstalledError::HostTargetInvalid)?;
    let source = source.strip_suffix('/').unwrap_or(source);
    if !is_safe_package_member_path(source) {
        return Err(InstalledError::HostTargetInvalid);
    }
    Ok(source)
}

#[derive(Default)]
pub(crate) struct TreeSnapshot {
    pub(crate) files: BTreeMap<String, Vec<u8>>,
    directories: BTreeSet<String>,
}

pub(crate) fn inspect_tree(root: &Path) -> Result<TreeSnapshot, InstalledError> {
    let mut snapshot = TreeSnapshot::default();
    let mut entries_seen = 0_usize;
    let mut total_bytes = 0_usize;
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(directory)
            .map_err(|_| InstalledError::InspectionFailed)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| InstalledError::InspectionFailed)?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries.into_iter().rev() {
            count_entry(&mut entries_seen)?;
            let path = entry.path();
            let metadata =
                fs::symlink_metadata(&path).map_err(|_| InstalledError::InspectionFailed)?;
            if metadata.file_type().is_symlink() {
                return Err(InstalledError::EntryKindInvalid);
            }
            let relative = portable_relative(root, &path)?;
            if metadata.is_dir() {
                snapshot.directories.insert(relative);
                pending.push(path);
                continue;
            }
            if !metadata.is_file() {
                return Err(InstalledError::EntryKindInvalid);
            }
            let length =
                usize::try_from(metadata.len()).map_err(|_| InstalledError::FileTooLarge)?;
            validate_file_length(length)?;
            add_file_bytes(&mut total_bytes, length)?;
            let file = fs::File::open(&path).map_err(|_| InstalledError::InspectionFailed)?;
            let limit = u64::try_from(MAX_ARCHIVE_FILE_BYTES)
                .unwrap_or(u64::MAX)
                .saturating_add(1);
            let mut bytes = Vec::with_capacity(length);
            file.take(limit)
                .read_to_end(&mut bytes)
                .map_err(|_| InstalledError::InspectionFailed)?;
            if bytes.len() != length {
                return Err(InstalledError::InspectionFailed);
            }
            snapshot.files.insert(relative, bytes);
        }
    }
    Ok(snapshot)
}

fn count_entry(entries: &mut usize) -> Result<(), InstalledError> {
    *entries = entries
        .checked_add(1)
        .ok_or(InstalledError::EntryPopulationTooLarge)?;
    if *entries > MAX_ARCHIVE_ENTRIES {
        return Err(InstalledError::EntryPopulationTooLarge);
    }
    Ok(())
}

fn validate_file_length(length: usize) -> Result<(), InstalledError> {
    if length > MAX_ARCHIVE_FILE_BYTES {
        return Err(InstalledError::FileTooLarge);
    }
    Ok(())
}

fn add_file_bytes(total: &mut usize, length: usize) -> Result<(), InstalledError> {
    *total = total
        .checked_add(length)
        .ok_or(InstalledError::TotalBytesTooLarge)?;
    if *total > MAX_ARCHIVE_EXPANDED_BYTES {
        return Err(InstalledError::TotalBytesTooLarge);
    }
    Ok(())
}

fn portable_relative(root: &Path, path: &Path) -> Result<String, InstalledError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| InstalledError::PathInvalid)?;
    let mut parts = Vec::new();
    for component in relative.components() {
        let value = component
            .as_os_str()
            .to_str()
            .ok_or(InstalledError::PathInvalid)?;
        parts.push(value);
    }
    let joined = parts.join("/");
    if !is_safe_package_member_path(&joined) {
        return Err(InstalledError::PathInvalid);
    }
    Ok(joined)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use ix_trace_rs::trace;

    fn write(path: &Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn complete_bundle() -> tempfile::TempDir {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        for required in ["contracts", "fixtures", "schemas", "skeletons"] {
            fs::create_dir_all(root.join("engineering_assurance").join(required)).unwrap();
        }
        write(
            &root.join("engineering_assurance/manifest.yaml"),
            b"name: engineering-assurance\n",
        );
        write(
            &root.join(format!("{CANONICAL_SKILL}/SKILL.md")),
            b"---\nname: assurance-onboarding\n---\n",
        );
        for workflow in WORKFLOWS {
            let definition = format!("name: {workflow}\nversion: 1\n");
            write(
                &root.join(format!("{CANONICAL_SKILL}/workflows/{workflow}/def.yaml")),
                definition.as_bytes(),
            );
            write(
                &root.join(format!(
                    "pilots/assurance-workflows/workflows/{workflow}/def.yaml"
                )),
                definition.as_bytes(),
            );
        }
        let plugin = br#"{"name":"engineering-assurance","version":"0.2.0","description":"fixture","skills":"./engineering_assurance/skills/"}"#;
        for manifest in [
            ".claude-plugin/plugin.json",
            ".codex-plugin/plugin.json",
            ".github/plugin/plugin.json",
        ] {
            write(&root.join(manifest), plugin);
        }
        write(
            &root.join("opencode.json"),
            br#"{"skills":["./engineering_assurance/skills"]}"#,
        );
        directory
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn host_target_normalization_is_closed() {
        assert_eq!(
            normalized_host_target("./engineering_assurance/skills/"),
            Ok("engineering_assurance/skills")
        );
        assert_eq!(
            normalized_host_target("engineering_assurance/skills"),
            Err(InstalledError::HostTargetInvalid)
        );
        assert_eq!(
            normalized_host_target("./../outside"),
            Err(InstalledError::HostTargetInvalid)
        );
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn installed_discovery_and_workflow_failures_are_typed() {
        let accepted = complete_bundle();
        let result = inspect(
            accepted.path(),
            &accepted.path().join("engineering_assurance"),
        )
        .unwrap();
        assert_eq!(result.canonical_file_count(), 5);

        let extra_skill = complete_bundle();
        write(
            &extra_skill
                .path()
                .join("engineering_assurance/skills/other/file.txt"),
            b"not a canonical skill",
        );
        assert_eq!(
            inspect(
                extra_skill.path(),
                &extra_skill.path().join("engineering_assurance")
            ),
            Err(InstalledError::CanonicalSkillInvalid)
        );

        let unknown_host_key = complete_bundle();
        write(
            &unknown_host_key.path().join(".codex-plugin/plugin.json"),
            br#"{"skills":"./engineering_assurance/skills/","instructions":"behavior"}"#,
        );
        assert_eq!(
            inspect(
                unknown_host_key.path(),
                &unknown_host_key.path().join("engineering_assurance")
            ),
            Err(InstalledError::HostManifestInvalid)
        );

        let workflow = complete_bundle();
        write(
            &workflow
                .path()
                .join("pilots/assurance-workflows/workflows/assurance-intake/def.yaml"),
            b"name: different\nversion: 1\n",
        );
        assert_eq!(
            inspect(
                workflow.path(),
                &workflow.path().join("engineering_assurance")
            ),
            Err(InstalledError::WorkflowMismatch)
        );
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn installed_resource_boundaries_are_exact() {
        let mut entries = MAX_ARCHIVE_ENTRIES - 1;
        count_entry(&mut entries).unwrap();
        assert_eq!(entries, MAX_ARCHIVE_ENTRIES);
        assert_eq!(
            count_entry(&mut entries),
            Err(InstalledError::EntryPopulationTooLarge)
        );
        assert!(validate_file_length(MAX_ARCHIVE_FILE_BYTES).is_ok());
        assert_eq!(
            validate_file_length(MAX_ARCHIVE_FILE_BYTES + 1),
            Err(InstalledError::FileTooLarge)
        );
        let mut bytes = MAX_ARCHIVE_EXPANDED_BYTES - 1;
        add_file_bytes(&mut bytes, 1).unwrap();
        assert_eq!(bytes, MAX_ARCHIVE_EXPANDED_BYTES);
        assert_eq!(
            add_file_bytes(&mut bytes, 1),
            Err(InstalledError::TotalBytesTooLarge)
        );
    }

    #[cfg(unix)]
    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn installed_links_are_rejected_without_following() {
        use std::os::unix::fs::symlink;

        let bundle = complete_bundle();
        symlink(
            bundle.path().join("engineering_assurance/manifest.yaml"),
            bundle
                .path()
                .join("engineering_assurance/contracts/linked.yaml"),
        )
        .unwrap();
        assert_eq!(
            inspect(bundle.path(), &bundle.path().join("engineering_assurance")),
            Err(InstalledError::EntryKindInvalid)
        );
    }
}
