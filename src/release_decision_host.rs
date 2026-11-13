// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Adapter for the `release-decision` command (FR-048).
//!
//! The command gathers what a named person needs to decide one release, holds
//! the run at the `hitl` gate of the `release-decision` workflow, and records
//! the person's explicit `release` or `reject`. It reads the Baseline from git
//! objects and the evidence status, SOUP report, anomaly list and snapshot from
//! supplied files; it computes none of them. It creates no commit and no tag,
//! signs nothing and pushes nothing: the `release` outcome is the Release
//! document, its path and the tag message for the person to commit and sign.

use std::{io::Read, path::Path};

use engineering_assurance::{
    approval_tag::is_meaning,
    approval_verify::{RECORD_DIGEST_TRAILER, RECORD_ID_TRAILER},
    release::is_repository_path,
    signature_verify::MEANING_TOKEN,
    workflow::{
        DecisionChoice, DecisionEvent, REQUEST_PROTOCOL as HOST_PROTOCOL, WorkflowBinding,
        WorkflowHostOutcome, WorkflowHostRequest, WorkflowHostResult, WorkflowOperation,
        WorkflowSnapshot,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::release_host::{BaselineError, path_exists_at, resolve_baseline};
use crate::workflow_host::{self, BoundRun, WorkflowHostError};

const REQUEST_PROTOCOL: &str = "engineering-assurance.release-decision-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.release-decision-result/v1";
const WORKFLOW: &str = "release-decision";
const WORKFLOW_VERSION: &str = "0.1.0";
const PRESENTATION_LIMIT: usize = 2 * 1024 * 1024;
const NOT_PROVIDED: &str = "not provided";
/// The largest supplied file read.
const MAX_INPUT_BYTES: u64 = 1024 * 1024;
/// The largest presentation as ix-flow prints it. ix-flow pretty-prints its
/// answers, and every answer carries the run's items (and the add-item event)
/// at some depth, while the host reads at most 16 MiB of an answer. A larger
/// item would leave a run no command can read again.
const MAX_PRESENTATION_BYTES: usize = PRESENTATION_LIMIT;
/// The closed Evidence Status vocabulary of `ix://agent-ix/quoin/FR-115`.
const EVIDENCE_STATUSES: [&str; 5] = [
    "bound",
    "stale",
    "suspect",
    "undischarged",
    "no run evidence",
];

#[derive(Debug, Error)]
pub(crate) enum ReleaseDecisionError {
    #[error("invalid release-decision request: {detail}")]
    Request { detail: String },
    #[error("{0}")]
    Workflow(#[from] WorkflowHostError),
    #[error("release input {file} is unreadable: {detail}")]
    InputUnreadable { file: String, detail: String },
    #[error("the Baseline cannot be resolved: {0}")]
    Baseline(&'static str),
    #[error("snapshot_path {path:?} breaks the path rules of a Baseline item")]
    SnapshotPathInvalid { path: String },
    #[error("snapshot_path {path:?} exists at the baseline commit")]
    SnapshotPathPresentAtBaseline { path: String },
    #[error("accepted anomaly {id:?} is not in the supplied anomaly list")]
    AcceptedAnomalyUnknown { id: String },
    #[error("unresolved anomalies are not named as accepted: {ids:?}")]
    AnomalyNotAccepted { ids: Vec<String> },
    #[error("the release meaning is empty, too long, or not carried unchanged by a trailer")]
    MeaningInvalid,
    #[error("the run holds no release presentation; run `present` first")]
    PresentationMissing,
    #[error("record_path {path:?} exists at the baseline commit")]
    RecordPathPresentAtBaseline { path: String },
    #[error("Baseline items are absent at the baseline commit: {paths:?}")]
    BaselineItemAbsent { paths: Vec<String> },
    #[error(
        "the presentation is {observed} bytes as ix-flow prints it; the limit is {MAX_PRESENTATION_BYTES}"
    )]
    PresentationTooLarge { observed: usize },
    #[error("the run already holds a different release decision: {detail}")]
    DecisionChanged { detail: &'static str },
}

impl ReleaseDecisionError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Request { .. } => "release_decision_request_invalid",
            Self::Workflow(error) => error.code(),
            Self::InputUnreadable { .. } => "release_input_unreadable",
            Self::Baseline(code) => code,
            Self::SnapshotPathInvalid { .. } => "release_snapshot_path_invalid",
            Self::SnapshotPathPresentAtBaseline { .. } => "snapshot_path_present_at_baseline",
            Self::AcceptedAnomalyUnknown { .. } => "accepted_anomaly_unknown",
            Self::AnomalyNotAccepted { .. } => "anomaly_not_accepted",
            Self::MeaningInvalid => "release_meaning_invalid",
            Self::PresentationMissing => "release_presentation_missing",
            Self::RecordPathPresentAtBaseline { .. } => "record_path_present_at_baseline",
            Self::BaselineItemAbsent { .. } => "baseline_item_absent",
            Self::PresentationTooLarge { .. } => "release_presentation_too_large",
            Self::DecisionChanged { .. } => "release_decision_changed",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Operation {
    /// Gather and retain the presentation and hold the run at the gate.
    Present,
    /// Return the gate state, or record the owner's explicit choice.
    Decide,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Choice {
    Release,
    Reject,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    operation: Operation,
    state_dir: String,
    skill_root: String,
    ix_flow_executable: String,
    run_id: String,
    decision_owner: String,
    repository: String,
    baseline_path: String,
    baseline_commit: String,
    snapshot_path: String,
    // `present` only.
    release_id: Option<String>,
    record_path: Option<String>,
    evidence_status: Option<String>,
    soup_report: Option<String>,
    anomalies: Option<String>,
    snapshot_file: Option<String>,
    // `decide` only.
    choice: Option<Choice>,
    accepted_anomalies: Option<Vec<String>>,
    meaning: Option<String>,
}

/// What identifies one release run, so a reused run id with any other value is
/// refused by the workflow binding.
#[derive(Serialize)]
struct Boundary<'a> {
    baseline_path: &'a str,
    baseline_commit: &'a str,
    snapshot_path: &'a str,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Output {
    Presentation {
        presentation: Value,
        snapshot: WorkflowSnapshot,
    },
    Waiting {
        snapshot: WorkflowSnapshot,
    },
    Decision {
        decision: DecisionEvent,
        #[serde(skip_serializing_if = "Option::is_none")]
        release: Option<ReleaseOutput>,
    },
}

#[derive(Serialize)]
struct ReleaseOutput {
    /// The Release document, for the owner to commit at `record_path`.
    document: String,
    record_path: String,
    /// The tag message, whose digest line the owner completes with the
    /// `git-blob:` id of the record at the release commit.
    tag_message: String,
}

#[derive(Serialize)]
struct ResultLine {
    protocol: &'static str,
    #[serde(flatten)]
    output: Output,
}

pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, ReleaseDecisionError> {
    let request: Request =
        serde_json::from_slice(bytes).map_err(|error| ReleaseDecisionError::Request {
            detail: error.to_string(),
        })?;
    validate(&request)?;
    let host_request = host_request(&request);
    // Only `present` makes a run; `decide` on a run that does not exist
    // refuses without making one.
    let run = workflow_host::bind_run(&host_request, request.operation == Operation::Present)?;
    let output = match request.operation {
        Operation::Present => present(&request, run)?,
        Operation::Decide => decide(&request, run)?,
    };
    let mut line = serde_json::to_vec(&ResultLine {
        protocol: RESULT_PROTOCOL,
        output,
    })
    .map_err(|error| ReleaseDecisionError::Request {
        detail: error.to_string(),
    })?;
    line.push(b'\n');
    Ok(line)
}

fn invalid(detail: impl Into<String>) -> ReleaseDecisionError {
    ReleaseDecisionError::Request {
        detail: detail.into(),
    }
}

fn validate(request: &Request) -> Result<(), ReleaseDecisionError> {
    if request.protocol != REQUEST_PROTOCOL {
        return Err(invalid(format!(
            "unsupported protocol {:?}",
            request.protocol
        )));
    }
    if !Path::new(&request.repository).is_absolute() {
        return Err(invalid("repository must be an absolute directory"));
    }
    if !is_repository_path(&request.baseline_path) {
        return Err(invalid("baseline_path is not a repository-relative path"));
    }
    if request.baseline_commit.trim().is_empty() {
        return Err(invalid("baseline_commit is blank"));
    }
    let present_only = [
        request.release_id.is_some(),
        request.record_path.is_some(),
        request.evidence_status.is_some(),
        request.soup_report.is_some(),
        request.anomalies.is_some(),
        request.snapshot_file.is_some(),
    ];
    let decide_only = [
        request.choice.is_some(),
        request.accepted_anomalies.is_some(),
        request.meaning.is_some(),
    ];
    match request.operation {
        Operation::Present => {
            if decide_only.contains(&true) {
                return Err(invalid(
                    "present takes no choice, accepted_anomalies or meaning",
                ));
            }
            let release_id = request
                .release_id
                .as_deref()
                .ok_or_else(|| invalid("present requires release_id"))?;
            if !release_id.strip_prefix("REL-").is_some_and(|digits| {
                !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
            }) {
                return Err(invalid("release_id must match REL-<digits>"));
            }
            let record_path = request
                .record_path
                .as_deref()
                .ok_or_else(|| invalid("present requires record_path"))?;
            if !is_repository_path(record_path) || record_path == request.snapshot_path {
                return Err(invalid(
                    "record_path must be a repository-relative path different from snapshot_path",
                ));
            }
            for (name, file) in [
                ("evidence_status", &request.evidence_status),
                ("soup_report", &request.soup_report),
                ("anomalies", &request.anomalies),
                ("snapshot_file", &request.snapshot_file),
            ] {
                if file.as_deref().is_some_and(|f| !Path::new(f).is_absolute()) {
                    return Err(invalid(format!("{name} must be an absolute file path")));
                }
            }
        }
        Operation::Decide => {
            if present_only.contains(&true) {
                return Err(invalid(
                    "decide takes the identity of the run and the choice only",
                ));
            }
            if request.choice != Some(Choice::Release)
                && (request.accepted_anomalies.is_some() || request.meaning.is_some())
            {
                return Err(invalid(
                    "accepted_anomalies and meaning are valid only with the release choice",
                ));
            }
            if request.choice == Some(Choice::Release) && request.meaning.is_none() {
                return Err(invalid("the release choice requires the owner's meaning"));
            }
        }
    }
    Ok(())
}

fn host_request(request: &Request) -> WorkflowHostRequest {
    let boundary = serde_json::to_string(&Boundary {
        baseline_path: &request.baseline_path,
        baseline_commit: &request.baseline_commit,
        snapshot_path: &request.snapshot_path,
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

fn present(request: &Request, mut run: BoundRun) -> Result<Output, ReleaseDecisionError> {
    if run.items("release_presentation")?.is_empty() {
        let presentation = gather(request)?;
        if run.items("release_request")?.is_empty() {
            run.record_answers(
                "release",
                &json!({
                    "release_id": request.release_id,
                    "baseline_path": request.baseline_path,
                    "baseline_commit": request.baseline_commit,
                    "snapshot_path": request.snapshot_path,
                    "record_path": request.record_path,
                    "owner": request.decision_owner,
                }),
            )?;
        }
        if run.phase()? == "capture" {
            run.advance("evidence_ready")?;
        }
        run.add_item("release_presentation", &presentation)?;
    }
    if run.phase()? == "evidence_ready" {
        run.advance("decision_ready")?;
    }
    let presentation = run
        .items("release_presentation")?
        .first()
        .cloned()
        .ok_or(ReleaseDecisionError::PresentationMissing)?;
    Ok(Output::Presentation {
        presentation,
        snapshot: run.snapshot()?,
    })
}

fn gather(request: &Request) -> Result<Value, ReleaseDecisionError> {
    let repository = Path::new(&request.repository);
    let baseline = resolve_baseline(repository, &request.baseline_commit, &request.baseline_path)
        .map_err(|error| {
        ReleaseDecisionError::Baseline(match error {
            BaselineError::CommitUnknown => "baseline_commit_unknown",
            BaselineError::FileAbsent => "baseline_file_absent",
            BaselineError::FileInvalid => "baseline_file_invalid",
            BaselineError::Git => "release_decision_git_failed",
        })
    })?;
    let soup_records: Vec<Value> = baseline
        .items
        .iter()
        .filter(|item| item.kind == crate::release_host::ItemKind::SoupRecord)
        .map(|item| json!({"path": item.path, "present": item.present}))
        .collect();

    let evidence = supplied(request.evidence_status.as_deref(), |source, document| {
        let criteria = evidence_criteria(source, &document)?;
        Ok(json!({"criteria": criteria, "document": document}))
    })?;
    let soup_report = supplied(request.soup_report.as_deref(), |source, document| {
        for row in array_member(source, &document, "rows")? {
            string_member(source, row, "purl")?;
        }
        Ok(json!({"document": document}))
    })?;
    let anomalies = supplied(request.anomalies.as_deref(), |source, document| {
        let items = array_member(source, &document, "anomalies")?
            .iter()
            .map(|anomaly| {
                let id = string_member(source, anomaly, "id")?;
                let status = anomaly
                    .get("status")
                    .filter(|s| s.is_string() || s.is_null());
                let Some(status) = status else {
                    return Err(unreadable(source, "an anomaly lacks its status member"));
                };
                Ok(json!({"id": id, "status": status}))
            })
            .collect::<Result<Vec<_>, ReleaseDecisionError>>()?;
        let unreadable_entries = match document.get("unreadable") {
            None => Vec::new(),
            Some(entries) => entries
                .as_array()
                .ok_or_else(|| unreadable(source, "the unreadable member is not a list"))?
                .iter()
                .map(|entry| {
                    let path = string_member(source, entry, "path")?;
                    Ok(json!({"path": path, "code": entry.get("code")}))
                })
                .collect::<Result<Vec<_>, ReleaseDecisionError>>()?,
        };
        Ok(json!({"items": items, "unreadable": unreadable_entries, "document": document}))
    })?;
    let snapshot_file = supplied_text(request.snapshot_file.as_deref(), |text| {
        Ok(json!({"text": text}))
    })?;

    let presentation = json!({
        "id": "presentation",
        "owner": request.decision_owner,
        "repository": request.repository,
        "baseline": {
            "path": request.baseline_path,
            "name": baseline.name,
            "commit": baseline.commit,
            "items": baseline.items,
        },
        "evidence_status": evidence,
        "soup": {"baseline_records": soup_records, "report": soup_report},
        "anomalies": anomalies,
        "snapshot": {"snapshot_path": request.snapshot_path, "file": snapshot_file},
    });
    let observed = printed_size(&presentation);
    if observed > MAX_PRESENTATION_BYTES {
        return Err(ReleaseDecisionError::PresentationTooLarge { observed });
    }
    Ok(presentation)
}

/// The size of `presentation` as ix-flow prints it: two-space indented, at the
/// depth it has inside an answer (`data.items.<kind>[]`).
fn printed_size(presentation: &Value) -> usize {
    serde_json::to_vec_pretty(&json!({
        "data": {"items": {"release_presentation": [presentation]}}
    }))
    .map_or(usize::MAX, |bytes| bytes.len())
}

fn unreadable(file: &str, detail: &str) -> ReleaseDecisionError {
    ReleaseDecisionError::InputUnreadable {
        file: file.to_owned(),
        detail: detail.to_owned(),
    }
}

/// The text of a supplied regular file within the input limit.
fn read_text(path: &str) -> Result<String, ReleaseDecisionError> {
    let regular = std::fs::metadata(path)
        .map_err(|error| unreadable(path, &error.to_string()))?
        .is_file();
    if !regular {
        return Err(unreadable(path, "the path is not a regular file"));
    }
    let mut text = String::new();
    std::fs::File::open(path)
        .and_then(|file| file.take(MAX_INPUT_BYTES + 1).read_to_string(&mut text))
        .map_err(|error| unreadable(path, &error.to_string()))?;
    if u64::try_from(text.len()).unwrap_or(u64::MAX) > MAX_INPUT_BYTES {
        return Err(unreadable(path, "the file exceeds the input limit"));
    }
    Ok(text)
}

/// A supplied text file, with its source, or `not provided`.
fn supplied_text(
    path: Option<&str>,
    read: impl FnOnce(String) -> Result<Value, ReleaseDecisionError>,
) -> Result<Value, ReleaseDecisionError> {
    let Some(path) = path else {
        return Ok(json!({"state": NOT_PROVIDED}));
    };
    Ok(supplied_value(path, read(read_text(path)?)?))
}

/// A supplied JSON object, with its source, or `not provided`. The document is
/// handed to `read` unchanged, and `read` returns what is kept of it.
fn supplied(
    path: Option<&str>,
    read: impl FnOnce(&str, Value) -> Result<Value, ReleaseDecisionError>,
) -> Result<Value, ReleaseDecisionError> {
    let Some(path) = path else {
        return Ok(json!({"state": NOT_PROVIDED}));
    };
    let document = parse_object(path, &read_text(path)?)?;
    Ok(supplied_value(path, read(path, document)?))
}

fn supplied_value(path: &str, mut value: Value) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.insert("state".to_owned(), json!("supplied"));
        object.insert("source".to_owned(), json!(path));
    }
    value
}

fn parse_object(file: &str, text: &str) -> Result<Value, ReleaseDecisionError> {
    serde_json::from_str::<Value>(text)
        .ok()
        .filter(Value::is_object)
        .ok_or_else(|| unreadable(file, "the file is not a JSON object"))
}

fn array_member<'a>(
    file: &str,
    document: &'a Value,
    member: &str,
) -> Result<&'a Vec<Value>, ReleaseDecisionError> {
    document
        .get(member)
        .and_then(Value::as_array)
        .ok_or_else(|| unreadable(file, &format!("the file lacks the {member} list")))
}

fn string_member<'a>(
    file: &str,
    object: &'a Value,
    member: &str,
) -> Result<&'a str, ReleaseDecisionError> {
    object
        .get(member)
        .and_then(Value::as_str)
        .ok_or_else(|| unreadable(file, &format!("an entry lacks its {member} member")))
}

