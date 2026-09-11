// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Filesystem adapter for retained cli-agent-evals reports and transcripts.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
};

use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use engineering_assurance::{
    evaluation::{
        EvaluationEnvelope, EvaluationFailure, EvaluationHost, aggregate_evaluations,
        is_immutable_revision,
    },
    evaluation_reports::{DecodedEvaluationSample, MAX_CLI_REPORT_BYTES, decode_cli_eval_report},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub(crate) const MAX_REPORT_COLLECTION: usize = 64;
pub(crate) const MAX_TRANSCRIPT_BYTES: usize = 64 * 1024 * 1024;

const ARTIFACT_REVISION: &str = "evaluation-aggregate-v1";

#[derive(Clone, Copy, Debug, Error)]
pub(crate) enum EvaluationReportHostError {
    #[error("evaluation report repository root is invalid")]
    RepositoryRootInvalid,
    #[error("evaluation workspace root is invalid")]
    WorkspaceRootInvalid,
    #[error("expected source revision is not immutable")]
    SourceRevisionInvalid,
    #[error("evaluation report collection exceeds its fixed limit")]
    ReportCollectionTooLarge,
    #[error("evaluation aggregate output path is invalid")]
    OutputPathInvalid,
    #[error("evaluation aggregate output could not be written")]
    OutputWriteFailed,
    #[error("evaluation aggregate artifact could not be encoded")]
    ArtifactEncodingFailed,
    #[error("evaluation aggregate artifact path is invalid")]
    ArtifactPathInvalid,
    #[error("evaluation aggregate artifact could not be read")]
    ArtifactReadFailed,
    #[error("evaluation aggregate artifact is invalid")]
    ArtifactInvalid,
    #[error("evaluation aggregate artifact does not match retained reports")]
    ArtifactMismatch,
}

impl EvaluationReportHostError {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::RepositoryRootInvalid => "evaluation_report_repository_root_invalid",
            Self::WorkspaceRootInvalid => "evaluation_report_workspace_root_invalid",
            Self::SourceRevisionInvalid => "evaluation_report_source_revision_invalid",
            Self::ReportCollectionTooLarge => "evaluation_report_collection_too_large",
            Self::OutputPathInvalid => "evaluation_report_output_path_invalid",
            Self::OutputWriteFailed => "evaluation_report_output_write_failed",
            Self::ArtifactEncodingFailed => "evaluation_report_artifact_encoding_failed",
            Self::ArtifactPathInvalid => "evaluation_report_artifact_path_invalid",
            Self::ArtifactReadFailed => "evaluation_report_artifact_read_failed",
            Self::ArtifactInvalid => "evaluation_report_artifact_invalid",
            Self::ArtifactMismatch => "evaluation_report_artifact_mismatch",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EvaluationReportIdentity {
    path: String,
    digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EvaluationAggregateArtifact {
    revision: String,
    generated_at: String,
    source_revision: String,
    reports: Vec<EvaluationReportIdentity>,
    models: BTreeMap<String, String>,
    failed_attempts: Vec<String>,
    required_cells: usize,
    complete_cells: usize,
    ok: bool,
    failures: Vec<String>,
}

impl EvaluationAggregateArtifact {
    pub(crate) const fn ok(&self) -> bool {
        self.ok
    }

    pub(crate) const fn complete_cells(&self) -> usize {
        self.complete_cells
    }

    pub(crate) const fn required_cells(&self) -> usize {
        self.required_cells
    }

    pub(crate) fn failures(&self) -> &[String] {
        &self.failures
    }

    fn to_json(&self) -> Result<Vec<u8>, EvaluationReportHostError> {
        let mut bytes = serde_json::to_vec_pretty(self)
            .map_err(|_| EvaluationReportHostError::ArtifactEncodingFailed)?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

pub(crate) fn execute(
    repository_root: &Path,
    workspace_root: &Path,
    report_paths: &[PathBuf],
    source_revision: &str,
    generated_at: String,
) -> Result<EvaluationAggregateArtifact, EvaluationReportHostError> {
    if report_paths.len() > MAX_REPORT_COLLECTION {
        return Err(EvaluationReportHostError::ReportCollectionTooLarge);
    }
    if !is_immutable_revision(source_revision) {
        return Err(EvaluationReportHostError::SourceRevisionInvalid);
    }
    let (_repository_root, repository) = open_root(
        repository_root,
        EvaluationReportHostError::RepositoryRootInvalid,
    )?;
    let (workspace_root, workspace) = open_root(
        workspace_root,
        EvaluationReportHostError::WorkspaceRootInvalid,
    )?;

    let mut identities = Vec::new();
    let mut envelopes = Vec::new();
    let mut failed_attempts = Vec::new();
    let mut diagnostics = Vec::new();
    let mut observed_models: BTreeMap<EvaluationHost, BTreeSet<String>> = BTreeMap::new();

    for report_path in report_paths {
        let Some(relative) = safe_relative_text(report_path) else {
            diagnostics.push("report-path-invalid".to_owned());
            continue;
        };
        let bytes = match read_rooted_file(&repository, Path::new(relative), MAX_CLI_REPORT_BYTES) {
            Ok(bytes) => bytes,
            Err(code) => {
                diagnostics.push(format!("{relative}:{code}"));
                continue;
            }
        };
        identities.push(EvaluationReportIdentity {
            path: relative.to_owned(),
            digest: sha256_hex(&bytes),
        });
        let decoded = match decode_cli_eval_report(&bytes, source_revision) {
            Ok(decoded) => decoded,
            Err(error) => {
                diagnostics.push(format!("{relative}:{}", error.code()));
                continue;
            }
        };
        observed_models
            .entry(decoded.host())
            .or_default()
            .insert(decoded.model().to_owned());
        admit_samples(
            &workspace_root,
            &workspace,
            relative,
            decoded.samples(),
            &mut envelopes,
            &mut failed_attempts,
            &mut diagnostics,
        );
    }

    let mut models = BTreeMap::new();
    for (host, values) in observed_models {
        if values.len() == 1 {
            models.insert(
                host.as_str().to_owned(),
                values.into_iter().next().unwrap_or_default(),
            );
        } else {
            diagnostics.push(format!(
                "{}:model-mismatch:{}",
                host.as_str(),
                values.into_iter().collect::<Vec<_>>().join(",")
            ));
        }
    }

    identities.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.digest.cmp(&right.digest))
    });
    failed_attempts.sort();
    diagnostics.sort();
    let aggregate = aggregate_evaluations(&envelopes);
    let mut failures = diagnostics;
    failures.extend(retained_aggregate_failures(&aggregate.failures));
    Ok(EvaluationAggregateArtifact {
        revision: ARTIFACT_REVISION.to_owned(),
        generated_at,
        source_revision: source_revision.to_owned(),
        reports: identities,
        models,
        failed_attempts,
        required_cells: aggregate.required_cells,
        complete_cells: aggregate.complete_cells,
        ok: failures.is_empty() && aggregate.ok,
        failures,
    })
}

/// Recompute a retained aggregate from its referenced reports and transcripts.
///
/// The caller supplies the expected immutable source revision instead of relying
/// on an ambient checkout. Successful verification proves every retained report
/// byte, transcript byte, host model, failed attempt, and aggregate field still
/// corresponds to the artifact.
pub(crate) fn verify_artifact(
    repository_root: &Path,
    workspace_root: &Path,
    artifact_path: &Path,
    expected_source_revision: &str,
) -> Result<(), EvaluationReportHostError> {
    if !is_immutable_revision(expected_source_revision) {
        return Err(EvaluationReportHostError::SourceRevisionInvalid);
    }
    let (_, repository) = open_root(
        repository_root,
        EvaluationReportHostError::RepositoryRootInvalid,
    )?;
    let Some(relative) = safe_relative_text(artifact_path) else {
        return Err(EvaluationReportHostError::ArtifactPathInvalid);
    };
    let bytes = read_rooted_file(&repository, Path::new(relative), MAX_CLI_REPORT_BYTES)
        .map_err(|_| EvaluationReportHostError::ArtifactReadFailed)?;
    let artifact: EvaluationAggregateArtifact =
        serde_json::from_slice(&bytes).map_err(|_| EvaluationReportHostError::ArtifactInvalid)?;
    if artifact.revision != ARTIFACT_REVISION
        || artifact.source_revision != expected_source_revision
        || artifact.reports.len() > MAX_REPORT_COLLECTION
    {
        return Err(EvaluationReportHostError::ArtifactInvalid);
    }
    let report_paths = artifact
        .reports
        .iter()
        .map(|identity| PathBuf::from(&identity.path))
        .collect::<Vec<_>>();
    let recomputed = execute(
        repository_root,
        workspace_root,
        &report_paths,
        expected_source_revision,
        artifact.generated_at.clone(),
    )?;
    if recomputed == artifact {
        Ok(())
    } else {
        Err(EvaluationReportHostError::ArtifactMismatch)
    }
}

fn retained_aggregate_failures(failures: &[EvaluationFailure]) -> Vec<String> {
    let mut missing = failures
        .iter()
        .filter(|failure| matches!(failure, EvaluationFailure::Missing(_)))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut cell_failures = failures
        .iter()
        .filter(|failure| {
            matches!(
                failure,
                EvaluationFailure::Duplicate(_) | EvaluationFailure::Envelope { .. }
            )
        })
        .collect::<Vec<_>>();
    let remaining = failures.iter().filter(|failure| {
        !matches!(
            failure,
            EvaluationFailure::Missing(_)
                | EvaluationFailure::Duplicate(_)
                | EvaluationFailure::Envelope { .. }
        )
    });
    missing.sort();
    cell_failures.sort_by_key(|failure| match failure {
        EvaluationFailure::Duplicate(cell) | EvaluationFailure::Envelope { cell, .. } => {
            (cell.host.as_str(), cell.scenario.as_str())
        }
        _ => unreachable!("filtered evaluation failure must carry a cell"),
    });
    missing
        .into_iter()
        .chain(cell_failures.into_iter().map(ToString::to_string))
        .chain(remaining.map(ToString::to_string))
        .collect()
}

pub(crate) fn write_artifact(
    repository_root: &Path,
    output_path: &Path,
    artifact: &EvaluationAggregateArtifact,
) -> Result<(), EvaluationReportHostError> {
    let (_, repository) = open_root(
        repository_root,
        EvaluationReportHostError::RepositoryRootInvalid,
    )?;
    let Some(relative) = safe_relative_text(output_path) else {
        return Err(EvaluationReportHostError::OutputPathInvalid);
    };
    let path = Path::new(relative);
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    ensure_directory_path(&repository, parent)?;
    match repository.symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(EvaluationReportHostError::OutputPathInvalid);
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => return Err(EvaluationReportHostError::OutputPathInvalid),
    }
    let bytes = artifact.to_json()?;
    let (stage, mut output) = open_staged_output(&repository, path)?;
    if output
        .write_all(&bytes)
        .and_then(|()| output.sync_all())
        .is_err()
    {
        drop(output);
        let _ = repository.remove_file(&stage);
        return Err(EvaluationReportHostError::OutputWriteFailed);
    }
    drop(output);
    if repository.rename(&stage, &repository, path).is_err() {
        let _ = repository.remove_file(&stage);
        return Err(EvaluationReportHostError::OutputWriteFailed);
    }
    Ok(())
}

fn admit_samples(
    workspace_root: &Path,
    workspace: &Dir,
    report_path: &str,
    samples: &[DecodedEvaluationSample],
    envelopes: &mut Vec<EvaluationEnvelope>,
    failed_attempts: &mut Vec<String>,
    diagnostics: &mut Vec<String>,
) {
    for sample in samples {
        match sample {
            DecodedEvaluationSample::Failed { diagnostic, .. } => {
                failed_attempts.push(diagnostic.clone());
            }
            DecodedEvaluationSample::Retained {
                work_dir,
                transcript_path,
                transcript_digest,
                envelope,
            } => match verify_transcript(
                workspace_root,
                workspace,
                work_dir,
                transcript_path,
                transcript_digest,
            ) {
                Ok(()) => envelopes.push((**envelope).clone()),
                Err(code) => diagnostics.push(format!(
                    "{report_path}:{}:{}:{code}",
                    envelope.host.as_str(),
                    envelope.scenario.as_str()
                )),
            },
        }
    }
}

fn verify_transcript(
    workspace_root: &Path,
    workspace: &Dir,
    reported_work_dir: &str,
    transcript_path: &str,
    expected_digest: &str,
) -> Result<(), &'static str> {
    let work_dir = Path::new(reported_work_dir);
    if !work_dir.is_absolute()
        || work_dir
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err("transcript-workdir-invalid");
    }
    let relative_work_dir = work_dir
        .strip_prefix(workspace_root)
        .map_err(|_| "transcript-workdir-escape")?;
    require_directory_path(workspace, relative_work_dir)
        .map_err(|_| "transcript-workdir-invalid")?;
    let work_directory = workspace
        .open_dir(relative_work_dir)
        .map_err(|_| "transcript-workdir-invalid")?;
    let bytes = read_rooted_file(
        &work_directory,
        Path::new(transcript_path),
        MAX_TRANSCRIPT_BYTES,
    )?;
    if sha256_hex(&bytes) == expected_digest {
        Ok(())
    } else {
        Err("transcript-digest-mismatch")
    }
}

