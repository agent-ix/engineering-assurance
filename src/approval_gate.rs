// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Role-policy validation, GitHub review normalization, and approval-gate
//! evaluation (FR-059, FR-061, FR-062, FR-063 and FR-065).
//!
//! This module is deterministic and performs no network, filesystem, or
//! process I/O. The workflow supplies the policy from the protected base and
//! source snapshots from GitHub.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The protocol literal for a repository role policy.
pub const POLICY_PROTOCOL: &str = "engineering-assurance.approval-policy/v1";
/// The protocol literal for a role-gate result.
pub const RESULT_PROTOCOL: &str = "engineering-assurance.approval-gate/v1";
/// JSON Schema for the protected-base role-policy document.
pub const POLICY_SCHEMA: &str =
    include_str!("approval_gate_schemas/approval-policy-v1.schema.json");
/// JSON Schema for the result of evaluating a role policy.
pub const RESULT_SCHEMA: &str =
    include_str!("approval_gate_schemas/approval-gate-result-v1.schema.json");
/// JSON Schema for one normalized review-gate request.
pub const REQUEST_SCHEMA: &str =
    include_str!("approval_gate_schemas/approval-gate-request-v1.schema.json");

/// An account identity supplied by an approval source.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AccountRef {
    /// The source namespace, such as \`github\`.
    pub source: String,
    /// A source-stable account identifier; a login is not a stable identifier.
    pub id: String,
}

/// One repository role policy read from the protected base revision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RolePolicy {
    /// Protocol version for this document.
    pub protocol: String,
    /// Roles that must be covered for the gate to pass.
    pub required_roles: Vec<String>,
    /// Individuals and the stable source accounts they control.
    pub individuals: BTreeMap<String, Vec<AccountRef>>,
    /// Individuals eligible to fill each role.
    pub roles: BTreeMap<String, Vec<String>>,
    /// Source accounts known to be agents; defaults to the empty list.
    #[serde(default)]
    pub agents: Vec<AccountRef>,
}

/// A GitHub review state relevant to whether an earlier approval remains active.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewState {
    /// An unfinished review that has not been submitted.
    Pending,
    /// A submitted approval.
    Approved,
    /// A submitted request for changes, which removes an earlier approval.
    ChangesRequested,
    /// A dismissed review, which removes an earlier approval.
    Dismissed,
    /// A comment, which does not change approval state.
    Commented,
}

/// A normalized review snapshot from one source.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewSnapshot {
    /// Source review identifier, used as a stable tie-breaker.
    pub review_id: u64,
    /// Review author's stable identity.
    pub account: AccountRef,
    /// Source account classification. Unknown accounts are never human.
    pub account_type: AccountType,
    /// Review state.
    pub state: ReviewState,
    /// Review body supplied by the source adapter; used only when unedited.
    pub submitted_body: String,
    /// Whether the source reports that the review body was edited after submission.
    /// Edited bodies cannot supply role markers; a reviewer must submit a fresh review.
    pub body_edited: bool,
    /// Source timestamp in RFC 3339 form.
    pub submitted_at: String,
    /// Commit head the review was submitted against.
    pub commit: String,
    /// Pull request reference, such as \`owner/repository#12\`.
    pub pull_request: String,
}

/// Source-reported account type. Missing/unrecognized types do not count as human.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountType {
    /// A source account reported as a human user.
    Human,
    /// A source account reported as a bot or application.
    BotOrApp,
    /// The source reported no usable type.
    Unknown,
    /// The source could not classify this account.
    Unavailable,
}

/// A source-neutral approval extracted from a submitted review.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Approval {
    /// Identity of the approval source.
    pub source: String,
    /// Stable source account identity.
    pub account: AccountRef,
    /// Whether the source and policy identify this account as human.
    pub human: bool,
    /// Role named in the review.
    pub role: String,
    /// Meaning supplied in the submitted review.
    pub meaning: String,
    /// Review submission time.
    pub timestamp: String,
    /// Commit head the review was submitted against.
    pub commit: String,
    /// Pull request reference.
    pub pull_request: String,
}

/// A role that was not assigned in a maximum matching.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MissingRole {
    /// Role name.
    pub role: String,
    /// Why no distinct eligible human was assigned.
    pub reason: MissingReason,
}

/// Why a required role was not covered.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingReason {
    /// No human approval named this role.
    NoApproval,
    /// Human approvals exist, but their individuals are assigned elsewhere.
    SharedIndividual,
    /// Only non-human approvals named this role.
    AgentOnly,
}

/// One role and the individual assigned to it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Assignment {
    /// Required role.
    pub role: String,
    /// Distinct policy individual assigned to this role.
    pub individual: String,
    /// Stable account identity used for the approval.
    pub account: AccountRef,
}

/// Gate status.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GateStatus {
    /// Every required role is assigned to a distinct eligible human.
    Satisfied,
    /// A policy or approval condition is unsatisfied.
    Unsatisfied,
    /// The source could not provide enough trustworthy data to decide.
    Unavailable,
}

/// The closed result of evaluating one pull request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GateResult {
    /// Result protocol.
    pub protocol: &'static str,
    /// Gate status.
    pub status: GateStatus,
    /// Roles that the policy requires.
    pub required_roles: Vec<String>,
    /// Deterministic maximum matching.
    pub assignments: Vec<Assignment>,
    /// Roles left uncovered by that matching.
    pub missing: Vec<MissingRole>,
    /// Stable error code, present when policy or source data is invalid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<GateErrorCode>,
    /// Accounts whose type lookup failed, if the source was unavailable.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unavailable_accounts: Vec<AccountRef>,
}

/// Stable failures returned by role-gate evaluation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GateErrorCode {
    /// The role-policy document is structurally or semantically invalid.
    ApprovalPolicyInvalid,
    /// A required role has no eligible human account.
    ApprovalRoleUnstaffed,
    /// The approval source or an account-type lookup is unavailable.
    ApprovalSourceUnavailable,
}

