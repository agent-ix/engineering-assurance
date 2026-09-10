// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Confined filesystem adapter for npm package staging and cleanup.

use std::{
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use thiserror::Error;

use engineering_assurance::package_lifecycle::PackageLifecycleResult;

const MODULE_DIRECTORY: &str = "engineering_assurance";
const STAGED_NAMES: [&str; 6] = [
    "manifest.yaml",
    "compatibility-matrix.json",
    "contracts",
    "fixtures",
    "schemas",
    "skeletons",
];
const MAX_ENTRIES: usize = 4_096;
const MAX_FILE_BYTES: usize = 8_388_608;
const MAX_TOTAL_BYTES: usize = 16_777_216;

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlannedFile {
    relative: PathBuf,
    length: usize,
    digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlannedRoot {
    name: &'static str,
    directory: bool,
    directories: Vec<PathBuf>,
    files: Vec<PlannedFile>,
}

#[derive(Default)]
struct Budget {
    entries: usize,
    bytes: usize,
}

/// Typed failure from the npm package filesystem adapter.
#[derive(Debug, Error)]
pub enum PackageHostError {
    /// The selected repository root is absent, linked, or not a directory.
    #[error("selected package root is not a regular directory")]
    RootInvalid,
    /// One fixed module source is absent.
    #[error("a required package source is missing")]
    SourceMissing,
    /// A source or destination has a linked or special file kind.
    #[error("package tree contains a linked or special entry")]
    EntryKindInvalid,
    /// A source path cannot be represented by the portable package contract.
    #[error("package tree contains a non-portable path")]
    PathInvalid,
    /// The fixed traversal population exceeds its ceiling.
    #[error("package tree exceeds {MAX_ENTRIES} filesystem entries")]
    EntryPopulationTooLarge,
    /// One selected file exceeds its byte ceiling.
    #[error("package file exceeds {MAX_FILE_BYTES} bytes")]
    FileTooLarge,
    /// The complete selected byte population exceeds its ceiling.
    #[error("package tree exceeds {MAX_TOTAL_BYTES} bytes")]
    TotalBytesTooLarge,
    /// A staged destination already exists before staging.
    #[error("a staged package destination already exists")]
    DestinationExists,
    /// Cleanup sees only part of the fixed staged population.
    #[error("the staged package population is incomplete")]
    DestinationPopulationIncomplete,
    /// Present staged bytes do not exactly match the current selected source.
    #[error("the staged package population does not match its source")]
    DestinationMismatch,
    /// A bounded filesystem read failed.
    #[error("package source inspection failed")]
    InspectionFailed,
    /// Copying a preflighted package tree failed.
    #[error("package staging failed")]
    StageFailed,
    /// Rollback could not remove only roots created by this invocation.
    #[error("package staging rollback failed")]
    RollbackFailed,
    /// Removing an exactly corresponding staged tree failed.
    #[error("package cleanup failed")]
    CleanupFailed,
}

impl PackageHostError {
    /// Return the stable machine category for this host failure.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::RootInvalid => "package_root_invalid",
            Self::SourceMissing => "package_source_missing",
            Self::EntryKindInvalid => "package_entry_kind_invalid",
            Self::PathInvalid => "package_path_invalid",
            Self::EntryPopulationTooLarge => "package_entry_population_too_large",
            Self::FileTooLarge => "package_file_too_large",
            Self::TotalBytesTooLarge => "package_total_bytes_too_large",
            Self::DestinationExists => "package_destination_exists",
            Self::DestinationPopulationIncomplete => "package_destination_population_incomplete",
            Self::DestinationMismatch => "package_destination_mismatch",
            Self::InspectionFailed => "package_inspection_failed",
            Self::StageFailed => "package_stage_failed",
            Self::RollbackFailed => "package_rollback_failed",
            Self::CleanupFailed => "package_cleanup_failed",
        }
    }
}

/// Stage the fixed npm module payload beneath an explicit repository root.
///
/// # Errors
///
/// Refuses invalid roots, source trees, resource populations, or pre-existing
/// destinations. A returned staging error attempts to remove only top-level
/// destinations created by this invocation.
pub fn stage(root: &Path) -> Result<PackageLifecycleResult, PackageHostError> {
    validate_root(root)?;
    let source = root.join(MODULE_DIRECTORY);
    validate_module_root(&source)?;
    let plans = inspect_selected(&source)?;
    for plan in &plans {
        if path_exists(&root.join(plan.name))? {
            return Err(PackageHostError::DestinationExists);
        }
    }

    let mut created = Vec::new();
    for plan in &plans {
        let destination = root.join(plan.name);
        let selected_source = source.join(plan.name);
        if let Err(error) = write_plan(&destination, &selected_source, plan, &mut created) {
            return match rollback(&created) {
                Ok(()) => Err(error),
                Err(()) => Err(PackageHostError::RollbackFailed),
            };
        }
    }

    match inspect_selected(root) {
        Ok(staged) if staged == plans => Ok(PackageLifecycleResult::staged()),
        Ok(_) | Err(_) => match rollback(&created) {
            Ok(()) => Err(PackageHostError::DestinationMismatch),
            Err(()) => Err(PackageHostError::RollbackFailed),
        },
    }
}

