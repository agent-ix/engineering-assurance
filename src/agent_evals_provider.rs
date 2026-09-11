// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! External cli-agent-evals provider for Engineering Assurance scenarios.
//!
//! This binary adapter owns only the consumer's fixture preparation and result
//! assertion semantics. The external runner owns selection, workspaces, agent
//! execution, transcripts, metrics, and report serialization.

use std::{
    collections::BTreeMap,
    env,
    ffi::OsStr,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use engineering_assurance::{evidence::GoverningVersions, workflow::DecisionEvent};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::process_host::{self, ProcessLimits};

const REQUEST_PROTOCOL: &str = "cli-agent-evals.external-provider-request/v1";
const RESULT_PROTOCOL: &str = "cli-agent-evals.external-provider-result/v1";
const MAX_PROVIDER_BYTES: usize = 1024 * 1024;
const MAX_FIXTURE_BYTES: usize = 1024 * 1024;
const MAX_SKILL_FILES: usize = 512;
const MAX_SKILL_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SKILL_DEPTH: usize = 16;
const IX_FLOW_TIMEOUT: Duration = Duration::from_secs(15);
const SNAPSHOT_ENV: &str = "EA_EVAL_GOVERNING_PATH";

#[derive(Debug, Error)]
pub(crate) enum AgentEvalsProviderError {
    #[error("external provider request is invalid")]
    RequestInvalid,
    #[error("external provider scenario is unsupported")]
    ScenarioUnsupported,
    #[error("external provider workspace is invalid")]
    WorkspaceInvalid,
    #[error("external provider fixture is invalid")]
    FixtureInvalid,
    #[error("external provider governing snapshot is invalid")]
    SnapshotInvalid,
    #[error("external provider result is invalid")]
    ResultInvalid,
    #[error("external provider response could not be encoded")]
    ResponseInvalid,
}

impl AgentEvalsProviderError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::RequestInvalid => "agent_evals_provider_request_invalid",
            Self::ScenarioUnsupported => "agent_evals_provider_scenario_unsupported",
            Self::WorkspaceInvalid => "agent_evals_provider_workspace_invalid",
            Self::FixtureInvalid => "agent_evals_provider_fixture_invalid",
            Self::SnapshotInvalid => "agent_evals_provider_snapshot_invalid",
            Self::ResultInvalid => "agent_evals_provider_result_invalid",
            Self::ResponseInvalid => "agent_evals_provider_response_invalid",
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProviderRequest {
    protocol: String,
    operation: ProviderOperation,
    context: ProviderContext,
    #[serde(default)]
    run: Option<ProviderRun>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum ProviderOperation {
    Describe,
    Prepare,
    Assert,
}

impl ProviderOperation {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Describe => "describe",
            Self::Prepare => "prepare",
            Self::Assert => "assert",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderContext {
    scenario: ProviderScenario,
    work_dir: String,
    cwd: String,
    session_id: String,
    report_dir: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProviderScenario {
    id: String,
    #[serde(default)]
    use_case: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    canary: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProviderRun {
    ok: bool,
    exit_reason: String,
    wall_ms: i64,
    #[serde(default, rename = "screenTail")]
    _screen_tail: Option<String>,
    #[serde(default, rename = "stdout")]
    _stdout: Option<String>,
    #[serde(default, rename = "stderr")]
    _stderr: Option<String>,
    #[serde(default, rename = "exitCode")]
    _exit_code: Option<i64>,
}

#[derive(Serialize)]
struct ProviderResponse<T: Serialize> {
    protocol: &'static str,
    operation: &'static str,
    #[serde(flatten)]
    body: T,
}

#[derive(Serialize)]
struct DescribeResponse {
    scenarios: Vec<ProviderScenario>,
}

#[derive(Serialize)]
struct PrepareResponse {
    prompt: String,
    environment: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct AssertResponse {
    assertion: Assertion,
}

#[derive(Serialize)]
struct Assertion {
    ok: bool,
    checks: BTreeMap<String, Value>,
    failures: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    suite_revision: String,
    #[serde(rename = "fixture_revision")]
    revision: String,
    scenarios: BTreeMap<String, FixtureScenario>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureScenario {
    expected: String,
    #[serde(default)]
    choice: Option<String>,
    input: FixtureInput,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureInput {
    decision_boundary: String,
    decision_owner: String,
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    workflow_state_dir: Option<String>,
    #[serde(default)]
    owner_decision: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoverningSnapshot {
    source_revision: String,
    host: VersionIdentity,
    governing: GoverningWithoutWorkflow,
    workflows: BTreeMap<String, VersionIdentity>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct VersionIdentity {
    name: String,
    version: String,
    digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GoverningWithoutWorkflow {
    module: VersionIdentity,
    plugin: VersionIdentity,
    skill: VersionIdentity,
    quire: VersionIdentity,
    quoin: VersionIdentity,
    ix_flow: VersionIdentity,
    schema: VersionIdentity,
    producer: VersionIdentity,
}

#[derive(Serialize)]
struct EvaluationInput<'a> {
    scenario: &'a str,
    suite_revision: &'a str,
    fixture_revision: &'a str,
    input: &'a FixtureInput,
    result_contract: ResultContract,
}

#[derive(Serialize)]
struct ResultContract {
    revision: &'static str,
    required_top_level_fields: [&'static str; 14],
    host: String,
    host_version: String,
    source_revision: String,
    suite_revision: String,
    fixture_revision: String,
    governing: GoverningVersions,
    governing_identities: [&'static str; 9],
    governing_identity_fields: [&'static str; 3],
    governing_digest_format: &'static str,
    observed_outcome: String,
    terminal_event_contract: TerminalContract,
    unsupported_additions: Vec<String>,
    count_type: &'static str,
}

#[derive(Serialize)]
struct TerminalContract {
    required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    required_fields: Option<[&'static str; 7]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workflow_state_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<()>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluationResult {
    host: String,
    host_version: String,
    source_revision: String,
    suite_revision: String,
    fixture_revision: String,
    governing: GoverningVersions,
    command_count: i64,
    elapsed_ms: i64,
    human_prompt_count: i64,
    manual_translation_count: i64,
    repeated_prompt_count: i64,
    observed_outcome: String,
    terminal_event: Option<DecisionEvent>,
    unsupported_additions: Vec<String>,
}

pub(crate) fn execute(root: &Path, bytes: &[u8]) -> Result<Vec<u8>, AgentEvalsProviderError> {
    if bytes.len() > MAX_PROVIDER_BYTES {
        return Err(AgentEvalsProviderError::RequestInvalid);
    }
    let request: ProviderRequest =
        serde_json::from_slice(bytes).map_err(|_| AgentEvalsProviderError::RequestInvalid)?;
    if request.protocol != REQUEST_PROTOCOL {
        return Err(AgentEvalsProviderError::RequestInvalid);
    }
    match request.operation {
        ProviderOperation::Describe => encode(
            ProviderOperation::Describe,
            DescribeResponse {
                scenarios: scenarios(),
            },
        ),
        ProviderOperation::Prepare => prepare(root, &request.context),
        ProviderOperation::Assert => assert_result(root, &request.context, request.run.as_ref()),
    }
}

fn scenarios() -> Vec<ProviderScenario> {
    [
        ("EA-001", "existing-profile", true),
        ("EA-002", "no-profile", false),
        ("EA-003", "malformed-producer", false),
        ("EA-004", "unavailable-producer", false),
        ("EA-005", "interruption-resume", false),
        ("EA-006", "human-acceptance", false),
        ("EA-007", "human-rejection", false),
    ]
    .into_iter()
    .map(|(id, use_case, canary)| ProviderScenario {
        id: id.to_owned(),
        use_case: Some(use_case.to_owned()),
        title: None,
        canary: canary.then_some(true),
    })
    .collect()
}

fn prepare(root: &Path, context: &ProviderContext) -> Result<Vec<u8>, AgentEvalsProviderError> {
    let scenario = scenario_name(&context.scenario)?;
    let fixture = fixture(root)?;
    let expected = fixture
        .scenarios
        .get(scenario)
        .ok_or(AgentEvalsProviderError::ScenarioUnsupported)?;
    let workspace = workspace(context)?;
    copy_skill(root, &workspace)?;
    let snapshot = snapshot(root)?;
    let contract = contract(&fixture, expected, &snapshot)?;
    let input = EvaluationInput {
        scenario,
        suite_revision: &fixture.suite_revision,
        fixture_revision: &fixture.revision,
        input: &expected.input,
        result_contract: contract,
    };
    write_json(&workspace.join("EVALUATION_INPUT.json"), &input)?;
    seed_fixture(&workspace, scenario, root)?;
    encode(
        ProviderOperation::Prepare,
        PrepareResponse {
            prompt: prompt(scenario),
            environment: BTreeMap::new(),
        },
    )
}

fn assert_result(
    root: &Path,
    context: &ProviderContext,
    run: Option<&ProviderRun>,
) -> Result<Vec<u8>, AgentEvalsProviderError> {
    let scenario = scenario_name(&context.scenario)?;
    let fixture = fixture(root)?;
    let expected = fixture
        .scenarios
        .get(scenario)
        .ok_or(AgentEvalsProviderError::ScenarioUnsupported)?;
    let workspace = workspace(context)?;
    let snapshot = snapshot(root)?;
    let contract = contract(&fixture, expected, &snapshot)?;
    let bytes = read_bounded(&workspace.join("EVALUATION_RESULT.json"), MAX_FIXTURE_BYTES)
        .map_err(|()| AgentEvalsProviderError::ResultInvalid)?;
    let raw: Value =
        serde_json::from_slice(&bytes).map_err(|_| AgentEvalsProviderError::ResultInvalid)?;
    let result: EvaluationResult =
        serde_json::from_value(raw.clone()).map_err(|_| AgentEvalsProviderError::ResultInvalid)?;
    let mut failures = Vec::new();
    if !matches!(run, Some(ProviderRun { ok: true, exit_reason, wall_ms, .. }) if exit_reason == "complete" && *wall_ms >= 0)
    {
        failures.push("agent exit was not complete".to_owned());
    }
    verify_result(&result, &contract, &mut failures);
    let mut checks = BTreeMap::new();
    checks.insert("evaluation_result".to_owned(), raw);
    if let Some(proof) = terminal_workflow_evidence(&workspace, &contract, &result, &mut failures) {
        checks.insert("terminal_workflow".to_owned(), proof);
    }
    encode(
        ProviderOperation::Assert,
        AssertResponse {
            assertion: Assertion {
                ok: failures.is_empty(),
                checks,
                failures,
            },
        },
    )
}

fn terminal_workflow_evidence(
    workspace: &Path,
    contract: &ResultContract,
    result: &EvaluationResult,
    failures: &mut Vec<String>,
) -> Option<Value> {
    let terminal = &contract.terminal_event_contract;
    if !terminal.required {
        return None;
    }
    let (
        Some(workflow),
        Some(workflow_version),
        Some(owner),
        Some(outcome),
        Some(run_id),
        Some(state_dir),
    ) = (
        terminal.workflow.as_deref(),
        terminal.workflow_version.as_deref(),
        terminal.owner.as_deref(),
        terminal.outcome.as_deref(),
        terminal.run_id.as_deref(),
        terminal.workflow_state_dir.as_deref(),
    )
    else {
        failures.push("terminal event contract invalid".to_owned());
        return None;
    };
    let Ok(state_dir) = workspace_relative_directory(workspace, state_dir) else {
        failures.push("terminal workflow state path escapes workspace".to_owned());
        return None;
    };
    let Ok(executable) = ix_flow_executable() else {
        failures.push("ix-flow executable is invalid".to_owned());
        return None;
    };
    let history = ix_flow_json(&executable, "history", run_id, &state_dir);
    let verification = ix_flow_json(&executable, "verify", run_id, &state_dir);
    let proof = json_proof(&history, &verification, run_id, &state_dir, workspace);
    let (Ok(history), Ok(verification)) = (history, verification) else {
        failures.push("ix-flow terminal history missing".to_owned());
        return Some(proof);
    };
    validate_terminal_history(
        &history,
        &verification,
        result.terminal_event.as_ref(),
        &TerminalHistoryExpectation {
            workflow,
            workflow_version,
            owner,
            outcome,
            run_id,
        },
        failures,
    );
    Some(proof)
}

fn workspace_relative_directory(workspace: &Path, value: &str) -> Result<PathBuf, ()> {
    let relative = Path::new(value);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(());
    }
    let candidate = workspace.join(relative);
    let metadata = fs::symlink_metadata(&candidate).map_err(|_| ())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(());
    }
    let canonical = fs::canonicalize(candidate).map_err(|_| ())?;
    canonical
        .starts_with(workspace)
        .then_some(canonical)
        .ok_or(())
}

fn ix_flow_executable() -> Result<PathBuf, ()> {
    let path = env::var_os("IX_FLOW_BIN").ok_or(())?;
    let path = PathBuf::from(path);
    let metadata = fs::symlink_metadata(&path).map_err(|_| ())?;
    if !path.is_absolute() || metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(());
    }
    fs::canonicalize(path).map_err(|_| ())
}

fn ix_flow_json(
    executable: &Path,
    operation: &str,
    run_id: &str,
    state_dir: &Path,
) -> Result<Value, ()> {
    let result = process_host::run_configured(
        executable.as_os_str(),
        &[
            OsStr::new(operation),
            OsStr::new(run_id),
            OsStr::new("--state-dir"),
            state_dir.as_os_str(),
            OsStr::new("--json"),
        ],
        None,
        &[],
        &[],
        ProcessLimits {
            timeout: IX_FLOW_TIMEOUT,
            max_output_bytes: MAX_FIXTURE_BYTES,
        },
    )
    .map_err(|_| ())?;
    if !result.status.success() || !result.stderr.is_empty() {
        return Err(());
    }
    serde_json::from_slice(&result.stdout).map_err(|_| ())
}

fn json_proof(
    history: &Result<Value, ()>,
    verification: &Result<Value, ()>,
    run_id: &str,
    state_dir: &Path,
    workspace: &Path,
) -> Value {
    json!({
        "run_id": run_id,
        "state_dir": state_dir.strip_prefix(workspace).ok().map(|value| value.to_string_lossy().replace('\\', "/")),
        "history_digest": history.as_ref().ok().and_then(|value| serde_json::to_vec(value).ok()).map(|bytes| digest(&bytes)),
        "history_ok": history.is_ok(),
        "verification_ok": verification.as_ref().ok().and_then(|value| value.get("ok")).and_then(Value::as_bool) == Some(true),
    })
}

struct TerminalHistoryExpectation<'a> {
    workflow: &'a str,
    workflow_version: &'a str,
    owner: &'a str,
    outcome: &'a str,
    run_id: &'a str,
}

fn validate_terminal_history(
    history: &Value,
    verification: &Value,
    event: Option<&DecisionEvent>,
    expected: &TerminalHistoryExpectation<'_>,
    failures: &mut Vec<String>,
) {
    if verification.get("ok").and_then(Value::as_bool) != Some(true) {
        failures.push("ix-flow verification failed".to_owned());
    }
    let Some(events) = history.get("data").and_then(Value::as_array) else {
        failures.push("ix-flow terminal history missing".to_owned());
        return;
    };
    if history.get("ok").and_then(Value::as_bool) != Some(true)
        || history.get("instance_id").and_then(Value::as_str) != Some(expected.run_id)
        || history.pointer("/summary/id").and_then(Value::as_str) != Some(expected.run_id)
    {
        failures.push("ix-flow run binding mismatch".to_owned());
    }
    if history.get("current_phase").and_then(Value::as_str) != Some(expected.outcome)
        || history.pointer("/summary/phase").and_then(Value::as_str) != Some(expected.outcome)
    {
        failures.push("ix-flow terminal phase mismatch".to_owned());
    }
    if history.pointer("/summary/defName").and_then(Value::as_str) != Some(expected.workflow)
        || history
            .pointer("/summary/defVersion")
            .and_then(Value::as_str)
            != Some(expected.workflow_version)
    {
        failures.push("ix-flow workflow identity mismatch".to_owned());
    }
    let transition = format!("decision_ready->{}", expected.outcome);
    let acknowledgement = events.iter().rev().find(|value| {
        value.get("kind").and_then(Value::as_str) == Some("gate.acknowledged")
            && value
                .pointer("/payload/transitionKey")
                .and_then(Value::as_str)
                == Some(transition.as_str())
    });
    if acknowledgement
        .and_then(|value| value.pointer("/payload/approver"))
        .and_then(Value::as_str)
        != Some(expected.owner)
    {
        failures.push("ix-flow terminal owner acknowledgement missing".to_owned());
    }
    let terminal = events.iter().rev().find(|value| {
        value.get("kind").and_then(Value::as_str) == Some("phase.advanced")
            && value.pointer("/payload/from").and_then(Value::as_str) == Some("decision_ready")
            && value.pointer("/payload/to").and_then(Value::as_str) == Some(expected.outcome)
    });
    if terminal
        .and_then(|value| value.get("ts"))
        .and_then(Value::as_str)
        != event.map(|value| value.timestamp.as_str())
    {
        failures.push("ix-flow terminal timestamp mismatch".to_owned());
    }
}

fn digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let mut value = String::with_capacity(64);
    for byte in hasher.finalize() {
        use std::fmt::Write as _;
        let _ = write!(value, "{byte:02x}");
    }
    value
}

fn scenario_name(scenario: &ProviderScenario) -> Result<&str, AgentEvalsProviderError> {
    let Some(use_case) = scenario.use_case.as_deref() else {
        return Err(AgentEvalsProviderError::ScenarioUnsupported);
    };
    if scenarios().iter().any(|candidate| {
        candidate.id == scenario.id && candidate.use_case.as_deref() == Some(use_case)
    }) {
        Ok(use_case)
    } else {
        Err(AgentEvalsProviderError::ScenarioUnsupported)
    }
}

fn fixture(root: &Path) -> Result<Fixture, AgentEvalsProviderError> {
    let bytes = read_bounded(&root.join("evals/fixtures/suite.json"), MAX_FIXTURE_BYTES)
        .map_err(|()| AgentEvalsProviderError::FixtureInvalid)?;
    serde_json::from_slice(&bytes).map_err(|_| AgentEvalsProviderError::FixtureInvalid)
}

fn snapshot(root: &Path) -> Result<GoverningSnapshot, AgentEvalsProviderError> {
    let path = env::var_os(SNAPSHOT_ENV).ok_or(AgentEvalsProviderError::SnapshotInvalid)?;
    let root = canonical_directory(root).map_err(|_| AgentEvalsProviderError::SnapshotInvalid)?;
    let allowed = root.join(".agent-evals");
    let path = Path::new(&path);
    let metadata =
        fs::symlink_metadata(path).map_err(|_| AgentEvalsProviderError::SnapshotInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(AgentEvalsProviderError::SnapshotInvalid);
    }
    let canonical = fs::canonicalize(path).map_err(|_| AgentEvalsProviderError::SnapshotInvalid)?;
    if !canonical.starts_with(&allowed) {
        return Err(AgentEvalsProviderError::SnapshotInvalid);
    }
    let bytes = read_bounded(&canonical, MAX_FIXTURE_BYTES)
        .map_err(|()| AgentEvalsProviderError::SnapshotInvalid)?;
    serde_json::from_slice(&bytes).map_err(|_| AgentEvalsProviderError::SnapshotInvalid)
}

fn contract(
    fixture: &Fixture,
    expected: &FixtureScenario,
    snapshot: &GoverningSnapshot,
) -> Result<ResultContract, AgentEvalsProviderError> {
    let workflow_name = if matches!(expected.choice.as_deref(), Some("accept" | "reject"))
        || expected.input.run_id.is_some()
    {
        "architecture-evaluation"
    } else {
        "assurance-intake"
    };
    let workflow = snapshot
        .workflows
        .get(workflow_name)
        .ok_or(AgentEvalsProviderError::SnapshotInvalid)?
        .clone();
    let governing = governing(snapshot, &workflow);
    if expected.input.owner_decision.as_deref() != expected.choice.as_deref() {
        return Err(AgentEvalsProviderError::FixtureInvalid);
    }
    let terminal_event_contract = match (
        &expected.choice,
        &expected.input.run_id,
        &expected.input.workflow_state_dir,
    ) {
        (Some(choice), Some(run_id), Some(workflow_state_dir)) => TerminalContract {
            required: true,
            required_fields: Some([
                "run_id",
                "workflow",
                "workflow_version",
                "owner",
                "choice",
                "outcome",
                "timestamp",
            ]),
            workflow: Some(workflow.name),
            workflow_version: Some(workflow.version),
            owner: Some(expected.input.decision_owner.clone()),
            choice: Some(choice.clone()),
            outcome: Some(expected.expected.clone()),
            run_id: Some(run_id.clone()),
            workflow_state_dir: Some(workflow_state_dir.clone()),
            timestamp: Some(
                "copy from the ix-flow phase.advanced event whose payload moves from decision_ready to the required terminal outcome; never use the gate.acknowledged timestamp",
            ),
            value: None,
        },
        (None, None, None) => TerminalContract {
            required: false,
            required_fields: None,
            workflow: None,
            workflow_version: None,
            owner: None,
            choice: None,
            outcome: None,
            run_id: None,
            workflow_state_dir: None,
            timestamp: None,
            value: Some(()),
        },
        _ => return Err(AgentEvalsProviderError::FixtureInvalid),
    };
    Ok(ResultContract {
        revision: "evaluation-result-v1",
        required_top_level_fields: [
            "host",
            "host_version",
            "source_revision",
            "suite_revision",
            "fixture_revision",
            "governing",
            "command_count",
            "elapsed_ms",
            "human_prompt_count",
            "manual_translation_count",
            "repeated_prompt_count",
            "observed_outcome",
            "terminal_event",
            "unsupported_additions",
        ],
        host: snapshot.host.name.clone(),
        host_version: snapshot.host.version.clone(),
        source_revision: snapshot.source_revision.clone(),
        suite_revision: fixture.suite_revision.clone(),
        fixture_revision: fixture.revision.clone(),
        governing,
        governing_identities: [
            "module", "plugin", "skill", "workflow", "quire", "quoin", "ix_flow", "schema",
            "producer",
        ],
        governing_identity_fields: ["name", "version", "digest"],
        governing_digest_format: "lowercase sha256",
        observed_outcome: expected.expected.clone(),
        terminal_event_contract,
        unsupported_additions: Vec::new(),
        count_type: "non-negative integer",
    })
}

fn governing(snapshot: &GoverningSnapshot, workflow: &VersionIdentity) -> GoverningVersions {
    GoverningVersions {
        module: identity(snapshot.governing.module.clone()),
        plugin: identity(snapshot.governing.plugin.clone()),
        skill: identity(snapshot.governing.skill.clone()),
        workflow: identity(workflow.clone()),
        quire: identity(snapshot.governing.quire.clone()),
        quoin: identity(snapshot.governing.quoin.clone()),
        ix_flow: identity(snapshot.governing.ix_flow.clone()),
        schema: identity(snapshot.governing.schema.clone()),
        producer: identity(snapshot.governing.producer.clone()),
    }
}

fn identity(value: VersionIdentity) -> engineering_assurance::evidence::VersionIdentity {
    engineering_assurance::evidence::VersionIdentity {
        name: value.name,
        version: value.version,
        digest: value.digest,
    }
}

fn verify_result(result: &EvaluationResult, contract: &ResultContract, failures: &mut Vec<String>) {
    for (name, actual, required) in [
        ("host", result.host.as_str(), contract.host.as_str()),
        (
            "host_version",
            result.host_version.as_str(),
            contract.host_version.as_str(),
        ),
        (
            "source_revision",
            result.source_revision.as_str(),
            contract.source_revision.as_str(),
        ),
        (
            "suite_revision",
            result.suite_revision.as_str(),
            contract.suite_revision.as_str(),
        ),
        (
            "fixture_revision",
            result.fixture_revision.as_str(),
            contract.fixture_revision.as_str(),
        ),
        (
            "observed_outcome",
            result.observed_outcome.as_str(),
            contract.observed_outcome.as_str(),
        ),
    ] {
        if actual != required {
            failures.push(format!("result field mismatch: {name}"));
        }
    }
    if result.governing != contract.governing {
        failures.push("governing versions mismatch".to_owned());
    }
    if !result.unsupported_additions.is_empty() {
        failures.push("unsupported additions present".to_owned());
    }
    if [
        result.command_count,
        result.elapsed_ms,
        result.human_prompt_count,
        result.manual_translation_count,
        result.repeated_prompt_count,
    ]
    .into_iter()
    .any(|value| value < 0)
    {
        failures.push("result count invalid".to_owned());
    }
    match (&contract.terminal_event_contract, &result.terminal_event) {
        (
            TerminalContract {
                required: false, ..
            },
            None,
        ) => {}
        (
            TerminalContract {
                required: false, ..
            },
            Some(_),
        ) => failures.push("unexpected terminal event".to_owned()),
        (
            TerminalContract {
                required: true,
                workflow: Some(workflow),
                workflow_version: Some(workflow_version),
                owner: Some(owner),
                choice: Some(choice),
                outcome: Some(outcome),
                run_id: Some(run_id),
                ..
            },
            Some(event),
        ) => {
            if (
                &event.workflow,
                &event.workflow_version,
                &event.owner,
                &event.choice.to_string(),
                &event.outcome,
                &event.run_id,
            ) != (workflow, workflow_version, owner, choice, outcome, run_id)
                || time::OffsetDateTime::parse(
                    &event.timestamp,
                    &time::format_description::well_known::Rfc3339,
                )
                .is_err()
            {
                failures.push("terminal event mismatch".to_owned());
            }
        }
        (TerminalContract { required: true, .. }, None) => {
            failures.push("terminal event missing".to_owned());
        }
        (TerminalContract { required: true, .. }, Some(_)) => {
            failures.push("terminal event contract invalid".to_owned());
        }
    }
}

fn workspace(context: &ProviderContext) -> Result<PathBuf, AgentEvalsProviderError> {
    let work_dir = Path::new(&context.work_dir);
    let cwd = Path::new(&context.cwd);
    if context.session_id.is_empty()
        || context.report_dir.is_empty()
        || !work_dir.is_absolute()
        || !cwd.is_absolute()
    {
        return Err(AgentEvalsProviderError::WorkspaceInvalid);
    }
    let work_dir = canonical_directory(work_dir)?;
    let cwd = canonical_directory(cwd)?;
    if !cwd.starts_with(&work_dir) {
        return Err(AgentEvalsProviderError::WorkspaceInvalid);
    }
    Ok(cwd)
}

fn canonical_directory(path: &Path) -> Result<PathBuf, AgentEvalsProviderError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(AgentEvalsProviderError::WorkspaceInvalid);
    }
    fs::canonicalize(path).map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)
}

fn copy_skill(root: &Path, workspace: &Path) -> Result<(), AgentEvalsProviderError> {
    let source = root.join("engineering_assurance/skills/assurance-onboarding");
    for target in [".agents/skills", ".claude/skills", ".github/skills"] {
        let mut budget = CopyBudget::default();
        copy_tree(
            &source,
            &workspace.join(target).join("assurance-onboarding"),
            0,
            &mut budget,
        )?;
    }
    Ok(())
}

#[derive(Default)]
struct CopyBudget {
    files: usize,
    bytes: u64,
}

fn copy_tree(
    source: &Path,
    target: &Path,
    depth: usize,
    budget: &mut CopyBudget,
) -> Result<(), AgentEvalsProviderError> {
    if depth > MAX_SKILL_DEPTH {
        return Err(AgentEvalsProviderError::WorkspaceInvalid);
    }
    let metadata =
        fs::symlink_metadata(source).map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(AgentEvalsProviderError::WorkspaceInvalid);
    }
    fs::create_dir_all(target).map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)?;
    for entry in fs::read_dir(source).map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)? {
        let entry = entry.map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)?;
        let from = entry.path();
        let to = target.join(entry.file_name());
        let kind =
            fs::symlink_metadata(&from).map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)?;
        if kind.file_type().is_symlink() {
            return Err(AgentEvalsProviderError::WorkspaceInvalid);
        }
        if kind.is_dir() {
            copy_tree(&from, &to, depth + 1, budget)?;
        } else if kind.is_file() {
            budget.files = budget
                .files
                .checked_add(1)
                .ok_or(AgentEvalsProviderError::WorkspaceInvalid)?;
            budget.bytes = budget
                .bytes
                .checked_add(kind.len())
                .ok_or(AgentEvalsProviderError::WorkspaceInvalid)?;
            if budget.files > MAX_SKILL_FILES || budget.bytes > MAX_SKILL_BYTES {
                return Err(AgentEvalsProviderError::WorkspaceInvalid);
            }
            fs::copy(&from, &to).map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)?;
        } else {
            return Err(AgentEvalsProviderError::WorkspaceInvalid);
        }
    }
    Ok(())
}

