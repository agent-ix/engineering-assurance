// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded binary-only adapter for ix-flow lifecycle and human decisions.

use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fs,
    path::{Component, Path, PathBuf},
    time::Duration,
};

use engineering_assurance::{
    compatibility,
    workflow::{
        DecisionChoice, DecisionEvent, WorkflowBinding, WorkflowGate, WorkflowHostRequest,
        WorkflowHostResult, WorkflowKind, WorkflowNextAction, WorkflowOperation, WorkflowSnapshot,
    },
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Map, Value};
use thiserror::Error;

use crate::process_host::{self, ProcessError, ProcessLimits};

const IX_FLOW_COMPONENT: &str = "ix-flow";
const HOST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_HOST_OUTPUT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy)]
struct HostLimits {
    timeout: Duration,
    max_output_bytes: usize,
}

impl Default for HostLimits {
    fn default() -> Self {
        Self {
            timeout: HOST_TIMEOUT,
            max_output_bytes: MAX_HOST_OUTPUT_BYTES,
        }
    }
}

#[derive(Debug, Error)]
pub(crate) enum WorkflowHostError {
    #[error("invalid workflow-host request: {detail}")]
    Request { detail: String },
    #[error("workflow binding does not match ix-flow state: {detail}")]
    Binding { detail: String },
    #[error("workflow transition is invalid: {detail}")]
    Transition { detail: String },
    #[error("workflow decision conflicts with ix-flow state: {detail}")]
    DecisionConflict { detail: String },
    #[error("ix-flow is unavailable: {detail}")]
    Unavailable { detail: String },
    #[error("ix-flow version {observed:?} does not match required {expected:?}")]
    VersionIncompatible { observed: String, expected: String },
    #[error("ix-flow command {operation:?} failed with {upstream_code:?}: {detail}")]
    CommandFailed {
        operation: String,
        upstream_code: String,
        detail: String,
    },
    #[error("invalid ix-flow response to {operation:?}: {detail}")]
    InvalidResponse { operation: String, detail: String },
    #[error("ix-flow outcome for {operation:?} is indeterminate: {detail}")]
    OutcomeIndeterminate { operation: String, detail: String },
}

impl WorkflowHostError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Request { .. } => "workflow_host_request_invalid",
            Self::Binding { .. } => "workflow_binding_invalid",
            Self::Transition { .. } => "workflow_transition_invalid",
            Self::DecisionConflict { .. } => "workflow_decision_conflict",
            Self::Unavailable { .. } => "ix_flow_unavailable",
            Self::VersionIncompatible { .. } => "ix_flow_version_incompatible",
            Self::CommandFailed { .. } => "ix_flow_command_failed",
            Self::InvalidResponse { .. } => "ix_flow_response_invalid",
            Self::OutcomeIndeterminate { .. } => "ix_flow_outcome_indeterminate",
        }
    }
}

pub(crate) fn execute(
    request: &WorkflowHostRequest,
) -> Result<WorkflowHostResult, WorkflowHostError> {
    execute_with_limits(request, HostLimits::default())
}

fn execute_with_limits(
    request: &WorkflowHostRequest,
    limits: HostLimits,
) -> Result<WorkflowHostResult, WorkflowHostError> {
    let workflow = request
        .binding
        .validate()
        .map_err(|error| WorkflowHostError::Binding {
            detail: error.to_string(),
        })?;
    let state_dir = absolute_state_directory(&request.state_dir)?;
    let skill_root = canonical_skill_root(&request.skill_root, &request.binding, workflow)?;
    let executable = executable(&request.ix_flow_executable)?;
    require_compatible_version(&executable, limits)?;

    let envelope = start_or_resume(
        &executable,
        &state_dir,
        &skill_root,
        &request.binding,
        workflow,
        limits,
    )?;
    verify_chain(&executable, &state_dir, &request.binding.run_id, limits)?;
    match request.operation {
        WorkflowOperation::StartOrResume => Ok(WorkflowHostResult::snapshot(snapshot(&envelope)?)),
        WorkflowOperation::Decide => decide(
            &executable,
            &state_dir,
            &request.binding,
            workflow,
            request.choice,
            envelope,
            limits,
        ),
    }
}

fn absolute_state_directory(value: &str) -> Result<PathBuf, WorkflowHostError> {
    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(WorkflowHostError::Request {
            detail: "state_dir must be absolute".to_owned(),
        });
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(WorkflowHostError::Request {
            detail: "state_dir must not contain parent or current-directory components".to_owned(),
        });
    }
    if path.exists() && !path.is_dir() {
        return Err(WorkflowHostError::Request {
            detail: "state_dir exists and is not a directory".to_owned(),
        });
    }
    Ok(path.to_owned())
}

