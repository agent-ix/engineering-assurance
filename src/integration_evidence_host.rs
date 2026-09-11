// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust host adapter for the release qualification evidence boundary.
//!
//! It owns bounded Quire invocation, typed coverage decoding, immutable source
//! revision observation, and reconciliation of retained governing identities.
//! The evaluation-report adapter remains the sole owner of report and
//! transcript decoding.

use std::{
    collections::BTreeMap,
    env,
    ffi::{OsStr, OsString},
    fmt::Write as _,
    fs,
    path::{Component, Path, PathBuf},
    time::Duration,
};

use engineering_assurance::evidence::VersionIdentity;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    evaluation_report_host::{self, RetainedGoverning},
    process_host::{self, ProcessError, ProcessLimits},
};

const COVERAGE_TIMEOUT: Duration = Duration::from_secs(60);
const VERSION_TIMEOUT: Duration = Duration::from_secs(15);
const GIT_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_MATRIX_BYTES: u64 = 8 * 1024 * 1024;
const MAX_RUNTIME_FILES: usize = 16_384;
const MAX_RUNTIME_BYTES: u64 = 64 * 1024 * 1024;
const REQUIRED_TEST_CASES: usize = 68;

#[derive(Debug, Error)]
pub(crate) enum IntegrationEvidenceError {
    #[error("integration-evidence repository root is invalid")]
    RootInvalid,
    #[error("quire coverage process is unavailable")]
    QuireUnavailable,
    #[error("quire coverage did not terminate before its time limit")]
    QuireTimedOut,
    #[error("quire coverage output exceeded its byte limit")]
    QuireOutputTooLarge,
    #[error("quire coverage process could not be observed")]
    QuireObservationFailed,
    #[error("quire coverage exited unsuccessfully")]
    QuireFailed,
    #[error("quire coverage returned an invalid document")]
    CoverageInvalid,
    #[error("repository test matrix is invalid")]
    MatrixInvalid,
    #[error("current source revision is unavailable")]
    RevisionUnavailable,
    #[error("current source revision is invalid")]
    RevisionInvalid,
    #[error("retained evaluation artifact is invalid")]
    EvaluationArtifactInvalid,
    #[error("retained governing identity differs from the current governed input")]
    GoverningIdentityChanged,
    #[error("governing executable is unavailable or invalid")]
    GoverningExecutableInvalid,
    #[error("governing runtime package is unavailable or invalid")]
    GoverningRuntimeInvalid,
}

impl IntegrationEvidenceError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::RootInvalid => "integration_evidence_root_invalid",
            Self::QuireUnavailable => "integration_evidence_quire_unavailable",
            Self::QuireTimedOut => "integration_evidence_quire_timed_out",
            Self::QuireOutputTooLarge => "integration_evidence_quire_output_too_large",
            Self::QuireObservationFailed => "integration_evidence_quire_observation_failed",
            Self::QuireFailed => "integration_evidence_quire_failed",
            Self::CoverageInvalid => "integration_evidence_coverage_invalid",
            Self::MatrixInvalid => "integration_evidence_matrix_invalid",
            Self::RevisionUnavailable => "integration_evidence_revision_unavailable",
            Self::RevisionInvalid => "integration_evidence_revision_invalid",
            Self::EvaluationArtifactInvalid => "integration_evidence_artifact_invalid",
            Self::GoverningIdentityChanged => "integration_evidence_governing_identity_changed",
            Self::GoverningExecutableInvalid => "integration_evidence_governing_executable_invalid",
            Self::GoverningRuntimeInvalid => "integration_evidence_governing_runtime_invalid",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IntegrationEvidence {
    pub(crate) evaluation_cells: Option<(usize, usize)>,
}

#[derive(Debug, Deserialize)]
struct CoverageDocument {
    totals: CoverageTotals,
    #[serde(default)]
    unbacked_rows: Vec<CoverageReference>,
    #[serde(default)]
    status_lies: Vec<CoverageReference>,
    #[serde(default)]
    untracked_symbols: Vec<CoverageReference>,
    #[serde(default)]
    groups: Vec<CoverageGroup>,
    #[serde(default)]
    diagnostics: Vec<CoverageDiagnostic>,
}

