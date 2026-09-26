// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Public contract tests for bounded producer execution.

#![cfg(target_os = "linux")]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use engineering_assurance::producer_execution::{
    ArgumentBinding, CancellationBinding, CancellationToken, ContainmentBinding, ContentDigest,
    ContractBinding, ExecutionBudget, ExecutionFailure, ExecutionProcedure, ExecutionRefusal,
    ExitCodeBinding, InputBinding, InvalidExecutionRequest, MalformedResponse, ObservedHostContext,
    OutputArtifact, OutputBinding, OutputTreeBinding, PRODUCER_EXECUTION_REQUEST_PROTOCOL,
    PRODUCER_EXECUTION_RESULT_PROTOCOL, ProcessEvidence, ProducerDescriptor,
    ProducerExecutionRequest, ProducerExecutionResult, ProducerExecutionState, ProducerExecutor,
    ProducerResponseAdapter, REQUEST_IDENTITY_SCHEME, RESULT_IDENTITY_SCHEME, ResponseBinding,
    StdinBinding, TerminalStatus,
};
use ix_trace_rs::trace;
use serde::{Serialize, Serializer};

#[derive(Clone)]
struct TextAdapter {
    binding: ResponseBinding,
    reject: bool,
}

impl ProducerResponseAdapter for TextAdapter {
    type Observation = String;

    fn binding(&self) -> &ResponseBinding {
        &self.binding
    }

    fn decode(
        &self,
        process: &ProcessEvidence,
        _artifacts: &[OutputArtifact],
    ) -> Result<Self::Observation, MalformedResponse> {
        if self.reject {
            return Err(MalformedResponse);
        }
        String::from_utf8(process.stdout.bytes.clone()).map_err(|_| MalformedResponse)
    }
}

struct SnapshotAdapter {
    binding: ResponseBinding,
}

impl ProducerResponseAdapter for SnapshotAdapter {
    type Observation = Vec<u8>;

    fn binding(&self) -> &ResponseBinding {
        &self.binding
    }

    fn decode(
        &self,
        _process: &ProcessEvidence,
        artifacts: &[OutputArtifact],
    ) -> Result<Self::Observation, MalformedResponse> {
        let artifact = artifacts.first().ok_or(MalformedResponse)?;
        let mut bytes = Vec::new();
        artifact
            .try_reader()
            .and_then(|mut reader| reader.read_to_end(&mut bytes))
            .map_err(|_| MalformedResponse)?;
        Ok(bytes)
    }
}

#[derive(Clone, Debug)]
struct NonCanonicalObservation;

impl Serialize for NonCanonicalObservation {
    fn serialize<S: Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(
            "fixture observation refuses serialization",
        ))
    }
}

struct NonCanonicalAdapter {
    binding: ResponseBinding,
}

impl ProducerResponseAdapter for NonCanonicalAdapter {
    type Observation = NonCanonicalObservation;

    fn binding(&self) -> &ResponseBinding {
        &self.binding
    }

    fn decode(
        &self,
        _process: &ProcessEvidence,
        _artifacts: &[OutputArtifact],
    ) -> Result<Self::Observation, MalformedResponse> {
        Ok(NonCanonicalObservation)
    }
}

fn fixture_executable() -> PathBuf {
    fs::canonicalize(env!("CARGO_BIN_EXE_producer-execution-fixture"))
        .expect("Rust fixture executable must be available")
}

fn contract(kind: &str) -> ContractBinding {
    ContractBinding {
        kind: kind.to_owned(),
        version: "1".to_owned(),
        revision: "fixture-revision".to_owned(),
        digest: ContentDigest::of_bytes(kind.as_bytes()),
    }
}

fn binding() -> ResponseBinding {
    ResponseBinding {
        protocol: contract("fixture.response"),
        adapter: contract("fixture.adapter"),
        exit_codes: ExitCodeBinding::Exact(BTreeSet::from([0])),
    }
}

fn request(root: &Path, arguments: &[&str]) -> ProducerExecutionRequest {
    let executable = fixture_executable();
    ProducerExecutionRequest {
        protocol: PRODUCER_EXECUTION_REQUEST_PROTOCOL.to_owned(),
        producer: ProducerDescriptor {
            name: "rust-fixture-producer".to_owned(),
            version: "1.0.0".to_owned(),
            source_revision: "fixture-revision".to_owned(),
            executable: executable.display().to_string(),
            executable_digest: ContentDigest::of_file(&executable)
                .expect("fixture executable must be hashable"),
        },
        caller: contract("fixture.request"),
        procedure: ExecutionProcedure::Direct,
        capability_root: fs::canonicalize(root)
            .expect("fixture root must be canonical")
            .display()
            .to_string(),
        arguments: arguments
            .iter()
            .map(|value| ArgumentBinding::Literal {
                value: (*value).to_owned(),
            })
            .collect(),
        environment: BTreeMap::new(),
        inputs: Vec::new(),
        stdin: StdinBinding::Null,
        outputs: Vec::new(),
        output_trees: Vec::new(),
        containment: ContainmentBinding::ProcessGroupV1 {
            contract: contract("fixture.cooperative-confinement"),
        },
        cancellation: CancellationBinding::Disabled,
        budget: ExecutionBudget {
            timeout_millis: 2_000,
            max_stdout_bytes: 1_024,
            max_stderr_bytes: 1_024,
            max_input_bytes: 1_024,
            max_output_artifacts: 0,
            max_output_bytes: 0,
            max_descendants: 64,
            max_concurrency: 1,
        },
        response: binding(),
    }
}

/// Bound for tests whose fixture is held open across a test-driven
/// rendezvous (a ready marker, then a test action, then a release or a
/// cancellation). Those tests exercise input snapshots and cancellation, not the
/// timeout, so both the readiness wait and the request's own timeout use
/// a bound well past any realistic scheduling delay on a loaded, shared
/// runner. Under the default 2s budget the request could time out before
/// the test acted, failing for reasons unrelated to the behavior under
/// test (PLAT-996). The happy path never waits this long.
const RENDEZVOUS_MILLIS: u64 = 30_000;

/// A request whose fixture waits on the test: see [`RENDEZVOUS_MILLIS`].
fn rendezvous_request(root: &Path, arguments: &[&str]) -> ProducerExecutionRequest {
    let mut request = request(root, arguments);
    request.budget.timeout_millis = RENDEZVOUS_MILLIS;
    request
}

/// Blocks until the fixture run by `worker` creates `path`. Fails at once
/// if `worker` finishes without creating it (the fixture died or was
/// refused), so the generous [`RENDEZVOUS_MILLIS`] bound only applies to
/// a fixture that is still running but has not yet signalled.
fn await_ready<T>(path: &Path, worker: &JoinHandle<T>, message: &str) {
    let deadline = Instant::now() + Duration::from_millis(RENDEZVOUS_MILLIS);
    while !path.exists() {
        assert!(
            !worker.is_finished() || path.exists(),
            "{message}: worker finished without signalling ready"
        );
        assert!(Instant::now() < deadline, "{message}: timed out");
        thread::sleep(Duration::from_millis(1));
    }
}

