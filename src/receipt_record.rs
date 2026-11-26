// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Receipt-input validation and statement construction (FR-074, FR-075).

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::implements;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    approval_gate::RolePolicy,
    approval_statement::ApprovalStatementLookup,
    attestation_signing::{STATEMENT_V1_TYPE, StatementV1, SubjectV1},
    attestation_verify::{TEST_RESULT_PREDICATE, VERIFICATION_SUMMARY_PREDICATE},
    change_record::{AgentSession, ReviewKind, ReviewVerdict, Ticket},
    receipt::ExpectedReceiptSubject,
    record_sink::{
        config::{ErrorCode, SinkKind, Visibility},
        form::{
            GitObjectId, Hashes, PublicProtocol, PublicRecord, Record, RecordId, RecordKind,
            Reference, References, Subject, Timestamp, Timestamps, Verdict, Verdicts,
        },
        sink::{ApprovalStatementReader, SinkLayer},
    },
};

/// Closed private-store receipt-input protocol.
pub const RECEIPT_INPUT_PROTOCOL: &str = "engineering-assurance.receipt-input/v1";
/// Result protocol emitted by the receipt producer.
pub const RECEIPT_RECORD_RESULT_PROTOCOL: &str = "engineering-assurance.receipt-record-result/v1";
/// Unprivileged gates-job artifact protocol.
pub const RECEIPT_GATE_RESULTS_PROTOCOL: &str = "engineering-assurance.receipt-gate-results/v1";
const MAX_RECEIPT_INPUT_BYTES: usize = 8 * 1024 * 1024;

/// Private review result supplied by the pre-merge flow.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReceiptReview {
    /// Artifact category.
    pub kind: ReviewKind,
    /// Review outcome.
    pub verdict: ReviewVerdict,
    /// Exact UTF-8 artifact contents retained privately.
    pub artifact: String,
}

/// Closed receipt input stored before merge and looked up by pull request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReceiptInput {
    /// Protocol identifier.
    pub protocol: String,
    /// Pull request reference in owner/repo#number form.
    pub pull_request: String,
    /// Linear ticket reference.
    pub ticket: Option<Ticket>,
    /// Ticket prose retained only in the private form.
    pub ticket_text: Option<String>,
    /// Ticket acceptance-criterion identifiers.
    pub criteria: Vec<String>,
    /// Specification-criterion identifiers.
    pub spec_criteria: Vec<String>,
    /// Private review artifacts and verdicts.
    pub reviews: Vec<ReceiptReview>,
    /// Producer-reported agent session descriptor.
    pub session: Option<AgentSession>,
}

impl ReceiptInput {
    /// Parse and validate a closed input for the exact merged pull request.
    ///
    /// # Errors
    ///
    /// Returns `ReceiptRecordError::ReceiptInput` for malformed or mismatched
    /// data.
    pub fn parse(bytes: &[u8], expected_pull_request: &str) -> Result<Self, ReceiptRecordError> {
        if bytes.len() > MAX_RECEIPT_INPUT_BYTES {
            return Err(ReceiptRecordError::ReceiptInput);
        }
        let input: Self =
            serde_json::from_slice(bytes).map_err(|_| ReceiptRecordError::ReceiptInput)?;
        if input.protocol != RECEIPT_INPUT_PROTOCOL
            || input.pull_request != expected_pull_request
            || !valid_pull_request(&input.pull_request)
            || input.criteria.len() > 256
            || input.spec_criteria.len() > 512
            || input.reviews.len() > 256
            || input
                .ticket_text
                .as_ref()
                .is_some_and(|text| text.len() > 1_048_576)
            || input.criteria.iter().any(|item| !valid_identifier(item))
            || input
                .spec_criteria
                .iter()
                .any(|item| !valid_identifier(item))
            || input
                .reviews
                .iter()
                .any(|review| review.artifact.len() > 1_048_576)
            || has_duplicates(&input.criteria)
            || has_duplicates(&input.spec_criteria)
        {
            return Err(ReceiptRecordError::ReceiptInput);
        }
        Ok(input)
    }
}

