// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary adapter that runs one FR-019 producer-execution request.
//!
//! The command owns no domain semantics: it parses the closed request, runs it
//! through the shared bounded executor with one built-in response adapter, and
//! returns the canonical FR-019 result line (FR-029).

use engineering_assurance::producer_execution::{
    CancellationToken, ContractBinding, ExitCodeBinding, MalformedResponse, OutputArtifact,
    ProcessEvidence, ProducerExecutionRequest, ProducerExecutor, ProducerResponseAdapter,
    ResponseBinding, RetainedRequestError, TerminalStatus,
};
use serde::Serialize;
use thiserror::Error;

/// Response protocol kind the built-in adapter is bound to.
pub(crate) const RESPONSE_PROTOCOL_KIND: &str = "engineering-assurance.process-evidence";
/// Adapter implementation kind of the built-in adapter.
pub(crate) const ADAPTER_KIND: &str = "engineering-assurance.cli-process-evidence-adapter";
const BINDING_VERSION: &str = "1";

const REQUEST_SCHEMA: &str =
    include_str!("producer_execute_schemas/producer-execution-request-v1.schema.json");
const RESULT_SCHEMA: &str =
    include_str!("producer_execute_schemas/producer-execution-result-v1.schema.json");

/// Published schema selected by `--schema`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SchemaKind {
    Request,
    Result,
}

impl SchemaKind {
    /// Parses the `--schema` value.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "request" => Some(Self::Request),
            "result" => Some(Self::Result),
            _ => None,
        }
    }

    /// Returns the published schema document bytes.
    pub(crate) const fn document(self) -> &'static [u8] {
        match self {
            Self::Request => REQUEST_SCHEMA.as_bytes(),
            Self::Result => RESULT_SCHEMA.as_bytes(),
        }
    }
}

/// Typed refusal that produces no execution result.
#[derive(Debug, Error)]
pub(crate) enum ProducerExecuteError {
    /// The request is not readable JSON.
    #[error("producer-execute request is not readable JSON")]
    Unreadable,
    /// The request is JSON that is not an exact request document.
    #[error("producer-execute request is not an exact producer-execution request")]
    Invalid,
    /// The request fails the executor's structural validation.
    #[error("producer-execute request is structurally invalid: {0}")]
    StructureInvalid(#[source] engineering_assurance::producer_execution::InvalidExecutionRequest),
    /// The result cannot be represented as canonical JSON.
    #[error("producer-execute result cannot be encoded canonically")]
    ResultEncoding,
}

impl ProducerExecuteError {
    /// Stable machine error code.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "producer_execute_request_unreadable",
            Self::Invalid => "producer_execute_request_invalid",
            Self::StructureInvalid(_) => "producer_execute_request_structure_invalid",
            Self::ResultEncoding => "producer_execute_result_encoding_failed",
        }
    }
}

/// Observation returned for an admitted normal exit.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExitObservation {
    exit_code: i32,
}

/// Built-in adapter that admits a normal exit and reports its code.
pub(crate) struct ProcessEvidenceAdapter {
    binding: ResponseBinding,
}

impl ProcessEvidenceAdapter {
    /// Builds the adapter whose fixed binding carries the request's exit policy.
    fn new(exit_codes: ExitCodeBinding) -> Self {
        Self {
            binding: ResponseBinding {
                protocol: ContractBinding {
                    kind: RESPONSE_PROTOCOL_KIND.to_owned(),
                    version: BINDING_VERSION.to_owned(),
                },
                adapter: ContractBinding {
                    kind: ADAPTER_KIND.to_owned(),
                    version: BINDING_VERSION.to_owned(),
                },
                exit_codes,
            },
        }
    }
}

impl ProducerResponseAdapter for ProcessEvidenceAdapter {
    type Observation = ExitObservation;

    fn binding(&self) -> &ResponseBinding {
        &self.binding
    }

    fn decode(
        &self,
        process: &ProcessEvidence,
        _artifacts: &[OutputArtifact],
    ) -> Result<Self::Observation, MalformedResponse> {
        match process.terminal_status {
            Some(TerminalStatus::ExitCode(exit_code)) => Ok(ExitObservation { exit_code }),
            Some(TerminalStatus::Signal(_)) | None => Err(MalformedResponse),
        }
    }
}

/// Runs one request document and returns the canonical result line.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, ProducerExecuteError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| ProducerExecuteError::Unreadable)?;
    let request =
        ProducerExecutionRequest::from_retained_value(&value).map_err(|error| match error {
            RetainedRequestError::Wire | RetainedRequestError::Shape => {
                ProducerExecuteError::Invalid
            }
            RetainedRequestError::Structure(structure) => {
                ProducerExecuteError::StructureInvalid(structure)
            }
        })?;
    // The request carries the exit policy; the rest of the binding is fixed, so
    // a request naming any other protocol or adapter is refused by the executor.
    let adapter = ProcessEvidenceAdapter::new(request.response.exit_codes.clone());
    let executor = ProducerExecutor::new(request.budget.max_concurrency)
        .map_err(|_| ProducerExecuteError::Invalid)?;
    let cancellation = CancellationToken::new(request.cancellation.clone());
    let result = executor
        .execute(&request, &cancellation, &adapter)
        .map_err(ProducerExecuteError::StructureInvalid)?;
    let mut line = result
        .canonical_bytes()
        .map_err(|_| ProducerExecuteError::ResultEncoding)?;
    line.push(b'\n');
    Ok(line)
}