fn adapter() -> TextAdapter {
    TextAdapter {
        binding: binding(),
        reject: false,
    }
}

fn execute(
    request: &ProducerExecutionRequest,
    adapter: &TextAdapter,
) -> ProducerExecutionResult<String> {
    ProducerExecutor::new(1)
        .expect("one execution slot must be valid")
        .execute(
            request,
            &CancellationToken::new(request.cancellation.clone()),
            adapter,
        )
        .expect("fixture request must be structurally valid")
}

/// Executes a just-written executable, retrying only while it is `Unavailable`.
///
/// A sibling test thread forking while our write descriptor is open makes
/// `execve` return ETXTBSY (rust-lang/rust#114554).
fn execute_staged(
    request: &ProducerExecutionRequest,
    adapter: &TextAdapter,
) -> ProducerExecutionState<String> {
    let mut state = execute(request, adapter).state;
    for _ in 1..20 {
        if !matches!(state, ProducerExecutionState::Unavailable) {
            break;
        }
        thread::sleep(Duration::from_millis(50));
        state = execute(request, adapter).state;
    }
    state
}

fn cargo_feature_tree(manifest_path: &Path, features: &[&str]) -> String {
    let feature_tree = Command::new(env!("CARGO"))
        .args([
            "tree",
            "--offline",
            "--edges",
            "features",
            "--invert",
            "serde_json",
            "--manifest-path",
        ])
        .arg(manifest_path)
        .args(features.iter().flat_map(|f| ["--features", f]))
        .output()
        .expect("feature tree must launch");
    assert!(
        feature_tree.status.success(),
        "feature tree failed: {}",
        String::from_utf8_lossy(&feature_tree.stderr)
    );
    String::from_utf8(feature_tree.stdout).expect("feature tree must be UTF-8")
}

fn assert_identity_changes(
    base: &ProducerExecutionRequest,
    mutate: impl FnOnce(&mut ProducerExecutionRequest),
) {
    let identity = base.identity().expect("base identity must be valid");
    let mut candidate = base.clone();
    mutate(&mut candidate);
    assert_ne!(
        candidate.identity().expect("mutation must remain valid"),
        identity
    );
}

fn identity_request(root: &Path) -> ProducerExecutionRequest {
    let mut request = request(root, &["emit", "a"]);
    request.environment.insert("B".to_owned(), "2".to_owned());
    request.environment.insert("A".to_owned(), "1".to_owned());
    request.inputs = vec![
        InputBinding {
            role: "input-1".to_owned(),
            path: "input-1".to_owned(),
            digest: ContentDigest::of_bytes(b"input-1"),
            executable: false,
        },
        InputBinding {
            role: "input-2".to_owned(),
            path: "input-2".to_owned(),
            digest: ContentDigest::of_bytes(b"input-2"),
            executable: false,
        },
    ];
    request.arguments.push(ArgumentBinding::InputArtifact {
        role: "input-1".to_owned(),
    });
    request.outputs = vec![
        OutputBinding {
            role: "output-1".to_owned(),
            path: "output-1".to_owned(),
            required: true,
        },
        OutputBinding {
            role: "output-2".to_owned(),
            path: "output-2".to_owned(),
            required: false,
        },
    ];
    request.arguments.push(ArgumentBinding::OutputArtifact {
        role: "output-1".to_owned(),
    });
    request.stdin = StdinBinding::InputArtifact {
        role: "input-1".to_owned(),
    };
    request.cancellation = CancellationBinding::Event {
        authority: "fixture".to_owned(),
        event_id: "event-1".to_owned(),
    };
    request.budget.max_output_artifacts = 2;
    request.budget.max_output_bytes = 8;
    request.response.exit_codes = ExitCodeBinding::Exact(BTreeSet::from([0, 7]));
    request
}

fn assert_descriptor_identity(base: &ProducerExecutionRequest) {
    assert_identity_changes(base, |value| value.producer.name.push('x'));
    assert_identity_changes(base, |value| value.producer.version.push('x'));
    assert_identity_changes(base, |value| value.producer.source_revision.push('x'));
    assert_identity_changes(base, |value| value.producer.executable.push('x'));
    assert_identity_changes(base, |value| {
        value.producer.executable_digest = ContentDigest::of_bytes(b"other executable");
    });
    assert_identity_changes(base, |value| value.caller.kind.push('x'));
    assert_identity_changes(base, |value| value.caller.version.push('x'));
    assert_identity_changes(base, |value| value.caller.revision.push('x'));
    assert_identity_changes(base, |value| {
        value.caller.digest = ContentDigest::of_bytes(b"other caller");
    });
    assert_identity_changes(base, |value| value.capability_root.push_str("/other"));
}

fn assert_io_identity(base: &ProducerExecutionRequest) {
    assert_identity_changes(base, |value| {
        value.arguments[0] = ArgumentBinding::Literal {
            value: "other".to_owned(),
        };
    });
    assert_identity_changes(base, |value| value.arguments.swap(0, 1));
    assert_identity_changes(base, |value| {
        value.environment.insert("A".to_owned(), "other".to_owned());
    });
    assert_identity_changes(base, |value| {
        value.environment.remove("A");
        value.environment.insert("C".to_owned(), "1".to_owned());
    });
    assert_identity_changes(base, |value| value.inputs[1].role.push('x'));
    assert_identity_changes(base, |value| value.inputs[0].path.push('x'));
    assert_identity_changes(base, |value| {
        value.inputs[0].digest = ContentDigest::of_bytes(b"other input");
    });
    assert_identity_changes(base, |value| value.inputs[0].executable = true);
    assert_identity_changes(base, |value| value.inputs.swap(0, 1));
    assert_identity_changes(base, |value| value.stdin = StdinBinding::Null);
    assert_identity_changes(base, |value| value.outputs[1].role.push('x'));
    assert_identity_changes(base, |value| value.outputs[0].path.push('x'));
    assert_identity_changes(base, |value| value.outputs[1].required = true);
    assert_identity_changes(base, |value| value.outputs.swap(0, 1));
}