#[derive(Debug, Deserialize)]
struct CoverageTotals {
    backed: usize,
    total: usize,
}

#[derive(Debug, Deserialize)]
struct CoverageReference {}

#[derive(Debug, Deserialize)]
struct CoverageGroup {
    document: String,
    target: String,
    backed: usize,
    total: usize,
}

#[derive(Debug, Deserialize)]
struct CoverageDiagnostic {
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VersionDocument {
    version: String,
}

#[derive(Serialize)]
pub(crate) struct AgentEvalsSnapshot {
    source_revision: String,
    host: VersionIdentity,
    governing: AgentEvalsGoverning,
    workflows: BTreeMap<String, VersionIdentity>,
}

#[derive(Serialize)]
struct AgentEvalsGoverning {
    module: VersionIdentity,
    plugin: VersionIdentity,
    skill: VersionIdentity,
    quire: VersionIdentity,
    quoin: VersionIdentity,
    ix_flow: VersionIdentity,
    schema: VersionIdentity,
    producer: VersionIdentity,
}

pub(crate) fn agent_evals_snapshot(
    root: &Path,
    agent: &str,
) -> Result<AgentEvalsSnapshot, IntegrationEvidenceError> {
    let root = canonical_root(root)?;
    let path = selected_path(&root);
    let module_version = yaml_version(&root.join("engineering_assurance/manifest.yaml"))?;
    let plugin_version = json_version(&root.join(".codex-plugin/plugin.json"))?;
    let mut workflows = BTreeMap::new();
    for name in ["assurance-intake", "architecture-evaluation"] {
        let definition = root
            .join("engineering_assurance/skills/assurance-onboarding/workflows")
            .join(name)
            .join("def.yaml");
        workflows.insert(
            name.to_owned(),
            file_identity(name, yaml_version(&definition)?, &definition)?,
        );
    }
    Ok(AgentEvalsSnapshot {
        source_revision: source_revision(&root)?,
        host: executable_identity(agent, &path)?,
        governing: AgentEvalsGoverning {
            module: file_identity(
                "engineering-assurance",
                module_version,
                &root.join("engineering_assurance/manifest.yaml"),
            )?,
            plugin: file_identity(
                "engineering-assurance-plugin",
                plugin_version.clone(),
                &root.join(".codex-plugin/plugin.json"),
            )?,
            skill: file_identity(
                "assurance-onboarding",
                plugin_version,
                &root.join("engineering_assurance/skills/assurance-onboarding/SKILL.md"),
            )?,
            quire: executable_identity("quire", &path)?,
            quoin: executable_identity("quoin", &path)?,
            ix_flow: runtime_identity("ix-flow", &path)?,
            schema: file_identity(
                "evaluation-result-contract",
                "evaluation-result-v1".to_owned(),
                &root.join("src/agent_evals_provider.rs"),
            )?,
            producer: runtime_identity("cli-evals", &path)?,
        },
        workflows,
    })
}

pub(crate) fn traceability(
    root: &Path,
    quire: &OsStr,
) -> Result<IntegrationEvidence, IntegrationEvidenceError> {
    let root = canonical_root(root)?;
    let coverage = run_coverage(&root, quire)?;
    validate_coverage(&coverage, &root)?;
    validate_matrix(&root)?;
    Ok(IntegrationEvidence {
        evaluation_cells: None,
    })
}

pub(crate) fn release(
    root: &Path,
    workspace_root: &Path,
    artifact: &Path,
    quire: &OsStr,
) -> Result<IntegrationEvidence, IntegrationEvidenceError> {
    let root = canonical_root(root)?;
    let coverage = run_coverage(&root, quire)?;
    validate_coverage(&coverage, &root)?;
    validate_matrix(&root)?;
    let revision = source_revision(&root)?;
    let retained = evaluation_report_host::verify_complete_artifact(
        &root,
        workspace_root,
        artifact,
        &revision,
    )
    .map_err(|_| IntegrationEvidenceError::EvaluationArtifactInvalid)?;
    validate_governing(&root, &retained)?;
    Ok(IntegrationEvidence {
        evaluation_cells: Some((28, 28)),
    })
}

fn canonical_root(root: &Path) -> Result<PathBuf, IntegrationEvidenceError> {
    let metadata = fs::symlink_metadata(root).map_err(|_| IntegrationEvidenceError::RootInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(IntegrationEvidenceError::RootInvalid);
    }
    fs::canonicalize(root).map_err(|_| IntegrationEvidenceError::RootInvalid)
}

fn run_coverage(root: &Path, quire: &OsStr) -> Result<CoverageDocument, IntegrationEvidenceError> {
    let arguments = [
        OsStr::new("coverage"),
        OsStr::new("--scope"),
        root.as_os_str(),
        OsStr::new("--json"),
    ];
    let output = process_host::run_configured(
        quire,
        &arguments,
        Some(root),
        &[],
        &[],
        ProcessLimits {
            timeout: COVERAGE_TIMEOUT,
            max_output_bytes: MAX_OUTPUT_BYTES,
        },
    )
    .map_err(|error| map_quire_process(&error))?;
    if !output.status.success() {
        return Err(IntegrationEvidenceError::QuireFailed);
    }
    serde_json::from_slice(&output.stdout).map_err(|_| IntegrationEvidenceError::CoverageInvalid)
}

fn map_quire_process(error: &ProcessError) -> IntegrationEvidenceError {
    match error {
        ProcessError::Unavailable { .. } => IntegrationEvidenceError::QuireUnavailable,
        ProcessError::TimedOut { .. } => IntegrationEvidenceError::QuireTimedOut,
        ProcessError::OutputTooLarge { .. } => IntegrationEvidenceError::QuireOutputTooLarge,
        ProcessError::PipeUnavailable { .. }
        | ProcessError::Observation { .. }
        | ProcessError::OutputUnreadable { .. } => IntegrationEvidenceError::QuireObservationFailed,
    }
}

fn validate_coverage(
    coverage: &CoverageDocument,
    root: &Path,
) -> Result<(), IntegrationEvidenceError> {
    if coverage.totals.total == 0 || coverage.totals.backed != coverage.totals.total {
        return Err(IntegrationEvidenceError::CoverageInvalid);
    }
    if !coverage.unbacked_rows.is_empty()
        || !coverage.status_lies.is_empty()
        || !coverage.untracked_symbols.is_empty()
    {
        return Err(IntegrationEvidenceError::CoverageInvalid);
    }
    let test_cases = coverage
        .groups
        .iter()
        .find(|group| group.document == "spec/tests.md" && group.target == "test-case");
    if !matches!(test_cases, Some(group) if group.backed == REQUIRED_TEST_CASES && group.total == REQUIRED_TEST_CASES)
    {
        return Err(IntegrationEvidenceError::CoverageInvalid);
    }
    if coverage.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.reason.as_deref(),
            Some("hollow-denominator" | "marker-form-mismatch" | "section-matches-nothing")
        ) && diagnostic
            .path
            .as_deref()
            .is_none_or(|path| is_local_diagnostic(path, root))
    }) {
        return Err(IntegrationEvidenceError::CoverageInvalid);
    }
    Ok(())
}

