// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Receipt bundle and Git-note contracts (FR-074, FR-076, FR-077, FR-078).
//!
//! This module defines the bounded JSON Lines envelope used by receipt
//! workflows. Signature creation and verification remain delegated to the
//! attestation-signing stack (NFR-010, NFR-011).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::attestation_signing::{
    BundleVerifier, ExpectedSubject, SourceWorkflowPolicy, StatementV1, SubjectKind,
    VerificationFailure, VerificationStatus, WorkflowIdentityPolicy, verify_statement_with_source,
};
use crate::attestation_verify::VerificationSummaryStatement;
use crate::change_record::predicate::validate_predicate;
use crate::change_record::{ChangeStatement, PREDICATE_TYPE, Status};
use crate::{
    approval_gate::{GateStatus, RolePolicy},
    approval_statement::{
        ApprovalStatementLookup, ApprovalStatementStatus, RetainedApprovalStatement,
        approval_from_statement, verify_approval_statements,
    },
    record_sink::sink::{ApprovalStatementReadError, ApprovalStatementReader},
};

/// Git notes ref that stores signed receipt bundles (FR-076).
pub const RECEIPT_NOTES_REF: &str = "refs/notes/assurance-receipts";
/// Maximum serialized receipt note size (FR-076).
pub const MAX_RECEIPT_NOTE_BYTES: usize = 1024 * 1024;
/// Receipt verification result protocol (FR-078).
pub const VERIFY_RESULT_PROTOCOL: &str = "engineering-assurance.receipt-verify-result/v1";
/// Missing receipt result protocol (FR-077).
pub const MISSING_RESULT_PROTOCOL: &str = "engineering-assurance.receipt-missing-result/v1";

/// One receipt's three Sigstore bundles in their semantic roles.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptBundleSet {
    /// Signed FR-072 change-record statement bundle.
    pub change_record: Value,
    /// Signed gate Test Result statement bundle.
    pub test_result: Value,
    /// Signed SLSA verification summary bundle.
    pub summary: Value,
}

/// A closed status returned by `receipt-verify` (FR-078).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptStatus {
    /// A passed receipt verified completely.
    Verified,
    /// A failed receipt verified completely.
    VerifiedFailed,
    /// The certificate does not match the protected receipt workflow identity.
    UntrustedSigner,
    /// A bundle signature or certificate proof is invalid.
    SignatureInvalid,
    /// The receipt names a different commit or tree.
    WrongSubject,
    /// A statement or predicate violates its closed contract.
    PredicateInvalid,
    /// The receipt's claimed roles do not match verified approval statements.
    ApprovalsMismatch,
    /// Required note or trust material could not be read.
    Unverifiable,
}

/// Whether a verified receipt makes a repeated note write idempotent.
#[must_use]
pub const fn receipt_status_allows_idempotent_skip(status: ReceiptStatus) -> bool {
    matches!(
        status,
        ReceiptStatus::Verified | ReceiptStatus::VerifiedFailed
    )
}

/// A closed reason accompanying an `unverifiable` result (FR-078).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnverifiableReason {
    /// No receipt note exists for the requested commit.
    NoteAbsent,
    /// The note could not be read or parsed.
    NoteUnreadable,
    /// Protected signer trust material could not be read.
    TrustRootUnreadable,
    /// Protected approval policy could not be read.
    PolicyUnreadable,
    /// The retained private approval-statement backend is unavailable.
    StatementsNotRetained,
}

/// One machine-readable receipt verification result.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptVerifyResult {
    /// Closed wire protocol identifier.
    pub protocol: String,
    /// Verification status.
    pub status: ReceiptStatus,
    /// Present only when status is `unverifiable`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<UnverifiableReason>,
}

/// A reason why a commit is reported by the missing-receipt check (FR-077).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingReceiptReason {
    /// No note exists under the receipt notes ref.
    NoNote,
    /// A note exists but receipt verification failed.
    InvalidReceipt,
    /// Note, verification, or platform push-time status is unknown.
    StatusUnknown,
    /// A receipt verified and records a failed gate or review.
    FailedReceipt,
}

/// One commit and its missing-receipt classification.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissingReceipt {
    /// Full commit object id.
    pub commit: String,
    /// Closed reason for listing the commit.
    pub reason: MissingReceiptReason,
}

/// Receipt state already obtained from note verification for one commit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptCheckState {
    /// No note exists for the commit.
    NoNote,
    /// The note was read but did not verify.
    InvalidReceipt,
    /// Verification or note status could not be determined.
    StatusUnknown,
    /// A passing receipt verified.
    Verified,
    /// A failed receipt verified.
    VerifiedFailed,
}

/// One commit on the default branch's first-parent history.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptHistoryEntry {
    /// Full commit object id.
    pub commit: String,
    /// Platform-recorded first reachability time, as Unix seconds.
    pub pushed_at_unix: Option<u64>,
    /// Whether a receipt run is currently active for the commit.
    pub receipt_run_in_progress: bool,
    /// Outcome from receipt note verification.
    pub state: ReceiptCheckState,
}

/// Failure to evaluate a missing-receipt request.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum MissingReceiptError {
    /// `adopted_at` does not identify an entry in first-parent order.
    #[error("receipt policy adopted_at is not on first-parent history")]
    AdoptedAtInvalid,
    /// The policy grace period is outside the supported 0..=1440 range.
    #[error("receipt policy grace_minutes is invalid")]
    GraceInvalid,
    /// The supplied history contains an invalid or repeated object id.
    #[error("receipt history has an invalid or repeated commit id")]
    HistoryInvalid,
}

/// Find missing or failed receipts after the adopted commit.
///
/// Entries must be ordered newest first as returned by `git rev-list
/// --first-parent`. Push ages use platform-recorded timestamps, never commit
/// author or committer dates.
///
/// # Errors
///
/// Returns a typed error for invalid policy bounds, object ids, duplicate
/// commits, or an adopted commit outside the supplied history.
pub fn find_missing_receipts(
    history: &[ReceiptHistoryEntry],
    adopted_at: &str,
    grace_minutes: u16,
    now_unix: u64,
) -> Result<Vec<MissingReceipt>, MissingReceiptError> {
    if grace_minutes > 1440 {
        return Err(MissingReceiptError::GraceInvalid);
    }
    let mut seen = std::collections::BTreeSet::new();
    if history
        .iter()
        .any(|entry| !is_object_id(&entry.commit) || !seen.insert(entry.commit.as_str()))
    {
        return Err(MissingReceiptError::HistoryInvalid);
    }
    let Some(adopted_index) = history.iter().position(|entry| entry.commit == adopted_at) else {
        return Err(MissingReceiptError::AdoptedAtInvalid);
    };
    let grace_seconds = u64::from(grace_minutes) * 60;
    let mut findings = Vec::new();
    for entry in &history[..=adopted_index] {
        let pushed_at = entry
            .pushed_at_unix
            .filter(|pushed_at| *pushed_at <= now_unix);
        let age = pushed_at.map(|pushed_at| now_unix - pushed_at);
        let within_grace = age.is_some_and(|seconds| seconds < grace_seconds);
        let reason = match (pushed_at, entry.state) {
            (_, ReceiptCheckState::VerifiedFailed) => Some(MissingReceiptReason::FailedReceipt),
            (None, _) => Some(MissingReceiptReason::StatusUnknown),
            (Some(_), ReceiptCheckState::NoNote) => Some(MissingReceiptReason::NoNote),
            (Some(_), ReceiptCheckState::InvalidReceipt) => {
                Some(MissingReceiptReason::InvalidReceipt)
            }
            (Some(_), ReceiptCheckState::StatusUnknown) => {
                Some(MissingReceiptReason::StatusUnknown)
            }
            (Some(_), ReceiptCheckState::Verified) => None,
        };
        if within_grace && reason != Some(MissingReceiptReason::FailedReceipt) {
            continue;
        }
        if let Some(reason) = reason {
            findings.push(MissingReceipt {
                commit: entry.commit.clone(),
                reason,
            });
        }
    }
    Ok(findings)
}

fn is_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Typed errors for receipt note encoding and parsing.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ReceiptNoteError {
    /// The note is larger than the FR-076 one MiB maximum.
    #[error("receipt note exceeds 1 MiB")]
    TooLarge,
    /// The note is not exactly three JSON Lines bundle records.
    #[error("receipt note must contain exactly three JSON bundle lines")]
    InvalidLines,
    /// A line is not a valid JSON bundle object.
    #[error("receipt note contains an invalid bundle object")]
    InvalidBundle,
}

/// Commit and tree values that a receipt must bind to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedReceiptSubject {
    /// Repository in owner/name form.
    pub repository: String,
    /// Full Git commit object id.
    pub commit: String,
    /// Full Git tree object id.
    pub tree: String,
    /// Merged pull-request reference used to query approval statement proofs.
    pub merged_pull_request: String,
}

/// Why receipt statements could not be structurally verified.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum ReceiptVerificationError {
    /// A bundle is invalid or does not contain a DSSE envelope.
    #[error("receipt bundle is invalid")]
    BundleInvalid,
    /// A statement violates the required predicate shape or digest links.
    #[error("receipt predicate is invalid")]
    PredicateInvalid,
    /// A canonical statement digest could not be computed.
    #[error("receipt statement digest is unavailable")]
    DigestUnavailable,
}

