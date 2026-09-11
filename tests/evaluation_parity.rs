// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Native Rust coverage and adverse cases for evaluation aggregation.

use engineering_assurance::{
    evaluation::{
        EvaluationEnvelope, EvaluationHost, EvaluationScenario, ExecutionStatus, MAX_REQUEST_BYTES,
        REQUEST_PROTOCOL, aggregate_evaluations, evaluate_request_bytes, required_matrix,
    },
    evidence::{GoverningVersions, VersionIdentity},
    workflow::{DecisionChoice, DecisionEvent},
};
use ix_trace_rs::trace;
use serde_json::json;

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SOURCE_REVISION: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn identity(name: &str) -> VersionIdentity {
    VersionIdentity {
        name: name.to_owned(),
        version: "1.2.3".to_owned(),
        digest: DIGEST.to_owned(),
    }
}

fn governing(scenario: EvaluationScenario) -> GoverningVersions {
    let workflow = match scenario {
        EvaluationScenario::InterruptionResume
        | EvaluationScenario::HumanAcceptance
        | EvaluationScenario::HumanRejection => "architecture-evaluation",
        EvaluationScenario::ExistingProfile
        | EvaluationScenario::NoProfile
        | EvaluationScenario::MalformedProducer
        | EvaluationScenario::UnavailableProducer => "assurance-intake",
    };
    GoverningVersions {
        module: identity("engineering-assurance"),
        plugin: identity("engineering-assurance-plugin"),
        skill: identity("assurance-onboarding"),
        workflow: identity(workflow),
        quire: identity("quire"),
        quoin: identity("quoin"),
        ix_flow: identity("ix-flow"),
        schema: identity("evaluation-envelope"),
        producer: identity("cli-agent-evals"),
    }
}

fn terminal(host: EvaluationHost, scenario: EvaluationScenario) -> Option<DecisionEvent> {
    let choice = match scenario {
        EvaluationScenario::HumanAcceptance => DecisionChoice::Accept,
        EvaluationScenario::HumanRejection => DecisionChoice::Reject,
        EvaluationScenario::ExistingProfile
        | EvaluationScenario::NoProfile
        | EvaluationScenario::MalformedProducer
        | EvaluationScenario::UnavailableProducer
        | EvaluationScenario::InterruptionResume => return None,
    };
    Some(DecisionEvent {
        run_id: format!("{}-{}-run", host.as_str(), choice.as_str()),
        workflow: "architecture-evaluation".to_owned(),
        workflow_version: "1.2.3".to_owned(),
        owner: "architecture-owner".to_owned(),
        choice,
        outcome: scenario.expected_outcome().to_owned(),
        timestamp: "2026-08-30T00:00:00Z".to_owned(),
    })
}

fn envelope(host: EvaluationHost, scenario: EvaluationScenario) -> EvaluationEnvelope {
    EvaluationEnvelope {
        host,
        scenario,
        execution_status: ExecutionStatus::Executed,
        passed: true,
        suite_revision: "suite-v1".to_owned(),
        fixture_revision: "fixtures-v1".to_owned(),
        source_revision: SOURCE_REVISION.to_owned(),
        host_version: Some("1.2.3".to_owned()),
        governing: Some(governing(scenario)),
        transcript_path: Some(format!(
            "transcripts/{}-{}.jsonl",
            host.as_str(),
            scenario.as_str()
        )),
        transcript_digest: Some(DIGEST.to_owned()),
        command_count: Some(4),
        elapsed_ms: Some(1_200),
        human_prompt_count: Some(i64::from(terminal(host, scenario).is_some())),
        manual_translation_count: Some(0),
        repeated_prompt_count: Some(0),
        observed_outcome: Some(scenario.expected_outcome().to_owned()),
        terminal_event: terminal(host, scenario),
        unsupported_additions: Vec::new(),
        diagnostic: None,
    }
}

fn complete_matrix() -> Vec<EvaluationEnvelope> {
    required_matrix()
        .into_iter()
        .map(|cell| envelope(cell.host, cell.scenario))
        .collect()
}

fn changed(
    source: &[EvaluationEnvelope],
    index: usize,
    change: impl FnOnce(&mut EvaluationEnvelope),
) -> Vec<EvaluationEnvelope> {
    let mut result = source.to_vec();
    change(&mut result[index]);
    result
}