fn assert_control_identity(base: &ProducerExecutionRequest) {
    assert_identity_changes(base, |value| match &mut value.containment {
        ContainmentBinding::ProcessGroupV1 { contract } => contract.kind.push('x'),
    });
    assert_identity_changes(base, |value| match &mut value.containment {
        ContainmentBinding::ProcessGroupV1 { contract } => contract.version.push('x'),
    });
    assert_identity_changes(base, |value| match &mut value.containment {
        ContainmentBinding::ProcessGroupV1 { contract } => contract.revision.push('x'),
    });
    assert_identity_changes(base, |value| match &mut value.containment {
        ContainmentBinding::ProcessGroupV1 { contract } => {
            contract.digest = ContentDigest::of_bytes(b"other confinement");
        }
    });
    assert_identity_changes(base, |value| match &mut value.cancellation {
        CancellationBinding::Disabled => unreachable!("fixture binds cancellation"),
        CancellationBinding::Event { authority, .. } => authority.push('x'),
    });
    assert_identity_changes(base, |value| match &mut value.cancellation {
        CancellationBinding::Disabled => unreachable!("fixture binds cancellation"),
        CancellationBinding::Event { event_id, .. } => event_id.push('x'),
    });
    assert_identity_changes(base, |value| value.budget.timeout_millis += 1);
    assert_identity_changes(base, |value| value.budget.max_stdout_bytes += 1);
    assert_identity_changes(base, |value| value.budget.max_stderr_bytes += 1);
    assert_identity_changes(base, |value| value.budget.max_input_bytes += 1);
    assert_identity_changes(base, |value| value.budget.max_output_artifacts += 1);
    assert_identity_changes(base, |value| value.budget.max_output_bytes += 1);
    assert_identity_changes(base, |value| value.budget.max_descendants += 1);
    assert_identity_changes(base, |value| value.budget.max_concurrency += 1);
}

fn assert_response_identity(base: &ProducerExecutionRequest) {
    assert_identity_changes(base, |value| value.response.protocol.kind.push('x'));
    assert_identity_changes(base, |value| value.response.protocol.version.push('x'));
    assert_identity_changes(base, |value| value.response.protocol.revision.push('x'));
    assert_identity_changes(base, |value| {
        value.response.protocol.digest = ContentDigest::of_bytes(b"other response");
    });
    assert_identity_changes(base, |value| value.response.adapter.kind.push('x'));
    assert_identity_changes(base, |value| value.response.adapter.version.push('x'));
    assert_identity_changes(base, |value| value.response.adapter.revision.push('x'));
    assert_identity_changes(base, |value| {
        value.response.adapter.digest = ContentDigest::of_bytes(b"other adapter");
    });
    assert_identity_changes(base, |value| {
        value.response.exit_codes = ExitCodeBinding::Exact(BTreeSet::from([0, 8]));
    });
}

fn assert_result_identity_changes(
    base: &ProducerExecutionResult<String>,
    mutate: impl FnOnce(&mut ProducerExecutionResult<String>),
) {
    let identity = base.identity().expect("base result identity");
    let mut candidate = base.clone();
    mutate(&mut candidate);
    assert_ne!(candidate.identity().expect("candidate identity"), identity);
}

#[test]
#[trace("TC-176", "FR-019-AC-4")]
fn tc_176_output_tree_binding_is_bounded_in_request_identity() {
    let root = tempfile::tempdir().expect("temporary root");
    let mut base = request(root.path(), &["emit"]);
    base.budget.max_output_artifacts = 2;
    base.budget.max_output_bytes = 1024;
    base.output_trees.push(OutputTreeBinding {
        role: "mutants".to_owned(),
        path: "mutants.out".to_owned(),
        required: true,
    });
    assert_identity_changes(&base, |value| {
        value.output_trees[0].path = "other.out".to_owned();
    });
    assert_identity_changes(&base, |value| value.output_trees[0].required = false);
    let mut overlapping = base.clone();
    overlapping.outputs.push(OutputBinding {
        role: "nested".to_owned(),
        path: "mutants.out/result.json".to_owned(),
        required: true,
    });
    assert_eq!(overlapping.identity(), Err(InvalidExecutionRequest::Output));
    let mut duplicate = base;
    duplicate.output_trees.push(OutputTreeBinding {
        role: "other".to_owned(),
        path: "mutants.out/nested".to_owned(),
        required: true,
    });
    assert_eq!(duplicate.identity(), Err(InvalidExecutionRequest::Output));
}

#[test]
#[trace("TC-176", "FR-019-AC-4")]
fn tc_176_live_output_tree_is_sealed_and_symlinks_refuse() {
    let root = tempfile::tempdir().expect("temporary root");
    let mut request = request(root.path(), &["emit-tree", "mutants.out"]);
    request.output_trees.push(OutputTreeBinding {
        role: "mutants".to_owned(),
        path: "mutants.out".to_owned(),
        required: true,
    });
    request.budget.max_output_artifacts = 2;
    request.budget.max_output_bytes = 1024;
    let result = execute(&request, &adapter());
    assert!(matches!(
        result.state,
        ProducerExecutionState::Completed { .. }
    ));
    assert_eq!(result.artifacts.len(), 1);
    assert_eq!(result.artifacts[0].role, "mutants/nested/report.json");
    assert_eq!(
        result.artifacts[0].digest,
        ContentDigest::of_bytes(b"{\"passed\":true}\n")
    );
    assert!(
        result.observed_host.is_some(),
        "Linux host context must be sampled"
    );
    let mut retained = Vec::new();
    result.artifacts[0]
        .try_reader()
        .expect("sealed tree output")
        .read_to_end(&mut retained)
        .expect("retained bytes");
    assert_eq!(retained, b"{\"passed\":true}\n");

    request.arguments[0] = ArgumentBinding::Literal {
        value: "emit-tree-link".to_owned(),
    };
    let refused = execute(&request, &adapter());
    assert!(matches!(
        refused.state,
        ProducerExecutionState::Failed {
            reason: ExecutionFailure::OutputArtifact
        }
    ));
}

#[test]
#[trace("TC-176", "FR-019-AC-4")]
fn tc_176_output_tree_absent_after_execution_fails_only_when_required() {
    let root = tempfile::tempdir().expect("temporary root");
    let mut request = request(root.path(), &["emit", "no-tree"]);
    request.output_trees.push(OutputTreeBinding {
        role: "mutants".to_owned(),
        path: "mutants.out".to_owned(),
        required: true,
    });
    request.budget.max_output_artifacts = 1;
    request.budget.max_output_bytes = 1024;
    assert!(matches!(
        execute(&request, &adapter()).state,
        ProducerExecutionState::Failed {
            reason: ExecutionFailure::OutputArtifact
        }
    ));

    request.output_trees[0].required = false;
    let result = execute(&request, &adapter());
    assert!(matches!(
        result.state,
        ProducerExecutionState::Completed { .. }
    ));
    assert!(result.artifacts.is_empty());
}

