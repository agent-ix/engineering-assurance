// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Guard the Engineering Assurance reusable release workflow's contract.

use std::{fs, path::Path, process::Command};

use ix_trace_rs::trace;
use serde_json::Value;

mod common;
#[path = "common/release_workflow.rs"]
mod release_workflow;

use common::is_structural_action_reference;
use release_workflow::assert_action_references_and_checkouts;

fn repository_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn reusable_workflow() -> Value {
    let path = repository_root().join(".github/workflows/release-attestation.yml");
    let bytes = fs::read(&path).expect("release-attestation reusable workflow exists");
    yaml_serde::from_slice(&bytes).expect("release-attestation reusable workflow parses as YAML")
}

#[trace("TC-559", "TC-622", "FR-070-AC-3", "NFR-011-AC-1", "NFR-011-AC-2")]
#[test]
fn signing_jobs_run_no_agent_or_repository_code() {
    let reusable = reusable_workflow();
    let signing_jobs = [(&reusable["jobs"]["attest"], "reusable workflow")];

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
        assert!(
            is_structural_action_reference(reference),
            "signing action reference has a valid owner/repository/path@ref shape: {reference}"
        );
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

#[trace(
    "TC-557",
    "TC-562",
    "TC-667",
    "TC-668",
    "TC-559",
    "FR-070-AC-1",
    "FR-070-AC-3",
    "FR-070-AC-6",
    "FR-070-AC-11",
    "FR-070-AC-12",
    "FR-067-AC-7"
)]
#[test]
fn reusable_release_workflow_checks_tag_and_fails_closed_before_signing_without_statement_source() {
    let workflow = reusable_workflow();
    assert_action_references_and_checkouts(&workflow);
    assert_eq!(
        workflow["on"].as_object().map(serde_json::Map::len),
        Some(1),
        "the EA workflow is reusable only"
    );
    assert!(workflow["on"].get("workflow_call").is_some());
    let job_names = workflow["jobs"]
        .as_object()
        .expect("reusable workflow jobs")
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(job_names, ["attest", "verify"]);
    assert_workflow_inputs_and_permissions(&workflow);
    assert_verify_job(&workflow);
    assert_attest_job(&workflow);
}

#[trace("TC-665", "FR-070-AC-9")]
#[test]
fn reusable_workflow_downloads_named_release_and_evidence_artifacts_without_an_ea_caller() {
    let workflow = reusable_workflow();
    assert_workflow_inputs_and_permissions(&workflow);
    let inputs = &workflow["on"]["workflow_call"]["inputs"];
    assert_eq!(inputs["artifact_name"]["default"], "release-artifacts");
    assert_eq!(inputs["evidence_artifact"]["default"], "release-evidence");

    let verify_steps = workflow["jobs"]["verify"]["steps"]
        .as_array()
        .expect("verify steps");
    for (step_name, input, path) in [
        (
            "Download the release artifact",
            "artifact_name",
            "release-artifacts",
        ),
        (
            "Download retained evidence when present",
            "evidence_artifact",
            "release-evidence",
        ),
    ] {
        let download = verify_steps
            .iter()
            .find(|step| step["name"] == step_name)
            .expect("artifact download step exists");
        assert!(
            download["uses"]
                .as_str()
                .is_some_and(|reference| reference.starts_with("actions/download-artifact@"))
        );
        assert_eq!(
            download["with"]["name"],
            format!("${{{{ inputs.{input} }}}}")
        );
        assert_eq!(download["with"]["path"], path);
    }
    assert!(
        !repository_root()
            .join(".github/workflows/release.yml")
            .exists(),
        "the EA tag caller remains deferred until a retained store exists"
    );
}

