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
    io::{self, Read, Seek, SeekFrom},
    path::{Component, Path},
    process::ExitStatus,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

#[cfg(unix)]
use std::{process::Command, thread, time::Instant};

#[cfg(target_os = "linux")]
use std::{
    io::Write,
    os::{
        fd::AsRawFd,
        unix::process::{CommandExt, ExitStatusExt},
    },
    process::Stdio,
    sync::Mutex,
};

use serde::{Deserialize, Deserializer, Serialize};
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
    /// Execution itself hashes the executable bytes read through one
    /// capability-confined descriptor, before the pinned path is executed, and
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

impl<'de> Deserialize<'de> for ContentDigest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
    /// Expected SHA-256 identity of the executable bytes read from the opened
    /// executable descriptor before the pinned path is executed.
    pub executable_digest: ContentDigest,
}

/// The closed invocation procedure.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionProcedure {
    /// Execute the pinned absolute path directly without a shell.
    Direct,
}

/// One ordered producer argument.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputBinding {
    /// Caller-owned unique role of the input.
    pub role: String,
    /// Normal relative path beneath the capability root.
    pub path: String,
    /// Expected identity of the selected input bytes.
    pub digest: ContentDigest,
    /// Declared executable mode; part of request identity and not enforced.
    #[serde(default, skip_serializing_if = "is_false")]
    pub executable: bool,
}

#[allow(clippy::trivially_copy_pass_by_ref)] // serde skip predicate requires a reference.
fn is_false(value: &bool) -> bool {
    !*value
}

/// Closed stdin source.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputBinding {
    /// Caller-owned unique role of the output.
    pub role: String,
    /// Normal relative path beneath the capability root.
    pub path: String,
    /// Whether absence after execution is an executor failure.
    pub required: bool,
}

/// One bounded directory of dynamic regular-file outputs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputTreeBinding {
    /// Caller-owned unique role; observed file roles append `/relative/path`.
    pub role: String,
    /// Normal relative directory path beneath the capability root.
    pub path: String,
    /// Whether absence after execution is an executor failure.
    pub required: bool,
}

/// Exit-code behavior declared by a response protocol.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "codes", rename_all = "snake_case")]
pub enum ExitCodeBinding {
    /// Give every normal exit code to the typed response adapter.
    Any,
    /// Give only this normalized set of normal exits to the adapter.
    Exact(BTreeSet<i32>),
}

/// Identity of the caller-owned response contract and decoder.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "profile", rename_all = "kebab-case")]
pub enum ContainmentBinding {
    /// POSIX process group plus an exact producer contract prohibiting escape.
    ProcessGroupV1 {
        /// Contract that prohibits `setsid`, daemonization and untracked children.
        contract: ContractBinding,
    },
}

/// Identity of the authority and event allowed to cancel an invocation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
    /// Dynamic output trees, whose files are individually sealed and hashed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_trees: Vec<OutputTreeBinding>,
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
    /// Parses a retained request without accepting fields or encodings that
    /// the typed request would discard before replay.
    ///
    /// # Errors
    ///
    /// Returns [`RetainedRequestError`] if the wire shape is invalid, differs
    /// from the typed serialization, or fails the request's structural checks.
    pub fn from_retained_value(value: &serde_json::Value) -> Result<Self, RetainedRequestError> {
        let request: Self =
            serde_json::from_value(value.clone()).map_err(|_| RetainedRequestError::Wire)?;
        if serde_json::to_value(&request).map_err(|_| RetainedRequestError::Wire)? != *value {
            return Err(RetainedRequestError::Shape);
        }
        request.identity()?;
        Ok(request)
    }

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

/// Refusal to interpret retained producer-request bytes as an exact request.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum RetainedRequestError {
    /// The retained JSON is not a typed request.
    #[error("invalid retained producer-request wire shape")]
    Wire,
    /// Deserialization would discard or normalize part of the retained shape.
    #[error("retained producer-request shape is not exact")]
    Shape,
    /// The typed request fails the executor's structural validation.
    #[error("invalid retained producer-request structure: {0}")]
    Structure(#[from] InvalidExecutionRequest),
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
    /// Declared relative path beneath the capability root.
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
    /// The executable bytes read before the pinned path is executed differ from
    /// the request.
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

/// Executor-observed host class for comparable native measurements.
///
/// The machine identifier is never serialized. Its digest is domain-separated
/// from other uses of the host's high-entropy identifier.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservedHostContext {
    /// Opaque identity of the execution host for the recorded source and scope.
    pub machine_digest: ContentDigest,
    /// Present when the digest identifies one kernel boot, rather than a stable machine ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_source: Option<HostIdentitySource>,
    /// Operating-system class.
    pub os: String,
    /// Running kernel release.
    pub kernel_release: String,
    /// Native architecture class.
    pub architecture: String,
    /// Kernel-reported processor model or exact ARM CPU-ID tuple.
    pub cpu_model: String,
    /// Present when `cpu_model` is an ARM CPU-ID tuple rather than a model name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_model_source: Option<CpuModelSource>,
    /// Exact process CPU affinity for boot-scoped identity, represented opaquely.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_affinity_digest: Option<ContentDigest>,
    /// CPUs available to this process at sampling time.
    pub logical_cpus: u32,
    /// Kernel-reported total physical memory in bytes.
    pub memory_bytes: u64,
    /// Executor runtime and confinement class.
    pub runtime_class: String,
}