fn canonical_skill_root(
    value: &str,
    binding: &WorkflowBinding,
    workflow: WorkflowKind,
) -> Result<PathBuf, WorkflowHostError> {
    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(WorkflowHostError::Request {
            detail: "skill_root must be absolute".to_owned(),
        });
    }
    let resolved = fs::canonicalize(path).map_err(|source| WorkflowHostError::Request {
        detail: format!("cannot resolve skill_root: {source}"),
    })?;
    if !resolved.join("SKILL.md").is_file() {
        return Err(WorkflowHostError::Request {
            detail: "skill_root contains no SKILL.md".to_owned(),
        });
    }
    let definition = resolved
        .join("workflows")
        .join(workflow.as_str())
        .join("def.yaml");
    let bytes = fs::read(&definition).map_err(|source| WorkflowHostError::Request {
        detail: format!("cannot read {}: {source}", definition.display()),
    })?;
    let identity: WorkflowDefinitionIdentity =
        yaml_serde::from_slice(&bytes).map_err(|source| WorkflowHostError::Request {
            detail: format!("cannot parse {}: {source}", definition.display()),
        })?;
    if identity.name != binding.workflow
        || identity.version != binding.workflow_version
        || identity.initial_phase != workflow.initial_phase()
    {
        return Err(WorkflowHostError::Binding {
            detail: format!(
                "canonical definition is {} {} at phase {}, binding requests {} {}",
                identity.name,
                identity.version,
                identity.initial_phase,
                binding.workflow,
                binding.workflow_version
            ),
        });
    }
    Ok(resolved)
}

fn executable(value: &str) -> Result<OsString, WorkflowHostError> {
    let path = Path::new(value);
    let is_name = path.components().count() == 1
        && matches!(path.components().next(), Some(Component::Normal(_)));
    if !path.is_absolute() && !is_name {
        return Err(WorkflowHostError::Request {
            detail: "ix_flow_executable must be a command name or absolute path".to_owned(),
        });
    }
    Ok(path.as_os_str().to_owned())
}

fn require_compatible_version(
    executable: &OsStr,
    limits: HostLimits,
) -> Result<(), WorkflowHostError> {
    let expected =
        compatibility::expected_component_version(IX_FLOW_COMPONENT).map_err(|error| {
            WorkflowHostError::Request {
                detail: error.to_string(),
            }
        })?;
    let completed = run_process(
        executable,
        &[OsStr::new("--version")],
        false,
        "version",
        limits,
    )?;
    if !completed.status.success() {
        return Err(WorkflowHostError::InvalidResponse {
            operation: "version".to_owned(),
            detail: format!("process exited with {}", completed.status),
        });
    }
    let observed = std::str::from_utf8(&completed.stdout)
        .map_err(|source| WorkflowHostError::InvalidResponse {
            operation: "version".to_owned(),
            detail: source.to_string(),
        })?
        .trim()
        .to_owned();
    if observed != expected {
        return Err(WorkflowHostError::VersionIncompatible { observed, expected });
    }
    Ok(())
}

fn start_or_resume(
    executable: &OsStr,
    state_dir: &Path,
    skill_root: &Path,
    binding: &WorkflowBinding,
    workflow: WorkflowKind,
    limits: HostLimits,
) -> Result<IxEnvelope, WorkflowHostError> {
    match status(executable, state_dir, &binding.run_id, limits)? {
        RunStatus::Found(envelope) => {
            let data = require_data(&envelope, "status")?;
            if binding_items(data).is_empty() && pristine_unbound(data, binding, workflow) {
                return add_binding(executable, state_dir, binding, limits);
            }
            assert_binding(data, binding)?;
            invoke_success(
                executable,
                &[OsString::from("resume"), OsString::from(&binding.run_id)],
                state_dir,
                false,
                "status",
                limits,
            )
        }
        RunStatus::Missing => {
            let created = invoke_success(
                executable,
                &[
                    OsString::from("run"),
                    OsString::from(&binding.workflow),
                    OsString::from("--path"),
                    skill_root.as_os_str().to_owned(),
                    OsString::from("--id"),
                    OsString::from(&binding.run_id),
                ],
                state_dir,
                true,
                "create",
                limits,
            )?;
            let data = require_data(&created, "create")?;
            if !pristine_unbound(data, binding, workflow) {
                return Err(WorkflowHostError::Binding {
                    detail: "new ix-flow run is not pristine before binding".to_owned(),
                });
            }
            add_binding(executable, state_dir, binding, limits)
        }
    }
}

fn add_binding(
    executable: &OsStr,
    state_dir: &Path,
    binding: &WorkflowBinding,
    limits: HostLimits,
) -> Result<IxEnvelope, WorkflowHostError> {
    verify_chain(executable, state_dir, &binding.run_id, limits)?;
    let item = binding_value(binding)?;
    let encoded = serde_json::to_string(&item).map_err(|source| WorkflowHostError::Request {
        detail: source.to_string(),
    })?;
    let envelope = invoke_success(
        executable,
        &[
            OsString::from("add-item"),
            OsString::from(&binding.run_id),
            OsString::from("run_binding"),
            OsString::from("--item"),
            OsString::from(encoded),
        ],
        state_dir,
        true,
        "add-item",
        limits,
    )?;
    assert_binding(require_data(&envelope, "add-item")?, binding)?;
    Ok(envelope)
}

