// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded execution of caller-declared native producers.
//!
//! This is the sole process-capable public library module. It owns retained
//! executable and input identity, capability-rooted artifact access,
//! environment isolation, bounded capture, cooperative cancellation, and the
//! explicitly cooperative process-group confinement profile. Callers retain
//! their response types and domain semantics through [`ProducerResponseAdapter`].

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Component, Path},
    process::ExitStatus,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[cfg(unix)]
use std::{
    os::unix::process::{CommandExt, ExitStatusExt},
    process::{Command, Stdio},
    thread,
    time::Instant,
};

#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Closed producer-execution request protocol.
pub const PRODUCER_EXECUTION_REQUEST_PROTOCOL: &str =
    "engineering-assurance.producer-execution-request/v1";
/// Closed producer-execution result protocol.
pub const PRODUCER_EXECUTION_RESULT_PROTOCOL: &str =
    "engineering-assurance.producer-execution-result/v1";
/// Request-identity scheme.
pub const REQUEST_IDENTITY_SCHEME: &str = "sha256-jcs";
/// Result-identity scheme.
pub const RESULT_IDENTITY_SCHEME: &str = "sha256-jcs";
/// Maximum accepted wall-clock budget in milliseconds (24 hours).
pub const MAX_TIMEOUT_MILLIS: u64 = 86_400_000;
/// Maximum accepted capture budget for either process stream.
pub const MAX_STREAM_BYTES: usize = 8_388_608;
/// Maximum aggregate selected-input byte budget.
pub const MAX_INPUT_BYTES: u64 = 1_073_741_824;
/// Maximum aggregate selected-output byte budget.
pub const MAX_OUTPUT_BYTES: u64 = 1_073_741_824;
/// Maximum argument population.
pub const MAX_ARGUMENTS: usize = 4_096;
/// Maximum selected input or output population.
pub const MAX_ARTIFACTS: usize = 4_096;
/// Maximum observable descendant population per invocation.
pub const MAX_DESCENDANTS: usize = 4_096;
/// Maximum explicit environment population.
pub const MAX_ENVIRONMENT: usize = 512;
/// Maximum executor-wide concurrent process population.
pub const MAX_CONCURRENCY: usize = 64;
const MAX_TEXT_BYTES: usize = 32_768;
const MAX_EXECUTABLE_BYTES: u64 = 1_073_741_824;

/// A validated lowercase SHA-256 retained-byte identity.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ContentDigest(Box<str>);

impl ContentDigest {
    /// Parses an exact lowercase SHA-256 hexadecimal value.
    ///
    /// # Errors
    ///
    /// Returns [`DigestError::Invalid`] unless `value` is exactly 64 lowercase
    /// hexadecimal characters.
    pub fn parse(value: &str) -> Result<Self, DigestError> {
        if value.len() != 64
            || !value
                .as_bytes()
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        {
            return Err(DigestError::Invalid);
        }
        Ok(Self(value.into()))
    }

    /// Computes the identity of an in-memory byte sequence.
    #[must_use]
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(hex_digest(Sha256::digest(bytes).as_slice()).into())
    }

    /// Computes a fixture or caller-side file identity.
    ///
    /// Execution itself uses retained, capability-confined descriptors and
    /// does not rely on this convenience helper.
    ///
    /// # Errors
    ///
    /// Returns [`DigestError`] when the path is unavailable, linked, not a
    /// bounded regular file, or cannot be read completely.
    pub fn of_file(path: &Path) -> Result<Self, DigestError> {
        let metadata = std::fs::symlink_metadata(path).map_err(|_| DigestError::Unavailable)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(DigestError::NotRegular);
        }
        if metadata.len() > MAX_EXECUTABLE_BYTES {
            return Err(DigestError::TooLarge);
        }
        let mut file = File::open(path).map_err(|_| DigestError::Unreadable)?;
        digest_reader(&mut file, metadata.len(), None).map_err(|outcome| match outcome {
            DigestOutcome::Cancelled | DigestOutcome::Unreadable => DigestError::Unreadable,
            DigestOutcome::TooLarge => DigestError::TooLarge,
        })
    }

    /// Returns the lowercase hexadecimal identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Failure while constructing a retained-byte identity outside execution.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum DigestError {
    /// The supplied text is not lowercase SHA-256 hexadecimal.
    #[error("digest is not lowercase SHA-256 hexadecimal")]
    Invalid,
    /// The selected file does not exist or cannot be resolved.
    #[error("selected file is unavailable")]
    Unavailable,
    /// The selected path is linked or is not a regular file.
    #[error("selected path is not a regular file")]
    NotRegular,
    /// The selected file exceeds the supported identity ceiling.
    #[error("selected file exceeds the identity ceiling")]
    TooLarge,
    /// The selected file could not be read completely.
    #[error("selected file is unreadable")]
    Unreadable,
}

/// Exact versioned identity of a caller contract or implementation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractBinding {
    /// Stable contract kind.
    pub kind: String,
    /// Exact contract version.
    pub version: String,
    /// Exact source revision.
    pub revision: String,
    /// SHA-256 identity of the retained contract or implementation bytes.
    pub digest: ContentDigest,
}

/// Exact identity and provenance for the selected producer executable.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProducerDescriptor {
    /// Stable producer name governed by the caller.
    pub name: String,
    /// Exact producer version governed by the caller.
    pub version: String,
    /// Exact source revision governed by the caller.
    pub source_revision: String,
    /// Absolute executable path; ambient `PATH` lookup is forbidden.
    pub executable: String,
    /// Expected identity of the retained executable bytes.
    pub executable_digest: ContentDigest,
}

/// The closed invocation procedure.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionProcedure {
    /// Execute the retained file descriptor directly without a shell.
    Direct,
}

/// One ordered producer argument.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ArgumentBinding {
    /// One exact literal argument.
    Literal {
        /// Argument value.
        value: String,
    },
    /// A retained selected input exposed through its descriptor path.
    InputArtifact {
        /// Unique input role declared by the request.
        role: String,
    },
    /// A declared output path relative to the capability root.
    OutputArtifact {
        /// Unique output role declared by the request.
        role: String,
    },
}

/// One selected regular-file input bound into the request identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputBinding {
    /// Caller-owned unique role of the input.
    pub role: String,
    /// Normal relative path beneath the capability root.
    pub path: String,
    /// Expected identity of the selected input bytes.
    pub digest: ContentDigest,
}

/// Closed stdin source.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StdinBinding {
    /// Deliver an immediate end-of-file.
    Null,
    /// Deliver bytes from an exact retained selected input.
    InputArtifact {
        /// Unique input role declared by the request.
        role: String,
    },
}

/// One selected regular-file output bound before launch.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputBinding {
    /// Caller-owned unique role of the output.
    pub role: String,
    /// Normal relative path beneath the capability root.
    pub path: String,
    /// Whether absence after execution is an executor failure.
    pub required: bool,
}

/// Exit-code behavior declared by a response protocol.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "codes", rename_all = "snake_case")]
pub enum ExitCodeBinding {
    /// Give every normal exit code to the typed response adapter.
    Any,
    /// Give only this normalized set of normal exits to the adapter.
    Exact(BTreeSet<i32>),
}

/// Identity of the caller-owned response contract and decoder.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseBinding {
    /// Exact response-protocol contract.
    pub protocol: ContractBinding,
    /// Exact response-adapter implementation.
    pub adapter: ContractBinding,
    /// Normal exit codes admitted to the response adapter.
    pub exit_codes: ExitCodeBinding,
}

/// Explicitly cooperative process-group confinement.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "profile", rename_all = "kebab-case")]
pub enum ContainmentBinding {
    /// POSIX process group plus an exact producer contract prohibiting escape.
    ProcessGroupV1 {
        /// Contract that prohibits `setsid`, daemonization and untracked children.
        contract: ContractBinding,
    },
}

/// Identity of the authority and event allowed to cancel an invocation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CancellationBinding {
    /// The request cannot be cancelled by a caller event.
    Disabled,
    /// One exact caller-owned cancellation event.
    Event {
        /// Stable authority identity.
        authority: String,
        /// Unique event identity in that authority's domain.
        event_id: String,
    },
}

/// Caller-selected execution ceilings beneath the library hard limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionBudget {
    /// Maximum wall-clock duration in milliseconds.
    pub timeout_millis: u64,
    /// Maximum captured stdout bytes.
    pub max_stdout_bytes: usize,
    /// Maximum captured stderr bytes.
    pub max_stderr_bytes: usize,
    /// Maximum aggregate bytes across selected inputs.
    pub max_input_bytes: u64,
    /// Maximum declared output-artifact population.
    pub max_output_artifacts: usize,
    /// Maximum aggregate bytes across observed output artifacts.
    pub max_output_bytes: u64,
    /// Maximum observable descendant population for this invocation.
    pub max_descendants: usize,
    /// Exact executor-wide concurrency ceiling expected by this request.
    pub max_concurrency: usize,
}

