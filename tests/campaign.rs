// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Mechanical campaign contract and exact-request binding tests.

#![cfg(feature = "campaign")]

use std::{collections::BTreeMap, fs, process::Command};

use engineering_assurance::{
    campaign::{
        CAMPAIGN_DEFINITION_VERSION, CampaignAttemptStatus, CampaignDefinition, CampaignError,
        CampaignRun, CampaignSource, MeasurementProcedure, PlanRegistration, ProcedureBindings,
        SourceTreeBinding, canonical_digest, resolve_procedure, validate_definition,
        validate_procedure, validate_run,
    },
    producer_execution::{
        CancellationBinding, ContainmentBinding, ContentDigest, ContractBinding, ExecutionBudget,
        ExitCodeBinding, InputBinding, OutputBinding, OutputTreeBinding, ProducerDescriptor,
        StdinBinding,
    },
};
use ix_trace_rs::trace;
use serde_json::json;

fn procedure() -> MeasurementProcedure {
    serde_json::from_value(json!({
        "schemaVersion": "engineering-assurance.measurement-procedure/v1",
        "producerName": "fictional.producer",
        "producerVersion": "1.0",
        "sourceRepository": "fictional/source",
        "arguments": [
            {"kind": "literal", "value": "--check"},
            {"kind": "input_artifact", "value": "fixture"},
            {"kind": "output_artifact", "value": "report"}
        ],
        "environment": [
            {"name":"MIRIFLAGS","kind":"literal","value":"-Zmiri-strict-provenance"},
            {"name":"CARGO_HOME","kind":"runtime","value":"cargo_home"}
        ],
        "inputs": [{"role":"fixture","required":true}],
        "outputs": [{"role":"report","required":true}],
        "responseProtocol": "fictional.response",
        "responseAdapter": "fictional.adapter",
        "responseAdapterVersion": "1",
        "repetitions": 2,
        "timeoutMillis": 5000
    }))
    .expect("valid generated procedure wire")
}

fn sample_definition() -> CampaignDefinition {
    serde_json::from_value(json!({
        "schemaVersion": CAMPAIGN_DEFINITION_VERSION,
        "id": "fictional-campaign",
        "subjectName": "fictional.subject",
        "subjectVersion": "commit-1",
        "sourceGraph": [{
            "repository": "fictional/source",
            "revision": "commit-1",
            "digest": ContentDigest::of_bytes(b"tree inventory").as_str()
        }],
        "members": [
            {"name":"build", "group":"stage-a", "planId":"MP-001", "definitionVersion":"v1", "required":true},
            {"name":"verify", "group":"stage-a", "planId":"MP-002", "definitionVersion":"v1", "dependsOn":["build"], "required":true}
        ],
        "completionRule": "all_required"
    })).expect("valid generated definition wire")
}

fn plans(procedure: &MeasurementProcedure) -> [PlanRegistration<'_>; 2] {
    [
        PlanRegistration {
            id: "MP-001",
            definition_version: "v1",
            procedure: Some(procedure),
        },
        PlanRegistration {
            id: "MP-002",
            definition_version: "v1",
            procedure: Some(procedure),
        },
    ]
}

fn staged_plans<'a>(
    upstream: &'a MeasurementProcedure,
    downstream: &'a MeasurementProcedure,
) -> [PlanRegistration<'a>; 2] {
    [
        PlanRegistration {
            id: "MP-001",
            definition_version: "v1",
            procedure: Some(upstream),
        },
        PlanRegistration {
            id: "MP-002",
            definition_version: "v1",
            procedure: Some(downstream),
        },
    ]
}

fn contract(kind: &str) -> ContractBinding {
    ContractBinding {
        kind: kind.to_owned(),
        version: "1".to_owned(),
        revision: "commit-1".to_owned(),
        digest: ContentDigest::of_bytes(kind.as_bytes()),
    }
}

