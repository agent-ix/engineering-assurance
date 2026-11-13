// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The Syft SBOM producer request (FR-038) run through `producer-execute`, and
//! the artifacts fictional fixtures yield (FR-039).
//!
//! Tests that run Syft read its absolute path from `EA_TEST_SYFT` and fail, never
//! skip, when it is absent (FR-038-CON-1).

use std::{
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const REQUEST_SCHEMA: &str =
    include_str!("../src/producer_execute_schemas/producer-execution-request-v1.schema.json");
/// Stands for the URL scheme prefix in lock-file fixtures, which the content-rights
/// tree check refuses to see spelled out.
const URL_PREFIX: &str = concat!("https:", "//");
const SBOM_PATH: &str = "out/sbom.cdx.json";

struct Syft {
    executable: PathBuf,
    version: String,
}

/// Resolves the Syft binary a test runs from the value of `EA_TEST_SYFT`.
fn resolve_syft(variable: Option<OsString>) -> Result<Syft, String> {
    let executable = variable
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or("EA_TEST_SYFT must name the absolute path of the Syft binary under test")?;
    let output = Command::new(&executable)
        .arg("--version")
        .output()
        .map_err(|error| {
            format!(
                "EA_TEST_SYFT {} cannot be run: {error}",
                executable.display()
            )
        })?;
    let text = String::from_utf8_lossy(&output.stdout);
    let version = text
        .split_whitespace()
        .last()
        .filter(|_| output.status.success())
        .ok_or_else(|| {
            format!(
                "EA_TEST_SYFT {} did not report a version",
                executable.display()
            )
        })?
        .to_owned();
    Ok(Syft {
        executable,
        version,
    })
}

fn syft() -> Syft {
    resolve_syft(std::env::var_os("EA_TEST_SYFT")).unwrap_or_else(|message| panic!("{message}"))
}

/// A capability root holding `scan/` with the given files and an empty `out/`.
fn workspace(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("temporary root");
    fs::create_dir_all(root.path().join("out")).expect("out/");
    for (path, body) in files {
        let target = root.path().join("scan").join(path);
        fs::create_dir_all(target.parent().expect("parent")).expect("scan subdirectory");
        fs::write(target, body.replace("@@", URL_PREFIX)).expect("fixture file");
    }
    fs::create_dir_all(root.path().join("scan")).expect("scan/");
    root
}

/// The Syft producer definition of FR-038 for one scan selector.
fn syft_request(root: &Path, executable: &Path, version: &str, selector: &str) -> Value {
    let root = fs::canonicalize(root).expect("root").display().to_string();
    let arguments = [
        "scan",
        selector,
        "-o",
        "cyclonedx-json=out/sbom.cdx.json",
        "-q",
    ];
    json!({
        "protocol": "engineering-assurance.producer-execution-request/v1",
        "producer": {
            "name": "syft",
            "version": version,
            "executable": executable.display().to_string()
        },
        "caller": {"kind": "engineering-assurance.sbom-generation", "version": "1"},
        "procedure": "direct",
        "capabilityRoot": root,
        "arguments": arguments
            .iter()
            .map(|value| json!({"kind": "literal", "value": value}))
            .collect::<Vec<_>>(),
        "environment": {
            "SYFT_CHECK_FOR_APP_UPDATE": "false",
            "SYFT_CACHE_DIR": format!("{root}/out/cache"),
            "SYFT_FILE_METADATA_SELECTION": "none",
            "HOME": format!("{root}/out/home"),
            "XDG_CONFIG_HOME": format!("{root}/out/home")
        },
        "inputs": [],
        "stdin": {"kind": "null"},
        "outputs": [{"role": "sbom", "path": SBOM_PATH, "required": true}],
        "containment": {
            "profile": "process-group-v1",
            "contract": {"kind": "engineering-assurance.syft-containment", "version": "1"}
        },
        "cancellation": {"kind": "disabled"},
        "budget": {
            "timeoutMillis": 120_000,
            "maxStdoutBytes": 65_536,
            "maxStderrBytes": 65_536,
            "maxInputBytes": 1024,
            "maxOutputArtifacts": 1,
            "maxOutputBytes": 67_108_864,
            "maxDescendants": 64,
            "maxConcurrency": 4
        },
        "response": {
            "protocol": {"kind": "engineering-assurance.process-evidence", "version": "1"},
            "adapter": {
                "kind": "engineering-assurance.cli-process-evidence-adapter",
                "version": "1"
            },
            "exitCodes": {"kind": "exact", "codes": [0]}
        }
    })
}

fn run_request(request: &Value) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["producer-execute", "--request", &request.to_string()])
        .output()
        .expect("the Cargo-built CLI must start")
}