/// One exact producer invocation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProducerExecutionRequest {
    /// Exact [`PRODUCER_EXECUTION_REQUEST_PROTOCOL`] discriminator.
    pub protocol: String,
    /// Selected producer executable and provenance.
    pub producer: ProducerDescriptor,
    /// Exact caller domain request or context contract.
    pub caller: ContractBinding,
    /// Direct execution procedure.
    pub procedure: ExecutionProcedure,
    /// Absolute process capability root opened without following links.
    pub capability_root: String,
    /// Ordered argument bindings excluding argv zero.
    pub arguments: Vec<ArgumentBinding>,
    /// Complete explicit child environment; the ambient environment is cleared.
    pub environment: BTreeMap<String, String>,
    /// Ordered input-artifact bindings.
    pub inputs: Vec<InputBinding>,
    /// Closed stdin binding.
    pub stdin: StdinBinding,
    /// Ordered output-artifact bindings.
    pub outputs: Vec<OutputBinding>,
    /// Explicit cooperative confinement contract.
    pub containment: ContainmentBinding,
    /// Exact cancellation authority and event.
    pub cancellation: CancellationBinding,
    /// Caller-selected resource ceilings.
    pub budget: ExecutionBudget,
    /// Expected caller-owned response protocol and adapter.
    pub response: ResponseBinding,
}

impl ProducerExecutionRequest {
    /// Validates the closed structure and computes its RFC 8785 identity.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidExecutionRequest`] before minting an identity when any
    /// closed request field is structurally invalid.
    pub fn identity(&self) -> Result<RequestIdentity, InvalidExecutionRequest> {
        validate_structure(self)?;
        let canonical = serde_json_canonicalizer::to_vec(self)
            .map_err(|_| InvalidExecutionRequest::IdentityEncoding)?;
        Ok(RequestIdentity {
            scheme: REQUEST_IDENTITY_SCHEME,
            digest: ContentDigest::of_bytes(&canonical),
        })
    }
}

/// RFC 8785 canonical request identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestIdentity {
    /// Exact identity algorithm and canonicalization scheme.
    pub scheme: &'static str,
    /// SHA-256 digest of the canonical request bytes.
    pub digest: ContentDigest,
}

/// Structurally invalid requests mint no request identity or result.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum InvalidExecutionRequest {
    /// The request protocol is unsupported.
    #[error("unsupported producer-execution request protocol")]
    Protocol,
    /// Producer or caller provenance is empty or over its limit.
    #[error("invalid producer or caller binding")]
    Provenance,
    /// The capability root or an artifact path is not syntactically admissible.
    #[error("invalid capability path")]
    Path,
    /// An argument binding is invalid or references an undeclared role.
    #[error("invalid argument binding")]
    Argument,
    /// An input binding is invalid or duplicated.
    #[error("invalid input binding")]
    Input,
    /// An output binding is invalid or duplicated.
    #[error("invalid output binding")]
    Output,
    /// A stdin binding references an undeclared input.
    #[error("invalid stdin binding")]
    Stdin,
    /// The explicit environment is invalid.
    #[error("invalid environment binding")]
    Environment,
    /// One caller-selected ceiling is invalid.
    #[error("invalid execution budget")]
    Budget,
    /// A response or confinement contract is invalid.
    #[error("invalid response or confinement binding")]
    Contract,
    /// A cancellation binding is invalid.
    #[error("invalid cancellation binding")]
    Cancellation,
    /// Canonical JSON encoding failed.
    #[error("request identity encoding failed")]
    IdentityEncoding,
}

/// Cooperative cancellation shared between a caller and an execution.
#[derive(Debug)]
pub struct CancellationToken {
    binding: CancellationBinding,
    requested: AtomicBool,
}

impl CancellationToken {
    /// Creates a token for the exact request cancellation binding.
    #[must_use]
    pub const fn new(binding: CancellationBinding) -> Self {
        Self {
            binding,
            requested: AtomicBool::new(false),
        }
    }

    /// Requests the bound event. Returns false for a disabled token.
    pub fn cancel(&self) -> bool {
        if matches!(self.binding, CancellationBinding::Disabled) {
            return false;
        }
        self.requested.store(true, Ordering::Release);
        true
    }

    fn is_cancelled(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }
}

/// One bounded captured stream.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedStream {
    /// Retained bytes, never exceeding the request limit.
    pub bytes: Vec<u8>,
    /// SHA-256 identity of the retained bytes.
    pub digest: ContentDigest,
    /// Whether additional bytes existed beyond the retained limit.
    pub truncated: bool,
}

/// Closed terminal status observed from the direct producer process.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum TerminalStatus {
    /// Normal process exit.
    ExitCode(i32),
    /// Signal termination on the supported Linux host.
    Signal(i32),
}

/// Bounded raw process evidence retained with every observable launched state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEvidence {
    /// Terminal status, absent when observation ended before one was available.
    pub terminal_status: Option<TerminalStatus>,
    /// Bounded stdout capture.
    pub stdout: CapturedStream,
    /// Bounded stderr capture.
    pub stderr: CapturedStream,
}

/// Exact observed output-artifact reference.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputArtifact {
    /// Caller-owned output role.
    pub role: String,
    /// Declared relative path beneath the invocation-owned staged root.
    pub path: String,
    /// Observed regular-file byte length.
    pub byte_length: u64,
    /// SHA-256 identity of the observed retained bytes.
    pub digest: ContentDigest,
    /// Sealed descriptor for the exact bytes represented by the metadata.
    #[serde(skip)]
    snapshot: Arc<File>,
}

impl OutputArtifact {
    /// Returns a reader for the immutable retained bytes represented by this artifact.
    ///
    /// # Errors
    ///
    /// Returns an I/O error when the retained descriptor cannot be duplicated or
    /// rewound. The producer pathname is never reopened.
    pub fn try_reader(&self) -> io::Result<File> {
        #[cfg(target_os = "linux")]
        let mut reader = File::open(descriptor_path(&self.snapshot))?;
        #[cfg(not(target_os = "linux"))]
        let mut reader = self.snapshot.try_clone()?;
        reader.seek(SeekFrom::Start(0))?;
        Ok(reader)
    }
}

impl PartialEq for OutputArtifact {
    fn eq(&self, other: &Self) -> bool {
        self.role == other.role
            && self.path == other.path
            && self.byte_length == other.byte_length
            && self.digest == other.digest
    }
}

impl Eq for OutputArtifact {}

/// A caller-owned typed decoder for one exact response binding.
pub trait ProducerResponseAdapter {
    /// Consumer-owned typed observation.
    type Observation: Serialize;

    /// Returns the exact response protocol and adapter implementation binding.
    fn binding(&self) -> &ResponseBinding;

    /// Converts one admitted bounded terminal response into a typed observation.
    ///
    /// # Errors
    ///
    /// Returns [`MalformedResponse`] when the evidence does not conform to the
    /// exact response contract implemented by this adapter.
    fn decode(
        &self,
        process: &ProcessEvidence,
        artifacts: &[OutputArtifact],
    ) -> Result<Self::Observation, MalformedResponse>;
}

/// Typed rejection by a caller-owned response adapter.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("producer response is malformed for the declared binding")]
pub struct MalformedResponse;

/// Identity-bearing request refusal decided before producer launch.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionRefusal {
    /// The retained executable bytes differ from the request.
    ExecutableIdentity,
    /// The capability root cannot be safely opened.
    CapabilityRoot,
    /// An input is missing, linked, unreadable, over-budget or has wrong bytes.
    Input,
    /// An output parent cannot be admitted without following links.
    Output,
    /// The request and runtime adapter differ.
    AdapterBinding,
    /// The request and runtime cancellation authority/event differ.
    CancellationBinding,
    /// The executor has no available concurrency slot.
    Concurrency,
}

/// Process failure categories not interpreted from diagnostic prose.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ExecutionFailure {
    /// The process exited normally with a code the response binding rejects.
    ExitCode(i32),
    /// The direct process terminated from a signal.
    Signal(i32),
    /// Stdout exceeded its exact request ceiling.
    StdoutTooLarge,
    /// Stderr exceeded its exact request ceiling.
    StderrTooLarge,
    /// The bounded process observation failed.
    Observation,
    /// A required or bounded output artifact could not be observed safely.
    OutputArtifact,
}

/// Closed execution state. Only [`Self::Completed`] carries `T`.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProducerExecutionState<T> {
    /// The selected executable could not be opened or launched.
    Unavailable,
    /// An identity-bearing request was refused before producer launch.
    Refused {
        /// Closed pre-launch refusal category.
        reason: ExecutionRefusal,
    },
    /// A launched process failed before a response could be accepted.
    Failed {
        /// Closed process failure category.
        reason: ExecutionFailure,
    },
    /// Monotonic execution time reached the request deadline.
    TimedOut,
    /// The exact caller-owned adapter rejected the bounded response.
    MalformedResponse,
    /// The selected confinement profile was unavailable or breached.
    ContainmentFailure,
    /// The exact bound caller cancellation event terminated the process.
    Cancelled,
    /// The caller-owned adapter produced a typed domain observation.
    Completed {
        /// Consumer-owned observation returned by the bound adapter.
        observation: T,
    },
}

