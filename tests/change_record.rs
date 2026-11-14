// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Acceptance coverage for change-record predicates and producer-reported BOMs.

use std::collections::BTreeMap;

use engineering_assurance::change_record::{
    AgentSession, ApprovalDecision, ApprovalVerdict, BomError, ChangeSubjectInput, Failure,
    McpServer, Plugin, PredicateInput, ReviewInput, ReviewKind, ReviewVerdict, Status, Ticket,
    TicketKind, build_bom, build_change_record, canonical_digest, validate_bom_value,
    verify_private_form_digest,
};
use engineering_assurance::record_sink::form::{PrivateRecord, RecordId};
use ix_trace_rs::trace;
use serde_json::{Value, json};

fn fixture() -> PredicateInput {
    let record_id = "8a6b6ed3-a08a-4bad-9163-2c318ff2f4a1";
    let mut private_form = PrivateRecord::new(RecordId::try_from(record_id).expect("record id"));
    private_form.ticket_text = Some("Fictional ticket title".to_owned());
    private_form.criteria_text = Some("Private acceptance sentence".to_owned());
    private_form.change_record = Some(json!({
        "title": "Fictional ticket title",
        "criteria": ["Private acceptance sentence"],
        "model_name": "Private Model Name",
        "approval_meaning": "Private meaning",
        "approver": "Fictional Person"
    }));
    PredicateInput {
        record_id: record_id.to_owned(),
        ticket: Some(Ticket {
            kind: TicketKind::LinearIssue,
            id: "EA-107".to_owned(),
        }),
        criteria: vec!["AC-1".to_owned(), "AC-2".to_owned()],
        spec_criteria: vec!["FR-072-AC-1".to_owned(), "FR-073-AC-1".to_owned()],
        reviews: vec![
            ReviewInput {
                kind: ReviewKind::CodeReview,
                verdict: ReviewVerdict::Pass,
                artifact_bytes: b"# Code review\nPass.\n".to_vec(),
            },
            ReviewInput {
                kind: ReviewKind::GapAnalysis,
                verdict: ReviewVerdict::Pass,
                artifact_bytes: b"# Gap analysis\nPass.\n".to_vec(),
            },
        ],
        approvals: ["developer", "engineer", "stakeholder"]
            .map(|role| {
                (
                    role.to_owned(),
                    ApprovalDecision::Covered {
                        individual: format!("{role}-person"),
                        is_human: true,
                        statement: json!({"_type": engineering_assurance::change_record::STATEMENT_TYPE, "predicateType":"approval/v1", "subject":[], "predicate":{"role": role}}),
                    },
                )
            })
            .into(),
        test_results: json!({"_type": engineering_assurance::change_record::STATEMENT_TYPE, "predicateType":"test-result/v1", "subject":[], "predicate":{"result":"passed"}}),
        subject: ChangeSubjectInput {
            repository: "fictional/project".to_owned(),
            merged_commit: "a".repeat(40),
            merged_tree: "b".repeat(40),
        },
        session: Some(session()),
        private_form,
        failures: vec![],
    }
}

#[trace(
    "TC-570",
    "TC-573",
    "TC-575",
    "FR-072-AC-1",
    "FR-072-AC-4",
    "FR-072-AC-6"
)]
#[test]
fn predicate_has_exact_public_shape_and_excludes_private_text() {
    let input = fixture();
    let record = build_change_record(input).expect("fixture builds");
    let predicate = &record.statement.predicate;
    let value = serde_json::to_value(predicate).expect("serializes");
    assert_eq!(predicate.status, Status::Passed);
    assert_eq!(
        engineering_assurance::change_record::PREDICATE_TYPE,
        concat!(
            "https:",
            "//schemas.agent-ix.ai/attestation/change-record/v1"
        )
    );
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
    assert!(verify_private_form_digest(predicate, &record.private_form));
}