fn status(
    executable: &OsStr,
    state_dir: &Path,
    run_id: &str,
    limits: HostLimits,
) -> Result<RunStatus, WorkflowHostError> {
    let envelope = invoke(
        executable,
        &[OsString::from("status"), OsString::from(run_id)],
        state_dir,
        false,
        "status",
        limits,
    )?;
    validate_command(&envelope, "status", false)?;
    if envelope.ok {
        validate_run_envelope(&envelope, "status", false)?;
        return Ok(RunStatus::Found(Box::new(envelope)));
    }
    if envelope
        .error
        .as_ref()
        .is_some_and(|error| error.code == "instance_not_found")
        && envelope.data.is_none()
        && envelope.events.is_empty()
        && envelope.summary.is_none()
        && envelope.current_phase.is_none()
        && envelope.open_gates.as_ref().is_none_or(Vec::is_empty)
        && envelope.state_version.is_none()
    {
        return Ok(RunStatus::Missing);
    }
    Err(command_error(envelope, "status", false))
}

fn decide(
    executable: &OsStr,
    state_dir: &Path,
    binding: &WorkflowBinding,
    workflow: WorkflowKind,
    choice: Option<DecisionChoice>,
    envelope: IxEnvelope,
    limits: HostLimits,
) -> Result<WorkflowHostResult, WorkflowHostError> {
    let Some(choice) = choice else {
        return Ok(WorkflowHostResult::snapshot(snapshot(&envelope)?));
    };
    let data = require_data(&envelope, "resume")?;
    assert_binding(data, binding)?;
    reject_automatic_terminal_events(data, workflow)?;
    require_human_terminal_gates(data, workflow)?;
    let outcome = workflow.terminal_phase(choice);
    let opposite = workflow.opposite_terminal_phase(choice);
    if data.phase == opposite {
        return Err(WorkflowHostError::DecisionConflict {
            detail: format!("run already has opposite terminal outcome {opposite:?}"),
        });
    }
    if data.phase == outcome {
        return Ok(WorkflowHostResult::decision(decision_event(
            data, binding, workflow, choice,
        )?));
    }
    if data.phase != "decision_ready" {
        return Err(WorkflowHostError::Transition {
            detail: format!("run is not decision-ready: {:?}", data.phase),
        });
    }
    let target = DecisionTarget {
        choice,
        outcome,
        transition_key: format!("decision_ready->{outcome}"),
    };
    let selected_open =
        ensure_decision_gate(executable, state_dir, binding, &target, envelope, limits)?;
    if let Some(gate) = selected_open {
        acknowledge_decision_gate(executable, state_dir, binding, &target, &gate, limits)?;
    }
    let current = advance_terminal(executable, state_dir, binding, outcome, limits)?;
    let current_data = require_data(&current, "advance")?;
    verify_chain(executable, state_dir, &binding.run_id, limits)?;
    Ok(WorkflowHostResult::decision(decision_event(
        current_data,
        binding,
        workflow,
        choice,
    )?))
}

struct DecisionTarget<'a> {
    choice: DecisionChoice,
    outcome: &'a str,
    transition_key: String,
}

fn ensure_decision_gate(
    executable: &OsStr,
    state_dir: &Path,
    binding: &WorkflowBinding,
    target: &DecisionTarget<'_>,
    mut current: IxEnvelope,
    limits: HostLimits,
) -> Result<Option<WorkflowGate>, WorkflowHostError> {
    let mut selected_open =
        selected_open_gate(require_data(&current, "resume")?, &target.transition_key)?;
    let selected_ack = selected_acknowledgement(
        require_data(&current, "resume")?,
        &target.transition_key,
        &binding.decision_owner,
        target.choice,
    )?;
    if selected_open.is_none() && selected_ack.is_none() {
        verify_chain(executable, state_dir, &binding.run_id, limits)?;
        current = invoke_gate_deferred(
            executable,
            &[
                OsString::from("advance"),
                OsString::from(&binding.run_id),
                OsString::from(target.outcome),
            ],
            state_dir,
            limits,
        )?;
        let current_data = require_data(&current, "advance")?;
        assert_binding(current_data, binding)?;
        selected_open = selected_open_gate(current_data, &target.transition_key)?;
        if selected_open.is_none() {
            return Err(WorkflowHostError::OutcomeIndeterminate {
                operation: "advance".to_owned(),
                detail: "deferred transition returned no unique selected gate; retry status to reconcile"
                    .to_owned(),
            });
        }
    }
    Ok(selected_open)
}