/// Failure to interpret one policy document.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum PolicyError {
    /// The policy input is not readable JSON.
    #[error("approval policy is not readable JSON")]
    Json,
    /// The schema bundled with this feature could not be compiled.
    #[error("approval policy schema is invalid")]
    Schema,
    /// A JSON value does not satisfy the closed policy schema.
    #[error("approval policy does not match its schema")]
    SchemaMismatch,
    /// The protocol member is not supported.
    #[error("approval policy protocol is invalid")]
    Protocol,
    /// A name does not match the closed role or individual grammar.
    #[error("approval policy contains an invalid name")]
    Name,
    /// The required-role set is empty or duplicated.
    #[error("required_roles must be non-empty and unique")]
    RequiredRoles,
    /// A required role has no role-map entry.
    #[error("required role has no roles entry")]
    MissingRoleEntry,
    /// A role-map entry refers to an undeclared individual.
    #[error("role refers to an undeclared individual")]
    UnknownIndividual,
    /// An account is duplicated or belongs to multiple individuals.
    #[error("an account is duplicated across policy entries")]
    DuplicateAccount,
    /// An account is both a person and an agent.
    #[error("an account cannot be both an individual and an agent")]
    AgentIsIndividual,
    /// A source or stable id is empty.
    #[error("account source and id must be non-empty")]
    Account,
}

/// An adapter that normalizes one approval source without changing the role
/// evaluator or the approval-statement predicate.
pub trait ApprovalSource {
    /// Source-specific review representation accepted by this adapter.
    type Review;

    /// Source identifier placed in normalized approvals.
    fn source(&self) -> &'static str;

    /// Normalize source review snapshots for a pull request head.
    fn approvals(
        &self,
        policy: &RolePolicy,
        reviews: &[Self::Review],
        current_head: &str,
    ) -> Vec<Approval>;
}

/// GitHub review adapter for source-neutral approvals.
#[derive(Clone, Copy, Debug, Default)]
pub struct GitHubReviewAdapter;

impl ApprovalSource for GitHubReviewAdapter {
    type Review = ReviewSnapshot;

    fn source(&self) -> &'static str {
        "github"
    }

    fn approvals(
        &self,
        policy: &RolePolicy,
        reviews: &[ReviewSnapshot],
        current_head: &str,
    ) -> Vec<Approval> {
        github_approvals(policy, reviews, current_head)
    }
}

/// Parse, schema-check, and semantically validate a policy document.
pub fn parse_policy(bytes: &[u8]) -> Result<RolePolicy, PolicyError> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|_| PolicyError::Json)?;
    let schema: serde_json::Value =
        serde_json::from_str(POLICY_SCHEMA).map_err(|_| PolicyError::Schema)?;
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&schema)
        .map_err(|_| PolicyError::Schema)?;
    if !validator.is_valid(&value) {
        return Err(PolicyError::SchemaMismatch);
    }
    let policy: RolePolicy =
        serde_json::from_value(value).map_err(|_| PolicyError::SchemaMismatch)?;
    validate_policy(&policy)?;
    Ok(policy)
}

/// Validate policy semantics after closed JSON decoding.
pub fn validate_policy(policy: &RolePolicy) -> Result<(), PolicyError> {
    if policy.protocol != POLICY_PROTOCOL {
        return Err(PolicyError::Protocol);
    }
    let mut required = BTreeSet::new();
    if policy.required_roles.is_empty() {
        return Err(PolicyError::RequiredRoles);
    }
    for role in &policy.required_roles {
        if !valid_name(role) {
            return Err(PolicyError::Name);
        }
        if !required.insert(role.as_str()) {
            return Err(PolicyError::RequiredRoles);
        }
        if !policy.roles.contains_key(role) {
            return Err(PolicyError::MissingRoleEntry);
        }
    }
    for (individual, accounts) in &policy.individuals {
        if !valid_name(individual) {
            return Err(PolicyError::Name);
        }
        if accounts.is_empty() {
            return Err(PolicyError::Account);
        }
    }
    for (role, individuals) in &policy.roles {
        if !valid_name(role)
            || individuals
                .iter()
                .any(|individual| !policy.individuals.contains_key(individual))
        {
            return Err(PolicyError::UnknownIndividual);
        }
        let mut seen = BTreeSet::new();
        if individuals
            .iter()
            .any(|individual| !seen.insert(individual))
        {
            return Err(PolicyError::UnknownIndividual);
        }
    }
    let people: BTreeSet<_> = policy.individuals.values().flatten().collect();
    if policy.agents.iter().any(|agent| people.contains(agent)) {
        return Err(PolicyError::AgentIsIndividual);
    }
    let mut accounts = BTreeSet::new();
    for account in policy.individuals.values().flatten().chain(&policy.agents) {
        if account.source.is_empty()
            || account.id.is_empty()
            || (account.source == "github"
                && (account.id.parse::<u64>().is_err()
                    || !account.id.bytes().all(|byte| byte.is_ascii_digit())))
        {
            return Err(PolicyError::Account);
        }
        if !accounts.insert(account) {
            return Err(PolicyError::DuplicateAccount);
        }
    }
    Ok(())
}