#[trace("TC-571", "FR-072-AC-2")]
#[test]
fn predicate_deserialization_and_schema_reject_invalid_shapes() {
    let predicate = build_change_record(fixture())
        .expect("fixture builds")
        .statement
        .predicate;
    let schema = include_str!("../src/change_record_schemas/change-record-v1.schema.json");
    let schema: Value = serde_json::from_str(schema).expect("schema");
    let validator = jsonschema::validator_for(&schema).expect("compiled schema");
    let mut unknown = serde_json::to_value(&predicate).expect("json");
    unknown["unexpected"] = json!(true);
    assert!(!validator.is_valid(&unknown));
    let mut bad_grammar = serde_json::to_value(&predicate).expect("json");
    bad_grammar["criteria"][0] = json!("AC-0000");
    assert!(!validator.is_valid(&bad_grammar));
    let mut repo = serde_json::to_value(&predicate).expect("json");
    repo["ticket"]["kind"] = json!("repo");
    assert!(!validator.is_valid(&repo));
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
    let predicate = build_change_record(input)
        .expect("failed predicate serializes")
        .statement
        .predicate;
    assert_eq!(predicate.status, Status::Failed);
    let engineer = predicate
        .approvals
        .iter()
        .find(|entry| entry.role == "engineer")
        .expect("engineer approval");
    assert_eq!(engineer.verdict, ApprovalVerdict::Missing);
    assert_eq!(engineer.statement_digest, None);
    let developer = predicate
        .approvals
        .iter()
        .find(|entry| entry.role == "developer")
        .expect("developer approval");
    assert_eq!(developer.verdict, ApprovalVerdict::Missing);
    assert_eq!(developer.statement_digest, None);
    for failure in [
        Failure::ApprovalsIncomplete,
        Failure::ApprovalRoleUnstaffed,
        Failure::ApprovalsUnavailable,
    ] {
        assert!(predicate.failures.contains(&failure));
    }
}

#[trace("FR-061-AC-2")]
#[test]
fn one_individual_cannot_cover_multiple_public_roles() {
    let mut input = fixture();
    input.approvals = vec![
        (
            "developer".to_owned(),
            ApprovalDecision::Covered {
                individual: "same-person".to_owned(),
                is_human: true,
                statement: json!({"role":"developer"}),
            },
        ),
        (
            "engineer".to_owned(),
            ApprovalDecision::Covered {
                individual: "same-person".to_owned(),
                is_human: true,
                statement: json!({"role":"engineer"}),
            },
        ),
        (
            "stakeholder".to_owned(),
            ApprovalDecision::Covered {
                individual: "other-person".to_owned(),
                is_human: true,
                statement: json!({"role":"stakeholder"}),
            },
        ),
    ];
    let predicate = build_change_record(input)
        .expect("incomplete predicate is emitted")
        .statement
        .predicate;
    assert_eq!(predicate.status, Status::Failed);
    assert_eq!(
        predicate
            .approvals
            .iter()
            .filter(|approval| approval.verdict == ApprovalVerdict::Covered)
            .count(),
        2
    );
    assert!(predicate.failures.contains(&Failure::ApprovalsIncomplete));
}

#[trace("FR-061-AC-8")]
#[test]
fn agent_approval_is_emitted_as_missing() {
    let mut input = fixture();
    input.approvals = vec![(
        "developer".to_owned(),
        ApprovalDecision::Covered {
            individual: "agent-identity".to_owned(),
            is_human: false,
            statement: json!({"role":"developer"}),
        },
    )];
    let predicate = build_change_record(input)
        .expect("failed predicate builds")
        .statement
        .predicate;
    assert_eq!(predicate.status, Status::Failed);
    assert_eq!(predicate.approvals[0].verdict, ApprovalVerdict::Missing);
    assert_eq!(predicate.approvals[0].statement_digest, None);
    assert!(predicate.failures.contains(&Failure::ApprovalsIncomplete));
}

#[trace("TC-576", "FR-072-AC-7")]
#[test]
fn canonical_digests_bind_each_document_and_private_form_change() {
    let input = fixture();
    let review_bytes = input
        .reviews
        .iter()
        .map(|review| review.artifact_bytes.clone())
        .collect::<Vec<_>>();
    let approvals = input
        .approvals
        .iter()
        .map(|(_, decision)| match decision {
            ApprovalDecision::Covered { statement, .. } => statement.clone(),
            _ => panic!("fixture approval is covered"),
        })
        .collect::<Vec<_>>();
    let test_results = input.test_results.clone();
    let mut record = build_change_record(input).expect("fixture builds");
    let predicate = &record.statement.predicate;
    let stored_approvals = record
        .private_form
        .statement
        .as_ref()
        .and_then(Value::as_array)
        .expect("private approval statements");
    assert_eq!(stored_approvals, &approvals);
    for (approval, entry) in approvals.iter().zip(&predicate.approvals) {
        assert_eq!(
            entry.statement_digest.as_deref(),
            Some(
                canonical_digest(approval)
                    .expect("approval digest")
                    .as_str()
            )
        );
    }
    assert_eq!(
        predicate.test_results,
        canonical_digest(&test_results).expect("test results digest")
    );
    let bom = record.private_form.agent_bom.as_ref().expect("private BOM");
    assert_eq!(
        predicate.agent_bom.as_ref(),
        Some(&canonical_digest(bom).expect("BOM digest"))
    );
    for (review, bytes) in predicate.reviews.iter().zip(review_bytes) {
        assert_eq!(review.artifact_digest, raw_sha256(&bytes));
    }
    assert!(
        record
            .private_form
            .change_record
            .as_ref()
            .expect("private change record")
            .get("private_form")
            .is_none()
    );
    let private_form_json = serde_json::to_value(&record.private_form).expect("private form JSON");
    assert_eq!(
        predicate.private_form,
        canonical_digest(&private_form_json).expect("private form digest")
    );

    let original = record.private_form.clone();
    record.private_form.ticket_text = Some("Fictional ticket titlf".to_owned());
    assert!(!verify_private_form_digest(predicate, &record.private_form));
    assert!(verify_private_form_digest(predicate, &original));
}

