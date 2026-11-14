// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Guard the Engineering Assurance release caller's workflow contract.

use std::{fs, path::Path};

use ix_trace_rs::trace;
use serde_json::Value;

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn workflow() -> Value {
    let path = repository_root().join(".github/workflows/release.yml");
    let bytes = fs::read(&path).expect("release caller workflow exists");
    yaml_serde::from_slice(&bytes).expect("release caller workflow parses as YAML")
}

fn reusable_workflow() -> Value {
    let path = repository_root().join(".github/workflows/release-attestation.yml");
    let bytes = fs::read(&path).expect("release-attestation reusable workflow exists");
    yaml_serde::from_slice(&bytes).expect("release-attestation reusable workflow parses as YAML")
}

#[trace(
    "TC-557",
    "TC-665",
    "TC-667",
    "TC-668",
    "FR-070-AC-1",
    "FR-070-AC-9",
    "FR-070-AC-11",
    "FR-070-AC-12"
)]
#[test]
fn engineering_assurance_tag_caller_builds_then_calls_the_matching_release_tag() {
    let workflow = workflow();
    assert_eq!(
        workflow["on"]["push"]["tags"][0], "v*",
        "the caller runs only for release tags"
    );

    let jobs = &workflow["jobs"];
    let build = &jobs["build"];
    assert_eq!(build["permissions"]["contents"], "read");
    let steps = build["steps"].as_array().expect("build steps");
    assert!(steps.iter().any(|step| {
        step["uses"]
            .as_str()
            .is_some_and(|reference| reference.starts_with("actions/checkout@"))
            && step["with"]["persist-credentials"] == false
    }));
    assert!(steps.iter().any(|step| {
        step["uses"]
            .as_str()
            .is_some_and(|reference| reference.starts_with("actions/upload-artifact@"))
            && step["with"]["name"] == "release-artifacts"
    }));
    assert!(
        steps
            .iter()
            .all(|step| step["with"]["name"] != "release-evidence")
    );

    let attest = &jobs["attest"];
    assert_eq!(attest["needs"], "build");
    assert_eq!(
        attest["permissions"],
        serde_json::json!({
            "contents":"read",
            "id-token":"write",
            "attestations":"write"
        })
    );
    assert_eq!(
        attest["uses"],
        "agent-ix/engineering-assurance/.github/workflows/release-attestation.yml@v0.9.0"
    );
    assert_eq!(attest["with"]["tag"], "${{ github.ref_name }}");
    assert_eq!(attest["with"]["ea_tooling_tag"], "v0.9.0");
    let reusable_ref = attest["uses"].as_str().expect("reusable workflow ref");
    let reusable_tag = reusable_ref
        .rsplit_once('@')
        .expect("pinned workflow tag")
        .1;
    assert_eq!(attest["with"]["ea_tooling_tag"], reusable_tag);
    assert_eq!(attest["with"]["quoin_tooling_tag"], "0.30.1");
    assert_eq!(
        attest["with"]["record_path"],
        "spec/releases/engineering-assurance.md"
    );
    assert_eq!(
        attest["with"]["allowed_signers_path"],
        ".engineering-assurance/allowed_signers"
    );
    assert!(
        attest.get("steps").is_none(),
        "attest job only calls the reusable workflow"
    );
}