/// Bounded reader for receipt input keyed by repository and PR ref.
pub trait ReceiptInputReader {
    /// Read one retained input document.
    ///
    /// # Errors
    ///
    /// Returns `ReceiptInputReadError` when the private store cannot complete
    /// the lookup.
    fn read_receipt_input(
        &self,
        destination: &str,
        repository: &str,
        merged_pull_request: &str,
    ) -> Result<Option<Vec<u8>>, ReceiptInputReadError>;
}

/// Persist one immutable receipt-input document before the merge.
pub trait ReceiptInputWriter: ReceiptInputReader {
    /// Create a receipt-input document if absent; identical replay is allowed.
    ///
    /// # Errors
    ///
    /// Returns an error when the input is invalid, belongs to another PR, or
    /// cannot be retained.
    fn write_receipt_input(
        &self,
        destination: &str,
        repository: &str,
        merged_pull_request: &str,
        input: &ReceiptInput,
    ) -> Result<(), ReceiptInputReadError>;
}

/// Private-store receipt-input lookup failure.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("receipt input lookup failed")]
pub struct ReceiptInputReadError;

/// Gate outcomes returned by the unprivileged gates job.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReceiptGateResults {
    /// Protocol identifier.
    pub protocol: String,
    /// Exact pushed commit tested by the gate runs.
    pub commit: String,
    /// All configured gate outcomes.
    pub results: Vec<ReceiptGateResult>,
}

/// One configured gate outcome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReceiptGateResult {
    /// Configured gate name.
    pub name: String,
    /// Observed normal exit code.
    pub exit_code: i32,
}

/// Typed refusals produced before a receipt bundle can be emitted.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ReceiptRecordError {
    /// Gate artifact is malformed or names another commit.
    #[error("receipt gate results are invalid")]
    GateResults,
    /// Quoin Test Result statement has invalid shape or subject.
    #[error("receipt Test Result statement is invalid")]
    TestResult,
    /// Retained receipt input is malformed or belongs to another PR.
    #[error("retained receipt input is invalid")]
    ReceiptInput,
    /// Trust policy or approval statements are unavailable or invalid.
    #[error("receipt approval evidence is unavailable")]
    ApprovalEvidence,
    /// Statement, record, or private form construction failed.
    #[error("receipt construction failed")]
    Construction,
    /// Resolved sink cannot retain a receipt.
    #[error("receipt private store is required")]
    PrivateStoreRequired,
    /// A public repository must use its configured public transparency sink.
    #[error("receipt public log is required")]
    PublicLogRequired,
    /// Private-repository timestamp signing failed.
    #[error("receipt private signing is unavailable")]
    PrivateSigningUnavailable,
    /// The selected sink refused storage, signing, leak checking, or publication.
    #[error("receipt sink refused publication: {0:?}")]
    Sink(ErrorCode),
}

/// Protected inputs for constructing the three receipt statements.
///
/// The gate Test Result is the exact data exported by the published Quoin
/// exporter. The approval reader and role policy must use protected-base
/// inputs.
pub struct ReceiptRecordInput<'a> {
    /// Expected commit, tree, repository, and merged PR.
    pub subject: &'a ExpectedReceiptSubject,
    /// Gate result artifact from the unprivileged job.
    pub gate_results: &'a ReceiptGateResults,
    /// Exact gate names from the protected receipt policy.
    pub expected_gate_names: &'a [&'a str],
    /// Quoin-exported Test Result statement.
    pub test_result: &'a Value,
    /// Receipt input read from the private store for the PR.
    pub receipt_input: Option<&'a ReceiptInput>,
    /// Protected role policy.
    pub role_policy: &'a RolePolicy,
    /// Exact verified role decisions from approval lookup.
    pub approvals: Vec<(String, crate::change_record::ApprovalDecision)>,
    /// Unique record id supplied by the host.
    pub record_id: crate::record_sink::form::RecordId,
    /// Creation timestamp in UTC seconds.
    pub created_at: &'a str,
    /// Receipt workflow identity URI.
    pub verifier_id: &'a str,
    /// Protected role-policy URI.
    pub policy_uri: &'a str,
}

/// Three statements before signing; callers pass the canonical bytes to the
/// shared `SinkLayer` so private storage and signer selection remain common.
pub struct ReceiptStatements {
    /// Canonical change-record statement.
    pub change_record: Vec<u8>,
    /// Canonical Quoin Test Result statement.
    pub test_result: Vec<u8>,
    /// Canonical verification summary.
    pub summary: Vec<u8>,
    /// Change-record status, also used for the summary result.
    pub status: crate::change_record::Status,
    /// Full private change-record data.
    pub private_form: crate::record_sink::form::PrivateRecord,
}