fn evidence_criteria(file: &str, document: &Value) -> Result<Vec<Value>, ReleaseDecisionError> {
    let mut criteria = Vec::new();
    for requirement in array_member(file, document, "requirements")? {
        for criterion in array_member(file, requirement, "criteria")? {
            let id = string_member(file, criterion, "id")?;
            let status = string_member(file, criterion, "evidence_status")?;
            if !EVIDENCE_STATUSES.contains(&status) {
                return Err(unreadable(
                    file,
                    &format!("criterion {id} has an evidence status outside the closed set"),
                ));
            }
            criteria.push(json!({"id": id, "evidence_status": status}));
        }
    }
    Ok(criteria)
}

fn decide(request: &Request, mut run: BoundRun) -> Result<Output, ReleaseDecisionError> {
    let Some(choice) = request.choice else {
        return Ok(Output::Waiting {
            snapshot: run.snapshot()?,
        });
    };
    let presentation = run
        .items("release_presentation")?
        .first()
        .cloned()
        .ok_or(ReleaseDecisionError::PresentationMissing)?;
    match choice {
        Choice::Reject => Ok(Output::Decision {
            decision: decision_event(run.decide(DecisionChoice::Reject)?)?,
            release: None,
        }),
        Choice::Release => {
            let release = release_outcome(request, &mut run, &presentation)?;
            Ok(Output::Decision {
                decision: decision_event(run.decide(DecisionChoice::Release)?)?,
                release: Some(release),
            })
        }
    }
}