/// Measured monotonic durations for one execution attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionTiming {
    /// Structure, identity and capability preflight duration.
    pub preflight_nanos: u64,
    /// Launched-process duration, absent when no launch occurred.
    pub execution_nanos: Option<u64>,
    /// Total executor-call duration.
    pub total_nanos: u64,
}

/// Identity-bound result of one structurally valid producer-execution attempt.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProducerExecutionResult<T> {
    /// Exact result contract discriminator.
    pub protocol: &'static str,
    /// RFC 8785 canonical identity of the complete request.
    pub request_identity: RequestIdentity,
    /// Exact caller-supplied producer provenance.
    pub producer: ProducerDescriptor,
    /// Observed retained executable identity when preflight reached it.
    pub observed_executable_digest: Option<ContentDigest>,
    /// Bound cancellation event when it was observed.
    pub cancellation_event: Option<CancellationBinding>,
    /// Measured monotonic durations.
    pub timing: ExecutionTiming,
    /// Bounded raw process evidence when a launch was observable.
    pub process: Option<ProcessEvidence>,
    /// Safely observed output artifact references.
    pub artifacts: Vec<OutputArtifact>,
    /// Closed execution state.
    pub state: ProducerExecutionState<T>,
}

impl<T: Serialize> ProducerExecutionResult<T> {
    /// Returns RFC 8785 canonical JSON bytes for every portable result field.
    ///
    /// The live descriptors retained by output artifacts are deliberately not
    /// encoded; their role, path, byte length and digest are encoded instead.
    ///
    /// # Errors
    ///
    /// Returns [`ResultEncodingError`] when the caller-owned observation or any
    /// result field cannot be represented as canonical JSON.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ResultEncodingError> {
        serde_json_canonicalizer::to_vec(self).map_err(|_| ResultEncodingError)
    }

    /// Computes the `sha256-jcs` identity of the canonical result bytes.
    ///
    /// # Errors
    ///
    /// Returns [`ResultEncodingError`] when canonical encoding fails. No result
    /// identity is returned in that case.
    pub fn identity(&self) -> Result<ResultIdentity, ResultEncodingError> {
        let canonical = self.canonical_bytes()?;
        Ok(ResultIdentity {
            scheme: RESULT_IDENTITY_SCHEME,
            digest: ContentDigest::of_bytes(&canonical),
        })
    }
}

/// RFC 8785 canonical identity of a complete portable execution result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultIdentity {
    /// Exact identity algorithm and canonicalization scheme.
    pub scheme: &'static str,
    /// SHA-256 digest of the canonical result bytes.
    pub digest: ContentDigest,
}

/// The complete portable result could not be represented canonically.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("producer execution result encoding failed")]
pub struct ResultEncodingError;

/// Construction failure for a shared executor.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ExecutorConfigurationError {
    /// The concurrency ceiling is zero or exceeds [`MAX_CONCURRENCY`].
    #[error("executor concurrency is outside one through {MAX_CONCURRENCY}")]
    Concurrency,
}

/// Shared bounded executor whose clones share one concurrency ceiling.
pub struct ProducerExecutor {
    maximum_concurrency: usize,
    active: Mutex<usize>,
}

impl ProducerExecutor {
    /// Creates an executor with a fixed concurrency ceiling.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutorConfigurationError::Concurrency`] when the ceiling is
    /// zero or exceeds [`MAX_CONCURRENCY`].
    pub fn new(maximum_concurrency: usize) -> Result<Self, ExecutorConfigurationError> {
        if !(1..=MAX_CONCURRENCY).contains(&maximum_concurrency) {
            return Err(ExecutorConfigurationError::Concurrency);
        }
        Ok(Self {
            maximum_concurrency,
            active: Mutex::new(0),
        })
    }

    /// Executes one exact request and returns a closed typed state.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidExecutionRequest`] without a result or request identity
    /// when the closed request structure is invalid.
    pub fn execute<A: ProducerResponseAdapter>(
        &self,
        request: &ProducerExecutionRequest,
        cancellation: &CancellationToken,
        adapter: &A,
    ) -> Result<ProducerExecutionResult<A::Observation>, InvalidExecutionRequest> {
        let call_started = monotonic_now();
        let request_identity = request.identity()?;
        let producer = request.producer.clone();

        if adapter.binding() != &request.response {
            return Ok(prelaunch_result(
                call_started,
                request_identity,
                producer,
                None,
                ProducerExecutionState::Refused {
                    reason: ExecutionRefusal::AdapterBinding,
                },
            ));
        }
        if cancellation.binding != request.cancellation {
            return Ok(prelaunch_result(
                call_started,
                request_identity,
                producer,
                None,
                ProducerExecutionState::Refused {
                    reason: ExecutionRefusal::CancellationBinding,
                },
            ));
        }
        if request.budget.max_concurrency != self.maximum_concurrency {
            return Ok(prelaunch_result(
                call_started,
                request_identity,
                producer,
                None,
                ProducerExecutionState::Refused {
                    reason: ExecutionRefusal::Concurrency,
                },
            ));
        }
        if cancellation.is_cancelled() {
            return Ok(prelaunch_result_with_cancellation(
                call_started,
                request_identity,
                producer,
                request.cancellation.clone(),
            ));
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = (adapter, cancellation);
            return Ok(prelaunch_result(
                call_started,
                request_identity,
                producer,
                None,
                ProducerExecutionState::ContainmentFailure,
            ));
        }

        #[cfg(target_os = "linux")]
        {
            Ok(self.execute_linux(
                request,
                cancellation,
                adapter,
                call_started,
                request_identity,
                producer,
            ))
        }
    }

    #[cfg(target_os = "linux")]
    fn execute_linux<A: ProducerResponseAdapter>(
        &self,
        request: &ProducerExecutionRequest,
        cancellation: &CancellationToken,
        adapter: &A,
        call_started: Instant,
        request_identity: RequestIdentity,
        producer: ProducerDescriptor,
    ) -> ProducerExecutionResult<A::Observation> {
        let validated = match validate_capabilities(request, cancellation) {
            Ok(value) => value,
            Err(failure) => {
                return preflight_failure_result(
                    call_started,
                    request_identity,
                    producer,
                    &request.cancellation,
                    failure,
                );
            }
        };
        let observed = Some(validated.executable_digest.clone());
        let _slot = match self.try_acquire() {
            Ok(slot) => slot,
            Err(failure) => {
                return acquisition_failure_result(
                    call_started,
                    request_identity,
                    producer,
                    observed,
                    failure,
                );
            }
        };
        let preflight_nanos = elapsed_nanos(call_started);
        let launched = monotonic_now();
        let outcome = run_process(request, &validated, cancellation);
        let execution_nanos = elapsed_nanos(launched);
        let (state, process, artifacts, cancellation_event) =
            finish_outcome(request, adapter, outcome);
        ProducerExecutionResult {
            protocol: PRODUCER_EXECUTION_RESULT_PROTOCOL,
            request_identity,
            producer,
            observed_executable_digest: observed,
            cancellation_event,
            timing: ExecutionTiming {
                preflight_nanos,
                execution_nanos: Some(execution_nanos),
                total_nanos: elapsed_nanos(call_started),
            },
            process,
            artifacts,
            state,
        }
    }

    fn try_acquire(&self) -> Result<ExecutionSlot<'_>, AcquireFailure> {
        let mut active = self.active.lock().map_err(|_| AcquireFailure::Poisoned)?;
        if *active >= self.maximum_concurrency {
            return Err(AcquireFailure::Full);
        }
        *active = active.saturating_add(1);
        Ok(ExecutionSlot { executor: self })
    }
}

struct ExecutionSlot<'a> {
    executor: &'a ProducerExecutor,
}

impl Drop for ExecutionSlot<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.executor.active.lock() {
            *active = active.saturating_sub(1);
        }
    }
}

#[derive(Clone, Copy)]
enum AcquireFailure {
    Full,
    Poisoned,
}

#[cfg(target_os = "linux")]
struct ValidatedExecution {
    executable: File,
    _working_tree: tempfile::TempDir,
    root: File,
    inputs: Vec<ValidatedInput>,
    executable_digest: ContentDigest,
}

#[cfg(target_os = "linux")]
struct ValidatedInput {
    role: String,
    path: String,
    file: File,
}

#[cfg(target_os = "linux")]
enum PreflightFailure {
    Unavailable,
    Refused(ExecutionRefusal, Option<ContentDigest>),
    Cancelled(Option<ContentDigest>),
}