fn acknowledge_decision_gate(
    executable: &OsStr,
    state_dir: &Path,
    binding: &WorkflowBinding,
    target: &DecisionTarget<'_>,
    gate: &WorkflowGate,
    limits: HostLimits,
) -> Result<(), WorkflowHostError> {
    verify_chain(executable, state_dir, &binding.run_id, limits)?;
    let current = invoke_success(
        executable,
        &[
            OsString::from("ack"),
            OsString::from(&binding.run_id),
            OsString::from(&gate.token),
            OsString::from("--reviewer"),
            OsString::from(&binding.decision_owner),
            OsString::from("--kind"),
            OsString::from("decision"),
            OsString::from("--note"),
            OsString::from(target.choice.as_str()),
        ],
        state_dir,
        true,
        "ack",
        limits,
    )?;
    let current_data = require_data(&current, "ack")?;
    assert_binding(current_data, binding)?;
    selected_acknowledgement(
        current_data,
        &target.transition_key,
        &binding.decision_owner,
        target.choice,
    )?
    .ok_or_else(|| WorkflowHostError::OutcomeIndeterminate {
        operation: "ack".to_owned(),
        detail: "acknowledgement response lacks the selected owner gate; retry status to reconcile"
            .to_owned(),
    })?;
    Ok(())
}

fn advance_terminal(
    executable: &OsStr,
    state_dir: &Path,
    binding: &WorkflowBinding,
    outcome: &str,
    limits: HostLimits,
) -> Result<IxEnvelope, WorkflowHostError> {
    verify_chain(executable, state_dir, &binding.run_id, limits)?;
    let current = invoke_success(
        executable,
        &[
            OsString::from("advance"),
            OsString::from(&binding.run_id),
            OsString::from(outcome),
        ],
        state_dir,
        true,
        "advance",
        limits,
    )?;
    let current_data = require_data(&current, "advance")?;
    assert_binding(current_data, binding)?;
    if current_data.phase != outcome {
        return Err(WorkflowHostError::OutcomeIndeterminate {
            operation: "advance".to_owned(),
            detail: format!(
                "terminal advance returned phase {:?}, expected {outcome:?}; retry status to reconcile",
                current_data.phase
            ),
        });
    }
    Ok(current)
}

fn invoke_success(
    executable: &OsStr,
    arguments: &[OsString],
    state_dir: &Path,
    mutation_possible: bool,
    expected_command: &str,
    limits: HostLimits,
) -> Result<IxEnvelope, WorkflowHostError> {
    let envelope = invoke(
        executable,
        arguments,
        state_dir,
        mutation_possible,
        expected_command,
        limits,
    )?;
    validate_command(&envelope, expected_command, mutation_possible)?;
    if !envelope.ok {
        return Err(command_error(envelope, expected_command, mutation_possible));
    }
    validate_run_envelope(&envelope, expected_command, mutation_possible)?;
    Ok(envelope)
}

fn invoke_gate_deferred(
    executable: &OsStr,
    arguments: &[OsString],
    state_dir: &Path,
    limits: HostLimits,
) -> Result<IxEnvelope, WorkflowHostError> {
    let envelope = invoke(executable, arguments, state_dir, true, "advance", limits)?;
    validate_command(&envelope, "advance", true)?;
    if envelope.ok || envelope.state.as_deref() != Some("gate_deferred") {
        return Err(WorkflowHostError::OutcomeIndeterminate {
            operation: "advance".to_owned(),
            detail: "terminal transition did not return the ix-flow gate_deferred envelope; retry status to reconcile"
                .to_owned(),
        });
    }
    validate_run_envelope(&envelope, "advance", true)?;
    Ok(envelope)
}

fn invoke(
    executable: &OsStr,
    arguments: &[OsString],
    state_dir: &Path,
    mutation_possible: bool,
    operation: &str,
    limits: HostLimits,
) -> Result<IxEnvelope, WorkflowHostError> {
    invoke_typed(
        executable,
        arguments,
        state_dir,
        mutation_possible,
        operation,
        limits,
    )
}

fn invoke_typed<T>(
    executable: &OsStr,
    arguments: &[OsString],
    state_dir: &Path,
    mutation_possible: bool,
    operation: &str,
    limits: HostLimits,
) -> Result<IxEnvelope<T>, WorkflowHostError>
where
    T: DeserializeOwned,
{
    let mut full_arguments = arguments.to_vec();
    full_arguments.extend([
        OsString::from("--state-dir"),
        state_dir.as_os_str().to_owned(),
        OsString::from("--json"),
    ]);
    let completed = run_process(
        executable,
        &full_arguments
            .iter()
            .map(OsString::as_os_str)
            .collect::<Vec<_>>(),
        mutation_possible,
        operation,
        limits,
    )?;
    let envelope: IxEnvelope<T> = serde_json::from_slice(&completed.stdout).map_err(|source| {
        response_error(
            mutation_possible,
            operation,
            &format!("stdout is not the exact JSON envelope: {source}"),
        )
    })?;
    if envelope.ok && !completed.stderr.is_empty() {
        return Err(response_error(
            mutation_possible,
            operation,
            "successful command emitted stderr",
        ));
    }
    Ok(envelope)
}