#[test]
#[trace("TC-122", "FR-019-AC-1")]
fn tc_122_completed_result_retains_canonical_identity_evidence_and_output_snapshot() {
    let root = tempfile::tempdir().expect("temporary root must be available");
    fs::write(root.path().join("input.txt"), b"accepted").expect("input fixture");
    let mut request = request(root.path(), &["copy-stdin"]);
    request.inputs.push(InputBinding {
        role: "stdin".to_owned(),
        path: "input.txt".to_owned(),
        digest: ContentDigest::of_bytes(b"accepted"),
        executable: false,
    });
    request.stdin = StdinBinding::InputArtifact {
        role: "stdin".to_owned(),
    };
    request.outputs.push(OutputBinding {
        role: "copy".to_owned(),
        path: "copy.txt".to_owned(),
        required: true,
    });
    request.arguments.push(ArgumentBinding::OutputArtifact {
        role: "copy".to_owned(),
    });
    request.budget.max_output_artifacts = 1;
    request.budget.max_output_bytes = 8;
    let expected_request_identity = request.identity().expect("valid request has identity");
    let result = ProducerExecutor::new(1)
        .expect("executor")
        .execute(
            &request,
            &CancellationToken::new(request.cancellation.clone()),
            &SnapshotAdapter { binding: binding() },
        )
        .expect("valid request");

    assert_eq!(result.protocol, PRODUCER_EXECUTION_RESULT_PROTOCOL);
    assert_eq!(result.request_identity, expected_request_identity);
    assert_eq!(result.request_identity.scheme, REQUEST_IDENTITY_SCHEME);
    assert_eq!(
        result.identity().expect("result identity").scheme,
        RESULT_IDENTITY_SCHEME
    );
    assert!(result.timing.total_nanos >= result.timing.preflight_nanos);
    assert!(matches!(
        result
            .process
            .as_ref()
            .and_then(|item| item.terminal_status),
        Some(TerminalStatus::ExitCode(0))
    ));
    assert_eq!(result.artifacts.len(), 1);
    assert_eq!(
        result.artifacts[0].digest,
        ContentDigest::of_bytes(b"accepted")
    );
    assert!(matches!(
        &result.state,
        ProducerExecutionState::Completed { observation } if observation == b"accepted"
    ));

    fs::write(root.path().join("copy.txt"), b"replaced").expect("replacement control");
    let mut retained = Vec::new();
    result.artifacts[0]
        .try_reader()
        .expect("retained reader")
        .read_to_end(&mut retained)
        .expect("retained bytes");
    assert_eq!(retained, b"accepted");
}

fn declared_output_request(
    root: &Path,
    arguments: &[&str],
    required: bool,
) -> ProducerExecutionRequest {
    let mut request = request(root, arguments);
    request.outputs.push(OutputBinding {
        role: "report".to_owned(),
        path: "report.json".to_owned(),
        required,
    });
    request.budget.max_output_artifacts = 1;
    request.budget.max_output_bytes = 1024;
    request
}

#[test]
#[trace("TC-122", "FR-019-AC-1")]
fn tc_122_stale_declared_output_is_removed_and_not_observed() {
    let root = tempfile::tempdir().expect("temporary root");
    fs::write(root.path().join("report.json"), b"stale").expect("stale output");
    let request = declared_output_request(root.path(), &["emit", "no-write"], true);
    let result = execute(&request, &adapter());
    assert!(matches!(
        result.state,
        ProducerExecutionState::Failed {
            reason: ExecutionFailure::OutputArtifact
        }
    ));
    assert!(result.artifacts.is_empty());
    assert!(!root.path().join("report.json").exists());
}

#[test]
#[trace("TC-122", "FR-019-AC-1")]
fn tc_122_declared_output_written_by_producer_is_observed_with_its_bytes() {
    let root = tempfile::tempdir().expect("temporary root");
    fs::write(root.path().join("report.json"), b"stale").expect("stale output");
    let request = declared_output_request(root.path(), &["touch", "report.json"], true);
    let result = execute(&request, &adapter());
    assert!(matches!(
        result.state,
        ProducerExecutionState::Completed { .. }
    ));
    assert_eq!(result.artifacts.len(), 1);
    assert_eq!(result.artifacts[0].path, "report.json");
    assert_eq!(
        result.artifacts[0].digest,
        ContentDigest::of_bytes(b"launched")
    );
    let mut retained = Vec::new();
    result.artifacts[0]
        .try_reader()
        .expect("retained reader")
        .read_to_end(&mut retained)
        .expect("retained bytes");
    assert_eq!(retained, b"launched");
}

#[test]
#[trace("TC-122", "FR-019-AC-1")]
fn tc_122_symlink_at_declared_output_is_removed_and_target_untouched() {
    let root = tempfile::tempdir().expect("temporary root");
    fs::write(root.path().join("target.txt"), b"keep").expect("link target");
    std::os::unix::fs::symlink("target.txt", root.path().join("report.json"))
        .expect("declared output link");
    let request = declared_output_request(root.path(), &["emit", "no-write"], false);
    let result = execute(&request, &adapter());
    assert!(matches!(
        result.state,
        ProducerExecutionState::Completed { .. }
    ));
    assert!(result.artifacts.is_empty());
    assert!(
        fs::symlink_metadata(root.path().join("report.json")).is_err(),
        "the link itself is removed"
    );
    assert_eq!(
        fs::read(root.path().join("target.txt")).expect("link target"),
        b"keep"
    );
}

#[test]
#[trace("TC-122", "FR-019-AC-1")]
fn tc_122_directory_at_declared_output_refuses_before_launch() {
    let root = tempfile::tempdir().expect("temporary root");
    fs::create_dir(root.path().join("report.json")).expect("directory at output path");
    let request = declared_output_request(root.path(), &["touch", "launched.marker"], false);
    let result = execute(&request, &adapter());
    assert!(matches!(
        result.state,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::Output
        }
    ));
    assert!(!root.path().join("launched.marker").exists());
}

#[test]
#[trace("TC-122", "FR-019-AC-1")]
fn tc_122_stale_nested_declared_output_is_removed_and_not_observed() {
    let root = tempfile::tempdir().expect("temporary root");
    fs::create_dir(root.path().join("target")).expect("output parent");
    fs::write(root.path().join("target/coverage.json"), b"stale").expect("stale output");
    let mut request = declared_output_request(root.path(), &["emit", "no-write"], true);
    request.outputs[0].path = "target/coverage.json".to_owned();
    let result = execute(&request, &adapter());
    assert!(matches!(
        result.state,
        ProducerExecutionState::Failed {
            reason: ExecutionFailure::OutputArtifact
        }
    ));
    assert!(result.artifacts.is_empty());
    assert!(!root.path().join("target/coverage.json").exists());
}

#[test]
#[trace("TC-122", "FR-019-AC-1")]
fn tc_122_nested_declared_output_with_missing_parent_is_skipped() {
    let root = tempfile::tempdir().expect("temporary root");
    let mut request = declared_output_request(root.path(), &["emit", "no-write"], false);
    request.outputs[0].path = "target/coverage.json".to_owned();
    let result = execute(&request, &adapter());
    assert!(matches!(
        result.state,
        ProducerExecutionState::Completed { .. }
    ));
    assert!(result.artifacts.is_empty());
    assert!(!root.path().join("target").exists());
}