fn bindings() -> ProcedureBindings {
    ProcedureBindings {
        producer: ProducerDescriptor {
            name: "fictional.producer".to_owned(),
            version: "1.0".to_owned(),
            source_revision: "commit-1".to_owned(),
            executable: "/fictional/bin/producer".to_owned(),
            executable_digest: ContentDigest::of_bytes(b"executable"),
        },
        caller: contract("fictional.caller"),
        capability_root: "/fictional/sandbox".to_owned(),
        environment: BTreeMap::from([
            (
                "MIRIFLAGS".to_owned(),
                "-Zmiri-strict-provenance".to_owned(),
            ),
            ("CARGO_HOME".to_owned(), "/fictional/cargo".to_owned()),
        ]),
        inputs: vec![InputBinding {
            role: "fixture".to_owned(),
            path: "input.txt".to_owned(),
            digest: ContentDigest::of_bytes(b"input"),
            executable: false,
        }],
        input_origins: None,
        source_tree: None,
        outputs: vec![OutputBinding {
            role: "report".to_owned(),
            path: "report.json".to_owned(),
            required: true,
        }],
        output_trees: Vec::new(),
        stdin: StdinBinding::Null,
        containment: ContainmentBinding::ProcessGroupV1 {
            contract: contract("fictional.containment"),
        },
        cancellation: CancellationBinding::Disabled,
        budget: ExecutionBudget {
            timeout_millis: 20_000,
            max_stdout_bytes: 1024,
            max_stderr_bytes: 1024,
            max_input_bytes: 1024,
            max_output_artifacts: 1,
            max_output_bytes: 1024,
            max_descendants: 8,
            max_concurrency: 1,
        },
        response_protocol: contract("fictional.response"),
        response_adapter: contract("fictional.adapter"),
        exit_codes: ExitCodeBinding::Any,
    }
}

#[test]
#[trace("TC-177", "FR-019-AC-9", "VO-001", "VO-012", "EN-006")]
fn tc_177_generated_wire_is_closed_and_procedure_requires_valid_roles() {
    let mut value = serde_json::to_value(procedure()).expect("encode procedure");
    value["unknown"] = json!(true);
    assert!(serde_json::from_value::<MeasurementProcedure>(value).is_err());

    let mut value = serde_json::to_value(procedure()).expect("encode procedure");
    value["arguments"][1]["kind"] = json!("shell");
    assert!(serde_json::from_value::<MeasurementProcedure>(value).is_err());

    let mut invalid = procedure();
    invalid.repetitions = 0;
    assert_eq!(
        validate_procedure(&invalid),
        Err(CampaignError::Limit {
            field: "repetitions"
        })
    );
    invalid = procedure();
    invalid.arguments.as_mut().expect("arguments")[1].value = "undeclared".to_owned();
    assert_eq!(
        validate_procedure(&invalid),
        Err(CampaignError::Binding {
            field: "arguments.role"
        })
    );

    let mut origin = procedure();
    origin.input_origins = Some(
        serde_json::from_value(json!([{
            "role": "fixture", "kind": "source_file",
            "sourceRepository": "fictional/source", "sourcePath": "fixtures/input.txt"
        }]))
        .expect("source origin"),
    );
    validate_procedure(&origin).expect("complete source origin");

    origin.input_origins.as_mut().expect("origins")[0].source_path =
        Some("fixtures//input.txt".to_owned());
    assert_eq!(
        validate_procedure(&origin),
        Err(CampaignError::Binding {
            field: "inputOrigins.kind"
        })
    );
    origin.input_origins.as_mut().expect("origins")[0].source_path = None;
    assert_eq!(
        validate_procedure(&origin),
        Err(CampaignError::Binding {
            field: "inputOrigins.kind"
        })
    );
    origin.input_origins = Some(
        serde_json::from_value(json!([{
            "role": "fixture", "kind": "dependency",
            "dependencyMember": "prepare", "dependencyArtifactRole": "output"
        }]))
        .expect("dependency origin"),
    );
    validate_procedure(&origin).expect("complete dependency origin");
    origin.input_origins.as_mut().expect("origins")[0].source_repository =
        Some("fictional/source".to_owned());
    assert_eq!(
        validate_procedure(&origin),
        Err(CampaignError::Binding {
            field: "inputOrigins.kind"
        })
    );
    origin.input_origins = Some(
        serde_json::from_value(json!([{
            "role": "fixture", "kind": "selected_bytes"
        }]))
        .expect("selected bytes origin"),
    );
    validate_procedure(&origin).expect("complete selected bytes origin");
    origin.input_origins.as_mut().expect("origins")[0].source_path =
        Some("fixtures/input.txt".to_owned());
    assert_eq!(
        validate_procedure(&origin),
        Err(CampaignError::Binding {
            field: "inputOrigins.kind"
        })
    );
    origin.input_origins = Some(Vec::new());
    assert_eq!(
        validate_procedure(&origin),
        Err(CampaignError::Binding {
            field: "inputOrigins.complete"
        })
    );
}