/// Convert GitHub review snapshots into source-neutral approvals.
///
/// Only the latest non-comment review for each reviewer decides. Edited review
/// bodies and reviews on a stale commit are not approvals. A later COMMENTED
/// review does not change the prior decision.
#[must_use]
pub fn github_approvals(
    policy: &RolePolicy,
    reviews: &[ReviewSnapshot],
    current_head: &str,
) -> Vec<Approval> {
    let agents: BTreeSet<_> = policy.agents.iter().collect();
    let mut latest: BTreeMap<&AccountRef, &ReviewSnapshot> = BTreeMap::new();
    for review in reviews {
        if review.account.source != "github"
            || matches!(review.state, ReviewState::Pending | ReviewState::Commented)
        {
            continue;
        }
        let replace = latest.get(&review.account).is_none_or(|previous| {
            (review.submitted_at.as_str(), review.review_id)
                > (previous.submitted_at.as_str(), previous.review_id)
        });
        if replace {
            latest.insert(&review.account, review);
        }
    }
    let mut approvals = Vec::new();
    for review in latest.values() {
        if review.state != ReviewState::Approved
            || review.body_edited
            || review.commit != current_head
        {
            continue;
        }
        let Some((role, meaning)) = parse_review_markers(&review.submitted_body) else {
            continue;
        };
        let eligible_role = policy.roles.get(&role).is_some_and(|individuals| {
            individuals.iter().any(|individual| {
                policy
                    .individuals
                    .get(individual)
                    .is_some_and(|accounts| accounts.contains(&review.account))
            })
        });
        if !eligible_role {
            continue;
        }
        let human = review.account_type == AccountType::Human && !agents.contains(&review.account);
        if !human {
            continue;
        }
        approvals.push(Approval {
            source: "github".to_owned(),
            account: review.account.clone(),
            human,
            role,
            meaning,
            timestamp: review.submitted_at.clone(),
            commit: review.commit.clone(),
            pull_request: review.pull_request.clone(),
        });
    }
    approvals.sort_by(|left, right| {
        (&left.role, &left.account, &left.timestamp).cmp(&(
            &right.role,
            &right.account,
            &right.timestamp,
        ))
    });
    approvals
}

/// Evaluate a policy against source-neutral approvals and account lookups.
///
/// \`account_types\` contains a classification for every policy account. An
/// \`Unavailable\` entry makes the decision unavailable rather than guessing.
#[must_use]
pub fn evaluate(
    policy: &RolePolicy,
    approvals: &[Approval],
    account_types: &BTreeMap<AccountRef, AccountType>,
) -> GateResult {
    let mut roles = policy.required_roles.clone();
    roles.sort();
    let Ok(()) = validate_policy(policy) else {
        return result(
            GateStatus::Unsatisfied,
            roles,
            Vec::new(),
            Vec::new(),
            Some(GateErrorCode::ApprovalPolicyInvalid),
            Vec::new(),
        );
    };
    let agents: BTreeSet<_> = policy.agents.iter().collect();
    let people: BTreeMap<_, _> = policy
        .individuals
        .iter()
        .flat_map(|(name, accounts)| accounts.iter().map(move |account| (account, name)))
        .collect();
    let unavailable: Vec<_> = policy
        .individuals
        .values()
        .flatten()
        .filter(|account| {
            matches!(
                account_types.get(*account),
                None | Some(AccountType::Unavailable)
            )
        })
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if !unavailable.is_empty() {
        return result(
            GateStatus::Unavailable,
            roles,
            Vec::new(),
            Vec::new(),
            Some(GateErrorCode::ApprovalSourceUnavailable),
            unavailable,
        );
    }
    let staffed: BTreeMap<_, _> = roles
        .iter()
        .map(|role| {
            let eligible = policy
                .roles
                .get(role)
                .into_iter()
                .flatten()
                .filter(|individual| {
                    policy.individuals.get(*individual).is_some_and(|accounts| {
                        accounts.iter().any(|account| {
                            !agents.contains(account)
                                && account_types.get(account) == Some(&AccountType::Human)
                        })
                    })
                })
                .cloned()
                .collect::<BTreeSet<_>>();
            (role.clone(), eligible)
        })
        .collect();
    let unstaffed: Vec<_> = roles
        .iter()
        .filter(|role| staffed.get(*role).is_none_or(BTreeSet::is_empty))
        .cloned()
        .collect();
    if !unstaffed.is_empty() {
        return result(
            GateStatus::Unsatisfied,
            roles,
            Vec::new(),
            unstaffed
                .into_iter()
                .map(|role| MissingRole {
                    role,
                    reason: MissingReason::NoApproval,
                })
                .collect(),
            Some(GateErrorCode::ApprovalRoleUnstaffed),
            Vec::new(),
        );
    }
    let mut candidates: BTreeMap<String, BTreeSet<String>> = roles
        .iter()
        .map(|role| (role.clone(), BTreeSet::new()))
        .collect();
    let mut nonhuman_roles = BTreeSet::new();
    for approval in approvals {
        if !roles.contains(&approval.role) || approval.source != approval.account.source {
            continue;
        }
        if !approval.human
            || agents.contains(&approval.account)
            || account_types.get(&approval.account) != Some(&AccountType::Human)
        {
            nonhuman_roles.insert(approval.role.clone());
            continue;
        }
        let Some(individual) = people.get(&approval.account) else {
            continue;
        };
        if staffed
            .get(&approval.role)
            .is_some_and(|eligible| eligible.contains(*individual))
        {
            candidates
                .entry(approval.role.clone())
                .or_default()
                .insert((*individual).clone());
        }
    }
    let (assigned, covered_roles) = lexicographic_maximum_matching(&roles, &candidates);
    let mut missing = Vec::new();
    for role in &roles {
        if covered_roles.contains(role) {
            continue;
        }
        let reason = if nonhuman_roles.contains(role)
            && candidates.get(role).is_none_or(BTreeSet::is_empty)
        {
            MissingReason::AgentOnly
        } else if candidates
            .get(role)
            .is_some_and(|individuals| !individuals.is_empty())
        {
            MissingReason::SharedIndividual
        } else {
            MissingReason::NoApproval
        };
        missing.push(MissingRole {
            role: role.clone(),
            reason,
        });
    }
    let assignments = assigned
        .into_iter()
        .filter_map(|(role, individual)| {
            let approval = approvals.iter().find(|approval| {
                approval.human
                    && approval.role == role
                    && people.get(&approval.account) == Some(&&individual)
                    && account_types.get(&approval.account) == Some(&AccountType::Human)
            })?;
            Some(Assignment {
                role,
                individual,
                account: approval.account.clone(),
            })
        })
        .collect::<Vec<_>>();
    let status = if missing.is_empty() {
        GateStatus::Satisfied
    } else {
        GateStatus::Unsatisfied
    };
    result(status, roles, assignments, missing, None, Vec::new())
}