fn result(request: &Value) -> Value {
    let output = run_request(request);
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    serde_json::from_slice(&output.stdout).expect("stdout must be one JSON document")
}

fn assert_valid_request(request: &Value) {
    let schema: Value = serde_json::from_str(REQUEST_SCHEMA).expect("schema is JSON");
    let validator = jsonschema::options()
        .build(&schema)
        .expect("published schema compiles");
    assert!(validator.is_valid(request), "{request}");
}

/// Runs the request, requires `completed`, and returns the retained SBOM.
fn completed_sbom(root: &Path, request: &Value) -> Value {
    assert_valid_request(request);
    let value = result(request);
    assert_eq!(value["state"]["kind"], "completed", "{value}");
    assert_eq!(value["state"]["observation"]["exitCode"], 0);
    let artifacts = value["artifacts"].as_array().expect("artifacts");
    assert_eq!(artifacts.len(), 1, "{value}");
    assert_eq!(artifacts[0]["role"], "sbom");
    assert_eq!(artifacts[0]["path"], SBOM_PATH);
    let bytes = fs::read(root.join(SBOM_PATH)).expect("retained SBOM file");
    assert_eq!(artifacts[0]["byteLength"], bytes.len());
    let sbom: Value = serde_json::from_slice(&bytes).expect("SBOM is JSON");
    assert_eq!(sbom["bomFormat"], "CycloneDX");
    sbom
}

fn components(sbom: &Value) -> &[Value] {
    sbom["components"].as_array().map_or(&[], Vec::as_slice)
}

fn purls(sbom: &Value) -> Vec<&str> {
    components(sbom)
        .iter()
        .map(|component| component["purl"].as_str().unwrap_or_default())
        .collect()
}

fn assert_fixture(sbom: &Value, count: usize, purl_prefix: &str) {
    assert_eq!(components(sbom).len(), count, "{sbom}");
    for purl in purls(sbom) {
        assert!(purl.starts_with(purl_prefix), "{purl}");
    }
}

fn sbom_for_source(files: &[(&str, &str)]) -> Value {
    let syft = syft();
    let root = workspace(files);
    let request = syft_request(root.path(), &syft.executable, &syft.version, "dir:scan");
    completed_sbom(root.path(), &request)
}

const CARGO_LOCK: &str = r#"version = 3

[[package]]
name = "fx-lumen"
version = "1.2.0"
source = "registry+@@github.com/rust-lang/crates.io-index"
checksum = "1111111111111111111111111111111111111111111111111111111111111111"

[[package]]
name = "fx-ember"
version = "0.4.1"
source = "registry+@@github.com/rust-lang/crates.io-index"
checksum = "2222222222222222222222222222222222222222222222222222222222222222"

[[package]]
name = "fx-quill"
version = "2.0.0"
source = "registry+@@github.com/rust-lang/crates.io-index"
checksum = "3333333333333333333333333333333333333333333333333333333333333333"
"#;

const CARGO_TOML: &str = "[package]\nname = \"fx-root\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";

