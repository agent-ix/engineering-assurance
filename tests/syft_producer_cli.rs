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
    for directory in ["out/home", "out/xdg", "out/tmp"] {
        fs::create_dir_all(root.path().join(directory)).expect("out/ subdirectory");
    }
    fs::write(root.path().join("out/empty-config.yaml"), "").expect("empty config");
    for (path, body) in files {
        let target = root.path().join("scan").join(path);
        fs::create_dir_all(target.parent().expect("parent")).expect("scan subdirectory");
        fs::write(target, body).expect("fixture file");
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
        "--config",
        "out/empty-config.yaml",
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
            "XDG_CONFIG_HOME": format!("{root}/out/home"),
            "XDG_CONFIG_DIRS": format!("{root}/out/xdg"),
            "TMPDIR": format!("{root}/out/tmp")
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
source = "registry+fictional"
checksum = "1111111111111111111111111111111111111111111111111111111111111111"

[[package]]
name = "fx-ember"
version = "0.4.1"
source = "registry+fictional"
checksum = "2222222222222222222222222222222222222222222222222222222222222222"

[[package]]
name = "fx-quill"
version = "2.0.0"
source = "registry+fictional"
checksum = "3333333333333333333333333333333333333333333333333333333333333333"
"#;

const CARGO_TOML: &str = "[package]\nname = \"fx-root\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";

const NPM_LOCK: &str = r#"{
  "name": "fx-root",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "requires": true,
  "packages": {
    "": {"dependencies": {"fx-lumen": "1.2.0", "fx-ember": "0.4.1", "fx-quill": "2.0.0"}},
    "node_modules/fx-lumen": {"version": "1.2.0"},
    "node_modules/fx-ember": {"version": "0.4.1"},
    "node_modules/fx-quill": {"version": "2.0.0"}
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

#[trace("TC-300", "FR-038-AC-1", "FR-039-AC-1")]
#[test]
#[ignore = "needs EA_TEST_SYFT"]
fn tc_300_rust_fixture_yields_declared_component_count() {
    let sbom = sbom_for_source(&[("Cargo.toml", CARGO_TOML), ("Cargo.lock", CARGO_LOCK)]);
    assert_fixture(&sbom, 3, "pkg:cargo/");
}

#[trace("TC-301", "FR-039-AC-2")]
#[test]
#[ignore = "needs EA_TEST_SYFT"]
fn tc_301_npm_fixture_yields_declared_component_count() {
    let sbom = sbom_for_source(&[
        ("package.json", NPM_PACKAGE),
        ("package-lock.json", NPM_LOCK),
    ]);
    assert_fixture(&sbom, 3, "pkg:npm/");
}

#[trace("TC-377", "FR-039-AC-8")]
#[test]
#[ignore = "needs EA_TEST_SYFT"]
fn tc_377_licence_fixture_components_carry_the_expected_licences() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/licence-npm");
    let lock = fs::read_to_string(fixture.join("scan/package-lock.json")).expect("lock");
    let package = fs::read_to_string(fixture.join("scan/package.json")).expect("package");
    let expected: Value = serde_json::from_str(
        &fs::read_to_string(fixture.join("expected.json")).expect("expected-result file"),
    )
    .expect("expected-result file is JSON");
    let expected = expected.as_object().expect("purl map");
    let sbom = sbom_for_source(&[("package.json", &package), ("package-lock.json", &lock)]);
    assert_fixture(&sbom, expected.len(), "pkg:npm/");
    assert!(
        components(&sbom)
            .iter()
            .any(|component| component.get("licenses").is_some()),
        "no component carries licenses: {sbom}"
    );
    for (purl, want) in expected {
        let component = components(&sbom)
            .iter()
            .find(|component| component["purl"] == purl.as_str())
            .unwrap_or_else(|| panic!("no component for {purl}: {sbom}"));
        let licenses = component.get("licenses");
        match want["field"].as_str().expect("field") {
            "none" => assert!(licenses.is_none(), "{purl}: {licenses:?}"),
            field => {
                let entries = licenses.and_then(Value::as_array).expect("licenses");
                assert_eq!(entries.len(), 1, "{purl}: {entries:?}");
                let got = match field {
                    "id" => &entries[0]["license"]["id"],
                    _ => &entries[0]["expression"],
                };
                assert_eq!(got, &want["text"], "{purl}: {entries:?}");
            }
        }
    }
}

#[trace("TC-302", "FR-039-AC-3")]
#[test]
#[ignore = "needs EA_TEST_SYFT"]
fn tc_302_python_fixture_yields_declared_component_count() {
    let sbom = sbom_for_source(&[("requirements.txt", REQUIREMENTS)]);
    assert_fixture(&sbom, 3, "pkg:pypi/");
}

#[trace("TC-303", "FR-038-AC-2", "FR-039-AC-4")]
#[test]
#[ignore = "needs EA_TEST_SYFT"]
fn tc_303_container_archive_yields_declared_component_count() {
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
    let mut names: Vec<&str> = components(&sbom)
        .iter()
        .map(|component| component["name"].as_str().unwrap_or_default())
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["fx-ember", "fx-lumen", "fx-quill"], "{sbom}");
    for component in components(&sbom) {
        assert_eq!(component["type"], "library");
    }
}