fn is_local_diagnostic(value: &str, root: &Path) -> bool {
    let path = Path::new(value);
    !path.is_absolute() || path.starts_with(root)
}

fn validate_matrix(root: &Path) -> Result<(), IntegrationEvidenceError> {
    let path = root.join("spec/tests.md");
    let metadata =
        fs::symlink_metadata(&path).map_err(|_| IntegrationEvidenceError::MatrixInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > MAX_MATRIX_BYTES
    {
        return Err(IntegrationEvidenceError::MatrixInvalid);
    }
    let bytes = fs::read(path).map_err(|_| IntegrationEvidenceError::MatrixInvalid)?;
    if bytes
        .windows("🚧".len())
        .any(|value| value == "🚧".as_bytes())
        || bytes
            .windows("⛔".len())
            .any(|value| value == "⛔".as_bytes())
        || bytes
            .windows("❌".len())
            .any(|value| value == "❌".as_bytes())
    {
        return Err(IntegrationEvidenceError::MatrixInvalid);
    }
    Ok(())
}

fn source_revision(root: &Path) -> Result<String, IntegrationEvidenceError> {
    let output = process_host::run_configured(
        OsStr::new("git"),
        &[OsStr::new("rev-parse"), OsStr::new("HEAD")],
        Some(root),
        &[],
        &[],
        ProcessLimits {
            timeout: GIT_TIMEOUT,
            max_output_bytes: 128,
        },
    )
    .map_err(|_| IntegrationEvidenceError::RevisionUnavailable)?;
    if !output.status.success() {
        return Err(IntegrationEvidenceError::RevisionUnavailable);
    }
    let revision = std::str::from_utf8(&output.stdout)
        .ok()
        .map(str::trim)
        .filter(|value| engineering_assurance::evaluation::is_immutable_revision(value))
        .ok_or(IntegrationEvidenceError::RevisionInvalid)?;
    Ok(revision.to_owned())
}

fn validate_governing(
    root: &Path,
    retained: &RetainedGoverning,
) -> Result<(), IntegrationEvidenceError> {
    let module = file_identity(
        "engineering-assurance",
        yaml_version(&root.join("engineering_assurance/manifest.yaml"))?,
        &root.join("engineering_assurance/manifest.yaml"),
    )?;
    let plugin_version = json_version(&root.join(".codex-plugin/plugin.json"))?;
    let plugin = file_identity(
        "engineering-assurance-plugin",
        plugin_version.clone(),
        &root.join(".codex-plugin/plugin.json"),
    )?;
    let skill = file_identity(
        "assurance-onboarding",
        plugin_version,
        &root.join("engineering_assurance/skills/assurance-onboarding/SKILL.md"),
    )?;
    let schema = file_identity(
        "evaluation-result-contract",
        "evaluation-result-v1".to_owned(),
        &root.join("src/agent_evals_provider.rs"),
    )?;
    if (module, plugin, skill, schema)
        != (
            retained.module.clone(),
            retained.plugin.clone(),
            retained.skill.clone(),
            retained.schema.clone(),
        )
    {
        return Err(IntegrationEvidenceError::GoverningIdentityChanged);
    }
    for (name, expected) in &retained.workflows {
        if !safe_name(name) {
            return Err(IntegrationEvidenceError::GoverningIdentityChanged);
        }
        let path = root
            .join("engineering_assurance/skills/assurance-onboarding/workflows")
            .join(name)
            .join("def.yaml");
        let current = file_identity(name, yaml_version(&path)?, &path)?;
        if current != *expected {
            return Err(IntegrationEvidenceError::GoverningIdentityChanged);
        }
    }
    let path = selected_path(root);
    let quire = executable_identity("quire", &path)?;
    let quoin = executable_identity("quoin", &path)?;
    let ix_flow = runtime_identity("ix-flow", &path)?;
    let producer = runtime_identity("cli-evals", &path)?;
    if (quire, quoin, ix_flow, producer)
        != (
            retained.quire.clone(),
            retained.quoin.clone(),
            retained.ix_flow.clone(),
            retained.producer.clone(),
        )
    {
        return Err(IntegrationEvidenceError::GoverningIdentityChanged);
    }
    Ok(())
}

fn yaml_version(path: &Path) -> Result<String, IntegrationEvidenceError> {
    let bytes = fs::read(path).map_err(|_| IntegrationEvidenceError::GoverningIdentityChanged)?;
    let document: VersionDocument = yaml_serde::from_slice(&bytes)
        .map_err(|_| IntegrationEvidenceError::GoverningIdentityChanged)?;
    if document.version.is_empty() {
        return Err(IntegrationEvidenceError::GoverningIdentityChanged);
    }
    Ok(document.version)
}

fn json_version(path: &Path) -> Result<String, IntegrationEvidenceError> {
    #[derive(Deserialize)]
    struct PackageVersion {
        version: String,
    }
    let bytes = fs::read(path).map_err(|_| IntegrationEvidenceError::GoverningIdentityChanged)?;
    let document: PackageVersion = serde_json::from_slice(&bytes)
        .map_err(|_| IntegrationEvidenceError::GoverningIdentityChanged)?;
    if document.version.is_empty() {
        return Err(IntegrationEvidenceError::GoverningIdentityChanged);
    }
    Ok(document.version)
}

fn file_identity(
    name: &str,
    version: String,
    path: &Path,
) -> Result<VersionIdentity, IntegrationEvidenceError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| IntegrationEvidenceError::GoverningIdentityChanged)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(IntegrationEvidenceError::GoverningIdentityChanged);
    }
    Ok(VersionIdentity {
        name: name.to_owned(),
        version,
        digest: digest(
            &fs::read(path).map_err(|_| IntegrationEvidenceError::GoverningIdentityChanged)?,
        ),
    })
}

