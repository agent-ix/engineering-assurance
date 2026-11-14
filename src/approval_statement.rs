// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Private-store lookup and verification for retained approval statements
//! (FR-064).
//!
//! This is a narrow query by repository, merged pull request, and merged
//! commit. It does not expose general record enumeration. The statement
//! writer supplies the retained bytes, signature bundle, and publish outcome
//! through the reader contract.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::record_sink::sink::ApprovalStatementReader;

use crate::{
    approval_gate::{self, AccountRef, AccountType, Approval, GateResult, RolePolicy},
    attestation_signing::{
        BundleVerifier, ExpectedSubject, StatementV1, SubjectKind, VerificationFailure,
        VerificationStatus, WorkflowIdentityPolicy, verify_statement,
    },
};

const APPROVAL_PREDICATE_TYPE: &str =
    concat!("https:", "//schemas.agent-ix.ai/attestation/approval/v1");

pub use crate::record_sink::sink::{
    ApprovalStatementLookup, ApprovalStatementPublishOutcome, ApprovalStatementReadError,
    RetainedApprovalStatement,
};

/// The aggregate status of the matching statement set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalStatementStatus {
    /// Every matching statement verified.
    Verified,
    /// At least one matching statement could not be verified or retained.
    Unverifiable,
    /// At least one statement's signature is invalid.
    SignatureInvalid,
    /// At least one statement has an untrusted signing workflow identity.
    UntrustedSigner,
    /// At least one statement does not bind the requested repository or merge.
    WrongSubject,
    /// At least one statement has an invalid approval predicate.
    PredicateInvalid,
}

/// Why an approval-statement lookup or publish outcome is unverifiable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApprovalStatementReason {
    /// The private-store read or another required verification input failed.
    InputUnreadable,
    /// A retained matching statement was refused by its publish sink.
    PublishRefused,
    /// The private store retained no matching approval statement.
    StatementsNotRetained,
}

