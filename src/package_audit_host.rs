// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Package builders, archives, installers, and filesystem orchestration.

use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use engineering_assurance::{
    content_rights::{ContentEntryKind, inspect_content},
    package_audit::PackageAuditResult,
    package_membership::{PackageMembershipOutcome, PackageMembershipPolicy},
};
use serde::Deserialize;
use tempfile::Builder;
use thiserror::Error;

use crate::{
    content_rights_host,
    package_archive::{self, ArchiveError, ArchiveSnapshot},
    package_host,
    package_install::{self, InstalledError},
    process_host::{self, ProcessError, ProcessLimits},
};

const PACKAGE_PROCESS_TIMEOUT: Duration = Duration::from_secs(180);
const MAX_PROCESS_OUTPUT_BYTES: usize = 8_388_608;
const MAX_ROOT_ENTRIES: usize = 4_096;
const MAX_WHEEL_SOURCE_BYTES: usize = 67_108_864;
const DISTRIBUTION_VERSION: &str = "0.3.1";
const PRIVATE_CLASSIFIER: &[u8] = b"Classifier: Private :: Do Not Upload\n";
const ROOT_DATA_FILES: [&str; 11] = [
    ".claude-plugin/plugin.json",
    ".codex-plugin/plugin.json",
    ".github/plugin/plugin.json",
    "opencode.json",
    "pilots/assurance-workflows/README.md",
    "pilots/assurance-workflows/SKILL.md",
    "pilots/assurance-workflows/scripts/invariants.js",
    "pilots/assurance-workflows/workflows/architecture-evaluation/def.yaml",
    "pilots/assurance-workflows/workflows/assurance-intake/def.yaml",
    "pilots/assurance-workflows/workflows/change-assurance/def.yaml",
    "pilots/assurance-workflows/workflows/measurement-promotion/def.yaml",
];
const NPM_ROOT_FILES: [&str; 8] = [
    "CONTENT_RIGHTS.md",
    "LICENSE",
    "README.md",
    "package.json",
    "engineering_assurance/INSTALL.md",
    "engineering_assurance/manifest.yaml",
    "engineering_assurance/compatibility-matrix.json",
    "manifest.yaml",
];
const NPM_MODULE_SUBTREES: [&str; 4] = ["contracts/", "fixtures/", "schemas/", "skeletons/"];

#[derive(Debug, Error)]
pub(crate) enum PackageAuditHostError {
    #[error("selected package-audit root is not a regular directory")]
    RootInvalid,
    #[error("package-audit temporary directory is unavailable")]
    TemporaryDirectoryUnavailable,
    #[error("package-audit wheel source staging failed")]
    SourceStagingFailed,
    #[error("package-audit protected-token input is invalid")]
    ProtectedTokensInvalid,
    #[error("package-audit child process is unavailable")]
    ProcessUnavailable,
    #[error("package-audit child process timed out")]
    ProcessTimedOut,
    #[error("package-audit child process output is too large")]
    ProcessOutputTooLarge,
    #[error("package-audit child process observation failed")]
    ProcessObservationFailed,
    #[error("package-audit child process failed")]
    ProcessFailed,
    #[error("package-audit repository entry population cannot be bounded")]
    RootPopulationInvalid,
    #[error("package-audit child process changed the repository entry population")]
    RootOutputChanged,
    #[error("package-audit archive population is missing or ambiguous")]
    ArchiveSelectionInvalid,
    #[error(transparent)]
    Archive(ArchiveError),
    #[error("package-audit expected membership cannot be constructed")]
    ExpectedMembershipInvalid,
    #[error("package-audit archive membership differs from its allowlist")]
    MembershipMismatch,
    #[error("package-audit archive member failed content-rights policy")]
    ContentRightsRefused,
    #[error("wheel license population or bytes are invalid")]
    WheelLicenseInvalid,
    #[error("wheel metadata does not retain the private-package classifier")]
    WheelMetadataInvalid,
    #[error("npm pack report is malformed or ambiguous")]
    NpmReportInvalid,
    #[error("npm lifecycle staging population is not clean")]
    NpmStagingInvalid,
    #[error("npm lifecycle staging cleanup failed")]
    NpmStagingCleanupFailed,
    #[error(transparent)]
    Installed(InstalledError),
    #[error("wheel and npm canonical installed bytes differ")]
    CanonicalBytesMismatch,
}

