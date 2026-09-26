// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Additive-migration parity with the retained JavaScript invariant provider.

use std::{
    io::Write,
    process::{Command, Stdio},
};

use engineering_assurance::workflow_invariants::{
    InvariantName, REQUEST_PROTOCOL, evaluate_request_bytes,
};
use ix_trace_rs::trace;
use serde_json::{Map, Value, json};

const EVALUATED_AT: &str = "2026-09-10T12:00:00Z";
const REFERENCE: &str = r#"
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const request = JSON.parse(fs.readFileSync(0, "utf8"));
Date.now = () => Date.parse(request.evaluated_at);
const provider = pathToFileURL(path.resolve(
  "engineering_assurance/skills/assurance-onboarding/scripts/invariants.js",
)).href;
const { invariants } = await import(provider);
const outcomes = request.invariants.map((name) => {
  const verdict = invariants[name]({ instance: request.instance });
  return verdict === true
    ? { invariant: name, status: "passed" }
    : {
        invariant: name,
        status: "failed",
        code: verdict.code,
        details: verdict.details ?? {},
      };
});
process.stdout.write(JSON.stringify(outcomes));
"#;

fn canonical_names() -> Vec<&'static str> {
    InvariantName::ALL
        .into_iter()
        .map(InvariantName::as_str)
        .collect()
}

fn request(instance: &Value, invariants: &[&str]) -> Value {
    json!({
        "protocol": REQUEST_PROTOCOL,
        "invariants": invariants,
        "instance": instance,
        "evaluated_at": EVALUATED_AT,
    })
}

fn passing_projection() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/workflow-invariants/passing-projection.json"
    ))
    .expect("passing projection fixture must be valid JSON")
}

fn workflow_projection(
    def_name: &str,
    terminal_transitions: &[&str],
    item_kinds: &[&str],
) -> Value {
    let catalog = passing_projection();
    let catalog_items = catalog["items"]
        .as_object()
        .expect("fixture item catalog must be an object");
    let items = item_kinds
        .iter()
        .map(|kind| {
            (
                (*kind).to_owned(),
                catalog_items
                    .get(*kind)
                    .unwrap_or_else(|| panic!("fixture item kind {kind} must exist"))
                    .clone(),
            )
        })
        .collect::<Map<_, _>>();
    let gate_config = terminal_transitions
        .iter()
        .map(|transition| ((*transition).to_owned(), json!("hitl")))
        .collect::<Map<_, _>>();
    json!({"defName": def_name, "gateConfig": gate_config, "items": items})
}