#[test]
#[trace("TC-178", "FR-019-AC-10", "VO-001")]
fn tc_178_output_tree_is_declared_and_exactly_bound() {
    let mut value = serde_json::to_value(procedure()).expect("encode procedure");
    value["outputTrees"] = json!([{"role":"mutants", "required":true}]);
    let procedure: MeasurementProcedure =
        serde_json::from_value(value).expect("generated output-tree wire");
    validate_procedure(&procedure).expect("declared output tree");
    let source_graph = sample_definition().source_graph;
    assert!(matches!(
        resolve_procedure(&procedure, &source_graph, bindings()),
        Err(CampaignError::Binding {
            field: "outputTrees.required"
        })
    ));
    let mut bound = bindings();
    bound.output_trees.push(OutputTreeBinding {
        role: "mutants".to_owned(),
        path: "mutants.out".to_owned(),
        required: true,
    });
    let resolved =
        resolve_procedure(&procedure, &source_graph, bound.clone()).expect("bound output tree");
    bound.output_trees[0].path = "different.out".to_owned();
    assert_ne!(
        resolved.identity.digest,
        resolve_procedure(&procedure, &source_graph, bound)
            .expect("alternate path")
            .identity
            .digest
    );
}

#[test]
#[trace("TC-183", "FR-019-AC-10", "VO-011")]
fn tc_183_dynamic_input_prefixes_are_component_safe_and_required_when_declared() {
    let mut procedure = procedure();
    procedure.input_role_prefixes = Some(vec![
        serde_json::from_value(json!({"prefix":"dependency/","required":true}))
            .expect("generated prefix wire"),
    ]);
    let graph = sample_definition().source_graph;
    assert_eq!(
        resolve_procedure(&procedure, &graph, bindings()).err(),
        Some(CampaignError::Binding {
            field: "inputRolePrefixes.required"
        })
    );
    let mut bound = bindings();
    bound.inputs.push(InputBinding {
        role: "dependency/build".to_owned(),
        path: "dependency/build.json".to_owned(),
        digest: ContentDigest::of_bytes(b"sealed dependency"),
        executable: false,
    });
    resolve_procedure(&procedure, &graph, bound.clone()).expect("declared dynamic input");
    bound.inputs.push(InputBinding {
        role: "dependency-evil/build".to_owned(),
        path: "dependency/evil.json".to_owned(),
        digest: ContentDigest::of_bytes(b"undeclared dependency"),
        executable: false,
    });
    assert_eq!(
        resolve_procedure(&procedure, &graph, bound).err(),
        Some(CampaignError::Binding {
            field: "artifact role undeclared"
        })
    );
    procedure.input_role_prefixes.as_mut().expect("prefixes")[0].prefix = "dependency".to_owned();
    assert_eq!(
        validate_procedure(&procedure),
        Err(CampaignError::Binding {
            field: "inputRolePrefixes.prefix"
        })
    );
}

