// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded host observation for the reviewed compatibility matrix.
//!
//! This is intentionally separate from `engineering_assurance::compatibility`:
//! it validates the selected root and invokes the four explicitly declared
//! version commands, then hands only typed observations to the pure classifier.

use std::{
    ffi::OsStr,
    io::Read as _,
    path::{Path, PathBuf},
    time::Duration,
};

use engineering_assurance::compatibility::{
    CompatibilityError, CompatibilityRequest, CompatibilityResult, ComponentObservation,
    REQUEST_PROTOCOL, evaluate_request_bytes,
};
use serde::Serialize;
use thiserror::Error;

use crate::process_host::{self, ProcessLimits};

const OBSERVATION_PROTOCOL: &str = "engineering-assurance.compatibility-observation/v1";
const OBSERVATION_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_OBSERVATION_OUTPUT_BYTES: usize = 64 * 1024;

/// The running binary's own version.
///
/// The `engineering-assurance` row observes this rather than `--root`'s git
/// tag: a consumer who installed the CLI from a tag and runs it in their own
/// project would otherwise get either no observation (untagged project) or
/// their project's tag reported as this tool's version. A pass therefore
/// describes the installed executable, not the checkout `--root` points at.
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Machine result for one environment observation and pure classification.
#[derive(Debug, Serialize)]
pub(crate) struct CompatibilityObservationResult {
    protocol: &'static str,
    classification: CompatibilityResult,
    module: ModuleObservation,
    pub(crate) gate_satisfied: bool,
}

/// The installed `engineering-assurance` Quoin module, compared with this binary.
///
/// The matrix pins toolchain versions; it does not see the module Quoin loaded
/// its schemas and skeletons from. A CLI at one release beside a module from
/// another validates documents against schemas the CLI was not built for, so
/// the pair must agree before the gate opens.
#[derive(Debug, Serialize)]
struct ModuleObservation {
    /// Version the installed module's `manifest.yaml` declares, or `null` when
    /// no module was found or its manifest could not be read.
    installed: Option<String>,
    /// Version of this binary.
    cli: &'static str,
    /// Whether the installed module version equals this binary's version.
    matches_cli: bool,
}

/// Stable failures at the impure observer boundary.
#[derive(Debug, Error)]
pub(crate) enum CompatibilityObservationError {
    #[error("compatibility-observe root is not a directory")]
    RootInvalid,
    #[error(transparent)]
    Classification(#[from] CompatibilityError),
    #[error("compatibility-observe result serialization failed: {0}")]
    ResultSerialization(serde_json::Error),
}

impl CompatibilityObservationError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::RootInvalid => "compatibility_observe_root_invalid",
            Self::Classification(error) => error.code(),
            Self::ResultSerialization(_) => "compatibility_observe_result_serialization_failed",
        }
    }
}

/// Observe the declared tools at `root`, then classify them against the matrix.
pub(crate) fn observe(
    root: &Path,
) -> Result<CompatibilityObservationResult, CompatibilityObservationError> {
    observe_with(root, &SystemToolRunner, installed_module_version())
}

fn observe_with(
    root: &Path,
    runner: &dyn ToolRunner,
    module_version: Option<String>,
) -> Result<CompatibilityObservationResult, CompatibilityObservationError> {
    if !root.is_dir() {
        return Err(CompatibilityObservationError::RootInvalid);
    }
    let observed = vec![
        ComponentObservation {
            component: "quire-cli".to_owned(),
            version: observe_quire(runner),
        },
        ComponentObservation {
            component: "quoin".to_owned(),
            version: observe_semver(runner, "quoin", &["--version"]),
        },
        ComponentObservation {
            component: "ix-flow".to_owned(),
            version: observe_semver(runner, "ix-flow", &["--version"]),
        },
        ComponentObservation {
            component: "engineering-assurance".to_owned(),
            version: Some(CLI_VERSION.to_owned()),
        },
    ];
    let input = serde_json::to_vec(&CompatibilityRequest {
        protocol: REQUEST_PROTOCOL.to_owned(),
        observed,
    })
    .map_err(CompatibilityObservationError::ResultSerialization)?;
    let classification = evaluate_request_bytes(&input)?;
    let module = ModuleObservation {
        matches_cli: module_version.as_deref() == Some(CLI_VERSION),
        installed: module_version,
        cli: CLI_VERSION,
    };
    let gate_satisfied = classification.gate_satisfied && module.matches_cli;
    Ok(CompatibilityObservationResult {
        protocol: OBSERVATION_PROTOCOL,
        classification,
        module,
        gate_satisfied,
    })
}

