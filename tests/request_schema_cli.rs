// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Published request-schema discovery for release and approval commands (FR-089).

use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const COMMANDS: [&str; 4] = [
    "release-history",
    "release-decision",
    "release-verify",
    "approval-tag",
];

fn run(command: &str, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .arg(command)
        .args(args)
        .output()
        .expect("the Cargo-built CLI must start")
}

fn validator(output: &Output) -> (String, jsonschema::Validator) {
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert_eq!(output.stderr, [] as [u8; 0]);
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    let schema_text = std::str::from_utf8(&output.stdout).expect("schema is UTF-8");
    let schema: Value = serde_json::from_str(schema_text).expect("schema is JSON");
    let protocol = schema["properties"]["protocol"]["const"]
        .as_str()
        .expect("schema publishes a request protocol")
        .to_owned();
    let validator = jsonschema::options()
        .build(&schema)
        .expect("published schema compiles");
    (protocol, validator)
}

#[trace("TC-1090", "FR-089-AC-1", "FR-089-AC-2", "FR-089-AC-3", "FR-089-AC-4")]
#[test]
fn request_schemas_accept_their_contract_shapes_and_refuse_unknown_members() {
    let samples = [
        json!({
            "protocol": "engineering-assurance.release-history-request/v1",
            "repository": "/fixture/repository",
            "baseline_commit": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "previous_release_tag": null,
            "records_dir": "records"
        }),
        json!({
            "protocol": "engineering-assurance.release-decision-request/v1",
            "operation": "present",
            "state_dir": "/fixture/state",
            "skill_root": "/fixture/skills",
            "ix_flow_executable": "/fixture/ix-flow",
            "run_id": "fixture-run",
            "decision_owner": "owner@example.test",
            "repository": "/fixture/repository",
            "baseline_path": "baseline.md",
            "baseline_commit": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "snapshot_path": "release/snapshot.json",
            "release_id": "REL-1",
            "record_path": "release/REL-1.md"
        }),
        json!({
            "protocol": "engineering-assurance.release-verify-request/v1",
            "repository": "/fixture/repository",
            "tag": "v1.0.0",
            "record_path": "release/REL-1.md",
            "allowed_signers": "/fixture/allowed_signers",
            "gitsign": {
                "certificate_identity": "builder@example.test",
                "certificate_oidc_issuer": "fictional-issuer"
            },
            "identity_registry": "HEAD:.engineering-assurance/identities.json"
        }),
        json!({
            "protocol": "engineering-assurance.approval-tag-request/v1",
            "repository": "/fixture/repository",
            "tag": "approval/REC-1",
            "record": "/fixture/record.json",
            "record_id": "REC-1",
            "record_digest": "git-blob:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "meaning": "approved",
            "gesture": "device_touch"
        }),
    ];

    for (command, sample) in COMMANDS.into_iter().zip(samples) {
        let output = run(command, &["--schema", "request"]);
        let (protocol, validator) = validator(&output);
        assert_eq!(sample["protocol"], protocol, "{command}");
        assert!(validator.is_valid(&sample), "{command}: {sample}");

        if command == "release-verify" {
            let mut missing_identity_registry = sample.clone();
            missing_identity_registry
                .as_object_mut()
                .expect("request is an object")
                .remove("identity_registry");
            assert!(
                !validator.is_valid(&missing_identity_registry),
                "release-verify accepted gitsign without identity_registry"
            );

            let mut missing_gitsign = sample.clone();
            missing_gitsign
                .as_object_mut()
                .expect("request is an object")
                .remove("gitsign");
            assert!(
                !validator.is_valid(&missing_gitsign),
                "release-verify accepted identity_registry without gitsign"
            );
            assert!(
                validator.is_valid(&sample),
                "release-verify rejected the paired gitsign and identity_registry"
            );
        }

        let mut unknown = sample;
        unknown["unexpected_member"] = json!(true);
        assert!(
            !validator.is_valid(&unknown),
            "{command} accepted an unknown member"
        );

        if command == "release-decision" {
            let decision = json!({
                "protocol": protocol,
                "operation": "decide",
                "state_dir": "/fixture/state",
                "skill_root": "/fixture/skills",
                "ix_flow_executable": "/fixture/ix-flow",
                "run_id": "fixture-run",
                "decision_owner": "owner@example.test",
                "repository": "/fixture/repository",
                "baseline_path": "baseline.md",
                "baseline_commit": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "snapshot_path": "release/snapshot.json",
                "choice": "release",
                "meaning": "accepted the listed release evidence"
            });
            assert!(
                validator.is_valid(&decision),
                "release-decision decide: {decision}"
            );
            let mut missing_meaning = decision;
            missing_meaning
                .as_object_mut()
                .expect("object")
                .remove("meaning");
            assert!(!validator.is_valid(&missing_meaning));

            let invalid_without_choice = json!({
                "protocol": protocol,
                "operation": "decide",
                "state_dir": "/fixture/state",
                "skill_root": "/fixture/skills",
                "ix_flow_executable": "/fixture/ix-flow",
                "run_id": "fixture-run",
                "decision_owner": "owner@example.test",
                "repository": "/fixture/repository",
                "baseline_path": "baseline.md",
                "baseline_commit": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "snapshot_path": "release/snapshot.json",
                "meaning": "a choice is required before a meaning"
            });
            assert!(!validator.is_valid(&invalid_without_choice));
        }
    }
}

#[trace("TC-1091", "FR-089-AC-5")]
#[test]
fn request_schema_options_reject_unsupported_or_ambiguous_invocations() {
    for command in COMMANDS {
        for args in [
            vec!["--schema", "result"],
            vec!["--schema"],
            vec!["--schema", "request", "--request", "{}"],
            vec![],
        ] {
            let output = run(command, &args);
            assert_eq!(output.status.code(), Some(2), "{command} {args:?}");
            assert!(output.stdout.is_empty(), "{command} {args:?}");
            assert!(!output.stderr.is_empty(), "{command} {args:?}");
        }
    }
}