#[test]
#[trace("TC-110", "FR-017-AC-2", "FR-017-CON-1", "FR-017-CON-3")]
#[trace("TC-033", "FR-006-AC-3")]
#[trace("TC-034", "FR-006-AC-4")]
fn complete_matrix_preserves_declared_aggregation_outcomes() {
    let complete = complete_matrix();
    let mut missing = complete.clone();
    missing.pop();
    let mut duplicate = complete.clone();
    duplicate.push(duplicate[0].clone());
    let failed = changed(&complete, 0, |item| item.passed = false);
    let drifted = changed(&complete, 0, |item| {
        item.source_revision = "c".repeat(40);
    });
    let unavailable = changed(&complete, 0, |item| {
        item.execution_status = ExecutionStatus::NotExecuted;
        item.passed = false;
        item.diagnostic = Some("executable-not-found".to_owned());
    });
    let complete_result = aggregate_evaluations(&complete);
    assert_eq!(complete_result.required_cells, 28);
    assert_eq!(complete_result.complete_cells, 28);
    assert!(complete_result.ok);
    assert!(complete_result.failures.is_empty());

    let cases = [
        (missing, 27, "missing:copilot:human-rejection"),
        (duplicate, 27, "duplicate:claude:existing-profile"),
        (failed, 27, "scenario-failed"),
        (drifted, 28, "matrix-source-revision-mismatch"),
        (unavailable, 27, "scenario-not-executed"),
    ];
    for (matrix, complete_cells, failure) in cases {
        let result = aggregate_evaluations(&matrix);
        assert!(!result.ok, "{failure} must withhold aggregation");
        assert_eq!(result.required_cells, 28);
        assert_eq!(result.complete_cells, complete_cells);
        assert!(
            result
                .failures
                .iter()
                .any(|item| item.to_string().contains(failure)),
            "{failure} absent from {:?}",
            result.failures
        );
    }
}

#[test]
#[trace("TC-110", "FR-017-AC-2", "FR-017-CON-1", "FR-017-CON-3")]
fn every_incomplete_semantic_class_withholds_aggregation() {
    let complete = complete_matrix();
    assert!(aggregate_evaluations(&complete).ok);

    let cases = [
        changed(&complete, 0, |item| {
            item.source_revision = "main".to_owned();
        }),
        changed(&complete, 0, |item| {
            item.governing
                .as_mut()
                .expect("fixture governing")
                .module
                .version = "1.2.4".to_owned();
        }),
        changed(&complete, 0, |item| {
            item.governing
                .as_mut()
                .expect("fixture governing")
                .workflow
                .version = "1.2.4".to_owned();
        }),
        changed(&complete, 0, |item| {
            item.unsupported_additions.push("invented-check".to_owned());
        }),
        changed(&complete, 0, |item| {
            item.transcript_path = Some("../escape.jsonl".to_owned());
        }),
        changed(&complete, 0, |item| {
            item.transcript_path = Some("..\\escape.jsonl".to_owned());
        }),
        changed(&complete, 0, |item| {
            item.transcript_path = Some("C:/escape.jsonl".to_owned());
        }),
        changed(&complete, 0, |item| item.command_count = Some(-1)),
        changed(&complete, 0, |item| {
            item.host_version = Some("nightly".to_owned());
        }),
        changed(&complete, 0, |item| {
            item.source_revision = "release-1.x".to_owned();
        }),
        changed(&complete, 0, |item| {
            item.observed_outcome = Some("invented".to_owned());
        }),
        changed(&complete, 0, |item| item.passed = false),
        changed(&complete, 0, |item| {
            item.execution_status = ExecutionStatus::NotExecuted;
            item.passed = false;
            item.diagnostic = Some("executable-not-found".to_owned());
        }),
        changed(&complete, 5, |item| {
            item.terminal_event
                .as_mut()
                .expect("decision fixture")
                .run_id = "claude-reject-run".to_owned();
        }),
        changed(&complete, 5, |item| item.terminal_event = None),
        changed(&complete, 5, |item| {
            item.terminal_event
                .as_mut()
                .expect("decision fixture")
                .workflow_version = "9.9.9".to_owned();
        }),
    ];

    let expected_codes = [
        "source-revision-not-immutable",
        "matrix-governing-versions-mismatch",
        "workflow-version-mismatch",
        "unsupported-assurance-addition",
        "transcript-path-invalid",
        "transcript-path-invalid",
        "transcript-path-invalid",
        "command-count-invalid",
        "host-version-not-immutable",
        "source-revision-not-immutable",
        "observed-outcome-mismatch",
        "scenario-failed",
        "scenario-not-executed",
        "terminal-pair-invalid",
        "terminal-event-missing",
        "terminal-event-invalid",
    ];
    for (case, expected) in cases.iter().zip(expected_codes) {
        let result = aggregate_evaluations(case);
        assert!(!result.ok, "{expected} case must withhold");
        assert!(
            result
                .failures
                .iter()
                .any(|failure| failure.to_string().contains(expected)),
            "{expected} absent from {:?}",
            result.failures
        );
    }
}