#[test]
#[trace("TC-122", "FR-019-AC-1")]
fn tc_122_symlinked_parent_of_declared_output_refuses_before_launch() {
    let root = tempfile::tempdir().expect("temporary root");
    fs::create_dir(root.path().join("real")).expect("link target directory");
    fs::write(root.path().join("real/coverage.json"), b"keep").expect("target output");
    std::os::unix::fs::symlink("real", root.path().join("target")).expect("parent link");
    let mut request = declared_output_request(root.path(), &["touch", "launched.marker"], false);
    request.outputs[0].path = "target/coverage.json".to_owned();
    let result = execute(&request, &adapter());
    assert!(matches!(
        result.state,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::Output
        }
    ));
    assert!(!root.path().join("launched.marker").exists());
    assert_eq!(
        fs::read(root.path().join("real/coverage.json")).expect("target output"),
        b"keep"
    );
}

fn assert_result_metadata_identity(result: &ProducerExecutionResult<String>) {
    assert_result_identity_changes(result, |value| value.protocol = "other");
    assert_result_identity_changes(result, |value| value.request_identity.scheme = "other");
    assert_result_identity_changes(result, |value| {
        value.request_identity.digest = ContentDigest::of_bytes(b"other request");
    });
    assert_result_identity_changes(result, |value| value.producer.name.push('x'));
    assert_result_identity_changes(result, |value| value.producer.version.push('x'));
    assert_result_identity_changes(result, |value| value.producer.source_revision.push('x'));
    assert_result_identity_changes(result, |value| value.producer.executable.push('x'));
    assert_result_identity_changes(result, |value| {
        value.producer.executable_digest = ContentDigest::of_bytes(b"other executable");
    });
    assert_result_identity_changes(result, |value| value.observed_executable_digest = None);
    assert_result_identity_changes(result, |value| {
        value.cancellation_event = Some(CancellationBinding::Disabled);
    });
    assert_result_identity_changes(result, |value| value.timing.preflight_nanos += 1);
    assert_result_identity_changes(result, |value| value.timing.execution_nanos = None);
    assert_result_identity_changes(result, |value| value.timing.total_nanos += 1);
    assert_result_identity_changes(result, |value| {
        value.observed_host = Some(ObservedHostContext {
            machine_digest: ContentDigest::of_bytes(b"fictional host"),
            identity_source: None,
            os: "fictional-os".to_owned(),
            kernel_release: "1".to_owned(),
            architecture: "fictional-arch".to_owned(),
            cpu_model: "fictional-cpu".to_owned(),
            cpu_model_source: None,
            cpu_affinity_digest: None,
            logical_cpus: 4,
            memory_bytes: 1_024,
            runtime_class: "fixture-runtime".to_owned(),
        });
    });
}

fn assert_result_process_identity(result: &ProducerExecutionResult<String>) {
    assert_result_identity_changes(result, |value| value.process = None);
    assert_result_identity_changes(result, |value| {
        value
            .process
            .as_mut()
            .expect("process")
            .stdout
            .bytes
            .push(b'x');
    });
    assert_result_identity_changes(result, |value| {
        value.process.as_mut().expect("process").terminal_status = Some(TerminalStatus::Signal(9));
    });
    assert_result_identity_changes(result, |value| {
        value.process.as_mut().expect("process").stdout.digest =
            ContentDigest::of_bytes(b"other stdout");
    });
    assert_result_identity_changes(result, |value| {
        value.process.as_mut().expect("process").stdout.truncated = true;
    });
    assert_result_identity_changes(result, |value| {
        value
            .process
            .as_mut()
            .expect("process")
            .stderr
            .bytes
            .push(b'x');
    });
    assert_result_identity_changes(result, |value| {
        value.process.as_mut().expect("process").stderr.digest =
            ContentDigest::of_bytes(b"other stderr");
    });
    assert_result_identity_changes(result, |value| {
        value.process.as_mut().expect("process").stderr.truncated = true;
    });
}

fn assert_result_artifact_and_state_identity(result: &ProducerExecutionResult<String>) {
    assert_result_identity_changes(result, |value| value.artifacts.clear());
    assert_result_identity_changes(result, |value| value.artifacts[0].role.push('x'));
    assert_result_identity_changes(result, |value| value.artifacts[0].path.push('x'));
    assert_result_identity_changes(result, |value| value.artifacts[0].byte_length += 1);
    assert_result_identity_changes(result, |value| {
        value.artifacts[0].digest = ContentDigest::of_bytes(b"other output");
    });
    assert_result_identity_changes(result, |value| match &mut value.state {
        ProducerExecutionState::Completed { observation } => observation.push('x'),
        _ => panic!("fixture must complete"),
    });

    let state_identities = [
        ProducerExecutionState::Unavailable,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::Input,
        },
        ProducerExecutionState::Failed {
            reason: ExecutionFailure::Observation,
        },
        ProducerExecutionState::TimedOut,
        ProducerExecutionState::MalformedResponse,
        ProducerExecutionState::ContainmentFailure,
        ProducerExecutionState::Cancelled,
        ProducerExecutionState::Completed {
            observation: "x".to_owned(),
        },
    ]
    .into_iter()
    .map(|state| {
        let mut candidate = result.clone();
        candidate.state = state;
        candidate.identity().expect("state identity").digest
    })
    .collect::<BTreeSet<_>>();
    assert_eq!(state_identities.len(), 8);
}

#[test]
#[trace("TC-122", "FR-019-AC-1")]
fn tc_122_every_portable_result_field_changes_canonical_identity() {
    let root = tempfile::tempdir().expect("temporary root");
    fs::write(root.path().join("input"), b"x").expect("input fixture");
    let mut request = request(root.path(), &["copy-stdin"]);
    request.inputs.push(InputBinding {
        role: "input".to_owned(),
        path: "input".to_owned(),
        digest: ContentDigest::of_bytes(b"x"),
        executable: false,
    });
    request.stdin = StdinBinding::InputArtifact {
        role: "input".to_owned(),
    };
    request.outputs.push(OutputBinding {
        role: "output".to_owned(),
        path: "output".to_owned(),
        required: true,
    });
    request.arguments.push(ArgumentBinding::OutputArtifact {
        role: "output".to_owned(),
    });
    request.budget.max_output_artifacts = 1;
    request.budget.max_output_bytes = 1;
    request.cancellation = CancellationBinding::Event {
        authority: "fixture".to_owned(),
        event_id: "not-observed".to_owned(),
    };
    let result = execute(&request, &adapter());
    assert_result_metadata_identity(&result);
    assert_result_process_identity(&result);
    assert_result_artifact_and_state_identity(&result);
}