/// Parse the exact review marker lines required by FR-059.
#[must_use]
pub fn parse_review_markers(body: &str) -> Option<(String, String)> {
    let mut meanings = Vec::new();
    let mut roles = Vec::new();
    for line in body.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if let Some(value) = line.strip_prefix("Signed-off-meaning:") {
            meanings.push(value.strip_prefix(' ').unwrap_or(value).trim_matches(' '));
        }
        if let Some(value) = line.strip_prefix("Role:") {
            roles.push(value.strip_prefix(' ').unwrap_or(value).trim_matches(' '));
        }
    }
    let [meaning] = meanings.as_slice() else {
        return None;
    };
    let [role] = roles.as_slice() else {
        return None;
    };
    if meaning.is_empty()
        || meaning.len() > 256
        || meaning.chars().any(char::is_control)
        || !valid_name(role)
    {
        return None;
    }
    Some(((*role).to_owned(), (*meaning).to_owned()))
}

fn valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    (1..=32).contains(&bytes.len())
        && (bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit())
        && bytes.iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

fn lexicographic_maximum_matching(
    roles: &[String],
    candidates: &BTreeMap<String, BTreeSet<String>>,
) -> (Vec<(String, String)>, BTreeSet<String>) {
    let target = maximum_matching_size(roles, candidates, &BTreeSet::new());
    let mut assigned = Vec::new();
    let mut used = BTreeSet::new();
    let mut remaining = target;
    for (index, role) in roles.iter().enumerate() {
        let rest = &roles[index + 1..];
        let options = candidates.get(role).into_iter().flatten();
        let mut selected = None;
        for individual in options {
            if used.contains(individual) {
                continue;
            }
            let mut excluded = used.clone();
            excluded.insert(individual.clone());
            if remaining > 0 && maximum_matching_size(rest, candidates, &excluded) >= remaining - 1
            {
                selected = Some(individual.clone());
                break;
            }
        }
        if let Some(individual) = selected {
            used.insert(individual.clone());
            assigned.push((role.clone(), individual));
            remaining -= 1;
        }
    }
    let covered = assigned.iter().map(|(role, _)| role.clone()).collect();
    (assigned, covered)
}

fn maximum_matching_size(
    roles: &[String],
    candidates: &BTreeMap<String, BTreeSet<String>>,
    unavailable: &BTreeSet<String>,
) -> usize {
    fn assign(
        role: &str,
        candidates: &BTreeMap<String, BTreeSet<String>>,
        unavailable: &BTreeSet<String>,
        individual_roles: &mut BTreeMap<String, String>,
        seen: &mut BTreeSet<String>,
    ) -> bool {
        for individual in candidates.get(role).into_iter().flatten() {
            if unavailable.contains(individual) || !seen.insert(individual.clone()) {
                continue;
            }
            let previous = individual_roles.get(individual).cloned();
            if previous.as_deref().is_none_or(|previous_role| {
                assign(
                    previous_role,
                    candidates,
                    unavailable,
                    individual_roles,
                    seen,
                )
            }) {
                individual_roles.insert(individual.clone(), role.to_owned());
                return true;
            }
        }
        false
    }
    let mut individual_roles = BTreeMap::new();
    let mut count = 0;
    for role in roles {
        if assign(
            role,
            candidates,
            unavailable,
            &mut individual_roles,
            &mut BTreeSet::new(),
        ) {
            count += 1;
        }
    }
    count
}