/// Explicit narrower scope used when a stable machine identifier is absent.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostIdentitySource {
    /// Linux's kernel boot UUID; stable only for the lifetime of that boot.
    KernelBootId,
}

/// Explicit origin for a CPU descriptor that is not a model name.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuModelSource {
    /// The exact ARM implementation/architecture/variant/part/revision tuple.
    ArmCpuId,
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
    /// SHA-256 of the executable bytes read before the pinned path is executed,
    /// when preflight reached it.
    pub observed_executable_digest: Option<ContentDigest>,
    /// Bound cancellation event when it was observed.
    pub cancellation_event: Option<CancellationBinding>,
    /// Measured monotonic durations.
    pub timing: ExecutionTiming,
    /// Execution-host context observed by the executor before launch, if complete.
    pub observed_host: Option<ObservedHostContext>,
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
    #[cfg(target_os = "linux")]
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
            #[cfg(target_os = "linux")]
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
            Ok(prelaunch_result(
                call_started,
                request_identity,
                producer,
                None,
                ProducerExecutionState::ContainmentFailure,
            ))
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
        if let Err(failure) = remove_declared_outputs(request, &validated) {
            return preflight_failure_result(
                call_started,
                request_identity,
                producer,
                &request.cancellation,
                failure,
            );
        }
        let preflight_nanos = elapsed_nanos(call_started);
        let observed_host = observe_host_context();
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
            observed_host,
            process,
            artifacts,
            state,
        }
    }

    #[cfg(target_os = "linux")]
    fn try_acquire(&self) -> Result<ExecutionSlot<'_>, AcquireFailure> {
        let mut active = self.active.lock().map_err(|_| AcquireFailure::Poisoned)?;
        if *active >= self.maximum_concurrency {
            return Err(AcquireFailure::Full);
        }
        *active = active.saturating_add(1);
        Ok(ExecutionSlot { executor: self })
    }
}

#[cfg(target_os = "linux")]
struct ExecutionSlot<'a> {
    executor: &'a ProducerExecutor,
}

#[cfg(target_os = "linux")]
impl Drop for ExecutionSlot<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.executor.active.lock() {
            *active = active.saturating_sub(1);
        }
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
enum AcquireFailure {
    Full,
    Poisoned,
}

#[cfg(target_os = "linux")]
struct ValidatedExecution {
    root: File,
    inputs: Vec<ValidatedInput>,
    executable_digest: ContentDigest,
}