impl PackageAuditHostError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::RootInvalid => "package_audit_root_invalid",
            Self::TemporaryDirectoryUnavailable => "package_audit_temporary_directory_unavailable",
            Self::SourceStagingFailed => "package_audit_source_staging_failed",
            Self::ProtectedTokensInvalid => "package_audit_protected_tokens_invalid",
            Self::ProcessUnavailable => "package_audit_process_unavailable",
            Self::ProcessTimedOut => "package_audit_process_timed_out",
            Self::ProcessOutputTooLarge => "package_audit_process_output_too_large",
            Self::ProcessObservationFailed => "package_audit_process_observation_failed",
            Self::ProcessFailed => "package_audit_process_failed",
            Self::RootPopulationInvalid => "package_audit_root_population_invalid",
            Self::RootOutputChanged => "package_audit_root_output_changed",
            Self::ArchiveSelectionInvalid => "package_audit_archive_selection_invalid",
            Self::Archive(error) => match error {
                ArchiveError::ArchiveInvalid => "package_audit_archive_invalid",
                ArchiveError::ArchiveTooLarge => "package_audit_archive_too_large",
                ArchiveError::DecodeFailed => "package_audit_archive_decode_failed",
                ArchiveError::EntryPopulationTooLarge => {
                    "package_audit_archive_entry_population_too_large"
                }
                ArchiveError::MemberPathInvalid => "package_audit_member_path_invalid",
                ArchiveError::MemberDuplicate => "package_audit_member_duplicate",
                ArchiveError::MemberKindInvalid => "package_audit_member_kind_invalid",
                ArchiveError::MemberTooLarge => "package_audit_member_too_large",
                ArchiveError::ExpandedBytesTooLarge => "package_audit_expanded_bytes_too_large",
            },
            Self::ExpectedMembershipInvalid => "package_audit_expected_membership_invalid",
            Self::MembershipMismatch => "package_audit_membership_mismatch",
            Self::ContentRightsRefused => "package_audit_content_rights_refused",
            Self::WheelLicenseInvalid => "package_audit_wheel_license_invalid",
            Self::WheelMetadataInvalid => "package_audit_wheel_metadata_invalid",
            Self::NpmReportInvalid => "package_audit_npm_report_invalid",
            Self::NpmStagingInvalid => "package_audit_npm_staging_invalid",
            Self::NpmStagingCleanupFailed => "package_audit_npm_staging_cleanup_failed",
            Self::Installed(error) => match error {
                InstalledError::RootInvalid => "package_audit_installed_root_invalid",
                InstalledError::EntryPopulationTooLarge => {
                    "package_audit_installed_entry_population_too_large"
                }
                InstalledError::PathInvalid => "package_audit_installed_path_invalid",
                InstalledError::EntryKindInvalid => "package_audit_installed_entry_kind_invalid",
                InstalledError::FileTooLarge => "package_audit_installed_file_too_large",
                InstalledError::TotalBytesTooLarge => {
                    "package_audit_installed_total_bytes_too_large"
                }
                InstalledError::InspectionFailed => "package_audit_installed_inspection_failed",
                InstalledError::ModuleRootIncomplete => {
                    "package_audit_installed_module_root_incomplete"
                }
                InstalledError::CanonicalSkillInvalid => {
                    "package_audit_installed_canonical_skill_invalid"
                }
                InstalledError::WorkflowPopulationInvalid => {
                    "package_audit_installed_workflow_population_invalid"
                }
                InstalledError::HostManifestInvalid => {
                    "package_audit_installed_host_manifest_invalid"
                }
                InstalledError::HostTargetInvalid => "package_audit_installed_host_target_invalid",
                InstalledError::WorkflowMismatch => "package_audit_installed_workflow_mismatch",
            },
            Self::CanonicalBytesMismatch => "package_audit_canonical_bytes_mismatch",
        }
    }
}

impl From<ArchiveError> for PackageAuditHostError {
    fn from(error: ArchiveError) -> Self {
        Self::Archive(error)
    }
}

impl From<InstalledError> for PackageAuditHostError {
    fn from(error: InstalledError) -> Self {
        Self::Installed(error)
    }
}

pub(crate) fn execute(root: &Path) -> Result<PackageAuditResult, PackageAuditHostError> {
    let root = selected_root(root)?;
    let protected_tokens = content_rights_host::protected_tokens()
        .map_err(|_| PackageAuditHostError::ProtectedTokensInvalid)?;
    let temporary = Builder::new()
        .prefix("engineering-assurance-package-audit-")
        .tempdir()
        .map_err(|_| PackageAuditHostError::TemporaryDirectoryUnavailable)?;
    let output = temporary.path();

    build_wheel(&root, output)?;
    let wheel_path = select_archive(output, "whl")?;
    let wheel = package_archive::read_wheel(&wheel_path)?;
    let wheel_expected = wheel_allowlist(&root, &wheel)?;
    audit_membership(&wheel_expected, &wheel.names())?;
    audit_content_rights(&wheel, &protected_tokens)?;
    validate_wheel_identity(&root, &wheel)?;

    let wheel_install = output.join("wheel-install");
    install_wheel(&root, &wheel_path, &wheel_install, output)?;
    let wheel_bundle =
        package_install::inspect(&wheel_install, &wheel_install.join("engineering_assurance"))?;

    let npm_report = build_npm(&root, output)?;
    let npm_expected = npm_allowlist(&root)?;
    let npm_path = select_archive(output, "tgz")?;
    let npm = package_archive::read_npm(&npm_path)?;
    audit_npm_membership(&npm_expected, &npm_report, &npm)?;
    audit_content_rights(&npm, &protected_tokens)?;

    let npm_install = output.join("npm-install");
    install_npm(&root, &npm_path, &npm_install, output)?;
    let npm_root = npm_install
        .join("node_modules")
        .join("@agent-ix")
        .join("engineering-assurance");
    let npm_bundle = package_install::inspect(&npm_root, &npm_root)?;
    if wheel_bundle.canonical_files != npm_bundle.canonical_files {
        return Err(PackageAuditHostError::CanonicalBytesMismatch);
    }

    Ok(PackageAuditResult::accepted(
        wheel.file_count(),
        npm.file_count(),
        wheel_bundle.canonical_file_count(),
    ))
}