#[test]
#[trace("TC-184", "FR-019-AC-10", "VO-012", "EN-006")]
fn tc_184_source_origin_binds_campaign_graph_and_runtime_selection() {
    let definition = sample_definition();
    let base = procedure();
    let mut source = procedure();
    source.input_origins = Some(
        serde_json::from_value(json!([{
            "role": "fixture", "kind": "source_file",
            "sourceRepository": "ghost", "sourcePath": "fixtures/input.txt"
        }]))
        .expect("source origin"),
    );
    let missing_source = source.clone();
    let registrations = [
        PlanRegistration {
            id: "MP-001",
            definition_version: "v1",
            procedure: Some(&base),
        },
        PlanRegistration {
            id: "MP-002",
            definition_version: "v1",
            procedure: Some(&missing_source),
        },
    ];
    assert_eq!(
        validate_definition(&definition, &registrations),
        Err(CampaignError::Unresolved {
            kind: "input origin source repository",
            name: "ghost".to_owned(),
        })
    );
    source.input_origins.as_mut().expect("origins")[0].source_repository =
        Some("fictional/source".to_owned());
    let registrations = [
        PlanRegistration {
            id: "MP-001",
            definition_version: "v1",
            procedure: Some(&base),
        },
        PlanRegistration {
            id: "MP-002",
            definition_version: "v1",
            procedure: Some(&source),
        },
    ];
    validate_definition(&definition, &registrations).expect("tracked source repository");

    let mut selected = bindings();
    assert_eq!(
        resolve_procedure(&source, &definition.source_graph, selected.clone()).err(),
        Some(CampaignError::Binding {
            field: "inputOrigins.binding"
        })
    );
    selected.input_origins = source.input_origins.clone();
    resolve_procedure(&source, &definition.source_graph, selected.clone())
        .expect("matching selected source origin");
    selected.input_origins.as_mut().expect("origins")[0].source_path =
        Some("fixtures/different.txt".to_owned());
    assert_eq!(
        resolve_procedure(&source, &definition.source_graph, selected).err(),
        Some(CampaignError::Binding {
            field: "inputOrigins.binding"
        })
    );
}

#[test]
#[trace("TC-184", "FR-019-AC-10", "VO-012", "EN-006")]
fn tc_184_dependency_origin_binds_declared_member_and_refuses_extra_input() {
    let definition = sample_definition();
    let base = procedure();
    let mut dependent = procedure();
    dependent.input_origins = Some(
        serde_json::from_value(json!([{
            "role": "fixture", "kind": "dependency",
            "dependencyMember": "ghost", "dependencyArtifactRole": "output"
        }]))
        .expect("dependency origin"),
    );
    let missing_dependency = dependent.clone();
    let registrations = [
        PlanRegistration {
            id: "MP-001",
            definition_version: "v1",
            procedure: Some(&base),
        },
        PlanRegistration {
            id: "MP-002",
            definition_version: "v1",
            procedure: Some(&missing_dependency),
        },
    ];
    assert_eq!(
        validate_definition(&definition, &registrations),
        Err(CampaignError::Binding {
            field: "inputOrigins.dependencyMember"
        })
    );
    dependent.input_origins.as_mut().expect("origins")[0].dependency_member =
        Some("build".to_owned());
    let invalid_role = dependent.clone();
    let registrations = [
        PlanRegistration {
            id: "MP-001",
            definition_version: "v1",
            procedure: Some(&base),
        },
        PlanRegistration {
            id: "MP-002",
            definition_version: "v1",
            procedure: Some(&invalid_role),
        },
    ];
    assert_eq!(
        validate_definition(&definition, &registrations),
        Err(CampaignError::Binding {
            field: "inputOrigins.dependencyArtifactRole"
        })
    );
    dependent.input_origins.as_mut().expect("origins")[0].dependency_artifact_role =
        Some("report".to_owned());
    dependent.input_role_prefixes = Some(vec![
        serde_json::from_value(json!({"prefix":"dependency/","required":false}))
            .expect("runtime prefix"),
    ]);
    let registrations = [
        PlanRegistration {
            id: "MP-001",
            definition_version: "v1",
            procedure: Some(&base),
        },
        PlanRegistration {
            id: "MP-002",
            definition_version: "v1",
            procedure: Some(&dependent),
        },
    ];
    validate_definition(&definition, &registrations).expect("declared dependency origin");

    let mut selected = bindings();
    selected.input_origins = dependent.input_origins.clone();
    selected.inputs.push(InputBinding {
        role: "dependency/extra".to_owned(),
        path: "extra.txt".to_owned(),
        digest: ContentDigest::of_bytes(b"extra"),
        executable: false,
    });
    assert_eq!(
        resolve_procedure(&dependent, &definition.source_graph, selected).err(),
        Some(CampaignError::Binding {
            field: "inputOrigins.binding"
        })
    );
}