fn raw_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(bytes))
}

#[trace("TC-574", "FR-072-AC-5")]
#[test]
fn statement_subject_names_squash_commit_and_merged_tree() {
    let record = build_change_record(fixture()).expect("fixture builds");
    let statement = record.statement;
    assert_eq!(statement.subject.len(), 1);
    assert_eq!(statement.subject[0].name, "fictional/project");
    assert_eq!(statement.subject[0].digest["gitCommit"], "a".repeat(40));
    assert_eq!(statement.subject[0].digest["gitTree"], "b".repeat(40));
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
    assert_eq!(bom.bom_format, "CycloneDX");
    assert_eq!(
        bom.components
            .iter()
            .filter(|component| component.component_type
                == engineering_assurance::change_record::bom::ComponentType::Application)
            .count(),
        1
    );
    assert_eq!(
        bom.components
            .iter()
            .filter(|component| component.component_type
                == engineering_assurance::change_record::bom::ComponentType::MachineLearningModel)
            .count(),
        1
    );
    assert!(
        bom.components
            .iter()
            .any(|component| component.purl.is_some())
    );
    assert_eq!(bom.services.len(), 2);
    let service_names = bom
        .services
        .iter()
        .map(|service| service.name.as_str())
        .collect::<Vec<_>>();
    assert!(service_names.contains(&"Private Model Name"));
    assert!(service_names.contains(&"Fictional MCP"));
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

#[trace("TC-580", "FR-073-AC-4")]
#[test]
fn built_record_binds_private_bom_and_keeps_model_name_out_of_public_bytes() {
    let record = build_change_record(fixture()).expect("record builds");
    let predicate = &record.statement.predicate;
    assert_eq!(record.private_form.record_id.as_str(), predicate.record_id);
    let private_bom = record
        .private_form
        .agent_bom
        .as_ref()
        .expect("private BOM stored");
    assert_eq!(
        predicate.agent_bom.as_deref(),
        Some(
            canonical_digest(private_bom)
                .expect("digest private BOM")
                .as_str()
        )
    );
    assert!(validate_bom_value(private_bom).is_ok());
    let public_bytes = serde_json::to_vec(&record.statement).expect("public statement bytes");
    assert!(!String::from_utf8_lossy(&public_bytes).contains("Private Model Name"));
}

#[trace("TC-581", "FR-073-AC-5")]
#[test]
fn missing_session_emits_no_bom_and_failed_contract_missing_receipt() {
    let mut input = fixture();
    input.session = None;
    let record = build_change_record(input).expect("missing session is a failed receipt");
    assert!(record.private_form.agent_bom.is_none());
    assert!(record.statement.predicate.agent_bom.is_none());
    assert_eq!(record.statement.predicate.status, Status::Failed);
    assert!(
        record
            .statement
            .predicate
            .failures
            .contains(&Failure::ContractMissing)
    );
}

#[trace("FR-073-AC-4")]
#[test]
fn builder_rejects_unrelated_bom_material_in_private_form() {
    let mut input = fixture();
    input.private_form.agent_bom = Some(json!({"bomFormat":"CycloneDX", "specVersion":"1.7"}));
    assert!(build_change_record(input).is_err());
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

#[trace("TC-582", "FR-073-AC-6")]
#[test]
fn bom_random_serial_changes_canonical_digest() {
    let first =
        serde_json::to_value(build_bom(&session()).expect("first BOM")).expect("first JSON");
    let second =
        serde_json::to_value(build_bom(&session()).expect("second BOM")).expect("second JSON");
    assert_ne!(first["serialNumber"], second["serialNumber"]);
    let first_uuid = first["serialNumber"]
        .as_str()
        .expect("serial number")
        .strip_prefix("urn:uuid:")
        .expect("UUID URN");
    assert_eq!(
        uuid::Uuid::parse_str(first_uuid)
            .expect("valid UUID")
            .get_version_num(),
        4
    );
    assert_ne!(
        engineering_assurance::change_record::canonical_digest(&first).expect("first digest"),
        engineering_assurance::change_record::canonical_digest(&second).expect("second digest"),
    );
}