fn reference_outcomes(request: &Value) -> Vec<u8> {
    let mut child = Command::new("node")
        .args(["--input-type=module", "-e", REFERENCE])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("retained JavaScript provider must start during additive migration");
    child
        .stdin
        .take()
        .expect("piped stdin must exist")
        .write_all(&serde_json::to_vec(request).expect("request fixture must serialize"))
        .expect("request fixture must be writable");
    let output = child.wait_with_output().expect("provider must terminate");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn assert_parity(request: &Value) {
    let rust = evaluate_request_bytes(
        &serde_json::to_vec(request).expect("request fixture must serialize"),
    )
    .expect("Rust must accept the shared valid request domain");
    let rust_bytes = serde_json::to_vec(&rust.outcomes).expect("outcomes must serialize");
    assert_eq!(rust_bytes, reference_outcomes(request));
}

fn assert_failed(request: &Value, expected_codes: &[&str]) {
    let result = evaluate_request_bytes(
        &serde_json::to_vec(request).expect("request fixture must serialize"),
    )
    .expect("incomplete domain state must produce outcomes");
    let outcomes = serde_json::to_value(result.outcomes).expect("outcomes must serialize");
    let codes = outcomes
        .as_array()
        .expect("outcomes must be an array")
        .iter()
        .map(|outcome| outcome["code"].as_str().expect("outcome must fail"))
        .collect::<Vec<_>>();
    assert_eq!(codes, expected_codes);
}

fn passing_workflow_cases() -> Vec<(Value, Vec<&'static str>)> {
    vec![
        (
            workflow_projection(
                "assurance-intake",
                &["decision_ready->accepted", "decision_ready->rejected"],
                &[
                    "intake_request",
                    "artifact_validation",
                    "exception",
                    "operator_observation",
                ],
            ),
            vec![
                "shared.observation_ready",
                "shared.terminal_gates",
                "shared.exceptions_ready",
                "intake.scope_ready",
                "intake.artifacts_ready",
            ],
        ),
        (
            workflow_projection(
                "architecture-evaluation",
                &["decision_ready->accepted", "decision_ready->rejected"],
                &[
                    "architecture_request",
                    "artifact_validation",
                    "architecture_scenario",
                    "review_validation",
                    "operator_observation",
                ],
            ),
            vec![
                "shared.observation_ready",
                "shared.terminal_gates",
                "shared.exceptions_ready",
                "architecture.scenarios_ready",
                "architecture.review_ready",
            ],
        ),
        (
            workflow_projection(
                "measurement-promotion",
                &["decision_ready->promoted", "decision_ready->not_promoted"],
                &[
                    "promotion_request",
                    "promotion_evidence",
                    "measurement_policy",
                    "measurement_verdict",
                    "operator_observation",
                ],
            ),
            vec![
                "shared.observation_ready",
                "shared.terminal_gates",
                "shared.exceptions_ready",
                "measurement.promotion_ready",
            ],
        ),
        (
            workflow_projection(
                "change-assurance",
                &["decision_ready->approved", "decision_ready->rejected"],
                &[
                    "change_request",
                    "impact_snapshot",
                    "assurance_snapshot",
                    "review_validation",
                    "operator_observation",
                ],
            ),
            vec![
                "shared.observation_ready",
                "shared.terminal_gates",
                "shared.exceptions_ready",
                "change.impact_ready",
                "change.snapshot_ready",
                "change.review_ready",
            ],
        ),
    ]
}

#[trace("TC-106", "FR-016-AC-2", "FR-016-CON-3")]
#[test]
fn tc_106_all_canonical_invariants_match_the_reference_in_request_order() {
    let cases = passing_workflow_cases();
    for (projection, names) in cases {
        assert_parity(&request(&projection, &names));
    }

    let change = workflow_projection(
        "change-assurance",
        &["decision_ready->approved", "decision_ready->rejected"],
        &[
            "change_request",
            "impact_snapshot",
            "assurance_snapshot",
            "review_validation",
            "operator_observation",
        ],
    );
    let mut ordered = vec![
        "shared.observation_ready",
        "shared.terminal_gates",
        "change.impact_ready",
        "change.snapshot_ready",
        "change.review_ready",
    ];
    ordered.reverse();
    assert_parity(&request(&change, &ordered));
}

#[trace("TC-106", "FR-016-AC-2", "FR-016-CON-3")]
#[test]
fn tc_106_failure_codes_and_time_boundaries_match_the_reference() {
    let empty = json!({"defName": "change-assurance", "items": {}});
    assert_parity(&request(&empty, &canonical_names()));

    let mut expired = workflow_projection(
        "assurance-intake",
        &["decision_ready->accepted", "decision_ready->rejected"],
        &["intake_request", "exception", "operator_observation"],
    );
    expired["items"]["exception"][0]["expires_at"] = json!(EVALUATED_AT);
    assert_parity(&request(&expired, &["shared.exceptions_ready"]));

    let mut future_snapshot = workflow_projection(
        "change-assurance",
        &["decision_ready->approved", "decision_ready->rejected"],
        &[
            "change_request",
            "assurance_snapshot",
            "operator_observation",
        ],
    );
    future_snapshot["items"]["assurance_snapshot"][0]["verified_at"] =
        json!("2026-09-10T12:00:01Z");
    assert_parity(&request(&future_snapshot, &["change.snapshot_ready"]));

    let selection_boundary = json!({
        "defName": "assurance-intake",
        "items": {
            "intake_request": [{"interviewId": "intake-1"}],
            "promotion_request": [{
                "interviewId": "promotion-1",
                "exceptions_expected": true,
            }],
        },
    });
    assert_parity(&request(&selection_boundary, &["shared.exceptions_ready"]));
}

#[trace("TC-106", "FR-016-AC-2", "FR-016-CON-3")]
#[test]
fn tc_106_absent_binding_subjects_cannot_match_absent_evidence_fields() {
    let mut architecture = passing_projection();
    architecture["defName"] = json!("architecture-evaluation");
    architecture["items"]
        .as_object_mut()
        .expect("items must be an object")
        .remove("architecture_request");
    architecture["items"]["artifact_validation"][0]
        .as_object_mut()
        .expect("validation must be an object")
        .remove("path");
    architecture["items"]["review_validation"][0]
        .as_object_mut()
        .expect("review must be an object")
        .remove("subject_path");
    assert_failed(
        &request(
            &architecture,
            &["architecture.scenarios_ready", "architecture.review_ready"],
        ),
        &[
            "architecture_context_incomplete",
            "architecture_review_missing",
        ],
    );

    let mut change = passing_projection();
    change["items"]
        .as_object_mut()
        .expect("items must be an object")
        .remove("change_request");
    change["items"]["assurance_snapshot"][0]
        .as_object_mut()
        .expect("snapshot must be an object")
        .remove("source_revision");
    change["items"]["review_validation"][1]
        .as_object_mut()
        .expect("review must be an object")
        .remove("source_revision");
    assert_failed(
        &request(&change, &["change.snapshot_ready", "change.review_ready"]),
        &["assurance_snapshot_invalid", "code_review_missing"],
    );
}

#[trace("TC-106", "FR-016-AC-2", "FR-016-CON-3")]
#[test]
fn tc_106_malformed_unknown_and_duplicate_requests_refuse_before_outcomes() {
    let cases = [
        (
            json!({
                "protocol": REQUEST_PROTOCOL,
                "invariants": [],
                "instance": passing_projection(),
                "evaluated_at": EVALUATED_AT,
            }),
            "workflow_invariant_request_invalid",
        ),
        (
            request(
                &passing_projection(),
                &["shared.observation_ready", "shared.observation_ready"],
            ),
            "workflow_invariant_request_invalid",
        ),
        (
            request(&passing_projection(), &["shared.not_registered"]),
            "workflow_invariant_unknown",
        ),
        (
            json!({
                "protocol": REQUEST_PROTOCOL,
                "invariants": ["shared.observation_ready"],
                "instance": {"defName": "unknown-workflow", "items": {}},
                "evaluated_at": EVALUATED_AT,
            }),
            "workflow_binding_invalid",
        ),
        (
            json!({
                "protocol": REQUEST_PROTOCOL,
                "invariants": ["shared.observation_ready"],
                "instance": passing_projection(),
                "evaluated_at": "not-an-instant",
            }),
            "workflow_invariant_request_invalid",
        ),
    ];

    for (request, expected_code) in cases {
        let error = evaluate_request_bytes(
            &serde_json::to_vec(&request).expect("request fixture must serialize"),
        )
        .expect_err("invalid request must be refused");
        assert_eq!(error.code(), expected_code);
    }

    let extended = json!({
        "protocol": REQUEST_PROTOCOL,
        "invariants": ["shared.observation_ready"],
        "instance": passing_projection(),
        "evaluated_at": EVALUATED_AT,
        "unexpected": true,
    });
    assert!(evaluate_request_bytes(&serde_json::to_vec(&extended).unwrap()).is_err());
    assert!(evaluate_request_bytes(b"{not-json").is_err());
}

/// A measurement-promotion projection with complete evidence bound to plan
/// `MP-001`, definition `v1`, and candidate `collection-2`, proposing
/// `observe -> baseline`. The fixture's policy requires `baseline`; the
/// fixture's checker result accepts over an attested order.
fn promotion() -> Value {
    workflow_projection(
        "measurement-promotion",
        &["decision_ready->promoted", "decision_ready->not_promoted"],
        &[
            "promotion_request",
            "promotion_evidence",
            "measurement_policy",
            "measurement_verdict",
            "operator_observation",
        ],
    )
}

fn without(mut projection: Value, kind: &str) -> Value {
    projection["items"]
        .as_object_mut()
        .expect("items must be an object")
        .remove(kind);
    projection
}

fn with_verdict(mut projection: Value, member: &str, value: Value) -> Value {
    projection["items"]["measurement_verdict"][0][member] = value;
    projection
}

fn rejected(projection: Value) -> Value {
    let projection = with_verdict(projection, "verdict", json!("reject"));
    with_verdict(projection, "reasons", json!(["rule_not_met"]))
}

fn with_exception(mut projection: Value, expires_at: &str) -> Value {
    projection["items"]["exception"] = json!([{
        "owner": "assurance-owner",
        "expires_at": expires_at,
        "rationale": "owner accepts the rejected candidate for this gate",
        "impact": "the promoted stage rests on a rejected measurement",
    }]);
    projection
}

fn checker(status: &str, verdict: &Value, reasons: &[&str]) -> Value {
    json!({"status": status, "verdict": verdict, "reasons": reasons})
}

/// A passed promotion outcome: a pass carries no details on the wire.
fn passed() -> Value {
    json!([{
        "invariant": "measurement.promotion_ready",
        "status": "passed",
    }])
}

fn refused(code: &str, checker: &Value) -> Value {
    json!([{
        "invariant": "measurement.promotion_ready",
        "status": "failed",
        "code": code,
        "details": {"checker": checker},
    }])
}

/// Assert the Rust outcome equals `expected` and the retained JavaScript
/// provider produces the same bytes.
fn assert_promotion(case: &str, projection: &Value, expected: &Value) {
    let request = request(projection, &["measurement.promotion_ready"]);
    let rust = evaluate_request_bytes(
        &serde_json::to_vec(&request).expect("request fixture must serialize"),
    )
    .unwrap_or_else(|error| panic!("{case}: Rust must accept the request: {error}"));
    let outcomes = serde_json::to_value(&rust.outcomes).expect("outcomes must serialize");
    assert_eq!(&outcomes, expected, "{case}");
    assert_parity(&request);
}

#[trace("TC-160", "FR-025-AC-2", "FR-025-AC-5")]
#[test]
fn tc_160_recommend_mode_never_refuses() {
    let recommend = without(promotion(), "measurement_policy");
    let mut other_schema = recommend.clone();
    other_schema["items"]["measurement_verdict"][0]["schema"] =
        json!("quoin.measurement-verdict.v2");
    other_schema["items"]["measurement_verdict"][0]["addedInV2"] = json!(true);
    let mut unlisted = rejected(promotion());
    unlisted["items"]["measurement_policy"][0]["stages"] = json!(["gate"]);
    let mut recommend_policy = rejected(promotion());
    recommend_policy["items"]["measurement_policy"][0]["mode"] = json!("recommend");
    let cases = [
        (
            "no policy, no checker result",
            without(recommend.clone(), "measurement_verdict"),
        ),
        ("no policy, rejected", rejected(recommend.clone())),
        (
            "no policy, other definition version",
            with_verdict(recommend.clone(), "definitionVersion", json!("v0")),
        ),
        (
            "no policy, caller-supplied order",
            with_verdict(recommend.clone(), "orderSource", json!("caller-supplied")),
        ),
        // A later checker schema is ignored, never refused, even with members
        // v1 does not know.
        ("no policy, result of a later schema", other_schema),
        ("no policy, accepted", recommend),
        // A require policy that does not list the proposed stage is recommend.
        ("require policy, stage not listed, rejected", unlisted),
        (
            "recommend policy listing the stage, rejected",
            recommend_policy,
        ),
    ];
    for (case, projection) in cases {
        assert_promotion(case, &projection, &passed());
    }
}

/// Each `require` case that must refuse, with its expected outcome.
fn require_refusal_cases() -> Vec<(&'static str, Value, Value)> {
    let mut unbound = promotion();
    unbound["items"]["promotion_evidence"][0]
        .as_object_mut()
        .expect("evidence must be an object")
        .remove("plan_id");
    let mut other_plan = promotion();
    other_plan["items"]["promotion_evidence"][0]["plan_id"] = json!("MP-002");
    let missing = checker("missing", &Value::Null, &[]);
    let mut cases = vec![
        (
            "require, no checker result",
            without(promotion(), "measurement_verdict"),
            refused("promotion_checker_missing", &missing),
        ),
        (
            "require, evidence names no plan id",
            unbound,
            refused("promotion_checker_missing", &missing),
        ),
        (
            "require, result for another plan",
            other_plan,
            refused("promotion_checker_missing", &missing),
        ),
        (
            "require, result of another schema",
            with_verdict(promotion(), "schema", json!("quoin.measurement-verdict.v0")),
            refused("promotion_checker_missing", &missing),
        ),
    ];
    cases.extend(verdict_refusal_cases());
    cases
}

/// The `require` cases whose checker result exists for the plan.
fn verdict_refusal_cases() -> Vec<(&'static str, Value, Value)> {
    let mut no_candidate = promotion();
    no_candidate["items"]["promotion_evidence"][0]
        .as_object_mut()
        .expect("evidence must be an object")
        .remove("candidate");
    let mismatch = checker("mismatch", &Value::Null, &[]);
    let unattested = checker("order_unattested", &json!("accept"), &[]);
    vec![
        (
            "require, rejected",
            rejected(promotion()),
            refused(
                "promotion_checker_not_accepted",
                &checker("not_accepted", &json!("reject"), &["rule_not_met"]),
            ),
        ),
        (
            "require, inconclusive",
            with_verdict(
                with_verdict(promotion(), "verdict", json!("inconclusive")),
                "reasons",
                json!(["population_too_small"]),
            ),
            refused(
                "promotion_checker_not_accepted",
                &checker(
                    "not_accepted",
                    &json!("inconclusive"),
                    &["population_too_small"],
                ),
            ),
        ),
        (
            "require, other definition version",
            with_verdict(promotion(), "definitionVersion", json!("v0")),
            refused("promotion_checker_mismatch", &mismatch),
        ),
        (
            "require, other candidate collection",
            with_verdict(promotion(), "candidate", json!("collection-1")),
            refused("promotion_checker_mismatch", &mismatch),
        ),
        (
            "require, checker decided no candidate",
            with_verdict(promotion(), "candidate", Value::Null),
            refused("promotion_checker_mismatch", &mismatch),
        ),
        (
            "require, evidence names no candidate",
            no_candidate,
            refused("promotion_checker_mismatch", &mismatch),
        ),
        (
            "require, caller-supplied order",
            with_verdict(promotion(), "orderSource", json!("caller-supplied")),
            refused("promotion_checker_order_unattested", &unattested),
        ),
        (
            "require, no order",
            with_verdict(promotion(), "orderSource", json!("none")),
            refused("promotion_checker_order_unattested", &unattested),
        ),
        (
            "require, shallow-clone order",
            with_verdict(promotion(), "orderSource", json!("git-shallow")),
            refused("promotion_checker_order_unattested", &unattested),
        ),
    ]
}

#[trace("TC-161", "FR-025-AC-3", "FR-025-AC-5")]
#[test]
fn tc_161_require_mode_refuses_without_an_attested_accept() {
    assert_promotion(
        "require, accepted over an attested order",
        &promotion(),
        &passed(),
    );

    for (case, projection, expected) in require_refusal_cases() {
        assert_promotion(case, &projection, &expected);
    }
}

#[trace("TC-161", "FR-025-AC-3", "FR-025-AC-5")]
#[test]
fn tc_161_conflicting_results_policies_and_stages_resolve_conservatively() {
    // Two matching results: the least favourable wins, whichever comes first.
    for order in [[0, 1], [1, 0]] {
        let accepted = promotion()["items"]["measurement_verdict"][0].clone();
        let reject = rejected(promotion())["items"]["measurement_verdict"][0].clone();
        let pair = [accepted, reject];
        let mut conflicting = promotion();
        conflicting["items"]["measurement_verdict"] =
            json!([pair[order[0]].clone(), pair[order[1]].clone()]);
        assert_promotion(
            "require, accepted and rejected results for one candidate",
            &conflicting,
            &refused(
                "promotion_checker_not_accepted",
                &checker("not_accepted", &json!("reject"), &["rule_not_met"]),
            ),
        );
    }

    // An unknown prior stage never counts as the stage before `observe`.
    let mut unknown_prior = promotion();
    unknown_prior["items"]["promotion_request"][0]["prior_stage"] = json!("draft");
    unknown_prior["items"]["promotion_request"][0]["proposed_stage"] = json!("observe");
    assert_promotion(
        "unknown prior stage",
        &unknown_prior,
        &json!([{
            "invariant": "measurement.promotion_ready",
            "status": "failed",
            "code": "promotion_must_advance_one_stage",
            "details": {},
        }]),
    );

    // A policy listing the stage wins over a recommend policy beside it.
    let mut both = promotion();
    both["items"]["measurement_policy"] = json!([
        {"mode": "recommend", "stages": ["baseline"]},
        {"mode": "require", "stages": ["baseline"]},
    ]);
    assert_promotion(
        "recommend and require policies, rejected",
        &rejected(both),
        &refused(
            "promotion_checker_not_accepted",
            &checker("not_accepted", &json!("reject"), &["rule_not_met"]),
        ),
    );
}

#[trace("TC-162", "FR-025-AC-4", "FR-025-AC-5")]
#[test]
fn tc_162_a_current_owned_exception_overrides_a_required_checker() {
    let refused_reject = refused(
        "promotion_checker_not_accepted",
        &checker("not_accepted", &json!("reject"), &["rule_not_met"]),
    );
    let cases = [
        (
            "require, rejected, current exception",
            with_exception(rejected(promotion()), "2026-09-10T12:00:01Z"),
            passed(),
        ),
        (
            "require, no checker result, current exception",
            with_exception(
                without(promotion(), "measurement_verdict"),
                "2026-09-10T12:00:01Z",
            ),
            passed(),
        ),
        (
            "require, rejected, exception expiring at the evaluation instant",
            with_exception(rejected(promotion()), EVALUATED_AT),
            refused_reject.clone(),
        ),
    ];
    for (case, projection, expected) in cases {
        assert_promotion(case, &projection, &expected);
    }
    let mut ownerless = with_exception(rejected(promotion()), "2026-09-10T12:00:01Z");
    ownerless["items"]["exception"][0]["owner"] = json!(" ");
    assert_promotion(
        "require, rejected, exception without an owner",
        &ownerless,
        &refused_reject,
    );
}

#[trace("TC-162", "FR-025-AC-5")]
#[test]
fn tc_162_malformed_checker_and_policy_items_are_refused_before_outcomes() {
    for (kind, member, value) in [
        ("measurement_verdict", "verdict", json!("accepted")),
        ("measurement_verdict", "reasons", json!("rule_not_met")),
        ("measurement_verdict", "unexpected", json!(true)),
        ("measurement_policy", "mode", json!("enforced")),
        ("measurement_policy", "stages", json!("baseline")),
        ("measurement_policy", "unexpected", json!(true)),
    ] {
        let mut projection = promotion();
        projection["items"][kind][0][member] = value;
        let error = evaluate_request_bytes(
            &serde_json::to_vec(&request(&projection, &["measurement.promotion_ready"]))
                .expect("request fixture must serialize"),
        )
        .expect_err("a malformed checker or policy item must be refused");
        assert_eq!(
            error.code(),
            "workflow_invariant_request_invalid",
            "{kind}.{member}"
        );
    }
    let mut scalar = promotion();
    scalar["items"]["measurement_verdict"][0] = json!("quoin.measurement-verdict.v1");
    assert!(
        evaluate_request_bytes(
            &serde_json::to_vec(&request(&scalar, &["measurement.promotion_ready"]))
                .expect("request fixture must serialize"),
        )
        .is_err(),
        "a non-object checker item must be refused"
    );
}

/// The single outcome of `invariant` over `projection`, after asserting the
/// retained JavaScript provider produces the same bytes.
fn sole_outcome(projection: &Value, invariant: &str) -> Value {
    let request = request(projection, &[invariant]);
    let rust = evaluate_request_bytes(
        &serde_json::to_vec(&request).expect("request fixture must serialize"),
    )
    .unwrap_or_else(|error| panic!("{invariant}: Rust must accept the request: {error}"));
    assert_parity(&request);
    let outcomes = serde_json::to_value(&rust.outcomes).expect("outcomes must serialize");
    let [outcome] = outcomes
        .as_array()
        .expect("outcomes must be an array")
        .as_slice()
    else {
        panic!("{invariant}: exactly one outcome expected, got {outcomes}");
    };
    outcome.clone()
}

/// Assert `invariant` passes over `projection`, the unbroken control a
/// refusal case is measured against.
fn assert_binding_holds(case: &str, projection: &Value, invariant: &str) {
    let outcome = sole_outcome(projection, invariant);
    assert_eq!(outcome["status"], json!("passed"), "{case}: {outcome}");
}

/// Assert `invariant` fails closed over `projection` with exactly `code`.
fn assert_binding_refused(case: &str, projection: &Value, invariant: &str, code: &str) {
    let outcome = sole_outcome(projection, invariant);
    assert_eq!(outcome["status"], json!("failed"), "{case}: {outcome}");
    assert_eq!(outcome["code"], json!(code), "{case}: {outcome}");
}

/// The `MeasurementPlan` maturity stages, in the order the schema declares them.
fn schema_stages() -> Vec<String> {
    let schema: Value = serde_json::from_str(include_str!(
        "../engineering_assurance/schemas/measurement-plan-frontmatter.schema.json"
    ))
    .expect("the MeasurementPlan schema must be valid JSON");
    schema["properties"]["stage"]["enum"]
        .as_array()
        .expect("stage must be an enum")
        .iter()
        .map(|stage| stage.as_str().expect("a stage is a string").to_owned())
        .collect()
}

fn with_stages(mut projection: Value, prior: &str, proposed: &str) -> Value {
    for kind in ["promotion_request", "promotion_evidence"] {
        projection["items"][kind][0]["prior_stage"] = json!(prior);
        projection["items"][kind][0]["proposed_stage"] = json!(proposed);
    }
    projection
}

#[trace("TC-201", "FR-016-AC-5")]
#[test]
fn tc_201_a_promotion_advances_exactly_one_schema_stage() {
    let stages = schema_stages();
    assert_eq!(stages.len(), 7, "{stages:?}");
    for (prior_index, prior) in stages.iter().enumerate() {
        for (proposed_index, proposed) in stages.iter().enumerate() {
            let case = format!("{prior} -> {proposed}");
            let projection = with_stages(promotion(), prior, proposed);
            if proposed_index == prior_index + 1 {
                assert_binding_holds(&case, &projection, "measurement.promotion_ready");
            } else {
                assert_binding_refused(
                    &case,
                    &projection,
                    "measurement.promotion_ready",
                    "promotion_must_advance_one_stage",
                );
            }
        }
    }
    for (prior, proposed) in [("observe", "release"), ("draft", "observe")] {
        assert_binding_refused(
            &format!("unknown stage {prior} -> {proposed}"),
            &with_stages(promotion(), prior, proposed),
            "measurement.promotion_ready",
            "promotion_must_advance_one_stage",
        );
    }
}

fn change_impact() -> Value {
    workflow_projection(
        "change-assurance",
        &["decision_ready->approved", "decision_ready->rejected"],
        &["change_request", "impact_snapshot"],
    )
}

#[trace("TC-201", "FR-016-AC-5")]
#[test]
fn tc_201_an_impact_snapshot_must_bind_the_change_and_carry_every_array() {
    let invariant = "change.impact_ready";
    assert_binding_holds("complete snapshot", &change_impact(), invariant);
    for array in [
        "changed_nodes",
        "impacted_nodes",
        "missing_edges",
        "stale_evidence",
        "suspect_evidence",
        "unknowns",
    ] {
        let mut projection = change_impact();
        projection["items"]["impact_snapshot"][0]
            .as_object_mut()
            .expect("snapshot must be an object")
            .remove(array);
        assert_binding_refused(
            &format!("without {array}"),
            &projection,
            invariant,
            "impact_snapshot_incomplete",
        );
    }
    for (member, value) in [
        ("baseline_id", json!("baseline-2")),
        ("profile_path", json!("assurance/other.yaml")),
        ("source_revision", json!("b".repeat(40))),
    ] {
        let mut projection = change_impact();
        projection["items"]["impact_snapshot"][0][member] = value;
        assert_binding_refused(
            &format!("snapshot for another {member}"),
            &projection,
            invariant,
            "impact_snapshot_incomplete",
        );
    }
}

fn intake_with_exception(expected: bool, exception: Option<Value>) -> Value {
    let mut projection = workflow_projection(
        "assurance-intake",
        &["decision_ready->accepted", "decision_ready->rejected"],
        &["intake_request"],
    );
    projection["items"]["intake_request"][0]["exceptions_expected"] = json!(expected);
    if let Some(exception) = exception {
        projection["items"]["exception"] = json!([exception]);
    }
    projection
}

fn current_exception() -> Value {
    passing_projection()["items"]["exception"][0].clone()
}

#[trace("TC-201", "FR-016-AC-5")]
#[test]
fn tc_201_every_recorded_exception_must_be_owned_and_current() {
    let invariant = "shared.exceptions_ready";
    let code = "owned_current_exception_required";
    assert_binding_holds(
        "expected and current",
        &intake_with_exception(true, Some(current_exception())),
        invariant,
    );
    assert_binding_holds(
        "none expected, none recorded",
        &intake_with_exception(false, None),
        invariant,
    );
    assert_binding_refused(
        "expected but none recorded",
        &intake_with_exception(true, None),
        invariant,
        code,
    );
    // An exception nobody expected is still checked: it must not slip through
    // because the request said none would be needed.
    for (member, value) in [
        ("expires_at", json!("not-a-date")),
        ("expires_at", json!(EVALUATED_AT)),
        ("owner", json!(" ")),
        ("rationale", json!("")),
        ("impact", Value::Null),
    ] {
        let mut exception = current_exception();
        exception[member] = value.clone();
        for expected in [false, true] {
            assert_binding_refused(
                &format!("{member} = {value}, expected = {expected}"),
                &intake_with_exception(expected, Some(exception.clone())),
                invariant,
                code,
            );
        }
    }
}

fn architecture_review() -> Value {
    workflow_projection(
        "architecture-evaluation",
        &["decision_ready->accepted", "decision_ready->rejected"],
        &["architecture_request", "review_validation"],
    )
}

fn change_review() -> Value {
    workflow_projection(
        "change-assurance",
        &["decision_ready->approved", "decision_ready->rejected"],
        &["change_request", "review_validation"],
    )
}

#[trace("TC-201", "FR-016-AC-5")]
#[test]
fn tc_201_a_review_must_bind_the_requested_subject() {
    let architecture = "architecture.review_ready";
    assert_binding_holds("matching review", &architecture_review(), architecture);
    let change = "change.review_ready";
    assert_binding_holds("matching review", &change_review(), change);

    // The architecture review is review_validation[0]; the code review is [1].
    for (member, value) in [
        ("subject_path", json!("spec/other-architecture.md")),
        ("valid", json!(false)),
        ("analysis", json!("code-review")),
        ("artifact_type", json!("ArchitectureDescription")),
        ("path", json!("")),
    ] {
        let mut projection = architecture_review();
        projection["items"]["review_validation"][0][member] = value.clone();
        assert_binding_refused(
            &format!("architecture review {member} = {value}"),
            &projection,
            architecture,
            "architecture_review_missing",
        );
    }
    for (member, value) in [
        ("source_revision", json!("b".repeat(40))),
        ("valid", json!(false)),
        ("analysis", json!("architecture-evaluation")),
        ("artifact_type", json!("ArchitectureDescription")),
        ("path", json!("")),
    ] {
        let mut projection = change_review();
        projection["items"]["review_validation"][1][member] = value.clone();
        assert_binding_refused(
            &format!("code review {member} = {value}"),
            &projection,
            change,
            "code_review_missing",
        );
    }
}