fn selected_root(root: &Path) -> Result<PathBuf, PackageAuditHostError> {
    let metadata = fs::symlink_metadata(root).map_err(|_| PackageAuditHostError::RootInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PackageAuditHostError::RootInvalid);
    }
    let root = fs::canonicalize(root).map_err(|_| PackageAuditHostError::RootInvalid)?;
    for file in ["setup.cfg", "package.json", "LICENSE"] {
        let metadata = fs::symlink_metadata(root.join(file))
            .map_err(|_| PackageAuditHostError::RootInvalid)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(PackageAuditHostError::RootInvalid);
        }
    }
    let module = fs::symlink_metadata(root.join("engineering_assurance"))
        .map_err(|_| PackageAuditHostError::RootInvalid)?;
    if module.file_type().is_symlink() || !module.is_dir() {
        return Err(PackageAuditHostError::RootInvalid);
    }
    package_install::inspect_tree(&root.join("engineering_assurance"))
        .map_err(|_| PackageAuditHostError::RootInvalid)?;
    let fixed_sources = ROOT_DATA_FILES
        .into_iter()
        .chain(
            NPM_ROOT_FILES
                .into_iter()
                .filter(|source| *source != "manifest.yaml"),
        )
        .chain(["pyproject.toml", "setup.cfg"])
        .collect::<BTreeSet<_>>();
    for source in fixed_sources {
        validate_regular_source(&root, source)?;
    }
    Ok(root)
}

fn validate_regular_source(root: &Path, relative: &str) -> Result<(), PackageAuditHostError> {
    let mut current = root.to_owned();
    let mut components = Path::new(relative).components().peekable();
    while let Some(component) = components.next() {
        let std::path::Component::Normal(component) = component else {
            return Err(PackageAuditHostError::RootInvalid);
        };
        current.push(component);
        let metadata =
            fs::symlink_metadata(&current).map_err(|_| PackageAuditHostError::RootInvalid)?;
        if metadata.file_type().is_symlink()
            || (components.peek().is_some() && !metadata.is_dir())
            || (components.peek().is_none() && !metadata.is_file())
        {
            return Err(PackageAuditHostError::RootInvalid);
        }
    }
    Ok(())
}

fn build_wheel(root: &Path, output: &Path) -> Result<(), PackageAuditHostError> {
    let source = stage_wheel_source(root, output)?;
    let arguments = vec![
        OsString::from("-m"),
        OsString::from("pip"),
        OsString::from("wheel"),
        OsString::from("."),
        OsString::from("--no-deps"),
        OsString::from("--no-build-isolation"),
        OsString::from("--wheel-dir"),
        output.as_os_str().to_owned(),
    ];
    let cache = output.join("pip-cache");
    let environment = package_temporary_environment(output, Some(("PIP_CACHE_DIR", &cache)));
    let root_before = root_population(root)?;
    let process = invoke(OsStr::new("python3"), &arguments, &source, &environment);
    ensure_root_unchanged(root, &root_before)?;
    let process = process?;
    if !process.status.success() {
        return Err(PackageAuditHostError::ProcessFailed);
    }
    Ok(())
}

fn stage_wheel_source(root: &Path, output: &Path) -> Result<PathBuf, PackageAuditHostError> {
    let source = output.join("wheel-source");
    fs::create_dir(&source).map_err(|_| PackageAuditHostError::SourceStagingFailed)?;
    let package = package_install::inspect_tree(&root.join("engineering_assurance"))
        .map_err(|_| PackageAuditHostError::SourceStagingFailed)?;
    let mut total_bytes = 0_usize;
    for (relative, bytes) in package
        .files
        .iter()
        .filter(|(path, _)| !path.split('/').any(|component| component == "__pycache__"))
    {
        add_wheel_source_bytes(&mut total_bytes, bytes.len())?;
        write_wheel_source(
            &source.join("engineering_assurance"),
            Path::new(relative),
            bytes,
        )?;
    }
    for relative in ["pyproject.toml", "setup.cfg", "README.md", "LICENSE"]
        .into_iter()
        .chain(ROOT_DATA_FILES)
    {
        let bytes = read_wheel_source(root, relative)?;
        add_wheel_source_bytes(&mut total_bytes, bytes.len())?;
        write_wheel_source(&source, Path::new(relative), &bytes)?;
    }
    Ok(source)
}