fn seed_fixture(
    workspace: &Path,
    scenario: &str,
    root: &Path,
) -> Result<(), AgentEvalsProviderError> {
    let spec = workspace.join("spec");
    fs::create_dir_all(&spec).map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)?;
    if scenario == "existing-profile" {
        fs::copy(
            root.join("engineering_assurance/skeletons/AssuranceProfile.md"),
            spec.join("AP-001.md"),
        )
        .map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)?;
    }
    if let Some(state_dir) = fixture_input(root, scenario)?.workflow_state_dir {
        let path = workspace.join(state_dir);
        fs::create_dir_all(&path).map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)?;
    }
    if matches!(scenario, "malformed-producer" | "unavailable-producer") {
        let producers = workspace.join("producers");
        fs::create_dir_all(&producers).map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)?;
        let content = if scenario == "malformed-producer" {
            "{invalid\n"
        } else {
            "{\"command\":\"fictional-missing-producer\"}\n"
        };
        fs::write(producers.join("fictional.json"), content)
            .map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)?;
    }
    Ok(())
}

fn fixture_input(root: &Path, scenario: &str) -> Result<FixtureInput, AgentEvalsProviderError> {
    fixture(root)?
        .scenarios
        .remove(scenario)
        .map(|scenario| scenario.input)
        .ok_or(AgentEvalsProviderError::ScenarioUnsupported)
}