#[test]
#[trace("TC-184", "FR-019-AC-10", "VO-012", "EN-006")]
fn tc_184_dependency_origin_accepts_only_declared_output_tree_child() {
    let definition = sample_definition();
    let mut upstream = procedure();
    upstream.output_trees = Some(vec![
        serde_json::from_value(json!({"role":"mutants","required":false})).expect("output tree"),
    ]);
    let mut downstream = procedure();
    downstream.input_origins = Some(
        serde_json::from_value(json!([{
            "role":"fixture", "kind":"dependency", "dependencyMember":"build",
            "dependencyArtifactRole":"mutants/case.txt"
        }]))
        .expect("tree child origin"),
    );
    validate_definition(&definition, &staged_plans(&upstream, &downstream))
        .expect("declared tree child");
    downstream.input_origins.as_mut().expect("origins")[0].dependency_artifact_role =
        Some("mutants/../escape".to_owned());
    assert_eq!(
        validate_definition(&definition, &staged_plans(&upstream, &downstream)),
        Err(CampaignError::Binding {
            field: "inputOrigins.dependencyArtifactRole"
        })
    );
}

#[test]
#[trace("TC-179", "FR-019-AC-9", "VO-004")]
fn tc_179_campaign_rejects_stale_missing_and_cyclic_plan_graphs() {
    let procedure = procedure();
    let mut definition = sample_definition();
    validate_definition(&definition, &plans(&procedure)).expect("valid campaign");

    definition.members[0].definition_version = "v2".to_owned();
    assert!(matches!(
        validate_definition(&definition, &plans(&procedure)),
        Err(CampaignError::PlanVersion { .. })
    ));
    definition = sample_definition();
    definition.members[0].depends_on = Some(vec!["verify".to_owned()]);
    assert_eq!(
        validate_definition(&definition, &plans(&procedure)),
        Err(CampaignError::DependencyCycle)
    );
    definition = sample_definition();
    definition.members[1].depends_on = Some(vec!["missing".to_owned()]);
    assert!(matches!(
        validate_definition(&definition, &plans(&procedure)),
        Err(CampaignError::Unresolved {
            kind: "dependency",
            ..
        })
    ));
    definition = sample_definition();
    let missing = [
        PlanRegistration {
            id: "MP-001",
            definition_version: "v1",
            procedure: None,
        },
        PlanRegistration {
            id: "MP-002",
            definition_version: "v1",
            procedure: Some(&procedure),
        },
    ];
    assert!(matches!(
        validate_definition(&definition, &missing),
        Err(CampaignError::MissingProcedure { .. })
    ));
}

#[test]
#[trace("TC-180", "FR-019-AC-10", "VO-001")]
fn tc_180_resolved_request_identity_changes_with_every_selected_binding() {
    let procedure = procedure();
    let source_graph = sample_definition().source_graph;
    let baseline =
        resolve_procedure(&procedure, &source_graph, bindings()).expect("resolved baseline");
    assert_eq!(baseline.request.budget.timeout_millis, 5_000);
    let mut altered = bindings();
    altered.inputs[0].digest = ContentDigest::of_bytes(b"changed input");
    assert_ne!(
        baseline.identity.digest,
        resolve_procedure(&procedure, &source_graph, altered)
            .expect("input variant")
            .identity
            .digest
    );
    let mut altered = bindings();
    altered.producer.executable_digest = ContentDigest::of_bytes(b"changed executable");
    assert_ne!(
        baseline.identity.digest,
        resolve_procedure(&procedure, &source_graph, altered)
            .expect("executable variant")
            .identity
            .digest
    );
    let mut altered = bindings();
    altered.response_adapter.digest = ContentDigest::of_bytes(b"changed adapter");
    assert_ne!(
        baseline.identity.digest,
        resolve_procedure(&procedure, &source_graph, altered)
            .expect("adapter variant")
            .identity
            .digest
    );
    let mut altered = procedure.clone();
    altered.arguments.as_mut().expect("arguments")[0].value = "--other".to_owned();
    assert_ne!(
        baseline.identity.digest,
        resolve_procedure(&altered, &source_graph, bindings())
            .expect("argument variant")
            .identity
            .digest
    );
    let mut altered = bindings();
    altered.producer.source_revision = "stale".to_owned();
    assert_eq!(
        resolve_procedure(&procedure, &source_graph, altered).err(),
        Some(CampaignError::Binding {
            field: "producer.sourceRevision"
        })
    );
    let mut altered = bindings();
    altered
        .environment
        .insert("MIRIFLAGS".to_owned(), "unsafe-flag".to_owned());
    assert_eq!(
        resolve_procedure(&procedure, &source_graph, altered).err(),
        Some(CampaignError::Binding {
            field: "environment.literal"
        })
    );
    let mut altered = bindings();
    altered
        .environment
        .insert("CARGO_HOME".to_owned(), "/other/cargo".to_owned());
    assert_ne!(
        baseline.identity.digest,
        resolve_procedure(&procedure, &source_graph, altered)
            .expect("runtime environment variant")
            .identity
            .digest
    );
}