fn read_wheel_source(root: &Path, relative: &str) -> Result<Vec<u8>, PackageAuditHostError> {
    validate_regular_source(root, relative)?;
    let path = root.join(relative);
    let metadata =
        fs::symlink_metadata(&path).map_err(|_| PackageAuditHostError::SourceStagingFailed)?;
    let expected =
        usize::try_from(metadata.len()).map_err(|_| PackageAuditHostError::SourceStagingFailed)?;
    if expected > MAX_PROCESS_OUTPUT_BYTES {
        return Err(PackageAuditHostError::SourceStagingFailed);
    }
    let input = fs::File::open(path).map_err(|_| PackageAuditHostError::SourceStagingFailed)?;
    let limit = u64::try_from(MAX_PROCESS_OUTPUT_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::with_capacity(expected);
    input
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| PackageAuditHostError::SourceStagingFailed)?;
    if bytes.len() != expected || bytes.len() > MAX_PROCESS_OUTPUT_BYTES {
        return Err(PackageAuditHostError::SourceStagingFailed);
    }
    Ok(bytes)
}

fn add_wheel_source_bytes(total: &mut usize, length: usize) -> Result<(), PackageAuditHostError> {
    *total = total
        .checked_add(length)
        .ok_or(PackageAuditHostError::SourceStagingFailed)?;
    if *total > MAX_WHEEL_SOURCE_BYTES {
        return Err(PackageAuditHostError::SourceStagingFailed);
    }
    Ok(())
}

fn write_wheel_source(
    root: &Path,
    relative: &Path,
    bytes: &[u8],
) -> Result<(), PackageAuditHostError> {
    let destination = root.join(relative);
    let parent = destination
        .parent()
        .ok_or(PackageAuditHostError::SourceStagingFailed)?;
    fs::create_dir_all(parent).map_err(|_| PackageAuditHostError::SourceStagingFailed)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_| PackageAuditHostError::SourceStagingFailed)?;
    output
        .write_all(bytes)
        .and_then(|()| output.sync_all())
        .map_err(|_| PackageAuditHostError::SourceStagingFailed)
}

fn install_wheel(
    root: &Path,
    wheel: &Path,
    destination: &Path,
    output: &Path,
) -> Result<(), PackageAuditHostError> {
    let arguments = vec![
        OsString::from("-m"),
        OsString::from("pip"),
        OsString::from("install"),
        OsString::from("--no-index"),
        OsString::from("--no-deps"),
        OsString::from("--target"),
        destination.as_os_str().to_owned(),
        wheel.as_os_str().to_owned(),
    ];
    let cache = output.join("pip-cache");
    let environment = package_temporary_environment(output, Some(("PIP_CACHE_DIR", &cache)));
    run_required_confined(OsStr::new("python3"), &arguments, root, &environment).map(|_| ())
}

fn build_npm(root: &Path, output: &Path) -> Result<Vec<String>, PackageAuditHostError> {
    package_host::require_staged_destinations_absent(root)
        .map_err(|_| PackageAuditHostError::NpmStagingInvalid)?;
    let root_before = root_population(root)?;
    let arguments = vec![
        OsString::from("pack"),
        OsString::from("--json"),
        OsString::from("--pack-destination"),
        output.as_os_str().to_owned(),
    ];
    let cache = output.join("npm-cache");
    let mut environment = package_temporary_environment(output, Some(("npm_config_cache", &cache)));
    environment.push((OsString::from("CARGO_BUILD_JOBS"), OsString::from("2")));
    let process = invoke(OsStr::new("npm"), &arguments, root, &environment);
    let process = finish_npm_pack(root, &root_before, process)?;
    if !process.status.success() {
        return Err(PackageAuditHostError::ProcessFailed);
    }
    parse_npm_report(&process.stdout)
}

fn install_npm(
    root: &Path,
    archive: &Path,
    destination: &Path,
    output: &Path,
) -> Result<(), PackageAuditHostError> {
    let arguments = vec![
        OsString::from("install"),
        OsString::from("--ignore-scripts"),
        OsString::from("--offline"),
        OsString::from("--prefix"),
        destination.as_os_str().to_owned(),
        archive.as_os_str().to_owned(),
    ];
    let cache = output.join("npm-cache");
    let environment = package_temporary_environment(output, Some(("npm_config_cache", &cache)));
    run_required_confined(OsStr::new("npm"), &arguments, root, &environment).map(|_| ())
}

fn run_required_confined(
    executable: &OsStr,
    arguments: &[OsString],
    root: &Path,
    environment: &[(OsString, OsString)],
) -> Result<process_host::CompletedProcess, PackageAuditHostError> {
    let before = root_population(root)?;
    let process = invoke(executable, arguments, root, environment);
    ensure_root_unchanged(root, &before)?;
    let process = process?;
    if !process.status.success() {
        return Err(PackageAuditHostError::ProcessFailed);
    }
    Ok(process)
}