fn result(
    status: GateStatus,
    required_roles: Vec<String>,
    assignments: Vec<Assignment>,
    missing: Vec<MissingRole>,
    error: Option<GateErrorCode>,
    unavailable_accounts: Vec<AccountRef>,
) -> GateResult {
    GateResult {
        protocol: RESULT_PROTOCOL,
        status,
        required_roles,
        assignments,
        missing,
        error,
        unavailable_accounts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    struct TestIdentityReview {
        account: AccountRef,
        role: String,
        meaning: String,
        commit: String,
        timestamp: String,
        pull_request: String,
    }

    struct TestIdentityFixtureAdapter;

    impl ApprovalSource for TestIdentityFixtureAdapter {
        type Review = TestIdentityReview;

        fn source(&self) -> &'static str {
            "test-identity-fixture"
        }

        fn approvals(
            &self,
            policy: &RolePolicy,
            reviews: &[Self::Review],
            current_head: &str,
        ) -> Vec<Approval> {
            reviews
                .iter()
                .filter(|review| {
                    review.commit == current_head
                        && policy.roles.get(&review.role).is_some_and(|individuals| {
                            individuals.iter().any(|individual| {
                                policy
                                    .individuals
                                    .get(individual)
                                    .is_some_and(|accounts| accounts.contains(&review.account))
                            })
                        })
                })
                .map(|review| Approval {
                    source: self.source().to_owned(),
                    account: review.account.clone(),
                    human: true,
                    role: review.role.clone(),
                    meaning: review.meaning.clone(),
                    timestamp: review.timestamp.clone(),
                    commit: review.commit.clone(),
                    pull_request: review.pull_request.clone(),
                })
                .collect()
        }
    }

    fn account(id: &str) -> AccountRef {
        AccountRef {
            source: "github".to_owned(),
            id: id.to_owned(),
        }
    }

    fn policy() -> RolePolicy {
        RolePolicy {
            protocol: POLICY_PROTOCOL.to_owned(),
            required_roles: vec!["engineer".to_owned(), "developer".to_owned()],
            individuals: BTreeMap::from([
                ("alice".to_owned(), vec![account("101")]),
                ("bob".to_owned(), vec![account("202")]),
            ]),
            roles: BTreeMap::from([
                ("developer".to_owned(), vec!["alice".to_owned()]),
                ("engineer".to_owned(), vec!["bob".to_owned()]),
            ]),
            agents: vec![account("303")],
        }
    }

    fn review(
        review_id: u64,
        id: &str,
        state: ReviewState,
        body: &str,
        commit: &str,
    ) -> ReviewSnapshot {
        ReviewSnapshot {
            review_id,
            account: account(id),
            account_type: if id == "303" {
                AccountType::BotOrApp
            } else {
                AccountType::Human
            },
            state,
            submitted_body: body.to_owned(),
            body_edited: false,
            submitted_at: format!("2026-11-14T03:00:{review_id:02}Z"),
            commit: commit.to_owned(),
            pull_request: "agent-ix/ea-109-sandbox#1".to_owned(),
        }
    }

    fn account_types() -> BTreeMap<AccountRef, AccountType> {
        BTreeMap::from([
            (account("101"), AccountType::Human),
            (account("202"), AccountType::Human),
            (account("303"), AccountType::BotOrApp),
        ])
    }

    #[test]
    #[trace("FR-059-AC-1", "FR-059-AC-3")]
    fn parses_one_trimmed_marker_pair_and_crlf() {
        assert_eq!(
            parse_review_markers(
                "Approved\r\nSigned-off-meaning:  Reviewed\r\nRole: developer\r\n"
            ),
            Some(("developer".to_owned(), "Reviewed".to_owned()))
        );
    }

    #[test]
    #[trace("FR-059-AC-3", "FR-059-AC-4")]
    fn refuses_missing_repeated_malformed_and_controlled_marker_values() {
        for body in [
            "Role: developer",
            "Signed-off-meaning: yes",
            "Signed-off-meaning: \nRole: developer",
            "Signed-off-meaning: a\nSigned-off-meaning: b\nRole: developer",
            "Signed-off-meaning: yes\nRole: developer\nRole: developer",
            "Signed-off-meaning: yes\nRole: Developer",
            "Signed-off-meaning: bad\u{0007}\nRole: developer",
        ] {
            assert_eq!(
                parse_review_markers(body),
                None,
                "accepted malformed body {body:?}"
            );
        }
        assert_eq!(
            parse_review_markers(&format!(
                "Signed-off-meaning: {}\nRole: developer",
                "x".repeat(257)
            )),
            None
        );
    }

    #[test]
    #[trace("FR-059-AC-2", "FR-059-AC-5", "FR-062-AC-1", "FR-062-AC-2")]
    fn only_latest_valid_same_head_human_review_counts_and_agents_do_not() {
        let policy = policy();
        let approvals = github_approvals(
            &policy,
            &[
                review(
                    1,
                    "101",
                    ReviewState::Approved,
                    "Signed-off-meaning: yes\nRole: developer",
                    "head",
                ),
                review(
                    2,
                    "101",
                    ReviewState::Commented,
                    "badly changed later",
                    "head",
                ),
                review(
                    3,
                    "202",
                    ReviewState::Approved,
                    "Signed-off-meaning: yes\nRole: engineer",
                    "old-head",
                ),
                review(
                    4,
                    "303",
                    ReviewState::Approved,
                    "Signed-off-meaning: yes\nRole: developer",
                    "head",
                ),
            ],
            "head",
        );
        assert_eq!(approvals.len(), 1);
        assert!(
            approvals
                .iter()
                .any(|approval| approval.account.id == "101" && approval.human)
        );
        assert!(
            !approvals
                .iter()
                .any(|approval| approval.account.id == "303")
        );
        assert!(
            !approvals
                .iter()
                .any(|approval| approval.account.id == "202")
        );
    }

    #[test]
    #[trace("FR-062-AC-2", "FR-059-AC-7")]
    fn policy_agent_and_ineligible_role_reviews_are_not_approvals() {
        let policy = policy();
        let mut listed_agent = review(
            1,
            "303",
            ReviewState::Approved,
            "Signed-off-meaning: yes\nRole: developer",
            "head",
        );
        listed_agent.account_type = AccountType::Human;
        let ineligible_role = review(
            2,
            "101",
            ReviewState::Approved,
            "Signed-off-meaning: yes\nRole: engineer",
            "head",
        );
        let unknown_individual = review(
            3,
            "404",
            ReviewState::Approved,
            "Signed-off-meaning: yes\nRole: developer",
            "head",
        );
        assert!(github_approvals(&policy, &[listed_agent], "head").is_empty());
        assert!(github_approvals(&policy, &[ineligible_role], "head").is_empty());
        assert!(github_approvals(&policy, &[unknown_individual], "head").is_empty());
    }

    #[test]
    #[trace("FR-062-AC-3")]
    fn unknown_review_account_type_is_not_human() {
        let mut unknown = review(
            1,
            "101",
            ReviewState::Approved,
            "Signed-off-meaning: yes\nRole: developer",
            "head",
        );
        unknown.account_type = AccountType::Unknown;
        assert!(github_approvals(&policy(), &[unknown], "head").is_empty());
    }

    #[test]
    #[trace("FR-059-AC-5")]
    fn latest_decision_revokes_and_later_fresh_approval_restores() {
        let policy = policy();
        let approval = review(
            1,
            "101",
            ReviewState::Approved,
            "Signed-off-meaning: yes\nRole: developer",
            "head",
        );
        let changes = review(
            2,
            "101",
            ReviewState::ChangesRequested,
            "Signed-off-meaning: yes\nRole: developer",
            "head",
        );
        let dismissed = review(
            3,
            "101",
            ReviewState::Dismissed,
            "Signed-off-meaning: yes\nRole: developer",
            "head",
        );
        let commented = review(
            4,
            "101",
            ReviewState::Commented,
            "not a submitted decision",
            "head",
        );
        let fresh = review(
            5,
            "101",
            ReviewState::Approved,
            "Signed-off-meaning: renewed\nRole: developer",
            "head",
        );

        assert!(github_approvals(&policy, &[approval.clone(), changes.clone()], "head").is_empty());
        assert!(github_approvals(&policy, &[approval.clone(), dismissed], "head").is_empty());
        assert_eq!(
            github_approvals(&policy, &[approval.clone(), commented], "head").len(),
            1
        );
        assert_eq!(
            github_approvals(&policy, &[changes, fresh], "head").len(),
            1
        );
    }

    #[test]
    #[trace("FR-059-AC-6")]
    fn edited_review_body_cannot_add_or_remove_an_approval_marker() {
        let policy = policy();
        let mut edited_to_add = review(
            1,
            "101",
            ReviewState::Approved,
            "unmarked submitted body",
            "head",
        );
        edited_to_add.submitted_body =
            "Signed-off-meaning: added later\nRole: developer".to_owned();
        edited_to_add.body_edited = true;
        assert!(github_approvals(&policy, &[edited_to_add], "head").is_empty());

        let mut edited_to_remove = review(
            2,
            "101",
            ReviewState::Approved,
            "Signed-off-meaning: removed later\nRole: developer",
            "head",
        );
        edited_to_remove.submitted_body = "edited body no longer has markers".to_owned();
        edited_to_remove.body_edited = true;
        assert!(github_approvals(&policy, &[edited_to_remove], "head").is_empty());

        let fresh = review(
            3,
            "101",
            ReviewState::Approved,
            "Signed-off-meaning: fresh\nRole: developer",
            "head",
        );
        assert_eq!(github_approvals(&policy, &[fresh], "head").len(), 1);
    }

    #[test]
    #[trace("FR-059-AC-9")]
    fn approval_contract_keeps_platform_details_out_of_member_names() {
        let approvals = github_approvals(
            &policy(),
            &[review(
                1,
                "101",
                ReviewState::Approved,
                "Signed-off-meaning: yes\nRole: developer",
                "head",
            )],
            "head",
        );
        let serialized = serde_json::to_value(approvals.first().expect("approval"))
            .expect("approval serializes");
        let object = serialized.as_object().expect("object");
        assert_eq!(
            object.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            BTreeSet::from([
                "account",
                "commit",
                "human",
                "meaning",
                "pull_request",
                "role",
                "source",
                "timestamp",
            ])
        );
        assert!(!object.contains_key("review_id"));
        assert!(!object.contains_key("github_login"));
    }

    #[test]
    fn github_adapter_rejects_accounts_from_another_source() {
        let mut other_source = review(
            1,
            "101",
            ReviewState::Approved,
            "Signed-off-meaning: yes\nRole: developer",
            "head",
        );
        other_source.account.source = "other-tracker".to_owned();
        assert!(github_approvals(&policy(), &[other_source], "head").is_empty());
    }

    #[test]
    #[trace("FR-059-AC-9")]
    fn test_identity_fixture_adapter_uses_the_source_neutral_approval_contract() {
        let fixture_account = AccountRef {
            source: "test-identity-fixture".to_owned(),
            id: "fixture-human-security".to_owned(),
        };
        let policy = RolePolicy {
            protocol: POLICY_PROTOCOL.to_owned(),
            required_roles: vec!["security".to_owned()],
            individuals: BTreeMap::from([("reviewer".to_owned(), vec![fixture_account.clone()])]),
            roles: BTreeMap::from([("security".to_owned(), vec!["reviewer".to_owned()])]),
            agents: Vec::new(),
        };
        let adapter = TestIdentityFixtureAdapter;
        let approvals = adapter.approvals(
            &policy,
            &[TestIdentityReview {
                account: fixture_account.clone(),
                role: "security".to_owned(),
                meaning: "fixture-only approval".to_owned(),
                commit: "head".to_owned(),
                timestamp: "2026-11-14T03:00:00Z".to_owned(),
                pull_request: "agent-ix/ea-109-sandbox-20261114#1".to_owned(),
            }],
            "head",
        );
        let result = evaluate(
            &policy,
            &approvals,
            &BTreeMap::from([(fixture_account, AccountType::Human)]),
        );
        assert_eq!(adapter.source(), "test-identity-fixture");
        assert_eq!(result.status, GateStatus::Satisfied);
    }

    #[test]
    #[trace("FR-061-AC-1", "FR-061-AC-2", "FR-063-AC-4")]
    fn evaluates_every_role_and_rejects_shared_or_missing_approvals() {
        let policy = policy();
        let reviews = [
            review(
                1,
                "101",
                ReviewState::Approved,
                "Signed-off-meaning: yes\nRole: developer",
                "head",
            ),
            review(
                2,
                "202",
                ReviewState::Approved,
                "Signed-off-meaning: yes\nRole: engineer",
                "head",
            ),
        ];
        let approvals = github_approvals(&policy, &reviews, "head");
        let satisfied = evaluate(&policy, &approvals, &account_types());
        assert_eq!(satisfied.status, GateStatus::Satisfied);
        assert_eq!(satisfied.assignments.len(), 2);
        let missing_approval = evaluate(&policy, &approvals[..1], &account_types());
        assert_eq!(missing_approval.status, GateStatus::Unsatisfied);
        assert_eq!(missing_approval.missing[0].role, "engineer");
        assert_eq!(
            missing_approval.missing[0].reason,
            MissingReason::NoApproval
        );

        let mut shared_policy = policy.clone();
        shared_policy
            .individuals
            .get_mut("alice")
            .expect("alice policy individual")
            .push(account("102"));
        shared_policy
            .roles
            .insert("engineer".to_owned(), vec!["alice".to_owned()]);
        let shared_reviews = [
            review(
                3,
                "101",
                ReviewState::Approved,
                "Signed-off-meaning: yes\nRole: developer",
                "head",
            ),
            review(
                4,
                "102",
                ReviewState::Approved,
                "Signed-off-meaning: yes\nRole: engineer",
                "head",
            ),
        ];
        let shared_approvals = github_approvals(&shared_policy, &shared_reviews, "head");
        let shared_types = BTreeMap::from([
            (account("101"), AccountType::Human),
            (account("102"), AccountType::Human),
            (account("202"), AccountType::Human),
        ]);
        let shared = evaluate(&shared_policy, &shared_approvals, &shared_types);
        assert_eq!(shared.status, GateStatus::Unsatisfied);
        assert_eq!(shared.missing.len(), 1);
        assert_eq!(shared.missing[0].reason, MissingReason::SharedIndividual);
    }

    #[test]
    #[trace("FR-061-AC-5", "FR-063-AC-6")]
    fn refuses_invalid_policies_and_reports_unavailable_account_lookups() {
        let mut invalid = policy();
        invalid.required_roles.clear();
        assert_eq!(
            evaluate(&invalid, &[], &account_types()).error,
            Some(GateErrorCode::ApprovalPolicyInvalid)
        );
        let policy = policy();
        let mut lookups = account_types();
        lookups.insert(account("202"), AccountType::Unavailable);
        let result = evaluate(&policy, &[], &lookups);
        assert_eq!(result.status, GateStatus::Unavailable);
        assert_eq!(result.error, Some(GateErrorCode::ApprovalSourceUnavailable));
        assert_eq!(result.unavailable_accounts, vec![account("202")]);
    }

    #[test]
    #[trace("FR-061-AC-10", "FR-061-AC-11", "FR-061-AC-12")]
    fn chooses_the_lexicographically_smallest_maximum_matching() {
        let policy = RolePolicy {
            protocol: POLICY_PROTOCOL.to_owned(),
            required_roles: vec!["a".into(), "b".into(), "c".into()],
            individuals: BTreeMap::from([
                ("x".into(), vec![account("101")]),
                ("y".into(), vec![account("202")]),
            ]),
            roles: BTreeMap::from([
                ("a".into(), vec!["x".into(), "y".into()]),
                ("b".into(), vec!["x".into()]),
                ("c".into(), vec!["y".into()]),
            ]),
            agents: Vec::new(),
        };
        let approvals = vec![
            Approval {
                source: "github".into(),
                account: account("101"),
                human: true,
                role: "a".into(),
                meaning: "ok".into(),
                timestamp: "t".into(),
                commit: "h".into(),
                pull_request: "p".into(),
            },
            Approval {
                source: "github".into(),
                account: account("101"),
                human: true,
                role: "b".into(),
                meaning: "ok".into(),
                timestamp: "t".into(),
                commit: "h".into(),
                pull_request: "p".into(),
            },
            Approval {
                source: "github".into(),
                account: account("202"),
                human: true,
                role: "a".into(),
                meaning: "ok".into(),
                timestamp: "t".into(),
                commit: "h".into(),
                pull_request: "p".into(),
            },
            Approval {
                source: "github".into(),
                account: account("202"),
                human: true,
                role: "c".into(),
                meaning: "ok".into(),
                timestamp: "t".into(),
                commit: "h".into(),
                pull_request: "p".into(),
            },
        ];
        let types = BTreeMap::from([
            (account("101"), AccountType::Human),
            (account("202"), AccountType::Human),
        ]);
        let result = evaluate(&policy, &approvals, &types);
        assert_eq!(
            result
                .assignments
                .iter()
                .map(|pair| (pair.role.as_str(), pair.individual.as_str()))
                .collect::<Vec<_>>(),
            vec![("a", "x"), ("c", "y")]
        );
        assert_eq!(
            result.missing,
            vec![MissingRole {
                role: "b".into(),
                reason: MissingReason::SharedIndividual
            }]
        );
    }

    #[test]
    #[trace("FR-061-AC-5")]
    fn policy_json_rejects_unknown_members() {
        let value = serde_json::json!({
            "protocol": POLICY_PROTOCOL,
            "required_roles": ["developer"],
            "individuals": {"alice": [{"source": "github", "id": "101"}]},
            "roles": {"developer": ["alice"]},
            "unknown": true
        });
        assert_eq!(
            parse_policy(&serde_json::to_vec(&value).expect("policy JSON")),
            Err(PolicyError::SchemaMismatch)
        );
    }

    #[test]
    #[trace("FR-061-AC-8")]
    fn published_example_role_policy_matches_the_closed_schema() {
        let parsed = parse_policy(include_bytes!("../examples/approval-policy.json"))
            .expect("example policy is valid");
        assert_eq!(parsed.required_roles, vec!["security", "technical"]);
        assert_eq!(parsed.agents, vec![account("1003")]);
    }

    #[test]
    fn public_request_and_result_schemas_are_valid_draft_2020_12() {
        for schema in [REQUEST_SCHEMA, RESULT_SCHEMA] {
            let value: serde_json::Value = serde_json::from_str(schema).expect("schema JSON");
            jsonschema::options()
                .with_draft(jsonschema::Draft::Draft202012)
                .build(&value)
                .expect("schema compiles");
        }
    }

    #[test]
    #[trace("FR-061-AC-5", "FR-059-AC-8")]
    fn policy_schema_refuses_duplicate_accounts_and_github_logins() {
        let duplicate = serde_json::json!({
            "protocol": POLICY_PROTOCOL,
            "required_roles": ["developer"],
            "individuals": {
                "alice": [{"source": "github", "id": "101"}],
                "bob": [{"source": "github", "id": "101"}]
            },
            "roles": {"developer": ["alice"]}
        });
        let login = serde_json::json!({
            "protocol": POLICY_PROTOCOL,
            "required_roles": ["developer"],
            "individuals": {
                "alice": [{"source": "github", "id": "alice"}]
            },
            "roles": {"developer": ["alice"]}
        });
        assert_eq!(
            parse_policy(&serde_json::to_vec(&duplicate).expect("json")),
            Err(PolicyError::DuplicateAccount)
        );
        assert_eq!(
            parse_policy(&serde_json::to_vec(&login).expect("json")),
            Err(PolicyError::Account)
        );
    }

    #[test]
    #[trace("FR-062-AC-4", "FR-063-AC-2", "FR-063-AC-5")]
    fn reports_agent_only_and_unstaffed_roles_without_weakening_the_gate() {
        let policy = policy();
        let agent_approval = Approval {
            source: "github".into(),
            account: account("303"),
            human: false,
            role: "developer".into(),
            meaning: "not counted".into(),
            timestamp: "2026-11-14T03:00:00Z".into(),
            commit: "head".into(),
            pull_request: "agent-ix/ea-109-sandbox#1".into(),
        };
        let result = evaluate(&policy, &[agent_approval], &account_types());
        assert_eq!(result.status, GateStatus::Unsatisfied);
        assert_eq!(result.missing[0].reason, MissingReason::AgentOnly);

        let mut types = account_types();
        types.insert(account("202"), AccountType::BotOrApp);
        let result = evaluate(&policy, &[], &types);
        assert_eq!(result.error, Some(GateErrorCode::ApprovalRoleUnstaffed));
        assert_eq!(
            result
                .missing
                .iter()
                .map(|row| row.role.as_str())
                .collect::<Vec<_>>(),
            vec!["engineer"]
        );
    }

    #[test]
    #[trace("FR-061-AC-6", "FR-061-AC-9")]
    fn role_matching_is_order_independent_and_equivalent_to_exhaustive_search() {
        let roles = vec!["a".to_owned(), "b".to_owned()];
        let people = ["x", "y"];
        let policy = RolePolicy {
            protocol: POLICY_PROTOCOL.to_owned(),
            required_roles: roles.clone(),
            individuals: BTreeMap::from([
                ("x".into(), vec![account("101")]),
                ("y".into(), vec![account("202")]),
            ]),
            roles: BTreeMap::from([
                ("a".into(), vec!["x".into(), "y".into()]),
                ("b".into(), vec!["x".into(), "y".into()]),
            ]),
            agents: Vec::new(),
        };
        let types = BTreeMap::from([
            (account("101"), AccountType::Human),
            (account("202"), AccountType::Human),
        ]);
        for mask in 0_u8..16 {
            let approvals = (0..4)
                .filter(|edge| mask & (1 << edge) != 0)
                .map(|edge| {
                    let role = if edge / 2 == 0 { "a" } else { "b" };
                    let person = people[edge % 2];
                    let id = if person == "x" { "101" } else { "202" };
                    Approval {
                        source: "github".into(),
                        account: account(id),
                        human: true,
                        role: role.into(),
                        meaning: "reviewed".into(),
                        timestamp: "2026-11-14T03:00:00Z".into(),
                        commit: "head".into(),
                        pull_request: "agent-ix/ea-109-sandbox#1".into(),
                    }
                })
                .collect::<Vec<_>>();
            let forward = evaluate(&policy, &approvals, &types);
            let reverse = evaluate(
                &policy,
                &approvals.iter().cloned().rev().collect::<Vec<_>>(),
                &types,
            );
            let exhaustive = (0..2).any(|first_person| {
                let first_edge = 1_u8 << first_person;
                let second_edge = 1_u8 << (2 + (1 - first_person));
                mask & first_edge != 0 && mask & second_edge != 0
            });
            assert_eq!(
                forward.status == GateStatus::Satisfied,
                exhaustive,
                "mask {mask:04b}"
            );
            assert_eq!(forward.status, reverse.status, "mask {mask:04b}");
            assert_eq!(forward.assignments, reverse.assignments, "mask {mask:04b}");
        }
    }

    #[test]
    #[trace("FR-061-AC-3", "FR-061-AC-7", "FR-061-AC-8")]
    fn one_role_and_multiple_accounts_resolve_to_one_policy_individual() {
        let policy = RolePolicy {
            protocol: POLICY_PROTOCOL.to_owned(),
            required_roles: vec!["stakeholder".into()],
            individuals: BTreeMap::from([("alice".into(), vec![account("101"), account("102")])]),
            roles: BTreeMap::from([("stakeholder".into(), vec!["alice".into()])]),
            agents: Vec::new(),
        };
        let types = BTreeMap::from([
            (account("101"), AccountType::Human),
            (account("102"), AccountType::Human),
        ]);
        let approval = Approval {
            source: "github".into(),
            account: account("102"),
            human: true,
            role: "stakeholder".into(),
            meaning: "approved".into(),
            timestamp: "2026-11-14T03:00:00Z".into(),
            commit: "head".into(),
            pull_request: "agent-ix/ea-109-sandbox#1".into(),
        };
        let result = evaluate(&policy, &[approval], &types);
        assert_eq!(result.status, GateStatus::Satisfied);
        assert_eq!(result.assignments[0].individual, "alice");
    }

    #[test]
    #[trace("FR-061-AC-5", "FR-063-AC-6")]
    fn absent_account_type_lookup_is_unavailable_not_a_nonhuman_guess() {
        let policy = policy();
        let result = evaluate(&policy, &[], &BTreeMap::new());
        assert_eq!(result.status, GateStatus::Unavailable);
        assert_eq!(result.error, Some(GateErrorCode::ApprovalSourceUnavailable));
        assert_eq!(
            result.unavailable_accounts,
            vec![account("101"), account("202")]
        );
    }
}