fn selected_path(root: &Path) -> OsString {
    let mut paths = vec![root.join(".agent-evals/bin")];
    paths.extend(
        env::var_os("PATH")
            .as_deref()
            .map(env::split_paths)
            .into_iter()
            .flatten(),
    );
    env::join_paths(paths).unwrap_or_default()
}

fn executable_identity(
    name: &str,
    path: &OsStr,
) -> Result<VersionIdentity, IntegrationEvidenceError> {
    let executable =
        find_executable(name, path).ok_or(IntegrationEvidenceError::GoverningExecutableInvalid)?;
    let version =
        command_version(&executable).ok_or(IntegrationEvidenceError::GoverningExecutableInvalid)?;
    let executable = fs::canonicalize(executable)
        .map_err(|_| IntegrationEvidenceError::GoverningExecutableInvalid)?;
    file_identity(name, version, &executable)
        .map_err(|_| IntegrationEvidenceError::GoverningExecutableInvalid)
}

fn runtime_identity(name: &str, path: &OsStr) -> Result<VersionIdentity, IntegrationEvidenceError> {
    let executable =
        find_executable(name, path).ok_or(IntegrationEvidenceError::GoverningRuntimeInvalid)?;
    let version =
        command_version(&executable).ok_or(IntegrationEvidenceError::GoverningRuntimeInvalid)?;
    let executable = fs::canonicalize(executable)
        .map_err(|_| IntegrationEvidenceError::GoverningRuntimeInvalid)?;
    let package = executable
        .parent()
        .and_then(Path::parent)
        .ok_or(IntegrationEvidenceError::GoverningRuntimeInvalid)?
        .to_path_buf();
    let manifest = package.join("package.json");
    let dist = package.join("dist");
    if !manifest.is_file() || !dist.is_dir() {
        return Err(IntegrationEvidenceError::GoverningRuntimeInvalid);
    }
    let mut files = vec![manifest, executable];
    let mut runtime_bytes = 0_u64;
    for file in &files {
        admit_runtime_file(file, &mut runtime_bytes)?;
    }
    collect_runtime_files(&dist, &mut files, &mut runtime_bytes)?;
    files.sort();
    files.dedup();
    let mut hasher = Sha256::new();
    for file in files {
        let relative = file
            .strip_prefix(&package)
            .map_err(|_| IntegrationEvidenceError::GoverningRuntimeInvalid)?;
        hasher.update(relative.to_string_lossy().replace('\\', "/").as_bytes());
        hasher.update([0]);
        hasher
            .update(fs::read(file).map_err(|_| IntegrationEvidenceError::GoverningRuntimeInvalid)?);
        hasher.update([0]);
    }
    Ok(VersionIdentity {
        name: name.to_owned(),
        version,
        digest: hex_digest(hasher.finalize()),
    })
}

