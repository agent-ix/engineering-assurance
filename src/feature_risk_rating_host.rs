// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Human-gated feature-risk ratings and their signed-commit acceptance check.

use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    fs,
    path::{Component, Path, PathBuf},
    time::Duration,
};

use engineering_assurance::{
    release::frontmatter,
    structured_yaml::parse_unambiguous_yaml_json,
    workflow::{
        DecisionChoice, DecisionEvent, REQUEST_PROTOCOL as HOST_PROTOCOL, WorkflowBinding,
        WorkflowHostOutcome, WorkflowHostRequest, WorkflowOperation, WorkflowSnapshot,
    },
};
use ix_trace_rs::implements;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::{
    process_host::{self, ProcessError, ProcessLimits},
    signature_verify_host::{self, REMOVED_ENVIRONMENT},
    workflow_host::{self, BoundRun, WorkflowHostError},
};

const WORKFLOW_REQUEST_PROTOCOL: &str = "engineering-assurance.feature-risk-rating-request/v1";
const WORKFLOW_RESULT_PROTOCOL: &str = "engineering-assurance.feature-risk-rating-result/v1";
const ACCEPTANCE_REQUEST_PROTOCOL: &str = "engineering-assurance.rating-acceptance-request/v1";
const ACCEPTANCE_RESULT_PROTOCOL: &str = "engineering-assurance.rating-acceptance-result/v1";
const WORKFLOW: &str = "feature-risk-rating";
const WORKFLOW_VERSION: &str = "0.1.0";
const SIGNATURE_REQUEST_PROTOCOL: &str = "engineering-assurance.signature-verify-request/v1";
const SIGNATURE_RESULT_PROTOCOL: &str = "engineering-assurance.signature-verify-result/v1";
const ACCEPTANCE_MEANING: &str = "feature-risk-rating-accepted";
const MAX_DOCUMENT_BYTES: u64 = 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Error)]
pub(crate) enum FeatureRiskRatingError {
    #[error("invalid feature-risk-rating request: {detail}")]
    Request { detail: String },
    #[error("invalid rating-acceptance request: {detail}")]
    AcceptanceRequest { detail: String },
    #[error("feature-risk-rating workflow failed: {0}")]
    Workflow(#[from] WorkflowHostError),
    #[error("validation could not list unrated features: {detail}")]
    Validation { detail: String },
    #[error("rating file is invalid: {detail}")]
    RatingFile { detail: String },
    #[error("could not encode feature-risk-rating result")]
    Encoding,
}

impl FeatureRiskRatingError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Request { .. } => "feature_risk_rating_request_invalid",
            Self::AcceptanceRequest { .. } => "rating_acceptance_request_invalid",
            Self::Workflow(error) => error.code(),
            Self::Validation { .. } => "feature_risk_rating_validation_failed",
            Self::RatingFile { .. } => "rating_acceptance_file_invalid",
            Self::Encoding => "feature_risk_rating_result_encoding_failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Operation {
    Present,
    Decide,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Choice {
    Accept,
    Reject,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Rating {
    HighProcessRisk,
    NotHighProcessRisk,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkflowRequest {
    protocol: String,
    operation: Operation,
    state_dir: String,
    skill_root: String,
    ix_flow_executable: String,
    run_id: String,
    decision_owner: String,
    repository: String,
    repository_id: String,
    quire_executable: String,
    allowed_signers: Option<String>,
    rating_id: Option<String>,
    rating_path: Option<String>,
    rating: Option<Rating>,
    rated_by: Option<String>,
    intended_use: Option<String>,
    fr_ids: Option<Vec<String>>,
    rationale: Option<String>,
    assurance_activities: Option<String>,
    choice: Option<Choice>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptanceRequest {
    protocol: String,
    repository: PathBuf,
    allowed_signers: PathBuf,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum WorkflowOutput {
    Proposal {
        presentation: Value,
        snapshot: WorkflowSnapshot,
    },
    Waiting {
        snapshot: WorkflowSnapshot,
    },
    Decision {
        decision: DecisionEvent,
        #[serde(skip_serializing_if = "Option::is_none")]
        document: Option<RatingDocument>,
    },
}

#[derive(Serialize)]
struct WorkflowResultLine {
    protocol: &'static str,
    #[serde(flatten)]
    output: WorkflowOutput,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct RatingDocument {
    path: String,
    document: String,
    commit_command: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
struct RatingDraft {
    rating_id: String,
    rating_path: String,
    rating: Rating,
    rated_by: String,
    intended_use: String,
    fr_ids: Vec<String>,
    rationale: String,
    assurance_activities: String,
}

#[derive(Serialize)]
struct AcceptanceResultLine {
    protocol: &'static str,
    results: Vec<RatingResult>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
struct RatingResult {
    id: String,
    path: String,
    commit: Option<String>,
    status: &'static str,
    reason: Option<&'static str>,
}

#[derive(Serialize)]
struct Boundary<'a> {
    repository_id: &'a str,
}

pub(crate) fn execute_workflow(bytes: &[u8]) -> Result<Vec<u8>, FeatureRiskRatingError> {
    let request: WorkflowRequest =
        serde_json::from_slice(bytes).map_err(|error| FeatureRiskRatingError::Request {
            detail: error.to_string(),
        })?;
    validate_workflow_request(&request)?;
    let mut run = workflow_host::bind_run(
        &host_request(&request),
        request.operation == Operation::Present,
    )?;
    let output = match request.operation {
        Operation::Present => present(&request, &mut run)?,
        Operation::Decide => decide(&request, run)?,
    };
    encode_workflow(output)
}

fn invalid_request(detail: impl Into<String>) -> FeatureRiskRatingError {
    FeatureRiskRatingError::Request {
        detail: detail.into(),
    }
}

fn validate_workflow_request(request: &WorkflowRequest) -> Result<(), FeatureRiskRatingError> {
    if request.protocol != WORKFLOW_REQUEST_PROTOCOL {
        return Err(invalid_request("unsupported protocol"));
    }
    if !Path::new(&request.repository).is_absolute()
        || !Path::new(&request.quire_executable).is_absolute()
    {
        return Err(invalid_request(
            "repository and quire_executable must be absolute paths",
        ));
    }
    if !valid_repository_id(&request.repository_id) {
        return Err(invalid_request("repository_id must be an org/repo name"));
    }
    match request.operation {
        Operation::Present => {
            if request.choice.is_some() {
                return Err(invalid_request("present does not take a choice"));
            }
            validate_draft_fields(request)?;
            validate_decision_owner(
                &request.decision_owner,
                request.rated_by.as_deref().unwrap_or_default(),
            )?;
        }
        Operation::Decide => {
            if request.choice.is_none() {
                return Err(invalid_request("decide requires choice"));
            }
            if request.rating_id.is_some()
                || request.rating_path.is_some()
                || request.rating.is_some()
                || request.rated_by.is_some()
                || request.intended_use.is_some()
                || request.fr_ids.is_some()
                || request.rationale.is_some()
                || request.assurance_activities.is_some()
                || request.allowed_signers.is_some()
            {
                return Err(invalid_request("decide takes no rating data"));
            }
        }
    }
    Ok(())
}

fn validate_draft_fields(request: &WorkflowRequest) -> Result<(), FeatureRiskRatingError> {
    let required = [
        ("rating_id", request.rating_id.as_deref()),
        ("rating_path", request.rating_path.as_deref()),
        ("rated_by", request.rated_by.as_deref()),
        ("intended_use", request.intended_use.as_deref()),
        ("rationale", request.rationale.as_deref()),
        (
            "assurance_activities",
            request.assurance_activities.as_deref(),
        ),
        ("allowed_signers", request.allowed_signers.as_deref()),
    ];
    if let Some((field, _)) = required
        .into_iter()
        .find(|(_, value)| value.is_none_or(|value| value.trim().is_empty()))
    {
        return Err(invalid_request(format!(
            "present requires non-empty {field}"
        )));
    }
    if request.rating.is_none() || request.fr_ids.as_ref().is_none_or(Vec::is_empty) {
        return Err(invalid_request(
            "present requires rating and at least one fr_id",
        ));
    }
    if request
        .allowed_signers
        .as_deref()
        .is_some_and(|path| !Path::new(path).is_absolute())
    {
        return Err(invalid_request("allowed_signers must be an absolute path"));
    }
    if request
        .rating_id
        .as_deref()
        .is_some_and(|id| !valid_rating_id(id))
    {
        return Err(invalid_request("rating_id must match FRR-NNN"));
    }
    if request
        .rating_path
        .as_deref()
        .is_some_and(|path| !valid_rating_path(path))
    {
        return Err(invalid_request("rating_path must be under spec/validation"));
    }
    if request.fr_ids.as_ref().is_some_and(|ids| {
        ids.iter().any(|id| !valid_fr_id(id))
            || ids.iter().collect::<BTreeSet<_>>().len() != ids.len()
    }) {
        return Err(invalid_request("fr_ids must be unique FR-NNN identifiers"));
    }
    Ok(())
}

fn validate_decision_owner(
    decision_owner: &str,
    rated_by: &str,
) -> Result<(), FeatureRiskRatingError> {
    if decision_owner != rated_by {
        return Err(invalid_request(
            "decision_owner must equal the rating's rated_by principal",
        ));
    }
    Ok(())
}

fn valid_repository_id(value: &str) -> bool {
    let mut parts = value.split('/');
    matches!((parts.next(), parts.next(), parts.next()), (Some(org), Some(repo), None)
        if !org.is_empty() && !repo.is_empty()
            && org.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            && repo.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')))
}

fn valid_fr_id(value: &str) -> bool {
    value
        .strip_prefix("FR-")
        .is_some_and(|digits| digits.len() == 3 && digits.bytes().all(|byte| byte.is_ascii_digit()))
}

fn valid_rating_id(value: &str) -> bool {
    value
        .strip_prefix("FRR-")
        .is_some_and(|digits| digits.len() == 3 && digits.bytes().all(|byte| byte.is_ascii_digit()))
}

fn valid_rating_path(value: &str) -> bool {
    let path = Path::new(value);
    !path.is_absolute()
        && path.starts_with("spec/validation")
        && path.extension().is_some_and(|extension| extension == "md")
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn host_request(request: &WorkflowRequest) -> WorkflowHostRequest {
    let boundary = serde_json::to_string(&Boundary {
        repository_id: &request.repository_id,
    })
    .unwrap_or_default();
    WorkflowHostRequest {
        protocol: HOST_PROTOCOL.to_owned(),
        operation: WorkflowOperation::StartOrResume,
        state_dir: request.state_dir.clone(),
        skill_root: request.skill_root.clone(),
        ix_flow_executable: request.ix_flow_executable.clone(),
        binding: WorkflowBinding {
            run_id: request.run_id.clone(),
            repository_id: request.repository.clone(),
            workflow: WORKFLOW.to_owned(),
            workflow_version: WORKFLOW_VERSION.to_owned(),
            decision_boundary: boundary,
            decision_owner: request.decision_owner.clone(),
        },
        choice: None,
    }
}

fn present(
    request: &WorkflowRequest,
    run: &mut BoundRun,
) -> Result<WorkflowOutput, FeatureRiskRatingError> {
    let existing = run.items("rating_proposal")?.first().cloned();
    let proposal = if let Some(proposal) = existing {
        proposal
    } else {
        let repository = Path::new(&request.repository);
        let unrated =
            validation_unrated_features(repository, Path::new(&request.quire_executable))?;
        let allowed_signers = request.allowed_signers.as_deref().unwrap_or_default();
        let rated_by = request.rated_by.as_deref().unwrap_or_default();
        validate_rated_by(rated_by, allowed_signers)?;
        let draft = RatingDraft {
            rating_id: request.rating_id.clone().unwrap_or_default(),
            rating_path: request.rating_path.clone().unwrap_or_default(),
            rating: request.rating.unwrap_or(Rating::NotHighProcessRisk),
            rated_by: rated_by.to_owned(),
            intended_use: request.intended_use.clone().unwrap_or_default(),
            fr_ids: request.fr_ids.clone().unwrap_or_default(),
            rationale: request.rationale.clone().unwrap_or_default(),
            assurance_activities: request.assurance_activities.clone().unwrap_or_default(),
        };
        if draft
            .fr_ids
            .iter()
            .any(|id| !unrated.iter().any(|feature| feature["id"] == *id))
        {
            return Err(invalid_request(
                "fr_ids must be selected from the validator's unrated features",
            ));
        }
        if run.items("rating_request")?.is_empty() {
            let answers = json!({
                "repository": request.repository,
                "decision_owner": request.decision_owner,
                "fr_ids": draft.fr_ids.join(", "),
            });
            run.record_answers("rating", &answers)?;
        }
        if run.phase()? == "capture" {
            run.advance("proposal_ready")?;
        }
        let proposal = json!({
            "id": "proposal",
            "repository": request.repository,
            "unrated_features": unrated,
            "draft": draft,
            "draft_status": "draft, not accepted",
        });
        run.add_item("rating_proposal", &proposal)?;
        proposal
    };
    if run.phase()? == "proposal_ready" {
        run.advance("decision_ready")?;
    }
    let snapshot = run.snapshot()?;
    Ok(WorkflowOutput::Proposal {
        presentation: proposal,
        snapshot,
    })
}

fn decide(
    request: &WorkflowRequest,
    mut run: BoundRun,
) -> Result<WorkflowOutput, FeatureRiskRatingError> {
    let proposal = run
        .items("rating_proposal")?
        .first()
        .cloned()
        .ok_or_else(|| invalid_request("rating proposal is missing; present first"))?;
    let draft: RatingDraft = serde_json::from_value(proposal["draft"].clone())
        .map_err(|_| invalid_request("stored rating draft is malformed"))?;
    validate_decision_owner(&request.decision_owner, &draft.rated_by)?;

    let choice = match request.choice {
        Some(Choice::Accept) => DecisionChoice::Accept,
        Some(Choice::Reject) => DecisionChoice::Reject,
        None => return Err(invalid_request("decide requires choice")),
    };
    let positive_phase = if choice == DecisionChoice::Accept {
        "recorded"
    } else {
        "abandoned"
    };
    if !run.terminal_acknowledged(positive_phase)? {
        return Ok(WorkflowOutput::Waiting {
            snapshot: run.snapshot()?,
        });
    }
    let document = if choice == DecisionChoice::Accept {
        let accepted_ids = draft.fr_ids.as_slice();
        let proposed_ids: BTreeSet<&str> = proposal["unrated_features"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|feature| feature["id"].as_str())
            .collect();
        if accepted_ids
            .iter()
            .any(|id| !proposed_ids.contains(id.as_str()))
        {
            return Err(invalid_request(
                "fr_ids must be selected from the presented unrated features",
            ));
        }
        let document = render_rating_document(&request.repository_id, &draft);
        let record = RatingDocument {
            path: draft.rating_path.clone(),
            commit_command: commit_command(&draft.rating_path),
            document,
        };
        let item = json!({
            "id": "outcome",
            "status": "draft, not accepted until signed commit verification",
            "document": record.document,
            "record_path": record.path,
            "commit_command": record.commit_command,
        });
        if run.items("rating_outcome")?.is_empty() {
            run.add_item("rating_outcome", &item)?;
        } else {
            let stored = &run.items("rating_outcome")?[0];
            if stored["document"] != record.document || stored["record_path"] != record.path {
                return Err(invalid_request(
                    "the acknowledged rating outcome cannot be changed",
                ));
            }
        }
        Some(record)
    } else {
        None
    };
    let result = run.decide(choice)?;
    let decision = match result.outcome {
        WorkflowHostOutcome::Decision { decision } => decision,
        WorkflowHostOutcome::Snapshot { .. } => {
            return Err(invalid_request(
                "the human gate returned no terminal decision",
            ));
        }
    };
    Ok(WorkflowOutput::Decision { decision, document })
}

fn allowed_signer_names(path: &str) -> Result<BTreeSet<String>, FeatureRiskRatingError> {
    let metadata = fs::metadata(path)
        .map_err(|error| invalid_request(format!("allowed_signers is unreadable: {error}")))?;
    if !metadata.is_file() || metadata.len() > MAX_DOCUMENT_BYTES {
        return Err(invalid_request(
            "allowed_signers must be a regular file within the size limit",
        ));
    }
    let bytes = fs::read(path)
        .map_err(|error| invalid_request(format!("allowed_signers is unreadable: {error}")))?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| invalid_request("allowed_signers is not UTF-8"))?;
    let mut names = BTreeSet::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        if let Some(principals) = line.split_whitespace().next() {
            names.extend(
                principals
                    .split(',')
                    .filter(|principal| !principal.is_empty())
                    .map(str::to_owned),
            );
        }
    }
    Ok(names)
}

fn validate_rated_by(rated_by: &str, allowed_signers: &str) -> Result<(), FeatureRiskRatingError> {
    if rated_by.trim().is_empty() || !allowed_signer_names(allowed_signers)?.contains(rated_by) {
        return Err(invalid_request(
            "rated_by is empty or is not a principal in allowed_signers",
        ));
    }
    Ok(())
}

fn render_rating_document(repository_id: &str, draft: &RatingDraft) -> String {
    let mut targets = String::new();
    for fr_id in &draft.fr_ids {
        targets.push_str("  - target: ix://agent-ix/");
        targets.push_str(repository_id);
        targets.push('/');
        targets.push_str(fr_id);
        targets.push_str("\n    type: rates_feature\n");
    }
    let quoted = |value: &str| serde_json::to_string(value).unwrap_or_default();
    format!(
        "---\nid: {{id}}\ntype: FeatureRiskRating\nrating: {}\nrated_by: {}\nintended_use: {}\nrelationships:\n{}---\n\n# Feature risk rating {{id}}\n\n## Rationale\n\n{}\n\n## Assurance activities\n\n{}\n",
        match draft.rating {
            Rating::HighProcessRisk => "high_process_risk",
            Rating::NotHighProcessRisk => "not_high_process_risk",
        },
        quoted(&draft.rated_by),
        quoted(&draft.intended_use),
        targets,
        draft.rationale,
        draft.assurance_activities,
    )
    .replace("{id}", &draft.rating_id)
}

fn commit_command(path: &str) -> String {
    format!(
        "git add -- {} && git commit -S -m {} -m {}",
        shell_quote(path),
        shell_quote("Accept feature risk rating"),
        shell_quote("Signed-off-meaning: feature-risk-rating-accepted"),
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn encode_workflow(output: WorkflowOutput) -> Result<Vec<u8>, FeatureRiskRatingError> {
    let mut bytes = serde_json::to_vec(&WorkflowResultLine {
        protocol: WORKFLOW_RESULT_PROTOCOL,
        output,
    })
    .map_err(|_| FeatureRiskRatingError::Encoding)?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(crate) fn execute_acceptance(bytes: &[u8]) -> Result<(Vec<u8>, bool), FeatureRiskRatingError> {
    let request: AcceptanceRequest = serde_json::from_slice(bytes).map_err(|error| {
        FeatureRiskRatingError::AcceptanceRequest {
            detail: error.to_string(),
        }
    })?;
    if request.protocol != ACCEPTANCE_REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.allowed_signers.is_absolute()
    {
        return Err(FeatureRiskRatingError::AcceptanceRequest {
            detail: "protocol, repository or allowed_signers is invalid".to_owned(),
        });
    }
    let results = acceptance_results(&request.repository, &request.allowed_signers)?;
    let complete = results.iter().all(|result| result.status == "accepted");
    let mut line = serde_json::to_vec(&AcceptanceResultLine {
        protocol: ACCEPTANCE_RESULT_PROTOCOL,
        results,
    })
    .map_err(|_| FeatureRiskRatingError::Encoding)?;
    line.push(b'\n');
    Ok((line, complete))
}

fn acceptance_results(
    repository: &Path,
    allowed_signers: &Path,
) -> Result<Vec<RatingResult>, FeatureRiskRatingError> {
    let root =
        repository
            .canonicalize()
            .map_err(|error| FeatureRiskRatingError::AcceptanceRequest {
                detail: format!("repository is unavailable: {error}"),
            })?;
    if !root.is_dir() {
        return Err(FeatureRiskRatingError::AcceptanceRequest {
            detail: "repository is not a directory".to_owned(),
        });
    }
    let directory = root.join("spec/validation");
    let mut files = Vec::new();
    collect_rating_files(&directory, &root, &mut files, 0)?;
    files.sort();
    files
        .into_iter()
        .map(|path| evaluate_rating_file(&root, &path, allowed_signers))
        .collect()
}

fn collect_rating_files(
    directory: &Path,
    root: &Path,
    files: &mut Vec<PathBuf>,
    depth: usize,
) -> Result<(), FeatureRiskRatingError> {
    if depth > 12 {
        return Err(FeatureRiskRatingError::RatingFile {
            detail: "spec/validation nesting exceeds the limit".to_owned(),
        });
    }
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(FeatureRiskRatingError::RatingFile {
                detail: error.to_string(),
            });
        }
    };
    for entry in entries {
        let entry = entry.map_err(|error| FeatureRiskRatingError::RatingFile {
            detail: error.to_string(),
        })?;
        let path = entry.path();
        let metadata =
            fs::symlink_metadata(&path).map_err(|error| FeatureRiskRatingError::RatingFile {
                detail: error.to_string(),
            })?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect_rating_files(&path, root, files, depth + 1)?;
        } else if metadata.is_file() && path.extension().is_some_and(|extension| extension == "md")
        {
            if !path.starts_with(root) {
                return Err(FeatureRiskRatingError::RatingFile {
                    detail: "rating file escaped the repository".to_owned(),
                });
            }
            let metadata =
                fs::metadata(&path).map_err(|error| FeatureRiskRatingError::RatingFile {
                    detail: error.to_string(),
                })?;
            if metadata.len() > MAX_DOCUMENT_BYTES {
                continue;
            }
            let bytes = fs::read(&path).map_err(|error| FeatureRiskRatingError::RatingFile {
                detail: error.to_string(),
            })?;
            let is_rating = frontmatter(std::str::from_utf8(&bytes).unwrap_or_default())
                .and_then(|frontmatter| parse_unambiguous_yaml_json(frontmatter.as_bytes()))
                .and_then(|data| data.get("type").and_then(Value::as_str).map(str::to_owned))
                .is_some_and(|kind| kind == "FeatureRiskRating");
            if is_rating {
                files.push(path);
            }
        }
    }
    Ok(())
}

#[implements("FR-090-AC-1")]
fn evaluate_rating_file(
    repository: &Path,
    path: &Path,
    allowed_signers: &Path,
) -> Result<RatingResult, FeatureRiskRatingError> {
    let relative =
        path.strip_prefix(repository)
            .map_err(|_| FeatureRiskRatingError::RatingFile {
                detail: "rating path is outside the repository".to_owned(),
            })?;
    let relative_text = relative.to_string_lossy().into_owned();
    let bytes = fs::read(path).map_err(|error| FeatureRiskRatingError::RatingFile {
        detail: error.to_string(),
    })?;
    let frontmatter = std::str::from_utf8(&bytes)
        .ok()
        .and_then(frontmatter)
        .and_then(|text| parse_unambiguous_yaml_json(text.as_bytes()));
    let id = frontmatter
        .as_ref()
        .and_then(|data| data.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    let rated_by = frontmatter
        .as_ref()
        .and_then(|data| data.get("rated_by"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let rating_status = frontmatter
        .as_ref()
        .and_then(|data| data.get("status"))
        .and_then(Value::as_str);
    let (commit, reason) = match latest_commit(repository, relative) {
        Ok(Some(commit)) => match path_is_dirty(repository, relative) {
            Err(_) => (Some(commit), Some("unverifiable")),
            Ok(true) => (Some(commit), Some("not_committed")),
            Ok(false) if rating_status != Some("accepted") => (
                Some(commit),
                Some(if rating_status == Some("proposed") {
                    "status_proposed"
                } else {
                    "status_not_accepted"
                }),
            ),
            Ok(false) if rated_by.as_deref().is_none_or(str::is_empty) => {
                (Some(commit), Some("unverifiable"))
            }
            Ok(false) => (
                Some(commit.clone()),
                verify_rating_commit(
                    repository,
                    &commit,
                    allowed_signers,
                    rated_by.as_deref().unwrap_or_default(),
                ),
            ),
        },
        Ok(None) => (None, Some("not_committed")),
        Err(_) => (None, Some("unverifiable")),
    };
    let status = if reason.is_none() {
        "accepted"
    } else {
        "not_accepted"
    };
    Ok(RatingResult {
        id,
        path: relative_text,
        commit,
        status,
        reason,
    })
}

fn latest_commit(repository: &Path, path: &Path) -> Result<Option<String>, ProcessError> {
    let (git, path_env) = signature_verify_host::resolve_program("git")?;
    let path_arg = path.as_os_str();
    let args = [
        OsStr::new("log"),
        OsStr::new("-1"),
        OsStr::new("--format=%H"),
        OsStr::new("HEAD"),
        OsStr::new("--"),
        path_arg,
    ];
    let environment = [(OsStr::new("PATH"), path_env.as_os_str())];
    let removed: Vec<&OsStr> = REMOVED_ENVIRONMENT.iter().map(OsStr::new).collect();
    let result = process_host::run_configured(
        git.as_os_str(),
        &args,
        Some(repository),
        &environment,
        &removed,
        ProcessLimits {
            timeout: PROCESS_TIMEOUT,
            max_output_bytes: MAX_OUTPUT_BYTES,
        },
    )?;
    if !result.status.success() {
        return Err(ProcessError::Observation {
            detail: format!("git log exited {}", result.status),
        });
    }
    let commit = std::str::from_utf8(&result.stdout)
        .ok()
        .map_or_default(str::trim);
    if commit.is_empty() {
        Ok(None)
    } else {
        Ok(Some(commit.to_owned()))
    }
}

fn path_is_dirty(repository: &Path, path: &Path) -> Result<bool, ProcessError> {
    let (git, path_env) = signature_verify_host::resolve_program("git")?;
    let args = [
        OsStr::new("status"),
        OsStr::new("--porcelain"),
        OsStr::new("--"),
        path.as_os_str(),
    ];
    let environment = [(OsStr::new("PATH"), path_env.as_os_str())];
    let removed: Vec<&OsStr> = REMOVED_ENVIRONMENT.iter().map(OsStr::new).collect();
    let result = process_host::run_configured(
        git.as_os_str(),
        &args,
        Some(repository),
        &environment,
        &removed,
        ProcessLimits {
            timeout: PROCESS_TIMEOUT,
            max_output_bytes: MAX_OUTPUT_BYTES,
        },
    )?;
    if !result.status.success() {
        return Err(ProcessError::Observation {
            detail: format!("git status exited {}", result.status),
        });
    }
    Ok(!result.stdout.is_empty())
}

fn verify_rating_commit(
    repository: &Path,
    revision: &str,
    allowed_signers: &Path,
    rated_by: &str,
) -> Option<&'static str> {
    let Ok(executable) = std::env::current_exe() else {
        return Some("unverifiable");
    };
    verify_rating_commit_with_executable(
        repository,
        revision,
        allowed_signers,
        rated_by,
        &executable,
    )
}

fn verify_rating_commit_with_executable(
    repository: &Path,
    revision: &str,
    allowed_signers: &Path,
    rated_by: &str,
    executable: &Path,
) -> Option<&'static str> {
    verify_rating_commit_with_executable_and_timeout(
        repository,
        revision,
        allowed_signers,
        rated_by,
        executable,
        PROCESS_TIMEOUT,
    )
}

fn verify_rating_commit_with_executable_and_timeout(
    repository: &Path,
    revision: &str,
    allowed_signers: &Path,
    rated_by: &str,
    executable: &Path,
    timeout: Duration,
) -> Option<&'static str> {
    let request = json!({
        "protocol": SIGNATURE_REQUEST_PROTOCOL,
        "repository": repository,
        "revision": revision,
        "kind": "commit",
        "allowed_signers": allowed_signers,
    });
    let Ok(bytes) = serde_json::to_vec(&request) else {
        return Some("unverifiable");
    };
    let Ok(directory) = tempfile::tempdir() else {
        return Some("unverifiable");
    };
    let request_path = directory.path().join("signature-request.json");
    if fs::write(&request_path, bytes).is_err() {
        return Some("unverifiable");
    }
    let args = [
        OsString::from("signature-verify"),
        OsString::from("--request"),
        OsString::from("-"),
    ];
    let args: Vec<&OsStr> = args.iter().map(OsString::as_os_str).collect();
    let Ok(request_file) = fs::File::open(&request_path) else {
        return Some("unverifiable");
    };
    let path_env = std::env::var_os("PATH").unwrap_or_default();
    let environment = [(OsStr::new("PATH"), path_env.as_os_str())];
    let removed: Vec<&OsStr> = REMOVED_ENVIRONMENT.iter().map(OsStr::new).collect();
    let Ok(result) = process_host::run_configured_stdin(
        executable.as_os_str(),
        &args,
        Some(repository),
        &environment,
        &removed,
        request_file,
        ProcessLimits {
            timeout,
            max_output_bytes: MAX_OUTPUT_BYTES,
        },
    ) else {
        return Some("unverifiable");
    };
    if !result.status.success() {
        return Some("unverifiable");
    }
    let value: Value = match serde_json::from_slice(&result.stdout) {
        Ok(value) => value,
        Err(_) => return Some("unverifiable"),
    };
    classify_signature_response(&value, rated_by)
}

fn classify_signature_response(value: &Value, rated_by: &str) -> Option<&'static str> {
    if value.get("protocol").and_then(Value::as_str) != Some(SIGNATURE_RESULT_PROTOCOL)
        || value.get("kind").and_then(Value::as_str) != Some("commit")
    {
        return Some("unverifiable");
    }
    match value.get("status").and_then(Value::as_str) {
        Some("unsigned") => return Some("unsigned"),
        Some("signed_bad") => return Some("signature_bad"),
        Some("signed_untrusted") => return Some("signer_unmapped"),
        Some("signed_valid") => {}
        _ => return Some("unverifiable"),
    }
    let Some(signer) = value.get("signer") else {
        return Some("unverifiable");
    };
    match signer.get("mapping").and_then(Value::as_str) {
        Some("mapped") => {}
        Some("unmapped") => return Some("signer_unmapped"),
        _ => return Some("unverifiable"),
    }
    let Some(principal) = signer.get("principal").and_then(Value::as_str) else {
        return Some("unverifiable");
    };
    if principal != rated_by {
        return Some("principal_mismatch");
    }
    let Some(meaning) = value.get("signed_off_meaning") else {
        return Some("unverifiable");
    };
    if meaning.get("presence").and_then(Value::as_str) != Some("present") {
        return Some("meaning_missing");
    }
    let Some(values) = meaning.get("values").and_then(Value::as_array) else {
        return Some("unverifiable");
    };
    if !values
        .iter()
        .any(|value| value.as_str() == Some(ACCEPTANCE_MEANING))
    {
        return Some("meaning_missing");
    }
    None
}

fn validation_unrated_features(
    repository: &Path,
    quire_executable: &Path,
) -> Result<Vec<Value>, FeatureRiskRatingError> {
    let metadata =
        fs::metadata(quire_executable).map_err(|error| FeatureRiskRatingError::Validation {
            detail: format!("quire executable is unavailable: {error}"),
        })?;
    if !metadata.is_file() {
        return Err(FeatureRiskRatingError::Validation {
            detail: "quire_executable is not a regular file".to_owned(),
        });
    }
    let path_env = std::env::var_os("PATH").unwrap_or_default();
    let args = [
        OsString::from("validate"),
        OsString::from("--scope"),
        repository.as_os_str().to_owned(),
        OsString::from("--severity"),
        OsString::from("trace:feature-without-risk-rating=error"),
        OsString::from("--diagnostics-format"),
        OsString::from("json"),
        OsString::from("spec/functional/*.md"),
    ];
    let args: Vec<&OsStr> = args.iter().map(OsString::as_os_str).collect();
    let environment = [(OsStr::new("PATH"), path_env.as_os_str())];
    let removed: Vec<&OsStr> = REMOVED_ENVIRONMENT.iter().map(OsStr::new).collect();
    let result = process_host::run_configured(
        quire_executable.as_os_str(),
        &args,
        Some(repository),
        &environment,
        &removed,
        ProcessLimits {
            timeout: PROCESS_TIMEOUT,
            max_output_bytes: MAX_OUTPUT_BYTES,
        },
    )
    .map_err(|error| FeatureRiskRatingError::Validation {
        detail: error.to_string(),
    })?;
    if !result.status.success() && result.status.code() != Some(2) {
        return Err(FeatureRiskRatingError::Validation {
            detail: format!("quire validate exited {}", result.status),
        });
    }
    let text =
        std::str::from_utf8(&result.stdout).map_err(|_| FeatureRiskRatingError::Validation {
            detail: "quire validate output is not UTF-8".to_owned(),
        })?;
    let mut features = Vec::new();
    let mut seen = BTreeSet::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let diagnostic: Value =
            serde_json::from_str(line).map_err(|error| FeatureRiskRatingError::Validation {
                detail: format!("quire validate output is malformed: {error}"),
            })?;
        let message = diagnostic
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !message.contains("feature-without-risk-rating") {
            continue;
        }
        let path = diagnostic
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| FeatureRiskRatingError::Validation {
                detail: "unrated-feature diagnostic has no path".to_owned(),
            })?;
        let path = Path::new(path);
        let path = if path.is_absolute() {
            path.strip_prefix(repository)
                .map_err(|_| FeatureRiskRatingError::Validation {
                    detail: "unrated feature path is outside the repository".to_owned(),
                })?
                .to_path_buf()
        } else {
            path.to_path_buf()
        };
        let document = fs::read(repository.join(&path)).map_err(|error| {
            FeatureRiskRatingError::Validation {
                detail: format!("cannot read unrated feature {}: {error}", path.display()),
            }
        })?;
        let id = std::str::from_utf8(&document)
            .ok()
            .and_then(frontmatter)
            .and_then(|text| parse_unambiguous_yaml_json(text.as_bytes()))
            .and_then(|data| data.get("id").and_then(Value::as_str).map(str::to_owned))
            .ok_or_else(|| FeatureRiskRatingError::Validation {
                detail: format!("unrated feature {} has invalid frontmatter", path.display()),
            })?;
        if seen.insert(id.clone()) {
            features.push(json!({"id": id, "path": path.to_string_lossy()}));
        }
    }
    Ok(features)
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use serde_json::json;

    use super::{
        ACCEPTANCE_MEANING, SIGNATURE_REQUEST_PROTOCOL, SIGNATURE_RESULT_PROTOCOL,
        WORKFLOW_REQUEST_PROTOCOL, WorkflowRequest, classify_signature_response, valid_rating_path,
        validate_decision_owner, validate_rated_by, validate_workflow_request,
        validation_unrated_features, verify_rating_commit_with_executable,
        verify_rating_commit_with_executable_and_timeout,
    };

    fn valid_response() -> serde_json::Value {
        json!({
            "protocol": SIGNATURE_RESULT_PROTOCOL,
            "kind": "commit",
            "status": "signed_valid",
            "signer": {"mapping": "mapped", "principal": "alice@example.test"},
            "signed_off_meaning": {
                "presence": "present",
                "values": [ACCEPTANCE_MEANING]
            }
        })
    }

    #[test]
    /// PLAT-1448 FR‑021 / FR‑027 AC-1: the mapped signed commit is accepted.
    #[trace("FR-036-AC-1", "FR-037-AC-1", "FR-037-AC-5")]
    fn mapped_signed_commit_with_acceptance_meaning_is_accepted() {
        assert_eq!(
            classify_signature_response(&valid_response(), "alice@example.test"),
            None
        );
    }

    #[test]
    /// PLAT-1448 FR‑021 / FR‑027 AC-2: unsigned signatures are refused.
    #[trace("FR-036-AC-2", "FR-037-AC-7")]
    fn unsigned_commit_has_the_unsigned_reason() {
        let mut response = valid_response();
        response["status"] = json!("unsigned");
        assert_eq!(
            classify_signature_response(&response, "alice@example.test"),
            Some("unsigned")
        );
    }

    #[test]
    /// PLAT-1448 FR‑021 / FR‑027: bad signatures are refused.
    #[trace("FR-036-AC-3", "FR-037-AC-7")]
    fn invalid_signature_has_the_signature_bad_reason() {
        let mut response = valid_response();
        response["status"] = json!("signed_bad");
        assert_eq!(
            classify_signature_response(&response, "alice@example.test"),
            Some("signature_bad")
        );
    }

    #[test]
    /// PLAT-1448 FR‑021 / FR‑027 AC-3: unmapped signatures are refused.
    #[trace("FR-036-AC-3", "FR-037-AC-2")]
    fn unmapped_signer_has_the_signer_unmapped_reason() {
        let mut response = valid_response();
        response["signer"]["mapping"] = json!("unmapped");
        assert_eq!(
            classify_signature_response(&response, "alice@example.test"),
            Some("signer_unmapped")
        );
    }

    #[test]
    /// PLAT-1448 FR‑021 / FR‑027 AC-4: a different mapped principal is refused.
    fn different_mapped_principal_has_the_principal_mismatch_reason() {
        let mut response = valid_response();
        response["signer"]["principal"] = json!("bob@example.test");
        assert_eq!(
            classify_signature_response(&response, "alice@example.test"),
            Some("principal_mismatch")
        );
    }

    #[test]
    /// PLAT-1448 FR‑021 / FR‑027 AC-5: absent meaning is refused.
    #[trace("FR-037-AC-6")]
    fn missing_acceptance_meaning_has_the_meaning_missing_reason() {
        let mut response = valid_response();
        response["signed_off_meaning"]["presence"] = json!("missing");
        assert_eq!(
            classify_signature_response(&response, "alice@example.test"),
            Some("meaning_missing")
        );
    }

    #[test]
    /// PLAT-1448 FR‑021 / FR‑027: malformed verifier output fails closed.
    #[trace("FR-014-AC-3", "FR-019-AC-3")]
    fn malformed_verifier_response_is_unverifiable() {
        assert_eq!(
            classify_signature_response(&json!({}), "alice@example.test"),
            Some("unverifiable")
        );
    }

    #[test]
    /// PLAT-1448 FR‑021 / FR‑027: rating paths are constrained to validation records.
    #[trace("FR-014-AC-3")]
    fn rating_paths_are_confined_to_validation_markdown() {
        assert!(valid_rating_path("spec/validation/FRR-001.md"));
        assert!(!valid_rating_path("spec/functional/FRR-001.md"));
        assert!(!valid_rating_path("spec/validation/../FRR-001.md"));
        assert!(!valid_rating_path("/spec/validation/FRR-001.md"));
    }

    #[test]
    /// PLAT-1448 FR‑021 / FR‑027: workflow requires a mapped human principal.
    #[trace("FR-014-AC-3")]
    fn workflow_rejects_empty_and_unmapped_rated_by_values() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("allowed_signers");
        std::fs::write(&path, "alice@example.test ssh-ed25519 AAAA\n").expect("allowed signers");
        let allowed_signers = path.to_string_lossy();
        assert!(validate_rated_by("", &allowed_signers).is_err());
        assert!(validate_rated_by("bob@example.test", &allowed_signers).is_err());
        assert!(validate_rated_by("alice@example.test", &allowed_signers).is_ok());
    }

    #[test]
    /// PLAT-1448 FR‑021 / FR‑027: the human gate owner must be the rated signer.
    #[trace("FR-014-AC-3")]
    fn workflow_rejects_decision_owner_different_from_rated_by() {
        let request: WorkflowRequest = serde_json::from_value(json!({
            "protocol": WORKFLOW_REQUEST_PROTOCOL,
            "operation": "present",
            "state_dir": "/tmp/flow-state",
            "skill_root": "/tmp/skill",
            "ix_flow_executable": "ix-flow",
            "run_id": "rating-1",
            "decision_owner": "bob@example.test",
            "repository": "/tmp/repository",
            "repository_id": "agent-ix/engineering-assurance",
            "quire_executable": "/usr/bin/quire",
            "allowed_signers": "/tmp/allowed_signers",
            "rating_id": "FRR-001",
            "rating_path": "spec/validation/FRR-001.md",
            "rating": "high_process_risk",
            "rated_by": "alice@example.test",
            "intended_use": "test",
            "fr_ids": ["FR-001"],
            "rationale": "test rationale",
            "assurance_activities": "test activity"
        }))
        .expect("workflow request");

        let error = validate_workflow_request(&request).expect_err("owner mismatch refused");
        assert!(error.to_string().contains("decision_owner must equal"));
        assert!(validate_decision_owner("bob@example.test", "alice@example.test").is_err());
        assert!(validate_decision_owner("alice@example.test", "alice@example.test").is_ok());
    }

    #[cfg(unix)]
    #[test]
    /// PLAT-1448 FR‑021 / FR‑027: report only the validator's unrated features.
    #[trace("FR-014-AC-2")]
    fn workflow_uses_exactly_the_validator_unrated_feature_list() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().expect("temporary directory");
        let repository = directory.path().join("repo");
        let spec_dir = repository.join("spec/functional");
        std::fs::create_dir_all(&spec_dir).expect("functional spec directory");
        for id in ["FR-001", "FR-002"] {
            std::fs::write(
                spec_dir.join(format!("{id}.md")),
                format!("---\nid: {id}\ntype: FunctionalRequirement\n---\n"),
            )
            .expect("functional requirement");
        }
        let diagnostics = ["FR-001", "FR-002"]
            .map(|id| {
                json!({
                    "kind": "TraceRuleViolation",
                    "message": "feature-without-risk-rating: feature has no rating",
                    "path": repository.join(format!("spec/functional/{id}.md")),
                })
            })
            .into_iter()
            .map(|value| serde_json::to_string(&value).expect("diagnostic JSON"))
            .collect::<Vec<_>>()
            .join("\n");
        let quire = directory.path().join("quire");
        std::fs::write(
            &quire,
            format!("#!/bin/sh\nprintf '%s\\n' '{diagnostics}'\n"),
        )
        .expect("validator fixture");
        let mut permissions = std::fs::metadata(&quire)
            .expect("validator metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&quire, permissions).expect("validator executable");

        let features =
            validation_unrated_features(&repository, &quire).expect("validator diagnostics parse");
        assert_eq!(
            features,
            vec![
                json!({"id": "FR-001", "path": "spec/functional/FR-001.md"}),
                json!({"id": "FR-002", "path": "spec/functional/FR-002.md"}),
            ]
        );
    }

    #[cfg(unix)]
    fn make_verifier(directory: &std::path::Path, name: &str, script: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let executable = directory.join(name);
        std::fs::write(&executable, script).expect("verifier fixture");
        let mut permissions = std::fs::metadata(&executable)
            .expect("verifier metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&executable, permissions).expect("verifier executable");
        executable
    }

    #[cfg(unix)]
    fn shell_quote(path: &std::path::Path) -> String {
        format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
    }

    #[cfg(unix)]
    fn process_fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
        let directory = tempfile::tempdir().expect("temporary directory");
        let repository = directory.path().join("repository");
        let allowed_signers = directory.path().join("allowed_signers");
        std::fs::create_dir(&repository).expect("repository directory");
        std::fs::write(&allowed_signers, "alice@example.test ssh-ed25519 AAAA\n")
            .expect("allowed-signers fixture");
        (directory, repository, allowed_signers)
    }

    #[cfg(unix)]
    #[test]
    /// PLAT-1448 FR‑021 / FR‑027 request-contract obligation: pass exactly the declared request members.
    fn verifier_process_receives_exact_five_member_signature_request() {
        let (directory, repository, allowed_signers) = process_fixture();
        let captured = directory.path().join("captured-request.json");
        let response = serde_json::to_string(&valid_response()).expect("response JSON");
        let executable = make_verifier(
            directory.path(),
            "signature-verify",
            &format!(
                "#!/bin/sh\n[ \"$1\" = signature-verify ] || exit 91\n[ \"$2\" = --request ] || exit 92\n[ \"$3\" = - ] || exit 93\ncat > {}\nprintf '%s\\n' '{}'\n",
                shell_quote(&captured),
                response
            ),
        );

        assert_eq!(
            verify_rating_commit_with_executable(
                &repository,
                "deadbeef",
                &allowed_signers,
                "alice@example.test",
                &executable,
            ),
            None
        );
        let request: serde_json::Value =
            serde_json::from_slice(&std::fs::read(captured).expect("captured request"))
                .expect("signature request JSON");
        let mut keys: Vec<_> = request
            .as_object()
            .expect("object request")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "allowed_signers",
                "kind",
                "protocol",
                "repository",
                "revision"
            ]
        );
        assert_eq!(request["protocol"], SIGNATURE_REQUEST_PROTOCOL);
        assert_eq!(request["repository"], repository.to_string_lossy().as_ref());
        assert_eq!(request["revision"], "deadbeef");
        assert_eq!(request["kind"], "commit");
        assert_eq!(
            request["allowed_signers"],
            allowed_signers.to_string_lossy().as_ref()
        );
    }

    #[cfg(unix)]
    #[test]
    /// PLAT-1448 FR‑021 / FR‑027 process-start obligation: an unavailable verifier fails closed.
    #[trace("FR-019-AC-3", "FR-036-AC-5")]
    fn unavailable_verifier_process_is_unverifiable() {
        let (directory, repository, allowed_signers) = process_fixture();
        let executable = directory.path().join("missing-verifier");
        assert_eq!(
            verify_rating_commit_with_executable(
                &repository,
                "deadbeef",
                &allowed_signers,
                "alice@example.test",
                &executable,
            ),
            Some("unverifiable")
        );
    }

    #[cfg(unix)]
    #[test]
    /// PLAT-1448 FR‑021 / FR‑027 timeout obligation: a timed-out verifier fails closed.
    #[trace("FR-019-AC-3")]
    fn verifier_timeout_is_unverifiable() {
        use std::time::Duration;

        let (directory, repository, allowed_signers) = process_fixture();
        let executable =
            make_verifier(directory.path(), "timeout-verifier", "#!/bin/sh\nsleep 1\n");
        assert_eq!(
            verify_rating_commit_with_executable_and_timeout(
                &repository,
                "deadbeef",
                &allowed_signers,
                "alice@example.test",
                &executable,
                Duration::from_millis(250),
            ),
            Some("unverifiable")
        );
    }

    #[cfg(unix)]
    #[test]
    /// PLAT-1448 FR‑021 / FR‑027 failed-process obligation: a non-zero verifier exit fails closed.
    #[trace("FR-019-AC-3")]
    fn verifier_nonzero_exit_is_unverifiable() {
        let (directory, repository, allowed_signers) = process_fixture();
        let executable =
            make_verifier(directory.path(), "failing-verifier", "#!/bin/sh\nexit 17\n");
        assert_eq!(
            verify_rating_commit_with_executable(
                &repository,
                "deadbeef",
                &allowed_signers,
                "alice@example.test",
                &executable,
            ),
            Some("unverifiable")
        );
    }

    #[cfg(unix)]
    #[test]
    /// PLAT-1448 FR‑021 / FR‑027 malformed-output obligation: invalid stdout fails closed.
    #[trace("FR-014-AC-3", "FR-019-AC-3")]
    fn malformed_verifier_stdout_is_unverifiable() {
        let (directory, repository, allowed_signers) = process_fixture();
        let executable = make_verifier(
            directory.path(),
            "malformed-verifier",
            "#!/bin/sh\nprintf '%s\\n' not-json\n",
        );
        assert_eq!(
            verify_rating_commit_with_executable(
                &repository,
                "deadbeef",
                &allowed_signers,
                "alice@example.test",
                &executable,
            ),
            Some("unverifiable")
        );
    }
}
