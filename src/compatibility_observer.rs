// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded host observation for the reviewed compatibility matrix.
//!
//! This is intentionally separate from `engineering_assurance::compatibility`:
//! it reads the selected tree and invokes the four explicitly declared version
//! commands, then hands only typed observations to the pure classifier.

use std::{ffi::OsStr, fmt::Write as _, fs, path::Path, time::Duration};

use engineering_assurance::compatibility::{
    CompatibilityError, CompatibilityRequest, CompatibilityResult, ComponentObservation,
    REQUEST_PROTOCOL, evaluate_request_bytes, recorded_artifact_digests,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
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
    artifacts_recorded: usize,
    artifacts_hashed: usize,
    artifact_absences: Vec<String>,
    artifact_mismatches: Vec<String>,
    artifact_digests_match: bool,
    pub(crate) gate_satisfied: bool,
}

/// What hashing the recorded artifact population against one tree established.
///
/// The counts travel beside the failure lists because a reader cannot otherwise
/// tell an empty `mismatches` that means "every recorded artifact reproduced its
/// digest" from an empty `mismatches` that means "no recorded artifact was ever
/// opened". Those are the same two bytes on the wire and opposite facts about
/// the tree.
#[derive(Debug, Default)]
struct ArtifactObservation {
    recorded: usize,
    hashed: usize,
    absences: Vec<String>,
    mismatches: Vec<String>,
}

impl ArtifactObservation {
    /// Whether this tree really is the artifact population the matrix records.
    ///
    /// Three conditions have to hold together. Every recorded artifact must
    /// have been found, because an artifact the observer never opened is one it
    /// never checked, and the reviewed matrix already rules that what could not
    /// be observed is unknown rather than compatible. Every artifact it did open
    /// must have reproduced its recorded digest. And the matrix must record at
    /// least one artifact, because a population of zero satisfies the first two
    /// conditions vacuously and would let a matrix that records nothing vouch
    /// for a tree nobody looked at.
    ///
    /// The hashed-against-recorded equality is deliberately kept alongside the
    /// emptiness of `absences`, even though the two cannot disagree today. It is
    /// the assertion that survives somebody later adding a continue, a filter,
    /// or an early return to the hashing loop: the counter would then disagree
    /// with the population and this answer would turn false, where an `absences`
    /// list that was never appended to would not.
    const fn population_verified(&self) -> bool {
        self.recorded > 0
            && self.hashed == self.recorded
            && self.absences.is_empty()
            && self.mismatches.is_empty()
    }
}

