// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The document converter producer requests (FR-045) run through
//! `producer-execute`.
//!
//! Tests that run the real converters read their absolute paths from
//! `EA_TEST_PANDOC` and `EA_TEST_TYPST` and fail, never skip, when one is absent
//! (FR-045-CON-1). The other tests run shell stand-ins written into a temporary
//! directory, so the repository holds no converter binary, template or
//! reference document.

use std::{
    ffi::OsString,
    fs,
    io::Read,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const REQUEST_SCHEMA: &str =
    include_str!("../src/producer_execute_schemas/producer-execution-request-v1.schema.json");

const FIXTURE_MARKDOWN: &str = "---\ntitle: Fictional Lumen Report\ntype: FR\n---\n\n# Ember Overview\n\n## Quill Details\n\n- first item\n- second item\n\n| Name | Value |\n| --- | --- |\n| lumen | 1 |\n\n```\nfn fictional() {}\n```\n";

/// Which converter a request runs, and so which argument list and output it has.
#[derive(Clone, Copy)]
enum Step {
    PandocDocx,
    PandocTypst,
    TypstPdf,
}

impl Step {
    fn name(self) -> &'static str {
        match self {
            Self::PandocDocx | Self::PandocTypst => "pandoc",
            Self::TypstPdf => "typst",
        }
    }

    fn arguments(self) -> Vec<&'static str> {
        match self {
            Self::PandocDocx => vec![
                "--sandbox",
                "--standalone",
                "--from",
                "markdown",
                "--to",
                "docx",
                "--output",
                "out/document.docx",
                "in/document.md",
            ],
            Self::PandocTypst => vec![
                "--sandbox",
                "--standalone",
                "--from",
                "markdown",
                "--to",
                "typst",
                "--output",
                "typ/document.typ",
                "in/document.md",
            ],
            Self::TypstPdf => vec!["compile", "typ/document.typ", "out/document.pdf"],
        }
    }

    fn output(self) -> &'static str {
        match self {
            Self::PandocDocx => "out/document.docx",
            Self::PandocTypst => "typ/document.typ",
            Self::TypstPdf => "out/document.pdf",
        }
    }
}

/// A capability root holding the assembled Markdown, and empty `typ/`, `out/home`
/// and `out/tmp`.
fn workspace() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("temporary root");
    for directory in ["in", "typ", "out/home", "out/tmp"] {
        fs::create_dir_all(root.path().join(directory)).expect("capability subdirectory");
    }
    fs::write(root.path().join("in/document.md"), FIXTURE_MARKDOWN).expect("Markdown input");
    root
}

