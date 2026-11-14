// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-061 role-matching vectors against the shared approval-gate evaluator.

use std::collections::{BTreeMap, BTreeSet};

use engineering_assurance::approval_gate::{
    AccountRef, AccountType, Approval, GateErrorCode, GateStatus, POLICY_PROTOCOL, RolePolicy,
    evaluate, parse_policy,
};
use ix_trace_rs::trace;
use serde_json::json;

const HEAD: &str = "head-1";

fn account(id: &str) -> AccountRef {
    AccountRef {
        source: "github".to_owned(),
        id: id.to_owned(),
    }
}

fn policy(roles: &[(&str, &[&str])]) -> RolePolicy {
    let required_roles = roles.iter().map(|(role, _)| (*role).to_owned()).collect();
    let mut individuals = BTreeMap::new();
    let mut role_map = BTreeMap::new();
    let mut people = BTreeSet::new();
    for (role, names) in roles {
        role_map.insert(
            (*role).to_owned(),
            names.iter().map(|name| (*name).to_owned()).collect(),
        );
        people.extend(names.iter().copied());
    }
    for name in people {
        let id = match name {
            "one" | "x" | "alice" => "101",
            "two" | "y" | "bob" => "202",
            "z" => "303",
            other => panic!("unexpected policy individual {other}"),
        };
        individuals.insert(name.to_owned(), vec![account(id)]);
    }
    RolePolicy {
        protocol: POLICY_PROTOCOL.to_owned(),
        required_roles,
        individuals,
        roles: role_map,
        agents: Vec::new(),
    }
}

fn approval(role: &str, id: &str) -> Approval {
    Approval {
        source: "github".to_owned(),
        account: account(id),
        human: true,
        role: role.to_owned(),
        meaning: "reviewed".to_owned(),
        timestamp: "2026-11-14T03:00:00Z".to_owned(),
        commit: HEAD.to_owned(),
        pull_request: "agent-ix/ea-110-sandbox#1".to_owned(),
    }
}

fn account_types(policy: &RolePolicy) -> BTreeMap<AccountRef, AccountType> {
    policy
        .individuals
        .values()
        .flatten()
        .chain(&policy.agents)
        .cloned()
        .map(|identity| (identity, AccountType::Human))
        .collect()
}

#[test]
#[trace("FR-061-AC-1")]
fn a_single_required_role_can_be_covered_by_one_person() {
    let policy = policy(&[("release_owner", &["one"])]);
    let result = evaluate(
        &policy,
        &[approval("release_owner", "101")],
        &account_types(&policy),
    );
    assert_eq!(result.status, GateStatus::Satisfied);
    assert_eq!(result.assignments.len(), 1);
    assert_eq!(result.assignments[0].individual, "one");
}

#[test]
#[trace("FR-061-AC-2", "FR-061-AC-11")]
fn one_individual_cannot_cover_two_roles_and_order_does_not_matter() {
    let policy = policy(&[("developer", &["one"]), ("engineer", &["one"])]);
    let first = evaluate(
        &policy,
        &[approval("developer", "101"), approval("engineer", "101")],
        &account_types(&policy),
    );
    let second = evaluate(
        &policy,
        &[approval("engineer", "101"), approval("developer", "101")],
        &account_types(&policy),
    );
    assert_eq!(first, second);
    assert_eq!(first.status, GateStatus::Unsatisfied);
    assert_eq!(first.missing[0].role, "engineer");
    assert_eq!(
        first.missing[0].reason,
        engineering_assurance::approval_gate::MissingReason::SharedIndividual
    );
}

#[test]
#[trace("FR-061-AC-5")]
fn invalid_policy_is_rejected_with_stable_code() {
    let unknown = json!({
        "protocol": POLICY_PROTOCOL,
        "required_roles": ["dev"],
        "individuals": {"person": [{"source":"github", "id":"101"}]},
        "roles": {"dev": ["person"]},
        "unexpected": true
    });
    assert!(parse_policy(&serde_json::to_vec(&unknown).unwrap()).is_err());

    let mut duplicate_role = policy(&[("dev", &["one"]), ("eng", &["two"])]);
    duplicate_role.required_roles.push("dev".to_owned());
    assert_eq!(
        evaluate(&duplicate_role, &[], &account_types(&duplicate_role)).error,
        Some(GateErrorCode::ApprovalPolicyInvalid)
    );

    let mut missing_role = policy(&[("dev", &["one"])]);
    missing_role.roles.remove("dev");
    assert_eq!(
        evaluate(&missing_role, &[], &account_types(&missing_role)).error,
        Some(GateErrorCode::ApprovalPolicyInvalid)
    );

    let mut undeclared = policy(&[("dev", &["one"])]);
    undeclared
        .roles
        .get_mut("dev")
        .unwrap()
        .push("unknown".to_owned());
    assert_eq!(
        evaluate(&undeclared, &[], &account_types(&undeclared)).error,
        Some(GateErrorCode::ApprovalPolicyInvalid)
    );

    let mut duplicate_account = policy(&[("dev", &["one"]), ("eng", &["two"])]);
    duplicate_account
        .individuals
        .get_mut("two")
        .unwrap()
        .push(account("101"));
    assert_eq!(
        evaluate(&duplicate_account, &[], &account_types(&duplicate_account)).error,
        Some(GateErrorCode::ApprovalPolicyInvalid)
    );

    let mut agent_overlap = policy(&[("dev", &["one"])]);
    agent_overlap.agents.push(account("101"));
    assert_eq!(
        evaluate(&agent_overlap, &[], &account_types(&agent_overlap)).error,
        Some(GateErrorCode::ApprovalPolicyInvalid)
    );
}

