// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Package builders, archives, installers, and filesystem orchestration.

use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    fs,
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
const DISTRIBUTION_VERSION: &str = "0.2.0";
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
            Self::ProtectedTokensInvalid => "package_audit_protected_tokens_invalid",
            Self::ProcessUnavailable => "package_audit_process_unavailable",
            Self::ProcessTimedOut => "package_audit_process_timed_out",
            Self::ProcessOutputTooLarge => "package_audit_process_output_too_large",
            Self::ProcessObservationFailed => "package_audit_process_observation_failed",
            Self::ProcessFailed => "package_audit_process_failed",
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
    install_wheel(&root, &wheel_path, &wheel_install)?;
    let wheel_bundle =
        package_install::inspect(&wheel_install, &wheel_install.join("engineering_assurance"))?;

    let npm_report = build_npm(&root, output)?;
    let npm_expected = npm_allowlist(&root)?;
    audit_membership(&npm_expected, &npm_report)?;
    let npm_path = select_archive(output, "tgz")?;
    let npm = package_archive::read_npm(&npm_path)?;
    audit_membership(&npm_expected, &npm.names())?;
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
    Ok(root)
}

fn build_wheel(root: &Path, output: &Path) -> Result<(), PackageAuditHostError> {
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
    run_required(OsStr::new("python3"), &arguments, root, &[]).map(|_| ())
}

fn install_wheel(
    root: &Path,
    wheel: &Path,
    destination: &Path,
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
    run_required(OsStr::new("python3"), &arguments, root, &[]).map(|_| ())
}

fn build_npm(root: &Path, output: &Path) -> Result<Vec<String>, PackageAuditHostError> {
    package_host::require_staged_destinations_absent(root)
        .map_err(|_| PackageAuditHostError::NpmStagingInvalid)?;
    let arguments = vec![
        OsString::from("pack"),
        OsString::from("--json"),
        OsString::from("--pack-destination"),
        output.as_os_str().to_owned(),
    ];
    let cache = output.join("npm-cache");
    let environment = vec![
        (
            OsString::from("npm_config_cache"),
            cache.as_os_str().to_owned(),
        ),
        (OsString::from("CARGO_BUILD_JOBS"), OsString::from("2")),
    ];
    let process = invoke(OsStr::new("npm"), &arguments, root, &environment);
    package_host::clean(root).map_err(|_| PackageAuditHostError::NpmStagingCleanupFailed)?;
    let process = process?;
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
    let environment = vec![(
        OsString::from("npm_config_cache"),
        cache.as_os_str().to_owned(),
    )];
    run_required(OsStr::new("npm"), &arguments, root, &environment).map(|_| ())
}

fn run_required(
    executable: &OsStr,
    arguments: &[OsString],
    root: &Path,
    environment: &[(OsString, OsString)],
) -> Result<process_host::CompletedProcess, PackageAuditHostError> {
    let process = invoke(executable, arguments, root, environment)?;
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
        ProcessLimits {
            timeout: PACKAGE_PROCESS_TIMEOUT,
            max_output_bytes: MAX_PROCESS_OUTPUT_BYTES,
        },
    )
    .map_err(|error| map_process_error(&error))
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
    use super::*;
    use ix_trace_rs::trace;

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
}
