// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust host adapter for the release qualification evidence boundary.
//!
//! It owns bounded Quire invocation, typed coverage decoding, and the
//! governing tool versions handed to the agent-evals provider. The
//! evaluation-report adapter remains the sole owner of report and transcript
//! decoding.

use std::{
    collections::BTreeMap,
    env,
    ffi::{OsStr, OsString},
    fmt, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use engineering_assurance::evidence::VersionIdentity;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::process_host::{self, ProcessError, ProcessLimits};

const COVERAGE_TIMEOUT: Duration = Duration::from_secs(60);
const VERSION_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_MATRIX_BYTES: u64 = 8 * 1024 * 1024;
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
    #[error("quire coverage returned a document that could not be decoded: {0}")]
    CoverageUnparseable(String),
    #[error("{0}")]
    CoverageRefused(CoverageRefusals),
    #[error("repository test matrix is invalid")]
    MatrixInvalid,
    #[error("governing version document is unavailable or invalid")]
    GoverningVersionInvalid,
    #[error("governing executable is unavailable or invalid")]
    GoverningExecutableInvalid,
    #[error("governing runtime package is unavailable or invalid")]
    GoverningRuntimeInvalid,
}

impl IntegrationEvidenceError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::RootInvalid => "integration_evidence_root_invalid",
            Self::QuireUnavailable => "integration_evidence_quire_unavailable",
            Self::QuireTimedOut => "integration_evidence_quire_timed_out",
            Self::QuireOutputTooLarge => "integration_evidence_quire_output_too_large",
            Self::QuireObservationFailed => "integration_evidence_quire_observation_failed",
            Self::QuireFailed => "integration_evidence_quire_failed",
            Self::CoverageUnparseable(_) => "integration_evidence_coverage_unparseable",
            Self::CoverageRefused(refusals) => refusals.code(),
            Self::MatrixInvalid => "integration_evidence_matrix_invalid",
            Self::GoverningVersionInvalid => "integration_evidence_governing_version_invalid",
            Self::GoverningExecutableInvalid => "integration_evidence_governing_executable_invalid",
            Self::GoverningRuntimeInvalid => "integration_evidence_governing_runtime_invalid",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct IntegrationEvidence;

/// One failing coverage condition, with its own code and the authored
/// locations that caused it.
///
/// The shared error envelope carries a single `code`, so a refusal set reports
/// the first condition in declaration order as the result code while every
/// condition still appears, named, in the message. Collapsing them into one
/// code is what made a repository gap indistinguishable from a coverage-tool
/// malfunction (FR-017-AC-10).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoverageRefusal {
    code: &'static str,
    detail: String,
}

impl fmt::Display for CoverageRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

/// Every failing coverage condition, in fixed severity order and never empty.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CoverageRefusals(Vec<CoverageRefusal>);

impl CoverageRefusals {
    fn code(&self) -> &'static str {
        self.0
            .first()
            .map_or("integration_evidence_coverage_refused", |refusal| {
                refusal.code
            })
    }
}

impl fmt::Display for CoverageRefusals {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "integration-evidence refused the repository coverage"
        )?;
        for refusal in &self.0 {
            write!(formatter, "\n{refusal}")?;
        }
        Ok(())
    }
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
    diagnostics: Vec<CoverageDiagnostic>,
}

#[derive(Debug, Deserialize)]
struct CoverageTotals {
    backed: usize,
    total: usize,
}

/// One offending coverage row.
///
/// Quire authors `document`, `row_id`, `line`, `path`, `symbol`, and
/// `trace_id` on its findings; decoding only `path` is what left the host
/// unable to name a single offending row (FR-017-AC-10).
#[derive(Clone, Debug, Default, Deserialize)]
struct CoverageReference {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    document: Option<String>,
    #[serde(default)]
    row_id: Option<String>,
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    trace_id: Option<String>,
    #[serde(default)]
    line: Option<u64>,
}