/// A signed and published three-statement receipt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReceiptRecordResult {
    /// Result protocol identifier.
    pub protocol: &'static str,
    /// Covered commit.
    pub commit: String,
    /// Change-record outcome.
    pub status: crate::change_record::Status,
    /// The change record, Test Result and verification summary bundles.
    pub bundles: crate::receipt::ReceiptBundleSet,
}

/// Retain the private form first, sign all three statements through the
/// resolved `SinkLayer`, and return the resulting bundle set.
///
/// # Errors
///
/// Returns a typed refusal for a missing private store, invalid record data,
/// signing failure, leak-check refusal, or sink publication failure.
/// Protected identity, subject and timing inputs for receipt publication.
pub struct ReceiptPublishContext<'a> {
    /// Bundle verifier for the selected Sigstore trust root.
    pub verifier: &'a dyn crate::attestation_signing::BundleVerifier,
    /// Protected workflow-identity policy for the receipt writer.
    pub signer_policy: &'a crate::attestation_signing::WorkflowIdentityPolicy,
    /// Protected policy for the caller workflow identity.
    pub source_policy: &'a crate::attestation_signing::SourceWorkflowPolicy,
    /// Receipt subject and merged pull-request identity.
    pub subject: &'a ExpectedReceiptSubject,
    /// Unique receipt record identifier.
    pub record_id: &'a RecordId,
    /// Receipt creation timestamp.
    pub created_at: &'a str,
}

/// Retain the private record, publish all three signed statements, and return
/// the bundles for the receipt note.
///
/// # Errors
///
/// Returns a typed error when retention, signing, identity verification, or
/// sink publication fails.
#[implements("FR-074-AC-7")]
pub fn publish_receipt(
    layer: &SinkLayer<'_>,
    context: &ReceiptPublishContext<'_>,
    statements: ReceiptStatements,
) -> Result<ReceiptRecordResult, ReceiptRecordError> {
    if layer.sink_kind() == SinkKind::None {
        return Err(ReceiptRecordError::PrivateStoreRequired);
    }
    if layer.sink_kind() == SinkKind::PrivateStore
        && layer.repository_visibility(&context.subject.repository) == Ok(Visibility::Public)
    {
        return Err(ReceiptRecordError::PublicLogRequired);
    }
    let created =
        Timestamp::try_from(context.created_at).map_err(|_| ReceiptRecordError::Construction)?;
    let statement: crate::change_record::ChangeStatement =
        serde_json::from_slice(&statements.change_record)
            .map_err(|_| ReceiptRecordError::Construction)?;
    let digest = format!(
        "sha256:{}",
        sigstore_verify::crypto::sha256(&statements.change_record).to_hex()
    );
    let ticket_ref = statement
        .predicate
        .ticket
        .as_ref()
        .map(|ticket| Reference::linear_issue(&ticket.id))
        .transpose()
        .map_err(|_| ReceiptRecordError::Construction)?;
    let public = PublicRecord {
        protocol: PublicProtocol::V1,
        record_id: context.record_id.clone(),
        kind: RecordKind::ChangeRecord,
        subject: Subject::repo(&context.subject.repository)
            .map_err(|_| ReceiptRecordError::Construction)?,
        references: References::new(ticket_ref.into_iter().collect())
            .map_err(|_| ReceiptRecordError::Construction)?,
        hashes: Hashes {
            statement: Some(
                crate::record_sink::form::Sha256Digest::try_from(digest)
                    .map_err(|_| ReceiptRecordError::Construction)?,
            ),
            subject_commit: Some(
                GitObjectId::try_from(context.subject.commit.as_str())
                    .map_err(|_| ReceiptRecordError::Construction)?,
            ),
            ..Hashes::default()
        },
        verdicts: Verdicts {
            change_record: Some(match statements.status {
                crate::change_record::Status::Passed => Verdict::Recorded,
                crate::change_record::Status::Failed => Verdict::Failed,
            }),
            ..Verdicts::default()
        },
        timestamps: Timestamps {
            created: Some(created),
            ..Timestamps::default()
        },
        signature: None,
    };
    let record = Record::new(public, statements.private_form)
        .map_err(|_| ReceiptRecordError::Construction)?;
    let bundles = layer
        .publish_receipt(
            &record,
            &[
                statements.change_record,
                statements.test_result,
                statements.summary,
            ],
            context.verifier,
            context.signer_policy,
            context.source_policy,
        )
        .map_err(|result| {
            let code = result.error.unwrap_or(ErrorCode::SigningFailed);
            if code == ErrorCode::SigningFailed && layer.sink_kind() == SinkKind::PrivateStore {
                ReceiptRecordError::PrivateSigningUnavailable
            } else {
                ReceiptRecordError::Sink(code)
            }
        })?;
    if bundles.len() != 3 {
        return Err(ReceiptRecordError::Construction);
    }
    let mut parsed = bundles
        .into_iter()
        .map(|bundle| serde_json::from_str(&bundle).map_err(|_| ReceiptRecordError::Construction));
    let change_record = parsed.next().ok_or(ReceiptRecordError::Construction)??;
    let test_result = parsed.next().ok_or(ReceiptRecordError::Construction)??;
    let summary = parsed.next().ok_or(ReceiptRecordError::Construction)??;
    Ok(ReceiptRecordResult {
        protocol: RECEIPT_RECORD_RESULT_PROTOCOL,
        commit: context.subject.commit.clone(),
        status: statements.status,
        bundles: crate::receipt::ReceiptBundleSet {
            change_record,
            test_result,
            summary,
        },
    })
}