/// Protected trust inputs used to verify one receipt.
pub struct ReceiptVerificationContext<'a> {
    /// The pushed commit and its tree.
    pub subject: &'a ExpectedReceiptSubject,
    /// Protected-base identity policy for the receipt workflow.
    pub receipt_signer: &'a WorkflowIdentityPolicy,
    /// Protected-base reusable-workflow caller policy.
    pub receipt_source: &'a SourceWorkflowPolicy,
    /// Protected-base identity policy for approval statement writers.
    pub approval_signer: &'a WorkflowIdentityPolicy,
    /// Protected-base role policy.
    pub role_policy: &'a RolePolicy,
    /// Private store reader for retained approval statement proofs.
    pub approval_reader: &'a dyn ApprovalStatementReader,
    /// Name of the private store destination selected by protected policy.
    pub private_destination: &'a str,
    /// Sigstore verifier created from the trusted root.
    pub verifier: &'a dyn BundleVerifier,
}

impl ReceiptBundleSet {
    /// Encode the bundle set as exactly three newline-terminated JSON Lines.
    ///
    /// # Errors
    ///
    /// Returns [`ReceiptNoteError::TooLarge`] when the encoded note exceeds
    /// the FR-076 maximum.
    pub fn encode_note(&self) -> Result<Vec<u8>, ReceiptNoteError> {
        let mut note = Vec::new();
        for bundle in [&self.change_record, &self.test_result, &self.summary] {
            serde_json::to_writer(&mut note, bundle)
                .map_err(|_| ReceiptNoteError::InvalidBundle)?;
            note.push(b'\n');
            if note.len() > MAX_RECEIPT_NOTE_BYTES {
                return Err(ReceiptNoteError::TooLarge);
            }
        }
        Self::parse_note(&note)?;
        Ok(note)
    }

    /// Parse three JSON Lines bundles, rejecting blank lines and extra data.
    ///
    /// # Errors
    ///
    /// Returns a typed error for oversized, malformed, or non-three-line
    /// notes.
    pub fn parse_note(bytes: &[u8]) -> Result<Self, ReceiptNoteError> {
        let mut groups = Self::parse_note_groups(bytes)?;
        if groups.len() != 1 {
            return Err(ReceiptNoteError::InvalidLines);
        }
        Ok(groups.remove(0))
    }

    /// Parse one or more complete receipt groups without relying on line order.
    /// Each summary binds its change-record and test-result by canonical digest.
    ///
    /// # Errors
    ///
    /// Returns a typed error for oversized, malformed, or incomplete receipt
    /// groups.
    pub fn parse_note_groups(bytes: &[u8]) -> Result<Vec<Self>, ReceiptNoteError> {
        if bytes.len() > MAX_RECEIPT_NOTE_BYTES {
            return Err(ReceiptNoteError::TooLarge);
        }
        let lines = bytes.split(|byte| *byte == b'\n').collect::<Vec<_>>();
        let lines = if lines.last().is_some_and(|line| line.is_empty()) {
            &lines[..lines.len().saturating_sub(1)]
        } else {
            &lines[..]
        };
        if lines.len() < 3 || lines.len() % 3 != 0 || lines.iter().any(|line| line.is_empty()) {
            return Err(ReceiptNoteError::InvalidLines);
        }
        let mut bundles = Vec::with_capacity(3);
        for line in lines {
            let bundle: Value =
                serde_json::from_slice(line).map_err(|_| ReceiptNoteError::InvalidBundle)?;
            if !bundle.is_object() {
                return Err(ReceiptNoteError::InvalidBundle);
            }
            bundles.push(bundle);
        }
        let mut change_records = Vec::new();
        let mut test_results = Vec::new();
        let mut summaries = Vec::new();
        for bundle in bundles {
            let predicate_type =
                bundle_predicate_type(&bundle).ok_or(ReceiptNoteError::InvalidBundle)?;
            let slot = match predicate_type.as_str() {
                crate::change_record::PREDICATE_TYPE => &mut change_records,
                crate::attestation_verify::TEST_RESULT_PREDICATE => &mut test_results,
                crate::attestation_verify::VERIFICATION_SUMMARY_PREDICATE => &mut summaries,
                _ => return Err(ReceiptNoteError::InvalidBundle),
            };
            slot.push(bundle);
        }
        if change_records.len() != summaries.len() || test_results.len() != summaries.len() {
            return Err(ReceiptNoteError::InvalidBundle);
        }
        let mut unused_changes = change_records.into_iter().collect::<Vec<_>>();
        let mut unused_tests = test_results.into_iter().collect::<Vec<_>>();
        let mut result = Vec::with_capacity(summaries.len());
        for summary in summaries {
            let summary_statement = statement_payload(&summary)
                .and_then(|payload| {
                    serde_json::from_slice::<VerificationSummaryStatement>(&payload).ok()
                })
                .ok_or(ReceiptNoteError::InvalidBundle)?;
            let digests = summary_statement
                .predicate
                .input_attestations
                .iter()
                .map(|input| input.digest.sha256.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            if digests.len() != 2 {
                return Err(ReceiptNoteError::InvalidBundle);
            }
            let change_index = matching_bundle_index(&unused_changes, digests.iter().copied())?;
            let test_index = matching_bundle_index(&unused_tests, digests.iter().copied())?;
            result.push(Self {
                change_record: unused_changes.remove(change_index),
                test_result: unused_tests.remove(test_index),
                summary,
            });
        }
        if !unused_changes.is_empty() || !unused_tests.is_empty() {
            return Err(ReceiptNoteError::InvalidBundle);
        }
        Ok(result)
    }
}

fn matching_bundle_index<'a>(
    bundles: &[Value],
    digests: impl Iterator<Item = &'a str>,
) -> Result<usize, ReceiptNoteError> {
    let wanted = digests.collect::<std::collections::BTreeSet<_>>();
    let mut matches = Vec::new();
    for (index, bundle) in bundles.iter().enumerate() {
        let payload = statement_payload(bundle).ok_or(ReceiptNoteError::InvalidBundle)?;
        let digest = canonical_json_digest(&payload).ok_or(ReceiptNoteError::InvalidBundle)?;
        if wanted.contains(digest.as_str()) {
            matches.push(index);
        }
    }
    matches
        .first()
        .copied()
        .ok_or(ReceiptNoteError::InvalidBundle)
}

fn statement_payload(bundle_value: &Value) -> Option<Vec<u8>> {
    let bundle =
        sigstore_types::Bundle::from_json(&serde_json::to_string(bundle_value).ok()?).ok()?;
    let sigstore_types::SignatureContent::DsseEnvelope(envelope) = bundle.content else {
        return None;
    };
    Some(envelope.payload.as_ref().to_vec())
}

fn canonical_json_digest(bytes: &[u8]) -> Option<String> {
    canonical_statement_digest(bytes).ok()
}

fn bundle_predicate_type(bundle_value: &Value) -> Option<String> {
    let bundle_json = serde_json::to_string(bundle_value).ok()?;
    let bundle = sigstore_types::Bundle::from_json(&bundle_json).ok()?;
    let sigstore_types::SignatureContent::DsseEnvelope(envelope) = bundle.content else {
        return None;
    };
    let statement: crate::attestation_signing::StatementV1 =
        serde_json::from_slice(envelope.payload.as_ref()).ok()?;
    Some(statement.predicate_type)
}

/// Verify a receipt's three signed statements and its retained approval proof set.
///
/// All policies in `context` must come from the protected base commit. The
/// approval reader is queried only for the exact repository, pull request and
/// merged commit supplied in the expected subject.
///
/// # Errors
///
/// Returns a typed error for malformed bundle, statement, predicate, summary,
/// or digest data. Signature and identity failures are returned as closed
/// [`ReceiptStatus`] values.
pub fn verify_receipt(
    bundles: &ReceiptBundleSet,
    context: &ReceiptVerificationContext<'_>,
) -> Result<ReceiptStatus, ReceiptVerificationError> {
    verify_receipt_inner(bundles, context)
}

fn verify_receipt_inner(
    bundles: &ReceiptBundleSet,
    context: &ReceiptVerificationContext<'_>,
) -> Result<ReceiptStatus, ReceiptVerificationError> {
    let change = extract_statement(&bundles.change_record)?;
    let test = extract_statement(&bundles.test_result)?;
    let summary = extract_statement(&bundles.summary)?;
    for statement in [&change, &test, &summary] {
        match verify_statement_with_source(
            context.verifier,
            &statement.payload,
            &statement.bundle_json,
            context.receipt_signer,
            Some(context.receipt_source),
            ExpectedSubject {
                kind: SubjectKind::Commit,
                value: &context.subject.commit,
            },
        ) {
            Ok(VerificationStatus::Verified) => {}
            Err(error) => return Ok(status_for_verification_failure(error)),
        }
    }

    let change_statement =
        verify_receipt_predicates(&change.payload, &test.payload, &summary.payload, context)?;
    verify_receipt_approvals(&change_statement, context)
}