#[cfg(target_os = "linux")]
fn preflight_failure_result<T>(
    started: Instant,
    request_identity: RequestIdentity,
    producer: ProducerDescriptor,
    cancellation: &CancellationBinding,
    failure: PreflightFailure,
) -> ProducerExecutionResult<T> {
    match failure {
        PreflightFailure::Unavailable => prelaunch_result(
            started,
            request_identity,
            producer,
            None,
            ProducerExecutionState::Unavailable,
        ),
        PreflightFailure::Refused(reason, observed) => prelaunch_result(
            started,
            request_identity,
            producer,
            observed,
            ProducerExecutionState::Refused { reason },
        ),
        PreflightFailure::Cancelled(observed) => prelaunch_result_with_observed_cancellation(
            started,
            request_identity,
            producer,
            observed,
            cancellation.clone(),
        ),
    }
}

#[cfg(target_os = "linux")]
fn acquisition_failure_result<T>(
    started: Instant,
    request_identity: RequestIdentity,
    producer: ProducerDescriptor,
    observed: Option<ContentDigest>,
    failure: AcquireFailure,
) -> ProducerExecutionResult<T> {
    let state = match failure {
        AcquireFailure::Full => ProducerExecutionState::Refused {
            reason: ExecutionRefusal::Concurrency,
        },
        AcquireFailure::Poisoned => ProducerExecutionState::ContainmentFailure,
    };
    prelaunch_result(started, request_identity, producer, observed, state)
}

#[cfg(target_os = "linux")]
struct ProcessOutcome {
    conclusion: ProcessConclusion,
    evidence: Option<ProcessEvidence>,
    artifacts: Vec<OutputArtifact>,
}

#[cfg(target_os = "linux")]
struct CapturedProcessOutcome {
    conclusion: ProcessConclusion,
    evidence: Option<ProcessEvidence>,
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
enum ProcessConclusion {
    Unavailable,
    Failed(ExecutionFailure),
    TimedOut,
    ContainmentFailure,
    Cancelled,
    Terminal(TerminalStatus),
}

#[cfg(unix)]
#[derive(Clone, Copy)]
struct KernelBudget {
    timeout: Duration,
    max_stdout_bytes: usize,
    max_stderr_bytes: usize,
}

#[cfg(unix)]
#[derive(Clone, Copy)]
enum DescendantPolicy {
    ProcessGroup,
    Observed { maximum: usize },
}

#[cfg(unix)]
enum KernelConclusion {
    Unavailable(String),
    PipeUnavailable(&'static str),
    Observation(String),
    OutputUnreadable {
        stream: &'static str,
        detail: String,
    },
    TimedOut,
    ContainmentFailure,
    Cancelled,
    Terminal(ExitStatus),
}

#[cfg(unix)]
struct KernelCapture {
    conclusion: KernelConclusion,
    terminal_status: Option<ExitStatus>,
    stdout: Option<CapturedBytes>,
    stderr: Option<CapturedBytes>,
}

#[cfg(unix)]
impl KernelCapture {
    fn without_evidence(conclusion: KernelConclusion) -> Self {
        Self::without_streams(conclusion, None)
    }

    fn without_streams(conclusion: KernelConclusion, terminal_status: Option<ExitStatus>) -> Self {
        Self {
            conclusion,
            terminal_status,
            stdout: None,
            stderr: None,
        }
    }
}

#[cfg(target_os = "linux")]
fn finish_outcome<A: ProducerResponseAdapter>(
    request: &ProducerExecutionRequest,
    adapter: &A,
    outcome: ProcessOutcome,
) -> (
    ProducerExecutionState<A::Observation>,
    Option<ProcessEvidence>,
    Vec<OutputArtifact>,
    Option<CancellationBinding>,
) {
    let ProcessOutcome {
        conclusion,
        evidence,
        artifacts,
    } = outcome;
    let cancellation_event =
        matches!(&conclusion, ProcessConclusion::Cancelled).then(|| request.cancellation.clone());
    let state = match conclusion {
        ProcessConclusion::Unavailable => ProducerExecutionState::Unavailable,
        ProcessConclusion::Failed(reason) => ProducerExecutionState::Failed { reason },
        ProcessConclusion::TimedOut => ProducerExecutionState::TimedOut,
        ProcessConclusion::ContainmentFailure => ProducerExecutionState::ContainmentFailure,
        ProcessConclusion::Cancelled => ProducerExecutionState::Cancelled,
        ProcessConclusion::Terminal(TerminalStatus::ExitCode(code))
            if exit_code_accepted(&request.response.exit_codes, code) =>
        {
            match evidence.as_ref() {
                Some(process) => match adapter.decode(process, &artifacts) {
                    Ok(observation) => ProducerExecutionState::Completed { observation },
                    Err(MalformedResponse) => ProducerExecutionState::MalformedResponse,
                },
                None => ProducerExecutionState::Failed {
                    reason: ExecutionFailure::Observation,
                },
            }
        }
        ProcessConclusion::Terminal(TerminalStatus::ExitCode(code)) => {
            ProducerExecutionState::Failed {
                reason: ExecutionFailure::ExitCode(code),
            }
        }
        ProcessConclusion::Terminal(TerminalStatus::Signal(signal)) => {
            ProducerExecutionState::Failed {
                reason: ExecutionFailure::Signal(signal),
            }
        }
    };
    (state, evidence, artifacts, cancellation_event)
}

fn validate_structure(request: &ProducerExecutionRequest) -> Result<(), InvalidExecutionRequest> {
    if request.protocol != PRODUCER_EXECUTION_REQUEST_PROTOCOL {
        return Err(InvalidExecutionRequest::Protocol);
    }
    validate_contract(&request.caller)?;
    validate_contract(&request.response.protocol)?;
    validate_contract(&request.response.adapter)?;
    let confinement_contract = match &request.containment {
        ContainmentBinding::ProcessGroupV1 { contract } => contract,
    };
    validate_contract(confinement_contract)?;
    for value in [
        &request.producer.name,
        &request.producer.version,
        &request.producer.source_revision,
    ] {
        validate_text(value).map_err(|()| InvalidExecutionRequest::Provenance)?;
    }
    validate_absolute_normal_path(&request.capability_root)?;
    validate_absolute_normal_path(&request.producer.executable)?;
    validate_budget(request.budget)?;
    if matches!(
        &request.response.exit_codes,
        ExitCodeBinding::Exact(codes)
            if codes.is_empty()
                || codes.len() > 256
                || codes.iter().any(|code| !(0..=255).contains(code))
    ) {
        return Err(InvalidExecutionRequest::Contract);
    }
    if request.environment.len() > MAX_ENVIRONMENT
        || request.environment.iter().any(|(name, value)| {
            name.is_empty()
                || name.contains(['=', '\0'])
                || name.len() > MAX_TEXT_BYTES
                || value.contains('\0')
                || value.len() > MAX_TEXT_BYTES
        })
    {
        return Err(InvalidExecutionRequest::Environment);
    }
    if request.inputs.len() > MAX_ARTIFACTS {
        return Err(InvalidExecutionRequest::Input);
    }
    if request.outputs.len() > request.budget.max_output_artifacts
        || request.outputs.len() > MAX_ARTIFACTS
    {
        return Err(InvalidExecutionRequest::Output);
    }
    let mut input_roles = BTreeSet::new();
    for input in &request.inputs {
        if validate_text(&input.role).is_err()
            || validate_relative_normal_path(&input.path).is_err()
            || !input_roles.insert(input.role.as_str())
        {
            return Err(InvalidExecutionRequest::Input);
        }
    }
    let mut output_roles = BTreeSet::new();
    let mut output_paths = BTreeSet::new();
    for output in &request.outputs {
        if validate_text(&output.role).is_err()
            || validate_relative_normal_path(&output.path).is_err()
            || !output_roles.insert(output.role.as_str())
            || !output_paths.insert(output.path.as_str())
        {
            return Err(InvalidExecutionRequest::Output);
        }
    }
    if request.arguments.len() > MAX_ARGUMENTS {
        return Err(InvalidExecutionRequest::Argument);
    }
    for argument in &request.arguments {
        let valid = match argument {
            ArgumentBinding::Literal { value } => validate_text(value).is_ok(),
            ArgumentBinding::InputArtifact { role } => input_roles.contains(role.as_str()),
            ArgumentBinding::OutputArtifact { role } => output_roles.contains(role.as_str()),
        };
        if !valid {
            return Err(InvalidExecutionRequest::Argument);
        }
    }
    if let StdinBinding::InputArtifact { role } = &request.stdin
        && !input_roles.contains(role.as_str())
    {
        return Err(InvalidExecutionRequest::Stdin);
    }
    match &request.cancellation {
        CancellationBinding::Disabled => {}
        CancellationBinding::Event {
            authority,
            event_id,
        } => {
            validate_text(authority).map_err(|()| InvalidExecutionRequest::Cancellation)?;
            validate_text(event_id).map_err(|()| InvalidExecutionRequest::Cancellation)?;
        }
    }
    Ok(())
}

fn validate_contract(binding: &ContractBinding) -> Result<(), InvalidExecutionRequest> {
    for value in [&binding.kind, &binding.version, &binding.revision] {
        validate_text(value).map_err(|()| InvalidExecutionRequest::Contract)?;
    }
    Ok(())
}

fn validate_budget(budget: ExecutionBudget) -> Result<(), InvalidExecutionRequest> {
    if budget.timeout_millis == 0
        || budget.timeout_millis > MAX_TIMEOUT_MILLIS
        || budget.max_stdout_bytes == 0
        || budget.max_stdout_bytes > MAX_STREAM_BYTES
        || budget.max_stderr_bytes == 0
        || budget.max_stderr_bytes > MAX_STREAM_BYTES
        || budget.max_input_bytes > MAX_INPUT_BYTES
        || budget.max_output_artifacts > MAX_ARTIFACTS
        || budget.max_output_bytes > MAX_OUTPUT_BYTES
        || !(1..=MAX_DESCENDANTS).contains(&budget.max_descendants)
        || !(1..=MAX_CONCURRENCY).contains(&budget.max_concurrency)
    {
        return Err(InvalidExecutionRequest::Budget);
    }
    Ok(())
}

fn validate_text(value: &str) -> Result<(), ()> {
    if value.is_empty() || value.len() > MAX_TEXT_BYTES || value.contains('\0') {
        return Err(());
    }
    Ok(())
}

fn validate_absolute_normal_path(value: &str) -> Result<(), InvalidExecutionRequest> {
    let path = Path::new(value);
    if !path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::CurDir | Component::Prefix(_)
            )
        })
    {
        return Err(InvalidExecutionRequest::Path);
    }
    Ok(())
}