/// Build the change-record, Test Result, and verification summary statements.
///
/// Missing input and incomplete roles become a failed Change Record. Invalid
/// gate attestations and malformed retained input fail closed.
///
/// # Errors
///
/// Returns a typed refusal for invalid gate/Test Result data or failed
/// canonical statement construction.
pub fn build_receipt_statements(
    mut input: ReceiptRecordInput<'_>,
) -> Result<ReceiptStatements, ReceiptRecordError> {
    validate_gate_results(&input)?;
    let test_statement = validated_test_statement(&input)?;
    validate_retained_receipt_input(&input)?;
    let failures = receipt_failures(&input);
    let reviews = receipt_reviews(&input);
    let private_form = private_receipt_form(&input);
    let approvals =
        normalize_role_decisions(input.role_policy, std::mem::take(&mut input.approvals));
    let built = build_receipt_change_record(&input, reviews, approvals, failures, private_form)?;
    let change_json =
        serde_json::to_value(&built.statement).map_err(|_| ReceiptRecordError::Construction)?;
    let change_record = serde_json_canonicalizer::to_vec(&change_json)
        .map_err(|_| ReceiptRecordError::Construction)?;
    let test_result = serde_json_canonicalizer::to_vec(&test_statement)
        .map_err(|_| ReceiptRecordError::Construction)?;
    let summary = build_receipt_summary(
        &input,
        built.statement.predicate.status,
        &change_record,
        &test_result,
    )?;
    Ok(ReceiptStatements {
        change_record,
        test_result,
        summary,
        status: built.statement.predicate.status,
        private_form: built.private_form,
    })
}

fn validate_gate_results(input: &ReceiptRecordInput<'_>) -> Result<(), ReceiptRecordError> {
    let results = &input.gate_results.results;
    let result_names = results
        .iter()
        .map(|result| result.name.as_str())
        .collect::<BTreeSet<_>>();
    let expected_names = input
        .expected_gate_names
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let names = results
        .iter()
        .map(|result| result.name.clone())
        .collect::<Vec<_>>();
    if input.gate_results.protocol != RECEIPT_GATE_RESULTS_PROTOCOL
        || input.gate_results.commit != input.subject.commit
        || results.is_empty()
        || results.len() > 256
        || input.expected_gate_names.is_empty()
        || input.expected_gate_names.len() > 256
        || results
            .iter()
            .any(|result| result.name.is_empty() || result.name.len() > 256)
        || has_duplicates(&names)
        || has_duplicates(input.expected_gate_names)
        || result_names != expected_names
    {
        return Err(ReceiptRecordError::GateResults);
    }
    Ok(())
}