impl CoverageReference {
    /// A trace tag inside a declared submodule belongs to that submodule's own
    /// repository, not to Engineering Assurance. Vendored dependencies carry their
    /// own. A reference with no
    /// path is treated as first-party so the gate fails closed.
    /// `document:line row_id -> trace_id`, using whichever locators the
    /// finding carries. An error about files must say where in those files the
    /// error is, so a finding with no locator at all still names itself.
    fn locate(&self) -> String {
        let where_ = self.document.as_deref().or(self.path.as_deref());
        let mut rendered = match (where_, self.line) {
            (Some(place), Some(line)) => format!("{place}:{line}"),
            (Some(place), None) => place.to_owned(),
            (None, Some(line)) => format!("<unlocated>:{line}"),
            (None, None) => "<unlocated>".to_owned(),
        };
        if let Some(subject) = self.row_id.as_deref().or(self.symbol.as_deref()) {
            rendered.push(' ');
            rendered.push_str(subject);
        }
        if let Some(trace_id) = self.trace_id.as_deref() {
            rendered.push_str(" -> ");
            rendered.push_str(trace_id);
        }
        rendered
    }

    fn is_first_party(&self, submodules: &[String]) -> bool {
        let Some(path) = self.path.as_deref() else {
            return true;
        };
        let path = path.trim_start_matches("./");
        !submodules
            .iter()
            .any(|prefix| path == prefix || path.starts_with(&format!("{prefix}/")))
    }
}

impl CoverageDiagnostic {
    fn locate(&self) -> String {
        let reason = self.reason.as_deref().unwrap_or("<unnamed reason>");
        let place = match (self.path.as_deref(), self.line) {
            (Some(path), Some(line)) => format!("{path}:{line}"),
            (Some(path), None) => path.to_owned(),
            (None, _) => "<unlocated>".to_owned(),
        };
        match self.declaration.as_deref() {
            Some(declaration) => format!("{place} {reason} (declaration `{declaration}`)"),
            None => format!("{place} {reason}"),
        }
    }
}

/// Submodule directories declared by the repository root `.gitmodules`.
fn submodule_paths(root: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(root.join(".gitmodules")) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| line.trim().strip_prefix("path"))
        .filter_map(|rest| rest.trim().strip_prefix('='))
        .map(|value| value.trim().trim_end_matches('/').to_owned())
        .filter(|value| !value.is_empty())
        .collect()
}

