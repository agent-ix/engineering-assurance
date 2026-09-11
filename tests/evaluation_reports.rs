// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Closed-decoder and relationship tests for retained cli-agent-evals reports.

use engineering_assurance::{
    evaluation::{EvaluationHost, EvaluationScenario},
    evaluation_reports::{
        CLI_REPORT_PROTOCOL, DecodedEvaluationSample, MAX_CLI_REPORT_BYTES, MAX_CLI_REPORT_RESULTS,
        RUNNER_DEFAULT_MODEL, decode_cli_eval_report,
    },
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SOURCE_REVISION: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn identity(name: &str) -> Value {
    json!({"name": name, "version": "1.2.3", "digest": DIGEST})
}

fn governing() -> Value {
    json!({
        "module": identity("engineering-assurance"),
        "plugin": identity("engineering-assurance-plugin"),
        "skill": identity("assurance-onboarding"),
        "workflow": identity("assurance-intake"),
        "quire": identity("quire"),
        "quoin": identity("quoin"),
        "ix_flow": identity("ix-flow"),
        "schema": identity("evaluation-envelope"),
        "producer": identity("cli-agent-evals")
    })
}

fn evaluation_result() -> Value {
    json!({
        "host": "codex",
        "host_version": "1.2.3",
        "source_revision": SOURCE_REVISION,
        "suite_revision": "suite-v1",
        "fixture_revision": "fixtures-v1",
        "governing": governing(),
        "command_count": 4,
        "elapsed_ms": 1000,
        "human_prompt_count": 0,
        "manual_translation_count": 0,
        "repeated_prompt_count": 0,
        "observed_outcome": "reused",
        "terminal_event": null,
        "unsupported_additions": []
    })
}

fn sample(ok: bool) -> Value {
    let mut value = json!({
        "ok": ok,
        "latencyMs": 1200,
        "exitReason": if ok { "complete" } else { "timeout" },
        "metricStatus": "available",
        "tokenUsage": {
            "input": 1,
            "output": 2,
            "cacheCreation": 0,
            "cacheRead": 0,
            "contextInput": 1,
            "total": 3
        },
        "toolCalls": 1,
        "toolBreakdown": {"Read": 1},
        "classified": {"assistantTurns": 1},
        "checks": if ok { json!({"evaluation_result": evaluation_result()}) } else { json!({}) },
        "failures": if ok { json!([]) } else { json!(["result envelope unreadable"]) },
        "workDir": "/workspace/codex-existing-profile",
        "sessionId": "session-1",
        "transcriptDigest": DIGEST,
        "transcriptRetention": if ok { "retained" } else { "not-retained" },
        "transcriptPath": ".cli-agent-evals/transcripts/sample.transcript"
    });
    if !ok {
        value
            .as_object_mut()
            .expect("sample fixture must be an object")
            .remove("transcriptPath");
    }
    value
}

fn report(ok: bool) -> Value {
    json!({
        "reportVersion": CLI_REPORT_PROTOCOL,
        "ok": ok,
        "generatedAt": "2026-09-10T00:00:00Z",
        "suite": "engineering-assurance-onboarding",
        "agent": "codex",
        "repeats": 1,
        "results": [{
            "id": "EA-001",
            "useCase": "existing-profile",
            "ok": ok,
            "passRate": if ok { "1/1" } else { "0/1" },
            "aggregate": {"latencyMs": {"p50": 1200, "p95": 1200}},
            "runs": [sample(ok)]
        }],
        "aggregates": {"successRate": if ok { "1/1" } else { "0/1" }}
    })
}

fn decode(
    value: &Value,
) -> Result<engineering_assurance::evaluation_reports::DecodedEvaluationReport, String> {
    decode_cli_eval_report(
        &serde_json::to_vec(value).expect("fixture must encode"),
        SOURCE_REVISION,
    )
    .map_err(|error| error.code().to_owned())
}

#[test]
#[trace("TC-129", "FR-017-AC-1", "FR-017-CON-1", "FR-017-CON-3")]
fn tc_129_decodes_one_retained_success_into_a_typed_envelope() {
    let decoded = decode(&report(true)).expect("valid retained report must decode");

    assert_eq!(decoded.host(), EvaluationHost::Codex);
    assert_eq!(decoded.model(), RUNNER_DEFAULT_MODEL);
    let [
        DecodedEvaluationSample::Retained {
            work_dir,
            transcript_path,
            transcript_digest,
            envelope,
        },
    ] = decoded.samples()
    else {
        panic!("one retained sample is required")
    };
    assert_eq!(work_dir, "/workspace/codex-existing-profile");
    assert_eq!(
        transcript_path,
        ".cli-agent-evals/transcripts/sample.transcript"
    );
    assert_eq!(transcript_digest, DIGEST);
    assert_eq!(envelope.host, EvaluationHost::Codex);
    assert_eq!(envelope.scenario, EvaluationScenario::ExistingProfile);
    assert!(envelope.errors().is_empty());
}

#[test]
#[trace("TC-129", "FR-017-AC-1", "FR-017-CON-1")]
fn tc_129_preserves_failed_attempts_without_admitting_envelopes() {
    let decoded = decode(&report(false)).expect("well-formed failure must decode");
    let [
        DecodedEvaluationSample::Failed {
            scenario,
            diagnostic,
        },
    ] = decoded.samples()
    else {
        panic!("one failed observation is required")
    };
    assert_eq!(*scenario, EvaluationScenario::ExistingProfile);
    assert_eq!(
        diagnostic,
        "codex:existing-profile:timeout:result envelope unreadable"
    );
}

#[test]
#[trace("TC-129", "FR-017-AC-1", "FR-017-CON-1")]
fn tc_129_rejects_open_or_malformed_nested_shapes() {
    let mut cases = Vec::new();
    let mut top = report(true);
    top["unexpected"] = json!(true);
    cases.push(top);
    let mut result = report(true);
    result["results"][0]["unexpected"] = json!(true);
    cases.push(result);
    let mut run = report(true);
    run["results"][0]["runs"][0]["unexpected"] = json!(true);
    cases.push(run);
    let mut usage = report(true);
    usage["results"][0]["runs"][0]["tokenUsage"]["unexpected"] = json!(true);
    cases.push(usage);
    let mut observation = report(true);
    observation["results"][0]["runs"][0]["checks"]["evaluation_result"]["unexpected"] = json!(true);
    cases.push(observation);
    let mut wrong_type = report(true);
    wrong_type["results"][0]["runs"][0]["tokenUsage"]["input"] = json!("1");
    cases.push(wrong_type);

    for value in cases {
        assert_eq!(decode(&value), Err("evaluation_report_invalid".to_owned()));
    }
}

#[test]
#[trace("TC-129", "FR-017-AC-1", "FR-017-CON-1")]
fn tc_129_rejects_version_outcome_and_identity_contradictions() {
    let mut version = report(true);
    version["reportVersion"] = json!("cli-agent-evals.report/v2");
    assert_eq!(
        decode(&version),
        Err("evaluation_report_protocol_unsupported".to_owned())
    );

    for (pointer, replacement) in [
        ("/ok", json!(false)),
        ("/results/0/ok", json!(false)),
        ("/results/0/passRate", json!("0/1")),
        (
            "/results/0/runs/0/checks/evaluation_result/host",
            json!("claude"),
        ),
        (
            "/results/0/runs/0/checks/evaluation_result/source_revision",
            json!("cccccccccccccccccccccccccccccccccccccccc"),
        ),
    ] {
        let mut value = report(true);
        *value
            .pointer_mut(pointer)
            .expect("fixture pointer must exist") = replacement;
        assert_eq!(
            decode(&value),
            Err("evaluation_report_contract_invalid".to_owned())
        );
    }
    let bytes = serde_json::to_vec(&report(true)).expect("fixture must encode");
    let error = decode_cli_eval_report(&bytes, "main").expect_err("mutable source must refuse");
    assert_eq!(error.code(), "evaluation_report_contract_invalid");
}

#[test]
#[trace("TC-129", "FR-017-AC-1", "FR-017-CON-1")]
fn tc_129_rejects_unsafe_or_contradictory_retention_identity() {
    for path in [
        "",
        "/absolute",
        "C:/absolute",
        "dir\\file",
        ".",
        "../escape",
        "a//b",
        "a/\u{7f}b",
    ] {
        let mut value = report(true);
        value["results"][0]["runs"][0]["transcriptPath"] = json!(path);
        assert!(
            decode(&value).is_err(),
            "unsafe path was accepted: {path:?}"
        );
    }

    let mut uppercase = report(true);
    uppercase["results"][0]["runs"][0]["transcriptDigest"] = json!("A".repeat(64));
    assert!(decode(&uppercase).is_err());
    let mut unavailable = report(true);
    unavailable["results"][0]["runs"][0]["transcriptRetention"] = json!("unavailable");
    assert!(decode(&unavailable).is_err());
    let mut null_path = report(true);
    null_path["results"][0]["runs"][0]["transcriptPath"] = Value::Null;
    assert_eq!(
        decode(&null_path),
        Err("evaluation_report_invalid".to_owned())
    );
    let mut missing_digest = report(true);
    missing_digest["results"][0]["runs"][0]
        .as_object_mut()
        .expect("sample fixture must be an object")
        .remove("transcriptDigest");
    assert_eq!(
        decode(&missing_digest),
        Err("evaluation_report_invalid".to_owned())
    );
}

#[test]
#[trace("TC-129", "FR-017-AC-1", "FR-017-CON-1")]
fn tc_129_enforces_predecode_and_result_population_ceilings() {
    let oversized = vec![b' '; MAX_CLI_REPORT_BYTES + 1];
    let error = decode_cli_eval_report(&oversized, SOURCE_REVISION)
        .expect_err("one byte beyond the report ceiling must refuse");
    assert_eq!(error.code(), "evaluation_report_too_large");

    let mut value = report(true);
    let result = value["results"][0].clone();
    value["results"] = Value::Array(vec![result; MAX_CLI_REPORT_RESULTS + 1]);
    let error = decode(&value).expect_err("one result beyond the ceiling must refuse");
    assert_eq!(error, "evaluation_report_result_limit");
}