/// The meaning and accepted anomaly ids of a release choice, kept in the run
/// before the gate is acknowledged so that nothing the owner signs can change
/// afterwards.
fn release_outcome(
    request: &Request,
    run: &mut BoundRun,
    presentation: &Value,
) -> Result<ReleaseOutput, ReleaseDecisionError> {
    let meaning = request.meaning.as_deref().unwrap_or_default();
    let mut wanted: Vec<&str> = Vec::new();
    for id in request.accepted_anomalies.iter().flatten() {
        if !wanted.contains(&id.as_str()) {
            wanted.push(id);
        }
    }
    let stored = run.items("release_outcome")?.first().cloned();
    // Once the owner's gate is acknowledged, or the run has left the gate, the
    // outcome is fixed: a repeat can read it and nothing can write it.
    let fixed = run.phase()? != "decision_ready" || run.terminal_acknowledged("released")?;
    if let Some(stored) = &stored {
        let same = text_at(stored, &["meaning"]) == meaning
            && stored["accepted_anomalies"].as_array().is_some_and(|ids| {
                ids.iter()
                    .filter_map(Value::as_str)
                    .eq(wanted.iter().copied())
            });
        if same {
            return Ok(ReleaseOutput {
                document: text_at(stored, &["document"]).to_owned(),
                record_path: text_at(stored, &["record_path"]).to_owned(),
                tag_message: text_at(stored, &["tag_message"]).to_owned(),
            });
        }
        if fixed {
            return Err(ReleaseDecisionError::DecisionChanged {
                detail: "the meaning or the accepted anomaly ids differ from the acknowledged choice",
            });
        }
    }
    if fixed {
        // A run past its gate, or with the gate acknowledged, that holds no
        // outcome to repeat takes no release choice, and nothing is written.
        return Err(ReleaseDecisionError::Workflow(
            WorkflowHostError::DecisionConflict {
                detail: "the run has no release outcome to repeat and its gate is closed"
                    .to_owned(),
            },
        ));
    }
    let release = prepare_release(request, run, presentation, &wanted)?;
    let item = json!({
        "id": "outcome",
        "meaning": meaning,
        "accepted_anomalies": wanted,
        "document": release.document,
        "record_path": release.record_path,
        "tag_message": release.tag_message,
    });
    if stored.is_some() {
        run.update_item("release_outcome", "outcome", &item)?;
    } else {
        run.add_item("release_outcome", &item)?;
    }
    Ok(release)
}