#[test]
#[trace("TC-122", "FR-019-AC-1")]
fn tc_122_noncanonical_adapter_observation_has_no_result_identity() {
    let root = tempfile::tempdir().expect("temporary root");
    let request = request(root.path(), &["emit", "accepted"]);
    let result = ProducerExecutor::new(1)
        .expect("executor")
        .execute(
            &request,
            &CancellationToken::new(request.cancellation.clone()),
            &NonCanonicalAdapter { binding: binding() },
        )
        .expect("valid request");
    assert!(matches!(
        result.state,
        ProducerExecutionState::Completed { .. }
    ));
    assert!(result.canonical_bytes().is_err());
    assert!(result.identity().is_err());
}

#[test]
#[trace("TC-123", "FR-019-AC-2", "FR-019-CON-1")]
fn tc_123_jcs_identity_covers_fields_and_declared_collection_semantics() {
    let root = tempfile::tempdir().expect("temporary root");
    let base = identity_request(root.path());
    let identity = base.identity().expect("base identity");
    let mut equivalent = base.clone();
    equivalent.environment = [
        ("A".to_owned(), "1".to_owned()),
        ("B".to_owned(), "2".to_owned()),
    ]
    .into_iter()
    .collect();
    equivalent.response.exit_codes =
        ExitCodeBinding::Exact([7, 0].into_iter().collect::<BTreeSet<_>>());
    assert_eq!(
        equivalent.identity().expect("normalized identity"),
        identity
    );
    assert_descriptor_identity(&base);
    assert_io_identity(&base);
    assert_control_identity(&base);
    assert_response_identity(&base);
}

#[test]
#[trace("TC-123", "FR-019-AC-2", "FR-019-CON-1")]
fn tc_123_invalid_and_mismatched_requests_refuse_without_launch() {
    let root = tempfile::tempdir().expect("temporary root");
    let marker = root.path().join("launched");
    let mut invalid = request(
        root.path(),
        &["touch", marker.to_str().expect("UTF-8 path")],
    );
    invalid.protocol = "unknown".to_owned();
    assert_eq!(invalid.identity(), Err(InvalidExecutionRequest::Protocol));
    assert_eq!(
        ProducerExecutor::new(1)
            .expect("executor")
            .execute(
                &invalid,
                &CancellationToken::new(invalid.cancellation.clone()),
                &adapter(),
            )
            .expect_err("invalid request has no result"),
        InvalidExecutionRequest::Protocol
    );
    let selected = root.path().join("selected");
    fs::write(&selected, b"selected").expect("selected fixture");
    let mut refused = request(
        root.path(),
        &["touch", marker.to_str().expect("UTF-8 path")],
    );
    refused.inputs.push(InputBinding {
        role: "candidate".to_owned(),
        path: "selected".to_owned(),
        digest: ContentDigest::of_bytes(b"wrong"),
        executable: false,
    });
    assert!(matches!(
        execute(&refused, &adapter()).state,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::Input
        }
    ));
    assert!(!marker.exists());
}

#[test]
#[trace("TC-123", "FR-019-AC-2")]
fn tc_123_declared_input_mutated_after_preflight_reads_as_verified_bytes() {
    let root = tempfile::tempdir().expect("temporary root");
    let controls = tempfile::tempdir().expect("control root");
    fs::write(root.path().join("selected.txt"), b"original").expect("selected input");
    let ready = controls.path().join("bound-ready");
    let release = controls.path().join("bound-release");
    let arguments = [
        "wait-read",
        ready.to_str().expect("UTF-8 path"),
        release.to_str().expect("UTF-8 path"),
    ];
    let mut frozen = rendezvous_request(root.path(), &arguments);
    frozen.arguments.push(ArgumentBinding::InputArtifact {
        role: "selected".to_owned(),
    });
    frozen.inputs.push(InputBinding {
        role: "selected".to_owned(),
        path: "selected.txt".to_owned(),
        digest: ContentDigest::of_bytes(b"original"),
        executable: false,
    });
    let worker = thread::spawn(move || execute(&frozen, &adapter()));
    await_ready(&ready, &worker, "fixture did not reach launch");
    fs::write(root.path().join("selected.txt"), b"replacement").expect("source mutation");
    fs::write(&release, b"release").expect("release fixture");
    let result = worker.join().expect("worker must terminate");
    assert!(matches!(
        result.state,
        ProducerExecutionState::Completed { observation } if observation == "original"
    ));
}

#[test]
#[trace("TC-123", "FR-019-AC-2")]
fn tc_123_adapter_cancellation_concurrency_and_environment_bindings_are_closed() {
    let root = tempfile::tempdir().expect("temporary root");
    let request = request(root.path(), &["emit", "accepted"]);
    let mut wrong_adapter = adapter();
    wrong_adapter.binding.adapter.revision = "other-revision".to_owned();
    assert!(matches!(
        execute(&request, &wrong_adapter).state,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::AdapterBinding
        }
    ));
    let wrong_cancellation = CancellationToken::new(CancellationBinding::Event {
        authority: "other".to_owned(),
        event_id: "event".to_owned(),
    });
    assert!(matches!(
        ProducerExecutor::new(1)
            .expect("executor")
            .execute(&request, &wrong_cancellation, &adapter())
            .expect("valid request")
            .state,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::CancellationBinding
        }
    ));
    let mut concurrency = request.clone();
    concurrency.budget.max_concurrency = 2;
    assert!(matches!(
        execute(&concurrency, &adapter()).state,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::Concurrency
        }
    ));
    let mut environment = request;
    environment.arguments = vec![ArgumentBinding::Literal {
        value: "environment".to_owned(),
    }];
    environment
        .environment
        .insert("ONLY".to_owned(), "one".to_owned());
    assert!(matches!(
        execute(&environment, &adapter()).state,
        ProducerExecutionState::Completed { observation } if observation == "ONLY=one\n"
    ));
}