fn validate_relative_normal_path(value: &str) -> Result<(), InvalidExecutionRequest> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(InvalidExecutionRequest::Path);
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn validate_capabilities(
    request: &ProducerExecutionRequest,
    cancellation: &CancellationToken,
) -> Result<ValidatedExecution, PreflightFailure> {
    let root = open_capability_root(&request.capability_root)?;
    let (executable, executable_digest) = open_executable(request, cancellation)?;
    let inputs = open_inputs(request, &root, &executable_digest, cancellation)?;
    let (working_tree, staged_root) =
        stage_working_projection(request, &inputs, &executable_digest, cancellation)?;
    Ok(ValidatedExecution {
        executable,
        _working_tree: working_tree,
        root: staged_root,
        inputs,
        executable_digest,
    })
}

#[cfg(target_os = "linux")]
fn open_capability_root(path: &str) -> Result<File, PreflightFailure> {
    use rustix::fs::{Mode, OFlags, ResolveFlags, openat2};

    let slash = File::open("/")
        .map_err(|_| PreflightFailure::Refused(ExecutionRefusal::CapabilityRoot, None))?;
    let root_relative = path.trim_start_matches('/');
    let root_fd = openat2(
        &slash,
        root_relative,
        OFlags::PATH | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS | ResolveFlags::NO_SYMLINKS,
    )
    .map_err(|_| PreflightFailure::Refused(ExecutionRefusal::CapabilityRoot, None))?;
    Ok(File::from(root_fd))
}

#[cfg(target_os = "linux")]
fn open_executable(
    request: &ProducerExecutionRequest,
    cancellation: &CancellationToken,
) -> Result<(File, ContentDigest), PreflightFailure> {
    use rustix::fs::{Mode, OFlags, ResolveFlags, openat2};

    let slash = File::open("/").map_err(|_| PreflightFailure::Unavailable)?;
    let executable_relative = request.producer.executable.trim_start_matches('/');
    let executable_fd = openat2(
        &slash,
        executable_relative,
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS | ResolveFlags::NO_SYMLINKS,
    )
    .map_err(|_| PreflightFailure::Unavailable)?;
    let mut selected = File::from(executable_fd);
    let executable_metadata = selected
        .metadata()
        .map_err(|_| PreflightFailure::Unavailable)?;
    if !executable_metadata.is_file() || executable_metadata.len() > MAX_EXECUTABLE_BYTES {
        return Err(PreflightFailure::Refused(
            ExecutionRefusal::ExecutableIdentity,
            None,
        ));
    }
    let (executable, executable_digest) = snapshot_reader(
        &mut selected,
        MAX_EXECUTABLE_BYTES,
        cancellation,
        "producer-executable",
    )
    .map_err(|failure| match failure {
        DigestOutcome::Cancelled => PreflightFailure::Cancelled(None),
        DigestOutcome::Unreadable | DigestOutcome::TooLarge => {
            PreflightFailure::Refused(ExecutionRefusal::ExecutableIdentity, None)
        }
    })?;
    if executable_digest != request.producer.executable_digest {
        return Err(PreflightFailure::Refused(
            ExecutionRefusal::ExecutableIdentity,
            Some(executable_digest),
        ));
    }
    Ok((executable, executable_digest))
}

#[cfg(target_os = "linux")]
fn open_inputs(
    request: &ProducerExecutionRequest,
    root: &File,
    executable_digest: &ContentDigest,
    cancellation: &CancellationToken,
) -> Result<Vec<ValidatedInput>, PreflightFailure> {
    let mut remaining = request.budget.max_input_bytes;
    let mut inputs = Vec::with_capacity(request.inputs.len());
    for input in &request.inputs {
        if cancellation.is_cancelled() {
            return Err(PreflightFailure::Cancelled(Some(executable_digest.clone())));
        }
        let file = open_input(input, root, executable_digest, cancellation, &mut remaining)?;
        inputs.push(ValidatedInput {
            role: input.role.clone(),
            path: input.path.clone(),
            file,
        });
    }
    Ok(inputs)
}

#[cfg(target_os = "linux")]
fn open_input(
    input: &InputBinding,
    root: &File,
    executable_digest: &ContentDigest,
    cancellation: &CancellationToken,
    remaining: &mut u64,
) -> Result<File, PreflightFailure> {
    use rustix::fs::{Mode, OFlags, ResolveFlags, openat2};

    let refused =
        || PreflightFailure::Refused(ExecutionRefusal::Input, Some(executable_digest.clone()));
    let fd = openat2(
        root,
        &input.path,
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS | ResolveFlags::NO_SYMLINKS,
    )
    .map_err(|_| refused())?;
    let mut selected = File::from(fd);
    let metadata = selected.metadata().map_err(|_| refused())?;
    if !metadata.is_file() {
        return Err(refused());
    }
    *remaining = remaining.checked_sub(metadata.len()).ok_or_else(refused)?;
    let (file, observed) = snapshot_reader(
        &mut selected,
        metadata.len(),
        cancellation,
        "producer-input",
    )
    .map_err(|failure| match failure {
        DigestOutcome::Cancelled => PreflightFailure::Cancelled(Some(executable_digest.clone())),
        DigestOutcome::Unreadable | DigestOutcome::TooLarge => refused(),
    })?;
    if observed != input.digest {
        return Err(refused());
    }
    Ok(file)
}

#[cfg(target_os = "linux")]
fn snapshot_reader(
    source: &mut impl Read,
    maximum: u64,
    cancellation: &CancellationToken,
    name: &str,
) -> Result<(File, ContentDigest), DigestOutcome> {
    use rustix::fs::{MemfdFlags, SealFlags, fcntl_add_seals, memfd_create};

    let descriptor = memfd_create(name, MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING)
        .map_err(|_| DigestOutcome::Unreadable)?;
    let mut snapshot = File::from(descriptor);
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        if cancellation.is_cancelled() {
            return Err(DigestOutcome::Cancelled);
        }
        let read = source
            .read(&mut buffer)
            .map_err(|_| DigestOutcome::Unreadable)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(u64::try_from(read).map_err(|_| DigestOutcome::TooLarge)?)
            .ok_or(DigestOutcome::TooLarge)?;
        if total > maximum {
            return Err(DigestOutcome::TooLarge);
        }
        hasher.update(&buffer[..read]);
        snapshot
            .write_all(&buffer[..read])
            .map_err(|_| DigestOutcome::Unreadable)?;
    }
    fcntl_add_seals(
        &snapshot,
        SealFlags::SEAL | SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE,
    )
    .map_err(|_| DigestOutcome::Unreadable)?;
    snapshot
        .seek(SeekFrom::Start(0))
        .map_err(|_| DigestOutcome::Unreadable)?;
    Ok((
        snapshot,
        ContentDigest(hex_digest(hasher.finalize().as_slice()).into()),
    ))
}

