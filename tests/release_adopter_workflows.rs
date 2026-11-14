// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Contract tests for immutable release-workflow fixtures from adopter PRs.

use std::{
    io::Write,
    process::{Command, Stdio},
};

use serde::Deserialize;
use serde_json::Value;

use ix_trace_rs::trace;

fn release_permissions() -> Value {
    serde_json::json!({"contents": "read"})
}

fn attestation_permissions() -> Value {
    serde_json::json!({
        "contents": "read",
        "id-token": "write",
        "attestations": "write"
    })
}

fn parse_yaml(source: &str) -> Value {
    yaml_serde::from_slice(source.as_bytes()).expect("workflow fixture parses as YAML")
}

#[derive(Debug, Deserialize)]
struct FixtureProvenance {
    protocol: String,
    sources: Vec<FixtureSource>,
}

#[derive(Debug, Deserialize)]
struct FixtureSource {
    repository: String,
    pull_request: u64,
    base_commit: String,
    head_commit: String,
    additive_workflow: String,
    additive_workflow_sha256: String,
    preserved_manual_workflow: String,
    base_manual_workflow_fixture: String,
    base_manual_workflow_sha256: String,
    preserved_manual_workflow_sha256: String,
}

fn provenance() -> FixtureProvenance {
    serde_json::from_str(include_str!("fixtures/adopters/provenance.json"))
        .expect("fixture provenance parses as JSON")
}

fn decode_base64_fixture(encoded: &str) -> String {
    fn digit(byte: u8) -> u8 {
        match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("base64 fixture has an invalid digit"),
        }
    }

    let compact = encoded
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    assert_eq!(compact.len() % 4, 0, "base64 fixture is padded");
    let mut decoded = Vec::new();
    for (chunk_index, chunk) in compact.chunks_exact(4).enumerate() {
        let is_last = chunk_index + 1 == compact.len() / 4;
        let first = digit(chunk[0]) as u32;
        let second = digit(chunk[1]) as u32;
        let third = if chunk[2] == b'=' {
            assert!(is_last && chunk[3] == b'=');
            0
        } else {
            digit(chunk[2]) as u32
        };
        let fourth = if chunk[3] == b'=' {
            assert!(is_last);
            0
        } else {
            digit(chunk[3]) as u32
        };
        let combined = (first << 18) | (second << 12) | (third << 6) | fourth;
        decoded.push((combined >> 16) as u8);
        if chunk[2] != b'=' {
            decoded.push((combined >> 8) as u8);
        }
        if chunk[3] != b'=' {
            decoded.push(combined as u8);
        }
    }
    String::from_utf8(decoded).expect("decoded workflow fixture is UTF-8")
}

fn assert_sha256(bytes: &str, expected: &str) {
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("sha256sum is available for fixture integrity");
    child
        .stdin
        .take()
        .expect("sha256sum stdin")
        .write_all(bytes.as_bytes())
        .expect("fixture bytes reach sha256sum");
    let output = child.wait_with_output().expect("sha256sum completes");
    assert!(output.status.success(), "sha256sum succeeds");
    let line = String::from_utf8(output.stdout).expect("sha256sum emits UTF-8");
    assert_eq!(line.split_whitespace().next(), Some(expected));
}