#[test]
#[trace("TC-124", "FR-019-AC-3")]
fn tc_124_all_closed_states_are_distinct_canonical_and_retain_evidence() {
    let root = tempfile::tempdir().expect("temporary root");
    let adapter = adapter();
    let mut results = Vec::new();
    let mut unavailable = request(root.path(), &["emit", "accepted"]);
    unavailable.producer.executable = root.path().join("missing").display().to_string();
    results.push(execute(&unavailable, &adapter));
    let mut refused = request(root.path(), &["emit", "accepted"]);
    refused.producer.executable_digest = ContentDigest::of_bytes(b"wrong");
    results.push(execute(&refused, &adapter));
    results.push(execute(
        &request(root.path(), &["exit", "1", "failed"]),
        &adapter,
    ));
    let mut timed_out = request(root.path(), &["wait", "1000"]);
    timed_out.budget.timeout_millis = 1;
    results.push(execute(&timed_out, &adapter));
    results.push(execute(
        &request(root.path(), &["emit", "accepted"]),
        &TextAdapter {
            binding: binding(),
            reject: true,
        },
    ));
    let mut cancelled = request(root.path(), &["emit", "accepted"]);
    cancelled.cancellation = CancellationBinding::Event {
        authority: "fixture".to_owned(),
        event_id: "cancel-before-launch".to_owned(),
    };
    let token = CancellationToken::new(cancelled.cancellation.clone());
    assert!(token.cancel());
    results.push(
        ProducerExecutor::new(1)
            .expect("executor")
            .execute(&cancelled, &token, &adapter)
            .expect("valid request"),
    );
    let escaped_pid = root.path().join("escaped.pid");
    let mut containment = request(
        root.path(),
        &["escape", escaped_pid.to_str().expect("UTF-8 path")],
    );
    containment.environment.insert(
        "EA_FIXTURE_EXECUTABLE".to_owned(),
        fixture_executable().display().to_string(),
    );
    results.push(execute(&containment, &adapter));
    results.push(execute(
        &request(root.path(), &["emit", "accepted"]),
        &adapter,
    ));

    assert!(matches!(
        results[0].state,
        ProducerExecutionState::Unavailable
    ));
    assert!(matches!(
        results[1].state,
        ProducerExecutionState::Refused { .. }
    ));
    assert!(matches!(
        results[2].state,
        ProducerExecutionState::Failed {
            reason: ExecutionFailure::ExitCode(1)
        }
    ));
    assert!(matches!(results[3].state, ProducerExecutionState::TimedOut));
    assert!(matches!(
        results[4].state,
        ProducerExecutionState::MalformedResponse
    ));
    assert!(matches!(
        results[5].state,
        ProducerExecutionState::Cancelled
    ));
    assert!(matches!(
        results[6].state,
        ProducerExecutionState::ContainmentFailure
    ));
    assert!(matches!(
        results[7].state,
        ProducerExecutionState::Completed { .. }
    ));
    let identities = results
        .iter()
        .map(|result| result.identity().expect("state must be canonical").digest)
        .collect::<BTreeSet<_>>();
    assert_eq!(identities.len(), results.len());
    assert!(results[2..5].iter().all(|result| result.process.is_some()));
    assert!(results[5].process.is_none());
}

#[test]
#[trace("TC-125", "FR-019-AC-4")]
fn tc_125_exact_input_stream_output_and_nonzero_exit_boundaries_are_enforced() {
    let root = tempfile::tempdir().expect("temporary root");
    fs::write(root.path().join("input"), b"x").expect("input fixture");
    let text_adapter = adapter();
    let mut exact = request(root.path(), &["copy-stdin"]);
    exact.inputs.push(InputBinding {
        role: "stdin".to_owned(),
        path: "input".to_owned(),
        digest: ContentDigest::of_bytes(b"x"),
        executable: false,
    });
    exact.stdin = StdinBinding::InputArtifact {
        role: "stdin".to_owned(),
    };
    exact.outputs.push(OutputBinding {
        role: "copy".to_owned(),
        path: "copy".to_owned(),
        required: true,
    });
    exact.arguments.push(ArgumentBinding::OutputArtifact {
        role: "copy".to_owned(),
    });
    exact.budget.max_stdout_bytes = 1;
    exact.budget.max_input_bytes = 1;
    exact.budget.max_output_artifacts = 1;
    exact.budget.max_output_bytes = 1;
    assert!(
        matches!(execute(&exact, &text_adapter).state, ProducerExecutionState::Completed { ref observation } if observation == "x")
    );
    let mut over_input = exact.clone();
    over_input.budget.max_input_bytes = 0;
    assert!(matches!(
        execute(&over_input, &text_adapter).state,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::Input
        }
    ));
    let mut over_output = exact.clone();
    over_output.budget.max_output_bytes = 0;
    assert!(matches!(
        execute(&over_output, &text_adapter).state,
        ProducerExecutionState::Failed {
            reason: ExecutionFailure::OutputArtifact
        }
    ));
    let mut over_stream = request(root.path(), &["emit", "xx"]);
    over_stream.budget.max_stdout_bytes = 1;
    assert!(matches!(
        execute(&over_stream, &text_adapter).state,
        ProducerExecutionState::Failed {
            reason: ExecutionFailure::StdoutTooLarge
        }
    ));
    let mut nonzero = request(root.path(), &["exit", "7", "domain"]);
    nonzero.response.exit_codes = ExitCodeBinding::Exact(BTreeSet::from([7]));
    let mut nonzero_adapter = adapter();
    nonzero_adapter.binding.exit_codes = ExitCodeBinding::Exact(BTreeSet::from([7]));
    assert!(
        matches!(execute(&nonzero, &nonzero_adapter).state, ProducerExecutionState::Completed { observation } if observation == "domain")
    );
}

#[test]
#[trace("TC-125", "FR-019-AC-4", "FR-019-CON-4")]
fn tc_125_cancellation_reaps_group_and_escape_mutant_fails_containment() {
    let root = tempfile::tempdir().expect("temporary root");
    let controls = tempfile::tempdir().expect("control root");
    let ready = controls.path().join("ready");
    let completed = controls.path().join("completed");
    let arguments = [
        "spawn-child",
        ready.to_str().expect("UTF-8 path"),
        completed.to_str().expect("UTF-8 path"),
    ];
    // The fixture must stay alive until `cancellation.cancel()` below, across
    // the readiness rendezvous and the concurrency check; its child outlives
    // this request's timeout, so only cancellation (or, on failure, the
    // timeout) ends it.
    let mut running_request = rendezvous_request(root.path(), &arguments);
    running_request.environment.insert(
        "EA_FIXTURE_EXECUTABLE".to_owned(),
        fixture_executable().display().to_string(),
    );
    running_request.cancellation = CancellationBinding::Event {
        authority: "fixture".to_owned(),
        event_id: "cancel-running".to_owned(),
    };
    let executor = Arc::new(ProducerExecutor::new(1).expect("executor"));
    let cancellation = Arc::new(CancellationToken::new(running_request.cancellation.clone()));
    let worker = {
        let executor = Arc::clone(&executor);
        let cancellation = Arc::clone(&cancellation);
        thread::spawn(move || {
            executor
                .execute(&running_request, &cancellation, &adapter())
                .expect("valid request")
        })
    };
    await_ready(&ready, &worker, "fixture producer did not start");
    let concurrent = request(root.path(), &["emit", "concurrent"]);
    let concurrent_state = executor
        .execute(
            &concurrent,
            &CancellationToken::new(concurrent.cancellation.clone()),
            &adapter(),
        )
        .expect("valid concurrent request")
        .state;
    assert!(
        matches!(
            concurrent_state,
            ProducerExecutionState::Refused {
                reason: ExecutionRefusal::Concurrency
            }
        ),
        "concurrent request must be refused while the first runs: {concurrent_state:?}"
    );
    assert!(cancellation.cancel());
    let result = worker.join().expect("executor thread must terminate");
    assert!(
        matches!(result.state, ProducerExecutionState::Cancelled),
        "running request must end by cancellation: {:?}",
        result.state
    );
    assert!(result.process.is_some());
    assert!(!completed.exists());
    let escaped_pid = controls.path().join("escaped.pid");
    let mut escape = request(
        root.path(),
        &["escape", escaped_pid.to_str().expect("UTF-8 path")],
    );
    escape.environment.insert(
        "EA_FIXTURE_EXECUTABLE".to_owned(),
        fixture_executable().display().to_string(),
    );
    let escaped = execute(&escape, &adapter());
    assert!(matches!(
        escaped.state,
        ProducerExecutionState::ContainmentFailure
    ));
}