pub(crate) fn to_json_line(
    result: &CompatibilityObservationResult,
) -> Result<Vec<u8>, CompatibilityObservationError> {
    let mut encoded =
        serde_json::to_vec(result).map_err(CompatibilityObservationError::ResultSerialization)?;
    encoded.push(b'\n');
    Ok(encoded)
}

fn observe_quire(runner: &dyn ToolRunner) -> Option<String> {
    let output = runner.run(OsStr::new("quire"), &[OsStr::new("provenance")])?;
    let value: serde_json::Value = serde_json::from_slice(&output).ok()?;
    value
        .get("cli")?
        .get("version")?
        .as_str()
        .filter(|version| !version.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn observe_semver(runner: &dyn ToolRunner, command: &str, arguments: &[&str]) -> Option<String> {
    let arguments = arguments
        .iter()
        .map(|argument| OsStr::new(*argument))
        .collect::<Vec<_>>();
    let output = runner.run(OsStr::new(command), &arguments)?;
    let text = std::str::from_utf8(&output).ok()?;
    let version = regex::Regex::new(r"\b(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)\b")
        .expect("literal semver extractor must compile")
        .captures(text)?
        .get(1)?
        .as_str();
    Some(version.to_owned())
}

/// Where Quoin installs modules: `IX_CONFIG_ROOT`, else `~/.ix`.
fn module_manifest_path() -> Option<PathBuf> {
    let config_root = std::env::var_os("IX_CONFIG_ROOT")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".ix")))?;
    Some(
        config_root
            .join("filament")
            .join("modules")
            .join("engineering-assurance")
            .join("manifest.yaml"),
    )
}

/// Version declared by the installed module's manifest, `None` when absent,
/// oversized, or unparseable (which the gate treats as not matching).
fn installed_module_version() -> Option<String> {
    let path = module_manifest_path()?;
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(MAX_OBSERVATION_OUTPUT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > MAX_OBSERVATION_OUTPUT_BYTES {
        return None;
    }
    module_version_from_manifest(&bytes)
}

fn module_version_from_manifest(bytes: &[u8]) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct Manifest {
        version: String,
    }
    let manifest: Manifest = yaml_serde::from_slice(bytes).ok()?;
    Some(manifest.version).filter(|version| !version.trim().is_empty())
}

trait ToolRunner {
    fn run(&self, command: &OsStr, arguments: &[&OsStr]) -> Option<Vec<u8>>;
}

struct SystemToolRunner;

