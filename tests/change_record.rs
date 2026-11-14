// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

use std::collections::BTreeMap;

use engineering_assurance::change_record::{
    AgentSession, ApprovalDecision, ApprovalVerdict, BomError, Failure, McpServer, Plugin,
    PredicateInput, Review, ReviewKind, ReviewVerdict, Status, Ticket, TicketKind, build_bom,
    build_predicate, validate_bom_value, verify_private_form_digest,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

fn sha(fill: char) -> String {
    format!("sha256:{}", fill.to_string().repeat(64))
}

fn fixture() -> PredicateInput {
    PredicateInput {
        record_id: "8a6b6ed3-a08a-4bad-9163-2c318ff2f4a1".to_owned(),
        ticket: Some(Ticket {
            kind: TicketKind::LinearIssue,
            id: "EA-107".to_owned(),
        }),
        criteria: vec!["AC-1".to_owned(), "AC-2".to_owned()],
        spec_criteria: vec!["FR-072-AC-1".to_owned(), "FR-073-AC-1".to_owned()],
        reviews: vec![
            Review {
                kind: ReviewKind::CodeReview,
                verdict: ReviewVerdict::Pass,
                artifact_digest: sha('a'),
            },
            Review {
                kind: ReviewKind::GapAnalysis,
                verdict: ReviewVerdict::Pass,
                artifact_digest: sha('b'),
            },
        ],
        approvals: ["developer", "engineer", "stakeholder"]
            .map(|role| {
                (
                    role.to_owned(),
                    ApprovalDecision::Covered {
                        statement_digest: sha('c'),
                    },
                )
            })
            .into(),
        test_results: sha('d'),
        agent_bom: sha('e'),
        private_form: json!({
            "ticket_title": "Fictional ticket title",
            "criterion_text": "Private acceptance sentence",
            "model_name": "Private Model Name",
            "approval_meaning": "Private meaning",
            "approver": "Fictional Person"
        }),
        failures: vec![],
    }
}

#[trace("TC-570", "TC-573", "FR-072-AC-1", "FR-072-AC-4")]
#[test]
fn predicate_has_exact_public_shape_and_excludes_private_text() {
    let input = fixture();
    let private_form = input.private_form.clone();
    let predicate = build_predicate(input).expect("fixture builds");
    let value = serde_json::to_value(&predicate).expect("serializes");
    assert_eq!(predicate.status, Status::Passed);
    assert_eq!(predicate.approvals.len(), 3);
    assert!(predicate.approvals.iter().all(|approval| {
        approval.verdict == ApprovalVerdict::Covered && approval.statement_digest.is_some()
    }));
    assert!(predicate.failures.is_empty());
    assert_eq!(value.as_object().expect("object").len(), 11);
    let bytes = serde_json::to_vec(&predicate).expect("public bytes");
    for private in [
        "Fictional ticket title",
        "Private acceptance sentence",
        "Private Model Name",
        "Private meaning",
        "Fictional Person",
    ] {
        assert!(!String::from_utf8_lossy(&bytes).contains(private));
    }
    assert!(verify_private_form_digest(&predicate, &private_form));
}

#[trace("TC-571", "FR-072-AC-2")]
#[test]
fn predicate_deserialization_and_schema_reject_invalid_shapes() {
    let predicate = build_predicate(fixture()).expect("fixture builds");
    let mut unknown = serde_json::to_value(&predicate).expect("json");
    unknown["unexpected"] = json!(true);
    assert!(
        serde_json::from_value::<engineering_assurance::change_record::ChangePredicate>(unknown)
            .is_err()
    );
    let mut bad_grammar = serde_json::to_value(&predicate).expect("json");
    bad_grammar["criteria"][0] = json!("AC-0000");
    assert!(
        engineering_assurance::change_record::predicate::validate_predicate(
            &serde_json::from_value::<engineering_assurance::change_record::ChangePredicate>(
                serde_json::to_value(&predicate).expect("json")
            )
            .expect("typed predicate")
        )
        .is_ok()
    );
    let schema = include_str!("../src/change_record_schemas/change-record-v1.schema.json");
    let schema: Value = serde_json::from_str(schema).expect("schema");
    let validator = jsonschema::validator_for(&schema).expect("compiled schema");
    assert!(!validator.is_valid(&bad_grammar));
    let mut repo = serde_json::to_value(&predicate).expect("json");
    repo["ticket"]["kind"] = json!("repo");
    assert!(
        serde_json::from_value::<engineering_assurance::change_record::ChangePredicate>(repo)
            .is_err()
    );
}

#[trace("TC-572", "FR-072-AC-3")]
#[test]
fn predicate_shows_missing_unstaffed_and_unavailable_roles() {
    let mut input = fixture();
    input.approvals = vec![
        ("engineer".to_owned(), ApprovalDecision::Missing),
        ("developer".to_owned(), ApprovalDecision::Unstaffed),
        ("stakeholder".to_owned(), ApprovalDecision::Unavailable),
    ];
    let predicate = build_predicate(input).expect("failed predicate serializes");
    assert_eq!(predicate.status, Status::Failed);
    assert_eq!(predicate.approvals[0].verdict, ApprovalVerdict::Missing);
    assert_eq!(predicate.approvals[0].statement_digest, None);
    for failure in [
        Failure::ApprovalsIncomplete,
        Failure::ApprovalRoleUnstaffed,
        Failure::ApprovalsUnavailable,
    ] {
        assert!(predicate.failures.contains(&failure));
    }
}

#[trace("TC-576", "FR-072-AC-7")]
#[test]
fn one_private_form_byte_change_breaks_digest_verification() {
    let input = fixture();
    let private_form = input.private_form.clone();
    let predicate = build_predicate(input).expect("fixture builds");
    let mut changed = private_form;
    changed["ticket_title"] = json!("Fictional ticket titlf");
    assert!(!verify_private_form_digest(&predicate, &changed));
}

fn session() -> AgentSession {
    AgentSession {
        harness: "Fictional Harness".to_owned(),
        model: "Private Model Name".to_owned(),
        plugins: vec![Plugin {
            name: "Fictional Plugin".to_owned(),
            purl: "pkg:generic/fictional-plugin@1.0".to_owned(),
        }],
        mcp_servers: vec![McpServer {
            name: "Fictional MCP".to_owned(),
            endpoint: None,
        }],
        facts: BTreeMap::from([("session-kind".to_owned(), "coding".to_owned())]),
    }
}

#[trace("TC-577", "TC-578", "FR-073-AC-1", "FR-073-AC-2")]
#[test]
fn bom_validates_and_every_property_is_producer_reported() {
    let bom = build_bom(&session()).expect("valid CycloneDX document");
    assert_eq!(bom.spec_version, "1.7");
    assert!(
        bom.components
            .iter()
            .any(|component| component.component_type
                == engineering_assurance::change_record::bom::ComponentType::Application)
    );
    assert!(
        bom.components
            .iter()
            .any(|component| component.component_type
                == engineering_assurance::change_record::bom::ComponentType::MachineLearningModel)
    );
    assert!(
        bom.components
            .iter()
            .any(|component| component.purl.is_some())
    );
    assert_eq!(bom.services.len(), 2);
    let value = serde_json::to_value(&bom).expect("json");
    let mut all_properties = value["properties"]
        .as_array()
        .expect("root properties")
        .clone();
    for component in value["components"].as_array().expect("components") {
        all_properties.extend(
            component["properties"]
                .as_array()
                .expect("component properties")
                .clone(),
        );
    }
    for service in value["services"].as_array().expect("services") {
        all_properties.extend(
            service["properties"]
                .as_array()
                .expect("service properties")
                .clone(),
        );
    }
    assert!(all_properties.iter().all(|property| {
        property["name"]
            .as_str()
            .is_some_and(|name| name.starts_with("agent-ix:"))
    }));
    assert!(
        all_properties
            .iter()
            .filter(|property| property["name"] == "agent-ix:reported-by")
            .all(|property| property["value"] == "producer")
    );
    assert!(validate_bom_value(&value).is_ok());
    let bom_bytes = serde_json::to_vec(&bom).expect("bom bytes");
    assert!(String::from_utf8_lossy(&bom_bytes).contains("Private Model Name"));
}

#[trace("TC-579", "FR-073-AC-3")]
#[test]
fn rejects_bom_formulation() {
    let mut value = serde_json::to_value(build_bom(&session()).expect("BOM")).expect("json");
    value["formulation"] = json!([]);
    assert!(matches!(
        validate_bom_value(&value),
        Err(BomError::ForbiddenSection)
    ));
}

/// Trace: TC-582, FR-073-AC-6
#[trace("TC-582", "FR-073-AC-6")]
#[test]
fn bom_random_serial_changes_canonical_digest() {
    let first =
        serde_json::to_value(build_bom(&session()).expect("first BOM")).expect("first JSON");
    let second =
        serde_json::to_value(build_bom(&session()).expect("second BOM")).expect("second JSON");
    assert_ne!(first["serialNumber"], second["serialNumber"]);
    assert_ne!(
        engineering_assurance::change_record::canonical_digest(&first).expect("first digest"),
        engineering_assurance::change_record::canonical_digest(&second).expect("second digest"),
    );
}