fn verify_chain(
    executable: &OsStr,
    state_dir: &Path,
    run_id: &str,
    limits: HostLimits,
) -> Result<(), WorkflowHostError> {
    let envelope: IxEnvelope<IxChainVerification> = invoke_typed(
        executable,
        &[OsString::from("verify"), OsString::from(run_id)],
        state_dir,
        false,
        "verify-chain",
        limits,
    )?;
    validate_command(&envelope, "verify-chain", false)?;
    if !envelope.ok {
        return Err(command_error(envelope, "verify-chain", false));
    }
    let verification = envelope.data.ok_or_else(|| {
        response_error(
            false,
            "verify-chain",
            "successful verification contains no chain result",
        )
    })?;
    let has_all_break_fields = verification.first_break_index.is_some()
        && verification.expected_hash.is_some()
        && verification.actual_hash.is_some();
    let has_any_break_field = verification.first_break_index.is_some()
        || verification.expected_hash.is_some()
        || verification.actual_hash.is_some();
    if verification.ok && !has_any_break_field {
        return Ok(());
    }
    if verification.ok || !has_all_break_fields {
        return Err(WorkflowHostError::InvalidResponse {
            operation: "verify-chain".to_owned(),
            detail: "chain result has an inconsistent success/break shape".to_owned(),
        });
    }
    Err(WorkflowHostError::CommandFailed {
        operation: "verify-chain".to_owned(),
        upstream_code: "event_chain_invalid".to_owned(),
        detail: format!(
            "chain break at event {:?}: expected {:?}, actual {:?}",
            verification.first_break_index, verification.expected_hash, verification.actual_hash
        ),
    })
}

fn run_process(
    executable: &OsStr,
    arguments: &[&OsStr],
    mutation_possible: bool,
    operation: &str,
    limits: HostLimits,
) -> Result<CompletedProcess, WorkflowHostError> {
    process_host::run(
        executable,
        arguments,
        ProcessLimits {
            timeout: limits.timeout,
            max_output_bytes: limits.max_output_bytes,
        },
    )
    .map(|completed| CompletedProcess {
        status: completed.status,
        stdout: completed.stdout,
        stderr: completed.stderr,
    })
    .map_err(|error| match error {
        ProcessError::Unavailable { detail } => WorkflowHostError::Unavailable { detail },
        error => response_error(mutation_possible, operation, &error.to_string()),
    })
}

fn response_error(mutation_possible: bool, operation: &str, detail: &str) -> WorkflowHostError {
    if mutation_possible {
        WorkflowHostError::OutcomeIndeterminate {
            operation: operation.to_owned(),
            detail: detail.to_owned(),
        }
    } else {
        WorkflowHostError::InvalidResponse {
            operation: operation.to_owned(),
            detail: detail.to_owned(),
        }
    }
}

fn validate_command<T>(
    envelope: &IxEnvelope<T>,
    expected: &str,
    mutation_possible: bool,
) -> Result<(), WorkflowHostError> {
    if envelope.command == expected {
        Ok(())
    } else {
        Err(response_error(
            mutation_possible,
            expected,
            &format!("response names command {:?}", envelope.command),
        ))
    }
}

fn command_error<T>(
    envelope: IxEnvelope<T>,
    operation: &str,
    mutation_possible: bool,
) -> WorkflowHostError {
    let Some(error) = envelope.error else {
        return response_error(
            mutation_possible,
            operation,
            "failed envelope contains no structured error",
        );
    };
    WorkflowHostError::CommandFailed {
        operation: operation.to_owned(),
        upstream_code: error.code,
        detail: error.message,
    }
}

fn require_data<'a>(
    envelope: &'a IxEnvelope,
    operation: &str,
) -> Result<&'a IxInstance, WorkflowHostError> {
    envelope
        .data
        .as_ref()
        .ok_or_else(|| WorkflowHostError::InvalidResponse {
            operation: operation.to_owned(),
            detail: "successful response contains no run data".to_owned(),
        })
}

fn validate_run_envelope(
    envelope: &IxEnvelope,
    operation: &str,
    mutation_possible: bool,
) -> Result<(), WorkflowHostError> {
    let data = envelope.data.as_ref().ok_or_else(|| {
        response_error(
            mutation_possible,
            operation,
            "successful response contains no run data",
        )
    })?;
    let summary = envelope.summary.as_ref().ok_or_else(|| {
        response_error(
            mutation_possible,
            operation,
            "successful response contains no run summary",
        )
    })?;
    if envelope.instance_id.as_deref() != Some(&data.id)
        || envelope.current_phase.as_deref() != Some(&data.phase)
        || envelope.state_version != Some(data.state_version)
        || envelope.open_gates.as_deref() != Some(data.open_gates.as_slice())
        || envelope.def_hash.as_deref() != Some(&data.def_hash)
        || summary.id != data.id
        || summary.def_name != data.def_name
        || summary.def_version != data.def_version
        || summary.phase != data.phase
        || summary.state_version != data.state_version
    {
        return Err(response_error(
            mutation_possible,
            operation,
            "envelope identity or summary fields disagree with run data",
        ));
    }
    Ok(())
}