fn verify_receipt_predicates(
    change_payload: &[u8],
    test_payload: &[u8],
    summary_payload: &[u8],
    context: &ReceiptVerificationContext<'_>,
) -> Result<ChangeStatement, ReceiptVerificationError> {
    let change_statement = parse_change_record(change_payload)?;
    if change_statement.predicate_type != PREDICATE_TYPE
        || validate_predicate(&change_statement.predicate).is_err()
        || !change_statement_has_subject(
            &change_statement.subject,
            &context.subject.repository,
            &context.subject.tree,
        )
    {
        return Err(ReceiptVerificationError::PredicateInvalid);
    }
    let test_statement: StatementV1 = serde_json::from_slice(test_payload)
        .map_err(|_| ReceiptVerificationError::PredicateInvalid)?;
    let test_result = validate_test_result(
        &serde_json::to_value(&test_statement.predicate)
            .map_err(|_| ReceiptVerificationError::PredicateInvalid)?,
    )?;
    if test_statement.predicate_type != crate::attestation_verify::TEST_RESULT_PREDICATE
        || !statement_has_subject(
            &test_statement.subject,
            &context.subject.repository,
            &context.subject.tree,
        )
        || (test_result.result == "FAILED" && change_statement.predicate.status != Status::Failed)
    {
        return Err(ReceiptVerificationError::PredicateInvalid);
    }
    let summary_statement: VerificationSummaryStatement =
        serde_json::from_slice(summary_payload)
            .map_err(|_| ReceiptVerificationError::PredicateInvalid)?;
    if summary_statement.type_ != crate::attestation_signing::STATEMENT_V1_TYPE
        || summary_statement.predicate_type
            != crate::attestation_verify::VERIFICATION_SUMMARY_PREDICATE
        || !summary_statement.subject.iter().any(|subject| {
            subject.name == context.subject.repository
                && subject
                    .digest
                    .get("gitTree")
                    .is_some_and(|actual| actual == &context.subject.tree)
        })
        || summary_statement.predicate.input_attestations.len() != 2
        || summary_statement
            .predicate
            .input_attestations
            .iter()
            .any(|input| input.uri.is_empty())
        || summary_statement.predicate.time_verified.is_empty()
        || summary_statement.predicate.resource_uri.is_empty()
        || summary_statement.predicate.verifier.id.is_empty()
        || summary_statement.predicate.policy.uri.is_empty()
    {
        return Err(ReceiptVerificationError::PredicateInvalid);
    }

    let change_digest = canonical_statement_digest(change_payload)?;
    let test_digest = canonical_statement_digest(test_payload)?;
    let mut input_digests = summary_statement
        .predicate
        .input_attestations
        .iter()
        .map(|input| input.digest.sha256.as_str())
        .collect::<Vec<_>>();
    input_digests.sort_unstable();
    let mut expected_digests = [change_digest.as_str(), test_digest.as_str()];
    expected_digests.sort_unstable();
    if input_digests != expected_digests || change_statement.predicate.test_results != test_digest {
        return Err(ReceiptVerificationError::PredicateInvalid);
    }
    let expected_result = match change_statement.predicate.status {
        Status::Passed => "PASSED",
        Status::Failed => "FAILED",
    };
    if summary_statement.predicate.verification_result != expected_result {
        return Err(ReceiptVerificationError::PredicateInvalid);
    }
    Ok(change_statement)
}

fn verify_receipt_approvals(
    change_statement: &ChangeStatement,
    context: &ReceiptVerificationContext<'_>,
) -> Result<ReceiptStatus, ReceiptVerificationError> {
    let lookup = ApprovalStatementLookup {
        repository: &context.subject.repository,
        merged_pull_request: &context.subject.merged_pull_request,
        merged_commit: &context.subject.commit,
    };
    let retained = match context
        .approval_reader
        .lookup_approval_statements(context.private_destination, lookup)
    {
        Ok(retained) => retained,
        Err(ApprovalStatementReadError) => return Ok(ReceiptStatus::Unverifiable),
    };
    let mut retained_digests = std::collections::BTreeSet::new();
    let mut retained_roles = std::collections::BTreeSet::new();
    let mut retained_pairs = std::collections::BTreeSet::new();
    for statement in &retained {
        let digest = canonical_statement_digest(&statement.statement_bytes)?;
        if !retained_digests.insert(digest.clone()) {
            return Ok(ReceiptStatus::ApprovalsMismatch);
        }
        let Ok(approval) = approval_from_statement(&statement.statement_bytes, lookup) else {
            return Ok(ReceiptStatus::ApprovalsMismatch);
        };
        if !retained_roles.insert(approval.role.clone())
            || !retained_pairs.insert((approval.role, digest))
        {
            return Ok(ReceiptStatus::ApprovalsMismatch);
        }
    }
    let mut claimed_digests = std::collections::BTreeSet::new();
    let mut claimed_roles = std::collections::BTreeSet::new();
    let mut claimed_pairs = std::collections::BTreeSet::new();
    for entry in &change_statement.predicate.approvals {
        if !claimed_roles.insert(entry.role.as_str()) {
            return Ok(ReceiptStatus::ApprovalsMismatch);
        }
        if entry.verdict == crate::change_record::predicate::ApprovalVerdict::Covered {
            let Some(digest) = entry.statement_digest.as_deref() else {
                return Ok(ReceiptStatus::ApprovalsMismatch);
            };
            if !claimed_digests.insert(digest.to_owned())
                || !claimed_pairs.insert((entry.role.clone(), digest.to_owned()))
            {
                return Ok(ReceiptStatus::ApprovalsMismatch);
            }
        }
    }
    if retained_digests != claimed_digests {
        return Ok(ReceiptStatus::ApprovalsMismatch);
    }
    let reader = SnapshotApprovalReader {
        statements: retained,
    };
    let approval_result = verify_approval_statements(
        &reader,
        context.private_destination,
        lookup,
        context.approval_signer,
        context.role_policy,
        context.verifier,
    );
    match approval_result.status {
        ApprovalStatementStatus::Verified => {}
        ApprovalStatementStatus::UntrustedSigner => return Ok(ReceiptStatus::UntrustedSigner),
        ApprovalStatementStatus::SignatureInvalid => return Ok(ReceiptStatus::SignatureInvalid),
        ApprovalStatementStatus::WrongSubject => return Ok(ReceiptStatus::WrongSubject),
        ApprovalStatementStatus::PredicateInvalid => return Ok(ReceiptStatus::PredicateInvalid),
        ApprovalStatementStatus::Unverifiable => return Ok(ReceiptStatus::Unverifiable),
    }
    if retained_pairs != claimed_pairs {
        return Ok(ReceiptStatus::ApprovalsMismatch);
    }
    let policy_roles = context
        .role_policy
        .required_roles
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    if policy_roles != claimed_roles {
        return Ok(ReceiptStatus::ApprovalsMismatch);
    }
    if change_statement.predicate.status == Status::Passed
        && approval_result.role_verdict.status != GateStatus::Satisfied
    {
        return Ok(ReceiptStatus::ApprovalsMismatch);
    }

    Ok(match change_statement.predicate.status {
        Status::Passed => ReceiptStatus::Verified,
        Status::Failed => ReceiptStatus::VerifiedFailed,
    })
}

/// Verify grouped receipt history and choose the newest fully verified group.
/// If every group fails verification, report the result belonging to the
/// summary with the latest valid `timeVerified` value.
///
/// # Errors
///
/// Returns an error when no groups are supplied or a summary cannot be parsed
/// or does not contain a valid `timeVerified` value.
pub fn verify_receipt_groups(
    groups: &[ReceiptBundleSet],
    context: &ReceiptVerificationContext<'_>,
) -> Result<ReceiptStatus, ReceiptVerificationError> {
    verify_receipt_groups_inner(groups, context)
}

fn verify_receipt_groups_inner(
    groups: &[ReceiptBundleSet],
    context: &ReceiptVerificationContext<'_>,
) -> Result<ReceiptStatus, ReceiptVerificationError> {
    if groups.is_empty() {
        return Err(ReceiptVerificationError::PredicateInvalid);
    }
    let mut results = Vec::with_capacity(groups.len());
    for group in groups {
        let summary = extract_statement(&group.summary)?;
        let statement: VerificationSummaryStatement = serde_json::from_slice(&summary.payload)
            .map_err(|_| ReceiptVerificationError::PredicateInvalid)?;
        let time = time::OffsetDateTime::parse(
            &statement.predicate.time_verified,
            &time::format_description::well_known::Rfc3339,
        )
        .map_err(|_| ReceiptVerificationError::PredicateInvalid)?
        .unix_timestamp_nanos();
        let status = match verify_receipt_inner(group, context) {
            Ok(status) => status,
            Err(error) => status_from_verification_error(error),
        };
        results.push((time, status));
    }
    let verified = results
        .iter()
        .filter(|(_, status)| {
            matches!(
                status,
                ReceiptStatus::Verified | ReceiptStatus::VerifiedFailed
            )
        })
        .max_by_key(|(time, _)| *time);
    Ok(verified
        .or_else(|| results.iter().max_by_key(|(time, _)| *time))
        .expect("nonempty group list has a result")
        .1)
}