#[cfg(target_os = "linux")]
fn stage_working_projection(
    request: &ProducerExecutionRequest,
    inputs: &[ValidatedInput],
    executable_digest: &ContentDigest,
    cancellation: &CancellationToken,
) -> Result<(tempfile::TempDir, File), PreflightFailure> {
    use std::{fs::OpenOptions, os::unix::fs::PermissionsExt};

    let refused =
        || PreflightFailure::Refused(ExecutionRefusal::Input, Some(executable_digest.clone()));
    let working_tree = tempfile::tempdir().map_err(|_| refused())?;
    for input in inputs {
        if cancellation.is_cancelled() {
            return Err(PreflightFailure::Cancelled(Some(executable_digest.clone())));
        }
        let destination = working_tree.path().join(&input.path);
        let parent = destination.parent().ok_or_else(refused)?;
        std::fs::create_dir_all(parent).map_err(|_| refused())?;
        let mut source = File::open(descriptor_path(&input.file)).map_err(|_| refused())?;
        source.seek(SeekFrom::Start(0)).map_err(|_| refused())?;
        let mut staged = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|_| refused())?;
        io::copy(&mut source, &mut staged).map_err(|_| refused())?;
        staged.flush().map_err(|_| refused())?;
        std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o400))
            .map_err(|_| refused())?;
    }

    for output in &request.outputs {
        if cancellation.is_cancelled() {
            return Err(PreflightFailure::Cancelled(Some(executable_digest.clone())));
        }
        let parent = Path::new(&output.path)
            .parent()
            .unwrap_or_else(|| Path::new("."));
        if !parent.as_os_str().is_empty() && parent != Path::new(".") {
            std::fs::create_dir_all(working_tree.path().join(parent)).map_err(|_| {
                PreflightFailure::Refused(ExecutionRefusal::Output, Some(executable_digest.clone()))
            })?;
        }
    }
    for output in &request.outputs {
        let destination = working_tree.path().join(&output.path);
        if destination.exists() {
            return Err(PreflightFailure::Refused(
                ExecutionRefusal::Output,
                Some(executable_digest.clone()),
            ));
        }
    }
    let root = File::open(working_tree.path()).map_err(|_| {
        PreflightFailure::Refused(
            ExecutionRefusal::CapabilityRoot,
            Some(executable_digest.clone()),
        )
    })?;
    Ok((working_tree, root))
}

#[cfg(target_os = "linux")]
fn run_process(
    request: &ProducerExecutionRequest,
    validated: &ValidatedExecution,
    cancellation: &CancellationToken,
) -> ProcessOutcome {
    let Some(command) = build_command(request, validated) else {
        return ProcessOutcome {
            conclusion: ProcessConclusion::Failed(ExecutionFailure::Observation),
            evidence: None,
            artifacts: Vec::new(),
        };
    };
    let captured = capture_process(command, request.budget, cancellation);
    let (artifacts, artifact_failed) = observe_outputs(request, &validated.root, cancellation);
    let conclusion = qualify_captured_process(&captured, artifact_failed);
    ProcessOutcome {
        conclusion,
        evidence: captured.evidence,
        artifacts,
    }
}

#[cfg(target_os = "linux")]
fn build_command(
    request: &ProducerExecutionRequest,
    validated: &ValidatedExecution,
) -> Option<Command> {
    let arguments = resolve_arguments(request, validated)?;
    let stdin = resolve_stdin(request, validated)?;
    let mut command = Command::new(descriptor_path(&validated.executable));
    command
        .args(arguments)
        .current_dir(descriptor_path(&validated.root))
        .env_clear()
        .envs(&request.environment)
        .stdin(stdin)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    Some(command)
}

