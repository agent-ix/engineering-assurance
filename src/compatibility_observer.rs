// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded host observation for the reviewed compatibility matrix.
//!
//! This is intentionally separate from `engineering_assurance::compatibility`:
//! it reads the selected tree and invokes the four explicitly declared version
//! commands, then hands only typed observations to the pure classifier.

use std::{ffi::OsStr, path::Path, time::Duration};

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

/// Machine result for one environment observation and pure classification.
#[derive(Debug, Serialize)]
pub(crate) struct CompatibilityObservationResult {
    protocol: &'static str,
    classification: CompatibilityResult,
    pub(crate) gate_satisfied: bool,
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
    observe_with(root, &SystemToolRunner)
}

fn observe_with(
    root: &Path,
    runner: &dyn ToolRunner,
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
            version: observe_self(root, runner),
        },
    ];
    let input = serde_json::to_vec(&CompatibilityRequest {
        protocol: REQUEST_PROTOCOL.to_owned(),
        observed,
    })
    .map_err(CompatibilityObservationError::ResultSerialization)?;
    let classification = evaluate_request_bytes(&input)?;
    let gate_satisfied = classification.gate_satisfied;
    Ok(CompatibilityObservationResult {
        protocol: OBSERVATION_PROTOCOL,
        classification,
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

fn observe_self(root: &Path, runner: &dyn ToolRunner) -> Option<String> {
    let root = root.as_os_str();
    let output = runner.run(
        OsStr::new("git"),
        &[
            OsStr::new("-C"),
            root,
            OsStr::new("describe"),
            OsStr::new("--tags"),
            OsStr::new("--abbrev=0"),
        ],
    )?;
    let tag = std::str::from_utf8(&output).ok()?.trim();
    tag.strip_prefix('v')
        .filter(|version| !version.is_empty())
        .map(ToOwned::to_owned)
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

    /// A runner reporting every declared tool at exactly its reviewed pin,
    /// observing this crate's own repository root.
    fn fixture_runner() -> FixtureRunner {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        FixtureRunner {
            outputs: BTreeMap::from([
                (
                    ("quire".to_owned(), vec!["provenance".to_owned()]),
                    br#"{"cli":{"version":"0.31.0"}}"#.to_vec(),
                ),
                (
                    ("quoin".to_owned(), vec!["--version".to_owned()]),
                    b"quoin 0.23.1\n".to_vec(),
                ),
                (
                    ("ix-flow".to_owned(), vec!["--version".to_owned()]),
                    b"0.2.3\n".to_vec(),
                ),
                (
                    (
                        "git".to_owned(),
                        vec![
                            "-C".to_owned(),
                            root.to_string_lossy().into_owned(),
                            "describe".to_owned(),
                            "--tags".to_owned(),
                            "--abbrev=0".to_owned(),
                        ],
                    ),
                    b"v0.3.1\n".to_vec(),
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
    #[trace("TC-130", "FR-012-AC-10", "FR-014-AC-2")]
    fn fixture_observer_separates_host_observation_from_pure_accepted_gate() {
        let result = observe_with(Path::new(env!("CARGO_MANIFEST_DIR")), &fixture_runner())
            .expect("fully pinned fixture observation must classify");
        assert!(result.classification.versions_compatible);
        assert!(result.gate_satisfied);
    }

    #[test]
    #[trace("TC-130", "FR-012-AC-10", "FR-014-AC-2")]
    fn the_emitted_result_delegates_its_verdict_to_the_pure_classifier() {
        // A reader of the JSON line has to see the same gate verdict the pure
        // classifier reached, without the host adapter layering any judgment of
        // its own on top of it (PLAT-973: this observer no longer verifies
        // working-tree artifacts against the matrix's informational digest
        // record, so the classifier's own verdict is the whole answer).
        let result = observe_with(Path::new(env!("CARGO_MANIFEST_DIR")), &fixture_runner())
            .expect("fully pinned fixture observation must classify");
        let encoded = to_json_line(&result).expect("the result must serialize");
        let emitted: serde_json::Value =
            serde_json::from_slice(&encoded).expect("the emitted line must be one JSON value");
        assert_eq!(
            emitted["gate_satisfied"], emitted["classification"]["gate_satisfied"],
            "the observation result disagreed with the classifier it delegates to"
        );
        assert_eq!(emitted["protocol"], serde_json::json!(OBSERVATION_PROTOCOL));
    }
}