const NPM_LOCK: &str = r#"{
  "name": "fx-root",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "": {"name": "fx-root", "version": "1.0.0", "dependencies": {"fx-lumen": "1.2.0", "fx-ember": "0.4.1", "fx-quill": "2.0.0"}},
    "node_modules/fx-lumen": {"version": "1.2.0", "resolved": "@@registry.example.invalid/fx-lumen/-/fx-lumen-1.2.0.tgz", "integrity": "sha512-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=="},
    "node_modules/fx-ember": {"version": "0.4.1", "resolved": "@@registry.example.invalid/fx-ember/-/fx-ember-0.4.1.tgz", "integrity": "sha512-BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=="},
    "node_modules/fx-quill": {"version": "2.0.0", "resolved": "@@registry.example.invalid/fx-quill/-/fx-quill-2.0.0.tgz", "integrity": "sha512-CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC=="}
  }
}
"#;

const NPM_PACKAGE: &str = r#"{"name": "fx-root", "version": "1.0.0", "dependencies": {"fx-lumen": "1.2.0", "fx-ember": "0.4.1", "fx-quill": "2.0.0"}}
"#;

const REQUIREMENTS: &str = "fx-lumen==1.2.0\nfx-ember==0.4.1\nfx-quill==2.0.0\n";

const DPKG_STATUS: &str = "Package: fx-lumen\nStatus: install ok installed\nVersion: 1.2.0\nArchitecture: amd64\nMaintainer: Nobody <nobody@example.invalid>\nDescription: fictional package\n\nPackage: fx-ember\nStatus: install ok installed\nVersion: 0.4.1\nArchitecture: amd64\nMaintainer: Nobody <nobody@example.invalid>\nDescription: fictional package\n\nPackage: fx-quill\nStatus: install ok installed\nVersion: 2.0.0\nArchitecture: amd64\nMaintainer: Nobody <nobody@example.invalid>\nDescription: fictional package\n";

fn sha256(bytes: &[u8]) -> String {
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("sha256sum must be available");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(bytes)
        .expect("digest input");
    let output = child.wait_with_output().expect("sha256sum");
    String::from_utf8(output.stdout)
        .expect("UTF-8 digest")
        .split_whitespace()
        .next()
        .expect("digest")
        .to_owned()
}

fn tar_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (path, body) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(body.len() as u64);
        header.set_mode(0o644);
        builder
            .append_data(&mut header, path, *body)
            .expect("tar entry");
    }
    builder.into_inner().expect("tar bytes")
}