#[cfg(target_os = "linux")]
fn resolve_arguments(
    request: &ProducerExecutionRequest,
    validated: &ValidatedExecution,
) -> Option<Vec<String>> {
    request
        .arguments
        .iter()
        .map(|argument| match argument {
            ArgumentBinding::Literal { value } => Some(value.clone()),
            ArgumentBinding::InputArtifact { role } => validated
                .inputs
                .iter()
                .find(|input| input.role == *role)
                .map(|input| descriptor_path(&input.file)),
            ArgumentBinding::OutputArtifact { role } => request
                .outputs
                .iter()
                .find(|output| output.role == *role)
                .map(|output| output.path.clone()),
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn resolve_stdin(
    request: &ProducerExecutionRequest,
    validated: &ValidatedExecution,
) -> Option<Stdio> {
    match &request.stdin {
        StdinBinding::Null => Some(Stdio::null()),
        StdinBinding::InputArtifact { role } => validated
            .inputs
            .iter()
            .find(|input| input.role == *role)
            .and_then(|input| input.file.try_clone().ok())
            .map(Stdio::from),
    }
}

#[cfg(target_os = "linux")]
fn descriptor_path(file: &File) -> String {
    format!("/proc/{}/fd/{}", std::process::id(), file.as_raw_fd())
}

#[cfg(target_os = "linux")]
fn capture_process(
    command: Command,
    budget: ExecutionBudget,
    cancellation: &CancellationToken,
) -> CapturedProcessOutcome {
    let captured = capture_command(
        command,
        KernelBudget {
            timeout: Duration::from_millis(budget.timeout_millis),
            max_stdout_bytes: budget.max_stdout_bytes,
            max_stderr_bytes: budget.max_stderr_bytes,
        },
        DescendantPolicy::Observed {
            maximum: budget.max_descendants,
        },
        Some(cancellation),
    );
    let KernelCapture {
        conclusion,
        terminal_status,
        stdout,
        stderr,
    } = captured;
    let Some((stdout, stderr)) = stdout.zip(stderr) else {
        return CapturedProcessOutcome {
            conclusion: match conclusion {
                KernelConclusion::Unavailable(_) => ProcessConclusion::Unavailable,
                KernelConclusion::ContainmentFailure | KernelConclusion::PipeUnavailable(_) => {
                    ProcessConclusion::ContainmentFailure
                }
                KernelConclusion::Observation(_)
                | KernelConclusion::OutputUnreadable { .. }
                | KernelConclusion::Terminal(_) => {
                    ProcessConclusion::Failed(ExecutionFailure::Observation)
                }
                KernelConclusion::TimedOut => ProcessConclusion::TimedOut,
                KernelConclusion::Cancelled => ProcessConclusion::Cancelled,
            },
            evidence: None,
        };
    };
    CapturedProcessOutcome {
        evidence: Some(ProcessEvidence {
            terminal_status: terminal_status.map(status_to_terminal),
            stdout: captured_stream(stdout),
            stderr: captured_stream(stderr),
        }),
        conclusion: match conclusion {
            KernelConclusion::Unavailable(_) => ProcessConclusion::Unavailable,
            KernelConclusion::PipeUnavailable(_)
            | KernelConclusion::Observation(_)
            | KernelConclusion::OutputUnreadable { .. } => {
                ProcessConclusion::Failed(ExecutionFailure::Observation)
            }
            KernelConclusion::TimedOut => ProcessConclusion::TimedOut,
            KernelConclusion::ContainmentFailure => ProcessConclusion::ContainmentFailure,
            KernelConclusion::Cancelled => ProcessConclusion::Cancelled,
            KernelConclusion::Terminal(status) => terminal_conclusion(status),
        },
    }
}

#[cfg(unix)]
fn capture_command(
    mut command: Command,
    budget: KernelBudget,
    descendant_policy: DescendantPolicy,
    cancellation: Option<&CancellationToken>,
) -> KernelCapture {
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return KernelCapture::without_evidence(KernelConclusion::Unavailable(
                error.to_string(),
            ));
        }
    };
    let Some(stdout) = child.stdout.take() else {
        let status = terminate_group(&mut child, &BTreeSet::new());
        return KernelCapture::without_streams(KernelConclusion::PipeUnavailable("stdout"), status);
    };
    let Some(stderr) = child.stderr.take() else {
        let status = terminate_group(&mut child, &BTreeSet::new());
        return KernelCapture::without_streams(KernelConclusion::PipeUnavailable("stderr"), status);
    };
    let stdout_reader = bounded_reader(stdout, budget.max_stdout_bytes);
    let stderr_reader = bounded_reader(stderr, budget.max_stderr_bytes);
    let (conclusion, terminal_status) =
        supervise_child(&mut child, budget.timeout, descendant_policy, cancellation);
    let stdout = match join_reader(stdout_reader) {
        Ok(captured) => captured,
        Err(detail) => {
            return KernelCapture::without_evidence(KernelConclusion::OutputUnreadable {
                stream: "stdout",
                detail,
            });
        }
    };
    let stderr = match join_reader(stderr_reader) {
        Ok(captured) => captured,
        Err(detail) => {
            return KernelCapture::without_evidence(KernelConclusion::OutputUnreadable {
                stream: "stderr",
                detail,
            });
        }
    };
    KernelCapture {
        conclusion,
        terminal_status,
        stdout: Some(stdout),
        stderr: Some(stderr),
    }
}

#[cfg(unix)]
fn supervise_child(
    child: &mut std::process::Child,
    timeout: Duration,
    descendant_policy: DescendantPolicy,
    cancellation: Option<&CancellationToken>,
) -> (KernelConclusion, Option<ExitStatus>) {
    let started = Instant::now();
    let mut tracked_descendants = BTreeSet::new();
    loop {
        if cancellation.is_some_and(CancellationToken::is_cancelled) {
            let status = terminate_group(child, &tracked_descendants);
            return (KernelConclusion::Cancelled, status);
        }
        if let DescendantPolicy::Observed { maximum } = descendant_policy {
            #[cfg(not(target_os = "linux"))]
            {
                let _ = maximum;
                let status = terminate_group(child, &tracked_descendants);
                return (KernelConclusion::ContainmentFailure, status);
            }
            #[cfg(target_os = "linux")]
            {
                let Ok(observation) = inspect_descendants(child.id(), maximum) else {
                    let status = terminate_group(child, &tracked_descendants);
                    return (KernelConclusion::ContainmentFailure, status);
                };
                tracked_descendants.extend(observation.descendants);
                if observation.escaped {
                    let status = terminate_group(child, &tracked_descendants);
                    return (KernelConclusion::ContainmentFailure, status);
                }
            }
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                terminate_process_group(child.id());
                return (KernelConclusion::Terminal(status), Some(status));
            }
            Ok(None) if started.elapsed() < timeout => thread::sleep(Duration::from_millis(1)),
            Ok(None) => {
                let status = terminate_group(child, &tracked_descendants);
                return (KernelConclusion::TimedOut, status);
            }
            Err(error) => {
                let status = terminate_group(child, &tracked_descendants);
                return (KernelConclusion::Observation(error.to_string()), status);
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn qualify_captured_process(
    captured: &CapturedProcessOutcome,
    artifact_failed: bool,
) -> ProcessConclusion {
    let Some(evidence) = &captured.evidence else {
        return match captured.conclusion {
            ProcessConclusion::Unavailable => ProcessConclusion::Unavailable,
            _ => ProcessConclusion::Failed(ExecutionFailure::Observation),
        };
    };
    if evidence.stdout.truncated {
        return ProcessConclusion::Failed(ExecutionFailure::StdoutTooLarge);
    }
    if evidence.stderr.truncated {
        return ProcessConclusion::Failed(ExecutionFailure::StderrTooLarge);
    }
    if artifact_failed && matches!(captured.conclusion, ProcessConclusion::Terminal(_)) {
        return ProcessConclusion::Failed(ExecutionFailure::OutputArtifact);
    }
    captured.conclusion
}

#[cfg(target_os = "linux")]
fn terminal_conclusion(status: ExitStatus) -> ProcessConclusion {
    ProcessConclusion::Terminal(status_to_terminal(status))
}

#[cfg(target_os = "linux")]
fn observe_outputs(
    request: &ProducerExecutionRequest,
    root: &File,
    cancellation: &CancellationToken,
) -> (Vec<OutputArtifact>, bool) {
    use rustix::fs::{Mode, OFlags, ResolveFlags, openat2};

    let mut artifacts = Vec::new();
    let mut remaining = request.budget.max_output_bytes;
    for output in &request.outputs {
        let descriptor = match openat2(
            root,
            &output.path,
            OFlags::RDONLY | OFlags::CLOEXEC,
            Mode::empty(),
            ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS | ResolveFlags::NO_SYMLINKS,
        ) {
            Ok(value) => value,
            Err(_) if !output.required => continue,
            Err(_) => return (artifacts, true),
        };
        let mut file = File::from(descriptor);
        let Ok(metadata) = file.metadata() else {
            return (artifacts, true);
        };
        if !metadata.is_file() {
            return (artifacts, true);
        }
        let Ok((snapshot, digest)) =
            snapshot_reader(&mut file, remaining, cancellation, "producer-output")
        else {
            return (artifacts, true);
        };
        let Ok(snapshot_metadata) = snapshot.metadata() else {
            return (artifacts, true);
        };
        let Some(next_remaining) = remaining.checked_sub(snapshot_metadata.len()) else {
            return (artifacts, true);
        };
        remaining = next_remaining;
        artifacts.push(OutputArtifact {
            role: output.role.clone(),
            path: output.path.clone(),
            byte_length: snapshot_metadata.len(),
            digest,
            snapshot: Arc::new(snapshot),
        });
    }
    (artifacts, false)
}

#[cfg(target_os = "linux")]
struct DescendantObservation {
    descendants: BTreeSet<i32>,
    escaped: bool,
}

#[cfg(target_os = "linux")]
fn inspect_descendants(root: u32, maximum: usize) -> Result<DescendantObservation, ()> {
    let root_raw = i32::try_from(root).map_err(|_| ())?;
    let Some(root_pid) = rustix::process::Pid::from_raw(root_raw) else {
        return Err(());
    };
    let mut pending = vec![root_raw];
    let mut descendants = BTreeSet::new();
    let mut escaped = false;
    while let Some(parent) = pending.pop() {
        let path = format!("/proc/{parent}/task/{parent}/children");
        let children = match std::fs::read_to_string(path) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound && parent != root_raw => continue,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(DescendantObservation {
                    descendants,
                    escaped,
                });
            }
            Err(_) => return Err(()),
        };
        for child in children.split_whitespace() {
            let Ok(raw) = child.parse::<i32>() else {
                return Err(());
            };
            if !descendants.insert(raw) {
                continue;
            }
            if descendants.len() > maximum {
                return Err(());
            }
            pending.push(raw);
            if let Some(pid) = rustix::process::Pid::from_raw(raw)
                && let Ok(group) = rustix::process::getpgid(Some(pid))
                && group != root_pid
            {
                escaped = true;
            }
        }
    }
    Ok(DescendantObservation {
        descendants,
        escaped,
    })
}

#[cfg(unix)]
fn terminate_group(
    child: &mut std::process::Child,
    descendants: &BTreeSet<i32>,
) -> Option<ExitStatus> {
    terminate_process_group(child.id());
    for raw in descendants {
        if let Some(pid) = rustix::process::Pid::from_raw(*raw) {
            let _ = rustix::process::kill_process(pid, rustix::process::Signal::KILL);
        }
    }
    let _ = child.kill();
    child.wait().ok()
}

#[cfg(target_os = "linux")]
fn status_to_terminal(status: ExitStatus) -> TerminalStatus {
    status.code().map_or_else(
        || TerminalStatus::Signal(status.signal().unwrap_or(0)),
        TerminalStatus::ExitCode,
    )
}

#[cfg(unix)]
fn terminate_process_group(id: u32) {
    let Ok(raw) = i32::try_from(id) else {
        return;
    };
    let Some(group) = rustix::process::Pid::from_raw(raw) else {
        return;
    };
    let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
}

#[cfg(unix)]
fn bounded_reader<R>(reader: R, maximum: usize) -> thread::JoinHandle<io::Result<CapturedBytes>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let limit = u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1);
        let mut bytes = Vec::new();
        reader.take(limit).read_to_end(&mut bytes)?;
        let truncated = bytes.len() > maximum;
        bytes.truncate(maximum);
        Ok(CapturedBytes { bytes, truncated })
    })
}

#[cfg(unix)]
fn join_reader(
    handle: thread::JoinHandle<io::Result<CapturedBytes>>,
) -> Result<CapturedBytes, String> {
    handle
        .join()
        .map_err(|_| "reader thread panicked".to_owned())?
        .map_err(|error| error.to_string())
}

#[cfg(unix)]
struct CapturedBytes {
    bytes: Vec<u8>,
    truncated: bool,
}

#[cfg(target_os = "linux")]
fn captured_stream(captured: CapturedBytes) -> CapturedStream {
    CapturedStream {
        digest: ContentDigest::of_bytes(&captured.bytes),
        bytes: captured.bytes,
        truncated: captured.truncated,
    }
}

/// Unstable package-internal bridge for EA's existing binary-only adapters.
///
/// This API is public only because Cargo builds the package library and binary
/// as separate crates. New external consumers should use [`ProducerExecutor`]
/// and the closed producer-execution protocol instead. No compatibility is
/// promised for anything beneath this namespace.
#[doc(hidden)]
pub mod __private {
    use std::{ffi::OsStr, path::Path, process::ExitStatus, time::Duration};

    use thiserror::Error;

    /// Timeout and per-stream capture limits for a legacy host invocation.
    #[derive(Clone, Copy)]
    pub struct ProcessLimits {
        /// Maximum wall-clock duration.
        pub timeout: Duration,
        /// Maximum retained bytes for each output stream.
        pub max_output_bytes: usize,
    }

    /// Terminal result returned to an existing binary adapter.
    pub struct CompletedProcess {
        /// Direct child terminal status.
        pub status: ExitStatus,
        /// Bounded standard output.
        pub stdout: Vec<u8>,
        /// Bounded standard error.
        pub stderr: Vec<u8>,
    }