impl ToolRunner for SystemToolRunner {
    fn run(&self, command: &OsStr, arguments: &[&OsStr]) -> Option<Vec<u8>> {
        let completed = process_host::run(
            command,
            arguments,
            ProcessLimits {
                timeout: OBSERVATION_TIMEOUT,
                max_output_bytes: MAX_OBSERVATION_OUTPUT_BYTES,
            },
        )
        .ok()?;
        completed.status.success().then_some(completed.stdout)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use ix_trace_rs::trace;

    struct FixtureRunner {
        outputs: BTreeMap<(String, Vec<String>), Vec<u8>>,
    }

    impl ToolRunner for FixtureRunner {
        fn run(&self, command: &OsStr, arguments: &[&OsStr]) -> Option<Vec<u8>> {
            let key = (
                command.to_string_lossy().into_owned(),
                arguments
                    .iter()
                    .map(|argument| argument.to_string_lossy().into_owned())
                    .collect(),
            );
            self.outputs.get(&key).cloned()
        }
    }

    /// A runner reporting every external tool at exactly its reviewed pin.
    fn fixture_runner() -> FixtureRunner {
        FixtureRunner {
            outputs: BTreeMap::from([
                (
                    ("quire".to_owned(), vec!["provenance".to_owned()]),
                    br#"{"cli":{"version":"0.33.0"}}"#.to_vec(),
                ),
                (
                    ("quoin".to_owned(), vec!["--version".to_owned()]),
                    b"quoin 0.24.1\n".to_vec(),
                ),
                (
                    ("ix-flow".to_owned(), vec!["--version".to_owned()]),
                    b"0.2.3\n".to_vec(),
                ),
            ]),
        }
    }

    #[test]
    #[trace("TC-130", "FR-012-AC-10", "FR-012-CON-1")]
    fn observer_keeps_a_missing_binary_as_an_unobserved_component() {
        assert_eq!(
            observe_semver(
                &FixtureRunner {
                    outputs: BTreeMap::new(),
                },
                "engineering-assurance-no-such-tool",
                &["--version"]
            ),
            None
        );
    }

    #[test]
    #[trace("TC-130", "FR-012-AC-10", "FR-012-CON-1")]
    fn the_observing_program_withholds_the_gate_when_nothing_was_observed() {
        // Nothing in this test file previously drove `observe_with` to a
        // withheld gate: every other test here pins the fixture runner to the
        // matrix's exact reviewed versions. An empty runner leaves every
        // component unobserved, so the pure classifier must call every one of
        // them `unknown` and withhold, and the host adapter must report that
        // verdict rather than opening the gate on its own.
        let result = observe_with(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            &FixtureRunner {
                outputs: BTreeMap::new(),
            },
            None,
        )
        .expect("an observation with no tool output must still classify");
        assert!(!result.classification.versions_compatible);
        assert!(!result.gate_satisfied);

        let encoded = to_json_line(&result).expect("the result must serialize");
        let emitted: serde_json::Value =
            serde_json::from_slice(&encoded).expect("the emitted line must be one JSON value");
        assert_eq!(
            emitted["gate_satisfied"], emitted["classification"]["gate_satisfied"],
            "the observation result disagreed with the classifier it delegates to"
        );
    }

    #[test]
    #[trace("TC-130", "FR-012-AC-10", "FR-014-AC-2")]
    fn fixture_observer_separates_host_observation_from_pure_accepted_gate() {
        let result = observe_with(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            &fixture_runner(),
            Some(env!("CARGO_PKG_VERSION").to_owned()),
        )
        .expect("fully pinned fixture observation must classify");
        assert!(result.classification.versions_compatible);
        assert!(result.gate_satisfied);
    }

    #[test]
    #[trace("TC-130", "FR-012-AC-10", "FR-012-CON-1")]
    fn the_engineering_assurance_row_is_the_binary_version_wherever_root_points() {
        // `--root` is a directory with no git history at all. The row must still
        // be observed, and as the running binary's version rather than as
        // anything read from the directory.
        let root = std::env::temp_dir();
        let result = observe_with(
            &root,
            &FixtureRunner {
                outputs: BTreeMap::new(),
            },
            None,
        )
        .expect("an observation with no tool output must still classify");
        let own = result
            .classification
            .components
            .iter()
            .find(|item| item.component == "engineering-assurance")
            .expect("the matrix must classify engineering-assurance");
        assert_eq!(own.observed.as_deref(), Some(env!("CARGO_PKG_VERSION")));
    }

    #[test]
    #[trace("TC-130", "FR-012-AC-10", "FR-014-AC-2")]
    fn a_module_that_disagrees_with_the_binary_withholds_an_otherwise_open_gate() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        // Every external tool is pinned, so the classifier alone opens the gate.
        for module in [Some("0.3.1".to_owned()), None] {
            let result = observe_with(root, &fixture_runner(), module.clone())
                .expect("fully pinned fixture observation must classify");
            assert!(
                result.classification.gate_satisfied,
                "the classifier no longer opens the gate, so this test proves nothing"
            );
            assert!(!result.module.matches_cli, "{module:?} matched the binary");
            assert!(!result.gate_satisfied, "{module:?} opened the gate");
            assert_eq!(result.module.installed, module);
        }
    }

    #[test]
    #[trace("TC-130", "FR-012-AC-10")]
    fn the_module_version_is_the_manifest_version() {
        assert_eq!(
            module_version_from_manifest(b"manifest_version: 1.0.0\nname: x\nversion: 0.3.1\n"),
            Some("0.3.1".to_owned())
        );
        assert_eq!(module_version_from_manifest(b"name: x\n"), None);
        assert_eq!(module_version_from_manifest(b"version: '  '\n"), None);
        assert_eq!(module_version_from_manifest(b"\x00not yaml: ["), None);
    }

    #[test]
    #[trace("TC-130", "FR-012-AC-10", "FR-014-AC-2")]
    fn the_emitted_result_delegates_its_verdict_to_the_pure_classifier() {
        // A reader of the JSON line has to see the same gate verdict the pure
        // classifier reached, without the host adapter layering any judgment of
        // its own on top of it (PLAT-973: this observer no longer verifies
        // working-tree artifacts against the matrix's informational digest
        // record, so the classifier's own verdict is the whole answer).
        let result = observe_with(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            &fixture_runner(),
            Some(env!("CARGO_PKG_VERSION").to_owned()),
        )
        .expect("fully pinned fixture observation must classify");
        let encoded = to_json_line(&result).expect("the result must serialize");
        let emitted: serde_json::Value =
            serde_json::from_slice(&encoded).expect("the emitted line must be one JSON value");
        assert_eq!(
            emitted["gate_satisfied"], emitted["classification"]["gate_satisfied"],
            "the observation result disagreed with the classifier it delegates to"
        );
        assert_eq!(emitted["module"]["matches_cli"], serde_json::json!(true));
        assert_eq!(emitted["protocol"], serde_json::json!(OBSERVATION_PROTOCOL));
    }
}
