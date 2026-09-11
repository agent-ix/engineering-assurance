// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Public contract tests for bounded producer execution.

#![cfg(target_os = "linux")]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
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
    ResponseBinding, StdinBinding, TerminalStatus,
};
use ix_trace_rs::trace;

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

fn executable(name: &str) -> PathBuf {
    ["/usr/bin", "/bin"]
        .into_iter()
        .map(|root| Path::new(root).join(name))
        .find(|path| path.is_file())
        .and_then(|path| fs::canonicalize(path).ok())
        .unwrap_or_else(|| panic!("fixture executable {name} must be available"))
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

fn cancellation() -> CancellationBinding {
    CancellationBinding::Disabled
}

fn request(root: &Path, executable: &Path, arguments: &[&str]) -> ProducerExecutionRequest {
    ProducerExecutionRequest {
        protocol: PRODUCER_EXECUTION_REQUEST_PROTOCOL.to_owned(),
        producer: ProducerDescriptor {
            name: "fixture-producer".to_owned(),
            version: "1.0.0".to_owned(),
            source_revision: "fixture-revision".to_owned(),
            executable: executable.display().to_string(),
            executable_digest: ContentDigest::of_file(executable)
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
        cancellation: cancellation(),
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
    let mut request = request(root, &executable("printf"), &["a", "b"]);
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

#[test]
#[trace("TC-122", "FR-019-AC-1")]
fn tc_122_completed_result_retains_jcs_identity_timing_raw_evidence_and_artifact() {
    let root = tempfile::tempdir().expect("temporary root must be available");
    fs::write(root.path().join("input.txt"), b"accepted").expect("input fixture must be written");
    let mut request = request(root.path(), &executable("tee"), &[]);
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
    let expected_identity = request.identity().expect("valid request has identity");
    let result = execute(&request, &adapter());

    assert_eq!(result.protocol, PRODUCER_EXECUTION_RESULT_PROTOCOL);
    assert_eq!(result.request_identity, expected_identity);
    assert_eq!(result.request_identity.scheme, REQUEST_IDENTITY_SCHEME);
    assert_eq!(
        result.observed_executable_digest,
        Some(request.producer.executable_digest.clone())
    );
    assert!(result.timing.total_nanos >= result.timing.preflight_nanos);
    assert!(result.timing.execution_nanos.is_some(), "{result:#?}");
    assert!(matches!(
        result
            .process
            .as_ref()
            .and_then(|item| item.terminal_status),
        Some(TerminalStatus::ExitCode(0))
    ));
    assert_eq!(result.artifacts.len(), 1);
    assert_eq!(result.artifacts[0].path, "copy.txt");
    assert_eq!(result.artifacts[0].byte_length, 8);
    assert_eq!(
        result.artifacts[0].digest,
        ContentDigest::of_bytes(b"accepted")
    );
    assert!(matches!(
        result.state,
        ProducerExecutionState::Completed { observation } if observation == "accepted"
    ));
}

#[test]
#[trace("TC-122", "FR-019-AC-1")]
fn tc_122_external_language_producer_uses_exact_interpreter_and_retained_script() {
    let root = tempfile::tempdir().expect("temporary root must be available");
    let script = b"print('python-producer')\n";
    fs::write(root.path().join("producer.py"), script).expect("script fixture must be written");
    let mut request = request(root.path(), &executable("python3"), &[]);
    request.inputs.push(InputBinding {
        role: "producer-script".to_owned(),
        path: "producer.py".to_owned(),
        digest: ContentDigest::of_bytes(script),
    });
    request.arguments.push(ArgumentBinding::InputArtifact {
        role: "producer-script".to_owned(),
    });
    request.budget.max_input_bytes =
        u64::try_from(script.len()).expect("fixture size must fit the request budget");

    assert!(matches!(
        execute(&request, &adapter()).state,
        ProducerExecutionState::Completed { observation }
            if observation == "python-producer\n"
    ));
}

#[test]
#[trace("TC-123", "FR-019-AC-2", "FR-019-CON-1")]
fn tc_123_jcs_identity_covers_fields_and_declared_collection_semantics() {
    let root = tempfile::tempdir().expect("temporary root must be available");
    let base = identity_request(root.path());
    let identity = base.identity().expect("base identity");

    let mut reordered = base.clone();
    reordered.arguments.swap(0, 1);
    assert_ne!(reordered.identity().expect("ordered identity"), identity);

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
fn tc_123_structural_error_has_no_result_and_preflight_refuses_without_launch() {
    let root = tempfile::tempdir().expect("temporary root must be available");
    let marker = root.path().join("launched");
    let marker_text = marker.to_str().expect("fixture path is UTF-8");
    let mut invalid = request(root.path(), &executable("touch"), &[marker_text]);
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

    let mut invalid_exit = request(root.path(), &executable("printf"), &["accepted"]);
    invalid_exit.response.exit_codes = ExitCodeBinding::Exact(BTreeSet::from([-1]));
    assert_eq!(
        invalid_exit.identity(),
        Err(InvalidExecutionRequest::Contract)
    );

    let selected = root.path().join("selected");
    fs::write(&selected, b"selected").expect("selected fixture");
    let mut refused = request(root.path(), &executable("touch"), &[marker_text]);
    refused.inputs.push(InputBinding {
        role: "candidate".to_owned(),
        path: "selected".to_owned(),
        digest: ContentDigest::of_bytes(b"wrong"),
    });
    let result = execute(&refused, &adapter());
    assert!(matches!(
        result.state,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::Input
        }
    ));
    assert!(!marker.exists());
}

#[test]
#[trace("TC-123", "FR-019-AC-2", "FR-019-CON-1")]
fn tc_123_capability_paths_reject_links_and_child_environment_is_closed() {
    let root = tempfile::tempdir().expect("temporary root must be available");
    fs::write(root.path().join("selected"), b"selected").expect("selected fixture");
    symlink(root.path().join("selected"), root.path().join("linked")).expect("fixture symlink");
    let mut linked = request(root.path(), &executable("printf"), &["unused"]);
    linked.inputs.push(InputBinding {
        role: "candidate".to_owned(),
        path: "linked".to_owned(),
        digest: ContentDigest::of_bytes(b"selected"),
    });
    assert!(matches!(
        execute(&linked, &adapter()).state,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::Input
        }
    ));

    symlink(
        root.path().join("selected"),
        root.path().join("linked-output"),
    )
    .expect("output symlink fixture");
    let mut linked_output = request(root.path(), &executable("printf"), &["unused"]);
    linked_output.outputs.push(OutputBinding {
        role: "result".to_owned(),
        path: "linked-output".to_owned(),
        required: true,
    });
    linked_output.budget.max_output_artifacts = 1;
    linked_output.budget.max_output_bytes = 8;
    assert!(matches!(
        execute(&linked_output, &adapter()).state,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::Output
        }
    ));

    let mut environment = request(root.path(), &executable("env"), &[]);
    environment
        .environment
        .insert("ONLY".to_owned(), "one".to_owned());
    let result = execute(&environment, &adapter());
    assert!(matches!(
        result.state,
        ProducerExecutionState::Completed { observation } if observation == "ONLY=one\n"
    ));
}

#[test]
#[trace("TC-123", "FR-019-AC-2")]
fn tc_123_runtime_adapter_and_cancellation_bindings_must_match() {
    let root = tempfile::tempdir().expect("temporary root must be available");
    let request = request(root.path(), &executable("printf"), &["accepted"]);
    let mut wrong_adapter = adapter();
    wrong_adapter.binding.adapter.revision = "other-revision".to_owned();
    assert!(matches!(
        execute(&request, &wrong_adapter).state,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::AdapterBinding
        }
    ));

    let wrong_cancellation = CancellationToken::new(CancellationBinding::Event {
        authority: "other-authority".to_owned(),
        event_id: "other-event".to_owned(),
    });
    let result = ProducerExecutor::new(1)
        .expect("executor")
        .execute(&request, &wrong_cancellation, &adapter())
        .expect("request structure remains valid");
    assert!(matches!(
        result.state,
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

    let mut invalid_descendants = request;
    invalid_descendants.budget.max_descendants = 0;
    assert_eq!(
        invalid_descendants.identity(),
        Err(InvalidExecutionRequest::Budget)
    );
}

#[test]
#[trace("TC-123", "FR-019-AC-2")]
fn tc_123_input_path_replacement_cannot_change_the_retained_snapshot() {
    let root = tempfile::tempdir().expect("temporary root must be available");
    let script = b"import pathlib,sys,time\npathlib.Path('ready').write_text('ready')\nwhile not pathlib.Path('continue').exists(): time.sleep(0.001)\nprint(pathlib.Path(sys.argv[1]).read_text(), end='')\n";
    fs::write(root.path().join("producer.py"), script).expect("script fixture must be written");
    fs::write(root.path().join("selected.txt"), b"original")
        .expect("selected input must be written");
    let mut request = request(root.path(), &executable("python3"), &[]);
    request.inputs = vec![
        InputBinding {
            role: "script".to_owned(),
            path: "producer.py".to_owned(),
            digest: ContentDigest::of_bytes(script),
        },
        InputBinding {
            role: "selected".to_owned(),
            path: "selected.txt".to_owned(),
            digest: ContentDigest::of_bytes(b"original"),
        },
    ];
    request.arguments = vec![
        ArgumentBinding::InputArtifact {
            role: "script".to_owned(),
        },
        ArgumentBinding::InputArtifact {
            role: "selected".to_owned(),
        },
    ];
    request.budget.max_input_bytes = 1_024;
    let executor = Arc::new(ProducerExecutor::new(1).expect("executor"));
    let worker = {
        let executor = Arc::clone(&executor);
        thread::spawn(move || {
            executor
                .execute(
                    &request,
                    &CancellationToken::new(request.cancellation.clone()),
                    &adapter(),
                )
                .expect("valid request")
        })
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    while !root.path().join("ready").exists() {
        assert!(Instant::now() < deadline, "fixture producer did not start");
        thread::yield_now();
    }
    fs::write(root.path().join("selected.txt"), b"replacement")
        .expect("selected path must be replaceable");
    fs::write(root.path().join("continue"), b"continue").expect("fixture release signal");
    let result = worker.join().expect("executor thread must terminate");
    assert!(matches!(
        result.state,
        ProducerExecutionState::Completed { observation } if observation == "original"
    ));
}

#[test]
#[trace("TC-124", "FR-019-AC-3")]
fn tc_124_public_result_states_remain_distinct_and_launched_states_retain_evidence() {
    let root = tempfile::tempdir().expect("temporary root must be available");
    let adapter = adapter();

    let mut unavailable = request(root.path(), &executable("printf"), &["accepted"]);
    unavailable.producer.executable = root.path().join("missing").display().to_string();
    assert!(matches!(
        execute(&unavailable, &adapter).state,
        ProducerExecutionState::Unavailable
    ));

    let mut refused = request(root.path(), &executable("printf"), &["accepted"]);
    refused.producer.executable_digest = ContentDigest::of_bytes(b"wrong");
    assert!(matches!(
        execute(&refused, &adapter).state,
        ProducerExecutionState::Refused { .. }
    ));

    let failed = execute(&request(root.path(), &executable("false"), &[]), &adapter);
    assert!(matches!(
        failed.state,
        ProducerExecutionState::Failed {
            reason: ExecutionFailure::ExitCode(1)
        }
    ));
    assert!(failed.process.is_some());

    let mut timed_out = request(root.path(), &executable("sleep"), &["1"]);
    timed_out.budget.timeout_millis = 1;
    let timed_out = execute(&timed_out, &adapter);
    assert!(matches!(timed_out.state, ProducerExecutionState::TimedOut));
    assert!(timed_out.process.is_some());

    let malformed = request(root.path(), &executable("printf"), &["accepted"]);
    let malformed = execute(
        &malformed,
        &TextAdapter {
            binding: binding(),
            reject: true,
        },
    );
    assert!(matches!(
        malformed.state,
        ProducerExecutionState::MalformedResponse
    ));
    assert!(malformed.process.is_some());

    let mut cancelled = request(root.path(), &executable("printf"), &["accepted"]);
    let cancellation_binding = CancellationBinding::Event {
        authority: "fixture".to_owned(),
        event_id: "cancel-before-launch".to_owned(),
    };
    cancelled.cancellation = cancellation_binding.clone();
    let token = CancellationToken::new(cancelled.cancellation.clone());
    assert!(token.cancel());
    let cancelled = ProducerExecutor::new(1)
        .expect("executor")
        .execute(&cancelled, &token, &adapter)
        .expect("valid request");
    assert!(matches!(cancelled.state, ProducerExecutionState::Cancelled));
    assert_eq!(cancelled.cancellation_event, Some(cancellation_binding));
    assert!(cancelled.process.is_none());

    let containment = request(
        root.path(),
        &executable("sh"),
        &["-c", "/usr/bin/setsid /usr/bin/sleep 10 & wait"],
    );
    assert!(matches!(
        execute(&containment, &adapter).state,
        ProducerExecutionState::ContainmentFailure
    ));

    assert!(matches!(
        execute(
            &request(root.path(), &executable("printf"), &["accepted"]),
            &adapter
        )
        .state,
        ProducerExecutionState::Completed { .. }
    ));
}

#[test]
#[trace("TC-125", "FR-019-AC-4")]
fn tc_125_exact_input_stream_output_and_nonzero_exit_boundaries_are_enforced() {
    let root = tempfile::tempdir().expect("temporary root must be available");
    fs::write(root.path().join("input"), b"x").expect("input fixture");
    let text_adapter = adapter();

    let mut exact = request(root.path(), &executable("tee"), &[]);
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
    let exact_result = execute(&exact, &text_adapter);
    assert!(
        matches!(
            exact_result.state,
            ProducerExecutionState::Completed { ref observation } if observation == "x"
        ),
        "{exact_result:#?}"
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
    over_output.outputs[0].path = "copy-over".to_owned();
    over_output.budget.max_output_bytes = 0;
    assert!(matches!(
        execute(&over_output, &text_adapter).state,
        ProducerExecutionState::Failed {
            reason: ExecutionFailure::OutputArtifact
        }
    ));

    let mut over_stream = request(root.path(), &executable("printf"), &["xx"]);
    over_stream.budget.max_stdout_bytes = 1;
    assert!(matches!(
        execute(&over_stream, &text_adapter).state,
        ProducerExecutionState::Failed {
            reason: ExecutionFailure::StdoutTooLarge
        }
    ));

    let mut nonzero = request(
        root.path(),
        &executable("sh"),
        &["-c", "printf domain; exit 7"],
    );
    nonzero.response.exit_codes = ExitCodeBinding::Exact(BTreeSet::from([7]));
    let mut nonzero_adapter = adapter();
    nonzero_adapter.binding.exit_codes = ExitCodeBinding::Exact(BTreeSet::from([7]));
    assert!(matches!(
        execute(&nonzero, &nonzero_adapter).state,
        ProducerExecutionState::Completed { observation } if observation == "domain"
    ));
}

#[test]
#[trace("TC-125", "FR-019-AC-4", "FR-019-CON-4")]
fn tc_125_cancellation_reaps_group_and_escape_mutant_fails_containment() {
    let root = tempfile::tempdir().expect("temporary root must be available");
    let ready = root.path().join("ready");
    let completed = root.path().join("completed");
    let script = "touch \"$1\"; /usr/bin/sleep 10 & wait; touch \"$2\"";
    let arguments = [
        "-c",
        script,
        "fixture",
        ready.to_str().expect("fixture path must be UTF-8"),
        completed.to_str().expect("fixture path must be UTF-8"),
    ];
    let mut running_request = request(root.path(), &executable("sh"), &arguments);
    running_request.cancellation = CancellationBinding::Event {
        authority: "fixture".to_owned(),
        event_id: "cancel-running".to_owned(),
    };
    let text_adapter = adapter();
    let executor = Arc::new(ProducerExecutor::new(1).expect("executor"));
    let cancellation = Arc::new(CancellationToken::new(running_request.cancellation.clone()));
    let worker = {
        let executor = Arc::clone(&executor);
        let cancellation = Arc::clone(&cancellation);
        thread::spawn(move || {
            executor
                .execute(&running_request, &cancellation, &text_adapter)
                .expect("valid request")
        })
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    while !ready.exists() {
        assert!(Instant::now() < deadline, "fixture producer did not start");
        thread::yield_now();
    }
    let concurrent = request(root.path(), &executable("printf"), &["concurrent"]);
    let concurrency_result = executor
        .execute(
            &concurrent,
            &CancellationToken::new(concurrent.cancellation.clone()),
            &adapter(),
        )
        .expect("concurrent request must be valid");
    assert!(matches!(
        concurrency_result.state,
        ProducerExecutionState::Refused {
            reason: ExecutionRefusal::Concurrency
        }
    ));
    assert!(cancellation.cancel());
    let result = worker.join().expect("executor thread must terminate");
    assert!(matches!(result.state, ProducerExecutionState::Cancelled));
    assert!(result.process.is_some());
    assert!(!completed.exists());

    let escaped_pid = root.path().join("escaped.pid");
    let escape_script = format!(
        "/usr/bin/setsid /usr/bin/sh -c 'echo $$ > {}; /usr/bin/sleep 10' & wait",
        escaped_pid.display()
    );
    let escape = request(root.path(), &executable("sh"), &["-c", &escape_script]);
    let escaped = execute(&escape, &adapter());
    assert!(matches!(
        escaped.state,
        ProducerExecutionState::ContainmentFailure
    ));
}