#[test]
#[trace("TC-110", "FR-017-AC-2", "FR-017-CON-1", "FR-017-CON-3")]
fn missing_duplicate_and_input_permutation_are_deterministic() {
    let complete = complete_matrix();
    let mut invalid = complete[1..].to_vec();
    invalid.push(complete[1].clone());
    invalid[1].passed = false;
    let expected = aggregate_evaluations(&invalid);
    assert!(!expected.ok);
    assert_eq!(
        expected.failures.first().map(ToString::to_string),
        Some("missing:claude:existing-profile".to_owned())
    );
    assert!(
        expected
            .failures
            .iter()
            .any(|failure| failure.to_string() == "duplicate:claude:no-profile")
    );

    invalid.reverse();
    assert_eq!(aggregate_evaluations(&invalid), expected);
}

#[test]
#[trace("TC-110", "FR-017-AC-2", "FR-017-CON-1", "FR-017-CON-3")]
fn closed_request_refuses_malformed_unsupported_or_open_input() {
    let valid = json!({
        "protocol": REQUEST_PROTOCOL,
        "envelopes": complete_matrix(),
    });
    let result = evaluate_request_bytes(
        &serde_json::to_vec(&valid).expect("valid request fixture must serialize"),
    )
    .expect("valid request must aggregate");
    assert!(result.ok);
    assert_eq!(result.required_cells, 28);

    let mut wrong_protocol = valid.clone();
    wrong_protocol["protocol"] = json!("engineering-assurance.evaluation-aggregate-request/v2");
    let error = evaluate_request_bytes(
        &serde_json::to_vec(&wrong_protocol).expect("fixture must serialize"),
    )
    .expect_err("unknown protocol must refuse");
    assert_eq!(error.code(), "evaluation_aggregate_protocol_unsupported");

    let mut open = valid.clone();
    open["envelopes"][0]["invented"] = json!(true);
    let mut invalid_scalar = valid.clone();
    invalid_scalar["envelopes"][0]["command_count"] = json!("four");
    let mut unsupported_scenario = valid.clone();
    unsupported_scenario["envelopes"][0]["scenario"] = json!("invented");
    let mut open_terminal = valid;
    open_terminal["envelopes"][5]["scenario"] = json!("human-acceptance");
    open_terminal["envelopes"][5]["terminal_event"] = json!({
        "run_id": "claude-accept-run",
        "workflow": "architecture-evaluation",
        "workflow_version": "1.2.3",
        "owner": "architecture-owner",
        "choice": "accept",
        "outcome": "accepted",
        "timestamp": "2026-08-30T00:00:00Z",
        "invented": true,
    });
    for malformed in [open, invalid_scalar, unsupported_scenario, open_terminal] {
        let error = evaluate_request_bytes(
            &serde_json::to_vec(&malformed).expect("fixture must serialize"),
        )
        .expect_err("malformed request must refuse");
        assert_eq!(error.code(), "evaluation_aggregate_request_invalid");
    }

    let unsupported_host = json!({
        "protocol": REQUEST_PROTOCOL,
        "envelopes": [{
            "host": "invented",
            "scenario": "existing-profile",
            "execution_status": "not_executed",
            "passed": false,
            "suite_revision": "suite-v1",
            "fixture_revision": "fixtures-v1",
            "source_revision": SOURCE_REVISION,
            "host_version": null,
            "governing": null,
            "transcript_path": null,
            "transcript_digest": null,
            "command_count": null,
            "elapsed_ms": null,
            "human_prompt_count": null,
            "manual_translation_count": null,
            "repeated_prompt_count": null,
            "observed_outcome": null,
            "terminal_event": null,
            "unsupported_additions": [],
            "diagnostic": "unsupported",
        }],
    });
    let error = evaluate_request_bytes(
        &serde_json::to_vec(&unsupported_host).expect("fixture must serialize"),
    )
    .expect_err("unsupported host must refuse");
    assert_eq!(error.code(), "evaluation_aggregate_request_invalid");

    let oversized = vec![b' '; MAX_REQUEST_BYTES + 1];
    let error = evaluate_request_bytes(&oversized).expect_err("oversized input must refuse");
    assert_eq!(error.code(), "evaluation_aggregate_request_too_large");
}