#[test]
#[trace("TC-181", "FR-019-AC-11", "VO-007")]
fn tc_181_run_inventory_rejects_stale_digest_and_unknown_attempt() {
    let definition = sample_definition();
    let mut run: CampaignRun = serde_json::from_value(json!({
        "schemaVersion":"engineering-assurance.campaign-run/v1",
        "id":"run-1",
        "definitionDigest":canonical_digest(&definition).expect("definition digest").as_str(),
        "sourceGraphDigest":canonical_digest(&definition.source_graph).expect("graph digest").as_str(),
        "attempts":[{"member":"build","index":1,"status":"invalid_request"}],
        "verdict":"inconclusive"
    })).expect("valid run wire");
    validate_run(&run, &definition).expect("valid inventory");
    assert_eq!(
        run.attempts.as_ref().expect("attempts")[0].status,
        CampaignAttemptStatus::InvalidRequest
    );
    run.attempts.as_mut().expect("attempts")[0].member = "missing".to_owned();
    assert!(matches!(
        validate_run(&run, &definition),
        Err(CampaignError::Unresolved {
            kind: "attempt member",
            ..
        })
    ));
    run = serde_json::from_value(json!({
        "schemaVersion":"engineering-assurance.campaign-run/v1", "id":"run-1",
        "definitionDigest":"0".repeat(64),
        "sourceGraphDigest":canonical_digest(&definition.source_graph).expect("graph digest").as_str(),
        "verdict":"inconclusive"
    })).expect("stale run wire");
    assert_eq!(
        validate_run(&run, &definition),
        Err(CampaignError::Binding {
            field: "definitionDigest"
        })
    );
}

fn git(root: &std::path::Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Fictional Tester")
        .env("GIT_AUTHOR_EMAIL", "tester@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fictional Tester")
        .env("GIT_COMMITTER_EMAIL", "tester@example.invalid")
        .output()
        .expect("git available");
    assert!(
        output.status.success(),
        "git command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
#[trace("TC-182", "TC-183", "FR-019-AC-10", "FR-019-AC-11", "VO-005", "VO-011")]
fn tc_182_source_projection_checks_complete_git_blob_inventory() {
    let temp_root = if cfg!(target_os = "macos") {
        "/private/tmp"
    } else {
        "/tmp"
    };
    let repo = tempfile::tempdir_in(temp_root).expect("temporary repository");
    fs::create_dir(repo.path().join("scripts")).expect("scripts directory");
    fs::write(
        repo.path().join("Cargo.toml"),
        b"[package]\nname = \"fixture\"\n",
    )
    .expect("manifest");
    fs::write(repo.path().join("scripts/check.sh"), b"#!/bin/sh\nexit 0\n").expect("script");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            repo.path().join("scripts/check.sh"),
            fs::Permissions::from_mode(0o755),
        )
        .expect("executable script");
        std::os::unix::fs::symlink("../Cargo.toml", repo.path().join("scripts/manifest-link"))
            .expect("tracked symlink");
    }
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["add", "."]);
    git(
        repo.path(),
        &[
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "-m",
            "fixture",
        ],
    );
    let revision = String::from_utf8(git(repo.path(), &["rev-parse", "HEAD"]))
        .expect("revision UTF-8")
        .trim()
        .to_owned();
    let manifest = git(
        repo.path(),
        &["ls-tree", "-r", "-z", "--full-tree", &revision],
    );
    let source: CampaignSource = serde_json::from_value(json!({
        "repository":"fictional/source", "revision":revision,
        "digest":ContentDigest::of_bytes(&manifest).as_str()
    }))
    .expect("source wire");
    let mut procedure = procedure();
    procedure.inputs = None;
    procedure
        .arguments
        .as_mut()
        .expect("arguments")
        .retain(|argument| argument.value != "fixture");
    let mut bindings = bindings();
    bindings.capability_root = repo
        .path()
        .canonicalize()
        .expect("canonical temporary root")
        .display()
        .to_string();
    bindings.producer.source_revision = revision;
    bindings.inputs.clear();
    bindings.source_tree = Some(SourceTreeBinding {
        repository: "fictional/source".to_owned(),
        manifest: manifest.clone(),
    });
    let resolved = resolve_procedure(&procedure, std::slice::from_ref(&source), bindings.clone())
        .expect("sealed source tree");
    assert_eq!(resolved.request.inputs.len(), 2);
    assert_eq!(resolved.omitted_source_links.len(), 1);
    assert_required_source_prefix(&procedure, &source, bindings.clone());
    assert_eq!(
        resolved.omitted_source_links[0].path,
        "scripts/manifest-link"
    );
    assert!(
        resolved
            .request
            .inputs
            .iter()
            .any(|input| input.role == "source/Cargo.toml")
    );
    #[cfg(unix)]
    assert!(
        resolved
            .request
            .inputs
            .iter()
            .any(|input| input.role == "source-exec/scripts/check.sh")
    );

    assert_source_tampering_rejected(repo.path(), &procedure, &source, bindings);
}