/// Every refusal of the `release` choice, before the gate is acknowledged, and
/// the Release document it yields.
fn prepare_release(
    request: &Request,
    run: &BoundRun,
    presentation: &Value,
    wanted: &[&str],
) -> Result<ReleaseOutput, ReleaseDecisionError> {
    let meaning = request.meaning.as_deref().unwrap_or_default();
    if !is_meaning(meaning) {
        return Err(ReleaseDecisionError::MeaningInvalid);
    }
    if !is_repository_path(&request.snapshot_path) {
        return Err(ReleaseDecisionError::SnapshotPathInvalid {
            path: request.snapshot_path.clone(),
        });
    }
    let commit = text_at(presentation, &["baseline", "commit"]);
    let name = text_at(presentation, &["baseline", "name"]);
    let absent: Vec<String> = presentation["baseline"]["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| item["present"] == false)
        .filter_map(|item| item["path"].as_str().map(str::to_owned))
        .collect();
    if !absent.is_empty() {
        return Err(ReleaseDecisionError::BaselineItemAbsent { paths: absent });
    }
    let stored = run
        .items("release_request")?
        .first()
        .ok_or(ReleaseDecisionError::PresentationMissing)?;
    let release_id = text_at(stored, &["release_id"]);
    let record_path = text_at(stored, &["record_path"]);
    let repository = Path::new(&request.repository);
    let git_failed = |_| ReleaseDecisionError::Baseline("release_decision_git_failed");
    if path_exists_at(repository, commit, &request.snapshot_path).map_err(git_failed)? {
        return Err(ReleaseDecisionError::SnapshotPathPresentAtBaseline {
            path: request.snapshot_path.clone(),
        });
    }
    if path_exists_at(repository, commit, record_path).map_err(git_failed)? {
        return Err(ReleaseDecisionError::RecordPathPresentAtBaseline {
            path: record_path.to_owned(),
        });
    }

    // Every anomaly the report lists, and every document it could not read,
    // is named by the owner or the release is refused.
    let mut listed: Vec<&str> = presentation["anomalies"]["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|anomaly| anomaly["id"].as_str())
        .collect();
    listed.extend(
        presentation["anomalies"]["unreadable"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry["path"].as_str()),
    );
    if let Some(id) = wanted.iter().find(|id| !listed.contains(id)) {
        return Err(ReleaseDecisionError::AcceptedAnomalyUnknown {
            id: (*id).to_owned(),
        });
    }
    let unaccepted: Vec<String> = listed
        .iter()
        .filter(|id| !wanted.contains(id))
        .map(|id| (*id).to_owned())
        .collect();
    if !unaccepted.is_empty() {
        return Err(ReleaseDecisionError::AnomalyNotAccepted { ids: unaccepted });
    }

    Ok(ReleaseOutput {
        document: release_document(
            release_id,
            name,
            &request.baseline_path,
            commit,
            &request.snapshot_path,
            &request.decision_owner,
            wanted,
        ),
        record_path: record_path.to_owned(),
        tag_message: tag_message(release_id, name, meaning),
    })
}

fn text_at<'a>(value: &'a Value, path: &[&str]) -> &'a str {
    path.iter()
        .try_fold(value, |value, key| value.get(key))
        .and_then(Value::as_str)
        .unwrap_or_default()
}

