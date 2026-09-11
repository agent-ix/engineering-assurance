// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Differential evidence for the pure Rust onboarding core.

use engineering_assurance::onboarding::{
    ArtifactType, Inventory, OnboardingPlan, REQUEST_PROTOCOL, authored_result,
    parse_request_bytes, plan, render_from_skeleton,
};
use ix_trace_rs::trace;
use serde_json::{Map, Value, json};

fn empty_inventory() -> Value {
    json!({
        "decisions": [],
        "measurements": [],
        "assurance_artifacts": [],
        "evidence_references": [],
        "producer_configurations": [],
        "unresolved_inputs": [],
    })
}

fn request(changes: &Value) -> Value {
    let mut request = json!({
        "protocol": REQUEST_PROTOCOL,
        "repository_root": "/tmp/fictional-onboarding-root",
        "module_root": "/tmp/fictional-module-root",
        "decision_boundary": "one fictional candidate revision",
        "decision_owner": "release-owner",
    });
    request
        .as_object_mut()
        .expect("request fixture must be an object")
        .extend(
            changes
                .as_object()
                .expect("request changes must be an object")
                .clone(),
        );
    request
}

fn rust_plan(request: &Value, inventory: &Value) -> Value {
    let parsed =
        parse_request_bytes(&serde_json::to_vec(request).expect("request fixture must serialize"))
            .expect("request fixture must parse");
    let inventory = serde_json::from_value::<Inventory>(inventory.clone())
        .expect("inventory fixture must parse");
    let result = match plan(&parsed, inventory) {
        OnboardingPlan::Complete(result) => result,
        OnboardingPlan::Publish(plan) => authored_result(plan),
    };
    let mut value = serde_json::to_value(result).expect("result must serialize");
    value
        .as_object_mut()
        .expect("result must be an object")
        .remove("protocol");
    value
}

#[trace("TC-105", "FR-016-AC-1", "FR-016-CON-4")]
#[test]
fn tc_105_recommendation_states_are_explicit_native_contracts() {
    let mut invalid = empty_inventory();
    invalid["assurance_artifacts"] = json!([{
        "path": "spec/AP-bad.md",
        "artifact_type": "AssuranceProfile",
        "valid": false,
        "diagnostics": ["required owner missing"],
    }]);
    let mut duplicate = invalid.clone();
    duplicate["assurance_artifacts"][0]["valid"] = json!(true);
    duplicate["assurance_artifacts"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "path": "spec/AP-other.md",
            "artifact_type": "AssuranceProfile",
            "valid": true,
            "diagnostics": [],
        }));
    let mut reusable = empty_inventory();
    reusable["assurance_artifacts"] = json!([{
        "path": "spec/AP-001.md",
        "artifact_type": "AssuranceProfile",
        "valid": true,
        "diagnostics": [],
    }]);

    let cases = [
        (
            request(&json!({"decision_boundary": null})),
            empty_inventory(),
            "needs-input",
            "Provide the exact decision boundary and human decision owner.",
            None,
        ),
        (
            request(&json!({})),
            empty_inventory(),
            "no-applicable-work",
            "No assurance artifact or governed workflow is justified by the request.",
            None,
        ),
        (
            request(&json!({"requested_artifact": "AssuranceProfile"})),
            empty_inventory(),
            "no-applicable-work",
            "No justification was supplied for a new AssuranceProfile.",
            None,
        ),
        (
            request(&json!({"requested_artifact": "AssuranceProfile"})),
            invalid,
            "needs-human-selection",
            "Applicable artifacts are malformed or conflicting; preserve them and select or correct one.",
            None,
        ),
        (
            request(&json!({"requested_artifact": "AssuranceProfile"})),
            duplicate,
            "needs-human-selection",
            "Applicable artifacts are malformed or conflicting; preserve them and select or correct one.",
            None,
        ),
        (
            request(&json!({"requested_artifact": "AssuranceProfile"})),
            reusable,
            "reuse",
            "Reuse the applicable validated AssuranceProfile.",
            Some("spec/AP-001.md"),
        ),
        (
            request(&json!({
                "requested_artifact": "AssuranceProfile",
                "justification": "material fictional decision",
            })),
            empty_inventory(),
            "needs-input",
            "Provide a confined target and artifact frontmatter before authoring.",
            None,
        ),
        (
            request(&json!({
                "requested_artifact": "AssuranceProfile",
                "justification": "material fictional decision",
                "target": "spec/AP-002.md",
                "frontmatter": {"id": "AP-002", "title": "Fictional profile"},
            })),
            empty_inventory(),
            "authored",
            "Authored and validated one justified AssuranceProfile.",
            Some("spec/AP-002.md"),
        ),
    ];
    for (request, inventory, status, recommendation, artifact_path) in cases {
        let result = rust_plan(&request, &inventory);
        assert_eq!(result["status"], status);
        assert_eq!(result["recommendation"], recommendation);
        assert_eq!(result["artifact_path"], json!(artifact_path));
        assert_eq!(result["inventory"], inventory);
    }
}

