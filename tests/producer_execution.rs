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
    thread,
    time::{Duration, Instant},
};

use engineering_assurance::producer_execution::{
    ArgumentBinding, CancellationBinding, CancellationToken, ContainmentBinding, ContentDigest,
    ContractBinding, ExecutionBudget, ExecutionFailure, ExecutionProcedure, ExecutionRefusal,
    ExitCodeBinding, InputBinding, InvalidExecutionRequest, MalformedResponse, OutputArtifact,
    OutputBinding, PRODUCER_EXECUTION_REQUEST_PROTOCOL, PRODUCER_EXECUTION_RESULT_PROTOCOL,
    ProcessEvidence, ProducerDescriptor, ProducerExecutionRequest, ProducerExecutionResult,
    ProducerExecutionState, ProducerExecutor, ProducerResponseAdapter, REQUEST_IDENTITY_SCHEME,
    RESULT_IDENTITY_SCHEME, ResponseBinding, StdinBinding, TerminalStatus,
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
        },
        InputBinding {
            role: "input-2".to_owned(),
            path: "input-2".to_owned(),
            digest: ContentDigest::of_bytes(b"input-2"),
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
#[trace("TC-122", "FR-019-AC-1")]
fn tc_122_completed_result_retains_canonical_identity_evidence_and_output_snapshot() {
    let root = tempfile::tempdir().expect("temporary root must be available");
    fs::write(root.path().join("input.txt"), b"accepted").expect("input fixture");
    let mut request = request(root.path(), &["copy-stdin"]);
    request.inputs.push(InputBinding {
        role: "stdin".to_owned(),
        path: "input.txt".to_owned(),
        digest: ContentDigest::of_bytes(b"accepted"),
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
#[trace("TC-123", "FR-019-AC-2", "FR-019-CON-1")]
fn tc_123_staged_projection_excludes_extra_and_freezes_declared_input() {
    let root = tempfile::tempdir().expect("temporary root");
    fs::write(root.path().join("extra.txt"), b"ambient").expect("extra control");
    let absent = execute(&request(root.path(), &["exists", "extra.txt"]), &adapter());
    assert!(matches!(
        absent.state,
        ProducerExecutionState::Completed { observation } if observation == "absent"
    ));

    let controls = tempfile::tempdir().expect("control root");
    let ready = controls.path().join("unbound-ready");
    let release = controls.path().join("unbound-release");
    let arguments = [
        "wait-exists",
        ready.to_str().expect("UTF-8 path"),
        release.to_str().expect("UTF-8 path"),
        "extra.txt",
    ];
    let unbound = request(root.path(), &arguments);
    let unbound_worker = thread::spawn(move || execute(&unbound, &adapter()));
    let deadline = Instant::now() + Duration::from_secs(2);
    while !ready.exists() {
        assert!(Instant::now() < deadline, "fixture did not reach launch");
        thread::yield_now();
    }
    fs::write(root.path().join("extra.txt"), b"changed-unbound").expect("unbound mutation");
    fs::write(&release, b"release").expect("release fixture");
    assert!(matches!(
        unbound_worker.join().expect("worker must terminate").state,
        ProducerExecutionState::Completed { observation } if observation == "absent"
    ));

    fs::write(root.path().join("selected.txt"), b"original").expect("selected input");
    let ready = controls.path().join("bound-ready");
    let release = controls.path().join("bound-release");
    let arguments = [
        "wait-read",
        ready.to_str().expect("UTF-8 path"),
        release.to_str().expect("UTF-8 path"),
        "selected.txt",
    ];
    let mut frozen = request(root.path(), &arguments);
    frozen.inputs.push(InputBinding {
        role: "selected".to_owned(),
        path: "selected.txt".to_owned(),
        digest: ContentDigest::of_bytes(b"original"),
    });
    let worker = thread::spawn(move || execute(&frozen, &adapter()));
    let deadline = Instant::now() + Duration::from_secs(2);
    while !ready.exists() {
        assert!(Instant::now() < deadline, "fixture did not reach launch");
        thread::yield_now();
    }
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
#[trace("TC-123", "FR-019-AC-2", "FR-019-CON-1")]
fn tc_123_output_projection_rejects_parent_leaf_conflicts_in_either_order() {
    let root = tempfile::tempdir().expect("temporary root");
    for paths in [["result", "result/nested"], ["result/nested", "result"]] {
        let mut candidate = request(root.path(), &["emit", "must-not-launch"]);
        candidate.outputs = paths
            .into_iter()
            .enumerate()
            .map(|(index, path)| OutputBinding {
                role: format!("output-{index}"),
                path: path.to_owned(),
                required: false,
            })
            .collect();
        candidate.budget.max_output_artifacts = 2;
        assert!(matches!(
            execute(&candidate, &adapter()).state,
            ProducerExecutionState::Refused {
                reason: ExecutionRefusal::Output
            }
        ));
    }
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
    let mut running_request = request(root.path(), &arguments);
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
    let deadline = Instant::now() + Duration::from_secs(2);
    while !ready.exists() {
        assert!(Instant::now() < deadline, "fixture producer did not start");
        thread::yield_now();
    }
    let concurrent = request(root.path(), &["emit", "concurrent"]);
    assert!(matches!(
        executor
            .execute(
                &concurrent,
                &CancellationToken::new(concurrent.cancellation.clone()),
                &adapter()
            )
            .expect("valid concurrent request")
            .state,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::Concurrency
        }
    ));
    assert!(cancellation.cancel());
    let result = worker.join().expect("executor thread must terminate");
    assert!(matches!(result.state, ProducerExecutionState::Cancelled));
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
#[trace("TC-128", "FR-019-AC-7")]
fn tc_128_minimal_downstream_compiles_only_producer_execution_feature() {
    let consumer = tempfile::tempdir().expect("consumer root");
    fs::create_dir(consumer.path().join("src")).expect("consumer source directory");
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::write(
        consumer.path().join("Cargo.toml"),
        format!(
            "[package]\nname='producer-consumer-fixture'\nversion='0.0.0'\nedition='2024'\nrust-version='1.98.1'\n[dependencies]\nengineering-assurance={{path={manifest_dir:?},default-features=false,features=['producer-execution']}}\n"
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
    let assurance_id = graph["packages"]
        .as_array()
        .expect("metadata packages")
        .iter()
        .find(|package| package["name"] == "engineering-assurance")
        .and_then(|package| package["id"].as_str())
        .expect("Engineering Assurance package id");
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
}