fn invoke(
    executable: &OsStr,
    arguments: &[OsString],
    root: &Path,
    environment: &[(OsString, OsString)],
) -> Result<process_host::CompletedProcess, PackageAuditHostError> {
    let arguments = arguments
        .iter()
        .map(OsString::as_os_str)
        .collect::<Vec<_>>();
    let environment = environment
        .iter()
        .map(|(name, value)| (name.as_os_str(), value.as_os_str()))
        .collect::<Vec<_>>();
    process_host::run_configured(
        executable,
        &arguments,
        Some(root),
        &environment,
        &[OsStr::new("ASSURANCE_PROTECTED_TOKENS")],
        ProcessLimits {
            timeout: PACKAGE_PROCESS_TIMEOUT,
            max_output_bytes: MAX_PROCESS_OUTPUT_BYTES,
        },
    )
    .map_err(|error| map_process_error(&error))
}

fn package_temporary_environment(
    output: &Path,
    cache: Option<(&str, &Path)>,
) -> Vec<(OsString, OsString)> {
    let mut environment = ["TMPDIR", "TMP", "TEMP"]
        .into_iter()
        .map(|name| (OsString::from(name), output.as_os_str().to_owned()))
        .collect::<Vec<_>>();
    if let Some((name, path)) = cache {
        environment.push((OsString::from(name), path.as_os_str().to_owned()));
    }
    environment
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum RootEntryKind {
    Directory,
    File,
    SymbolicLink,
    Other,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct RootEntry {
    name: OsString,
    kind: RootEntryKind,
}

fn root_population(root: &Path) -> Result<Vec<RootEntry>, PackageAuditHostError> {
    let entries = fs::read_dir(root).map_err(|_| PackageAuditHostError::RootPopulationInvalid)?;
    let mut population = Vec::new();
    for entry in entries {
        if population.len() >= MAX_ROOT_ENTRIES {
            return Err(PackageAuditHostError::RootPopulationInvalid);
        }
        let entry = entry.map_err(|_| PackageAuditHostError::RootPopulationInvalid)?;
        let file_type = entry
            .file_type()
            .map_err(|_| PackageAuditHostError::RootPopulationInvalid)?;
        let kind = if file_type.is_file() {
            RootEntryKind::File
        } else if file_type.is_dir() {
            RootEntryKind::Directory
        } else if file_type.is_symlink() {
            RootEntryKind::SymbolicLink
        } else {
            RootEntryKind::Other
        };
        population.push(RootEntry {
            name: entry.file_name(),
            kind,
        });
    }
    population.sort();
    Ok(population)
}

fn ensure_root_unchanged(root: &Path, before: &[RootEntry]) -> Result<(), PackageAuditHostError> {
    if root_population(root)? != before {
        return Err(PackageAuditHostError::RootOutputChanged);
    }
    Ok(())
}

fn finish_npm_pack(
    root: &Path,
    root_before: &[RootEntry],
    process: Result<process_host::CompletedProcess, PackageAuditHostError>,
) -> Result<process_host::CompletedProcess, PackageAuditHostError> {
    package_host::clean(root).map_err(|_| PackageAuditHostError::NpmStagingCleanupFailed)?;
    ensure_root_unchanged(root, root_before)?;
    process
}

fn map_process_error(error: &ProcessError) -> PackageAuditHostError {
    match error {
        ProcessError::Unavailable { .. } => PackageAuditHostError::ProcessUnavailable,
        ProcessError::TimedOut { .. } => PackageAuditHostError::ProcessTimedOut,
        ProcessError::OutputTooLarge { .. } => PackageAuditHostError::ProcessOutputTooLarge,
        ProcessError::PipeUnavailable { .. }
        | ProcessError::Observation { .. }
        | ProcessError::OutputUnreadable { .. } => PackageAuditHostError::ProcessObservationFailed,
    }
}

fn select_archive(output: &Path, extension: &str) -> Result<PathBuf, PackageAuditHostError> {
    let mut matches = fs::read_dir(output)
        .map_err(|_| PackageAuditHostError::ArchiveSelectionInvalid)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| PackageAuditHostError::ArchiveSelectionInvalid)?
        .into_iter()
        .map(|entry| entry.path())
        .filter(|path| path.extension() == Some(OsStr::new(extension)))
        .collect::<Vec<_>>();
    matches.sort();
    if matches.len() != 1 {
        return Err(PackageAuditHostError::ArchiveSelectionInvalid);
    }
    Ok(matches.remove(0))
}

fn wheel_allowlist(
    root: &Path,
    wheel: &ArchiveSnapshot,
) -> Result<Vec<String>, PackageAuditHostError> {
    let package_root = root.join("engineering_assurance");
    let metadata = fs::symlink_metadata(&package_root)
        .map_err(|_| PackageAuditHostError::ExpectedMembershipInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PackageAuditHostError::ExpectedMembershipInvalid);
    }
    let package = package_install::inspect_tree(&package_root)
        .map_err(|_| PackageAuditHostError::ExpectedMembershipInvalid)?;
    let metadata_root = format!("engineering_assurance-{DISTRIBUTION_VERSION}.dist-info");
    let data_root = format!("engineering_assurance-{DISTRIBUTION_VERSION}.data/data");
    let mut expected = BTreeSet::new();
    for path in package.files.keys() {
        if !path.split('/').any(|component| component == "__pycache__") {
            expected.insert(format!("engineering_assurance/{path}"));
        }
    }
    for path in ROOT_DATA_FILES {
        expected.insert(format!("{data_root}/{path}"));
    }
    for name in ["METADATA", "RECORD", "WHEEL", "top_level.txt"] {
        expected.insert(format!("{metadata_root}/{name}"));
    }
    let license_candidates = [
        format!("{metadata_root}/LICENSE"),
        format!("{metadata_root}/licenses/LICENSE"),
    ];
    let emitted = license_candidates
        .iter()
        .filter(|candidate| wheel.file(candidate).is_some())
        .collect::<Vec<_>>();
    if emitted.len() != 1 {
        return Err(PackageAuditHostError::WheelLicenseInvalid);
    }
    expected.insert(emitted[0].clone());
    Ok(expected.into_iter().collect())
}