fn open_root(
    root: &Path,
    error: EvaluationReportHostError,
) -> Result<(PathBuf, Dir), EvaluationReportHostError> {
    let metadata = std::fs::symlink_metadata(root).map_err(|_| match error {
        EvaluationReportHostError::RepositoryRootInvalid => {
            EvaluationReportHostError::RepositoryRootInvalid
        }
        _ => EvaluationReportHostError::WorkspaceRootInvalid,
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(error);
    }
    let canonical = std::fs::canonicalize(root).map_err(|_| match error {
        EvaluationReportHostError::RepositoryRootInvalid => {
            EvaluationReportHostError::RepositoryRootInvalid
        }
        _ => EvaluationReportHostError::WorkspaceRootInvalid,
    })?;
    let directory = Dir::open_ambient_dir(&canonical, ambient_authority()).map_err(|_| error)?;
    Ok((canonical, directory))
}

fn safe_relative_text(path: &Path) -> Option<&str> {
    let value = path.to_str()?;
    if value.is_empty()
        || value.starts_with('/')
        || value.contains('\\')
        || value.bytes().any(|byte| byte.is_ascii_control())
        || value
            .as_bytes()
            .get(..2)
            .is_some_and(|prefix| prefix[0].is_ascii_alphabetic() && prefix[1] == b':')
        || !value
            .split('/')
            .all(|part| !part.is_empty() && !matches!(part, "." | ".."))
    {
        None
    } else {
        Some(value)
    }
}

fn read_rooted_file(root: &Dir, relative: &Path, maximum: usize) -> Result<Vec<u8>, &'static str> {
    require_file_path(root, relative)?;
    let file = root.open(relative).map_err(|_| "file-unreadable")?;
    let metadata = file.metadata().map_err(|_| "file-unreadable")?;
    if !metadata.is_file() {
        return Err("file-kind-invalid");
    }
    if metadata.len() > u64::try_from(maximum).unwrap_or(u64::MAX) {
        return Err("file-too-large");
    }
    let limit = u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    file.take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| "file-unreadable")?;
    if bytes.len() > maximum {
        Err("file-too-large")
    } else {
        Ok(bytes)
    }
}

