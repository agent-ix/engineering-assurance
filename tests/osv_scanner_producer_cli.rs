// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The OSV-Scanner producer request run through `producer-execute`.
//!
//! Scanner and database dependent cases are ignored by default and run only in
//! `make osv-scanner-integration`. Fixture scanner cases exercise executor
//! states; they do not create advisory evidence.

use std::{
    ffi::OsString,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const REQUEST_SCHEMA: &str =
    include_str!("../src/producer_execute_schemas/producer-execution-request-v1.schema.json");
const SBOM_PATH: &str = "in/sbom.cdx.json";
const REPORT_PATH: &str = "out/osv-scanner.json";
const OS_NAME: &str = "libwebp";
const OS_VERSION: &str = "0.6.1-2.1+deb11u1";
const OS_PURL: &str = "pkg:deb/debian/libwebp@0.6.1-2.1%2Bdeb11u1?arch=amd64&distro=debian-11";
const EXPECTED_OSV_ID: &str = "DEBIAN-CVE-2023-4863";
const EXPECTED_CVE_ALIAS: &str = "CVE-2023-4863";

struct OsvScanner {
    executable: PathBuf,
    version: String,
}

fn resolve_osv(scanner: Option<OsString>) -> Result<OsvScanner, String> {
    let executable = scanner
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or("EA_TEST_OSV_SCANNER must name the absolute path of the OSV-Scanner binary")?;
    let metadata = fs::metadata(&executable).map_err(|error| {
        format!(
            "EA_TEST_OSV_SCANNER {} cannot be read: {error}",
            executable.display()
        )
    })?;
    if !metadata.is_file() || !is_executable(&metadata) {
        return Err(format!(
            "EA_TEST_OSV_SCANNER {} must be an executable regular file",
            executable.display()
        ));
    }
    let output = Command::new(&executable)
        .arg("--version")
        .output()
        .map_err(|error| format!("EA_TEST_OSV_SCANNER cannot be run: {error}"))?;
    let text = String::from_utf8_lossy(&output.stdout);
    let version = text
        .split_whitespace()
        .last()
        .filter(|_| output.status.success())
        .ok_or_else(|| "EA_TEST_OSV_SCANNER did not report a version".to_owned())?
        .to_owned();
    Ok(OsvScanner {
        executable,
        version,
    })
}

#[cfg(unix)]
fn is_executable(metadata: &fs::Metadata) -> bool {
    metadata.permissions().mode() & 0o111 != 0
}

fn scanner() -> OsvScanner {
    resolve_osv(std::env::var_os("EA_TEST_OSV_SCANNER"))
        .unwrap_or_else(|message| panic!("{message}"))
}

fn database_root(variable: Option<OsString>) -> Result<PathBuf, String> {
    let root = variable
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or("EA_TEST_OSV_DB must name the absolute path of a local database directory")?;
    let metadata = fs::metadata(&root)
        .map_err(|error| format!("EA_TEST_OSV_DB {} cannot be read: {error}", root.display()))?;
    if !metadata.is_dir() {
        return Err(format!(
            "EA_TEST_OSV_DB {} must be a directory",
            root.display()
        ));
    }
    let has_database = fs::read_dir(root.join("osv-scalibr"))
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .any(|ecosystem| ecosystem.path().join("all.zip").is_file());
    if !has_database {
        return Err(format!(
            "EA_TEST_OSV_DB {} must contain osv-scalibr/<ecosystem>/all.zip",
            root.display()
        ));
    }
    if root.join("osv-scalibr/wolfi/all.zip").is_file() {
        return Err("EA_TEST_OSV_DB must not include the Wolfi ecosystem".to_owned());
    }
    Ok(root)
}

fn database() -> PathBuf {
    database_root(std::env::var_os("EA_TEST_OSV_DB")).unwrap_or_else(|message| panic!("{message}"))
}

fn copy_database(source: &Path, destination: &Path) {
    let ecosystems =
        fs::read_dir(source.join("osv-scalibr")).expect("database has osv-scalibr directory");
    for ecosystem in ecosystems {
        let ecosystem = ecosystem.expect("database ecosystem entry");
        let name = ecosystem.file_name();
        assert!(
            name != "wolfi",
            "EA_TEST_OSV_DB must not include the Wolfi ecosystem"
        );
        let archive = ecosystem.path().join("all.zip");
        if archive.is_file() {
            let target = destination.join("osv-scalibr").join(name).join("all.zip");
            fs::create_dir_all(target.parent().expect("database parent"))
                .expect("database directory");
            fs::copy(archive, target).expect("copy caller-supplied database archive");
        }
    }
}

/// Creates isolated input, config, home, cache and temporary directories.
fn workspace(sbom: &[u8]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("temporary root");
    for directory in ["in", "db", "out/home", "out/xdg", "out/tmp"] {
        fs::create_dir_all(root.path().join(directory)).expect("capability subdirectory");
    }
    fs::write(root.path().join("out/empty-config.toml"), b"").expect("empty config");
    fs::write(root.path().join(SBOM_PATH), sbom).expect("retained SBOM copy");
    root
}

fn osv_request(root: &Path, executable: &Path, version: &str, db: &Path) -> Value {
    let root = fs::canonicalize(root).expect("root").display().to_string();
    let db = fs::canonicalize(db)
        .expect("database")
        .display()
        .to_string();
    let arguments = [
        "scan",
        "source",
        "--offline",
        "--all-packages",
        "--format",
        "json",
        "--output-file",
        REPORT_PATH,
        "--config",
        "out/empty-config.toml",
        "-L",
        SBOM_PATH,
    ];
    json!({
        "protocol": "engineering-assurance.producer-execution-request/v1",
        "producer": {
            "name": "osv-scanner",
            "version": version,
            "executable": executable.display().to_string()
        },
        "caller": {"kind": "engineering-assurance.advisory-scan", "version": "1"},
        "procedure": "direct",
        "capabilityRoot": root,
        "arguments": arguments.iter().map(|value| json!({"kind": "literal", "value": value})).collect::<Vec<_>>(),
        "environment": {
            "OSV_SCANNER_LOCAL_DB_CACHE_DIRECTORY": db,
            "HOME": format!("{root}/out/home"),
            "XDG_CACHE_HOME": format!("{root}/out/home"),
            "XDG_CONFIG_HOME": format!("{root}/out/home"),
            "XDG_CONFIG_DIRS": format!("{root}/out/xdg"),
            "TMPDIR": format!("{root}/out/tmp")
        },
        "inputs": [{"role": "sbom", "path": SBOM_PATH}],
        "stdin": {"kind": "null"},
        "outputs": [{"role": "advisory-report", "path": REPORT_PATH, "required": true}],
        "containment": {
            "profile": "process-group-v1",
            "contract": {"kind": "engineering-assurance.osv-scanner-containment", "version": "1"}
        },
        "cancellation": {"kind": "disabled"},
        "budget": {
            "timeoutMillis": 120_000,
            "maxStdoutBytes": 65536,
            "maxStderrBytes": 65536,
            "maxInputBytes": 67_108_864,
            "maxOutputArtifacts": 1,
            "maxOutputBytes": 67_108_864,
            "maxDescendants": 64,
            "maxConcurrency": 1
        },
        "response": {
            "protocol": {"kind": "engineering-assurance.process-evidence", "version": "1"},
            "adapter": {"kind": "engineering-assurance.cli-process-evidence-adapter", "version": "1"},
            "exitCodes": {"kind": "exact", "codes": [0, 1]}
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
        .expect("schema compiles");
    assert!(validator.is_valid(request), "{request}");
}

fn write_scanner(root: &Path, script: &str) -> PathBuf {
    let path = root.join("scanner-fixture");
    fs::write(&path, format!("#!/bin/sh\n{script}")).expect("scanner fixture");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("executable bit");
    path
}

fn fixture_request(root: &Path, script: &str) -> Value {
    let executable = write_scanner(root, script);
    let db = root.join("db");
    osv_request(root, &executable, "fixture", &db)
}

#[trace("TC-9001", "FR-044-AC-6")]
#[test]
fn tc_9001_request_matches_the_published_osv_scanner_definition() {
    let root = workspace(br#"{"bomFormat":"CycloneDX","components":[]}"#);
    let db = root.path().join("db");
    let request = osv_request(root.path(), Path::new("/opt/osv-scanner"), "2.0.0", &db);
    assert_valid_request(&request);
    let base = fs::canonicalize(root.path())
        .expect("root")
        .display()
        .to_string();
    let arguments: Vec<Value> = [
        "scan",
        "source",
        "--offline",
        "--all-packages",
        "--format",
        "json",
        "--output-file",
        REPORT_PATH,
        "--config",
        "out/empty-config.toml",
        "-L",
        SBOM_PATH,
    ]
    .iter()
    .map(|value| json!({"kind":"literal", "value":value}))
    .collect();
    assert_eq!(request["arguments"], json!(arguments));
    for forbidden in [
        "--download-offline-databases",
        "--offline-vulnerabilities",
        "--allow-no-lockfiles",
        "--call-analysis",
        "--licenses",
        "--serve",
        "--x-exclude",
        "--all-vulns",
    ] {
        assert!(!request["arguments"].to_string().contains(forbidden));
    }
    assert_eq!(
        request["inputs"],
        json!([{"role":"sbom", "path":SBOM_PATH}])
    );
    assert_eq!(request["stdin"], json!({"kind":"null"}));
    assert_eq!(
        request["environment"]["OSV_SCANNER_LOCAL_DB_CACHE_DIRECTORY"],
        format!("{base}/db")
    );
    assert_eq!(request["environment"]["HOME"], format!("{base}/out/home"));
    assert_eq!(
        request["environment"]["XDG_CACHE_HOME"],
        format!("{base}/out/home")
    );
    assert_eq!(
        request["environment"]["XDG_CONFIG_HOME"],
        format!("{base}/out/home")
    );
    assert_eq!(
        request["environment"]["XDG_CONFIG_DIRS"],
        format!("{base}/out/xdg")
    );
    assert_eq!(request["environment"]["TMPDIR"], format!("{base}/out/tmp"));
    assert_eq!(
        request["outputs"],
        json!([{"role":"advisory-report", "path":REPORT_PATH, "required":true}])
    );
    assert_eq!(
        request["containment"]["contract"],
        json!({"kind":"engineering-assurance.osv-scanner-containment", "version":"1"})
    );
    assert_eq!(
        request["response"]["exitCodes"],
        json!({"kind":"exact", "codes":[0,1]})
    );
}

#[cfg(target_os = "linux")]
#[trace("TC-9002", "FR-044-AC-3", "FR-044-AC-4", "FR-044-AC-7")]
#[test]
fn tc_9002_failed_or_incomplete_fixture_scan_never_retains_report() {
    let root = workspace(br#"{"bomFormat":"CycloneDX","components":[]}"#);
    let missing = root.path().join("missing-osv-scanner");
    assert!(
        !missing.exists(),
        "the unavailable producer path must not exist"
    );
    let mut request = osv_request(root.path(), &missing, "fixture", &root.path().join("db"));
    let value = result(&request);
    assert_eq!(value["state"]["kind"], "unavailable", "{value}");
    assert!(value["artifacts"].as_array().expect("artifacts").is_empty());

    request = fixture_request(root.path(), "exit 127");
    fs::write(root.path().join(REPORT_PATH), b"stale report").expect("stale report");
    let value = result(&request);
    assert_eq!(value["state"]["kind"], "failed", "{value}");
    assert!(value["artifacts"].as_array().expect("artifacts").is_empty());
    assert!(
        !root.path().join(REPORT_PATH).exists(),
        "stale output must be removed"
    );

    request = fixture_request(root.path(), "exit 128");
    assert_eq!(result(&request)["state"]["kind"], "failed");

    request = fixture_request(root.path(), "exit 0");
    let value = result(&request);
    assert_eq!(
        value["state"]["kind"], "failed",
        "success without required report: {value}"
    );
    assert!(value["artifacts"].as_array().expect("artifacts").is_empty());

    request = fixture_request(
        root.path(),
        "echo '{\"results\":[]}' > out/osv-scanner.json; exit 1",
    );
    let value = result(&request);
    assert_eq!(value["state"]["kind"], "completed", "{value}");
    assert_eq!(value["state"]["observation"]["exitCode"], 1, "{value}");
    assert_eq!(value["artifacts"][0]["role"], "advisory-report");

    request = fixture_request(
        root.path(),
        "sleep 3; echo '{}' > out/osv-scanner.json; exit 0",
    );
    request["budget"]["timeoutMillis"] = json!(1000);
    let value = result(&request);
    assert_eq!(value["state"]["kind"], "timed_out", "{value}");
    assert!(value["artifacts"].as_array().expect("artifacts").is_empty());
}

#[trace("TC-9003", "FR-044-AC-12")]
#[test]
fn tc_9003_database_validation_rejects_wolfi_and_missing_archives() {
    let root = tempfile::tempdir().expect("database root");
    let missing = root.path().join("missing");
    fs::create_dir_all(&missing).expect("missing database directory");
    assert!(
        database_root(Some(missing.into_os_string()))
            .expect_err("database archive is required")
            .contains("EA_TEST_OSV_DB")
    );

    let wolfi = root.path().join("wolfi");
    let archive = wolfi.join("osv-scalibr/wolfi/all.zip");
    fs::create_dir_all(archive.parent().expect("archive parent")).expect("ecosystem directory");
    fs::write(&archive, b"fixture database marker").expect("archive marker");
    assert!(
        database_root(Some(wolfi.into_os_string()))
            .expect_err("redistributable mirror excludes Wolfi")
            .contains("Wolfi")
    );
}

#[trace("TC-9004", "FR-044-CON-1", "FR-044-AC-9")]
#[test]
fn tc_9004_integration_lane_refuses_missing_scanner_or_database() {
    let non_executable = tempfile::NamedTempFile::new().expect("plain file");
    let non_directory = tempfile::NamedTempFile::new().expect("plain file");
    for (scanner, db, expected) in [
        (None, Some("/tmp"), "EA_TEST_OSV_SCANNER"),
        (
            Some("relative/scanner"),
            Some("/tmp"),
            "EA_TEST_OSV_SCANNER",
        ),
        (
            non_executable.path().to_str(),
            Some("/tmp"),
            "EA_TEST_OSV_SCANNER",
        ),
        (Some("/bin/true"), None, "EA_TEST_OSV_DB"),
        (Some("/bin/true"), Some("relative/db"), "EA_TEST_OSV_DB"),
        (
            Some("/bin/true"),
            non_directory.path().to_str(),
            "EA_TEST_OSV_DB",
        ),
    ] {
        let mut make = Command::new("make");
        make.args(["-C", env!("CARGO_MANIFEST_DIR"), "osv-scanner-integration"])
            .env_remove("EA_TEST_OSV_SCANNER")
            .env_remove("EA_TEST_OSV_DB");
        if let Some(path) = scanner {
            make.env("EA_TEST_OSV_SCANNER", path);
        }
        if let Some(path) = db {
            make.env("EA_TEST_OSV_DB", path);
        }
        let output = make.output().expect("make must start");
        assert_eq!(output.status.code(), Some(2), "{scanner:?} {db:?}");
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }
}

#[trace("TC-9005", "FR-044-AC-13", "FR-044-AC-14")]
#[test]
#[ignore = "needs EA_TEST_OSV_SCANNER and EA_TEST_OSV_DB; fixture outcomes are not compliance evidence"]
fn tc_9005_scans_the_debian_sbom_fixture_for_its_expected_advisory() {
    let scanner = scanner();
    let supplied_database = database();
    let sbom = json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "components": [{
            "type": "library",
            "name": OS_NAME,
            "version": OS_VERSION,
            "purl": OS_PURL
        }]
    })
    .to_string()
    .into_bytes();
    let root = workspace(&sbom);
    copy_database(&supplied_database, &root.path().join("db"));
    let request = osv_request(
        root.path(),
        &scanner.executable,
        &scanner.version,
        &root.path().join("db"),
    );
    assert_valid_request(&request);
    let value = result(&request);
    assert_eq!(value["state"]["kind"], "completed", "{value}");
    assert!(
        matches!(
            value["state"]["observation"]["exitCode"].as_i64(),
            Some(0 | 1)
        ),
        "{value}"
    );
    let artifacts = value["artifacts"].as_array().expect("artifacts");
    assert_eq!(artifacts.len(), 1, "{value}");
    assert_eq!(artifacts[0]["role"], "advisory-report");
    assert_eq!(artifacts[0]["path"], REPORT_PATH);
    let report_bytes = fs::read(root.path().join(REPORT_PATH)).expect("retained report");
    assert_eq!(artifacts[0]["byteLength"], report_bytes.len());
    let report: Value = serde_json::from_slice(&report_bytes).expect("scanner report JSON");
    assert!(report.is_object(), "{report}");
    let results = report["results"].as_array().expect("OSV report results");
    let expected_package = results
        .iter()
        .flat_map(|result| result["packages"].as_array().into_iter().flatten())
        .find(|package| {
            package["package"]["name"] == OS_NAME
                && package["package"]["version"] == OS_VERSION
                && package["package"]["purl"] == OS_PURL
        })
        .unwrap_or_else(|| {
            panic!("OSV report did not capture the declared Debian fixture package: {report}")
        });
    assert!(
        expected_package["vulnerabilities"]
            .as_array()
            .is_some_and(|vulnerabilities| {
                vulnerabilities.iter().any(|vulnerability| {
                    vulnerability["id"] == EXPECTED_OSV_ID
                        || vulnerability["aliases"].as_array().is_some_and(|aliases| {
                            aliases.iter().any(|alias| alias == EXPECTED_CVE_ALIAS)
                        })
                })
            }),
        "OSV report did not associate {EXPECTED_OSV_ID} with {OS_PURL}: {report}"
    );
    // Runtime fixture behavior is not a compliance claim. In particular this
    // test does not interpret an empty vulnerability list as clean evidence.
}