fn binding_value(binding: &WorkflowBinding) -> Result<Value, WorkflowHostError> {
    serde_json::to_value(IxRunBindingItem {
        id: "binding",
        binding,
    })
    .map_err(|source| WorkflowHostError::Request {
        detail: source.to_string(),
    })
}

fn binding_items(data: &IxInstance) -> &[Value] {
    data.items
        .get("run_binding")
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn assert_binding(data: &IxInstance, binding: &WorkflowBinding) -> Result<(), WorkflowHostError> {
    if data.id != binding.run_id
        || data.def_name != binding.workflow
        || data.def_version != binding.workflow_version
    {
        return Err(WorkflowHostError::Binding {
            detail: "run id, workflow, or workflow version differs".to_owned(),
        });
    }
    let expected = binding_value(binding)?;
    if binding_items(data) != [expected] {
        return Err(WorkflowHostError::Binding {
            detail: "run_binding is absent, duplicated, incomplete, or different".to_owned(),
        });
    }
    Ok(())
}

fn pristine_unbound(data: &IxInstance, binding: &WorkflowBinding, workflow: WorkflowKind) -> bool {
    data.def_name == workflow.as_str()
        && data.def_version == binding.workflow_version
        && data.phase == workflow.initial_phase()
        && data.state_version == 0
        && data.items.is_empty()
        && data.open_gates.is_empty()
        && data.satisfied_gates.is_empty()
        && data.events.len() == 1
        && data.events[0].kind == "workflow.created"
        && human_terminal_gates(data, workflow)
}

fn human_terminal_gates(data: &IxInstance, workflow: WorkflowKind) -> bool {
    [DecisionChoice::Accept, DecisionChoice::Reject]
        .into_iter()
        .all(|choice| {
            data.gate_config
                .get(&format!(
                    "decision_ready->{}",
                    workflow.terminal_phase(choice)
                ))
                .is_some_and(|mode| *mode == GateMode::Hitl)
        })
}

fn require_human_terminal_gates(
    data: &IxInstance,
    workflow: WorkflowKind,
) -> Result<(), WorkflowHostError> {
    if human_terminal_gates(data, workflow) {
        Ok(())
    } else {
        Err(WorkflowHostError::DecisionConflict {
            detail: "one or more terminal transitions are not configured hitl".to_owned(),
        })
    }
}

fn reject_automatic_terminal_events(
    data: &IxInstance,
    workflow: WorkflowKind,
) -> Result<(), WorkflowHostError> {
    let terminal = [DecisionChoice::Accept, DecisionChoice::Reject]
        .map(|choice| workflow.terminal_phase(choice));
    for event in data
        .events
        .iter()
        .filter(|event| event.kind == "gate.auto_acked")
    {
        let automatic: IxAutomaticGateEvent = serde_json::from_value(event.payload.clone())
            .map_err(|source| WorkflowHostError::DecisionConflict {
                detail: format!("ix-flow history contains malformed automatic-gate data: {source}"),
            })?;
        if terminal.contains(&automatic.to.as_str()) {
            return Err(WorkflowHostError::DecisionConflict {
                detail: "ix-flow history contains an automatic terminal gate event".to_owned(),
            });
        }
    }
    Ok(())
}

fn selected_open_gate(
    data: &IxInstance,
    transition_key: &str,
) -> Result<Option<WorkflowGate>, WorkflowHostError> {
    let matches = data
        .open_gates
        .iter()
        .filter(|gate| gate.transition_key == transition_key)
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [gate] if gate.kind == transition_key => Ok(Some(gate.clone())),
        [_] => Err(WorkflowHostError::DecisionConflict {
            detail: "selected terminal gate is not hitl".to_owned(),
        }),
        _ => Err(WorkflowHostError::DecisionConflict {
            detail: "selected terminal gate is ambiguous".to_owned(),
        }),
    }
}

fn selected_acknowledgement(
    data: &IxInstance,
    transition_key: &str,
    owner: &str,
    choice: DecisionChoice,
) -> Result<Option<IxGateAcknowledgement>, WorkflowHostError> {
    let all_selected = data
        .satisfied_gates
        .iter()
        .filter(|gate| gate.transition_key == transition_key)
        .collect::<Vec<_>>();
    if all_selected.is_empty() {
        return Ok(None);
    }
    if all_selected.len() == 1
        && all_selected[0].approver == owner
        && all_selected[0].note.as_deref() == Some(choice.as_str())
    {
        Ok(Some(all_selected[0].clone()))
    } else {
        Err(WorkflowHostError::DecisionConflict {
            detail: "terminal acknowledgement is duplicated or attributed differently".to_owned(),
        })
    }
}