/// Remove the fixed staged population only after exact source correspondence.
///
/// # Errors
///
/// Refuses a partial, linked, special, or byte-divergent staged population
/// before deleting any destination.
pub fn clean(root: &Path) -> Result<PackageLifecycleResult, PackageHostError> {
    validate_root(root)?;
    let states = STAGED_NAMES
        .map(|name| path_exists(&root.join(name)))
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    if states.iter().all(|present| !present) {
        return Ok(PackageLifecycleResult::cleaned());
    }
    if states.iter().any(|present| !present) {
        return Err(PackageHostError::DestinationPopulationIncomplete);
    }

    let module_root = root.join(MODULE_DIRECTORY);
    validate_module_root(&module_root)?;
    let source = inspect_selected(&module_root)?;
    let staged = inspect_selected(root)?;
    if staged != source {
        return Err(PackageHostError::DestinationMismatch);
    }

    for plan in staged.iter().rev() {
        let destination = root.join(plan.name);
        let result = if plan.directory {
            fs::remove_dir_all(destination)
        } else {
            fs::remove_file(destination)
        };
        result.map_err(|_| PackageHostError::CleanupFailed)?;
    }
    Ok(PackageLifecycleResult::cleaned())
}

fn validate_root(root: &Path) -> Result<(), PackageHostError> {
    let metadata = fs::symlink_metadata(root).map_err(|_| PackageHostError::RootInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PackageHostError::RootInvalid);
    }
    Ok(())
}

fn validate_module_root(root: &Path) -> Result<(), PackageHostError> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(PackageHostError::SourceMissing);
        }
        Err(_) => return Err(PackageHostError::InspectionFailed),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PackageHostError::EntryKindInvalid);
    }
    Ok(())
}

fn path_exists(path: &Path) -> Result<bool, PackageHostError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(PackageHostError::InspectionFailed),
    }
}

fn inspect_selected(base: &Path) -> Result<Vec<PlannedRoot>, PackageHostError> {
    let mut budget = Budget::default();
    STAGED_NAMES
        .iter()
        .map(|name| inspect_root(&base.join(name), name, &mut budget))
        .collect()
}

fn inspect_root(
    root: &Path,
    name: &'static str,
    budget: &mut Budget,
) -> Result<PlannedRoot, PackageHostError> {
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(PackageHostError::SourceMissing);
        }
        Err(_) => return Err(PackageHostError::InspectionFailed),
    };
    if metadata.file_type().is_symlink() {
        return Err(PackageHostError::EntryKindInvalid);
    }
    if metadata.is_file() {
        count_entry(budget)?;
        let (length, digest) = inspect_bounded_file(root, budget)?;
        return Ok(PlannedRoot {
            name,
            directory: false,
            directories: Vec::new(),
            files: vec![PlannedFile {
                relative: PathBuf::new(),
                length,
                digest,
            }],
        });
    }
    if !metadata.is_dir() {
        return Err(PackageHostError::EntryKindInvalid);
    }
    count_entry(budget)?;

    let mut directories = Vec::new();
    let mut files = Vec::new();
    let mut pending = vec![(root.to_owned(), PathBuf::new())];
    while let Some((directory, relative)) = pending.pop() {
        let mut entries = fs::read_dir(directory)
            .map_err(|_| PackageHostError::InspectionFailed)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| PackageHostError::InspectionFailed)?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries.into_iter().rev() {
            validate_segment(&entry.file_name())?;
            let child_relative = relative.join(entry.file_name());
            let child_path = entry.path();
            let child_metadata = fs::symlink_metadata(&child_path)
                .map_err(|_| PackageHostError::InspectionFailed)?;
            if child_metadata.file_type().is_symlink() {
                return Err(PackageHostError::EntryKindInvalid);
            }
            count_entry(budget)?;
            if child_metadata.is_dir() {
                directories.push(child_relative.clone());
                pending.push((child_path, child_relative));
            } else if child_metadata.is_file() {
                let (length, digest) = inspect_bounded_file(&child_path, budget)?;
                files.push(PlannedFile {
                    relative: child_relative,
                    length,
                    digest,
                });
            } else {
                return Err(PackageHostError::EntryKindInvalid);
            }
        }
    }
    directories.sort();
    files.sort_by(|left, right| left.relative.cmp(&right.relative));
    Ok(PlannedRoot {
        name,
        directory: true,
        directories,
        files,
    })
}

fn validate_segment(segment: &OsStr) -> Result<(), PackageHostError> {
    let Some(value) = segment.to_str() else {
        return Err(PackageHostError::PathInvalid);
    };
    if value.is_empty()
        || matches!(value, "." | "..")
        || value.contains('\\')
        || value.chars().any(char::is_control)
    {
        return Err(PackageHostError::PathInvalid);
    }
    Ok(())
}