/// A JSON string is a YAML double-quoted scalar, so any value is carried
/// unchanged whatever characters it holds.
fn quoted(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

fn release_document(
    id: &str,
    name: &str,
    baseline_path: &str,
    baseline_commit: &str,
    snapshot_path: &str,
    owner: &str,
    accepted: &[&str],
) -> String {
    let accepted_section = if accepted.is_empty() {
        String::new()
    } else {
        let entries: Vec<String> = accepted
            .iter()
            .map(|anomaly| format!("  - {}\n", quoted(anomaly)))
            .collect();
        format!("accepted_anomalies:\n{}", entries.concat())
    };
    format!(
        "---\nid: {id}\ntitle: {}\ntype: Release\nname: {}\nbaseline:\n  path: {}\n  commit: {}\n\
         snapshot_path: {}\ndecision_owner: {}\n{accepted_section}---\n\n# Release of {}\n\n\
         ## Release Summary\n\nThis record names the Baseline and the decision owner of the \
         release. The release decision is the decision owner's signed release tag; this record \
         is not an approval.\n",
        quoted(&format!("Release of {name}")),
        quoted(name),
        quoted(baseline_path),
        quoted(baseline_commit),
        quoted(snapshot_path),
        quoted(owner),
        name.split_whitespace().collect::<Vec<_>>().join(" ")
    )
}

/// The tag message with its digest line left open: the owner appends the
/// `git-blob:` id that git reports for the record at the release commit.
fn tag_message(id: &str, name: &str, meaning: &str) -> String {
    let subject = name.split_whitespace().collect::<Vec<_>>().join(" ");
    format!(
        "Release {subject}\n\n{RECORD_ID_TRAILER}: {id}\n{RECORD_DIGEST_TRAILER}: git-blob:\n\
         {MEANING_TOKEN}: {meaning}\n"
    )
}

fn decision_event(result: WorkflowHostResult) -> Result<DecisionEvent, ReleaseDecisionError> {
    match result.outcome {
        WorkflowHostOutcome::Decision { decision } => Ok(decision),
        WorkflowHostOutcome::Snapshot { .. } => Err(invalid("the gate returned no decision")),
    }
}