#[derive(Debug, Default, Deserialize)]
struct CoverageDiagnostic {
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    line: Option<u64>,
    #[serde(default)]
    declaration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VersionDocument {
    version: String,
}

#[derive(Serialize)]
pub(crate) struct AgentEvalsSnapshot {
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
            version_identity(name, yaml_version(&definition)?),
        );
    }
    Ok(AgentEvalsSnapshot {
        host: executable_identity(agent, &path)?,
        governing: AgentEvalsGoverning {
            module: version_identity("engineering-assurance", module_version),
            plugin: version_identity("engineering-assurance-plugin", plugin_version.clone()),
            skill: version_identity("assurance-onboarding", plugin_version),
            quire: executable_identity("quire", &path)?,
            quoin: executable_identity("quoin", &path)?,
            ix_flow: runtime_identity("ix-flow", &path)?,
            schema: version_identity(
                "evaluation-result-contract",
                "evaluation-result-v1".to_owned(),
            ),
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
    Ok(IntegrationEvidence)
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
    serde_json::from_slice(&output.stdout)
        .map_err(|error| IntegrationEvidenceError::CoverageUnparseable(error.to_string()))
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

/// Diagnostic reasons that make the census itself untrustworthy.
///
/// `hollow-denominator`, `marker-form-mismatch`, and `section-matches-nothing`
/// each mean a population was counted wrongly or not at all, so a complete
/// result computed over it asserts more than was measured.
/// `status-column-matches-nothing` is the same failure in its most dangerous
/// form: the tool reports that status classification was *skipped*, after
/// which `status_lies` is empty because nothing ran, not because the rows are
/// honest. Accepting that emptiness certified an unmeasured property
/// (FR-017-AC-10).
const CENSUS_FATAL_REASONS: [&str; 4] = [
    "hollow-denominator",
    "marker-form-mismatch",
    "section-matches-nothing",
    "status-column-matches-nothing",
];

/// Render at most `LOCATION_SAMPLE` locations, then say how many remain, so a
/// large gap stays readable without hiding its size.
const LOCATION_SAMPLE: usize = 20;

fn render_locations(locations: &[String]) -> String {
    let shown = locations
        .iter()
        .take(LOCATION_SAMPLE)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n    ");
    if locations.len() > LOCATION_SAMPLE {
        format!(
            "{shown}\n    ... and {} more",
            locations.len() - LOCATION_SAMPLE
        )
    } else {
        shown
    }
}

/// A refusal naming every row in `references`, or `None` when there are none.
fn located_refusal(
    code: &'static str,
    summary: &str,
    references: &[CoverageReference],
) -> Option<CoverageRefusal> {
    if references.is_empty() {
        return None;
    }
    let locations = references
        .iter()
        .map(CoverageReference::locate)
        .collect::<Vec<_>>();
    Some(CoverageRefusal {
        code,
        detail: format!(
            "{} {summary}:\n    {}",
            locations.len(),
            render_locations(&locations)
        ),
    })
}

/// A partial census means every count below it was measured over a population
/// the tool could not fully classify, so it is reported first.
fn census_refusal(coverage: &CoverageDocument, root: &Path) -> Option<CoverageRefusal> {
    let locations = coverage
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic
                .reason
                .as_deref()
                .is_some_and(|reason| CENSUS_FATAL_REASONS.contains(&reason))
                && diagnostic
                    .path
                    .as_deref()
                    .is_none_or(|path| is_local_diagnostic(path, root))
        })
        .map(CoverageDiagnostic::locate)
        .collect::<Vec<_>>();
    if locations.is_empty() {
        return None;
    }
    Some(CoverageRefusal {
        code: "integration_evidence_coverage_census_partial",
        detail: format!(
            "repository traceability census is partial, so the counts below were measured \
             over an incomplete population:\n    {}",
            render_locations(&locations)
        ),
    })
}

fn totals_refusal(totals: &CoverageTotals) -> Option<CoverageRefusal> {
    if totals.total == 0 {
        return Some(CoverageRefusal {
            code: "integration_evidence_coverage_population_empty",
            detail: "traceability totals report no population at all".to_owned(),
        });
    }
    if totals.backed == totals.total {
        return None;
    }
    Some(CoverageRefusal {
        code: "integration_evidence_coverage_incomplete",
        detail: format!(
            "traceability is {}/{}, expected complete backing",
            totals.backed, totals.total
        ),
    })
}

/// Collect every failing condition, in fixed severity order.
///
/// Each condition is evaluated independently: stopping at the first one is
/// what made a multi-cause failure look like a single defect (FR-017-AC-10).
fn validate_coverage(
    coverage: &CoverageDocument,
    root: &Path,
) -> Result<(), IntegrationEvidenceError> {
    let submodules = submodule_paths(root);
    let orphaned = coverage
        .untracked_symbols
        .iter()
        .filter(|symbol| symbol.is_first_party(&submodules))
        .cloned()
        .collect::<Vec<_>>();

    let refusals = [
        census_refusal(coverage, root),
        totals_refusal(&coverage.totals),
        located_refusal(
            "integration_evidence_coverage_unbacked_rows",
            "row(s) name no backing test",
            &coverage.unbacked_rows,
        ),
        located_refusal(
            "integration_evidence_coverage_status_lies",
            "row(s) claim a status their backing does not support",
            &coverage.status_lies,
        ),
        located_refusal(
            "integration_evidence_coverage_untracked_symbols",
            "first-party trace tag(s) bind to nothing the matrix minted",
            &orphaned,
        ),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();

    if refusals.is_empty() {
        Ok(())
    } else {
        Err(IntegrationEvidenceError::CoverageRefused(CoverageRefusals(
            refusals,
        )))
    }
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

fn yaml_version(path: &Path) -> Result<String, IntegrationEvidenceError> {
    let bytes = fs::read(path).map_err(|_| IntegrationEvidenceError::GoverningVersionInvalid)?;
    let document: VersionDocument = yaml_serde::from_slice(&bytes)
        .map_err(|_| IntegrationEvidenceError::GoverningVersionInvalid)?;
    if document.version.is_empty() {
        return Err(IntegrationEvidenceError::GoverningVersionInvalid);
    }
    Ok(document.version)
}

fn json_version(path: &Path) -> Result<String, IntegrationEvidenceError> {
    #[derive(Deserialize)]
    struct PackageVersion {
        version: String,
    }
    let bytes = fs::read(path).map_err(|_| IntegrationEvidenceError::GoverningVersionInvalid)?;
    let document: PackageVersion = serde_json::from_slice(&bytes)
        .map_err(|_| IntegrationEvidenceError::GoverningVersionInvalid)?;
    if document.version.is_empty() {
        return Err(IntegrationEvidenceError::GoverningVersionInvalid);
    }
    Ok(document.version)
}

fn version_identity(name: &str, version: String) -> VersionIdentity {
    VersionIdentity {
        name: name.to_owned(),
        version,
    }
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
    Ok(version_identity(name, version))
}

fn runtime_identity(name: &str, path: &OsStr) -> Result<VersionIdentity, IntegrationEvidenceError> {
    let executable =
        find_executable(name, path).ok_or(IntegrationEvidenceError::GoverningRuntimeInvalid)?;
    let version =
        command_version(&executable).ok_or(IntegrationEvidenceError::GoverningRuntimeInvalid)?;
    Ok(version_identity(name, version))
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

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[test]
    #[trace("TC-111", "FR-017-AC-3")]
    fn tc_111_untracked_symbols_inside_declared_submodules_are_not_local_gaps() {
        let root = tempfile::tempdir().expect("fixture root");
        std::fs::write(
            root.path().join(".gitmodules"),
            "[submodule \"vendor/ix-trace-rs\"]\n\tpath = vendor/ix-trace-rs\n\
             \turl = ../ix-trace-rs.git\n",
        )
        .expect("submodule declaration must be writable");
        let document = |paths: &[&str]| CoverageDocument {
            totals: CoverageTotals {
                backed: 92,
                total: 92,
            },
            unbacked_rows: vec![],
            status_lies: vec![],
            untracked_symbols: paths
                .iter()
                .map(|path| CoverageReference {
                    path: Some((*path).to_owned()),
                    ..CoverageReference::default()
                })
                .collect(),
            diagnostics: vec![],
        };

        // Fixtures inside a declared submodule are that submodule's business.
        assert!(
            validate_coverage(&document(&["vendor/ix-trace-rs/src/lib.rs",]), root.path(),).is_ok()
        );

        // A first-party untracked tag is still a gate failure, and it names
        // the condition rather than a generic invalid document.
        let orphan = validate_coverage(&document(&["src/evaluation.rs"]), root.path())
            .expect_err("a first-party untracked tag must refuse");
        assert_eq!(
            orphan.code(),
            "integration_evidence_coverage_untracked_symbols"
        );
        assert!(orphan.to_string().contains("src/evaluation.rs"));

        // A path-less reference fails closed.
        assert_eq!(
            validate_coverage(
                &CoverageDocument {
                    untracked_symbols: vec![CoverageReference::default()],
                    ..document(&[])
                },
                root.path(),
            )
            .expect_err("a path-less reference must fail closed")
            .code(),
            "integration_evidence_coverage_untracked_symbols"
        );

        // A directory that merely shares a prefix with a submodule is first-party.
        assert_eq!(
            validate_coverage(
                &document(&["vendor/ix-trace-rs-tools/src/lib.rs"]),
                root.path()
            )
            .expect_err("a prefix-sharing directory must refuse")
            .code(),
            "integration_evidence_coverage_untracked_symbols"
        );
    }

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
        let refusal = validate_coverage(&incomplete, root.path())
            .expect_err("incomplete backing must refuse");
        assert_eq!(refusal.code(), "integration_evidence_coverage_incomplete");
        assert!(refusal.to_string().contains("91/92"));
    }

    fn complete_coverage() -> CoverageDocument {
        CoverageDocument {
            totals: CoverageTotals {
                backed: 92,
                total: 92,
            },
            unbacked_rows: vec![],
            status_lies: vec![],
            untracked_symbols: vec![],
            diagnostics: vec![],
        }
    }

    fn matrix_row(row_id: &str, line: u64) -> CoverageReference {
        CoverageReference {
            document: Some("spec/tests.md".to_owned()),
            row_id: Some(row_id.to_owned()),
            line: Some(line),
            ..CoverageReference::default()
        }
    }

    #[test]
    #[trace("TC-137", "FR-017-AC-10")]
    fn tc_137_each_failing_coverage_condition_has_its_own_code_and_location() {
        let root = tempfile::tempdir().expect("fixture root");
        assert!(validate_coverage(&complete_coverage(), root.path()).is_ok());

        // A skipped status classification is a refusal in its own right. This
        // is the regression that mattered most: `status_lies` is empty because
        // nothing ran, and the gate used to read that emptiness as honesty.
        let skipped = CoverageDocument {
            diagnostics: vec![CoverageDiagnostic {
                reason: Some("status-column-matches-nothing".to_owned()),
                path: Some("spec/tests.md".to_owned()),
                line: Some(67),
                declaration: Some("functional-coverage".to_owned()),
            }],
            ..complete_coverage()
        };
        let refusal = validate_coverage(&skipped, root.path())
            .expect_err("a skipped status classification must refuse");
        assert_eq!(
            refusal.code(),
            "integration_evidence_coverage_census_partial"
        );
        let rendered = refusal.to_string();
        assert!(
            rendered.contains("spec/tests.md:67"),
            "a census refusal must name the authored location, got: {rendered}"
        );
        assert!(rendered.contains("functional-coverage"));

        // A status lie is its own condition, distinct from an unbacked row.
        let lie = CoverageDocument {
            status_lies: vec![matrix_row("FR-001", 69)],
            ..complete_coverage()
        };
        let refusal = validate_coverage(&lie, root.path()).expect_err("a status lie must refuse");
        assert_eq!(refusal.code(), "integration_evidence_coverage_status_lies");
        assert!(refusal.to_string().contains("spec/tests.md:69 FR-001"));
    }

    #[test]
    #[trace("TC-137", "FR-017-AC-10")]
    fn tc_137_all_failing_conditions_are_reported_without_blaming_the_coverage_tool() {
        let root = tempfile::tempdir().expect("fixture root");
        let unbacked = CoverageDocument {
            totals: CoverageTotals {
                backed: 91,
                total: 92,
            },
            unbacked_rows: vec![matrix_row("FR-016", 123)],
            ..complete_coverage()
        };
        let refusal =
            validate_coverage(&unbacked, root.path()).expect_err("an unbacked row must refuse");
        let rendered = refusal.to_string();

        assert!(
            rendered.contains("spec/tests.md:123 FR-016"),
            "an unbacked row must be located, got: {rendered}"
        );
        // Both failing conditions appear, each under its own code.
        assert!(rendered.contains("integration_evidence_coverage_incomplete"));
        assert!(rendered.contains("integration_evidence_coverage_unbacked_rows"));
        assert_eq!(
            refusal.code(),
            "integration_evidence_coverage_incomplete",
            "the result code is the first condition in severity order"
        );

        // A repository gap is never described as a coverage-tool failure.
        assert_ne!(
            refusal.code(),
            "integration_evidence_coverage_unparseable",
            "a repository gap must not reuse the decoding code"
        );
        assert!(
            !rendered.contains("invalid document"),
            "a repository gap must not be described as a tool malfunction"
        );
    }
}