fn prompt(scenario: &str) -> String {
    format!(
        "Use the installed assurance-onboarding skill for this fictional repository. Execute the {scenario} scenario from EVALUATION_INPUT.json. Do not invent artifacts, evidence, applicability decisions, or terminal outcomes. Write EVALUATION_RESULT.json exactly to its result_contract, copying identity fields verbatim and using the declared outcome. For a required terminal event, copy only the real ix-flow decision transition timestamp and do not invent values."
    )
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), AgentEvalsProviderError> {
    let mut bytes =
        serde_json::to_vec_pretty(value).map_err(|_| AgentEvalsProviderError::FixtureInvalid)?;
    bytes.push(b'\n');
    fs::write(path, bytes).map_err(|_| AgentEvalsProviderError::WorkspaceInvalid)
}

fn read_bounded(path: &Path, maximum: usize) -> Result<Vec<u8>, ()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > u64::try_from(maximum).unwrap_or(u64::MAX)
    {
        return Err(());
    }
    let mut file = fs::File::open(path).map_err(|_| ())?;
    let mut bytes = Vec::new();
    file.by_ref()
        .take(u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    (bytes.len() <= maximum).then_some(bytes).ok_or(())
}

fn encode<T: Serialize>(
    operation: ProviderOperation,
    body: T,
) -> Result<Vec<u8>, AgentEvalsProviderError> {
    let mut bytes = serde_json::to_vec(&ProviderResponse {
        protocol: RESULT_PROTOCOL,
        operation: operation.as_str(),
        body,
    })
    .map_err(|_| AgentEvalsProviderError::ResponseInvalid)?;
    bytes.push(b'\n');
    Ok(bytes)
}