fn collect_runtime_files(
    directory: &Path,
    files: &mut Vec<PathBuf>,
    total_bytes: &mut u64,
) -> Result<(), IntegrationEvidenceError> {
    for entry in
        fs::read_dir(directory).map_err(|_| IntegrationEvidenceError::GoverningRuntimeInvalid)?
    {
        let entry = entry.map_err(|_| IntegrationEvidenceError::GoverningRuntimeInvalid)?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| IntegrationEvidenceError::GoverningRuntimeInvalid)?;
        if metadata.file_type().is_symlink() {
            return Err(IntegrationEvidenceError::GoverningRuntimeInvalid);
        }
        if metadata.is_dir() {
            collect_runtime_files(&path, files, total_bytes)?;
        } else if metadata.is_file() {
            if files.len() >= MAX_RUNTIME_FILES {
                return Err(IntegrationEvidenceError::GoverningRuntimeInvalid);
            }
            admit_runtime_metadata(&metadata, total_bytes)?;
            files.push(path);
        } else {
            return Err(IntegrationEvidenceError::GoverningRuntimeInvalid);
        }
    }
    Ok(())
}

fn admit_runtime_file(path: &Path, total_bytes: &mut u64) -> Result<(), IntegrationEvidenceError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| IntegrationEvidenceError::GoverningRuntimeInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(IntegrationEvidenceError::GoverningRuntimeInvalid);
    }
    admit_runtime_metadata(&metadata, total_bytes)
}