fn assert_additive_release_caller(additive: &str, preserved_manual: &str) {
    let manual = parse_yaml(preserved_manual);
    assert!(
        manual["on"]["workflow_dispatch"].is_object(),
        "existing publish workflow remains manually triggered"
    );

    let workflow = parse_yaml(additive);
    assert_eq!(workflow["on"]["push"]["tags"][0], "v*");
    assert!(workflow["on"]["workflow_dispatch"].is_null());
    assert!(workflow["on"]["workflow_run"].is_null());

    let jobs = &workflow["jobs"];
    let build = &jobs["build"];
    assert_eq!(
        build.get("permissions").unwrap_or(&workflow["permissions"]),
        &release_permissions()
    );
    let release_artifact_jobs =
        jobs.as_object()
            .expect("workflow jobs are a mapping")
            .iter()
            .filter_map(|(name, job)| {
                let uploads_release_artifacts = job["steps"].as_array().is_some_and(|steps| {
                    steps.iter().any(|step| {
                        step["uses"].as_str().is_some_and(|reference| {
                            reference.starts_with("actions/upload-artifact@")
                        }) && step["with"]["name"] == "release-artifacts"
                    })
                });
                let invokes_local_builder = job["uses"]
                    .as_str()
                    .is_some_and(|reference| reference.starts_with("./.github/workflows/"));
                (uploads_release_artifacts || invokes_local_builder).then_some(name.as_str())
            })
            .collect::<Vec<_>>();
    assert!(
        !release_artifact_jobs.is_empty(),
        "a tag workflow uploads the release artifacts"
    );

    let attest = &jobs["attest"];
    assert_eq!(attest["permissions"], attestation_permissions());
    assert_eq!(
        attest["uses"]
            .as_str()
            .expect("attest calls reusable workflow"),
        "agent-ix/engineering-assurance/.github/workflows/release-attestation.yml@v0.9.0"
    );
    assert_eq!(attest["with"]["ea_tooling_tag"], "v0.9.0");
    assert_eq!(attest["with"]["quoin_tooling_tag"], "0.30.1");
    assert_eq!(attest["with"]["artifact_name"], "release-artifacts");
    assert!(
        attest["with"]["record_path"]
            .as_str()
            .is_some_and(|path| !path.is_empty())
    );
    assert!(
        attest["with"]["allowed_signers_path"]
            .as_str()
            .is_some_and(|path| !path.is_empty())
    );

    let needs = attest["needs"].as_array().cloned().unwrap_or_else(|| {
        attest["needs"]
            .as_str()
            .map(|job| vec![Value::String(job.to_owned())])
            .unwrap_or_default()
    });
    assert!(needs.iter().any(|dependency| {
        release_artifact_jobs
            .iter()
            .any(|job| *job == dependency.as_str().unwrap_or_default())
    }));
    for dependency in needs {
        let dependency = dependency.as_str().expect("job dependency is a string");
        assert!(
            jobs.get(dependency).is_some(),
            "{dependency} is a local job"
        );
        assert_ne!(dependency, "publish");
        assert_ne!(dependency, "publish-npm");
    }
}