#[trace("TC-105", "FR-016-AC-1", "FR-016-CON-4")]
#[test]
fn tc_105_rendered_skeletons_preserve_frontmatter_and_body_semantics() {
    for replacements in [
        json!({
            "id": "EX-042",
            "title": "Fictional candidate artifact",
            "owner": "release-owner",
        }),
        json!({
            "id": "EX-043",
            "title": "   ",
            "owner": "release-owner",
        }),
    ] {
        let replacements = replacements
            .as_object()
            .expect("replacements must be an object")
            .clone();
        for artifact_type in ArtifactType::ALL {
            let skeleton = std::fs::read_to_string(format!(
                "{}/engineering_assurance/skeletons/{artifact_type}.md",
                env!("CARGO_MANIFEST_DIR")
            ))
            .expect("installed skeleton fixture must be readable");
            let rust = render_from_skeleton(artifact_type, &skeleton, &replacements)
                .expect("Rust renderer must accept installed skeleton");
            let (rust_frontmatter, rust_body) = rendered_parts(&rust);
            let (skeleton_frontmatter, skeleton_body) = rendered_parts(&skeleton);
            let rust_frontmatter = rust_frontmatter
                .as_mapping()
                .expect("rendered frontmatter must remain a mapping");
            let skeleton_frontmatter = skeleton_frontmatter
                .as_mapping()
                .expect("skeleton frontmatter must be a mapping");
            for key in ["id", "owner"] {
                let expected = replacements
                    .get(key)
                    .map(|value| yaml_serde::to_value(value).expect("value must convert"))
                    .or_else(|| {
                        skeleton_frontmatter
                            .get(yaml_serde::Value::String(key.to_owned()))
                            .cloned()
                    });
                assert_eq!(
                    rust_frontmatter.get(yaml_serde::Value::String(key.to_owned())),
                    expected.as_ref(),
                    "{artifact_type} must preserve or replace {key}"
                );
            }
            assert_eq!(
                rust_frontmatter.get(yaml_serde::Value::String("type".to_owned())),
                Some(&yaml_serde::Value::String(artifact_type.to_string()))
            );
            let expected_title = replacements
                .get("title")
                .and_then(Value::as_str)
                .filter(|title| !title.trim().is_empty());
            if let Some(title) = expected_title {
                assert!(rust_body.starts_with(&format!("\n# {title}\n")));
                assert_eq!(body_after_title(rust_body), body_after_title(skeleton_body));
            } else {
                assert_eq!(rust_body, skeleton_body);
            }
        }
    }
}

fn body_after_title(body: &str) -> &str {
    body.strip_prefix("\n# ")
        .expect("installed skeleton body must begin with one title")
        .split_once('\n')
        .expect("installed skeleton title must end with a newline")
        .1
}

fn rendered_parts(rendered: &str) -> (yaml_serde::Value, &str) {
    let remainder = rendered
        .strip_prefix("---\n")
        .expect("rendered artifact must start with frontmatter");
    let boundary = remainder
        .find("\n---\n")
        .expect("rendered artifact must close frontmatter");
    let frontmatter =
        yaml_serde::from_str(&remainder[..boundary]).expect("rendered frontmatter must parse");
    (frontmatter, &remainder[boundary + 5..])
}

#[trace("TC-105", "FR-016-AC-1", "FR-016-CON-4")]
#[test]
fn tc_105_frontmatter_type_matches_yaml_boundary_cases() {
    let shared_cases = [
        "no frontmatter\n",
        "---\ntype: AssuranceProfile\n---\n# Profile\n",
        "---\n- not-a-mapping\n---\n# Invalid\n",
        "---\ntype: [unterminated\n---\n# Invalid\n",
        "---\ntype: yes\n---\n# Implicit boolean\n",
    ];
    let expected = [None, Some("AssuranceProfile")];
    for (text, expected) in shared_cases[..2].iter().zip(expected) {
        assert_eq!(
            engineering_assurance::onboarding::frontmatter_type(text).expect("valid boundary"),
            expected.map(str::to_owned),
        );
    }
    for text in &shared_cases[2..4] {
        assert!(engineering_assurance::onboarding::frontmatter_type(text).is_err());
    }
    assert_eq!(
        engineering_assurance::onboarding::frontmatter_type(shared_cases[4])
            .expect("implicit non-string type is not malformed"),
        Some("yes".to_owned())
    );

    for text in [
        "---\ntype: DecisionRecord\ntype: AssuranceProfile\n---\n# Duplicate\n",
        "---\ndefaults: &defaults\n  type: AssuranceProfile\n<<: *defaults\n---\n# Merge\n",
    ] {
        assert!(
            engineering_assurance::onboarding::frontmatter_type(text).is_err(),
            "ambiguous artifact identity must fail closed: {text:?}"
        );
    }
}

#[trace("TC-105", "FR-016-AC-1", "FR-016-CON-4")]
#[test]
fn tc_105_request_and_frontmatter_failures_are_typed() {
    let unsupported = request(&json!({"requested_artifact": "UnknownArtifact"}));
    let error = parse_request_bytes(&serde_json::to_vec(&unsupported).unwrap())
        .expect_err("unsupported artifact type must refuse");
    assert_eq!(error.code(), "onboarding_request_invalid");
    assert!(parse_request_bytes(b"{not-json").is_err());

    let extended = request(&json!({"unexpected": true}));
    assert!(parse_request_bytes(&serde_json::to_vec(&extended).unwrap()).is_err());

    let empty = Map::new();
    let error = render_from_skeleton(ArtifactType::AssuranceProfile, "# no frontmatter\n", &empty)
        .expect_err("malformed skeleton must refuse");
    assert_eq!(error.code(), "onboarding_publication_failed");
}

#[trace("TC-105", "FR-016-AC-1", "FR-016-CON-4")]
#[test]
fn tc_105_reusable_onboarding_core_has_no_host_capability() {
    let library = include_str!("../src/lib.rs");
    let onboarding = include_str!("../src/onboarding.rs");
    assert!(!library.contains("onboarding_host"));
    for forbidden in [
        "std::{",
        "std::env",
        "std::fs",
        "std::net",
        "std::path",
        "std::process",
        "Command::",
        "File::",
        "SystemTime",
        "TcpStream",
        "UdpSocket",
    ] {
        assert!(
            !onboarding.contains(forbidden),
            "reusable onboarding core contains host capability {forbidden:?}"
        );
    }
}
