// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Differential evidence for the pure Rust onboarding core.

use std::{
    io::Write,
    process::{Command, Stdio},
    str::FromStr,
};

use engineering_assurance::onboarding::{
    ArtifactType, Inventory, OnboardingPlan, REQUEST_PROTOCOL, authored_result,
    parse_request_bytes, plan, render_from_skeleton,
};
use ix_trace_rs::trace;
use serde_json::{Map, Value, json};

const PYTHON_REFERENCE: &str = r#"
import json
import sys
import tempfile
from pathlib import Path

import engineering_assurance.onboarding as onboarding

payload = json.load(sys.stdin)
if payload["mode"] == "frontmatter":
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "artifact.md"
        path.write_text(payload["text"])
        data, error = onboarding._frontmatter(path)
    value = data.get("type") if isinstance(data, dict) else None
    print(json.dumps({
        "type": value if isinstance(value, str) else None,
        "error": error,
    }, separators=(",", ":")))
elif payload["mode"] == "render":
    sys.stdout.write(onboarding.render_from_skeleton(
        payload["artifact_type"], payload["replacements"]
    ))
else:
    inventory = onboarding.Inventory(
        decisions=payload["inventory"]["decisions"],
        measurements=[onboarding.Validation(**item) for item in payload["inventory"]["measurements"]],
        assurance_artifacts=[onboarding.Validation(**item) for item in payload["inventory"]["assurance_artifacts"]],
        evidence_references=payload["inventory"]["evidence_references"],
        producer_configurations=payload["inventory"]["producer_configurations"],
        unresolved_inputs=payload["inventory"]["unresolved_inputs"],
    )
    onboarding.inventory_repository = lambda *_args, **_kwargs: inventory
    onboarding.publish_validated_artifact = (
        lambda root, target, *_args, **_kwargs: Path(root) / target
    )
    request_data = payload["request"]
    request_data["repository_root"] = Path(request_data["repository_root"])
    if request_data.get("target") is not None:
        request_data["target"] = Path(request_data["target"])
    request = onboarding.OnboardingRequest(**request_data)
    result = onboarding.run_onboarding(request)
    print(json.dumps({
        "status": result.status,
        "inventory": result.inventory.to_dict(),
        "recommendation": result.recommendation,
        "artifact_path": result.artifact_path,
    }, separators=(",", ":")))
"#;

fn python(payload: &Value) -> Vec<u8> {
    let mut child = Command::new("python3")
        .args(["-c", PYTHON_REFERENCE])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("retained Python onboarding reference must start");
    child
        .stdin
        .take()
        .expect("piped stdin must exist")
        .write_all(&serde_json::to_vec(payload).expect("payload must serialize"))
        .expect("payload must be writable");
    let output = child.wait_with_output().expect("reference must terminate");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

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

fn python_plan(request: &Value, inventory: &Value) -> Value {
    let mut python_request = request.clone();
    let object = python_request
        .as_object_mut()
        .expect("request fixture must be an object");
    object.remove("protocol");
    object.remove("module_root");
    object.remove("quire_executable");
    if let Some(target) = object.get_mut("target") {
        *target = json!(target.as_str().expect("target must be a string"));
    }
    let output = python(&json!({
        "mode": "plan",
        "request": python_request,
        "inventory": inventory,
    }));
    serde_json::from_slice(&output).expect("reference result must be JSON")
}

#[trace("TC-105", "FR-016-AC-1", "FR-016-CON-4")]
#[test]
fn tc_105_recommendation_states_match_the_retained_reference() {
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
        ),
        (request(&json!({})), empty_inventory()),
        (
            request(&json!({"requested_artifact": "AssuranceProfile"})),
            empty_inventory(),
        ),
        (
            request(&json!({"requested_artifact": "AssuranceProfile"})),
            invalid,
        ),
        (
            request(&json!({"requested_artifact": "AssuranceProfile"})),
            duplicate,
        ),
        (
            request(&json!({"requested_artifact": "AssuranceProfile"})),
            reusable,
        ),
        (
            request(&json!({
                "requested_artifact": "AssuranceProfile",
                "justification": "material fictional decision",
            })),
            empty_inventory(),
        ),
        (
            request(&json!({
                "requested_artifact": "AssuranceProfile",
                "justification": "material fictional decision",
                "target": "spec/AP-002.md",
                "frontmatter": {"id": "AP-002", "title": "Fictional profile"},
            })),
            empty_inventory(),
        ),
    ];
    for (request, inventory) in cases {
        assert_eq!(
            rust_plan(&request, &inventory),
            python_plan(&request, &inventory)
        );
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
            let payload = json!({
                "mode": "render",
                "artifact_type": artifact_type,
                "replacements": replacements,
            });
            let python =
                String::from_utf8(python(&payload)).expect("reference output must be UTF-8");
            let (rust_frontmatter, rust_body) = rendered_parts(&rust);
            let (python_frontmatter, python_body) = rendered_parts(&python);
            assert_eq!(rust_frontmatter, python_frontmatter);
            assert_eq!(rust_body, python_body);
        }
    }
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
    for text in shared_cases {
        let rust = match engineering_assurance::onboarding::frontmatter_type(text) {
            Ok(artifact_type) => json!({
                "type": artifact_type.filter(|value| {
                    ArtifactType::from_str(value).is_ok()
                        || value.to_lowercase().contains("decision")
                }),
                "error": null,
            }),
            Err(_) => json!({"type": null, "error": "malformed-frontmatter"}),
        };
        let reference = python(&json!({"mode": "frontmatter", "text": text}));
        let reference: Value =
            serde_json::from_slice(&reference).expect("reference result must be JSON");
        assert_eq!(rust, reference, "frontmatter case differs: {text:?}");
    }

    for text in [
        "---\ntype: DecisionRecord\ntype: AssuranceProfile\n---\n# Duplicate\n",
        "---\ndefaults: &defaults\n  type: AssuranceProfile\n<<: *defaults\n---\n# Merge\n",
    ] {
        let reference = python(&json!({"mode": "frontmatter", "text": text}));
        let reference: Value =
            serde_json::from_slice(&reference).expect("reference result must be JSON");
        assert_eq!(reference["type"], "AssuranceProfile");
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