fn status_from_verification_error(error: ReceiptVerificationError) -> ReceiptStatus {
    match error {
        ReceiptVerificationError::BundleInvalid => ReceiptStatus::SignatureInvalid,
        ReceiptVerificationError::PredicateInvalid
        | ReceiptVerificationError::DigestUnavailable => ReceiptStatus::PredicateInvalid,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TestResultPredicate {
    result: String,
    configuration: Value,
    #[serde(rename = "passedTests")]
    _passed_tests: Vec<String>,
    #[serde(rename = "warnedTests")]
    warned_tests: Vec<String>,
    #[serde(rename = "failedTests")]
    failed_tests: Vec<String>,
}

struct ExtractedStatement {
    payload: Vec<u8>,
    bundle_json: String,
}

fn extract_statement(bundle_value: &Value) -> Result<ExtractedStatement, ReceiptVerificationError> {
    let bundle_json =
        serde_json::to_string(bundle_value).map_err(|_| ReceiptVerificationError::BundleInvalid)?;
    let bundle = sigstore_types::Bundle::from_json(&bundle_json)
        .map_err(|_| ReceiptVerificationError::BundleInvalid)?;
    let sigstore_types::SignatureContent::DsseEnvelope(envelope) = bundle.content else {
        return Err(ReceiptVerificationError::BundleInvalid);
    };
    Ok(ExtractedStatement {
        payload: envelope.payload.as_ref().to_vec(),
        bundle_json,
    })
}

fn parse_change_record(payload: &[u8]) -> Result<ChangeStatement, ReceiptVerificationError> {
    serde_json::from_slice(payload).map_err(|_| ReceiptVerificationError::PredicateInvalid)
}

fn statement_has_subject(
    subjects: &[crate::attestation_signing::SubjectV1],
    repository: &str,
    tree: &str,
) -> bool {
    subjects.iter().any(|subject| {
        subject.name == repository
            && subject
                .digest
                .get("gitTree")
                .is_some_and(|actual| actual == tree)
    })
}

fn change_statement_has_subject(
    subjects: &[crate::change_record::StatementSubject],
    repository: &str,
    tree: &str,
) -> bool {
    subjects.iter().any(|subject| {
        subject.name == repository
            && subject
                .digest
                .get("gitTree")
                .is_some_and(|actual| actual == tree)
    })
}

pub(crate) fn validate_test_result(
    predicate: &Value,
) -> Result<TestResultPredicate, ReceiptVerificationError> {
    let predicate: TestResultPredicate = serde_json::from_value(predicate.clone())
        .map_err(|_| ReceiptVerificationError::PredicateInvalid)?;
    let expected_result = if !predicate.failed_tests.is_empty() {
        "FAILED"
    } else if !predicate.warned_tests.is_empty() {
        "WARNED"
    } else {
        "PASSED"
    };
    if predicate.result != expected_result || predicate.configuration.is_null() {
        return Err(ReceiptVerificationError::PredicateInvalid);
    }
    Ok(predicate)
}

fn canonical_statement_digest(bytes: &[u8]) -> Result<String, ReceiptVerificationError> {
    let statement: StatementV1 =
        serde_json::from_slice(bytes).map_err(|_| ReceiptVerificationError::PredicateInvalid)?;
    let canonical = statement
        .canonical_bytes()
        .map_err(|_| ReceiptVerificationError::DigestUnavailable)?;
    Ok(format!(
        "sha256:{}",
        sigstore_verify::crypto::sha256(&canonical).to_hex()
    ))
}

fn status_for_verification_failure(failure: VerificationFailure) -> ReceiptStatus {
    match failure {
        VerificationFailure::UntrustedSigner => ReceiptStatus::UntrustedSigner,
        VerificationFailure::WrongSubject => ReceiptStatus::WrongSubject,
        VerificationFailure::InvalidSignature | VerificationFailure::InvalidStatement => {
            ReceiptStatus::SignatureInvalid
        }
    }
}

struct SnapshotApprovalReader {
    statements: Vec<RetainedApprovalStatement>,
}

impl ApprovalStatementReader for SnapshotApprovalReader {
    fn lookup_approval_statements(
        &self,
        _destination: &str,
        _lookup: ApprovalStatementLookup<'_>,
    ) -> Result<Vec<RetainedApprovalStatement>, ApprovalStatementReadError> {
        Ok(self.statements.clone())
    }
}

/// Validate the protocol and the conditional reason field of a result.
///
/// # Errors
///
/// Returns `false` for a protocol mismatch or a reason/status mismatch.
#[must_use]
pub fn is_valid_verify_result(result: &ReceiptVerifyResult) -> bool {
    result.protocol == VERIFY_RESULT_PROTOCOL
        && match result.status {
            ReceiptStatus::Unverifiable => result.reason.is_some(),
            _ => result.reason.is_none(),
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attestation_signing::{
        AuthenticatedWorkflowClaims, SourceWorkflowPolicy, TransparencyRequirement,
        WorkflowIdentityPolicy,
    };
    use crate::record_sink::sink::{ApprovalStatementLookup, RetainedApprovalStatement};
    use ix_trace_rs::trace;
    use serde_json::json;
    use std::collections::BTreeSet;

    const FIXTURE_REPOSITORY: &str = "fictional.example/project.example";
    const FIXTURE_RECEIPT_ISSUER: &str = "issuer.example";

    fn signed_bundle_payload(payload: Vec<u8>) -> Value {
        let envelope = sigstore_types::DsseEnvelope::new(
            "application/vnd.in-toto+json",
            sigstore_types::PayloadBytes::from(payload),
            sigstore_types::DsseSignature::new(
                sigstore_types::SignatureBytes::from(vec![0]),
                sigstore_types::KeyId::default(),
            ),
        );
        let bundle = sigstore_types::Bundle::new(
            sigstore_types::MediaType::Bundle0_3,
            sigstore_types::VerificationMaterial::new(
                sigstore_types::VerificationMaterialContent::PublicKey(
                    sigstore_types::PublicKeyIdentifier::new("fictional"),
                ),
            ),
            sigstore_types::SignatureContent::DsseEnvelope(envelope),
        );
        serde_json::to_value(bundle).expect("bundle fixture encodes")
    }

    fn bundle_set() -> ReceiptBundleSet {
        bundle_set_with("one", "2026-11-14T00:00:00Z")
    }

    fn fixture_source_policy() -> SourceWorkflowPolicy {
        SourceWorkflowPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            event: "push".to_owned(),
            source_ref: Some("refs/heads/main".to_owned()),
            source_ref_prefix: None,
        }
    }

    fn bundle_set_with(marker: &str, time_verified: &str) -> ReceiptBundleSet {
        let change_record = signed_bundle_payload(
            serde_json::to_vec(&json!({
                "_type": crate::attestation_signing::STATEMENT_V1_TYPE,
                "subject": [],
                "predicateType": crate::change_record::PREDICATE_TYPE,
                "predicate": {"marker": marker}
            }))
            .unwrap(),
        );
        let test_result = signed_bundle_payload(
            serde_json::to_vec(&json!({
                "_type": crate::attestation_signing::STATEMENT_V1_TYPE,
                "subject": [],
                "predicateType": crate::attestation_verify::TEST_RESULT_PREDICATE,
                "predicate": {"marker": marker}
            }))
            .unwrap(),
        );
        let change = extract_statement(&change_record).unwrap();
        let test = extract_statement(&test_result).unwrap();
        let change_digest = canonical_statement_digest(&change.payload).unwrap();
        let test_digest = canonical_statement_digest(&test.payload).unwrap();
        let summary = signed_bundle_payload(
            serde_json::to_vec(&json!({
                "_type": crate::attestation_signing::STATEMENT_V1_TYPE,
                "subject": [],
                "predicateType": crate::attestation_verify::VERIFICATION_SUMMARY_PREDICATE,
                "predicate": {
                    "verifier": {"id": "fixture"},
                    "timeVerified": time_verified,
                    "resourceUri": "fixture",
                    "policy": {"uri": "fixture"},
                    "inputAttestations": [
                        {"uri": "change", "digest": {"sha256": change_digest}},
                        {"uri": "test", "digest": {"sha256": test_digest}}
                    ],
                    "verificationResult": "PASSED",
                    "verifiedLevels": []
                }
            }))
            .unwrap(),
        );
        ReceiptBundleSet {
            change_record,
            test_result,
            summary,
        }
    }

    struct FixtureVerifier {
        rejected_bundles: BTreeSet<String>,
        invalid_bundles: BTreeSet<String>,
        mismatched_claims: bool,
    }

    impl BundleVerifier for FixtureVerifier {
        fn verify(
            &self,
            _payload: &[u8],
            bundle_json: &str,
            _policy: &WorkflowIdentityPolicy,
        ) -> Result<(), VerificationFailure> {
            if self.rejected_bundles.contains(bundle_json) {
                Err(VerificationFailure::UntrustedSigner)
            } else if self.invalid_bundles.contains(bundle_json) {
                Err(VerificationFailure::InvalidSignature)
            } else {
                Ok(())
            }
        }

        fn verify_with_claims(
            &self,
            payload: &[u8],
            bundle_json: &str,
            policy: &WorkflowIdentityPolicy,
        ) -> Result<Option<AuthenticatedWorkflowClaims>, VerificationFailure> {
            self.verify(payload, bundle_json, policy)?;
            let source = fixture_source_policy();
            let claims = source.expected_claims_for_test();
            Ok(Some(if self.mismatched_claims {
                AuthenticatedWorkflowClaims::default()
            } else {
                claims
            }))
        }
    }

    struct EmptyApprovalReader;

    impl ApprovalStatementReader for EmptyApprovalReader {
        fn lookup_approval_statements(
            &self,
            _destination: &str,
            _lookup: ApprovalStatementLookup<'_>,
        ) -> Result<Vec<RetainedApprovalStatement>, ApprovalStatementReadError> {
            Ok(Vec::new())
        }
    }

    struct OneApprovalReader(RetainedApprovalStatement);

    impl ApprovalStatementReader for OneApprovalReader {
        fn lookup_approval_statements(
            &self,
            _destination: &str,
            _lookup: ApprovalStatementLookup<'_>,
        ) -> Result<Vec<RetainedApprovalStatement>, ApprovalStatementReadError> {
            Ok(vec![self.0.clone()])
        }
    }

    struct ManyApprovalReader(Vec<RetainedApprovalStatement>);

    impl ApprovalStatementReader for ManyApprovalReader {
        fn lookup_approval_statements(
            &self,
            _destination: &str,
            _lookup: ApprovalStatementLookup<'_>,
        ) -> Result<Vec<RetainedApprovalStatement>, ApprovalStatementReadError> {
            Ok(self.0.clone())
        }
    }

    fn valid_receipt_fixture(status: Status) -> (ReceiptBundleSet, OneApprovalReader, RolePolicy) {
        let repository = FIXTURE_REPOSITORY;
        let commit = "a".repeat(40);
        let tree = "b".repeat(40);
        let pull_request = format!("{FIXTURE_REPOSITORY}#1");
        let approval_statement = serde_json::to_vec(&json!({
            "_type": crate::attestation_signing::STATEMENT_V1_TYPE,
            "subject": [{"name": repository, "digest": {"gitCommit": commit}}],
            "predicateType": crate::approval_statement::APPROVAL_PREDICATE_TYPE,
            "predicate": {
                "source": "github",
                "approver": "github:123",
                "approver_name": "Fixture Reviewer",
                "role": "engineer",
                "meaning": "approved",
                "signed_at": "2026-11-13T00:00:00Z",
                "head_commit": "c".repeat(40),
                "pull_request": pull_request,
                "merged_commit": commit
            }
        }))
        .unwrap();
        let approval_digest = canonical_statement_digest(&approval_statement).unwrap();
        let test_statement = json!({
            "_type": crate::attestation_signing::STATEMENT_V1_TYPE,
            "subject": [{
                "name": repository,
                "digest": {"gitCommit": commit, "gitTree": tree}
            }],
            "predicateType": crate::attestation_verify::TEST_RESULT_PREDICATE,
            "predicate": {
                "result": if status == Status::Passed { "PASSED" } else { "FAILED" },
                "configuration": {},
                "passedTests": if status == Status::Passed { vec!["fixture"] } else { vec![] },
                "warnedTests": [],
                "failedTests": if status == Status::Passed { vec![] } else { vec!["fixture"] }
            }
        });
        let test_payload = serde_json::to_vec(&test_statement).unwrap();
        let test_digest = canonical_statement_digest(&test_payload).unwrap();
        let change_statement = json!({
            "_type": crate::attestation_signing::STATEMENT_V1_TYPE,
            "subject": [{
                "name": repository,
                "digest": {"gitCommit": commit, "gitTree": tree}
            }],
            "predicateType": crate::change_record::PREDICATE_TYPE,
            "predicate": {
                "record_id": "11111111-1111-4111-8111-111111111111",
                "status": if status == Status::Passed { "passed" } else { "failed" },
                "ticket": {"kind": "linear_issue", "id": "EA-108"},
                "criteria": [],
                "spec_criteria": [],
                "reviews": [],
                "approvals": [{
                    "role": "engineer", "verdict": "covered", "statement_digest": approval_digest
                }],
                "test_results": test_digest,
                "agent_bom": format!("sha256:{}", "1".repeat(64)),
                "private_form": format!("sha256:{}", "2".repeat(64)),
                "failures": if status == Status::Passed { vec![] } else { vec!["gate_failed"] }
            }
        });
        let change_payload = serde_json::to_vec(&change_statement).unwrap();
        let change_digest = canonical_statement_digest(&change_payload).unwrap();
        let summary_statement = json!({
            "_type": crate::attestation_signing::STATEMENT_V1_TYPE,
            "subject": [{
                "name": repository,
                "digest": {"gitCommit": commit, "gitTree": tree}
            }],
            "predicateType": crate::attestation_verify::VERIFICATION_SUMMARY_PREDICATE,
            "predicate": {
                "verifier": {"id": "fixture"},
                "timeVerified": "2026-11-14T00:00:00Z",
                "resourceUri": "fixture",
                "policy": {"uri": "fixture"},
                "inputAttestations": [
                    {"uri": "change", "digest": {"sha256": change_digest}},
                    {"uri": "test", "digest": {"sha256": test_digest}}
                ],
                "verificationResult": if status == Status::Passed { "PASSED" } else { "FAILED" },
                "verifiedLevels": []
            }
        });
        let bundles = ReceiptBundleSet {
            change_record: signed_bundle_payload(change_payload),
            test_result: signed_bundle_payload(test_payload),
            summary: signed_bundle_payload(serde_json::to_vec(&summary_statement).unwrap()),
        };
        let reader = OneApprovalReader(RetainedApprovalStatement {
            statement_bytes: approval_statement,
            signature_bundle: "fixture-approval-bundle".to_owned(),
            publish_outcome: crate::approval_statement::ApprovalStatementPublishOutcome::Published,
        });
        let policy = fixture_role_policy();
        (bundles, reader, policy)
    }

    fn fixture_role_policy() -> RolePolicy {
        RolePolicy {
            protocol: crate::approval_gate::POLICY_PROTOCOL.to_owned(),
            required_roles: vec!["engineer".to_owned()],
            individuals: [(
                "alice".to_owned(),
                vec![crate::approval_gate::AccountRef {
                    source: "github".to_owned(),
                    id: "123".to_owned(),
                }],
            )]
            .into(),
            roles: [("engineer".to_owned(), vec!["alice".to_owned()])].into(),
            agents: Vec::new(),
        }
    }

    fn valid_two_role_fixture() -> (ReceiptBundleSet, ManyApprovalReader, RolePolicy) {
        let (mut bundles, first_reader, mut policy) = valid_receipt_fixture(Status::Passed);
        let first = first_reader.0;
        let mut second_statement: Value = serde_json::from_slice(&first.statement_bytes).unwrap();
        second_statement["predicate"]["approver"] = json!("github:456");
        second_statement["predicate"]["approver_name"] = json!("Fixture Manager");
        second_statement["predicate"]["role"] = json!("manager");
        let second_statement = serde_json::to_vec(&second_statement).unwrap();
        let first_digest = canonical_statement_digest(&first.statement_bytes).unwrap();
        let second_digest = canonical_statement_digest(&second_statement).unwrap();

        let change_payload = extract_statement(&bundles.change_record).unwrap().payload;
        let mut change: Value = serde_json::from_slice(&change_payload).unwrap();
        change["predicate"]["approvals"] = json!([
            {"role": "engineer", "verdict": "covered", "statement_digest": first_digest},
            {"role": "manager", "verdict": "covered", "statement_digest": second_digest}
        ]);
        let change_payload = serde_json::to_vec(&change).unwrap();
        let change_digest = canonical_statement_digest(&change_payload).unwrap();
        bundles.change_record = signed_bundle_payload(change_payload);
        let summary_payload = extract_statement(&bundles.summary).unwrap().payload;
        let mut summary: Value = serde_json::from_slice(&summary_payload).unwrap();
        let input = summary["predicate"]["inputAttestations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|input| input["uri"] == "change")
            .unwrap();
        input["digest"]["sha256"] = json!(change_digest);
        bundles.summary = signed_bundle_payload(serde_json::to_vec(&summary).unwrap());

        let mut second = first.clone();
        second.statement_bytes = second_statement;
        second.signature_bundle = "fixture-manager-bundle".to_owned();
        policy.required_roles.push("manager".to_owned());
        policy.individuals.insert(
            "bob".to_owned(),
            vec![crate::approval_gate::AccountRef {
                source: "github".to_owned(),
                id: "456".to_owned(),
            }],
        );
        policy
            .roles
            .insert("manager".to_owned(), vec!["bob".to_owned()]);
        (bundles, ManyApprovalReader(vec![first, second]), policy)
    }

    fn replace_approval_rows(bundles: &mut ReceiptBundleSet, rows: Value) {
        let change_payload = extract_statement(&bundles.change_record).unwrap().payload;
        let mut change: Value = serde_json::from_slice(&change_payload).unwrap();
        change["predicate"]["approvals"] = rows;
        let change_payload = serde_json::to_vec(&change).unwrap();
        let change_digest = canonical_statement_digest(&change_payload).unwrap();
        bundles.change_record = signed_bundle_payload(change_payload);
        let summary_payload = extract_statement(&bundles.summary).unwrap().payload;
        let mut summary: Value = serde_json::from_slice(&summary_payload).unwrap();
        let input = summary["predicate"]["inputAttestations"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|input| input["uri"] == "change")
            .unwrap();
        input["digest"]["sha256"] = json!(change_digest);
        bundles.summary = signed_bundle_payload(serde_json::to_vec(&summary).unwrap());
    }

    fn verification_context<'a>(
        verifier: &'a dyn BundleVerifier,
        reader: &'a dyn ApprovalStatementReader,
        subject: &'a ExpectedReceiptSubject,
        signer: &'a WorkflowIdentityPolicy,
        source: &'a SourceWorkflowPolicy,
        role_policy: &'a RolePolicy,
    ) -> ReceiptVerificationContext<'a> {
        ReceiptVerificationContext {
            subject,
            receipt_signer: signer,
            receipt_source: source,
            approval_signer: signer,
            role_policy,
            approval_reader: reader,
            private_destination: "private-store",
            verifier,
        }
    }

    #[trace("TC-597", "TC-602", "FR-076-AC-1", "FR-076-AC-6")]
    #[test]
    fn receipt_note_round_trips_three_json_lines_without_private_text() {
        let encoded = bundle_set().encode_note().expect("bounded note encoding");
        assert_eq!(encoded.split_inclusive(|byte| *byte == b'\n').count(), 3);
        assert!(!String::from_utf8_lossy(&encoded).contains("ticket title"));
        assert_eq!(ReceiptBundleSet::parse_note(&encoded), Ok(bundle_set()));
    }

    #[trace("TC-617", "FR-078-AC-7")]
    #[test]
    fn receipt_note_reader_identifies_statements_by_predicate_type() {
        let mut lines = bundle_set().encode_note().unwrap();
        let parsed = ReceiptBundleSet::parse_note(&lines).unwrap();
        assert_eq!(parsed, bundle_set());
        lines.clear();
        let bundles = bundle_set();
        for bundle in [
            &bundles.summary,
            &bundles.change_record,
            &bundles.test_result,
        ] {
            serde_json::to_writer(&mut lines, bundle).unwrap();
            lines.push(b'\n');
        }
        assert_eq!(ReceiptBundleSet::parse_note(&lines), Ok(bundle_set()));
    }

    #[trace("TC-617", "FR-078-AC-7")]
    #[test]
    fn grouped_receipt_notes_bind_each_summary_to_its_input_digests() {
        let older = bundle_set_with("older", "2026-11-13T00:00:00Z");
        let newer = bundle_set_with("newer", "2026-11-14T00:00:00Z");
        let mut note = Vec::new();
        for bundle in [
            &newer.test_result,
            &older.summary,
            &newer.summary,
            &older.change_record,
            &newer.change_record,
            &older.test_result,
        ] {
            serde_json::to_writer(&mut note, bundle).unwrap();
            note.push(b'\n');
        }
        let groups = ReceiptBundleSet::parse_note_groups(&note).unwrap();
        assert_eq!(groups, vec![older, newer]);

        let first = bundle_set_with("shared", "2026-11-13T00:00:00Z");
        let second = bundle_set_with("shared", "2026-11-14T00:00:00Z");
        let mut duplicate_statement_note = Vec::new();
        for bundle in [
            &first.change_record,
            &first.test_result,
            &second.summary,
            &second.change_record,
            &second.test_result,
            &first.summary,
        ] {
            serde_json::to_writer(&mut duplicate_statement_note, bundle).unwrap();
            duplicate_statement_note.push(b'\n');
        }
        assert_eq!(
            ReceiptBundleSet::parse_note_groups(&duplicate_statement_note).unwrap(),
            vec![second, first]
        );
    }

    #[trace("TC-612", "TC-617", "FR-078-AC-2", "FR-078-AC-7")]
    #[test]
    fn all_invalid_groups_report_the_latest_summary_result() {
        let older = bundle_set_with("older", "2026-11-13T00:00:00Z");
        let newer = bundle_set_with("newer", "2026-11-14T00:00:00Z");
        let rejected = [
            serde_json::to_string(&newer.change_record).unwrap(),
            serde_json::to_string(&newer.test_result).unwrap(),
            serde_json::to_string(&newer.summary).unwrap(),
        ]
        .into();
        let verifier = FixtureVerifier {
            rejected_bundles: rejected,
            invalid_bundles: BTreeSet::new(),
            mismatched_claims: false,
        };
        let reader = EmptyApprovalReader;
        let subject = ExpectedReceiptSubject {
            repository: FIXTURE_REPOSITORY.to_owned(),
            commit: "a".repeat(40),
            tree: "b".repeat(40),
            merged_pull_request: format!("{FIXTURE_REPOSITORY}#1"),
        };
        let signer = WorkflowIdentityPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            references: BTreeSet::from(["refs/heads/main".to_owned()]),
            issuer: FIXTURE_RECEIPT_ISSUER.to_owned(),
            transparency: TransparencyRequirement::PublicLog,
        };
        let source = SourceWorkflowPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            event: "push".to_owned(),
            source_ref: Some("refs/heads/main".to_owned()),
            source_ref_prefix: None,
        };
        let role_policy = RolePolicy {
            protocol: "engineering-assurance.approval-policy/v1".to_owned(),
            required_roles: Vec::new(),
            individuals: std::collections::BTreeMap::default(),
            roles: std::collections::BTreeMap::default(),
            agents: Vec::new(),
        };
        let context =
            verification_context(&verifier, &reader, &subject, &signer, &source, &role_policy);
        assert_eq!(
            verify_receipt_groups(&[older, newer], &context),
            Ok(ReceiptStatus::UntrustedSigner)
        );
    }

    #[trace("TC-611", "FR-078-AC-1")]
    #[test]
    fn receipt_verification_distinguishes_passed_and_failed_records() {
        let verifier = FixtureVerifier {
            rejected_bundles: BTreeSet::new(),
            invalid_bundles: BTreeSet::new(),
            mismatched_claims: false,
        };
        let signer = WorkflowIdentityPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            references: BTreeSet::from(["refs/heads/main".to_owned()]),
            issuer: FIXTURE_RECEIPT_ISSUER.to_owned(),
            transparency: TransparencyRequirement::PublicLog,
        };
        let source = SourceWorkflowPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            event: "push".to_owned(),
            source_ref: Some("refs/heads/main".to_owned()),
            source_ref_prefix: None,
        };
        let subject = ExpectedReceiptSubject {
            repository: FIXTURE_REPOSITORY.to_owned(),
            commit: "a".repeat(40),
            tree: "b".repeat(40),
            merged_pull_request: format!("{FIXTURE_REPOSITORY}#1"),
        };
        for (change_status, expected_status) in [
            (Status::Passed, ReceiptStatus::Verified),
            (Status::Failed, ReceiptStatus::VerifiedFailed),
        ] {
            let (bundles, reader, role_policy) = valid_receipt_fixture(change_status);
            let context =
                verification_context(&verifier, &reader, &subject, &signer, &source, &role_policy);
            assert_eq!(verify_receipt(&bundles, &context), Ok(expected_status));
            assert!(receipt_status_allows_idempotent_skip(expected_status));
        }

        let (bundles, reader, role_policy) = valid_receipt_fixture(Status::Passed);
        let mismatched = FixtureVerifier {
            rejected_bundles: BTreeSet::new(),
            invalid_bundles: BTreeSet::new(),
            mismatched_claims: true,
        };
        let context = verification_context(
            &mismatched,
            &reader,
            &subject,
            &signer,
            &source,
            &role_policy,
        );
        assert_eq!(
            verify_receipt(&bundles, &context),
            Ok(ReceiptStatus::UntrustedSigner)
        );
        assert!(!receipt_status_allows_idempotent_skip(
            ReceiptStatus::UntrustedSigner
        ));
    }

    #[trace("TC-586", "FR-074-AC-4")]
    #[test]
    fn changing_a_test_result_without_updating_the_summary_digest_is_rejected() {
        let (mut bundles, reader, role_policy) = valid_receipt_fixture(Status::Passed);
        let verifier = FixtureVerifier {
            rejected_bundles: BTreeSet::new(),
            invalid_bundles: BTreeSet::new(),
            mismatched_claims: false,
        };
        let subject = ExpectedReceiptSubject {
            repository: FIXTURE_REPOSITORY.to_owned(),
            commit: "a".repeat(40),
            tree: "b".repeat(40),
            merged_pull_request: format!("{FIXTURE_REPOSITORY}#1"),
        };
        let signer = WorkflowIdentityPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            references: BTreeSet::from(["refs/heads/main".to_owned()]),
            issuer: FIXTURE_RECEIPT_ISSUER.to_owned(),
            transparency: TransparencyRequirement::PublicLog,
        };
        let source = fixture_source_policy();
        let context =
            verification_context(&verifier, &reader, &subject, &signer, &source, &role_policy);
        let test_payload = extract_statement(&bundles.test_result).unwrap().payload;
        let mut test: Value = serde_json::from_slice(&test_payload).unwrap();
        test["predicate"]["passedTests"] = json!(["changed case"]);
        bundles.test_result = signed_bundle_payload(serde_json::to_vec(&test).unwrap());
        assert_eq!(
            verify_receipt(&bundles, &context),
            Err(ReceiptVerificationError::PredicateInvalid)
        );
    }

    #[trace("TC-613", "FR-078-AC-3")]
    #[test]
    fn a_failed_test_result_cannot_be_certified_by_a_passing_change_and_summary() {
        let (mut bundles, reader, role_policy) = valid_receipt_fixture(Status::Passed);
        let test_payload = extract_statement(&bundles.test_result).unwrap().payload;
        let mut test: Value = serde_json::from_slice(&test_payload).unwrap();
        test["predicate"]["result"] = json!("FAILED");
        test["predicate"]["passedTests"] = json!([]);
        test["predicate"]["failedTests"] = json!(["fixture"]);
        let test_payload = serde_json::to_vec(&test).unwrap();
        let test_digest = canonical_statement_digest(&test_payload).unwrap();
        bundles.test_result = signed_bundle_payload(test_payload);

        let change_payload = extract_statement(&bundles.change_record).unwrap().payload;
        let mut change: Value = serde_json::from_slice(&change_payload).unwrap();
        change["predicate"]["test_results"] = json!(test_digest);
        let change_payload = serde_json::to_vec(&change).unwrap();
        let change_digest = canonical_statement_digest(&change_payload).unwrap();
        bundles.change_record = signed_bundle_payload(change_payload);

        let summary_payload = extract_statement(&bundles.summary).unwrap().payload;
        let mut summary: Value = serde_json::from_slice(&summary_payload).unwrap();
        for input in summary["predicate"]["inputAttestations"]
            .as_array_mut()
            .unwrap()
        {
            match input["uri"].as_str() {
                Some("change") => input["digest"]["sha256"] = json!(change_digest),
                Some("test") => input["digest"]["sha256"] = json!(test_digest),
                _ => {}
            }
        }
        bundles.summary = signed_bundle_payload(serde_json::to_vec(&summary).unwrap());

        let verifier = FixtureVerifier {
            rejected_bundles: BTreeSet::new(),
            invalid_bundles: BTreeSet::new(),
            mismatched_claims: false,
        };
        let subject = ExpectedReceiptSubject {
            repository: FIXTURE_REPOSITORY.to_owned(),
            commit: "a".repeat(40),
            tree: "b".repeat(40),
            merged_pull_request: format!("{FIXTURE_REPOSITORY}#1"),
        };
        let signer = WorkflowIdentityPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            references: BTreeSet::from(["refs/heads/main".to_owned()]),
            issuer: FIXTURE_RECEIPT_ISSUER.to_owned(),
            transparency: TransparencyRequirement::PublicLog,
        };
        let source = fixture_source_policy();
        let context =
            verification_context(&verifier, &reader, &subject, &signer, &source, &role_policy);
        assert_eq!(
            verify_receipt(&bundles, &context),
            Err(ReceiptVerificationError::PredicateInvalid)
        );
    }

    #[trace("TC-613", "FR-078-AC-3")]
    #[test]
    fn verification_separates_subject_signature_and_closed_schema_failures() {
        let source = SourceWorkflowPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            event: "push".to_owned(),
            source_ref: Some("refs/heads/main".to_owned()),
            source_ref_prefix: None,
        };
        let signer = WorkflowIdentityPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            references: BTreeSet::from(["refs/heads/main".to_owned()]),
            issuer: FIXTURE_RECEIPT_ISSUER.to_owned(),
            transparency: TransparencyRequirement::PublicLog,
        };
        let subject = ExpectedReceiptSubject {
            repository: FIXTURE_REPOSITORY.to_owned(),
            commit: "a".repeat(40),
            tree: "b".repeat(40),
            merged_pull_request: format!("{FIXTURE_REPOSITORY}#1"),
        };
        let (bundles, reader, role_policy) = valid_receipt_fixture(Status::Passed);
        let accepting = FixtureVerifier {
            rejected_bundles: BTreeSet::new(),
            invalid_bundles: BTreeSet::new(),
            mismatched_claims: false,
        };
        let wrong_subject = ExpectedReceiptSubject {
            commit: "c".repeat(40),
            ..subject.clone()
        };
        let context = verification_context(
            &accepting,
            &reader,
            &wrong_subject,
            &signer,
            &source,
            &role_policy,
        );
        assert_eq!(
            verify_receipt(&bundles, &context),
            Ok(ReceiptStatus::WrongSubject)
        );

        let changed_bundle = serde_json::to_string(&bundles.change_record).unwrap();
        let invalid_signature_verifier = FixtureVerifier {
            rejected_bundles: BTreeSet::new(),
            invalid_bundles: BTreeSet::from([changed_bundle]),
            mismatched_claims: false,
        };
        let context = verification_context(
            &invalid_signature_verifier,
            &reader,
            &subject,
            &signer,
            &source,
            &role_policy,
        );
        assert_eq!(
            verify_receipt(&bundles, &context),
            Ok(ReceiptStatus::SignatureInvalid)
        );

        let mut malformed = bundles.clone();
        let change_payload = extract_statement(&malformed.change_record).unwrap().payload;
        let mut change: Value = serde_json::from_slice(&change_payload).unwrap();
        change["predicate"]["unexpected"] = serde_json::json!(true);
        let change_payload = serde_json::to_vec(&change).unwrap();
        let changed_digest = canonical_statement_digest(&change_payload).unwrap();
        malformed.change_record = signed_bundle_payload(change_payload);
        let summary_payload = extract_statement(&malformed.summary).unwrap().payload;
        let mut summary: Value = serde_json::from_slice(&summary_payload).unwrap();
        summary["predicate"]["inputAttestations"][0]["digest"]["sha256"] =
            serde_json::json!(changed_digest);
        malformed.summary = signed_bundle_payload(serde_json::to_vec(&summary).unwrap());
        let context = verification_context(
            &accepting,
            &reader,
            &subject,
            &signer,
            &source,
            &role_policy,
        );
        assert_eq!(
            verify_receipt(&malformed, &context),
            Err(ReceiptVerificationError::PredicateInvalid)
        );
    }

    #[trace("TC-614", "FR-078-AC-4")]
    #[test]
    fn claimed_approval_without_a_retained_statement_is_a_mismatch() {
        let (bundles, _retained_reader, role_policy) = valid_receipt_fixture(Status::Passed);
        let verifier = FixtureVerifier {
            rejected_bundles: BTreeSet::new(),
            invalid_bundles: BTreeSet::new(),
            mismatched_claims: false,
        };
        let reader = EmptyApprovalReader;
        let subject = ExpectedReceiptSubject {
            repository: FIXTURE_REPOSITORY.to_owned(),
            commit: "a".repeat(40),
            tree: "b".repeat(40),
            merged_pull_request: format!("{FIXTURE_REPOSITORY}#1"),
        };
        let signer = WorkflowIdentityPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            references: BTreeSet::from(["refs/heads/main".to_owned()]),
            issuer: FIXTURE_RECEIPT_ISSUER.to_owned(),
            transparency: TransparencyRequirement::PublicLog,
        };
        let source = SourceWorkflowPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            event: "push".to_owned(),
            source_ref: Some("refs/heads/main".to_owned()),
            source_ref_prefix: None,
        };
        let context =
            verification_context(&verifier, &reader, &subject, &signer, &source, &role_policy);
        assert_eq!(
            verify_receipt(&bundles, &context),
            Ok(ReceiptStatus::ApprovalsMismatch)
        );
    }

    #[trace("TC-614", "FR-078-AC-4")]
    #[test]
    fn approval_rows_bind_each_required_role_to_one_verified_statement_digest() {
        let (valid, reader, role_policy) = valid_two_role_fixture();
        let verifier = FixtureVerifier {
            rejected_bundles: BTreeSet::new(),
            invalid_bundles: BTreeSet::new(),
            mismatched_claims: false,
        };
        let subject = ExpectedReceiptSubject {
            repository: FIXTURE_REPOSITORY.to_owned(),
            commit: "a".repeat(40),
            tree: "b".repeat(40),
            merged_pull_request: format!("{FIXTURE_REPOSITORY}#1"),
        };
        let signer = WorkflowIdentityPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            references: BTreeSet::from(["refs/heads/main".to_owned()]),
            issuer: FIXTURE_RECEIPT_ISSUER.to_owned(),
            transparency: TransparencyRequirement::PublicLog,
        };
        let source = fixture_source_policy();
        let context =
            verification_context(&verifier, &reader, &subject, &signer, &source, &role_policy);
        assert_eq!(
            verify_receipt(&valid, &context),
            Ok(ReceiptStatus::Verified)
        );

        let engineer_digest = canonical_statement_digest(&reader.0[0].statement_bytes).unwrap();
        let manager_digest = canonical_statement_digest(&reader.0[1].statement_bytes).unwrap();
        let mut swapped = valid.clone();
        replace_approval_rows(
            &mut swapped,
            json!([
                {"role": "engineer", "verdict": "covered", "statement_digest": manager_digest},
                {"role": "manager", "verdict": "covered", "statement_digest": engineer_digest}
            ]),
        );
        assert_eq!(
            verify_receipt(&swapped, &context),
            Ok(ReceiptStatus::ApprovalsMismatch),
            "a valid digest assigned to the wrong role must fail closed"
        );

        let mut duplicated = valid;
        replace_approval_rows(
            &mut duplicated,
            json!([
                {"role": "engineer", "verdict": "covered", "statement_digest": engineer_digest},
                {"role": "engineer", "verdict": "covered", "statement_digest": engineer_digest}
            ]),
        );
        assert_eq!(
            verify_receipt(&duplicated, &context),
            Ok(ReceiptStatus::ApprovalsMismatch),
            "duplicate role and digest rows must fail closed"
        );
    }

    #[trace("TC-614", "FR-078-AC-4")]
    #[test]
    fn empty_private_store_fails_closed_as_unverifiable() {
        let (mut bundles, _retained_reader, role_policy) = valid_receipt_fixture(Status::Passed);
        let change_payload = extract_statement(&bundles.change_record).unwrap().payload;
        let mut change: Value = serde_json::from_slice(&change_payload).unwrap();
        change["predicate"]["approvals"] = serde_json::json!([]);
        let change_payload = serde_json::to_vec(&change).unwrap();
        let change_digest = canonical_statement_digest(&change_payload).unwrap();
        bundles.change_record = signed_bundle_payload(change_payload);
        let summary_payload = extract_statement(&bundles.summary).unwrap().payload;
        let mut summary: Value = serde_json::from_slice(&summary_payload).unwrap();
        summary["predicate"]["inputAttestations"][0]["digest"]["sha256"] =
            serde_json::json!(change_digest);
        bundles.summary = signed_bundle_payload(serde_json::to_vec(&summary).unwrap());
        let verifier = FixtureVerifier {
            rejected_bundles: BTreeSet::new(),
            invalid_bundles: BTreeSet::new(),
            mismatched_claims: false,
        };
        let reader = EmptyApprovalReader;
        let subject = ExpectedReceiptSubject {
            repository: FIXTURE_REPOSITORY.to_owned(),
            commit: "a".repeat(40),
            tree: "b".repeat(40),
            merged_pull_request: format!("{FIXTURE_REPOSITORY}#1"),
        };
        let signer = WorkflowIdentityPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            references: BTreeSet::from(["refs/heads/main".to_owned()]),
            issuer: FIXTURE_RECEIPT_ISSUER.to_owned(),
            transparency: TransparencyRequirement::PublicLog,
        };
        let source = SourceWorkflowPolicy {
            repository: FIXTURE_REPOSITORY.to_owned(),
            workflow_path: ".github/workflows/change-receipt.yml".to_owned(),
            event: "push".to_owned(),
            source_ref: Some("refs/heads/main".to_owned()),
            source_ref_prefix: None,
        };
        let context =
            verification_context(&verifier, &reader, &subject, &signer, &source, &role_policy);
        assert_eq!(
            verify_receipt(&bundles, &context),
            Ok(ReceiptStatus::Unverifiable)
        );
    }

    #[trace("TC-598", "FR-076-AC-2")]
    #[test]
    fn oversized_receipt_note_is_refused() {
        let oversized = ReceiptBundleSet {
            change_record: json!({"payload": "x".repeat(MAX_RECEIPT_NOTE_BYTES)}),
            ..bundle_set()
        };
        assert_eq!(oversized.encode_note(), Err(ReceiptNoteError::TooLarge));
    }

    #[trace("TC-615", "FR-078-AC-5")]
    #[test]
    fn verify_result_requires_closed_reason_only_for_unverifiable() {
        let missing_reason = ReceiptVerifyResult {
            protocol: VERIFY_RESULT_PROTOCOL.to_owned(),
            status: ReceiptStatus::Unverifiable,
            reason: None,
        };
        assert!(!is_valid_verify_result(&missing_reason));
        let valid = ReceiptVerifyResult {
            reason: Some(UnverifiableReason::NoteAbsent),
            ..missing_reason
        };
        assert!(is_valid_verify_result(&valid));
        let missing_store = ReceiptVerifyResult {
            reason: Some(UnverifiableReason::StatementsNotRetained),
            ..valid.clone()
        };
        assert!(is_valid_verify_result(&missing_store));
        assert_eq!(
            serde_json::to_value(missing_store).unwrap()["reason"],
            "statements_not_retained"
        );
        let misplaced = ReceiptVerifyResult {
            status: ReceiptStatus::Verified,
            ..valid
        };
        assert!(!is_valid_verify_result(&misplaced));
    }

    #[trace("TC-608", "TC-610", "FR-075-AC-5", "FR-077-AC-5", "FR-077-AC-7")]
    #[test]
    fn missing_receipts_use_platform_push_time_and_first_parent_cutoff() {
        let history = [
            ReceiptHistoryEntry {
                commit: "a".repeat(40),
                pushed_at_unix: Some(1_000),
                receipt_run_in_progress: true,
                state: ReceiptCheckState::NoNote,
            },
            ReceiptHistoryEntry {
                commit: "c".repeat(40),
                pushed_at_unix: None,
                receipt_run_in_progress: false,
                state: ReceiptCheckState::Verified,
            },
            ReceiptHistoryEntry {
                commit: "b".repeat(40),
                pushed_at_unix: Some(100),
                receipt_run_in_progress: false,
                state: ReceiptCheckState::VerifiedFailed,
            },
            ReceiptHistoryEntry {
                commit: "d".repeat(40),
                pushed_at_unix: Some(10),
                receipt_run_in_progress: false,
                state: ReceiptCheckState::NoNote,
            },
        ];
        let findings = find_missing_receipts(&history, &"b".repeat(40), 30, 2_000)
            .expect("adopted commit is on first-parent history");
        assert_eq!(
            findings,
            vec![
                MissingReceipt {
                    commit: "c".repeat(40),
                    reason: MissingReceiptReason::StatusUnknown
                },
                MissingReceipt {
                    commit: "b".repeat(40),
                    reason: MissingReceiptReason::FailedReceipt
                },
            ]
        );
    }

    #[trace(
        "TC-604",
        "TC-605",
        "TC-606",
        "FR-077-AC-1",
        "FR-077-AC-2",
        "FR-077-AC-3"
    )]
    #[test]
    fn missing_receipt_report_distinguishes_absent_invalid_unknown_and_failed() {
        let history = [
            ReceiptHistoryEntry {
                commit: "a".repeat(40),
                pushed_at_unix: Some(1),
                receipt_run_in_progress: false,
                state: ReceiptCheckState::Verified,
            },
            ReceiptHistoryEntry {
                commit: "b".repeat(40),
                pushed_at_unix: Some(1),
                receipt_run_in_progress: false,
                state: ReceiptCheckState::NoNote,
            },
            ReceiptHistoryEntry {
                commit: "c".repeat(40),
                pushed_at_unix: Some(1),
                receipt_run_in_progress: false,
                state: ReceiptCheckState::InvalidReceipt,
            },
            ReceiptHistoryEntry {
                commit: "d".repeat(40),
                pushed_at_unix: Some(1),
                receipt_run_in_progress: false,
                state: ReceiptCheckState::StatusUnknown,
            },
            ReceiptHistoryEntry {
                commit: "e".repeat(40),
                pushed_at_unix: Some(1),
                receipt_run_in_progress: false,
                state: ReceiptCheckState::VerifiedFailed,
            },
        ];
        assert_eq!(
            find_missing_receipts(&history, &"e".repeat(40), 0, 10).unwrap(),
            vec![
                MissingReceipt {
                    commit: "b".repeat(40),
                    reason: MissingReceiptReason::NoNote,
                },
                MissingReceipt {
                    commit: "c".repeat(40),
                    reason: MissingReceiptReason::InvalidReceipt,
                },
                MissingReceipt {
                    commit: "d".repeat(40),
                    reason: MissingReceiptReason::StatusUnknown,
                },
                MissingReceipt {
                    commit: "e".repeat(40),
                    reason: MissingReceiptReason::FailedReceipt,
                },
            ]
        );
    }

    #[trace("TC-605", "FR-077-AC-2")]
    #[test]
    fn verified_failed_receipt_is_reported_even_within_the_grace_window() {
        let history = [
            ReceiptHistoryEntry {
                commit: "a".repeat(40),
                pushed_at_unix: Some(1_999),
                receipt_run_in_progress: true,
                state: ReceiptCheckState::VerifiedFailed,
            },
            ReceiptHistoryEntry {
                commit: "b".repeat(40),
                pushed_at_unix: Some(1_999),
                receipt_run_in_progress: true,
                state: ReceiptCheckState::NoNote,
            },
            ReceiptHistoryEntry {
                commit: "c".repeat(40),
                pushed_at_unix: None,
                receipt_run_in_progress: false,
                state: ReceiptCheckState::VerifiedFailed,
            },
        ];
        assert_eq!(
            find_missing_receipts(&history, &"c".repeat(40), 30, 2_000).unwrap(),
            vec![
                MissingReceipt {
                    commit: "a".repeat(40),
                    reason: MissingReceiptReason::FailedReceipt,
                },
                MissingReceipt {
                    commit: "c".repeat(40),
                    reason: MissingReceiptReason::FailedReceipt,
                },
            ]
        );
    }

    #[trace("TC-609", "FR-077-AC-6")]
    #[test]
    fn verified_first_parent_history_produces_an_empty_missing_list() {
        let history = [ReceiptHistoryEntry {
            commit: "a".repeat(40),
            pushed_at_unix: Some(1),
            receipt_run_in_progress: false,
            state: ReceiptCheckState::Verified,
        }];
        assert!(
            find_missing_receipts(&history, &history[0].commit, 0, 10)
                .unwrap()
                .is_empty()
        );
    }

    #[trace("TC-607", "FR-077-AC-4")]
    #[test]
    fn missing_receipt_check_refuses_side_branch_adoption_point() {
        let history = [ReceiptHistoryEntry {
            commit: "a".repeat(40),
            pushed_at_unix: Some(1),
            receipt_run_in_progress: false,
            state: ReceiptCheckState::Verified,
        }];
        assert_eq!(
            find_missing_receipts(&history, &"f".repeat(40), 0, 10),
            Err(MissingReceiptError::AdoptedAtInvalid)
        );
    }
}
