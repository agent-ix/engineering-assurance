// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Request adapter for the opt-in `review-gate` command (FR-059, FR-065).

use std::collections::BTreeMap;

use engineering_assurance::approval_gate::{
    AccountRef, AccountType, ApprovalSource, GateErrorCode, GateResult, GateStatus,
    GitHubReviewAdapter, POLICY_PROTOCOL, ReviewSnapshot, RolePolicy, evaluate, parse_policy,
};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

const REQUEST_PROTOCOL: &str = "engineering-assurance.review-gate-request/v1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    current_head: String,
    policy: Value,
    reviews: Vec<ReviewSnapshot>,
    account_types: Vec<AccountClassification>,
    #[serde(default)]
    source_unavailable: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountClassification {
    account: AccountRef,
    account_type: AccountType,
}

/// A malformed or unreadable gate request.
#[derive(Debug, Error)]
pub(crate) enum RequestError {
    /// The request is not readable JSON.
    #[error("review-gate request is not readable JSON")]
    Unreadable,
    /// The request is not an exact request for this protocol.
    #[error("review-gate request is not an exact request")]
    Invalid,
    /// The result could not be encoded.
    #[error("review-gate result cannot be encoded")]
    Encoding,
}

impl RequestError {
    /// Stable machine error code.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "review_gate_request_unreadable",
            Self::Invalid => "review_gate_request_invalid",
            Self::Encoding => "review_gate_result_encoding_failed",
        }
    }
}

/// Evaluate one normalized GitHub review request.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, RequestError> {
    let request: Request = serde_json::from_slice(bytes).map_err(|_| RequestError::Unreadable)?;
    if request.protocol != REQUEST_PROTOCOL || request.current_head.is_empty() {
        return Err(RequestError::Invalid);
    }
    let policy_bytes = serde_json::to_vec(&request.policy).map_err(|_| RequestError::Invalid)?;
    let Ok(policy) = parse_policy(&policy_bytes) else {
        let result = GateResult {
            protocol: engineering_assurance::approval_gate::RESULT_PROTOCOL,
            status: GateStatus::Unsatisfied,
            required_roles: Vec::new(),
            assignments: Vec::new(),
            missing: Vec::new(),
            error: Some(GateErrorCode::ApprovalPolicyInvalid),
            unavailable_accounts: Vec::new(),
        };
        return encode(&result);
    };
    if policy.protocol != POLICY_PROTOCOL {
        return Err(RequestError::Invalid);
    }
    if request.source_unavailable {
        let result = GateResult {
            protocol: engineering_assurance::approval_gate::RESULT_PROTOCOL,
            status: GateStatus::Unavailable,
            required_roles: sorted_roles(&policy),
            assignments: Vec::new(),
            missing: Vec::new(),
            error: Some(GateErrorCode::ApprovalSourceUnavailable),
            unavailable_accounts: Vec::new(),
        };
        return encode(&result);
    }
    let mut account_types = BTreeMap::new();
    for classification in request.account_types {
        if account_types
            .insert(classification.account, classification.account_type)
            .is_some()
        {
            return Err(RequestError::Invalid);
        }
    }
    let adapter = GitHubReviewAdapter;
    let approvals = adapter.approvals(&policy, &request.reviews, &request.current_head);
    let result = evaluate(&policy, &approvals, &account_types);
    encode(&result)
}

fn sorted_roles(policy: &RolePolicy) -> Vec<String> {
    let mut roles = policy.required_roles.clone();
    roles.sort();
    roles
}

fn encode(result: &GateResult) -> Result<Vec<u8>, RequestError> {
    let mut bytes = serde_json::to_vec(result).map_err(|_| RequestError::Encoding)?;
    bytes.push(b'\n');
    Ok(bytes)
}