#[cfg(target_os = "linux")]
struct ValidatedInput {
    role: String,
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
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Observed {
        maximum: usize,
    },
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
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
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
    let (input_roles, fixed_output_roles) = validate_artifact_bindings(request)?;
    if request.arguments.len() > MAX_ARGUMENTS {
        return Err(InvalidExecutionRequest::Argument);
    }
    for argument in &request.arguments {
        let valid = match argument {
            ArgumentBinding::Literal { value } => validate_text(value).is_ok(),
            ArgumentBinding::InputArtifact { role } => input_roles.contains(role.as_str()),
            ArgumentBinding::OutputArtifact { role } => fixed_output_roles.contains(role.as_str()),
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

fn validate_artifact_bindings(
    request: &ProducerExecutionRequest,
) -> Result<(BTreeSet<&str>, BTreeSet<&str>), InvalidExecutionRequest> {
    if request.inputs.len() > MAX_ARTIFACTS {
        return Err(InvalidExecutionRequest::Input);
    }
    if request.outputs.len() > request.budget.max_output_artifacts
        || request.outputs.len() + request.output_trees.len() > MAX_ARTIFACTS
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
    let fixed_output_roles = output_roles.clone();
    for tree in &request.output_trees {
        if validate_text(&tree.role).is_err()
            || validate_relative_normal_path(&tree.path).is_err()
            || !output_roles.insert(tree.role.as_str())
            || request
                .inputs
                .iter()
                .any(|input| paths_overlap(&input.path, &tree.path))
            || request
                .outputs
                .iter()
                .any(|output| paths_overlap(&output.path, &tree.path))
            || request
                .output_trees
                .iter()
                .any(|other| other.role != tree.role && paths_overlap(&other.path, &tree.path))
        {
            return Err(InvalidExecutionRequest::Output);
        }
    }
    Ok((input_roles, fixed_output_roles))
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

fn paths_overlap(left: &str, right: &str) -> bool {
    left == right
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(target_os = "linux")]
fn validate_capabilities(
    request: &ProducerExecutionRequest,
    cancellation: &CancellationToken,
) -> Result<ValidatedExecution, PreflightFailure> {
    let root = open_capability_root(&request.capability_root)?;
    let executable_digest = open_executable(request, cancellation)?;
    let inputs = open_inputs(request, &root, &executable_digest, cancellation)?;
    Ok(ValidatedExecution {
        root,
        inputs,
        executable_digest,
    })
}

/// Removes each declared fixed output that exists beneath the capability root,
/// so every declared fixed output is absent when the producer starts.
///
/// A missing parent or a missing file is nothing to remove. A symlink at the
/// declared path is removed as a link, never followed. Any other failure, such
/// as a directory at the path, refuses the request before launch.
#[cfg(target_os = "linux")]
fn remove_declared_outputs(
    request: &ProducerExecutionRequest,
    validated: &ValidatedExecution,
) -> Result<(), PreflightFailure> {
    use rustix::{
        fs::{AtFlags, unlinkat},
        io::Errno,
    };

    let refused = || {
        PreflightFailure::Refused(
            ExecutionRefusal::Output,
            Some(validated.executable_digest.clone()),
        )
    };
    for output in &request.outputs {
        let (directory, name) = output
            .path
            .rsplit_once('/')
            .map_or((None, output.path.as_str()), |(directory, name)| {
                (Some(directory), name)
            });
        let opened;
        let parent = match directory {
            None => &validated.root,
            Some(directory) => {
                opened = match open_beneath(&validated.root, directory, true) {
                    Ok(parent) => parent,
                    Err(Errno::NOENT) => continue,
                    Err(_) => return Err(refused()),
                };
                &opened
            }
        };
        match unlinkat(parent, name, AtFlags::empty()) {
            Ok(()) | Err(Errno::NOENT) => {}
            Err(_) => return Err(refused()),
        }
    }
    Ok(())
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
) -> Result<ContentDigest, PreflightFailure> {
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
    let executable_digest = digest_reader(&mut selected, MAX_EXECUTABLE_BYTES, Some(cancellation))
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
    Ok(executable_digest)
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
    let mut command = Command::new(&request.producer.executable);
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
    #[cfg(target_os = "linux")]
    let mut tracked_descendants = BTreeSet::new();
    #[cfg(not(target_os = "linux"))]
    let tracked_descendants = BTreeSet::new();
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
struct OutputCollector {
    artifacts: Vec<OutputArtifact>,
    remaining: u64,
    directories: usize,
}

#[cfg(target_os = "linux")]
fn open_beneath(root: &File, path: &str, directory: bool) -> Result<File, rustix::io::Errno> {
    use rustix::fs::{Mode, OFlags, ResolveFlags, openat2};

    let mut flags = OFlags::RDONLY | OFlags::CLOEXEC;
    if directory {
        flags |= OFlags::DIRECTORY;
    }
    openat2(
        root,
        path,
        flags,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS | ResolveFlags::NO_SYMLINKS,
    )
    .map(File::from)
}

/// Snapshots one regular output file within the remaining byte budget, or
/// returns `None` when it cannot be read or exceeds the budget.
#[cfg(target_os = "linux")]
fn snapshot_output(
    file: &mut File,
    collector: &mut OutputCollector,
    cancellation: &CancellationToken,
    label: &str,
) -> Option<(File, ContentDigest, u64)> {
    let (snapshot, digest) =
        snapshot_reader(file, collector.remaining, cancellation, label).ok()?;
    let length = snapshot.metadata().ok()?.len();
    collector.remaining = collector.remaining.checked_sub(length)?;
    Some((snapshot, digest, length))
}

#[cfg(target_os = "linux")]
fn observe_outputs(
    request: &ProducerExecutionRequest,
    root: &File,
    cancellation: &CancellationToken,
) -> (Vec<OutputArtifact>, bool) {
    let mut collector = OutputCollector {
        artifacts: Vec::new(),
        remaining: request.budget.max_output_bytes,
        directories: 0,
    };
    let failed = observe_fixed_outputs(request, root, cancellation, &mut collector)
        || observe_output_trees(request, root, cancellation, &mut collector);
    let mut artifacts = collector.artifacts;
    if !failed {
        artifacts.sort_by(|left, right| left.role.cmp(&right.role));
    }
    (artifacts, failed)
}

/// Observes every declared fixed output; returns `true` on failure.
#[cfg(target_os = "linux")]
fn observe_fixed_outputs(
    request: &ProducerExecutionRequest,
    root: &File,
    cancellation: &CancellationToken,
    collector: &mut OutputCollector,
) -> bool {
    for output in &request.outputs {
        let mut file = match open_beneath(root, &output.path, false) {
            Ok(file) => file,
            Err(_) if !output.required => continue,
            Err(_) => return true,
        };
        let Ok(metadata) = file.metadata() else {
            return true;
        };
        if !metadata.is_file() {
            return true;
        }
        let Some((snapshot, digest, byte_length)) =
            snapshot_output(&mut file, collector, cancellation, "producer-output")
        else {
            return true;
        };
        collector.artifacts.push(OutputArtifact {
            role: output.role.clone(),
            path: output.path.clone(),
            byte_length,
            digest,
            snapshot: Arc::new(snapshot),
        });
    }
    false
}

/// Observes every declared output tree; returns `true` on failure.
#[cfg(target_os = "linux")]
fn observe_output_trees(
    request: &ProducerExecutionRequest,
    root: &File,
    cancellation: &CancellationToken,
    collector: &mut OutputCollector,
) -> bool {
    for tree in &request.output_trees {
        let mut pending = vec![tree.path.clone()];
        while let Some(path) = pending.pop() {
            if cancellation.is_cancelled() || collector.directories >= MAX_ARTIFACTS {
                return true;
            }
            collector.directories += 1;
            let directory = match open_beneath(root, &path, true) {
                Ok(directory) => directory,
                Err(rustix::io::Errno::NOENT) if path == tree.path && !tree.required => break,
                Err(_) => return true,
            };
            let Ok(entries) = std::fs::read_dir(descriptor_path(&directory)) else {
                return true;
            };
            let mut children = Vec::new();
            for entry in entries {
                if children.len()
                    + pending.len()
                    + collector.directories
                    + collector.artifacts.len()
                    >= MAX_ARTIFACTS
                {
                    return true;
                }
                let Ok(entry) = entry else {
                    return true;
                };
                let Ok(name) = entry.file_name().into_string() else {
                    return true;
                };
                if validate_relative_normal_path(&name).is_err() {
                    return true;
                }
                children.push(format!("{path}/{name}"));
            }
            children.sort();
            for child in children {
                let Ok(mut file) = open_beneath(root, &child, false) else {
                    return true;
                };
                let Ok(metadata) = file.metadata() else {
                    return true;
                };
                if metadata.is_dir() {
                    pending.push(child);
                    continue;
                }
                if !metadata.is_file()
                    || collector.artifacts.len() >= request.budget.max_output_artifacts
                {
                    return true;
                }
                let Some((snapshot, digest, byte_length)) =
                    snapshot_output(&mut file, collector, cancellation, "producer-output-tree")
                else {
                    return true;
                };
                let Some(relative) = child
                    .strip_prefix(&tree.path)
                    .and_then(|p| p.strip_prefix('/'))
                else {
                    return true;
                };
                collector.artifacts.push(OutputArtifact {
                    role: format!("{}/{relative}", tree.role),
                    path: child,
                    byte_length,
                    digest,
                    snapshot: Arc::new(snapshot),
                });
            }
        }
    }
    false
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

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
fn read_host_text(path: &str, maximum: u64) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut text = String::new();
    file.take(maximum.checked_add(1)?)
        .read_to_string(&mut text)
        .ok()?;
    (u64::try_from(text.len()).ok()? <= maximum).then_some(text)
}

#[cfg(target_os = "linux")]
fn observe_host_context() -> Option<ObservedHostContext> {
    let machine_id =
        read_host_text("/etc/machine-id", 128).filter(|value| !value.trim().is_empty());
    let boot_id = machine_id
        .is_none()
        .then(|| read_host_text("/proc/sys/kernel/random/boot_id", 128))
        .flatten();
    let kernel_release = read_host_text("/proc/sys/kernel/osrelease", 256)?;
    let cpuinfo = read_host_text("/proc/cpuinfo", 1_048_576)?;
    let status = read_host_text("/proc/self/status", 65_536);
    let logical_cpus = u32::try_from(std::thread::available_parallelism().ok()?.get()).ok()?;
    let meminfo = read_host_text("/proc/meminfo", 65_536)?;
    parse_host_context(
        machine_id.as_deref(),
        boot_id.as_deref(),
        &kernel_release,
        &cpuinfo,
        status.as_deref(),
        &meminfo,
        logical_cpus,
    )
}

#[cfg(any(test, target_os = "linux"))]
fn parse_host_context(
    machine_id: Option<&str>,
    boot_id: Option<&str>,
    kernel_release: &str,
    cpuinfo: &str,
    status: Option<&str>,
    meminfo: &str,
    logical_cpus: u32,
) -> Option<ObservedHostContext> {
    let mut identity_bytes = b"engineering-assurance.observed-host/v1\0".to_vec();
    let identity_source = if let Some(machine_id) = machine_id {
        let machine_id = machine_id.trim();
        if machine_id.len() != 32 || !machine_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        identity_bytes.extend_from_slice(machine_id.as_bytes());
        None
    } else {
        let boot_id = boot_id?.trim();
        if boot_id.len() != 36
            || !boot_id.bytes().enumerate().all(|(index, byte)| {
                if matches!(index, 8 | 13 | 18 | 23) {
                    byte == b'-'
                } else {
                    byte.is_ascii_hexdigit()
                }
            })
        {
            return None;
        }
        identity_bytes.extend_from_slice(b"kernel-boot-id\0");
        identity_bytes.extend_from_slice(boot_id.as_bytes());
        Some(HostIdentitySource::KernelBootId)
    };
    let kernel_release = kernel_release.trim().to_owned();
    let reported_model = cpuinfo
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            matches!(name.trim(), "model name" | "Processor" | "Hardware")
                .then(|| value.trim().to_owned())
        })
        .filter(|value| !value.is_empty());
    let arm_ids_present = cpuinfo.lines().any(|line| {
        line.split_once(':')
            .is_some_and(|(name, _)| name.trim() == "CPU implementer")
    });
    let affinity = if identity_source.is_some() || arm_ids_present || reported_model.is_none() {
        Some(parse_cpu_affinity(status?)?)
    } else {
        None
    };
    let (cpu_model, cpu_model_source) = if arm_ids_present || reported_model.is_none() {
        let (_, selected) = affinity.as_ref()?;
        (
            arm_cpu_id(cpuinfo, selected)?,
            Some(CpuModelSource::ArmCpuId),
        )
    } else {
        (reported_model?, None)
    };
    let cpu_affinity_digest = if let Some((allowed, _)) = affinity {
        let mut bytes = b"engineering-assurance.observed-host-cpu-affinity/v1\0".to_vec();
        bytes.extend_from_slice(allowed.as_bytes());
        Some(ContentDigest::of_bytes(&bytes))
    } else {
        None
    };
    let memory_kib = meminfo.lines().find_map(|line| {
        let value = line.strip_prefix("MemTotal:")?;
        value.split_whitespace().next()?.parse::<u64>().ok()
    })?;
    let memory_bytes = memory_kib.checked_mul(1024)?;
    if kernel_release.is_empty()
        || cpu_model.is_empty()
        || cpu_model.len() > 256
        || logical_cpus == 0
        || memory_bytes == 0
    {
        return None;
    }
    Some(ObservedHostContext {
        machine_digest: ContentDigest::of_bytes(&identity_bytes),
        identity_source,
        os: "linux".to_owned(),
        kernel_release,
        architecture: std::env::consts::ARCH.to_owned(),
        cpu_model,
        cpu_model_source,
        cpu_affinity_digest,
        logical_cpus,
        memory_bytes,
        runtime_class: "linux-process-group-v1".to_owned(),
    })
}

#[cfg(any(test, target_os = "linux"))]
fn parse_cpu_affinity(status: &str) -> Option<(&str, std::collections::BTreeSet<u32>)> {
    let allowed = status
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:").map(str::trim))?;
    if allowed.is_empty() || allowed.len() > 4096 {
        return None;
    }
    let mut selected = std::collections::BTreeSet::new();
    for range in allowed.split(',') {
        let mut bounds = range.split('-');
        let first = bounds.next()?.parse::<u32>().ok()?;
        let last = bounds
            .next()
            .map_or(Some(first), |value| value.parse::<u32>().ok())?;
        if bounds.next().is_some() || last < first || last - first > 4096 {
            return None;
        }
        for cpu in first..=last {
            if !selected.insert(cpu) || selected.len() > 4096 {
                return None;
            }
        }
    }
    Some((allowed, selected))
}

#[cfg(any(test, target_os = "linux"))]
fn arm_cpu_id(cpuinfo: &str, selected: &std::collections::BTreeSet<u32>) -> Option<String> {
    let mut observed = std::collections::BTreeSet::new();
    let mut common = None;
    for processor in cpuinfo.split("\n\n") {
        let field = |key: &str| {
            processor.lines().find_map(|line| {
                let (name, value) = line.split_once(':')?;
                (name.trim() == key).then(|| value.trim())
            })
        };
        let Some(index) = field("processor") else {
            continue;
        };
        let index = index.parse::<u32>().ok()?;
        if !selected.contains(&index) {
            continue;
        }
        let implementer = field("CPU implementer")?;
        let architecture = field("CPU architecture")?;
        let variant = field("CPU variant")?;
        let part = field("CPU part")?;
        let revision = field("CPU revision")?;
        for value in [implementer, architecture, variant, part, revision] {
            let digits = value.strip_prefix("0x").unwrap_or(value);
            if digits.is_empty()
                || digits.len() > 16
                || !digits.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return None;
            }
        }
        let tuple = format!(
            "arm-cpu-id:implementer={implementer},architecture={architecture},variant={variant},part={part},revision={revision}"
        );
        if common.as_ref().is_some_and(|prior| prior != &tuple) || !observed.insert(index) {
            return None;
        }
        common = Some(tuple);
    }
    if observed != *selected {
        return None;
    }
    common
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
        observed_host: None,
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

#[cfg(test)]
mod host_context_tests {
    use super::{CpuModelSource, HostIdentitySource, parse_host_context};
    use ix_trace_rs::trace;

    #[test]
    #[trace("TC-175", "FR-019-AC-8")]
    fn tc_175_host_context_is_complete_stable_and_opaque() {
        let machine = "0123456789abcdef0123456789abcdef";
        let cpu = "processor : 0\nmodel name : Fictional CPU\n";
        let memory = "MemTotal: 32768 kB\n";
        let first = parse_host_context(Some(machine), None, "6.1.0", cpu, None, memory, 4)
            .expect("complete synthetic host context");
        let second = parse_host_context(Some(machine), None, "6.1.0", cpu, None, memory, 4)
            .expect("stable synthetic host context");
        assert_eq!(first, second);
        assert_eq!(first.memory_bytes, 33_554_432);
        assert_eq!(first.cpu_model, "Fictional CPU");
        assert_eq!(first.identity_source, None);
        assert_eq!(first.cpu_model_source, None);
        assert_eq!(first.cpu_affinity_digest, None);
        let serialized = serde_json::to_value(&first).expect("serialize");
        assert!(serialized.get("identitySource").is_none());
        assert!(serialized.get("cpuModelSource").is_none());
        assert!(serialized.get("cpuAffinityDigest").is_none());
        assert_ne!(first.machine_digest.as_str(), machine);
        assert!(
            !serde_json::to_string(&first)
                .expect("serialize")
                .contains(machine)
        );
        assert!(parse_host_context(Some("bad"), None, "6.1.0", cpu, None, memory, 4).is_none());
        assert!(parse_host_context(Some(machine), None, "6.1.0", cpu, None, "", 4).is_none());
        assert!(parse_host_context(Some(machine), None, "6.1.0", cpu, None, memory, 0).is_none());
    }

    #[test]
    #[trace("TC-175", "FR-019-AC-8")]
    fn tc_175_arm_container_uses_boot_scoped_identity_and_exact_cpu_id() {
        let boot = "01234567-89ab-4cde-8123-0123456789ab";
        let cpu = "processor : 0\nCPU implementer : 0x61\nCPU architecture : 8\nCPU variant : 0x0\nCPU part : 0x000\nCPU revision : 0\n\nprocessor : 1\nCPU implementer : 0x61\nCPU architecture : 8\nCPU variant : 0x0\nCPU part : 0x000\nCPU revision : 0\n";
        let affinity = "Name:\tfictional\nCpus_allowed_list:\t0\n";
        let memory = "MemTotal: 32768 kB\n";
        let first =
            parse_host_context(None, Some(boot), "6.1.0", cpu, Some(affinity), memory, 4).unwrap();
        let second =
            parse_host_context(None, Some(boot), "6.1.0", cpu, Some(affinity), memory, 4).unwrap();
        assert_eq!(first, second);
        let blank_model = format!("Hardware : \n{cpu}");
        assert_eq!(
            parse_host_context(
                None,
                Some(boot),
                "6.1.0",
                &blank_model,
                Some(affinity),
                memory,
                4
            )
            .unwrap()
            .cpu_model,
            first.cpu_model
        );
        assert_eq!(
            first.identity_source,
            Some(HostIdentitySource::KernelBootId)
        );
        assert_eq!(first.cpu_model_source, Some(CpuModelSource::ArmCpuId));
        assert_eq!(
            first.cpu_model,
            "arm-cpu-id:implementer=0x61,architecture=8,variant=0x0,part=0x000,revision=0"
        );
        let serialized = serde_json::to_value(&first).unwrap();
        assert_eq!(serialized["identitySource"], "kernel_boot_id");
        assert_eq!(serialized["cpuModelSource"], "arm_cpu_id");
        assert!(serialized.get("cpuAffinityDigest").is_some());
        assert!(!serialized.to_string().contains(boot));
        assert!(!serialized.to_string().contains("Cpus_allowed_list"));
        let different_affinity = parse_host_context(
            None,
            Some(boot),
            "6.1.0",
            cpu,
            Some("Cpus_allowed_list:\t1\n"),
            memory,
            4,
        )
        .unwrap();
        assert_ne!(
            first.cpu_affinity_digest,
            different_affinity.cpu_affinity_digest
        );
        let next_boot = parse_host_context(
            None,
            Some("01234567-89ab-4cde-8123-0123456789ac"),
            "6.1.0",
            cpu,
            Some(affinity),
            memory,
            4,
        )
        .unwrap();
        assert_ne!(first.machine_digest, next_boot.machine_digest);
    }

    #[test]
    #[trace("TC-175", "FR-019-AC-8")]
    fn tc_175_arm_affinity_selects_the_observed_cpu_class() {
        let machine = "0123456789abcdef0123456789abcdef";
        let boot = "01234567-89ab-4cde-8123-0123456789ab";
        let cpu = "processor : 0\nCPU implementer : 0x61\nCPU architecture : 8\nCPU variant : 0x0\nCPU part : 0x000\nCPU revision : 0\n\nprocessor : 1\nCPU implementer : 0x61\nCPU architecture : 8\nCPU variant : 0x0\nCPU part : 0x001\nCPU revision : 0\n\nHardware : Fictional Board\n";
        let memory = "MemTotal: 32768 kB\n";
        let observe = |machine_id, allowed| {
            parse_host_context(
                machine_id,
                Some(boot),
                "6.1.0",
                cpu,
                Some(allowed),
                memory,
                1,
            )
        };
        let first = observe(Some(machine), "Cpus_allowed_list:\t0\n").unwrap();
        let second = observe(Some(machine), "Cpus_allowed_list:\t1\n").unwrap();
        assert_ne!(first.cpu_model, second.cpu_model);
        assert_ne!(first.cpu_affinity_digest, second.cpu_affinity_digest);
        assert_eq!(first.identity_source, None);
        assert_eq!(second.cpu_model_source, Some(CpuModelSource::ArmCpuId));
        assert!(observe(Some(machine), "Cpus_allowed_list:\t0-1\n").is_none());
        assert!(observe(None, "Cpus_allowed_list:\t0-1\n").is_none());
        assert!(observe(Some(machine), "Cpus_allowed_list:\t2\n").is_none());
        assert!(parse_host_context(Some(machine), None, "6.1.0", cpu, None, memory, 1).is_none());
    }

    #[test]
    #[trace("TC-175", "FR-019-AC-8")]
    fn tc_175_boot_scoped_host_refuses_missing_or_malformed_inputs() {
        let boot = "01234567-89ab-4cde-8123-0123456789ab";
        let cpu = "processor : 0\nCPU implementer : 0x61\nCPU architecture : 8\nCPU variant : 0x0\nCPU part : 0x000\nCPU revision : 0\n";
        let affinity = "Cpus_allowed_list:\t0\n";
        let memory = "MemTotal: 32768 kB\n";
        assert!(parse_host_context(None, None, "6.1.0", cpu, Some(affinity), memory, 4).is_none());
        assert!(
            parse_host_context(None, Some("bad"), "6.1.0", cpu, Some(affinity), memory, 4)
                .is_none()
        );
        assert!(
            parse_host_context(
                Some("bad"),
                Some(boot),
                "6.1.0",
                cpu,
                Some(affinity),
                memory,
                4
            )
            .is_none()
        );
        assert!(parse_host_context(None, Some(boot), "6.1.0", cpu, None, memory, 4).is_none());
        assert!(
            parse_host_context(
                None,
                Some(boot),
                "6.1.0",
                cpu,
                Some("Cpus_allowed_list: 0-x"),
                memory,
                4
            )
            .is_none()
        );
        assert!(
            parse_host_context(
                None,
                Some(boot),
                "6.1.0",
                "processor : 0",
                Some(affinity),
                memory,
                4
            )
            .is_none()
        );
        assert!(
            parse_host_context(
                None,
                Some(boot),
                "6.1.0",
                &cpu.replace("0x61", "0xx"),
                Some(affinity),
                memory,
                4
            )
            .is_none()
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "run in a provisioned Linux host or container with observed identity files"]
    #[trace("TC-175", "FR-019-AC-8")]
    fn tc_175_live_linux_host_observation_is_explicit() {
        let observed = super::observe_host_context().expect("complete observed Linux host");
        if super::read_host_text("/etc/machine-id", 128).is_none_or(|value| value.trim().is_empty())
        {
            assert_eq!(
                observed.identity_source,
                Some(HostIdentitySource::KernelBootId)
            );
        } else {
            assert_eq!(observed.identity_source, None);
        }
    }
}

#[cfg(feature = "campaign")]
mod campaign_source_projection {
    use super::{
        BTreeSet, Component, ContentDigest, Digest, File, InputBinding, MAX_ARTIFACTS,
        MAX_INPUT_BYTES, Path, Read, Sha256,
    };
    use crate::campaign::{CampaignError, CampaignSource, OmittedSourceLink, SourceTreeBinding};
    use sha1::{Digest as Sha1Digest, Sha1};
    use std::io::Cursor;

    fn lowercase_hex(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut result = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            result.push(char::from(HEX[usize::from(byte >> 4)]));
            result.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        result
    }

    /// Expands and verifies an exact Git tree into FR-019 regular-file inputs.
    /// The executor repeats the SHA-256 check against sealed descriptors before
    /// launch, so a mutation between resolution and execution is refused.
    pub(crate) fn validate_source_tree(
        tree: &SourceTreeBinding,
        source: &CampaignSource,
        root: &str,
    ) -> Result<(Vec<InputBinding>, Vec<OmittedSourceLink>), CampaignError> {
        if tree.repository != source.repository {
            return Err(CampaignError::Binding {
                field: "sourceTree.repository",
            });
        }
        if tree.manifest.is_empty()
            || tree.manifest.len() > 8 * 1024 * 1024
            || !tree.manifest.ends_with(&[0])
        {
            return Err(CampaignError::SourceTree { field: "manifest" });
        }
        if ContentDigest::of_bytes(&tree.manifest).as_str() != source.digest {
            return Err(CampaignError::Binding {
                field: "sourceTree.manifestDigest",
            });
        }
        let root = Path::new(root);
        if !root.is_absolute() {
            return Err(CampaignError::SourceTree {
                field: "capability root",
            });
        }
        let mut seen = BTreeSet::new();
        let mut inputs = Vec::new();
        let mut omitted_links = Vec::new();
        let mut total = 0_u64;
        for record in tree.manifest[..tree.manifest.len() - 1].split(|byte| *byte == 0) {
            let (mode, oid, path) = parse_source_record(record)?;
            if !seen.insert(path) {
                return Err(CampaignError::Duplicate {
                    kind: "source path",
                    name: path.to_owned(),
                });
            }
            if inputs.len() + omitted_links.len() >= MAX_ARTIFACTS {
                return Err(CampaignError::Limit {
                    field: "source file population",
                });
            }
            if mode == "120000" {
                omitted_links.push(verify_source_link(root, path, oid, &mut total)?);
                continue;
            }
            inputs.push(verify_source_file(
                root,
                path,
                oid,
                mode == "100755",
                &mut total,
            )?);
        }
        if inputs.is_empty() {
            return Err(CampaignError::NoEntries {
                field: "source tree files",
            });
        }
        Ok((inputs, omitted_links))
    }

    fn parse_source_record(record: &[u8]) -> Result<(&str, &str, &str), CampaignError> {
        let separator =
            record
                .iter()
                .position(|byte| *byte == b'\t')
                .ok_or(CampaignError::SourceTree {
                    field: "manifest record",
                })?;
        let (header, path_with_tab) = record.split_at(separator);
        let header = std::str::from_utf8(header).map_err(|_| CampaignError::SourceTree {
            field: "manifest header",
        })?;
        let mut tokens = header.split(' ');
        let (Some(mode), Some(kind), Some(oid), None) =
            (tokens.next(), tokens.next(), tokens.next(), tokens.next())
        else {
            return Err(CampaignError::SourceTree {
                field: "manifest header",
            });
        };
        if kind != "blob" || !matches!(mode, "100644" | "100755" | "120000") {
            return Err(CampaignError::SourceTree {
                field: "unsupported Git mode",
            });
        }
        if !matches!(oid.len(), 40 | 64)
            || !oid
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(CampaignError::SourceTree {
                field: "Git blob OID",
            });
        }
        let path =
            std::str::from_utf8(&path_with_tab[1..]).map_err(|_| CampaignError::SourceTree {
                field: "source path encoding",
            })?;
        let relative = Path::new(path);
        if path.is_empty()
            || relative.is_absolute()
            || relative
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(CampaignError::SourceTree {
                field: "source path",
            });
        }
        Ok((mode, oid, path))
    }

    fn verify_source_link(
        root: &Path,
        path: &str,
        oid: &str,
        total: &mut u64,
    ) -> Result<OmittedSourceLink, CampaignError> {
        let (parent, leaf) = source_parent(root, path)?;
        let target = rustix::fs::readlinkat(&parent, leaf, Vec::new()).map_err(|_| {
            CampaignError::SourceTree {
                field: "source link read",
            }
        })?;
        let target = target.to_str().map_err(|_| CampaignError::SourceTree {
            field: "source link encoding",
        })?;
        let length = u64::try_from(target.len()).map_err(|_| CampaignError::Limit {
            field: "source byte population",
        })?;
        *total = total.checked_add(length).ok_or(CampaignError::Limit {
            field: "source byte population",
        })?;
        if *total > MAX_INPUT_BYTES {
            return Err(CampaignError::Limit {
                field: "source byte population",
            });
        }
        let digest = hash_source_blob(&mut Cursor::new(target.as_bytes()), length, oid)?;
        Ok(OmittedSourceLink {
            path: path.to_owned(),
            digest,
        })
    }

    fn verify_source_file(
        root: &Path,
        path: &str,
        oid: &str,
        executable: bool,
        total: &mut u64,
    ) -> Result<InputBinding, CampaignError> {
        use rustix::fs::{Mode, OFlags, openat};

        let (parent, leaf) = source_parent(root, path)?;
        let descriptor = openat(
            &parent,
            leaf,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|_| CampaignError::SourceTree {
            field: "source file open",
        })?;
        let mut file = File::from(descriptor);
        let metadata = file.metadata().map_err(|_| CampaignError::SourceTree {
            field: "source file metadata",
        })?;
        if !metadata.is_file() {
            return Err(CampaignError::SourceTree {
                field: "source file type",
            });
        }
        *total = total
            .checked_add(metadata.len())
            .ok_or(CampaignError::Limit {
                field: "source byte population",
            })?;
        if *total > MAX_INPUT_BYTES {
            return Err(CampaignError::Limit {
                field: "source byte population",
            });
        }
        let digest = hash_source_blob(&mut file, metadata.len(), oid)?;
        let role = if executable {
            format!("source-exec/{path}")
        } else {
            format!("source/{path}")
        };
        Ok(InputBinding {
            role,
            path: path.to_owned(),
            digest,
            executable,
        })
    }

    /// Open every root and parent component relative to a verified directory
    /// descriptor. No path component may be replaced by a symlink between a
    /// separate metadata check and the read of its child.
    fn source_parent<'a>(root: &Path, path: &'a str) -> Result<(File, &'a str), CampaignError> {
        use rustix::fs::{Mode, OFlags, openat};

        if !root.is_absolute() {
            return Err(CampaignError::SourceTree {
                field: "capability root",
            });
        }
        let mut directory = File::open("/").map_err(|_| CampaignError::SourceTree {
            field: "source root open",
        })?;
        let open_directory = |directory: File, name: &std::ffi::OsStr| {
            openat(
                &directory,
                name,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map(File::from)
            .map_err(|_| CampaignError::SourceTree {
                field: "source parent directory",
            })
        };
        for component in root.components() {
            match component {
                Component::RootDir => {}
                Component::Normal(name) => directory = open_directory(directory, name)?,
                _ => {
                    return Err(CampaignError::SourceTree {
                        field: "capability root",
                    });
                }
            }
        }
        let (parents, leaf) = path.rsplit_once('/').unwrap_or(("", path));
        for name in parents.split('/').filter(|name| !name.is_empty()) {
            directory = open_directory(directory, std::ffi::OsStr::new(name))?;
        }
        Ok((directory, leaf))
    }

    fn hash_source_blob<R: Read>(
        file: &mut R,
        length: u64,
        oid: &str,
    ) -> Result<ContentDigest, CampaignError> {
        let header = format!("blob {length}\0");
        let mut git_sha1 = Sha1::new();
        let mut git_sha256 = Sha256::new();
        git_sha1.update(header.as_bytes());
        git_sha256.update(header.as_bytes());
        let mut content_sha256 = Sha256::new();
        let mut read_total = 0_u64;
        let mut buffer = [0_u8; 16 * 1024];
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|_| CampaignError::SourceTree {
                    field: "source file read",
                })?;
            if count == 0 {
                break;
            }
            read_total = read_total
                .checked_add(u64::try_from(count).map_err(|_| CampaignError::Limit {
                    field: "source byte population",
                })?)
                .ok_or(CampaignError::Limit {
                    field: "source byte population",
                })?;
            if read_total > length {
                return Err(CampaignError::SourceTree {
                    field: "source file changed",
                });
            }
            git_sha1.update(&buffer[..count]);
            git_sha256.update(&buffer[..count]);
            content_sha256.update(&buffer[..count]);
        }
        if read_total != length {
            return Err(CampaignError::SourceTree {
                field: "source file changed",
            });
        }
        let observed_oid = if oid.len() == 40 {
            lowercase_hex(&git_sha1.finalize())
        } else {
            lowercase_hex(&git_sha256.finalize())
        };
        if observed_oid != oid {
            return Err(CampaignError::SourceTree {
                field: "Git blob mismatch",
            });
        }
        ContentDigest::parse(&lowercase_hex(&content_sha256.finalize())).map_err(|_| {
            CampaignError::SourceTree {
                field: "source digest",
            }
        })
    }
}

#[cfg(feature = "campaign")]
pub(crate) use campaign_source_projection::validate_source_tree;