fn npm_allowlist(root: &Path) -> Result<Vec<String>, PackageAuditHostError> {
    let package_root = root.join("engineering_assurance");
    let metadata = fs::symlink_metadata(&package_root)
        .map_err(|_| PackageAuditHostError::ExpectedMembershipInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PackageAuditHostError::ExpectedMembershipInvalid);
    }
    let package = package_install::inspect_tree(&package_root)
        .map_err(|_| PackageAuditHostError::ExpectedMembershipInvalid)?;
    let mut expected = BTreeSet::new();
    expected.extend(NPM_ROOT_FILES.into_iter().map(str::to_owned));
    expected.extend(ROOT_DATA_FILES.into_iter().map(str::to_owned));
    expected.insert("compatibility-matrix.json".to_owned());
    for path in package.files.keys() {
        if path == "manifest.yaml" {
            expected.insert(path.clone());
        }
        if NPM_MODULE_SUBTREES
            .iter()
            .any(|prefix| path.starts_with(prefix))
        {
            expected.insert(path.clone());
        }
        if path.starts_with("skills/assurance-onboarding/") {
            expected.insert(format!("engineering_assurance/{path}"));
        }
    }
    Ok(expected.into_iter().collect())
}

fn audit_membership(expected: &[String], observed: &[String]) -> Result<(), PackageAuditHostError> {
    let result = PackageMembershipPolicy::new(expected)
        .map_err(|_| PackageAuditHostError::ExpectedMembershipInvalid)?
        .compare(observed)
        .map_err(|_| PackageAuditHostError::MembershipMismatch)?;
    if result.outcome != PackageMembershipOutcome::Accepted {
        return Err(PackageAuditHostError::MembershipMismatch);
    }
    Ok(())
}

fn audit_npm_membership(
    expected: &[String],
    report: &[String],
    archive: &ArchiveSnapshot,
) -> Result<(), PackageAuditHostError> {
    audit_membership(expected, report)?;
    audit_membership(expected, &archive.names())
}

fn audit_content_rights(
    archive: &ArchiveSnapshot,
    protected_tokens: &[String],
) -> Result<(), PackageAuditHostError> {
    for file in &archive.files {
        let policy_path = if file.path.rsplit('/').next() == Some("LICENSE") {
            "LICENSE"
        } else {
            &file.path
        };
        let findings = inspect_content(
            policy_path,
            ContentEntryKind::File,
            &file.bytes,
            protected_tokens,
        )
        .map_err(|_| PackageAuditHostError::ContentRightsRefused)?;
        if !findings.is_empty() {
            return Err(PackageAuditHostError::ContentRightsRefused);
        }
    }
    Ok(())
}

fn validate_wheel_identity(
    root: &Path,
    wheel: &ArchiveSnapshot,
) -> Result<(), PackageAuditHostError> {
    let metadata_root = format!("engineering_assurance-{DISTRIBUTION_VERSION}.dist-info");
    let license = [
        format!("{metadata_root}/LICENSE"),
        format!("{metadata_root}/licenses/LICENSE"),
    ]
    .into_iter()
    .filter_map(|path| wheel.file(&path))
    .collect::<Vec<_>>();
    if license.len() != 1 {
        return Err(PackageAuditHostError::WheelLicenseInvalid);
    }
    let canonical =
        fs::read(root.join("LICENSE")).map_err(|_| PackageAuditHostError::WheelLicenseInvalid)?;
    if license[0] != canonical {
        return Err(PackageAuditHostError::WheelLicenseInvalid);
    }
    let metadata = wheel
        .file(&format!("{metadata_root}/METADATA"))
        .ok_or(PackageAuditHostError::WheelMetadataInvalid)?;
    if !metadata
        .windows(PRIVATE_CLASSIFIER.len())
        .any(|window| window == PRIVATE_CLASSIFIER)
    {
        return Err(PackageAuditHostError::WheelMetadataInvalid);
    }
    Ok(())
}