fn validated_test_statement(
    input: &ReceiptRecordInput<'_>,
) -> Result<StatementV1, ReceiptRecordError> {
    let statement: StatementV1 = serde_json::from_value(input.test_result.clone())
        .map_err(|_| ReceiptRecordError::TestResult)?;
    let predicate =
        serde_json::to_value(&statement.predicate).map_err(|_| ReceiptRecordError::TestResult)?;
    crate::receipt::validate_test_result(&predicate).map_err(|_| ReceiptRecordError::TestResult)?;
    let valid_subject = statement.subject.len() == 1
        && statement.subject.iter().all(|subject| {
            subject.name == input.subject.repository
                && subject.digest.get("gitCommit") == Some(&input.subject.commit)
                && subject.digest.get("gitTree") == Some(&input.subject.tree)
        });
    if statement.type_ != STATEMENT_V1_TYPE
        || statement.predicate_type != TEST_RESULT_PREDICATE
        || !valid_subject
    {
        return Err(ReceiptRecordError::TestResult);
    }
    Ok(statement)
}

fn validate_retained_receipt_input(
    input: &ReceiptRecordInput<'_>,
) -> Result<(), ReceiptRecordError> {
    if input.receipt_input.is_some_and(|retained| {
        retained.protocol != RECEIPT_INPUT_PROTOCOL
            || retained.pull_request != input.subject.merged_pull_request
    }) {
        return Err(ReceiptRecordError::ReceiptInput);
    }
    Ok(())
}

fn receipt_failures(input: &ReceiptRecordInput<'_>) -> Vec<crate::change_record::Failure> {
    let mut failures = Vec::new();
    if input
        .gate_results
        .results
        .iter()
        .any(|gate| gate.exit_code != 0)
    {
        failures.push(crate::change_record::Failure::GateFailed);
    }
    if input.receipt_input.is_none() {
        failures.push(crate::change_record::Failure::ContractMissing);
    }
    if input
        .receipt_input
        .is_none_or(|retained| retained.reviews.is_empty())
    {
        failures.push(crate::change_record::Failure::ReviewsMissing);
    }
    failures
}

fn receipt_reviews(input: &ReceiptRecordInput<'_>) -> Vec<crate::change_record::ReviewInput> {
    input.receipt_input.map_or_else(Vec::new, |retained| {
        retained
            .reviews
            .iter()
            .map(|review| crate::change_record::ReviewInput {
                kind: review.kind,
                verdict: review.verdict,
                artifact_bytes: review.artifact.as_bytes().to_vec(),
            })
            .collect()
    })
}

fn private_receipt_form(input: &ReceiptRecordInput<'_>) -> crate::record_sink::form::PrivateRecord {
    let mut private_form = crate::record_sink::form::PrivateRecord::new(input.record_id.clone());
    private_form.change_record = Some(input.receipt_input.map_or_else(
        || json!({"protocol": RECEIPT_INPUT_PROTOCOL, "pullRequest": input.subject.merged_pull_request}),
        |retained| json!({
            "protocol": retained.protocol,
            "pullRequest": retained.pull_request,
            "ticketText": retained.ticket_text,
            "criteria": retained.criteria,
            "specCriteria": retained.spec_criteria,
            "reviews": retained.reviews,
        }),
    ));
    private_form.ticket_text = input
        .receipt_input
        .and_then(|retained| retained.ticket_text.clone());
    private_form.criteria_text = input.receipt_input.map(|retained| {
        serde_json::to_string(
            &json!({"criteria":retained.criteria,"specCriteria":retained.spec_criteria}),
        )
        .unwrap_or_default()
    });
    private_form
}

fn build_receipt_change_record(
    input: &ReceiptRecordInput<'_>,
    reviews: Vec<crate::change_record::ReviewInput>,
    approvals: Vec<(String, crate::change_record::ApprovalDecision)>,
    failures: Vec<crate::change_record::Failure>,
    private_form: crate::record_sink::form::PrivateRecord,
) -> Result<crate::change_record::BuiltChangeRecord, ReceiptRecordError> {
    let retained = input.receipt_input;
    crate::change_record::build_change_record(crate::change_record::PredicateInput {
        record_id: input.record_id.as_str().to_owned(),
        ticket: retained.and_then(|value| value.ticket.clone()),
        criteria: retained.map_or_else(Vec::new, |value| value.criteria.clone()),
        spec_criteria: retained.map_or_else(Vec::new, |value| value.spec_criteria.clone()),
        reviews,
        approvals,
        test_results: input.test_result.clone(),
        subject: crate::change_record::ChangeSubjectInput {
            repository: input.subject.repository.clone(),
            merged_commit: input.subject.commit.clone(),
            merged_tree: input.subject.tree.clone(),
        },
        session: retained.and_then(|value| value.session.clone()),
        private_form,
        failures,
    })
    .map_err(|_| ReceiptRecordError::Construction)
}