#[test]
#[trace("TC-032", "FR-006-AC-2")]
fn tc_032_an_executed_scenario_without_a_complete_observation_is_refused() {
    // An executed scenario is only re-checkable if every observation named by
    // the envelope contract actually reached the record. Each case below blanks
    // exactly one required observation on an otherwise complete matrix and
    // expects that field's own failure code, so a validator that stopped
    // requiring one field cannot hide behind the failures raised for the
    // others.
    let complete = complete_matrix();
    let cases = [
        (
            changed(&complete, 0, |item| item.governing = None),
            "governing-versions-missing",
        ),
        (
            changed(&complete, 0, |item| item.host_version = None),
            "host-version-not-immutable",
        ),
        (
            changed(&complete, 0, |item| item.transcript_path = None),
            "transcript-path-invalid",
        ),
        (
            changed(&complete, 0, |item| item.transcript_digest = None),
            "transcript-digest-invalid",
        ),
        (
            changed(&complete, 0, |item| item.command_count = None),
            "command-count-invalid",
        ),
        (
            changed(&complete, 0, |item| item.elapsed_ms = None),
            "elapsed-count-invalid",
        ),
        (
            changed(&complete, 0, |item| item.human_prompt_count = None),
            "human-prompt-count-invalid",
        ),
        (
            changed(&complete, 0, |item| item.manual_translation_count = None),
            "manual-translation-count-invalid",
        ),
        (
            changed(&complete, 0, |item| item.repeated_prompt_count = None),
            "repeated-prompt-count-invalid",
        ),
        (
            changed(&complete, 0, |item| item.observed_outcome = None),
            "observed-outcome-mismatch",
        ),
        (
            changed(&complete, 0, |item| {
                item.fixture_revision = "latest".to_owned();
            }),
            "fixture-revision-not-immutable",
        ),
        (
            changed(&complete, 5, |item| item.terminal_event = None),
            "terminal-event-missing",
        ),
    ];
    for (matrix, expected) in cases {
        let result = aggregate_evaluations(&matrix);
        assert!(!result.ok, "{expected} must withhold aggregation");
        assert!(
            result
                .failures
                .iter()
                .any(|failure| failure.to_string().ends_with(expected)),
            "{expected} absent from {:?}",
            result.failures
        );
    }
}

#[test]
#[trace("TC-049", "FR-006-AC-5")]
fn tc_049_every_host_retains_one_distinct_explicit_acceptance_and_rejection() {
    // The two terminal scenarios are the pair a reader uses to tell a real
    // human decision from a default. The required population is checked first
    // so a host cannot be evaluated for acceptance alone, then each way the
    // pair can stop being two distinct attributed decisions is checked to
    // withhold: a shared run identity, an absent decision, and a decision
    // attached to a scenario that reaches no terminal state at all.
    for host in EvaluationHost::ALL {
        let terminal = required_matrix()
            .into_iter()
            .filter(|cell| cell.host == host)
            .filter_map(|cell| match cell.scenario {
                EvaluationScenario::HumanAcceptance | EvaluationScenario::HumanRejection => {
                    Some(cell.scenario)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            terminal,
            [
                EvaluationScenario::HumanAcceptance,
                EvaluationScenario::HumanRejection
            ],
            "{} must be evaluated for both terminal choices",
            host.as_str()
        );
    }

    let complete = complete_matrix();
    assert!(aggregate_evaluations(&complete).ok);
    let acceptance = complete
        .iter()
        .position(|item| item.scenario == EvaluationScenario::HumanAcceptance)
        .expect("the required matrix evaluates explicit acceptance");
    let rejection = complete
        .iter()
        .position(|item| item.scenario == EvaluationScenario::HumanRejection)
        .expect("the required matrix evaluates explicit rejection");
    let shared_run = changed(&complete, rejection, |item| {
        item.terminal_event
            .as_mut()
            .expect("the rejection fixture carries a decision")
            .run_id = "claude-accept-run".to_owned();
    });
    let absent = changed(&complete, acceptance, |item| item.terminal_event = None);
    let inferred = changed(&complete, 0, |item| {
        item.terminal_event = complete[acceptance].terminal_event.clone();
    });

    for (matrix, expected) in [
        (shared_run, "terminal-pair-invalid"),
        (absent, "terminal-event-missing"),
        (inferred, "unexpected-terminal-event"),
    ] {
        let result = aggregate_evaluations(&matrix);
        assert!(!result.ok, "{expected} must withhold aggregation");
        assert!(
            result
                .failures
                .iter()
                .any(|failure| failure.to_string().ends_with(expected)),
            "{expected} absent from {:?}",
            result.failures
        );
    }
}