fn assert_required_source_prefix(
    procedure: &MeasurementProcedure,
    source: &CampaignSource,
    bindings: ProcedureBindings,
) {
    let mut required_source = procedure.clone();
    required_source.input_role_prefixes = Some(vec![
        serde_json::from_value(json!({"prefix":"source/","required":true}))
            .expect("generated prefix wire"),
    ]);
    let source_only = resolve_procedure(&required_source, std::slice::from_ref(source), bindings)
        .expect("verified source tree satisfies required source prefix");
    assert!(
        source_only
            .request
            .inputs
            .iter()
            .any(|input| input.role == "source/Cargo.toml")
    );
}

fn assert_source_tampering_rejected(
    repo: &std::path::Path,
    procedure: &MeasurementProcedure,
    source: &CampaignSource,
    bindings: ProcedureBindings,
) {
    fs::write(repo.join("Cargo.toml"), b"changed source\n").expect("tamper source");
    assert_eq!(
        resolve_procedure(procedure, std::slice::from_ref(source), bindings.clone()).err(),
        Some(CampaignError::SourceTree {
            field: "Git blob mismatch"
        })
    );
    fs::write(repo.join("Cargo.toml"), b"[package]\nname = \"fixture\"\n")
        .expect("restore regular file");
    fs::remove_file(repo.join("scripts/manifest-link")).expect("remove link");
    std::os::unix::fs::symlink("../different", repo.join("scripts/manifest-link"))
        .expect("changed link target");
    assert_eq!(
        resolve_procedure(procedure, std::slice::from_ref(source), bindings.clone()).err(),
        Some(CampaignError::SourceTree {
            field: "Git blob mismatch"
        })
    );
    let mut altered = bindings.clone();
    altered.source_tree.as_mut().expect("tree").manifest.push(0);
    assert_eq!(
        resolve_procedure(procedure, std::slice::from_ref(source), altered).err(),
        Some(CampaignError::Binding {
            field: "sourceTree.manifestDigest"
        })
    );
    fs::remove_file(repo.join("scripts/manifest-link")).expect("remove changed link");
    std::os::unix::fs::symlink("../Cargo.toml", repo.join("scripts/manifest-link"))
        .expect("restore tracked link");
    fs::rename(repo.join("scripts"), repo.join("scripts-real")).expect("move original directory");
    std::os::unix::fs::symlink("scripts-real", repo.join("scripts"))
        .expect("substitute parent link");
    assert_eq!(
        resolve_procedure(procedure, std::slice::from_ref(source), bindings).err(),
        Some(CampaignError::SourceTree {
            field: "source parent directory"
        })
    );
}