#[trace("TC-304", "FR-038-AC-7")]
#[test]
#[ignore = "needs EA_TEST_SYFT"]
fn tc_304_scan_ignores_planted_configuration_and_earlier_output() {
    let syft = syft();
    let planted = "exclude:\n  - \"**\"\n";
    let root = workspace(&[
        ("Cargo.toml", CARGO_TOML),
        ("Cargo.lock", CARGO_LOCK),
        (".syft.yaml", planted),
        (".syft/config.yaml", planted),
    ]);
    fs::create_dir_all(root.path().join("out/cache")).expect("cache");
    fs::write(root.path().join("out/cache/package-lock.json"), NPM_LOCK).expect("earlier cache");
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

#[cfg(target_os = "linux")]
#[trace("TC-305", "FR-038-AC-3", "FR-039-AC-6")]
#[test]
fn tc_305_missing_executable_is_unavailable_without_an_sbom() {
    let root = workspace(&[]);
    let absent = root.path().join("absent-syft");
    let request = syft_request(root.path(), &absent, "1.0.0", "dir:scan");
    assert_valid_request(&request);
    let value = result(&request);
    assert_eq!(value["state"]["kind"], "unavailable", "{value}");
    assert!(value["artifacts"].as_array().expect("artifacts").is_empty());
}

#[cfg(target_os = "linux")]
#[trace("TC-306", "FR-038-AC-4", "FR-039-AC-6")]
#[test]
fn tc_306_failed_and_timed_out_runs_carry_no_sbom() {
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

#[cfg(target_os = "linux")]
#[trace("TC-307", "FR-038-AC-6")]
#[test]
fn tc_307_stale_sbom_is_absent_from_a_failed_result() {
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

#[trace("TC-308", "FR-038-AC-5", "NFR-006-AC-2")]
#[test]
fn tc_308_request_matches_the_definition_and_the_published_schema() {
    let root = workspace(&[]);
    let request = syft_request(
        root.path(),
        Path::new("/opt/syft/bin/syft"),
        "1.0.0",
        "dir:scan",
    );
    assert_valid_request(&request);

    let base = fs::canonicalize(root.path())
        .expect("root")
        .display()
        .to_string();
    let literals: Vec<Value> = [
        "scan",
        "dir:scan",
        "-o",
        "cyclonedx-json=out/sbom.cdx.json",
        "-q",
        "--config",
        "out/empty-config.yaml",
    ]
    .iter()
    .map(|value| json!({"kind": "literal", "value": value}))
    .collect();
    assert_eq!(request["arguments"], json!(literals));
    assert_eq!(
        request["environment"],
        json!({
            "SYFT_CHECK_FOR_APP_UPDATE": "false",
            "SYFT_CACHE_DIR": format!("{base}/out/cache"),
            "SYFT_FILE_METADATA_SELECTION": "none",
            "HOME": format!("{base}/out/home"),
            "XDG_CONFIG_HOME": format!("{base}/out/home"),
            "XDG_CONFIG_DIRS": format!("{base}/out/xdg"),
            "TMPDIR": format!("{base}/out/tmp")
        })
    );
    assert_eq!(request["inputs"], json!([]));
    assert_eq!(request["stdin"], json!({"kind": "null"}));
    assert_eq!(
        request["containment"],
        json!({
            "profile": "process-group-v1",
            "contract": {"kind": "engineering-assurance.syft-containment", "version": "1"}
        })
    );
    assert_eq!(
        request["outputs"],
        json!([{"role": "sbom", "path": "out/sbom.cdx.json", "required": true}])
    );
}

#[trace("TC-309", "FR-038-AC-8", "FR-038-CON-1", "FR-039-AC-5")]
#[test]
fn tc_309_missing_syft_fails_the_integration_target_naming_the_variable() {
    let not_executable = tempfile::NamedTempFile::new().expect("plain file");
    for syft in [None, Some("relative/syft"), not_executable.path().to_str()] {
        let mut make = Command::new("make");
        make.args(["-C", env!("CARGO_MANIFEST_DIR"), "syft-integration"])
            .env_remove("EA_TEST_SYFT");
        if let Some(path) = syft {
            make.env("EA_TEST_SYFT", path);
        }
        let output = make.output().expect("make must start");
        assert_eq!(output.status.code(), Some(2), "{syft:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("EA_TEST_SYFT"), "{stderr}");
    }
    let message = resolve_syft(None).err().expect("unset must fail");
    assert!(message.contains("EA_TEST_SYFT"), "{message}");
}