fn admit_runtime_metadata(
    metadata: &fs::Metadata,
    total_bytes: &mut u64,
) -> Result<(), IntegrationEvidenceError> {
    *total_bytes = total_bytes
        .checked_add(metadata.len())
        .ok_or(IntegrationEvidenceError::GoverningRuntimeInvalid)?;
    if *total_bytes > MAX_RUNTIME_BYTES {
        return Err(IntegrationEvidenceError::GoverningRuntimeInvalid);
    }
    Ok(())
}

fn find_executable(name: &str, path: &OsStr) -> Option<PathBuf> {
    env::split_paths(path)
        .map(|directory| directory.join(name))
        .find(|candidate| candidate.is_file())
}

fn command_version(executable: &Path) -> Option<String> {
    let output = process_host::run(
        executable.as_os_str(),
        &[OsStr::new("--version")],
        ProcessLimits {
            timeout: VERSION_TIMEOUT,
            max_output_bytes: 8 * 1024,
        },
    )
    .ok()?;
    if !output.status.success() {
        return None;
    }
    let bytes = if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    };
    std::str::from_utf8(bytes)
        .ok()?
        .lines()
        .next()
        .map(str::to_owned)
        .filter(|line| !line.is_empty())
}

fn safe_name(value: &str) -> bool {
    !value.is_empty()
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && !value.contains(['/', '\\'])
}

fn digest(bytes: &[u8]) -> String {
    hex_digest(Sha256::digest(bytes))
}

fn hex_digest<D: AsRef<[u8]>>(digest: D) -> String {
    let mut encoded = String::with_capacity(digest.as_ref().len().saturating_mul(2));
    for byte in digest.as_ref() {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[test]
    #[trace("TC-111", "FR-017-AC-3")]
    fn tc_111_coverage_requires_the_complete_closed_population() {
        let complete = CoverageDocument {
            totals: CoverageTotals {
                backed: 92,
                total: 92,
            },
            unbacked_rows: vec![],
            status_lies: vec![],
            untracked_symbols: vec![],
            groups: vec![CoverageGroup {
                document: "spec/tests.md".into(),
                target: "test-case".into(),
                backed: 68,
                total: 68,
            }],
            diagnostics: vec![],
        };
        let root = tempfile::tempdir().expect("fixture root");
        assert!(validate_coverage(&complete, root.path()).is_ok());
        let incomplete = CoverageDocument {
            totals: CoverageTotals {
                backed: 91,
                total: 92,
            },
            ..complete
        };
        assert!(matches!(
            validate_coverage(&incomplete, root.path()),
            Err(IntegrationEvidenceError::CoverageInvalid)
        ));
    }
}