fn decision_event(
    data: &IxInstance,
    binding: &WorkflowBinding,
    workflow: WorkflowKind,
    choice: DecisionChoice,
) -> Result<DecisionEvent, WorkflowHostError> {
    let outcome = workflow.terminal_phase(choice);
    let acknowledgements = data
        .events
        .iter()
        .filter(|event| event.kind == "gate.acknowledged")
        .map(|event| {
            serde_json::from_value::<IxGateAcknowledgement>(event.payload.clone()).map_err(
                |source| WorkflowHostError::DecisionConflict {
                    detail: format!(
                        "ix-flow history contains malformed gate acknowledgement: {source}"
                    ),
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let matches = acknowledgements
        .iter()
        .filter(|event| {
            event.to == outcome
                && event.approver == binding.decision_owner
                && event.note.as_deref() == Some(choice.as_str())
        })
        .collect::<Vec<_>>();
    let [event] = matches.as_slice() else {
        return Err(WorkflowHostError::DecisionConflict {
            detail: "terminal decision has no unique owner acknowledgement event".to_owned(),
        });
    };
    Ok(DecisionEvent {
        run_id: binding.run_id.clone(),
        workflow: binding.workflow.clone(),
        workflow_version: binding.workflow_version.clone(),
        owner: binding.decision_owner.clone(),
        choice,
        outcome: outcome.to_owned(),
        timestamp: event.acknowledged_at.clone(),
    })
}

fn snapshot(envelope: &IxEnvelope) -> Result<WorkflowSnapshot, WorkflowHostError> {
    let data = require_data(envelope, &envelope.command)?;
    Ok(WorkflowSnapshot {
        run_id: data.id.clone(),
        workflow: data.def_name.clone(),
        workflow_version: data.def_version.clone(),
        phase: data.phase.clone(),
        state_version: data.state_version,
        next_actions: envelope.next_actions.clone(),
        open_gates: data.open_gates.clone(),
    })
}

enum RunStatus {
    Found(Box<IxEnvelope>),
    Missing,
}

#[derive(Debug)]
struct CompletedProcess {
    status: std::process::ExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowDefinitionIdentity {
    name: String,
    version: String,
    initial_phase: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
enum GateMode {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "hitl")]
    Hitl,
    #[serde(rename = "full-auto")]
    FullAuto,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IxEnvelope<T = IxInstance> {
    ok: bool,
    command: String,
    #[serde(default, rename = "instance_id")]
    instance_id: Option<String>,
    #[serde(default)]
    state: Option<String>,
    data: Option<T>,
    #[serde(default)]
    error: Option<IxError>,
    events: Vec<IxEvent>,
    #[serde(default)]
    summary: Option<IxSummary>,
    #[serde(default)]
    current_phase: Option<String>,
    #[serde(default)]
    #[serde(rename = "transitions_available")]
    _transitions_available: Option<Vec<String>>,
    #[serde(default)]
    open_gates: Option<Vec<WorkflowGate>>,
    #[serde(default)]
    state_version: Option<u64>,
    #[serde(default)]
    #[serde(rename = "def_hash")]
    def_hash: Option<String>,
    #[serde(default, rename = "cli_version")]
    _cli_version: Option<String>,
    next_actions: Vec<WorkflowNextAction>,
    #[serde(rename = "nextActions")]
    _legacy_next_actions: Vec<String>,
    #[serde(default, rename = "recipe_steps")]
    _recipe_steps: Option<Vec<IxRecipeStep>>,
    #[serde(default, rename = "interview_followups")]
    _interview_followups: Option<Vec<IxInterviewFollowup>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IxError {
    code: String,
    message: String,
    #[serde(default)]
    #[serde(rename = "details")]
    _details: Option<Map<String, Value>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IxChainVerification {
    ok: bool,
    #[serde(default, rename = "firstBreakIndex")]
    first_break_index: Option<u64>,
    #[serde(default, rename = "expectedHash")]
    expected_hash: Option<String>,
    #[serde(default, rename = "actualHash")]
    actual_hash: Option<String>,
}

#[derive(Serialize)]
struct IxRunBindingItem<'a> {
    id: &'static str,
    #[serde(flatten)]
    binding: &'a WorkflowBinding,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IxAutomaticGateEvent {
    #[serde(rename = "transitionKey")]
    _transition_key: String,
    #[serde(rename = "from")]
    _from: String,
    to: String,
    #[serde(rename = "defaultGate")]
    _default_gate: GateMode,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IxInstance {
    id: String,
    def_name: String,
    def_version: String,
    #[serde(rename = "defHash")]
    def_hash: String,
    #[serde(rename = "writerVersion")]
    _writer_version: String,
    #[serde(rename = "name")]
    _name: String,
    #[serde(rename = "targets")]
    _targets: Vec<IxTarget>,
    phase: String,
    gate_config: BTreeMap<String, GateMode>,
    #[serde(default)]
    open_gates: Vec<WorkflowGate>,
    #[serde(default)]
    satisfied_gates: Vec<IxGateAcknowledgement>,
    items: BTreeMap<String, Vec<Value>>,
    #[serde(rename = "links")]
    _links: Vec<Value>,
    #[serde(rename = "artifacts")]
    _artifacts: Vec<Value>,
    #[serde(rename = "openQuestions")]
    _open_questions: Vec<Value>,
    events: Vec<IxEvent>,
    state_version: u64,
    #[serde(default)]
    #[serde(rename = "skillPath")]
    _skill_path: Option<IxSkillPath>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IxTarget {
    #[serde(rename = "kind")]
    _kind: String,
    #[serde(rename = "ref")]
    _reference: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IxSkillPath {
    #[serde(default)]
    #[serde(rename = "relative")]
    _relative: Option<String>,
    #[serde(rename = "absolute")]
    _absolute: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IxEvent {
    #[serde(rename = "id")]
    _id: String,
    #[serde(rename = "ts")]
    _ts: String,
    #[serde(rename = "actor")]
    _actor: IxActor,
    kind: String,
    payload: Value,
    #[serde(rename = "prevHash")]
    _prev_hash: String,
    #[serde(rename = "hash")]
    _hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IxActor {
    #[serde(rename = "kind")]
    _kind: String,
    #[serde(rename = "id")]
    _id: String,
    #[serde(default)]
    #[serde(rename = "ackToken")]
    _ack_token: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IxGateAcknowledgement {
    #[serde(rename = "token")]
    _token: String,
    transition_key: String,
    #[serde(rename = "from")]
    _from: String,
    to: String,
    #[serde(rename = "kind")]
    _kind: String,
    #[serde(rename = "issuedAt")]
    _issued_at: String,
    approver: String,
    #[serde(default)]
    note: Option<String>,
    acknowledged_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IxSummary {
    id: String,
    #[serde(rename = "name")]
    _name: String,
    def_name: String,
    def_version: String,
    phase: String,
    state_version: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IxRecipeStep {
    #[serde(rename = "index")]
    _index: u64,
    #[serde(rename = "command")]
    _command: String,
    #[serde(rename = "ok")]
    _ok: bool,
    #[serde(default)]
    #[serde(rename = "state")]
    _state: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IxInterviewFollowup {
    #[serde(rename = "interviewId")]
    _interview_id: String,
    #[serde(rename = "questionKey")]
    _question_key: String,
    #[serde(rename = "prompt")]
    _prompt: String,
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use ix_trace_rs::trace;

    use super::*;

    #[trace("TC-107", "FR-016-AC-3", "FR-016-CON-5")]
    #[test]
    fn tc_107_process_adapter_bounds_time_and_output() {
        let timeout = run_process(
            OsStr::new("sleep"),
            &[OsStr::new("1")],
            false,
            "timeout-probe",
            HostLimits {
                timeout: Duration::from_millis(20),
                max_output_bytes: 64,
            },
        )
        .expect_err("slow host must time out");
        assert_eq!(timeout.code(), "ix_flow_response_invalid");

        let overflow = run_process(
            OsStr::new("printf"),
            &[OsStr::new("0123456789")],
            true,
            "overflow-probe",
            HostLimits {
                timeout: Duration::from_secs(1),
                max_output_bytes: 4,
            },
        )
        .expect_err("large host output must be refused");
        assert_eq!(overflow.code(), "ix_flow_outcome_indeterminate");

        let malformed = invoke(
            OsStr::new("printf"),
            &[OsString::from("{not-json")],
            Path::new("/tmp"),
            false,
            "status",
            HostLimits {
                timeout: Duration::from_secs(1),
                max_output_bytes: 64,
            },
        )
        .expect_err("malformed host response must be refused");
        assert_eq!(malformed.code(), "ix_flow_response_invalid");

        let malformed_after_mutation = invoke(
            OsStr::new("printf"),
            &[OsString::from("{not-json")],
            Path::new("/tmp"),
            true,
            "advance",
            HostLimits {
                timeout: Duration::from_secs(1),
                max_output_bytes: 64,
            },
        )
        .expect_err("malformed mutating response must be indeterminate");
        assert_eq!(
            malformed_after_mutation.code(),
            "ix_flow_outcome_indeterminate"
        );
    }

    #[trace("TC-107", "FR-016-AC-3", "FR-016-CON-5")]
    #[test]
    fn tc_107_response_command_identity_preserves_mutation_uncertainty() {
        let envelope: IxEnvelope<IxChainVerification> = serde_json::from_value(serde_json::json!({
            "ok": false,
            "command": "status",
            "state": "blocked",
            "data": null,
            "error": null,
            "events": [],
            "next_actions": [],
            "nextActions": []
        }))
        .expect("closed ix-flow fixture must deserialize");
        let failure = validate_command(&envelope, "advance", true)
            .expect_err("wrong mutating command identity must fail");
        assert_eq!(failure.code(), "ix_flow_outcome_indeterminate");

        let failure = command_error(envelope, "advance", true);
        assert_eq!(failure.code(), "ix_flow_outcome_indeterminate");
    }
}