fn build_receipt_summary(
    input: &ReceiptRecordInput<'_>,
    status: crate::change_record::Status,
    change_record: &[u8],
    test_result: &[u8],
) -> Result<Vec<u8>, ReceiptRecordError> {
    use crate::change_record::Status;

    let digest =
        |bytes: &[u8]| format!("sha256:{}", sigstore_verify::crypto::sha256(bytes).to_hex());
    let time_verified = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|_| ReceiptRecordError::Construction)?;
    let summary = StatementV1 {
        type_: STATEMENT_V1_TYPE.to_owned(),
        subject: vec![SubjectV1 {
            name: input.subject.repository.clone(),
            digest: BTreeMap::from([
                ("gitCommit".to_owned(), input.subject.commit.clone()),
                ("gitTree".to_owned(), input.subject.tree.clone()),
            ]),
        }],
        predicate_type: VERIFICATION_SUMMARY_PREDICATE.to_owned(),
        predicate: BTreeMap::from([
            ("verifier".to_owned(), json!({"id": input.verifier_id})),
            ("timeVerified".to_owned(), json!(time_verified)),
            (
                "resourceUri".to_owned(),
                json!(format!(
                    concat!("https:", "//github.com/{}/commit/{}"),
                    input.subject.repository, input.subject.commit
                )),
            ),
            ("policy".to_owned(), json!({"uri": input.policy_uri})),
            (
                "inputAttestations".to_owned(),
                json!([
                    {"uri":"urn:engineering-assurance:receipt:change-record","digest":{"sha256":digest(change_record)}},
                    {"uri":"urn:engineering-assurance:receipt:test-result","digest":{"sha256":digest(test_result)}}
                ]),
            ),
            (
                "verificationResult".to_owned(),
                json!(match status {
                    Status::Passed => "PASSED",
                    Status::Failed => "FAILED",
                }),
            ),
            ("verifiedLevels".to_owned(), json!([])),
        ]),
    };
    serde_json_canonicalizer::to_vec(&summary).map_err(|_| ReceiptRecordError::Construction)
}

fn normalize_role_decisions(
    policy: &RolePolicy,
    decisions: Vec<(String, crate::change_record::ApprovalDecision)>,
) -> Vec<(String, crate::change_record::ApprovalDecision)> {
    use crate::change_record::ApprovalDecision;

    let mut by_role = BTreeMap::new();
    let mut invalid = false;
    for (role, decision) in decisions {
        if !policy.required_roles.contains(&role) || by_role.insert(role, decision).is_some() {
            invalid = true;
        }
    }
    policy
        .required_roles
        .iter()
        .map(|role| {
            let has_eligible_person = policy
                .roles
                .get(role)
                .is_some_and(|people| !people.is_empty());
            let decision = if invalid {
                ApprovalDecision::Unavailable
            } else {
                by_role.remove(role).unwrap_or(if has_eligible_person {
                    ApprovalDecision::Missing
                } else {
                    ApprovalDecision::Unstaffed
                })
            };
            let decision = match decision {
                ApprovalDecision::Covered {
                    individual,
                    is_human,
                    statement,
                } if is_human
                    && policy
                        .roles
                        .get(role)
                        .is_some_and(|people| people.contains(&individual)) =>
                {
                    ApprovalDecision::Covered {
                        individual,
                        is_human,
                        statement,
                    }
                }
                ApprovalDecision::Covered { .. } => ApprovalDecision::Unavailable,
                other => other,
            };
            (role.clone(), decision)
        })
        .collect()
}