#[test]
#[trace("FR-061-AC-6")]
fn generated_two_by_two_graphs_match_exhaustive_search() {
    let names = ["one", "two"];
    let role_names = ["a", "b"];
    for mask in 0_u8..16 {
        let mut roles = Vec::new();
        let mut accepted = BTreeSet::new();
        let mut approvals = Vec::new();
        for (role_index, role) in role_names.iter().enumerate() {
            let people: Vec<&str> = names
                .iter()
                .enumerate()
                .filter_map(|(person_index, person)| {
                    let bit = role_index * names.len() + person_index;
                    ((mask & (1 << bit)) != 0).then_some(*person)
                })
                .collect();
            for person in &people {
                accepted.insert(((*role).to_owned(), (*person).to_owned()));
                let id = if *person == "one" { "101" } else { "202" };
                approvals.push(approval(role, id));
            }
            roles.push((*role, people));
        }
        let role_refs = roles
            .iter()
            .map(|(role, people)| (*role, people.as_slice()))
            .collect::<Vec<_>>();
        let policy = policy(&role_refs);
        let types = account_types(&policy);
        let first = evaluate(&policy, &approvals, &types);
        approvals.reverse();
        let second = evaluate(&policy, &approvals, &types);
        assert_eq!(first, second, "mask={mask:04b}");

        let expected_satisfied = accepted.iter().any(|(first_role, first_person)| {
            accepted.iter().any(|(second_role, second_person)| {
                first_role != second_role && first_person != second_person
            })
        });
        assert_eq!(
            first.status == GateStatus::Satisfied,
            expected_satisfied,
            "mask={mask:04b}"
        );
    }
}

#[test]
#[trace("FR-061-AC-7", "FR-061-AC-8")]
fn account_identity_and_role_eligibility_are_respected() {
    let mut policy = policy(&[("developer", &["one"]), ("engineer", &["one", "two"])]);
    policy.individuals.get_mut("one").unwrap().push(AccountRef {
        source: "jira".to_owned(),
        id: "same-jira".to_owned(),
    });
    let mut types = account_types(&policy);
    types.insert(
        AccountRef {
            source: "jira".to_owned(),
            id: "same-jira".to_owned(),
        },
        AccountType::Human,
    );
    let mut jira_engineer = approval("engineer", "101");
    jira_engineer.account = AccountRef {
        source: "jira".to_owned(),
        id: "same-jira".to_owned(),
    };
    let mut unknown_developer = approval("developer", "202");
    let unknown_role = approval("engineer", "999");
    unknown_developer.role = "unlisted".to_owned();
    let result = evaluate(
        &policy,
        &[
            approval("developer", "101"),
            jira_engineer,
            unknown_role,
            unknown_developer,
            approval("engineer", "202"),
        ],
        &types,
    );
    assert_eq!(result.status, GateStatus::Satisfied);
    assert_eq!(result.assignments[0].individual, "one");
    assert_eq!(result.assignments[1].individual, "two");
    assert_eq!(result.assignments[1].account.id, "202");
}

#[test]
#[trace("FR-061-AC-10")]
fn matching_reassigns_earlier_role_to_find_full_coverage() {
    let policy = policy(&[("a", &["one", "two"]), ("b", &["one"])]);
    let result = evaluate(
        &policy,
        &[
            approval("b", "101"),
            approval("a", "101"),
            approval("a", "202"),
        ],
        &account_types(&policy),
    );
    assert_eq!(result.status, GateStatus::Satisfied);
    assert_eq!(result.assignments[0].individual, "two");
    assert_eq!(result.assignments[1].individual, "one");
}

#[test]
#[trace("FR-061-AC-11", "FR-061-AC-12")]
fn missing_role_is_the_smallest_maximum_matching_complement() {
    let policy = policy(&[("a", &["one"]), ("b", &["one"]), ("c", &["one", "two"])]);
    let result = evaluate(
        &policy,
        &[
            approval("c", "202"),
            approval("a", "101"),
            approval("b", "101"),
            approval("c", "101"),
        ],
        &account_types(&policy),
    );
    assert_eq!(result.status, GateStatus::Unsatisfied);
    assert_eq!(
        result
            .missing
            .iter()
            .map(|item| item.role.as_str())
            .collect::<Vec<_>>(),
        vec!["b"]
    );
    assert_eq!(
        result.missing[0].reason,
        engineering_assurance::approval_gate::MissingReason::SharedIndividual
    );
}