fn count_entry(budget: &mut Budget) -> Result<(), PackageHostError> {
    budget.entries = budget.entries.saturating_add(1);
    if budget.entries > MAX_ENTRIES {
        return Err(PackageHostError::EntryPopulationTooLarge);
    }
    Ok(())
}

fn inspect_bounded_file(
    path: &Path,
    budget: &mut Budget,
) -> Result<(usize, [u8; 32]), PackageHostError> {
    let metadata = fs::metadata(path).map_err(|_| PackageHostError::InspectionFailed)?;
    let length = usize::try_from(metadata.len()).map_err(|_| PackageHostError::FileTooLarge)?;
    if length > MAX_FILE_BYTES {
        return Err(PackageHostError::FileTooLarge);
    }
    let mut source = fs::File::open(path).map_err(|_| PackageHostError::InspectionFailed)?;
    let mut hasher = Sha256::new();
    let mut observed = 0_usize;
    let mut buffer = [0_u8; 8_192];
    loop {
        let count = source
            .read(&mut buffer)
            .map_err(|_| PackageHostError::InspectionFailed)?;
        if count == 0 {
            break;
        }
        observed = observed.saturating_add(count);
        if observed > MAX_FILE_BYTES {
            return Err(PackageHostError::FileTooLarge);
        }
        budget.bytes = budget.bytes.saturating_add(count);
        if budget.bytes > MAX_TOTAL_BYTES {
            return Err(PackageHostError::TotalBytesTooLarge);
        }
        hasher.update(&buffer[..count]);
    }
    if observed != length {
        return Err(PackageHostError::InspectionFailed);
    }
    Ok((observed, hasher.finalize().into()))
}

fn write_plan(
    destination: &Path,
    source: &Path,
    plan: &PlannedRoot,
    created: &mut Vec<(PathBuf, bool)>,
) -> Result<(), PackageHostError> {
    if plan.directory {
        fs::create_dir(destination).map_err(|_| PackageHostError::StageFailed)?;
        created.push((destination.to_owned(), true));
        for relative in &plan.directories {
            fs::create_dir(destination.join(relative))
                .map_err(|_| PackageHostError::StageFailed)?;
        }
        for file in &plan.files {
            copy_new_file(
                &source.join(&file.relative),
                &destination.join(&file.relative),
                file.length,
            )?;
        }
    } else {
        let file = plan.files.first().ok_or(PackageHostError::StageFailed)?;
        if file.relative != Path::new("") {
            return Err(PackageHostError::StageFailed);
        }
        let mut output = open_new_file(destination)?;
        created.push((destination.to_owned(), false));
        copy_bytes(source, &mut output, file.length)?;
    }
    Ok(())
}

fn copy_new_file(
    source: &Path,
    destination: &Path,
    expected_length: usize,
) -> Result<(), PackageHostError> {
    let mut output = open_new_file(destination)?;
    copy_bytes(source, &mut output, expected_length)
}

fn open_new_file(path: &Path) -> Result<fs::File, PackageHostError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| PackageHostError::StageFailed)
}

fn copy_bytes(
    source: &Path,
    output: &mut fs::File,
    expected_length: usize,
) -> Result<(), PackageHostError> {
    let input = fs::File::open(source).map_err(|_| PackageHostError::StageFailed)?;
    let limit = u64::try_from(expected_length)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let copied =
        io::copy(&mut input.take(limit), output).map_err(|_| PackageHostError::StageFailed)?;
    if copied != u64::try_from(expected_length).unwrap_or(u64::MAX) {
        return Err(PackageHostError::StageFailed);
    }
    output.sync_all().map_err(|_| PackageHostError::StageFailed)
}

fn rollback(created: &[(PathBuf, bool)]) -> Result<(), ()> {
    let mut failed = false;
    for (path, directory) in created.iter().rev() {
        let result = if *directory {
            fs::remove_dir_all(path)
        } else {
            fs::remove_file(path)
        };
        if result.is_err() {
            failed = true;
        }
    }
    if failed { Err(()) } else { Ok(()) }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use ix_trace_rs::trace;

    use super::*;

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[test]
    #[trace("TC-111", "FR-017-AC-3")]
    fn tc_111_rollback_removes_only_invocation_owned_top_level_destinations() {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "engineering-assurance-rollback-{}-{sequence}",
            std::process::id()
        ));
        let owned_directory = root.join("owned-directory");
        let owned_file = root.join("owned-file");
        let unowned = root.join("unowned");
        fs::create_dir_all(owned_directory.join("nested")).unwrap();
        fs::write(owned_directory.join("nested/value"), b"owned").unwrap();
        fs::write(&owned_file, b"owned").unwrap();
        fs::write(&unowned, b"preserve").unwrap();

        rollback(&[(owned_directory.clone(), true), (owned_file.clone(), false)]).unwrap();

        assert!(!owned_directory.exists());
        assert!(!owned_file.exists());
        assert_eq!(fs::read(&unowned).unwrap(), b"preserve");
        fs::remove_dir_all(root).unwrap();
    }
}