/// The converter producer definition of FR-045 for one step.
fn converter_request(root: &Path, executable: &Path, step: Step) -> Value {
    let root = fs::canonicalize(root).expect("root").display().to_string();
    json!({
        "protocol": "engineering-assurance.producer-execution-request/v1",
        "producer": {
            "name": step.name(),
            "version": "unreported",
            "executable": executable.display().to_string()
        },
        "caller": {"kind": "quire.export", "version": "1"},
        "procedure": "direct",
        "capabilityRoot": root,
        "arguments": step
            .arguments()
            .iter()
            .map(|value| json!({"kind": "literal", "value": value}))
            .collect::<Vec<_>>(),
        "environment": {
            "HOME": format!("{root}/out/home"),
            "TMPDIR": format!("{root}/out/tmp")
        },
        "inputs": [],
        "stdin": {"kind": "null"},
        "outputs": [{"role": "document", "path": step.output(), "required": true}],
        "containment": {
            "profile": "process-group-v1",
            "contract": {"kind": "quire.export-containment", "version": "1"}
        },
        "cancellation": {"kind": "disabled"},
        "budget": {
            "timeoutMillis": 120_000,
            "maxStdoutBytes": 1_048_576,
            "maxStderrBytes": 1_048_576,
            "maxInputBytes": 268_435_456,
            "maxOutputArtifacts": 1,
            "maxOutputBytes": 268_435_456,
            "maxDescendants": 64,
            "maxConcurrency": 1
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

fn assert_valid_request(request: &Value) {
    let schema: Value = serde_json::from_str(REQUEST_SCHEMA).expect("schema is JSON");
    let validator = jsonschema::options()
        .build(&schema)
        .expect("published schema compiles");
    assert!(validator.is_valid(request), "{request}");
}

fn result(request: &Value) -> Value {
    assert_valid_request(request);
    let output = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["producer-execute", "--request", &request.to_string()])
        .output()
        .expect("the Cargo-built CLI must start");
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    serde_json::from_slice(&output.stdout).expect("stdout must be one JSON document")
}

/// Runs the request, requires `completed`, and returns the retained document.
fn completed_document(root: &Path, request: &Value, step: Step) -> Vec<u8> {
    let value = result(request);
    assert_eq!(value["state"]["kind"], "completed", "{value}");
    assert_eq!(value["state"]["observation"]["exitCode"], 0);
    let artifacts = value["artifacts"].as_array().expect("artifacts");
    assert_eq!(artifacts.len(), 1, "{value}");
    assert_eq!(artifacts[0]["role"], "document");
    assert_eq!(artifacts[0]["path"], step.output());
    let bytes = fs::read(root.join(step.output())).expect("retained document");
    assert_eq!(artifacts[0]["byteLength"], bytes.len());
    bytes
}

fn write_script(directory: &Path, name: &str, body: &str) -> PathBuf {
    let path = fs::canonicalize(directory).expect("directory").join(name);
    fs::write(&path, format!("#!/bin/sh\n{body}")).expect("stand-in script");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("executable bit");
    path
}

/// A pandoc stand-in that copies its last argument to its `--output` path.
const PANDOC_COPY: &str = "while [ $# -gt 0 ]; do\n  case \"$1\" in\n    --output) out=\"$2\"; shift ;;\n    *) last=\"$1\" ;;\n  esac\n  shift\ndone\ncp \"$last\" \"$out\"\n";

/// A typst stand-in for `compile <input> <output>`.
const TYPST_COPY: &str = "test \"$1\" = compile || exit 64\ncp \"$2\" \"$3\"\n";

#[cfg(target_os = "linux")]
#[trace("TC-368", "FR-045-AC-1")]
#[test]
fn tc_368_stand_in_pandoc_makes_a_docx_document() {
    let tools = tempfile::tempdir().expect("tools");
    let pandoc = write_script(tools.path(), "pandoc", PANDOC_COPY);
    let root = workspace();
    let request = converter_request(root.path(), &pandoc, Step::PandocDocx);
    let bytes = completed_document(root.path(), &request, Step::PandocDocx);
    assert_eq!(bytes, FIXTURE_MARKDOWN.as_bytes());
}

#[cfg(target_os = "linux")]
#[trace("TC-369", "FR-045-AC-2")]
#[test]
fn tc_369_stand_in_pandoc_then_typst_share_one_capability_root() {
    let tools = tempfile::tempdir().expect("tools");
    let pandoc = write_script(tools.path(), "pandoc", PANDOC_COPY);
    let typst = write_script(tools.path(), "typst", TYPST_COPY);
    let root = workspace();
    let first = converter_request(root.path(), &pandoc, Step::PandocTypst);
    let source = completed_document(root.path(), &first, Step::PandocTypst);
    assert_eq!(source, FIXTURE_MARKDOWN.as_bytes());
    let second = converter_request(root.path(), &typst, Step::TypstPdf);
    let pdf = completed_document(root.path(), &second, Step::TypstPdf);
    assert_eq!(pdf, source);
}

#[cfg(target_os = "linux")]
#[trace("TC-370", "FR-045-AC-3")]
#[test]
fn tc_370_missing_converter_is_unavailable_without_a_document() {
    let tools = tempfile::tempdir().expect("tools");
    let root = workspace();
    for step in [Step::PandocDocx, Step::TypstPdf] {
        let absent = fs::canonicalize(tools.path())
            .expect("tools")
            .join(step.name());
        let value = result(&converter_request(root.path(), &absent, step));
        assert_eq!(value["state"]["kind"], "unavailable", "{value}");
        assert!(value["artifacts"].as_array().expect("artifacts").is_empty());
        assert_eq!(value["producer"]["name"], step.name());
        assert_eq!(
            value["producer"]["executable"],
            absent.display().to_string()
        );
    }
    let directory = fs::canonicalize(tools.path()).expect("tools");
    let value = result(&converter_request(
        root.path(),
        &directory,
        Step::PandocDocx,
    ));
    assert_eq!(value["state"]["kind"], "refused", "{value}");
    assert!(value["artifacts"].as_array().expect("artifacts").is_empty());
}

#[cfg(target_os = "linux")]
#[trace("TC-371", "FR-045-AC-4")]
#[test]
fn tc_371_converter_failure_is_failed_and_carries_its_stderr() {
    let tools = tempfile::tempdir().expect("tools");
    let pandoc = write_script(
        tools.path(),
        "pandoc",
        "echo 'pandoc: fictional conversion failure' >&2\nexit 7\n",
    );
    let root = workspace();
    let value = result(&converter_request(root.path(), &pandoc, Step::PandocDocx));
    assert_eq!(value["state"]["kind"], "failed", "{value}");
    assert_eq!(
        value["process"]["terminalStatus"],
        json!({"kind": "exit_code", "value": 7}),
        "{value}"
    );
    assert!(value["artifacts"].as_array().expect("artifacts").is_empty());
    let stderr: Vec<u8> = value["process"]["stderr"]["bytes"]
        .as_array()
        .expect("retained stderr bytes")
        .iter()
        .map(|byte| u8::try_from(byte.as_u64().expect("byte")).expect("byte range"))
        .collect();
    assert_eq!(
        String::from_utf8(stderr).expect("UTF-8 stderr"),
        "pandoc: fictional conversion failure\n"
    );
}

#[cfg(target_os = "linux")]
#[trace("TC-372", "FR-045-AC-4")]
#[test]
fn tc_372_exit_zero_without_the_required_output_is_failed() {
    let tools = tempfile::tempdir().expect("tools");
    let pandoc = write_script(tools.path(), "pandoc", "exit 0\n");
    let root = workspace();
    let value = result(&converter_request(root.path(), &pandoc, Step::PandocDocx));
    assert_eq!(value["state"]["kind"], "failed", "{value}");
    assert!(value["artifacts"].as_array().expect("artifacts").is_empty());
}

#[trace("TC-373", "FR-045-AC-5")]
#[test]
fn tc_373_requests_match_the_definition_and_the_published_schema() {
    let root = workspace();
    let base = fs::canonicalize(root.path())
        .expect("root")
        .display()
        .to_string();
    let steps = [
        (Step::PandocDocx, "out/document.docx"),
        (Step::PandocTypst, "typ/document.typ"),
        (Step::TypstPdf, "out/document.pdf"),
    ];
    for (step, path) in steps {
        let request = converter_request(root.path(), Path::new("/opt/tools/bin/tool"), step);
        assert_valid_request(&request);
        assert_eq!(request["producer"]["version"], "unreported");
        assert_eq!(
            request["caller"],
            json!({"kind": "quire.export", "version": "1"})
        );
        assert_eq!(request["inputs"], json!([]));
        assert_eq!(request["stdin"], json!({"kind": "null"}));
        assert_eq!(request["cancellation"], json!({"kind": "disabled"}));
        assert_eq!(
            request["environment"],
            json!({"HOME": format!("{base}/out/home"), "TMPDIR": format!("{base}/out/tmp")})
        );
        assert_eq!(
            request["containment"]["contract"],
            json!({"kind": "quire.export-containment", "version": "1"})
        );
        assert_eq!(
            request["outputs"],
            json!([{"role": "document", "path": path, "required": true}])
        );
    }
    let expected: [(Step, &[&str]); 3] = [
        (
            Step::PandocDocx,
            &[
                "--sandbox",
                "--standalone",
                "--from",
                "markdown",
                "--to",
                "docx",
                "--output",
                "out/document.docx",
                "in/document.md",
            ],
        ),
        (
            Step::PandocTypst,
            &[
                "--sandbox",
                "--standalone",
                "--from",
                "markdown",
                "--to",
                "typst",
                "--output",
                "typ/document.typ",
                "in/document.md",
            ],
        ),
        (
            Step::TypstPdf,
            &["compile", "typ/document.typ", "out/document.pdf"],
        ),
    ];
    for (step, literals) in expected {
        let request = converter_request(root.path(), Path::new("/x/tool"), step);
        let actual: Vec<&str> = request["arguments"]
            .as_array()
            .expect("arguments")
            .iter()
            .map(|argument| argument["value"].as_str().expect("literal"))
            .collect();
        assert_eq!(actual, literals);
    }
}

/// A real converter a test runs, from the value of an `EA_TEST_*` variable.
fn resolve_tool(variable: &str, value: Option<OsString>) -> Result<PathBuf, String> {
    let path = value
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| format!("{variable} must name the absolute path of the tool under test"))?;
    let executable = fs::metadata(&path)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0);
    if executable {
        Ok(path)
    } else {
        Err(format!(
            "{variable} {} is not an executable file",
            path.display()
        ))
    }
}

fn tool(variable: &str) -> PathBuf {
    resolve_tool(variable, std::env::var_os(variable)).unwrap_or_else(|message| panic!("{message}"))
}

#[trace("TC-374", "FR-045-AC-6")]
#[test]
#[ignore = "needs EA_TEST_PANDOC"]
fn tc_374_real_pandoc_converts_the_fixture_to_docx() {
    let pandoc = tool("EA_TEST_PANDOC");
    let root = workspace();
    let request = converter_request(root.path(), &pandoc, Step::PandocDocx);
    let bytes = completed_document(root.path(), &request, Step::PandocDocx);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("DOCX is a zip");
    let mut body = String::new();
    archive
        .by_name("word/document.xml")
        .expect("word/document.xml")
        .read_to_string(&mut body)
        .expect("UTF-8 document body");
    for heading in ["Ember Overview", "Quill Details", "fn fictional"] {
        assert!(body.contains(heading), "missing {heading}");
    }
}

#[trace("TC-375", "FR-045-AC-6")]
#[test]
#[ignore = "needs EA_TEST_PANDOC and EA_TEST_TYPST"]
fn tc_375_real_pandoc_and_typst_convert_the_fixture_to_pdf() {
    let pandoc = tool("EA_TEST_PANDOC");
    let typst = tool("EA_TEST_TYPST");
    let root = workspace();
    let first = converter_request(root.path(), &pandoc, Step::PandocTypst);
    let source = completed_document(root.path(), &first, Step::PandocTypst);
    let source = String::from_utf8(source).expect("Typst source is UTF-8");
    assert!(source.contains("Ember Overview"), "{source}");
    let second = converter_request(root.path(), &typst, Step::TypstPdf);
    let pdf = completed_document(root.path(), &second, Step::TypstPdf);
    assert!(pdf.starts_with(b"%PDF"), "the output must be a PDF");
}

#[trace("TC-376", "FR-045-AC-6", "FR-045-CON-1")]
#[test]
fn tc_376_missing_converter_fails_the_integration_target_naming_the_variable() {
    let not_executable = tempfile::NamedTempFile::new().expect("plain file");
    let plain = not_executable.path().to_str().expect("UTF-8 path");
    let cases = [
        (None, None, "EA_TEST_PANDOC"),
        (Some("relative/pandoc"), Some(plain), "EA_TEST_PANDOC"),
        (Some("/bin/sh"), None, "EA_TEST_TYPST"),
        (Some("/bin/sh"), Some(plain), "EA_TEST_TYPST"),
    ];
    for (pandoc, typst, named) in cases {
        let output = make_integration(pandoc, typst);
        assert_eq!(output.status.code(), Some(2), "{pandoc:?} {typst:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(named), "{stderr}");
    }
    let message = resolve_tool("EA_TEST_PANDOC", None).expect_err("unset fails");
    assert!(message.contains("EA_TEST_PANDOC"), "{message}");
}

fn make_integration(pandoc: Option<&str>, typst: Option<&str>) -> Output {
    let mut make = Command::new("make");
    make.args(["-C", env!("CARGO_MANIFEST_DIR"), "converter-integration"])
        .env_remove("EA_TEST_PANDOC")
        .env_remove("EA_TEST_TYPST");
    if let Some(path) = pandoc {
        make.env("EA_TEST_PANDOC", path);
    }
    if let Some(path) = typst {
        make.env("EA_TEST_TYPST", path);
    }
    make.output().expect("make must start")
}