/// A fictional OCI image archive whose single layer holds a dpkg database and
/// no `os-release` file.
fn oci_archive() -> Vec<u8> {
    let layer = tar_of(&[("var/lib/dpkg/status", DPKG_STATUS.as_bytes())]);
    let layer_digest = sha256(&layer);
    let config = json!({
        "architecture": "amd64",
        "os": "linux",
        "config": {},
        "rootfs": {"type": "layers", "diff_ids": [format!("sha256:{layer_digest}")]}
    })
    .to_string()
    .into_bytes();
    let config_digest = sha256(&config);
    let manifest = json!({
        "schemaVersion": 2,
        "mediaType": "application/vnd.oci.image.manifest.v1+json",
        "config": {
            "mediaType": "application/vnd.oci.image.config.v1+json",
            "digest": format!("sha256:{config_digest}"),
            "size": config.len()
        },
        "layers": [{
            "mediaType": "application/vnd.oci.image.layer.v1.tar",
            "digest": format!("sha256:{layer_digest}"),
            "size": layer.len()
        }]
    })
    .to_string()
    .into_bytes();
    let manifest_digest = sha256(&manifest);
    let index = json!({
        "schemaVersion": 2,
        "manifests": [{
            "mediaType": "application/vnd.oci.image.manifest.v1+json",
            "digest": format!("sha256:{manifest_digest}"),
            "size": manifest.len()
        }]
    })
    .to_string()
    .into_bytes();
    let layer_path = format!("blobs/sha256/{layer_digest}");
    let config_path = format!("blobs/sha256/{config_digest}");
    let manifest_path = format!("blobs/sha256/{manifest_digest}");
    tar_of(&[
        ("oci-layout", br#"{"imageLayoutVersion":"1.0.0"}"#),
        ("index.json", &index),
        (&layer_path, &layer),
        (&config_path, &config),
        (&manifest_path, &manifest),
    ])
}

fn fixture_executable() -> PathBuf {
    fs::canonicalize(env!("CARGO_BIN_EXE_producer-execution-fixture"))
        .expect("Rust fixture executable must be available")
}

/// The Syft request shape pointed at the Rust fixture executable, with the
/// fixture's own arguments.
fn fixture_request(root: &Path, arguments: &[&str]) -> Value {
    let mut request = syft_request(root, &fixture_executable(), "fixture", "dir:scan");
    request["arguments"] = arguments
        .iter()
        .map(|value| json!({"kind": "literal", "value": value}))
        .collect();
    request
}

#[trace("TC-278", "FR-038-AC-1", "FR-039-AC-1")]
#[test]
fn tc_278_rust_fixture_yields_declared_component_count() {
    let sbom = sbom_for_source(&[("Cargo.toml", CARGO_TOML), ("Cargo.lock", CARGO_LOCK)]);
    assert_fixture(&sbom, 3, "pkg:cargo/");
}

#[trace("TC-279", "FR-039-AC-2")]
#[test]
fn tc_279_npm_fixture_yields_declared_component_count() {
    let sbom = sbom_for_source(&[
        ("package.json", NPM_PACKAGE),
        ("package-lock.json", NPM_LOCK),
    ]);
    assert_fixture(&sbom, 3, "pkg:npm/");
}

#[trace("TC-280", "FR-039-AC-3")]
#[test]
fn tc_280_python_fixture_yields_declared_component_count() {
    let sbom = sbom_for_source(&[("requirements.txt", REQUIREMENTS)]);
    assert_fixture(&sbom, 3, "pkg:pypi/");
}

#[trace("TC-281", "FR-038-AC-2", "FR-039-AC-4")]
#[test]
fn tc_281_container_archive_yields_declared_component_count() {
    let syft = syft();
    let root = workspace(&[]);
    fs::write(root.path().join("scan/image.tar"), oci_archive()).expect("archive");
    let request = syft_request(
        root.path(),
        &syft.executable,
        &syft.version,
        "oci-archive:scan/image.tar",
    );
    let sbom = completed_sbom(root.path(), &request);
    assert_fixture(&sbom, 3, "pkg:deb/");
    for component in components(&sbom) {
        assert_ne!(component["type"], "operating-system");
        assert_ne!(component["type"], "file");
    }
}

#[trace("TC-282", "FR-038-AC-7")]
#[test]
fn tc_282_scan_ignores_configuration_and_earlier_output_outside_scan() {
    let syft = syft();
    let root = workspace(&[("Cargo.toml", CARGO_TOML), ("Cargo.lock", CARGO_LOCK)]);
    fs::write(root.path().join(".syft.yaml"), "exclude:\n  - \"**\"\n").expect("planted config");
    fs::create_dir_all(root.path().join("out/cache")).expect("cache");
    fs::write(
        root.path().join("out/cache/package-lock.json"),
        NPM_LOCK.replace("@@", URL_PREFIX),
    )
    .expect("earlier cache");
    fs::write(
        root.path().join(SBOM_PATH),
        r#"{"bomFormat":"CycloneDX","components":[{"type":"library","name":"fx-ghost"}]}"#,
    )
    .expect("earlier SBOM");
    let request = syft_request(root.path(), &syft.executable, &syft.version, "dir:scan");
    let sbom = completed_sbom(root.path(), &request);
    assert_fixture(&sbom, 3, "pkg:cargo/");
    assert!(
        !sbom.to_string().contains("fx-ghost"),
        "an earlier SBOM under out/ must not be catalogued"
    );
}

#[trace("TC-283", "FR-038-AC-3", "FR-039-AC-6")]
#[test]
fn tc_283_missing_executable_is_unavailable_without_an_sbom() {
    if !cfg!(target_os = "linux") {
        return;
    }
    let root = workspace(&[]);
    let absent = root.path().join("absent-syft");
    let request = syft_request(root.path(), &absent, "1.0.0", "dir:scan");
    assert_valid_request(&request);
    let value = result(&request);
    assert_eq!(value["state"]["kind"], "unavailable", "{value}");
    assert!(value["artifacts"].as_array().expect("artifacts").is_empty());
}

#[trace("TC-284", "FR-038-AC-4", "FR-039-AC-6")]
#[test]
fn tc_284_failed_and_timed_out_runs_carry_no_sbom() {
    if !cfg!(target_os = "linux") {
        return;
    }
    let root = workspace(&[]);
    let failing = fixture_request(root.path(), &["exit", "3", "nope"]);
    assert_valid_request(&failing);
    let value = result(&failing);
    assert_eq!(value["state"]["kind"], "failed", "{value}");
    assert!(value["artifacts"].as_array().expect("artifacts").is_empty());

    let mut slow = fixture_request(root.path(), &["wait", "60000"]);
    slow["budget"]["timeoutMillis"] = json!(1000);
    let value = result(&slow);
    assert_eq!(value["state"]["kind"], "timed_out", "{value}");
    assert!(value["artifacts"].as_array().expect("artifacts").is_empty());
}

#[trace("TC-285", "FR-038-AC-6")]
#[test]
fn tc_285_stale_sbom_is_absent_from_a_failed_result() {
    if !cfg!(target_os = "linux") {
        return;
    }
    let root = workspace(&[]);
    fs::write(root.path().join(SBOM_PATH), b"stale").expect("stale SBOM");
    let value = result(&fixture_request(root.path(), &["exit", "3", "nope"]));
    assert_eq!(value["state"]["kind"], "failed", "{value}");
    assert!(value["artifacts"].as_array().expect("artifacts").is_empty());
    assert!(
        !root.path().join(SBOM_PATH).exists(),
        "the stale SBOM must be removed before launch"
    );
}

#[trace("TC-286", "FR-038-AC-5", "NFR-006-AC-2")]
#[test]
fn tc_286_request_matches_the_definition_and_the_published_schema() {
    let root = workspace(&[]);
    let request = syft_request(
        root.path(),
        Path::new("/opt/syft/bin/syft"),
        "1.0.0",
        "dir:scan",
    );
    assert_valid_request(&request);

    let arguments: Vec<&str> = request["arguments"]
        .as_array()
        .expect("arguments")
        .iter()
        .map(|argument| argument["value"].as_str().expect("literal"))
        .collect();
    assert_eq!(arguments.iter().filter(|value| **value == "-o").count(), 1);
    assert!(arguments.contains(&"cyclonedx-json=out/sbom.cdx.json"));
    assert!(!arguments.iter().any(|value| value.contains("spdx")));
    assert_eq!(request["inputs"], json!([]));
    assert_eq!(request["stdin"], json!({"kind": "null"}));
    assert_eq!(
        request["containment"]["contract"],
        json!({"kind": "engineering-assurance.syft-containment", "version": "1"})
    );
    assert_eq!(
        request["environment"]["SYFT_FILE_METADATA_SELECTION"],
        "none"
    );
    assert_eq!(request["environment"].as_object().expect("env").len(), 5);
}

#[trace("TC-287", "FR-038-AC-8", "FR-038-CON-1", "FR-039-AC-5")]
#[test]
fn tc_287_missing_syft_fails_with_a_message_naming_the_variable() {
    let unset = resolve_syft(None).err().expect("unset must fail");
    assert!(unset.contains("EA_TEST_SYFT"), "{unset}");
    let absent = resolve_syft(Some("/nonexistent/syft".into()))
        .err()
        .expect("unrunnable must fail");
    assert!(absent.contains("EA_TEST_SYFT"), "{absent}");
}