    /// Process-mechanics failure returned to an existing binary adapter.
    #[derive(Debug, Error)]
    pub enum ProcessError {
        /// The selected child could not be launched on this host.
        #[error("child process is unavailable: {detail}")]
        Unavailable {
            /// Host diagnostic retained for the adapter's existing mapping.
            detail: String,
        },
        /// A configured output pipe was unavailable after launch.
        #[error("child process {stream} pipe is unavailable")]
        PipeUnavailable {
            /// Missing stream name.
            stream: &'static str,
        },
        /// The child could not be observed or reaped.
        #[error("cannot observe child process: {detail}")]
        Observation {
            /// Host diagnostic retained for the adapter's existing mapping.
            detail: String,
        },
        /// The child exceeded its wall-clock limit.
        #[error("child process exceeded the {timeout:?} time limit")]
        TimedOut {
            /// Configured wall-clock limit.
            timeout: Duration,
        },
        /// One captured stream could not be read.
        #[error("cannot read child process {stream}: {detail}")]
        OutputUnreadable {
            /// Unreadable stream name.
            stream: &'static str,
            /// Host diagnostic retained for the adapter's existing mapping.
            detail: String,
        },
        /// One captured stream exceeded its byte limit.
        #[error("child process {stream} exceeded the {limit}-byte limit")]
        OutputTooLarge {
            /// Oversized stream name.
            stream: &'static str,
            /// Configured byte ceiling.
            limit: usize,
        },
    }

    /// Runs an existing binary adapter with inherited environment and no cwd.
    ///
    /// # Errors
    ///
    /// Returns a typed process-mechanics failure while preserving the legacy
    /// adapter contract.
    pub fn run(
        executable: &OsStr,
        arguments: &[&OsStr],
        limits: ProcessLimits,
    ) -> Result<CompletedProcess, ProcessError> {
        run_configured(executable, arguments, None, &[], &[], limits)
    }

    /// Runs an existing binary adapter through the shared execution kernel.
    ///
    /// # Errors
    ///
    /// Returns a typed process-mechanics failure while preserving inherited
    /// environment, explicit overrides/removals, and optional cwd.
    pub fn run_configured(
        executable: &OsStr,
        arguments: &[&OsStr],
        current_directory: Option<&Path>,
        environment: &[(&OsStr, &OsStr)],
        removed_environment: &[&OsStr],
        limits: ProcessLimits,
    ) -> Result<CompletedProcess, ProcessError> {
        #[cfg(unix)]
        {
            run_unix(
                executable,
                arguments,
                current_directory,
                environment,
                removed_environment,
                limits,
            )
        }
        #[cfg(not(unix))]
        {
            let _ = (
                executable,
                arguments,
                current_directory,
                environment,
                removed_environment,
                limits,
            );
            Err(ProcessError::Unavailable {
                detail: "descendant process-group containment is unavailable on this host"
                    .to_owned(),
            })
        }
    }

    #[cfg(unix)]
    fn run_unix(
        executable: &OsStr,
        arguments: &[&OsStr],
        current_directory: Option<&Path>,
        environment: &[(&OsStr, &OsStr)],
        removed_environment: &[&OsStr],
        limits: ProcessLimits,
    ) -> Result<CompletedProcess, ProcessError> {
        use std::os::unix::process::CommandExt;
        use std::process::{Command, Stdio};

        use super::{
            DescendantPolicy, KernelBudget, KernelCapture, KernelConclusion, capture_command,
        };

        let mut command = Command::new(executable);
        command.args(arguments);
        if let Some(directory) = current_directory {
            command.current_dir(directory);
        }
        command.envs(environment.iter().copied());
        for name in removed_environment {
            command.env_remove(name);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0);
        let KernelCapture {
            conclusion,
            stdout,
            stderr,
            ..
        } = capture_command(
            command,
            KernelBudget {
                timeout: limits.timeout,
                max_stdout_bytes: limits.max_output_bytes,
                max_stderr_bytes: limits.max_output_bytes,
            },
            DescendantPolicy::ProcessGroup,
            None,
        );
        match conclusion {
            KernelConclusion::Unavailable(detail) => Err(ProcessError::Unavailable { detail }),
            KernelConclusion::PipeUnavailable(stream) => {
                Err(ProcessError::PipeUnavailable { stream })
            }
            KernelConclusion::Observation(detail) => Err(ProcessError::Observation { detail }),
            KernelConclusion::OutputUnreadable { stream, detail } => {
                Err(ProcessError::OutputUnreadable { stream, detail })
            }
            KernelConclusion::TimedOut => Err(ProcessError::TimedOut {
                timeout: limits.timeout,
            }),
            KernelConclusion::ContainmentFailure | KernelConclusion::Cancelled => {
                Err(ProcessError::Observation {
                    detail: "shared execution kernel lost process containment".to_owned(),
                })
            }
            KernelConclusion::Terminal(status) => finish(status, stdout, stderr, limits),
        }
    }

    #[cfg(unix)]
    fn finish(
        status: ExitStatus,
        stdout: Option<super::CapturedBytes>,
        stderr: Option<super::CapturedBytes>,
        limits: ProcessLimits,
    ) -> Result<CompletedProcess, ProcessError> {
        let stdout = stdout.ok_or(ProcessError::OutputUnreadable {
            stream: "stdout",
            detail: "bounded capture was unavailable".to_owned(),
        })?;
        let stderr = stderr.ok_or(ProcessError::OutputUnreadable {
            stream: "stderr",
            detail: "bounded capture was unavailable".to_owned(),
        })?;
        if stdout.truncated {
            return Err(ProcessError::OutputTooLarge {
                stream: "stdout",
                limit: limits.max_output_bytes,
            });
        }
        if stderr.truncated {
            return Err(ProcessError::OutputTooLarge {
                stream: "stderr",
                limit: limits.max_output_bytes,
            });
        }
        Ok(CompletedProcess {
            status,
            stdout: stdout.bytes,
            stderr: stderr.bytes,
        })
    }
}

fn digest_reader(
    reader: &mut impl Read,
    maximum: u64,
    cancellation: Option<&CancellationToken>,
) -> Result<ContentDigest, DigestOutcome> {
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        if cancellation.is_some_and(CancellationToken::is_cancelled) {
            return Err(DigestOutcome::Cancelled);
        }
        let read = reader
            .read(&mut buffer)
            .map_err(|_| DigestOutcome::Unreadable)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(u64::try_from(read).map_err(|_| DigestOutcome::TooLarge)?)
            .ok_or(DigestOutcome::TooLarge)?;
        if total > maximum {
            return Err(DigestOutcome::TooLarge);
        }
        hasher.update(&buffer[..read]);
    }
    Ok(ContentDigest(
        hex_digest(hasher.finalize().as_slice()).into(),
    ))
}

enum DigestOutcome {
    Cancelled,
    TooLarge,
    Unreadable,
}

fn exit_code_accepted(binding: &ExitCodeBinding, code: i32) -> bool {
    match binding {
        ExitCodeBinding::Any => true,
        ExitCodeBinding::Exact(codes) => codes.contains(&code),
    }
}

#[cfg(target_os = "linux")]
fn monotonic_now() -> Instant {
    Instant::now()
}

#[cfg(not(target_os = "linux"))]
fn monotonic_now() -> std::time::Instant {
    std::time::Instant::now()
}

fn elapsed_nanos(started: std::time::Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn prelaunch_result<T>(
    started: std::time::Instant,
    request_identity: RequestIdentity,
    producer: ProducerDescriptor,
    observed_executable_digest: Option<ContentDigest>,
    state: ProducerExecutionState<T>,
) -> ProducerExecutionResult<T> {
    let total_nanos = elapsed_nanos(started);
    ProducerExecutionResult {
        protocol: PRODUCER_EXECUTION_RESULT_PROTOCOL,
        request_identity,
        producer,
        observed_executable_digest,
        cancellation_event: None,
        timing: ExecutionTiming {
            preflight_nanos: total_nanos,
            execution_nanos: None,
            total_nanos,
        },
        process: None,
        artifacts: Vec::new(),
        state,
    }
}

fn prelaunch_result_with_cancellation<T>(
    started: std::time::Instant,
    request_identity: RequestIdentity,
    producer: ProducerDescriptor,
    cancellation: CancellationBinding,
) -> ProducerExecutionResult<T> {
    prelaunch_result_with_observed_cancellation(
        started,
        request_identity,
        producer,
        None,
        cancellation,
    )
}

fn prelaunch_result_with_observed_cancellation<T>(
    started: std::time::Instant,
    request_identity: RequestIdentity,
    producer: ProducerDescriptor,
    observed: Option<ContentDigest>,
    cancellation: CancellationBinding,
) -> ProducerExecutionResult<T> {
    let mut result = prelaunch_result(
        started,
        request_identity,
        producer,
        observed,
        ProducerExecutionState::Cancelled,
    );
    result.cancellation_event = Some(cancellation);
    result
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}