/// Stable failures at the impure observer boundary.
#[derive(Debug, Error)]
pub(crate) enum CompatibilityObservationError {
    #[error("compatibility-observe root is not a directory")]
    RootInvalid,
    #[error(transparent)]
    Classification(#[from] CompatibilityError),
    #[error("cannot read recorded artifact digest input")]
    ArtifactUnreadable,
    #[error("the reviewed matrix records no artifact to verify")]
    ArtifactPopulationEmpty,
    #[error("compatibility-observe result serialization failed: {0}")]
    ResultSerialization(serde_json::Error),
}

impl CompatibilityObservationError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::RootInvalid => "compatibility_observe_root_invalid",
            Self::Classification(error) => error.code(),
            Self::ArtifactUnreadable => "compatibility_observe_artifact_unreadable",
            Self::ArtifactPopulationEmpty => "compatibility_observe_artifact_population_empty",
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
    let artifacts = observe_artifacts(root)?;
    let artifact_digests_match = artifacts.population_verified();
    let gate_satisfied = classification.gate_satisfied && artifact_digests_match;
    Ok(CompatibilityObservationResult {
        protocol: OBSERVATION_PROTOCOL,
        classification,
        artifacts_recorded: artifacts.recorded,
        artifacts_hashed: artifacts.hashed,
        artifact_absences: artifacts.absences,
        artifact_mismatches: artifacts.mismatches,
        artifact_digests_match,
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

/// Hash every artifact the reviewed matrix records against one selected tree.
///
/// An artifact the tree does not contain is recorded as an absence rather than
/// passed over. The earlier reading — that a recorded path this tree does not
/// contain is somebody else's tree, and so not this observer's business —
/// answered the wrong question. It is true that an installed-package tree or a
/// partial checkout is not the reviewed tree, but the conclusion that follows
/// is not "then there is nothing to report": it is that this observer is being
/// pointed at a tree the matrix does not describe and therefore cannot vouch
/// for. Skipping turned that into silence, and silence here is indistinguishable
/// from every artifact having reproduced its digest, which is how
/// `compatibility-observe` came to exit 0 over a directory holding one empty
/// commit. Absence is reported separately from drift so an operator can still
/// tell the wrong tree from a tampered file; both withhold the gate.
fn observe_artifacts(root: &Path) -> Result<ArtifactObservation, CompatibilityObservationError> {
    let recorded = recorded_artifact_digests()?;
    if recorded.is_empty() {
        return Err(CompatibilityObservationError::ArtifactPopulationEmpty);
    }
    let mut observation = ArtifactObservation {
        recorded: recorded.len(),
        ..ArtifactObservation::default()
    };
    for artifact in recorded {
        let path = root.join(&artifact.path);
        if !path.is_file() {
            observation.absences.push(format!(
                "{}: not present in the observed tree",
                artifact.path
            ));
            continue;
        }
        let bytes =
            fs::read(path).map_err(|_| CompatibilityObservationError::ArtifactUnreadable)?;
        observation.hashed += 1;
        let actual = hex_digest(&bytes);
        if actual != artifact.sha256 {
            observation.mismatches.push(format!(
                "{}: {actual}, matrix records {}",
                artifact.path, artifact.sha256
            ));
        }
    }
    Ok(observation)
}

/// Lowercase hexadecimal SHA-256 of `bytes`.
fn hex_digest(bytes: &[u8]) -> String {
    let mut hex = String::with_capacity(64);
    for byte in Sha256::digest(bytes) {
        write!(&mut hex, "{byte:02x}").expect("writing into String cannot fail");
    }
    hex
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

    fn fixture_runner() -> FixtureRunner {
        runner_reporting_pinned_tools_for(Path::new(env!("CARGO_MANIFEST_DIR")))
    }

    /// A runner reporting every declared tool at exactly its reviewed pin.
    ///
    /// The observed root is a parameter because the self-observation runs
    /// `git -C <root> describe`, so a fixture keyed on one fixed directory can
    /// only ever answer for that directory. A test that points the observer at
    /// some other tree needs the version half of the answer to stay compatible,
    /// otherwise the classifier withholds the gate on its own and the test can
    /// no longer show what the artifact population decided.
    fn runner_reporting_pinned_tools_for(root: &Path) -> FixtureRunner {
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
                    b"v0.2.1\n".to_vec(),
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
        assert!(result.artifact_digests_match);
        assert!(result.gate_satisfied);
    }

    #[test]
    #[trace("TC-083", "FR-012-AC-5")]
    fn matrix_artifact_digests_match_the_repository_root() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let observation = observe_artifacts(root).expect("repository artifacts must be readable");
        assert!(
            observation.absences.is_empty(),
            "recorded artifacts are absent from this tree: {:?}",
            observation.absences
        );
        assert!(
            observation.mismatches.is_empty(),
            "recorded artifacts drifted from their digests: {:?}",
            observation.mismatches
        );

        // An empty mismatch list is also what a matrix recording nothing, or a
        // tree containing none of the recorded paths, produces. The counts are
        // asserted against each other and against the ten schema assets so the
        // clean result above cannot be a clean result over nothing.
        assert_eq!(observation.hashed, observation.recorded);
        assert!(
            observation.recorded >= 10,
            "the digest check examined too few artifacts: {}",
            observation.recorded
        );
        assert!(observation.population_verified());
    }

    #[test]
    #[trace("TC-083", "FR-012-AC-5", "FR-012-AC-10")]
    fn a_tree_missing_the_recorded_artifacts_reports_every_absence() {
        let empty = tempfile::tempdir().expect("a temporary directory must be creatable");
        let observation =
            observe_artifacts(empty.path()).expect("an empty tree must still be observable");

        // Every recorded artifact has to appear as an absence, not as silence.
        // This is the assertion the shipping observer lacked: before it, an
        // empty tree and a fully verified tree produced byte-identical evidence.
        assert_eq!(observation.hashed, 0);
        assert_eq!(observation.absences.len(), observation.recorded);
        assert!(observation.mismatches.is_empty());
        assert!(
            !observation.population_verified(),
            "an artifact population nothing hashed must never read as verified"
        );
    }

    #[test]
    #[trace("TC-130", "FR-012-AC-5", "FR-012-AC-10")]
    fn the_observing_program_withholds_the_gate_over_a_tree_it_did_not_verify() {
        // This drives `observe_with`, the function the CLI calls, rather than
        // the artifact helper alone. The fixture runner reports every declared
        // tool at its exact pin, so the classifier's own verdict is satisfied
        // and the artifact population is the only thing left that can withhold
        // the gate. Against a tree holding none of the recorded artifacts it
        // must withhold: the defect this test exists to prevent was the
        // production path reporting `gate_satisfied` while it had hashed zero
        // of the recorded artifacts.
        let empty = tempfile::tempdir().expect("a temporary directory must be creatable");
        let runner = runner_reporting_pinned_tools_for(empty.path());
        let result = observe_with(empty.path(), &runner)
            .expect("a fully pinned toolchain over an empty tree must still classify");

        assert!(result.classification.gate_satisfied);
        assert!(!result.artifact_digests_match);
        assert!(!result.gate_satisfied);
        assert_eq!(result.artifacts_hashed, 0);
        assert_eq!(result.artifact_absences.len(), result.artifacts_recorded);
    }

    #[test]
    #[trace("TC-083", "FR-012-AC-5")]
    fn every_condition_of_a_verified_population_is_independently_necessary() {
        // Each conjunct is driven on its own. Asserting only that an empty tree
        // withholds the gate would leave any single condition removable without
        // a test noticing, because the remaining ones still answer false for
        // that one tree — which is how the production path came to have no
        // floor at all while a unit test appeared to defend one.
        let verified = ArtifactObservation {
            recorded: 11,
            hashed: 11,
            absences: Vec::new(),
            mismatches: Vec::new(),
        };
        assert!(verified.population_verified());

        // A matrix recording nothing must not vouch for a tree. Zero hashed of
        // zero recorded satisfies every other condition vacuously.
        assert!(
            !ArtifactObservation::default().population_verified(),
            "an empty recorded population must never read as verified"
        );

        // Fewer artifacts hashed than recorded means some recorded artifact was
        // never opened, whatever the failure lists say.
        assert!(
            !ArtifactObservation {
                hashed: 10,
                ..verified_like()
            }
            .population_verified()
        );

        // A reported absence withholds even when the counts were not updated.
        assert!(
            !ArtifactObservation {
                absences: vec!["schemas/one.json: not present".to_owned()],
                ..verified_like()
            }
            .population_verified()
        );

        // Digest drift withholds on its own terms; it is the condition that
        // already worked and it must keep working.
        assert!(
            !ArtifactObservation {
                mismatches: vec!["schemas/one.json: drifted".to_owned()],
                ..verified_like()
            }
            .population_verified()
        );
    }

    /// A fully verified observation, for tests that spoil exactly one condition.
    fn verified_like() -> ArtifactObservation {
        ArtifactObservation {
            recorded: 11,
            hashed: 11,
            absences: Vec::new(),
            mismatches: Vec::new(),
        }
    }

    #[test]
    #[trace("TC-130", "FR-012-AC-10", "FR-014-AC-2")]
    fn the_emitted_result_publishes_what_the_observation_actually_examined() {
        // A reader of the JSON line has to be able to tell a verified population
        // from an unexamined one without re-deriving it, so the counts are part
        // of the machine result rather than a local variable.
        let result = observe_with(Path::new(env!("CARGO_MANIFEST_DIR")), &fixture_runner())
            .expect("fully pinned fixture observation must classify");
        let encoded = to_json_line(&result).expect("the result must serialize");
        let emitted: serde_json::Value =
            serde_json::from_slice(&encoded).expect("the emitted line must be one JSON value");
        assert_eq!(
            emitted["artifacts_hashed"], emitted["artifacts_recorded"],
            "the emitted result claims a verified population it did not hash"
        );
        assert!(
            emitted["artifacts_recorded"]
                .as_u64()
                .is_some_and(|recorded| recorded >= 10)
        );
        assert_eq!(emitted["artifact_absences"], serde_json::json!([]));
    }
}