#[trace("TC-622", "NFR-011-AC-1", "NFR-011-AC-2")]
#[test]
fn signing_jobs_run_no_agent_or_repository_code() {
    let caller = workflow();
    let reusable = reusable_workflow();
    let signing_jobs = [
        (&caller["jobs"]["attest"], "release caller"),
        (&reusable["jobs"]["attest"], "reusable workflow"),
    ];

    for (job, name) in signing_jobs {
        let permissions = &job["permissions"];
        assert_eq!(permissions["id-token"], "write", "{name} OIDC permission");
        assert_eq!(
            permissions["attestations"], "write",
            "{name} signing permission"
        );
        if let Some(reference) = job["uses"].as_str() {
            assert!(
                reference.starts_with("agent-ix/engineering-assurance/.github/workflows/"),
                "{name} calls the trusted reusable workflow"
            );
            assert!(
                !reference.starts_with("./"),
                "{name} must not load local workflow code"
            );
            assert!(
                job.get("steps").is_none(),
                "{name} invokes only the reusable workflow"
            );
        } else {
            assert!(
                job["steps"]
                    .as_array()
                    .is_some_and(|steps| !steps.is_empty()),
                "{name} has explicit audited action steps"
            );
        }
    }

    let reusable_steps = reusable["jobs"]["attest"]["steps"]
        .as_array()
        .expect("reusable signing steps");
    assert!(
        !reusable_steps.is_empty(),
        "the reusable signing job is exercised"
    );
    for step in reusable_steps {
        assert!(
            step.get("run").is_none(),
            "signing steps do not run repository commands"
        );
        let reference = step["uses"].as_str().expect("signing step action");
        assert!(
            !reference.starts_with("./"),
            "signing steps do not load repository actions"
        );
        assert!(
            !reference.starts_with("actions/checkout@"),
            "signing steps do not check out repository code"
        );
        let (_, revision) = reference.split_once('@').expect("pinned signing action");
        assert_eq!(revision.len(), 40, "signing actions use full commit pins");
        assert!(revision.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
}

#[trace("TC-623", "NFR-011-AC-3")]
#[test]
fn signing_jobs_use_the_github_workflow_identity() {
    let workflow = reusable_workflow();
    let job = &workflow["jobs"]["attest"];
    assert_eq!(job["permissions"]["id-token"], "write");
    let steps = job["steps"].as_array().expect("attestation steps");
    let attest_steps = steps
        .iter()
        .filter(|step| {
            step["uses"]
                .as_str()
                .is_some_and(|reference| reference.starts_with("actions/attest@"))
        })
        .collect::<Vec<_>>();
    assert!(
        !attest_steps.is_empty(),
        "the workflow identity signs attestations"
    );

    for step in attest_steps {
        let inputs = step["with"].as_object().expect("attestation action inputs");
        for alternate_identity in ["identity", "issuer", "signer", "token"] {
            assert!(
                !inputs.contains_key(alternate_identity),
                "the action does not override workflow identity with {alternate_identity}"
            );
        }
        assert!(
            step.get("env").is_none(),
            "no alternate signing credential is injected"
        );
    }
    assert!(
        job.get("env").is_none(),
        "the signing job has no custom identity environment"
    );
}

#[trace("TC-562", "TC-667", "FR-070-AC-6", "FR-070-AC-11", "FR-067-AC-7")]
#[test]
fn reusable_release_workflow_checks_tag_and_fails_closed_before_signing_without_statement_source() {
    let workflow = reusable_workflow();
    let inputs = &workflow["on"]["workflow_call"]["inputs"];
    assert_eq!(
        inputs,
        &serde_json::json!({
            "tag": {"required": true, "type": "string"},
            "ea_tooling_tag": {"required": true, "type": "string"},
            "quoin_tooling_tag": {"required": true, "type": "string"},
            "record_path": {"required": true, "type": "string"},
            "allowed_signers_path": {"required": true, "type": "string"},
            "artifact_name": {"required": false, "type": "string", "default": "release-artifacts"},
            "evidence_artifact": {"required": false, "type": "string", "default": "release-evidence"}
        })
    );

    let jobs = &workflow["jobs"];
    let verify = &jobs["verify"];
    assert_eq!(
        verify["permissions"],
        serde_json::json!({"contents": "read"})
    );
    assert_eq!(
        verify["outputs"]["attestation_inputs_artifact"],
        "${{ steps.attestation-artifact-name.outputs.name }}"
    );
    let verify_steps = verify["steps"].as_array().expect("verify steps");
    assert!(verify_steps.iter().any(|step| {
        step["uses"]
            .as_str()
            .is_some_and(|reference| reference.starts_with("actions/download-artifact@"))
            && step["with"]["name"] == "${{ inputs.artifact_name }}"
    }));
    assert!(verify_steps.iter().any(|step| {
        step["uses"]
            .as_str()
            .is_some_and(|reference| reference.starts_with("actions/download-artifact@"))
            && step["with"]["name"] == "${{ inputs.evidence_artifact }}"
    }));
    assert!(verify_steps.iter().any(|step| {
        step["id"] == "attestation-artifact-name"
            && step["run"]
                .as_str()
                .is_some_and(|run| run.contains("secrets.token_hex(32)"))
    }));
    let tooling_download = verify_steps
        .iter()
        .find(|step| step["name"] == "Download trusted Engineering Assurance tooling")
        .expect("trusted EA release tooling is downloaded");
    assert_eq!(
        tooling_download["env"]["EA_TOOLING_TAG"],
        "${{ inputs.ea_tooling_tag }}"
    );
    let tooling_script = tooling_download["run"]
        .as_str()
        .expect("tool download script");
    assert!(tooling_script.contains("gh release download \"$EA_TOOLING_TAG\""));
    assert!(tooling_script.contains("--repo agent-ix/engineering-assurance"));
    let tag_check = verify_steps
        .iter()
        .position(|step| step["name"] == "Check the triggering release tag")
        .expect("trigger tag check exists");
    let release_check = verify_steps
        .iter()
        .position(|step| step["name"] == "Verify the signed release tag against protected policy")
        .expect("release-verify check exists");
    let release_check_run = verify_steps[release_check]["run"]
        .as_str()
        .expect("release verification script");
    assert!(
        release_check_run.contains("git -C protected-base show \"HEAD:$ALLOWED_SIGNERS_PATH\"")
    );
    let role_source_check = verify_steps
        .iter()
        .position(|step| step["name"] == "Refuse without a trusted approval-statement source")
        .expect("missing statement source refuses");
    assert!(tag_check < release_check && release_check < role_source_check);
    let missing_store = verify_steps
        .iter()
        .position(|step| step["name"] == "Refuse an empty retained release evidence artifact")
        .expect("empty or absent evidence refuses");
    let quoin_install = verify_steps
        .iter()
        .position(|step| step["name"] == "Install the exact published Quoin exporter")
        .expect("Quoin installer exists");
    assert!(missing_store < quoin_install && quoin_install < role_source_check);
    assert!(verify_steps.iter().any(|step| {
        step["run"]
            .as_str()
            .is_some_and(|run| run.contains("release_input_tag_mismatch"))
    }));
    assert!(verify_steps.iter().any(|step| {
        step["run"]
            .as_str()
            .is_some_and(|run| run.contains("no_retained_evidence_store"))
    }));
    assert!(verify_steps.iter().any(|step| {
        step["run"]
            .as_str()
            .is_some_and(|run| run.contains("release_roles_source_unavailable"))
    }));
    for step in verify_steps {
        if let Some(run) = step["run"].as_str() {
            assert!(!run.contains("${{"), "run values use env for expressions");
        }
    }

    let attest = &jobs["attest"];
    assert_eq!(attest["needs"], "verify");
    assert_eq!(
        attest["permissions"],
        serde_json::json!({
            "contents": "read",
            "id-token": "write",
            "attestations": "write"
        })
    );
    let attest_steps = attest["steps"].as_array().expect("attest steps");
    assert_eq!(attest_steps.len(), 6);
    for step in attest_steps {
        let reference = step["uses"].as_str().expect("pinned action reference");
        let (_, revision) = reference.split_once('@').expect("action has ref");
        assert_eq!(revision.len(), 40, "action is pinned by full commit SHA");
        assert!(revision.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
    assert!(attest_steps.iter().all(|step| step.get("run").is_none()));
    assert!(
        attest_steps
            .iter()
            .filter(|step| step["uses"]
                .as_str()
                .is_some_and(|name| name.starts_with("actions/attest@")))
            .all(|step| step["with"]["create-storage-record"] == false)
    );
    assert!(attest_steps.iter().any(|step| {
        step["with"]["predicate-type"] == concat!("https:", "//slsa.dev/verification_summary/v1")
    }));
    assert_eq!(
        attest_steps[1]["with"]["name"],
        "${{ needs.verify.outputs.attestation_inputs_artifact }}"
    );
}

#[trace("TC-669", "FR-070-AC-13")]
#[test]
fn reusable_workflow_installs_exact_quoin_version_from_explicit_npmjs_registry() {
    let workflow = reusable_workflow();
    let verify_steps = workflow["jobs"]["verify"]["steps"]
        .as_array()
        .expect("verify steps");
    let install = verify_steps
        .iter()
        .find(|step| step["name"] == "Install the exact published Quoin exporter")
        .expect("Quoin installer exists");
    assert_eq!(
        install["env"]["NPMJS_REGISTRY"],
        concat!("https:", "//registry.npmjs.org")
    );
    assert_eq!(install["env"]["NPM_CONFIG_USERCONFIG"], "/dev/null");
    assert_eq!(
        install["env"]["QUOIN_TOOLING_TAG"],
        "${{ inputs.quoin_tooling_tag }}"
    );
    assert_eq!(
        install["working-directory"],
        "${{ runner.temp }}/quoin-tooling"
    );
    let run = install["run"].as_str().expect("Quoin install script");
    assert!(run.contains("npm install --ignore-scripts --registry=\"$NPMJS_REGISTRY\" \"@agent-ix/quoin@$QUOIN_TOOLING_TAG\""));
    assert!(!run.contains("${{"), "input is passed through env");
    assert!(!run.contains("latest"), "the published version is exact");
    assert!(!run.contains("caller"), "the caller binary is not used");
}