/// Query and verify all approval statements for the exact merged PR and
/// commit, then return one role decision per protected required role.
pub fn verified_receipt_approvals(
    reader: &dyn ApprovalStatementReader,
    destination: &str,
    subject: &ExpectedReceiptSubject,
    identity: &crate::attestation_signing::WorkflowIdentityPolicy,
    policy: &RolePolicy,
    verifier: &dyn crate::attestation_signing::BundleVerifier,
) -> Vec<(String, crate::change_record::ApprovalDecision)> {
    use crate::change_record::ApprovalDecision;
    if subject.merged_pull_request.is_empty() {
        return missing_role_decisions(policy);
    }
    let lookup = ApprovalStatementLookup {
        repository: &subject.repository,
        merged_pull_request: &subject.merged_pull_request,
        merged_commit: &subject.commit,
    };
    let Ok(retained) = reader.lookup_approval_statements(destination, lookup) else {
        return unavailable_role_decisions(policy);
    };
    if retained.is_empty() {
        return missing_role_decisions(policy);
    }
    let snapshot = RetainedApprovalSnapshot {
        statements: retained.clone(),
    };
    let result = crate::approval_statement::verify_approval_statements(
        &snapshot,
        destination,
        lookup,
        identity,
        policy,
        verifier,
    );
    if result.status != crate::approval_statement::ApprovalStatementStatus::Verified {
        return unavailable_role_decisions(policy);
    }
    let mut decisions = Vec::with_capacity(policy.required_roles.len());
    for role in &policy.required_roles {
        let Some(assignment) = result
            .role_verdict
            .assignments
            .iter()
            .find(|item| &item.role == role)
        else {
            decisions.push((role.clone(), missing_role_decision(policy, role)));
            continue;
        };
        let Some((approval, statement)) = retained.iter().find_map(|retained| {
            let approval = crate::approval_statement::approval_from_statement(
                &retained.statement_bytes,
                lookup,
            )
            .ok()?;
            if approval.role != *role || approval.account != assignment.account {
                return None;
            }
            let statement = serde_json::from_slice::<Value>(&retained.statement_bytes).ok()?;
            Some((approval, statement))
        }) else {
            decisions.push((role.clone(), ApprovalDecision::Unavailable));
            continue;
        };
        decisions.push((
            role.clone(),
            ApprovalDecision::Covered {
                individual: assignment.individual.clone(),
                is_human: approval.human,
                statement,
            },
        ));
    }
    decisions
}

fn missing_role_decisions(
    policy: &RolePolicy,
) -> Vec<(String, crate::change_record::ApprovalDecision)> {
    policy
        .required_roles
        .iter()
        .map(|role| (role.clone(), missing_role_decision(policy, role)))
        .collect()
}

fn missing_role_decision(
    policy: &RolePolicy,
    role: &str,
) -> crate::change_record::ApprovalDecision {
    if policy
        .roles
        .get(role)
        .is_some_and(|people| !people.is_empty())
    {
        crate::change_record::ApprovalDecision::Missing
    } else {
        crate::change_record::ApprovalDecision::Unstaffed
    }
}

fn unavailable_role_decisions(
    policy: &RolePolicy,
) -> Vec<(String, crate::change_record::ApprovalDecision)> {
    policy
        .required_roles
        .iter()
        .map(|role| {
            (
                role.clone(),
                crate::change_record::ApprovalDecision::Unavailable,
            )
        })
        .collect()
}

struct RetainedApprovalSnapshot {
    statements: Vec<crate::approval_statement::RetainedApprovalStatement>,
}

impl ApprovalStatementReader for RetainedApprovalSnapshot {
    fn lookup_approval_statements(
        &self,
        _destination: &str,
        _lookup: ApprovalStatementLookup<'_>,
    ) -> Result<
        Vec<crate::approval_statement::RetainedApprovalStatement>,
        crate::approval_statement::ApprovalStatementReadError,
    > {
        Ok(self.statements.clone())
    }
}

fn valid_pull_request(value: &str) -> bool {
    let Some((repository, number)) = value.split_once('#') else {
        return false;
    };
    let Some((owner, name)) = repository.split_once('/') else {
        return false;
    };
    !owner.is_empty()
        && !name.is_empty()
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn has_duplicates<T: Ord>(values: &[T]) -> bool {
    let mut seen = BTreeSet::new();
    values.iter().any(|value| !seen.insert(value))
}