fn require_file_path(root: &Dir, relative: &Path) -> Result<(), &'static str> {
    let components = normal_components(relative)?;
    if components.is_empty() {
        return Err("file-path-invalid");
    }
    let mut current = PathBuf::new();
    for (index, component) in components.iter().enumerate() {
        current.push(component);
        let metadata = root
            .symlink_metadata(&current)
            .map_err(|_| "file-missing")?;
        if metadata.file_type().is_symlink() {
            return Err("file-linked");
        }
        if index + 1 == components.len() {
            if !metadata.is_file() {
                return Err("file-kind-invalid");
            }
        } else if !metadata.is_dir() {
            return Err("file-path-invalid");
        }
    }
    Ok(())
}

fn require_directory_path(root: &Dir, relative: &Path) -> Result<(), &'static str> {
    let components = normal_components(relative)?;
    let mut current = PathBuf::new();
    for component in components {
        current.push(component);
        let metadata = root
            .symlink_metadata(&current)
            .map_err(|_| "directory-missing")?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("directory-invalid");
        }
    }
    Ok(())
}

fn ensure_directory_path(root: &Dir, relative: &Path) -> Result<(), EvaluationReportHostError> {
    let components =
        normal_components(relative).map_err(|_| EvaluationReportHostError::OutputPathInvalid)?;
    let mut current = PathBuf::new();
    let mut missing = false;
    for component in components {
        current.push(component);
        if missing {
            continue;
        }
        match root.symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err(EvaluationReportHostError::OutputPathInvalid);
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => missing = true,
            Err(_) => return Err(EvaluationReportHostError::OutputPathInvalid),
        }
    }
    root.create_dir_all(relative)
        .map_err(|_| EvaluationReportHostError::OutputWriteFailed)?;
    require_directory_path(root, relative).map_err(|_| EvaluationReportHostError::OutputPathInvalid)
}