#[cfg(unix)]
#[trace("TC-667", "FR-070-AC-11", "FR-070-AC-10")]
#[test]
fn nonempty_invalid_evidence_is_refused_before_the_role_source_failure() {
    use std::os::unix::fs::PermissionsExt;

    let workflow = reusable_workflow();
    let verify_steps = workflow["jobs"]["verify"]["steps"]
        .as_array()
        .expect("verify steps");
    let export_index = verify_steps
        .iter()
        .position(|step| {
            step["name"] == "Validate the retained evidence store with the Quoin exporter"
        })
        .expect("Quoin evidence validation step exists");
    let role_index = verify_steps
        .iter()
        .position(|step| step["name"] == "Refuse without a trusted approval-statement source")
        .expect("role-source refusal exists");
    assert!(
        export_index < role_index,
        "invalid evidence fails before role-source refusal"
    );
    let script = verify_steps[export_index]["run"]
        .as_str()
        .expect("Quoin evidence validation script");

    let root = tempfile::tempdir().expect("temporary workflow root");
    let workspace = root.path().join("workspace");
    let runner_temp = root.path().join("runner-temp");
    let artifacts = workspace.join("release-artifacts");
    let evidence = workspace.join("release-evidence");
    fs::create_dir_all(&workspace).expect("workflow workspace");
    fs::create_dir_all(&runner_temp).expect("runner temp directory");
    fs::create_dir_all(&artifacts).expect("release artifact directory");
    fs::write(artifacts.join("release.tar.gz"), b"release artifact")
        .expect("release artifact fixture");
    fs::create_dir_all(evidence.join("runs/suite")).expect("non-empty invalid evidence directory");
    fs::write(evidence.join("bindings.json"), b"{").expect("invalid non-empty evidence fixture");

    let fake_quoin = root.path().join("quoin");
    let args_capture = root.path().join("quoin-args.txt");
    fs::write(
        &fake_quoin,
        "#!/bin/bash\nprintf '%s\\n' \"$@\" > \"$QUOIN_ARGS_CAPTURE\"\nprintf '%s\\n' 'evidence_store_invalid: release-evidence/bindings.json: invalid JSON' >&2\nexit 3\n",
    )
    .expect("fake Quoin exporter script");
    let mut permissions = fs::metadata(&fake_quoin)
        .expect("fake Quoin metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_quoin, permissions).expect("fake Quoin executable");

    let output = Command::new("bash")
        .args(["-euo", "pipefail", "-c", script])
        .current_dir(&workspace)
        .env("RUNNER_TEMP", &runner_temp)
        .env("QUOIN_CLI", &fake_quoin)
        .env("QUOIN_ARGS_CAPTURE", &args_capture)
        .env("RELEASE_ARTIFACTS_DIR", &artifacts)
        .env("EVIDENCE_DIR", &evidence)
        .env("TEST_RESULT_OUTPUT", runner_temp.join("test-result.json"))
        .output()
        .expect("run the production workflow step");
    assert_eq!(output.status.code(), Some(1));
    let rendered = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        rendered.contains("no_retained_evidence_store"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("release_roles_source_unavailable"),
        "{rendered}"
    );
    assert!(!runner_temp.join("test-result.json").exists());

    let arguments = fs::read_to_string(args_capture).expect("Quoin invocation was captured");
    assert!(
        arguments.contains("evidence\nexport-test-result\n"),
        "{arguments}"
    );
    assert!(arguments.contains("--evidence-dir\n"), "{arguments}");
    assert!(arguments.contains("--subject\n"), "{arguments}");
    assert!(arguments.contains("--output\n"), "{arguments}");
    let subject_path = runner_temp.join("release-artifact-subject.json");
    let subject: Value = serde_json::from_slice(
        &fs::read(subject_path).expect("subject computed from downloaded artifact"),
    )
    .expect("computed subject is JSON");
    assert_eq!(subject[0]["name"], "release.tar.gz");
    assert_eq!(
        subject[0]["digest"]["sha256"].as_str().map(str::len),
        Some(64)
    );
}

#[cfg(unix)]
#[trace("TC-553", "FR-069-AC-1", "FR-069-AC-4", "FR-070-AC-10")]
#[test]
fn tc_553_failed_quoin_result_is_copied_to_attestation_inputs_without_rewriting() {
    use std::os::unix::fs::PermissionsExt;

    let workflow = reusable_workflow();
    let verify_steps = workflow["jobs"]["verify"]["steps"]
        .as_array()
        .expect("verify steps");
    let export = verify_steps
        .iter()
        .find(|step| step["name"] == "Validate the retained evidence store with the Quoin exporter")
        .expect("Quoin exporter step exists");
    let script = export["run"].as_str().expect("export script");
    assert_eq!(export["env"]["EVIDENCE_DIR"], "release-evidence");
    assert_eq!(export["env"]["RELEASE_ARTIFACTS_DIR"], "release-artifacts");
    assert!(script.contains("--subject \"$subject_path\""));
    assert!(script.contains("--evidence-dir \"$EVIDENCE_DIR\""));
    assert!(script.contains("--output \"$TEST_RESULT_OUTPUT\""));
    let attest_steps = workflow["jobs"]["attest"]["steps"]
        .as_array()
        .expect("attest steps");
    assert!(attest_steps.iter().all(|step| {
        step["run"]
            .as_str()
            .is_none_or(|run| !run.contains("quoin"))
    }));
    assert!(
        workflow["on"]["workflow_call"]["inputs"]
            .get("test_result")
            .is_none()
    );
    let attest_test_result = attest_steps
        .iter()
        .find(|step| step["name"] == "Attest Quoin test result")
        .expect("Test Result attestation step exists");
    assert_eq!(
        attest_test_result["with"]["predicate-type"],
        engineering_assurance::attestation_verify::TEST_RESULT_PREDICATE
    );
    assert_eq!(
        attest_test_result["with"]["predicate-path"],
        "attestation-inputs/test-result.json"
    );

    let root = tempfile::tempdir().expect("temporary workflow root");
    let workspace = root.path().join("workspace");
    let runner_temp = root.path().join("runner-temp");
    let artifacts = workspace.join("release-artifacts");
    let evidence = workspace.join("release-evidence");
    fs::create_dir_all(&workspace).expect("workflow workspace");
    fs::create_dir_all(&runner_temp).expect("runner temp directory");
    fs::create_dir_all(&artifacts).expect("release artifact directory");
    fs::write(artifacts.join("release.tar.gz"), b"fixture release").expect("release artifact");
    fs::create_dir_all(evidence.join("runs/suite")).expect("evidence run directory");
    fs::write(evidence.join("bindings.json"), "{}\n").expect("valid evidence bindings");

    let expected = concat!(
        r#"{"_type":"https:"#,
        r#"//in-toto.io/Statement/v1","predicateType":"https:"#,
        r#"//in-toto.io/attestation/test-result/v0.1","subject":[{"name":"release.tar.gz","digest":{"sha256":"fixture"}}],"predicate":{"result":"FAILED","failedTests":[{"name":"retained-check","result":"FAILED"}]}}"#
    );
    let fixture_result = root.path().join("fixture-test-result.json");
    fs::write(&fixture_result, expected.as_bytes()).expect("Quoin result fixture");
    let fake_quoin = root.path().join("quoin");
    fs::write(
        &fake_quoin,
        "#!/bin/bash\nset -euo pipefail\nwhile (($#)); do\n  if [[ \"$1\" == --output ]]; then shift; output=$1; fi\n  shift\ndone\ncp -- \"$FIXTURE_TEST_RESULT\" \"$output\"\n",
    )
    .expect("fake Quoin exporter");
    let mut permissions = fs::metadata(&fake_quoin)
        .expect("fake Quoin metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_quoin, permissions).expect("make fake Quoin executable");

    let output = Command::new("bash")
        .args(["-euo", "pipefail", "-c", script])
        .current_dir(&workspace)
        .env("RUNNER_TEMP", &runner_temp)
        .env("QUOIN_CLI", &fake_quoin)
        .env("FIXTURE_TEST_RESULT", &fixture_result)
        .env("RELEASE_ARTIFACTS_DIR", &artifacts)
        .env("EVIDENCE_DIR", &evidence)
        .env("TEST_RESULT_OUTPUT", runner_temp.join("test-result.json"))
        .output()
        .expect("run the production workflow step");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(workspace.join("attestation-inputs/test-result.json"))
            .expect("attestation input exists"),
        expected.as_bytes(),
        "the failed Quoin result is retained byte for byte for signing"
    );
}

#[cfg(unix)]
#[trace("TC-670", "FR-069-AC-2")]
#[test]
fn tc_670_missing_or_mismatched_quoin_export_stops_before_attestation() {
    use std::os::unix::fs::PermissionsExt;

    let workflow = reusable_workflow();
    assert_eq!(workflow["jobs"]["attest"]["needs"], "verify");
    let verify_steps = workflow["jobs"]["verify"]["steps"]
        .as_array()
        .expect("verify steps");
    let export = verify_steps
        .iter()
        .find(|step| step["name"] == "Validate the retained evidence store with the Quoin exporter")
        .expect("Quoin exporter step exists");
    let script = export["run"].as_str().expect("export script");

    for mode in ["missing", "mismatched-subject"] {
        let root = tempfile::tempdir().expect("temporary workflow root");
        let workspace = root.path().join("workspace");
        let runner_temp = root.path().join("runner-temp");
        let artifacts = workspace.join("release-artifacts");
        let evidence = workspace.join("release-evidence");
        fs::create_dir_all(&workspace).expect("workflow workspace");
        fs::create_dir_all(&runner_temp).expect("runner temp directory");
        fs::create_dir_all(&artifacts).expect("release artifact directory");
        fs::write(artifacts.join("release.tar.gz"), b"fixture release").expect("release artifact");
        fs::create_dir_all(evidence.join("runs/suite")).expect("evidence run directory");
        fs::write(evidence.join("bindings.json"), "{}\n").expect("evidence bindings");

        let fixture_result = root.path().join("fixture-test-result.json");
        fs::write(
            &fixture_result,
            serde_json::json!({
                "_type": concat!("https:", "//in-toto.io/Statement/v1"),
                "predicateType": concat!("https:", "//in-toto.io/attestation/test-result/v0.1"),
                "subject": [{
                    "name": "release.tar.gz",
                    "digest": {"sha256": "not-the-workflow-computed-digest"}
                }],
                "predicate": {"result": "FAILED", "failedTests": []}
            })
            .to_string(),
        )
        .expect("mismatched export fixture");

        let fake_quoin = root.path().join("quoin");
        let quoin_script = match mode {
            "missing" => "#!/bin/sh\nexit 0\n".to_owned(),
            "mismatched-subject" => {
                "#!/bin/sh\ncp -- \"$FIXTURE_TEST_RESULT\" \"$TEST_RESULT_OUTPUT\"\n".to_owned()
            }
            _ => unreachable!("mode is listed above"),
        };
        fs::write(&fake_quoin, quoin_script).expect("fake Quoin exporter");
        let mut permissions = fs::metadata(&fake_quoin)
            .expect("fake Quoin metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_quoin, permissions).expect("make fake Quoin executable");

        let output = Command::new("bash")
            .args(["-euo", "pipefail", "-c", script])
            .current_dir(&workspace)
            .env("RUNNER_TEMP", &runner_temp)
            .env("QUOIN_CLI", &fake_quoin)
            .env("FIXTURE_TEST_RESULT", &fixture_result)
            .env("RELEASE_ARTIFACTS_DIR", &artifacts)
            .env("EVIDENCE_DIR", &evidence)
            .env("TEST_RESULT_OUTPUT", runner_temp.join("test-result.json"))
            .output()
            .expect("run the production workflow step");
        assert_eq!(output.status.code(), Some(1), "{mode}");
        let rendered = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            rendered.contains("test_result_export_invalid"),
            "{mode}: {rendered}"
        );
        assert!(
            !workspace
                .join("attestation-inputs/test-result.json")
                .exists(),
            "{mode}: invalid exports are not handed to the signing job"
        );
    }
}

fn assert_workflow_inputs_and_permissions(workflow: &Value) {
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

    let verify = &workflow["jobs"]["verify"];
    assert_eq!(
        verify["permissions"],
        serde_json::json!({"contents": "read"})
    );
    assert_eq!(
        verify["outputs"]["attestation_inputs_artifact"],
        "${{ steps.attestation-artifact-name.outputs.name }}"
    );
}

fn assert_verify_job(workflow: &Value) {
    let verify = &workflow["jobs"]["verify"];
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
}

fn assert_attest_job(workflow: &Value) {
    let attest = &workflow["jobs"]["attest"];
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
        let reference = step["uses"].as_str().expect("action reference");
        assert!(
            is_structural_action_reference(reference),
            "attestation action reference has a valid owner/repository/path@ref shape: {reference}"
        );
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