#[trace("TC-664", "TC-665", "TC-667", "FR-070-AC-8", "FR-070-AC-11")]
#[test]
fn adopter_tag_attestation_callers_are_additive_to_manual_release_workflows() {
    let provenance = provenance();
    assert_eq!(
        provenance.protocol,
        "engineering-assurance.release-adopter-fixtures/v1"
    );
    assert_eq!(provenance.sources.len(), 3);
    assert_eq!(provenance.sources[0].repository, "agent-ix/quoin");
    assert_eq!(provenance.sources[0].pull_request, 899);
    assert_eq!(
        provenance.sources[0].base_commit,
        "b8eac0189e358ffa2d78cd0bfce2f624fa9180d9"
    );
    assert_eq!(
        provenance.sources[0].head_commit,
        "453fda114d81ed2aefe630028dcebd4bfcd8ae1b"
    );
    assert_eq!(
        provenance.sources[0].additive_workflow,
        "tag-attestation.yml"
    );
    assert_eq!(
        provenance.sources[0].additive_workflow_sha256,
        "0f02a8ae9a10adc128856190297a34b426a63c19c95f07477bf17ab5160cc5cc"
    );
    assert_eq!(
        provenance.sources[0].preserved_manual_workflow,
        "release.yml"
    );
    assert_eq!(
        provenance.sources[0].base_manual_workflow_fixture,
        "base-release.yml.txt"
    );
    assert_eq!(
        provenance.sources[0].base_manual_workflow_sha256,
        "7664e48ff95ec3c3838d734028864e014ace64d8fbd3f3634452240c5feeda0d"
    );
    assert_eq!(
        provenance.sources[0].preserved_manual_workflow_sha256,
        "7664e48ff95ec3c3838d734028864e014ace64d8fbd3f3634452240c5feeda0d"
    );
    assert_eq!(provenance.sources[1].repository, "agent-ix/quire-rs");
    assert_eq!(provenance.sources[1].pull_request, 682);
    assert_eq!(
        provenance.sources[1].base_commit,
        "5a45ec949737a41c69a90f6ac5eefcafb2e10716"
    );
    assert_eq!(
        provenance.sources[1].head_commit,
        "258f73e201a34f2d683a98a9fd1cf213895322ad"
    );
    assert_eq!(
        provenance.sources[1].additive_workflow,
        "tag-attestation.yml"
    );
    assert_eq!(
        provenance.sources[1].additive_workflow_sha256,
        "6efaac3125c276bf614b06fd864670791cd68bad3d7324a13ca655b6d0926d6c"
    );
    assert_eq!(
        provenance.sources[1].preserved_manual_workflow,
        "wheels.yml"
    );
    assert_eq!(
        provenance.sources[1].base_manual_workflow_fixture,
        "base-wheels.yml.txt"
    );
    assert_eq!(
        provenance.sources[1].base_manual_workflow_sha256,
        "158d86d5fcef94c16f0f8e6c1778d12bc85beb14d1f3937bcf9b23beaa338c15"
    );
    assert_eq!(
        provenance.sources[1].preserved_manual_workflow_sha256,
        "29f503fb48081406ffb7920a08d54bbf387fd22ed4f1fe03c125d2b983bafd6e"
    );
    assert_eq!(provenance.sources[2].repository, "agent-ix/quire-cli");
    assert_eq!(provenance.sources[2].pull_request, 201);
    assert_eq!(
        provenance.sources[2].base_commit,
        "3b1965894a00e7955b5386dd0acafe1ce4af610a"
    );
    assert_eq!(
        provenance.sources[2].head_commit,
        "5ebc5fa90364ab4d52e1822f9f4a59f1af0775ec"
    );
    assert_eq!(
        provenance.sources[2].additive_workflow,
        "tag-attestation.yml"
    );
    assert_eq!(
        provenance.sources[2].additive_workflow_sha256,
        "96cb83765ad4dec7a6ecde84387e11838e9ea89d6cb2a07f4037e28c96bf2714"
    );
    assert_eq!(
        provenance.sources[2].preserved_manual_workflow,
        "release.yml"
    );
    assert_eq!(
        provenance.sources[2].base_manual_workflow_fixture,
        "base-release.yml.txt"
    );
    assert_eq!(
        provenance.sources[2].base_manual_workflow_sha256,
        "774218c38a3b8ed264102e74cde57bf069e872bff01b66814e1b543ff3ea5c9b"
    );
    assert_eq!(
        provenance.sources[2].preserved_manual_workflow_sha256,
        "774218c38a3b8ed264102e74cde57bf069e872bff01b66814e1b543ff3ea5c9b"
    );

    let quoin_tag = decode_base64_fixture(include_str!(
        "fixtures/adopters/quoin/tag-attestation.yml.txt"
    ));
    let quoin_release =
        decode_base64_fixture(include_str!("fixtures/adopters/quoin/release.yml.txt"));
    let quoin_base_release =
        decode_base64_fixture(include_str!("fixtures/adopters/quoin/base-release.yml.txt"));
    let quire_rs_tag = decode_base64_fixture(include_str!(
        "fixtures/adopters/quire-rs/tag-attestation.yml.txt"
    ));
    let quire_rs_wheels =
        decode_base64_fixture(include_str!("fixtures/adopters/quire-rs/wheels.yml.txt"));
    let quire_rs_base_wheels = decode_base64_fixture(include_str!(
        "fixtures/adopters/quire-rs/base-wheels.yml.txt"
    ));
    let quire_cli_tag = decode_base64_fixture(include_str!(
        "fixtures/adopters/quire-cli/tag-attestation.yml.txt"
    ));
    let quire_cli_release =
        decode_base64_fixture(include_str!("fixtures/adopters/quire-cli/release.yml.txt"));
    let quire_cli_base_release = decode_base64_fixture(include_str!(
        "fixtures/adopters/quire-cli/base-release.yml.txt"
    ));
    assert_sha256(&quoin_tag, &provenance.sources[0].additive_workflow_sha256);
    assert_sha256(
        &quoin_release,
        &provenance.sources[0].preserved_manual_workflow_sha256,
    );
    assert_sha256(
        &quoin_base_release,
        &provenance.sources[0].base_manual_workflow_sha256,
    );
    assert_eq!(quoin_base_release, quoin_release);
    assert_sha256(
        &quire_rs_tag,
        &provenance.sources[1].additive_workflow_sha256,
    );
    assert_sha256(
        &quire_rs_wheels,
        &provenance.sources[1].preserved_manual_workflow_sha256,
    );
    assert_sha256(
        &quire_rs_base_wheels,
        &provenance.sources[1].base_manual_workflow_sha256,
    );
    let quire_rs_base_manual = parse_yaml(&quire_rs_base_wheels);
    let quire_rs_manual = parse_yaml(&quire_rs_wheels);
    assert!(quire_rs_base_manual["on"]["workflow_dispatch"].is_object());
    assert!(quire_rs_manual["on"]["workflow_dispatch"].is_object());
    assert_eq!(
        quire_rs_base_manual["jobs"]["publish"]["if"],
        "${{ inputs.publish }}"
    );
    assert_eq!(
        quire_rs_manual["jobs"]["publish"]["if"],
        "${{ github.event_name == 'workflow_dispatch' && inputs.publish }}"
    );
    assert_sha256(
        &quire_cli_tag,
        &provenance.sources[2].additive_workflow_sha256,
    );
    assert_sha256(
        &quire_cli_release,
        &provenance.sources[2].preserved_manual_workflow_sha256,
    );
    assert_sha256(
        &quire_cli_base_release,
        &provenance.sources[2].base_manual_workflow_sha256,
    );
    assert_eq!(quire_cli_base_release, quire_cli_release);

    assert_additive_release_caller(&quoin_tag, &quoin_release);
    assert_additive_release_caller(&quire_rs_tag, &quire_rs_wheels);
    assert_additive_release_caller(&quire_cli_tag, &quire_cli_release);
    let quire_rs_workflow = parse_yaml(&quire_rs_tag);
    let retained_evidence = &quire_rs_workflow["jobs"]["retained-evidence"];
    assert!(retained_evidence.is_object());
    assert!(retained_evidence["steps"].as_array().is_some_and(|steps| {
        steps.iter().any(|step| {
            step["run"]
                .as_str()
                .is_some_and(|run| run.contains("no_retained_evidence_store"))
        })
    }));
    assert!(
        quire_rs_workflow["jobs"]["attest"]["needs"]
            .as_array()
            .is_some_and(|needs| needs.iter().any(|job| job == "retained-evidence"))
    );

    let quire_cli_workflow = parse_yaml(&quire_cli_tag);
    let quire_cli_build = &quire_cli_workflow["jobs"]["build"];
    assert!(quire_cli_build["steps"].as_array().is_some_and(|steps| {
        steps.iter().any(|step| {
            step["run"].as_str().is_some_and(|run| {
                run.contains("no_retained_evidence_store")
                    && run.contains("no Test Result or release statement will be created")
            })
        })
    }));
    assert_eq!(quire_cli_workflow["jobs"]["attest"]["needs"], "build");
}