#[derive(Deserialize)]
struct NpmPackReport {
    files: Vec<NpmPackFile>,
}

#[derive(Deserialize)]
struct NpmPackFile {
    path: String,
}

fn parse_npm_report(bytes: &[u8]) -> Result<Vec<String>, PackageAuditHostError> {
    let mut reports: Vec<NpmPackReport> =
        serde_json::from_slice(bytes).map_err(|_| PackageAuditHostError::NpmReportInvalid)?;
    if reports.len() != 1 {
        return Err(PackageAuditHostError::NpmReportInvalid);
    }
    Ok(reports
        .remove(0)
        .files
        .into_iter()
        .map(|file| file.path)
        .collect())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::package_archive::ArchiveFile;
    use ix_trace_rs::trace;

    fn lifecycle_root() -> tempfile::TempDir {
        let directory = tempfile::tempdir().expect("temporary directory must be available");
        let module = directory.path().join("engineering_assurance");
        for name in ["contracts", "fixtures", "schemas", "skeletons"] {
            fs::create_dir_all(module.join(name)).expect("fixture directory must be created");
        }
        fs::write(module.join("manifest.yaml"), b"name: fixture\n")
            .expect("fixture manifest must be written");
        fs::write(module.join("compatibility-matrix.json"), b"{}\n")
            .expect("fixture matrix must be written");
        directory
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn npm_report_requires_exactly_one_result() {
        assert_eq!(
            parse_npm_report(br#"[{"files":[{"path":"one"}]}]"#).unwrap(),
            ["one"]
        );
        assert!(parse_npm_report(b"[]").is_err());
        assert!(parse_npm_report(b"{}").is_err());
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn process_failures_keep_typed_categories() {
        assert!(matches!(
            map_process_error(&ProcessError::TimedOut {
                timeout: Duration::from_secs(1)
            }),
            PackageAuditHostError::ProcessTimedOut
        ));
        assert!(matches!(
            map_process_error(&ProcessError::OutputTooLarge {
                stream: "stdout",
                limit: 1
            }),
            PackageAuditHostError::ProcessOutputTooLarge
        ));
    }

    #[test]
    #[trace("TC-018", "FR-003-AC-5", "TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn npm_report_and_archive_membership_are_independent_gates() {
        let expected = vec!["one".to_owned()];
        let matching = ArchiveSnapshot {
            files: vec![ArchiveFile {
                path: "one".to_owned(),
                bytes: Vec::new(),
            }],
        };
        let mismatching = ArchiveSnapshot {
            files: vec![ArchiveFile {
                path: "other".to_owned(),
                bytes: Vec::new(),
            }],
        };
        assert!(audit_npm_membership(&expected, &["one".to_owned()], &matching).is_ok());
        assert!(matches!(
            audit_npm_membership(&expected, &["other".to_owned()], &matching),
            Err(PackageAuditHostError::MembershipMismatch)
        ));
        assert!(matches!(
            audit_npm_membership(&expected, &["one".to_owned()], &mismatching),
            Err(PackageAuditHostError::MembershipMismatch)
        ));
    }

    #[test]
    #[trace("TC-040", "NFR-003-AC-1")]
    fn repository_allowlists_retain_the_existing_module_roots() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let metadata_root = format!("engineering_assurance-{DISTRIBUTION_VERSION}.dist-info");
        let wheel = ArchiveSnapshot {
            files: vec![ArchiveFile {
                path: format!("{metadata_root}/LICENSE"),
                bytes: Vec::new(),
            }],
        };
        let wheel_members = wheel_allowlist(root, &wheel)
            .expect("wheel allowlist must retain the current module root");
        for required in [
            "engineering_assurance/manifest.yaml",
            "engineering_assurance/contracts/",
            "engineering_assurance/fixtures/",
            "engineering_assurance/schemas/",
            "engineering_assurance/skeletons/",
        ] {
            assert!(
                wheel_members
                    .iter()
                    .any(|member| member == required.trim_end_matches('/')
                        || member.starts_with(required)),
                "wheel allowlist is missing {required}"
            );
        }

        let npm_members =
            npm_allowlist(root).expect("npm allowlist must retain the current module root");
        for required in [
            "engineering_assurance/manifest.yaml",
            "contracts/",
            "fixtures/",
            "schemas/",
            "skeletons/",
        ] {
            assert!(
                npm_members
                    .iter()
                    .any(|member| member == required.trim_end_matches('/')
                        || member.starts_with(required)),
                "npm allowlist is missing {required}"
            );
        }
    }

    #[test]
    #[trace("TC-040", "NFR-003-AC-2")]
    fn archive_observations_cannot_enlarge_the_explicit_allowlist() {
        let expected = ["declared".to_owned()];
        assert!(audit_membership(&expected, &expected).is_ok());
        assert!(matches!(
            audit_membership(&expected, &["declared".to_owned(), "extra".to_owned()]),
            Err(PackageAuditHostError::MembershipMismatch)
        ));
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn archive_content_rights_gate_rejects_a_policy_finding() {
        let denied = ["legal", " advice"].concat();
        let archive = ArchiveSnapshot {
            files: vec![ArchiveFile {
                path: "candidate.md".to_owned(),
                bytes: denied.into_bytes(),
            }],
        };
        assert!(matches!(
            audit_content_rights(&archive, &[]),
            Err(PackageAuditHostError::ContentRightsRefused)
        ));
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn failed_npm_pack_still_cleans_only_the_preflighted_staging_population() {
        let directory = lifecycle_root();
        let before = root_population(directory.path()).expect("fixture root must be inspectable");
        package_host::stage(directory.path()).expect("fixture staging must succeed");
        let result = finish_npm_pack(
            directory.path(),
            &before,
            Err(PackageAuditHostError::ProcessFailed),
        );
        assert!(matches!(result, Err(PackageAuditHostError::ProcessFailed)));
        assert_eq!(
            root_population(directory.path()).expect("cleaned root must be inspectable"),
            before
        );
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn child_created_repository_output_fails_confinement() {
        let directory = tempfile::tempdir().expect("temporary directory must be available");
        let arguments = [OsString::from("-c"), OsString::from("touch escaped-output")];
        let environment = package_temporary_environment(directory.path(), None);
        let result =
            run_required_confined(OsStr::new("sh"), &arguments, directory.path(), &environment);
        assert!(matches!(
            result,
            Err(PackageAuditHostError::RootOutputChanged)
        ));
    }

    #[test]
    #[trace("TC-017", "FR-003-AC-4", "TC-111", "FR-017-AC-3")]
    fn repository_source_install_preserves_installed_discovery() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let temporary = tempfile::tempdir().expect("temporary directory must be available");
        let source = stage_wheel_source(root, temporary.path())
            .expect("repository source must stage outside the selected root");
        let destination = temporary.path().join("repository-source-install");
        let arguments = vec![
            OsString::from("-m"),
            OsString::from("pip"),
            OsString::from("install"),
            OsString::from("--no-index"),
            OsString::from("--no-deps"),
            OsString::from("--no-build-isolation"),
            OsString::from("--target"),
            destination.as_os_str().to_owned(),
            OsString::from("."),
        ];
        let cache = temporary.path().join("pip-cache");
        let environment =
            package_temporary_environment(temporary.path(), Some(("PIP_CACHE_DIR", &cache)));
        let before = root_population(root).expect("repository root must be inspectable");
        let process = invoke(OsStr::new("python3"), &arguments, &source, &environment)
            .expect("repository-source installation must be observable");
        assert!(
            process.status.success(),
            "{}",
            String::from_utf8_lossy(&process.stderr)
        );
        assert_eq!(
            root_population(root).expect("repository root must remain inspectable"),
            before
        );
        let installed =
            package_install::inspect(&destination, &destination.join("engineering_assurance"))
                .expect("repository-source installation must preserve discovery");
        assert!(installed.canonical_file_count() > 0);
    }

    #[cfg(unix)]
    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn fixed_package_sources_refuse_linked_path_components_before_build() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("temporary directory must be available");
        fs::create_dir(directory.path().join("real")).expect("fixture directory must be created");
        fs::write(directory.path().join("real/source.txt"), b"source")
            .expect("fixture source must be written");
        assert!(validate_regular_source(directory.path(), "real/source.txt").is_ok());

        symlink("real", directory.path().join("linked"))
            .expect("fixture directory link must be created");
        assert!(matches!(
            validate_regular_source(directory.path(), "linked/source.txt"),
            Err(PackageAuditHostError::RootInvalid)
        ));
    }

    #[cfg(unix)]
    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn wheel_build_configuration_is_a_preflighted_fixed_source() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().expect("temporary directory must be available");
        let fixed_sources = ROOT_DATA_FILES
            .into_iter()
            .chain(
                NPM_ROOT_FILES
                    .into_iter()
                    .filter(|source| *source != "manifest.yaml"),
            )
            .chain(["pyproject.toml", "setup.cfg"])
            .collect::<BTreeSet<_>>();
        for source in fixed_sources {
            let path = directory.path().join(source);
            fs::create_dir_all(path.parent().expect("fixed source must have a parent"))
                .expect("fixed-source parent must be created");
            fs::write(path, b"fixture").expect("fixed source must be written");
        }
        fs::write(directory.path().join("outside.toml"), b"[build-system]\n")
            .expect("fixture build configuration must be written");
        fs::remove_file(directory.path().join("pyproject.toml"))
            .expect("regular build configuration must be removed");
        symlink("outside.toml", directory.path().join("pyproject.toml"))
            .expect("fixture build configuration link must be created");
        assert!(matches!(
            selected_root(directory.path()),
            Err(PackageAuditHostError::RootInvalid)
        ));
    }
}