/// Aggregate statement verification and role evaluation for one exact merge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalStatementResult {
    /// Aggregate status in FR-064-AC-20 precedence order.
    pub status: ApprovalStatementStatus,
    /// Present only when the aggregate status is `Unverifiable`.
    pub reason: Option<ApprovalStatementReason>,
    /// FR-061 evaluation over only the verified approval statements.
    pub role_verdict: GateResult,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StatementStatus {
    Verified,
    Unverifiable(ApprovalStatementReason),
    SignatureInvalid,
    UntrustedSigner,
    WrongSubject,
    PredicateInvalid,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalPredicate {
    source: String,
    approver: String,
    approver_name: String,
    role: String,
    meaning: String,
    signed_at: String,
    head_commit: String,
    pull_request: String,
    merged_commit: String,
}

/// Read all exact matches, verify each statement, and evaluate only the
/// verified approvals against the supplied FR-061 role policy.
#[must_use]
pub fn verify_approval_statements(
    reader: &dyn ApprovalStatementReader,
    destination: &str,
    lookup: ApprovalStatementLookup<'_>,
    identity_policy: &WorkflowIdentityPolicy,
    role_policy: &RolePolicy,
    verifier: &dyn BundleVerifier,
) -> ApprovalStatementResult {
    let statements = match reader.lookup_approval_statements(destination, lookup) {
        Ok(statements) => statements,
        Err(ApprovalStatementReadError) => {
            return result(
                &[StatementStatus::Unverifiable(
                    ApprovalStatementReason::InputUnreadable,
                )],
                &[],
                role_policy,
            );
        }
    };
    if statements.is_empty() {
        return result(
            &[StatementStatus::Unverifiable(
                ApprovalStatementReason::StatementsNotRetained,
            )],
            &[],
            role_policy,
        );
    }

    let mut statuses = Vec::with_capacity(statements.len());
    let mut approvals = Vec::with_capacity(statements.len());
    for retained in &statements {
        match retained.publish_outcome {
            ApprovalStatementPublishOutcome::Refused => {
                statuses.push(StatementStatus::Unverifiable(
                    ApprovalStatementReason::PublishRefused,
                ));
            }
            ApprovalStatementPublishOutcome::Published => {
                let status = verify_one(verifier, retained, lookup, identity_policy);
                statuses.push(status);
                if status == StatementStatus::Verified
                    && let Ok(approval) = approval_from_statement(&retained.statement_bytes, lookup)
                {
                    approvals.push(approval);
                }
            }
        }
    }
    result(&statuses, &approvals, role_policy)
}

fn verify_one(
    verifier: &dyn BundleVerifier,
    retained: &RetainedApprovalStatement,
    lookup: ApprovalStatementLookup<'_>,
    identity_policy: &WorkflowIdentityPolicy,
) -> StatementStatus {
    let verification = verify_statement(
        verifier,
        &retained.statement_bytes,
        &retained.signature_bundle,
        identity_policy,
        ExpectedSubject {
            kind: SubjectKind::Commit,
            value: lookup.merged_commit,
        },
    );
    match verification {
        Err(VerificationFailure::InvalidSignature) => return StatementStatus::SignatureInvalid,
        Err(VerificationFailure::UntrustedSigner) => return StatementStatus::UntrustedSigner,
        Err(VerificationFailure::WrongSubject) => return StatementStatus::WrongSubject,
        Err(VerificationFailure::InvalidStatement) => return StatementStatus::PredicateInvalid,
        Ok(VerificationStatus::Verified) => {}
    }

    match approval_from_statement(&retained.statement_bytes, lookup) {
        Ok(_) => StatementStatus::Verified,
        Err(VerificationFailure::WrongSubject) => StatementStatus::WrongSubject,
        Err(_) => StatementStatus::PredicateInvalid,
    }
}

fn approval_from_statement(
    statement_bytes: &[u8],
    lookup: ApprovalStatementLookup<'_>,
) -> Result<Approval, VerificationFailure> {
    let statement: StatementV1 = serde_json::from_slice(statement_bytes)
        .map_err(|_| VerificationFailure::InvalidStatement)?;
    if statement.type_ != crate::attestation_signing::STATEMENT_V1_TYPE
        || statement.predicate_type != APPROVAL_PREDICATE_TYPE
    {
        return Err(VerificationFailure::InvalidStatement);
    }
    let subject_matches = statement.subject.iter().any(|subject| {
        subject.name == lookup.repository
            && subject
                .digest
                .get("gitCommit")
                .is_some_and(|commit| commit == lookup.merged_commit)
    });
    if !subject_matches {
        return Err(VerificationFailure::WrongSubject);
    }
    let predicate_value = serde_json::Value::Object(statement.predicate.into_iter().collect());
    let predicate: ApprovalPredicate = serde_json::from_value(predicate_value)
        .map_err(|_| VerificationFailure::InvalidStatement)?;
    if predicate.pull_request != lookup.merged_pull_request
        || predicate.merged_commit != lookup.merged_commit
    {
        return Err(VerificationFailure::WrongSubject);
    }
    let (source, id) = predicate
        .approver
        .split_once(':')
        .ok_or(VerificationFailure::InvalidStatement)?;
    drop(predicate.approver_name);
    if source.is_empty() || id.is_empty() || predicate.source != source {
        return Err(VerificationFailure::InvalidStatement);
    }
    Ok(Approval {
        source: source.to_owned(),
        account: AccountRef {
            source: source.to_owned(),
            id: id.to_owned(),
        },
        human: true,
        role: predicate.role,
        meaning: predicate.meaning,
        timestamp: predicate.signed_at,
        commit: predicate.head_commit,
        pull_request: predicate.pull_request,
    })
}

fn result(
    statuses: &[StatementStatus],
    approvals: &[Approval],
    role_policy: &RolePolicy,
) -> ApprovalStatementResult {
    let mut account_types = BTreeMap::new();
    for account in role_policy.individuals.values().flatten() {
        account_types.insert(account.clone(), AccountType::Human);
    }
    let role_verdict = approval_gate::evaluate(role_policy, approvals, &account_types);
    let (status, reason) = aggregate_status(statuses);
    ApprovalStatementResult {
        status,
        reason,
        role_verdict,
    }
}

fn aggregate_status(
    statuses: &[StatementStatus],
) -> (ApprovalStatementStatus, Option<ApprovalStatementReason>) {
    if statuses.is_empty() {
        return (
            ApprovalStatementStatus::Unverifiable,
            Some(ApprovalStatementReason::StatementsNotRetained),
        );
    }
    if statuses
        .iter()
        .all(|status| *status == StatementStatus::Verified)
    {
        return (ApprovalStatementStatus::Verified, None);
    }
    for status in [
        ApprovalStatementStatus::Unverifiable,
        ApprovalStatementStatus::SignatureInvalid,
        ApprovalStatementStatus::UntrustedSigner,
        ApprovalStatementStatus::WrongSubject,
        ApprovalStatementStatus::PredicateInvalid,
    ] {
        if statuses
            .iter()
            .any(|candidate| status_of(*candidate) == status)
        {
            let reason = (status == ApprovalStatementStatus::Unverifiable)
                .then(|| first_unverifiable_reason(statuses))
                .flatten();
            return (status, reason);
        }
    }
    (
        ApprovalStatementStatus::Unverifiable,
        Some(ApprovalStatementReason::StatementsNotRetained),
    )
}

fn status_of(status: StatementStatus) -> ApprovalStatementStatus {
    match status {
        StatementStatus::Verified => ApprovalStatementStatus::Verified,
        StatementStatus::Unverifiable(_) => ApprovalStatementStatus::Unverifiable,
        StatementStatus::SignatureInvalid => ApprovalStatementStatus::SignatureInvalid,
        StatementStatus::UntrustedSigner => ApprovalStatementStatus::UntrustedSigner,
        StatementStatus::WrongSubject => ApprovalStatementStatus::WrongSubject,
        StatementStatus::PredicateInvalid => ApprovalStatementStatus::PredicateInvalid,
    }
}

fn first_unverifiable_reason(statuses: &[StatementStatus]) -> Option<ApprovalStatementReason> {
    [
        ApprovalStatementReason::InputUnreadable,
        ApprovalStatementReason::PublishRefused,
        ApprovalStatementReason::StatementsNotRetained,
    ]
    .into_iter()
    .find(|reason| statuses.contains(&StatementStatus::Unverifiable(*reason)))
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, collections::BTreeSet};

    use ix_trace_rs::trace;
    use serde_json::{Value, json};

    use super::*;
    use crate::attestation_signing::TransparencyRequirement;

    const LOOKUP: ApprovalStatementLookup<'static> = ApprovalStatementLookup {
        repository: "fictional/project",
        merged_pull_request: "fictional/project#12",
        merged_commit: "merge-commit",
    };

    struct Stored {
        repository: &'static str,
        pull_request: &'static str,
        commit: &'static str,
        statement: RetainedApprovalStatement,
    }

    #[derive(Default)]
    struct FixtureReader {
        entries: Vec<Stored>,
        lookups: RefCell<Vec<(String, String, String, String)>>,
        fail: bool,
    }

    impl ApprovalStatementReader for FixtureReader {
        fn lookup_approval_statements(
            &self,
            destination: &str,
            lookup: ApprovalStatementLookup<'_>,
        ) -> Result<Vec<RetainedApprovalStatement>, ApprovalStatementReadError> {
            self.lookups.borrow_mut().push((
                destination.to_owned(),
                lookup.repository.to_owned(),
                lookup.merged_pull_request.to_owned(),
                lookup.merged_commit.to_owned(),
            ));
            if self.fail {
                return Err(ApprovalStatementReadError);
            }
            Ok(self
                .entries
                .iter()
                .filter(|entry| {
                    entry.repository == lookup.repository
                        && entry.pull_request == lookup.merged_pull_request
                        && entry.commit == lookup.merged_commit
                })
                .map(|entry| entry.statement.clone())
                .collect())
        }
    }

    #[derive(Default)]
    struct FixtureVerifier {
        expected: Vec<(Vec<u8>, String, Result<(), VerificationFailure>)>,
        calls: RefCell<Vec<(Vec<u8>, String)>>,
    }

    impl BundleVerifier for FixtureVerifier {
        fn verify(
            &self,
            payload: &[u8],
            bundle_json: &str,
            _policy: &WorkflowIdentityPolicy,
        ) -> Result<(), VerificationFailure> {
            self.calls
                .borrow_mut()
                .push((payload.to_vec(), bundle_json.to_owned()));
            self.expected
                .iter()
                .find(|(expected_payload, expected_bundle, _)| {
                    expected_payload == payload && expected_bundle == bundle_json
                })
                .map_or(
                    Err(VerificationFailure::InvalidSignature),
                    |(_, _, result)| *result,
                )
        }
    }

    fn identity_policy() -> WorkflowIdentityPolicy {
        WorkflowIdentityPolicy {
            repository: "fictional/project".to_owned(),
            workflow_path: ".github/workflows/statement-writer.yml".to_owned(),
            references: BTreeSet::from(["refs/heads/main".to_owned()]),
            issuer: concat!("https:", "//token.actions.githubusercontent.com").to_owned(),
            transparency: TransparencyRequirement::PublicLog,
        }
    }

    fn role_policy() -> RolePolicy {
        RolePolicy {
            protocol: approval_gate::POLICY_PROTOCOL.to_owned(),
            required_roles: vec!["developer".to_owned()],
            individuals: BTreeMap::from([
                (
                    "alice".to_owned(),
                    vec![AccountRef {
                        source: "github".to_owned(),
                        id: "1".to_owned(),
                    }],
                ),
                (
                    "bob".to_owned(),
                    vec![AccountRef {
                        source: "github".to_owned(),
                        id: "2".to_owned(),
                    }],
                ),
            ]),
            roles: BTreeMap::from([(
                "developer".to_owned(),
                vec!["alice".to_owned(), "bob".to_owned()],
            )]),
            agents: Vec::new(),
        }
    }

    fn statement(account: &str, bundle: &str) -> RetainedApprovalStatement {
        statement_for_role(account, "developer", bundle)
    }

    fn statement_for_role(account: &str, role: &str, bundle: &str) -> RetainedApprovalStatement {
        let payload = json!({
            "_type": crate::attestation_signing::STATEMENT_V1_TYPE,
            "subject": [{
                "name": LOOKUP.repository,
                "digest": {"gitCommit": LOOKUP.merged_commit}
            }],
            "predicateType": APPROVAL_PREDICATE_TYPE,
            "predicate": {
                "source": "github",
                "approver": format!("github:{account}"),
                "approver_name": format!("reviewer-{account}"),
                "role": role,
                "meaning": "reviewed the change",
                "signed_at": "2026-11-13T10:00:00Z",
                "head_commit": "reviewed-head",
                "pull_request": LOOKUP.merged_pull_request,
                "merged_commit": LOOKUP.merged_commit
            }
        });
        RetainedApprovalStatement {
            statement_bytes: serde_json::to_vec(&payload).expect("fixture statement serializes"),
            signature_bundle: bundle.to_owned(),
            publish_outcome: ApprovalStatementPublishOutcome::Published,
        }
    }

    fn three_role_policy() -> RolePolicy {
        RolePolicy {
            protocol: approval_gate::POLICY_PROTOCOL.to_owned(),
            required_roles: vec![
                "developer".to_owned(),
                "engineer".to_owned(),
                "stakeholder".to_owned(),
            ],
            individuals: BTreeMap::from([
                (
                    "alice".to_owned(),
                    vec![AccountRef {
                        source: "github".to_owned(),
                        id: "1".to_owned(),
                    }],
                ),
                (
                    "bob".to_owned(),
                    vec![AccountRef {
                        source: "github".to_owned(),
                        id: "2".to_owned(),
                    }],
                ),
                (
                    "carol".to_owned(),
                    vec![AccountRef {
                        source: "github".to_owned(),
                        id: "3".to_owned(),
                    }],
                ),
            ]),
            roles: BTreeMap::from([
                ("developer".to_owned(), vec!["alice".to_owned()]),
                ("engineer".to_owned(), vec!["bob".to_owned()]),
                ("stakeholder".to_owned(), vec!["carol".to_owned()]),
            ]),
            agents: Vec::new(),
        }
    }

    fn reader_for(entries: Vec<(&'static str, RetainedApprovalStatement)>) -> FixtureReader {
        FixtureReader {
            entries: entries
                .into_iter()
                .map(|(pull_request, statement)| Stored {
                    repository: LOOKUP.repository,
                    pull_request,
                    commit: LOOKUP.merged_commit,
                    statement,
                })
                .collect(),
            ..FixtureReader::default()
        }
    }

    #[trace("TC-960", "FR-064-AC-1", "FR-064-AC-10")]
    #[test]
    fn verified_statements_satisfy_roles_and_untrusted_statement_names_its_role() {
        let roles = ["developer", "engineer", "stakeholder"];
        let entries = roles
            .iter()
            .enumerate()
            .map(|(index, role)| {
                let account = (index + 1).to_string();
                let bundle = format!("bundle-{role}");
                (
                    LOOKUP.merged_pull_request,
                    statement_for_role(&account, role, &bundle),
                )
            })
            .collect::<Vec<_>>();
        let expected = entries
            .iter()
            .map(|(_, retained)| {
                (
                    retained.statement_bytes.clone(),
                    retained.signature_bundle.clone(),
                    Ok(()),
                )
            })
            .collect();
        let verifier = FixtureVerifier {
            expected,
            ..FixtureVerifier::default()
        };
        let reader = reader_for(entries);

        let all_verified = verify_approval_statements(
            &reader,
            "private-store-a",
            LOOKUP,
            &identity_policy(),
            &three_role_policy(),
            &verifier,
        );

        assert_eq!(all_verified.status, ApprovalStatementStatus::Verified);
        assert_eq!(
            all_verified.role_verdict.status,
            approval_gate::GateStatus::Satisfied
        );
        assert_eq!(all_verified.role_verdict.assignments.len(), 3);

        let mixed_entries = roles
            .iter()
            .enumerate()
            .map(|(index, role)| {
                let account = (index + 1).to_string();
                let bundle = format!("mixed-{role}");
                (
                    LOOKUP.merged_pull_request,
                    statement_for_role(&account, role, &bundle),
                )
            })
            .collect::<Vec<_>>();
        let untrusted_payload = mixed_entries[2].1.statement_bytes.clone();
        let untrusted_bundle = mixed_entries[2].1.signature_bundle.clone();
        let mixed_verifier = FixtureVerifier {
            expected: mixed_entries
                .iter()
                .map(|(_, retained)| {
                    let result = if retained.signature_bundle == untrusted_bundle {
                        Err(VerificationFailure::UntrustedSigner)
                    } else {
                        Ok(())
                    };
                    (
                        retained.statement_bytes.clone(),
                        retained.signature_bundle.clone(),
                        result,
                    )
                })
                .collect(),
            ..FixtureVerifier::default()
        };
        let mixed_reader = reader_for(mixed_entries);
        let mixed = verify_approval_statements(
            &mixed_reader,
            "private-store-a",
            LOOKUP,
            &identity_policy(),
            &three_role_policy(),
            &mixed_verifier,
        );

        assert_eq!(
            mixed.status,
            ApprovalStatementStatus::UntrustedSigner,
            "one untrusted statement prevents an all-verified aggregate"
        );
        assert_eq!(
            mixed.role_verdict.status,
            approval_gate::GateStatus::Unsatisfied
        );
        assert_eq!(mixed.role_verdict.missing.len(), 1);
        assert_eq!(mixed.role_verdict.missing[0].role, "stakeholder");
        assert!(
            mixed_verifier
                .calls
                .borrow()
                .iter()
                .any(|(payload, _)| payload == &untrusted_payload)
        );
    }

    #[trace("TC-961", "FR-062-AC-4")]
    #[test]
    fn a_statement_naming_a_listed_agent_is_not_counted_for_a_role() {
        let mut policy = role_policy();
        policy.agents.push(AccountRef {
            source: "github".to_owned(),
            id: "9".to_owned(),
        });
        let retained = statement("9", "agent-bundle");
        let verifier = FixtureVerifier {
            expected: vec![(
                retained.statement_bytes.clone(),
                retained.signature_bundle.clone(),
                Ok(()),
            )],
            ..FixtureVerifier::default()
        };
        let reader = reader_for(vec![(LOOKUP.merged_pull_request, retained)]);

        let result = verify_approval_statements(
            &reader,
            "private-store-a",
            LOOKUP,
            &identity_policy(),
            &policy,
            &verifier,
        );

        assert_eq!(result.status, ApprovalStatementStatus::Verified);
        assert_eq!(
            result.role_verdict.status,
            approval_gate::GateStatus::Unsatisfied
        );
        assert_eq!(result.role_verdict.missing.len(), 1);
        assert_eq!(
            result.role_verdict.missing[0].reason,
            approval_gate::MissingReason::AgentOnly
        );
    }

    #[trace("TC-954", "FR-064-AC-16", "FR-064-AC-20")]
    #[test]
    fn exact_lookup_verifies_every_match_and_counts_only_verified_approvals() {
        let verified_statement = statement("1", "verified-bundle");
        let untrusted = statement("2", "untrusted-bundle");
        let expected_verified = verified_statement.statement_bytes.clone();
        let expected_untrusted = untrusted.statement_bytes.clone();
        let reader = reader_for(vec![
            (LOOKUP.merged_pull_request, untrusted.clone()),
            (LOOKUP.merged_pull_request, verified_statement.clone()),
            ("fictional/project#13", statement("2", "decoy-pull-request")),
        ]);
        let other_repository = statement("2", "decoy-repository");
        reader.entries.push(Stored {
            repository: "fictional/other",
            pull_request: LOOKUP.merged_pull_request,
            commit: LOOKUP.merged_commit,
            statement: other_repository.clone(),
        });
        let other_commit = statement("2", "decoy-commit");
        reader.entries.push(Stored {
            repository: LOOKUP.repository,
            pull_request: LOOKUP.merged_pull_request,
            commit: "another-commit",
            statement: other_commit.clone(),
        });
        let verifier = FixtureVerifier {
            expected: vec![
                (
                    expected_untrusted.clone(),
                    "untrusted-bundle".to_owned(),
                    Err(VerificationFailure::UntrustedSigner),
                ),
                (
                    expected_verified.clone(),
                    "verified-bundle".to_owned(),
                    Ok(()),
                ),
            ],
            ..FixtureVerifier::default()
        };

        let result = verify_approval_statements(
            &reader,
            "private-store-a",
            LOOKUP,
            &identity_policy(),
            &role_policy(),
            &verifier,
        );

        assert_eq!(
            reader.lookups.borrow().as_slice(),
            &[(
                "private-store-a".to_owned(),
                LOOKUP.repository.to_owned(),
                LOOKUP.merged_pull_request.to_owned(),
                LOOKUP.merged_commit.to_owned(),
            )],
            "the reader receives the exact repository, merge request and commit tuple"
        );
        assert_eq!(
            verifier.calls.borrow().as_slice(),
            &[
                (expected_untrusted, "untrusted-bundle".to_owned()),
                (expected_verified, "verified-bundle".to_owned()),
            ],
            "every retained match is verified using its exact bytes and associated bundle"
        );
        assert_eq!(result.status, ApprovalStatementStatus::UntrustedSigner);
        assert_eq!(result.reason, None);
        assert_eq!(
            result.role_verdict.status,
            approval_gate::GateStatus::Satisfied
        );
        assert_eq!(result.role_verdict.assignments[0].individual, "alice");
    }

    #[trace("TC-955", "FR-064-AC-17")]
    #[test]
    fn no_exact_retained_match_is_unverifiable_and_does_not_satisfy_a_role() {
        let reader = reader_for(vec![(
            "fictional/project#12",
            statement("1", "other-commit"),
        )]);
        let verifier = FixtureVerifier::default();
        let mut wrong_commit = reader;
        wrong_commit.entries[0].commit = "another-commit";

        let result = verify_approval_statements(
            &wrong_commit,
            "private-store-a",
            LOOKUP,
            &identity_policy(),
            &role_policy(),
            &verifier,
        );

        assert_eq!(result.status, ApprovalStatementStatus::Unverifiable);
        assert_eq!(
            result.reason,
            Some(ApprovalStatementReason::StatementsNotRetained)
        );
        assert_eq!(
            result.role_verdict.status,
            approval_gate::GateStatus::Unsatisfied
        );
        assert!(verifier.calls.borrow().is_empty());
    }

    #[trace("TC-956", "FR-064-AC-18")]
    #[test]
    fn private_store_read_failure_is_unverifiable() {
        let reader = FixtureReader {
            fail: true,
            ..FixtureReader::default()
        };
        let result = verify_approval_statements(
            &reader,
            "private-store-a",
            LOOKUP,
            &identity_policy(),
            &role_policy(),
            &FixtureVerifier::default(),
        );

        assert_eq!(result.status, ApprovalStatementStatus::Unverifiable);
        assert_eq!(
            result.reason,
            Some(ApprovalStatementReason::InputUnreadable)
        );
        assert_eq!(
            result.role_verdict.status,
            approval_gate::GateStatus::Unsatisfied
        );
    }

    #[trace("TC-957", "FR-064-AC-19")]
    #[test]
    fn retained_publish_refusal_is_not_verified_or_counted() {
        let mut refused = statement("1", "must-not-verify");
        refused.publish_outcome = ApprovalStatementPublishOutcome::Refused;
        let reader = reader_for(vec![(LOOKUP.merged_pull_request, refused)]);
        let verifier = FixtureVerifier::default();
        let result = verify_approval_statements(
            &reader,
            "private-store-a",
            LOOKUP,
            &identity_policy(),
            &role_policy(),
            &verifier,
        );

        assert_eq!(result.status, ApprovalStatementStatus::Unverifiable);
        assert_eq!(result.reason, Some(ApprovalStatementReason::PublishRefused));
        assert_eq!(
            result.role_verdict.status,
            approval_gate::GateStatus::Unsatisfied
        );
        assert!(verifier.calls.borrow().is_empty());
    }

    #[trace("TC-958", "FR-064-AC-20")]
    #[test]
    fn aggregate_uses_fixed_status_and_unverifiable_reason_precedence() {
        let statuses = [
            StatementStatus::Unverifiable(ApprovalStatementReason::PublishRefused),
            StatementStatus::Unverifiable(ApprovalStatementReason::InputUnreadable),
            StatementStatus::Unverifiable(ApprovalStatementReason::StatementsNotRetained),
            StatementStatus::UntrustedSigner,
            StatementStatus::Verified,
        ];
        assert_eq!(
            aggregate_status(&statuses),
            (
                ApprovalStatementStatus::Unverifiable,
                Some(ApprovalStatementReason::InputUnreadable)
            )
        );
        assert_eq!(
            aggregate_status(&[StatementStatus::Verified, StatementStatus::UntrustedSigner,]),
            (ApprovalStatementStatus::UntrustedSigner, None)
        );
        assert_eq!(
            aggregate_status(&[
                StatementStatus::Verified,
                StatementStatus::PredicateInvalid,
                StatementStatus::WrongSubject,
                StatementStatus::SignatureInvalid,
            ]),
            (ApprovalStatementStatus::SignatureInvalid, None)
        );
    }

    #[trace("TC-959", "FR-064-AC-5")]
    #[test]
    fn a_signed_approval_with_an_open_or_wrong_predicate_is_rejected() {
        for predicate in [
            json!({
                "source": "github",
                "approver": "github:1",
                "approver_name": "reviewer-1",
                "role": "developer",
                "meaning": "reviewed the change",
                "signed_at": "2026-11-13T10:00:00Z",
                "head_commit": "reviewed-head",
                "pull_request": LOOKUP.merged_pull_request,
                "merged_commit": LOOKUP.merged_commit,
                "unexpected": true
            }),
            json!({
                "source": "github",
                "approver": "github:1",
                "approver_name": "reviewer-1",
                "role": "developer",
                "meaning": "reviewed the change",
                "signed_at": "2026-11-13T10:00:00Z",
                "head_commit": "reviewed-head",
                "pull_request": LOOKUP.merged_pull_request,
                "merged_commit": LOOKUP.merged_commit
            }),
        ] {
            let mut payload: Value =
                serde_json::from_slice(&statement("1", "bundle").statement_bytes)
                    .expect("fixture parses");
            payload["predicate"] = predicate;
            if payload["predicate"]["unexpected"].is_null() {
                payload["predicateType"] = json!(concat!(
                    "https:",
                    "//schemas.agent-ix.ai/attestation/not-approval/v1"
                ));
            }
            let bytes = serde_json::to_vec(&payload).expect("changed fixture serializes");
            let reader = reader_for(vec![(
                LOOKUP.merged_pull_request,
                RetainedApprovalStatement {
                    statement_bytes: bytes.clone(),
                    signature_bundle: "bundle".to_owned(),
                    publish_outcome: ApprovalStatementPublishOutcome::Published,
                },
            )]);
            let verifier = FixtureVerifier {
                expected: vec![(bytes, "bundle".to_owned(), Ok(()))],
                ..FixtureVerifier::default()
            };
            let result = verify_approval_statements(
                &reader,
                "private-store-a",
                LOOKUP,
                &identity_policy(),
                &role_policy(),
                &verifier,
            );
            assert_eq!(result.status, ApprovalStatementStatus::PredicateInvalid);
            assert_eq!(
                result.role_verdict.status,
                approval_gate::GateStatus::Unsatisfied
            );
        }
    }
}