#[test]
#[trace("TC-187", "FR-019-AC-13")]
fn tc_187_producer_finds_its_own_path_sibling_and_reexecution_target() {
    let root = tempfile::tempdir().expect("temporary root must be available");
    let toolchain = tempfile::tempdir().expect("temporary toolchain must be available");
    let producer = toolchain.path().join("producer");
    fs::copy(fixture_executable(), &producer).expect("producer copy");
    fs::copy(fixture_executable(), toolchain.path().join("sibling")).expect("sibling copy");
    let producer = fs::canonicalize(producer).expect("producer path must be canonical");

    let observe = |arguments: &[&str]| {
        let mut request = request(root.path(), arguments);
        request.producer.executable = producer.display().to_string();
        request.producer.executable_digest =
            ContentDigest::of_file(&producer).expect("producer must be hashable");
        execute_staged(&request, &adapter())
    };

    assert!(matches!(
        observe(&["self-exe"]),
        ProducerExecutionState::Completed { observation }
            if observation == producer.display().to_string()
    ));
    assert!(matches!(
        observe(&["argv0"]),
        ProducerExecutionState::Completed { observation }
            if observation == producer.display().to_string()
    ));
    assert!(matches!(
        observe(&["reexec"]),
        ProducerExecutionState::Completed { observation } if observation == "reexecuted"
    ));
    assert!(matches!(
        observe(&["sibling", "sibling"]),
        ProducerExecutionState::Completed { observation } if observation == "sibling-ran"
    ));
}

#[test]
#[trace("TC-187", "FR-019-AC-13")]
fn tc_187_script_producer_sees_its_pinned_path_as_dollar_zero() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempfile::tempdir().expect("temporary root must be available");
    let toolchain = tempfile::tempdir().expect("temporary toolchain must be available");
    let script = toolchain.path().join("producer.sh");
    fs::write(&script, "#!/bin/sh\nprintf '%s' \"$0\"\n").expect("script write");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("script mode");
    let script = fs::canonicalize(script).expect("script path must be canonical");

    let mut request = request(root.path(), &[]);
    request.producer.executable = script.display().to_string();
    request.producer.executable_digest =
        ContentDigest::of_file(&script).expect("script must be hashable");

    assert!(matches!(
        execute_staged(&request, &adapter()),
        ProducerExecutionState::Completed { observation }
            if observation == script.display().to_string()
    ));
}

#[test]
#[trace("TC-128", "FR-019-AC-7")]
fn tc_128_minimal_downstream_compiles_only_producer_execution_feature() {
    let consumer = tempfile::tempdir().expect("consumer root");
    fs::create_dir(consumer.path().join("src")).expect("consumer source directory");
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::write(
        consumer.path().join("Cargo.toml"),
        format!(
            "[package]\nname='producer-consumer-fixture'\nversion='0.0.0'\nedition='2024'\nrust-version='1.98.1'\n[dependencies]\nengineering-assurance={{path={manifest_dir:?},default-features=false,features=['producer-execution']}}\nserde={{version='=1.0.228',features=['derive']}}\n"
        ),
    )
    .expect("consumer manifest");
    fs::write(
        consumer.path().join("src/main.rs"),
        "use engineering_assurance::producer_execution::RESULT_IDENTITY_SCHEME;\nfn main(){assert_eq!(RESULT_IDENTITY_SCHEME,\"sha256-jcs\");}\n",
    )
    .expect("consumer source");
    let status = Command::new(env!("CARGO"))
        .args(["check", "--offline", "--manifest-path"])
        .arg(consumer.path().join("Cargo.toml"))
        .env("CARGO_BUILD_JOBS", "2")
        .env("CARGO_TARGET_DIR", consumer.path().join("target"))
        .status()
        .expect("consumer cargo check must launch");
    assert!(status.success(), "minimal consumer must compile");
    let metadata = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--offline",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(consumer.path().join("Cargo.toml"))
        .output()
        .expect("consumer metadata must launch");
    assert!(metadata.status.success());
    let graph: serde_json::Value = serde_json::from_slice(&metadata.stdout).expect("metadata JSON");
    let assurance_package = graph["packages"]
        .as_array()
        .expect("metadata packages")
        .iter()
        .find(|package| package["name"] == "engineering-assurance")
        .expect("Engineering Assurance package");
    let assurance_id = assurance_package["id"]
        .as_str()
        .expect("Engineering Assurance package id");
    for dependency in assurance_package["dependencies"]
        .as_array()
        .expect("Engineering Assurance dependencies")
        .iter()
        .filter(|dependency| dependency["kind"].is_null())
    {
        let requirement = dependency["req"]
            .as_str()
            .expect("normal dependency requirement");
        assert!(
            requirement.starts_with('^'),
            "normal dependency {} must remain a caret range, got {requirement}",
            dependency["name"].as_str().expect("normal dependency name")
        );
    }
    let direct_dependencies = graph["resolve"]["nodes"]
        .as_array()
        .expect("metadata resolve nodes")
        .iter()
        .find(|node| node["id"] == assurance_id)
        .and_then(|node| node["deps"].as_array())
        .expect("Engineering Assurance resolve node")
        .iter()
        .filter_map(|dependency| dependency["name"].as_str())
        .collect::<BTreeSet<_>>();
    for forbidden in [
        "cap-std",
        "clap",
        "flate2",
        "jsonschema",
        "regex",
        "syn",
        "tar",
        "time",
        "unicode-casefold",
        "yaml_serde",
        "zip",
    ] {
        assert!(
            !direct_dependencies.contains(forbidden),
            "unexpected activated direct dependency {forbidden}"
        );
    }

    let feature_tree = cargo_feature_tree(&consumer.path().join("Cargo.toml"), &[]);
    assert!(
        !feature_tree.contains("arbitrary_precision"),
        "minimal producer-execution consumer must not enable serde_json arbitrary_precision"
    );

    let full_feature_tree = cargo_feature_tree(&manifest_dir.join("Cargo.toml"), &["full"]);
    assert!(
        full_feature_tree.contains("arbitrary_precision"),
        "full Engineering Assurance feature set must retain serde_json arbitrary_precision"
    );
}