fn normal_components(relative: &Path) -> Result<Vec<&std::ffi::OsStr>, &'static str> {
    relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value),
            _ => Err("path-invalid"),
        })
        .collect()
}

fn open_staged_output(
    repository: &Dir,
    path: &Path,
) -> Result<(PathBuf, cap_std::fs::File), EvaluationReportHostError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    for attempt in 0..64 {
        let stage = staged_output_path(path, attempt);
        match repository.open_with(&stage, &options) {
            Ok(output) => return Ok((stage, output)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(EvaluationReportHostError::OutputWriteFailed),
        }
    }
    Err(EvaluationReportHostError::OutputWriteFailed)
}

fn staged_output_path(path: &Path, attempt: usize) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("evaluation-aggregate.json");
    parent.join(format!(".{name}.ea-stage-{}-{attempt}", std::process::id()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use ix_trace_rs::trace;
    use serde_json::{Value, json};
    use tempfile::TempDir;

    use super::*;

    const SOURCE_REVISION: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn identity(name: &str) -> Value {
        json!({"name": name, "version": "1.2.3", "digest": "a".repeat(64)})
    }

    fn report(work_dir: &Path, digest: &str, model: Option<&str>) -> Value {
        let governing = json!({
            "module": identity("engineering-assurance"),
            "plugin": identity("engineering-assurance-plugin"),
            "skill": identity("assurance-onboarding"),
            "workflow": identity("assurance-intake"),
            "quire": identity("quire"),
            "quoin": identity("quoin"),
            "ix_flow": identity("ix-flow"),
            "schema": identity("evaluation-envelope"),
            "producer": identity("cli-agent-evals")
        });
        let observation = json!({
            "host": "codex",
            "host_version": "1.2.3",
            "source_revision": SOURCE_REVISION,
            "suite_revision": "suite-v1",
            "fixture_revision": "fixtures-v1",
            "governing": governing,
            "command_count": 1,
            "elapsed_ms": 1,
            "human_prompt_count": 0,
            "manual_translation_count": 0,
            "repeated_prompt_count": 0,
            "observed_outcome": "reused",
            "terminal_event": null,
            "unsupported_additions": []
        });
        let sample = json!({
            "ok": true,
            "latencyMs": 1,
            "exitReason": "complete",
            "metricStatus": "available",
            "tokenUsage": {
                "input": 1,
                "output": 1,
                "cacheCreation": 0,
                "cacheRead": 0,
                "contextInput": 1,
                "total": 2
            },
            "toolCalls": 0,
            "toolBreakdown": {},
            "classified": {},
            "checks": {"evaluation_result": observation},
            "failures": [],
            "workDir": work_dir,
            "sessionId": "session-1",
            "transcriptDigest": digest,
            "transcriptRetention": "retained",
            "transcriptPath": ".cli-agent-evals/transcripts/sample.transcript"
        });
        let result = json!({
            "id": "EA-001",
            "useCase": "existing-profile",
            "ok": true,
            "passRate": "1/1",
            "aggregate": {"latencyMs": {"p50": 1, "p95": 1}},
            "runs": [sample]
        });
        let mut report = json!({
            "reportVersion": "cli-agent-evals.report/v1",
            "ok": true,
            "generatedAt": "2026-09-10T00:00:00Z",
            "suite": "engineering-assurance-onboarding",
            "agent": "codex",
            "model": model,
            "repeats": 1,
            "results": [result],
            "aggregates": {"successRate": "1/1"}
        });
        if model.is_none() {
            report
                .as_object_mut()
                .expect("report fixture must be an object")
                .remove("model");
        }
        report
    }

    fn fixture(model: Option<&str>) -> (TempDir, TempDir, PathBuf, PathBuf) {
        let repository = TempDir::new().expect("repository fixture must be creatable");
        let workspace = TempDir::new().expect("workspace fixture must be creatable");
        let work_dir = workspace.path().join("codex-existing-profile");
        let transcript = work_dir.join(".cli-agent-evals/transcripts/sample.transcript");
        fs::create_dir_all(transcript.parent().expect("transcript must have a parent"))
            .expect("transcript parent must be creatable");
        fs::write(&transcript, b"retained transcript\n")
            .expect("transcript fixture must be writable");
        let report_path = repository.path().join("reports/codex.json");
        fs::create_dir_all(report_path.parent().expect("report must have a parent"))
            .expect("report parent must be creatable");
        fs::write(
            &report_path,
            serde_json::to_vec(&report(
                &work_dir,
                &sha256_hex(b"retained transcript\n"),
                model,
            ))
            .expect("report fixture must encode"),
        )
        .expect("report fixture must be writable");
        (repository, workspace, report_path, transcript)
    }

    fn execute_fixture(
        repository: &TempDir,
        workspace: &TempDir,
        paths: &[PathBuf],
    ) -> EvaluationAggregateArtifact {
        execute(
            repository.path(),
            workspace.path(),
            paths,
            SOURCE_REVISION,
            "2026-09-10T00:00:00Z".to_owned(),
        )
        .expect("fixture roots must be accepted")
    }

    #[test]
    #[trace("TC-129", "FR-017-AC-1", "FR-017-CON-1", "FR-017-CON-3")]
    fn tc_129_verifies_exact_transcript_bytes_beneath_the_workspace_root() {
        let (repository, workspace, _, _) = fixture(None);
        let artifact = execute_fixture(
            &repository,
            &workspace,
            &[PathBuf::from("reports/codex.json")],
        );

        assert_eq!(artifact.complete_cells, 1);
        assert_eq!(
            artifact.models.get("codex"),
            Some(&"runner-default".to_owned())
        );
        assert_eq!(artifact.reports.len(), 1);
        assert!(!artifact.ok);
        assert!(
            artifact
                .failures
                .iter()
                .any(|value| value == "missing:claude:existing-profile")
        );
        assert!(
            !artifact
                .failures
                .iter()
                .any(|value| value.contains("transcript-"))
        );
    }

    #[test]
    #[trace("TC-129", "FR-017-AC-1", "FR-017-CON-3")]
    fn tc_129_withholds_changed_missing_linked_and_non_regular_transcripts() {
        let (repository, workspace, _, transcript) = fixture(Some("model-a"));
        fs::write(&transcript, b"changed").expect("transcript must be mutable");
        let changed = execute_fixture(
            &repository,
            &workspace,
            &[PathBuf::from("reports/codex.json")],
        );
        assert_eq!(changed.complete_cells, 0);
        assert!(
            changed
                .failures
                .iter()
                .any(|value| value.ends_with("transcript-digest-mismatch"))
        );

        fs::remove_file(&transcript).expect("transcript must be removable");
        let missing = execute_fixture(
            &repository,
            &workspace,
            &[PathBuf::from("reports/codex.json")],
        );
        assert!(
            missing
                .failures
                .iter()
                .any(|value| value.ends_with("file-missing"))
        );

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("outside", &transcript)
                .expect("transcript link must be creatable");
            let linked = execute_fixture(
                &repository,
                &workspace,
                &[PathBuf::from("reports/codex.json")],
            );
            assert!(
                linked
                    .failures
                    .iter()
                    .any(|value| value.ends_with("file-linked"))
            );
            fs::remove_file(&transcript).expect("transcript link must be removable");

            fs::create_dir(&transcript).expect("non-regular transcript fixture must be creatable");
            let non_regular = execute_fixture(
                &repository,
                &workspace,
                &[PathBuf::from("reports/codex.json")],
            );
            assert!(
                non_regular
                    .failures
                    .iter()
                    .any(|value| value.ends_with("file-kind-invalid"))
            );
        }
    }

    #[test]
    #[trace("TC-129", "FR-017-AC-1", "FR-017-CON-3")]
    fn tc_129_rejects_report_and_workspace_link_or_escape_paths() {
        let (repository, workspace, report_path, _) = fixture(Some("model-a"));
        let escaped = execute_fixture(&repository, &workspace, &[PathBuf::from("../outside.json")]);
        assert_eq!(escaped.complete_cells, 0);
        assert!(
            escaped
                .failures
                .iter()
                .any(|value| value == "report-path-invalid")
        );

        #[cfg(unix)]
        {
            let linked_report = repository.path().join("reports/linked.json");
            std::os::unix::fs::symlink(&report_path, &linked_report)
                .expect("report link must be creatable");
            let linked = execute_fixture(
                &repository,
                &workspace,
                &[PathBuf::from("reports/linked.json")],
            );
            assert!(
                linked
                    .failures
                    .iter()
                    .any(|value| value.ends_with("file-linked"))
            );
        }

        let outside = TempDir::new().expect("outside fixture must be creatable");
        let payload = fs::read(&report_path).expect("report fixture must be readable");
        let mut value: Value =
            serde_json::from_slice(&payload).expect("report fixture must decode");
        value["results"][0]["runs"][0]["workDir"] = json!(outside.path());
        fs::write(
            &report_path,
            serde_json::to_vec(&value).expect("report must encode"),
        )
        .expect("report must be writable");
        let workdir_escape = execute_fixture(
            &repository,
            &workspace,
            &[PathBuf::from("reports/codex.json")],
        );
        assert!(
            workdir_escape
                .failures
                .iter()
                .any(|value| value.ends_with("transcript-workdir-escape"))
        );
    }

    #[test]
    #[trace("TC-129", "FR-017-AC-1", "FR-017-CON-3")]
    fn tc_129_bounds_collection_and_transcript_bytes_before_allocation() {
        let (repository, workspace, _, transcript) = fixture(Some("model-a"));
        let exact_paths = vec![PathBuf::from("reports/codex.json"); MAX_REPORT_COLLECTION];
        execute(
            repository.path(),
            workspace.path(),
            &exact_paths,
            SOURCE_REVISION,
            "2026-09-10T00:00:00Z".to_owned(),
        )
        .expect("the exact report collection ceiling must be accepted");
        let paths = vec![PathBuf::from("reports/codex.json"); MAX_REPORT_COLLECTION + 1];
        let error = execute(
            repository.path(),
            workspace.path(),
            &paths,
            SOURCE_REVISION,
            "2026-09-10T00:00:00Z".to_owned(),
        )
        .expect_err("one report above the collection ceiling must refuse");
        assert_eq!(error.code(), "evaluation_report_collection_too_large");

        fs::OpenOptions::new()
            .write(true)
            .open(transcript)
            .expect("transcript fixture must open")
            .set_len(u64::try_from(MAX_TRANSCRIPT_BYTES).expect("limit must fit u64") + 1)
            .expect("sparse oversized transcript must be creatable");
        let oversized = execute_fixture(
            &repository,
            &workspace,
            &[PathBuf::from("reports/codex.json")],
        );
        assert!(
            oversized
                .failures
                .iter()
                .any(|value| value.ends_with("file-too-large"))
        );

        let boundary = repository.path().join("reports/boundary.bin");
        fs::write(&boundary, b"1234").expect("boundary fixture must be writable");
        let root = Dir::open_ambient_dir(repository.path(), ambient_authority())
            .expect("fixture root must open");
        assert_eq!(
            read_rooted_file(&root, Path::new("reports/boundary.bin"), 4)
                .expect("exact byte ceiling must read"),
            b"1234"
        );
        assert_eq!(
            read_rooted_file(&root, Path::new("reports/boundary.bin"), 3),
            Err("file-too-large")
        );
    }

    #[test]
    #[trace("TC-129", "FR-017-AC-1", "FR-017-CON-3")]
    fn tc_129_orders_equivalent_collections_and_rejects_model_drift() {
        let (repository, workspace, report_path, _) = fixture(Some("model-a"));
        let second_path = repository.path().join("reports/second.json");
        fs::copy(&report_path, &second_path).expect("second report must be creatable");
        let forward = execute_fixture(
            &repository,
            &workspace,
            &[
                PathBuf::from("reports/codex.json"),
                PathBuf::from("reports/second.json"),
            ],
        );
        let reverse = execute_fixture(
            &repository,
            &workspace,
            &[
                PathBuf::from("reports/second.json"),
                PathBuf::from("reports/codex.json"),
            ],
        );
        assert_eq!(forward, reverse);

        let mut value: Value = serde_json::from_slice(
            &fs::read(&second_path).expect("second report must be readable"),
        )
        .expect("second report must decode");
        value["model"] = json!("model-b");
        fs::write(
            &second_path,
            serde_json::to_vec(&value).expect("report must encode"),
        )
        .expect("report must be writable");
        let drifted = execute_fixture(
            &repository,
            &workspace,
            &[
                PathBuf::from("reports/codex.json"),
                PathBuf::from("reports/second.json"),
            ],
        );
        assert!(
            drifted
                .failures
                .iter()
                .any(|value| value == "codex:model-mismatch:model-a,model-b")
        );
        assert!(!drifted.models.contains_key("codex"));
    }

    #[test]
    #[trace("TC-129", "FR-017-AC-1", "FR-017-CON-3")]
    fn tc_129_writes_inside_the_repository_without_following_output_links() {
        let (repository, workspace, _, _) = fixture(Some("model-a"));
        let artifact = execute_fixture(
            &repository,
            &workspace,
            &[PathBuf::from("reports/codex.json")],
        );
        write_artifact(
            repository.path(),
            Path::new("artifacts/aggregate.json"),
            &artifact,
        )
        .expect("safe output must be writable");
        let encoded = fs::read(repository.path().join("artifacts/aggregate.json"))
            .expect("aggregate output must be readable");
        let observed: Value = serde_json::from_slice(&encoded).expect("aggregate must decode");
        assert_eq!(observed["revision"], json!(ARTIFACT_REVISION));

        let stale = repository
            .path()
            .join(staged_output_path(Path::new("artifacts/aggregate.json"), 0));
        fs::write(&stale, b"stale prior stage").expect("stale stage must be creatable");
        write_artifact(
            repository.path(),
            Path::new("artifacts/aggregate.json"),
            &artifact,
        )
        .expect("one stale stage must not block a later atomic write");
        assert_eq!(
            fs::read(stale).expect("unowned stale stage must remain untouched"),
            b"stale prior stage"
        );

        #[cfg(unix)]
        {
            let outside = TempDir::new().expect("outside fixture must be creatable");
            std::os::unix::fs::symlink(outside.path(), repository.path().join("linked-output"))
                .expect("output link must be creatable");
            let error = write_artifact(
                repository.path(),
                Path::new("linked-output/aggregate.json"),
                &artifact,
            )
            .expect_err("linked output parent must refuse");
            assert_eq!(error.code(), "evaluation_report_output_path_invalid");
        }
    }
}
