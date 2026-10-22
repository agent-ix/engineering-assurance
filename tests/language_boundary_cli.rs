// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! STD-30 acceptance probes for the Git-based executable boundary.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use ix_trace_rs::trace;
use serde_json::Value;

struct Repository {
    directory: tempfile::TempDir,
}

impl Repository {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("temporary repository");
        let root = directory.path();
        git(root, &["init", "-q", "-b", "main"]);
        git(root, &["config", "user.name", "Boundary Test"]);
        git(root, &["config", "user.email", "boundary@example.invalid"]);
        fs::write(root.join("README.md"), "fixture\n").expect("fixture write");
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", "base"]);
        Self { directory }
    }

    fn root(&self) -> &Path {
        self.directory.path()
    }

    fn write(&self, path: &str, body: &str) {
        let selected = self.root().join(path);
        fs::create_dir_all(selected.parent().expect("fixture parent")).expect("fixture directory");
        fs::write(selected, body).expect("fixture write");
    }

    fn commit(&self) -> String {
        git(self.root(), &["add", "-A"]);
        git(self.root(), &["commit", "-q", "-m", "candidate"]);
        sha(self.root(), "HEAD")
    }
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git starts");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn sha(root: &Path, name: &str) -> String {
    let output = Command::new("git")
        .args(["rev-parse", name])
        .current_dir(root)
        .output()
        .expect("git starts");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .expect("ASCII sha")
        .trim()
        .to_owned()
}

fn guard(root: &Path, repository: &str, base: Option<&str>, head: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"));
    command
        .args(["language-boundary", "--root"])
        .arg(root)
        .args(["--repository", repository]);
    if let Some(base) = base {
        command.args(["--base-revision", base]);
    }
    if let Some(head) = head {
        command.args(["--head-revision", head]);
    }
    command.output().expect("guard starts")
}

fn findings(output: &Output) -> Vec<Value> {
    serde_json::from_slice::<Value>(&output.stdout).expect("machine result")["findings"]
        .as_array()
        .expect("findings")
        .clone()
}

fn assert_schema(output: &Output) {
    let schema: Value = serde_json::from_str(include_str!(
        "../engineering_assurance/schemas/language-boundary-result-v1.schema.json"
    ))
    .expect("packaged result schema");
    let validator = jsonschema::options()
        .build(&schema)
        .expect("result schema compiles");
    let value: Value = serde_json::from_slice(&output.stdout).expect("structured result");
    assert!(
        validator.is_valid(&value),
        "result does not match packaged contract: {value}"
    );
}

#[test]
#[trace("STD-30")]
fn new_renamed_and_expanded_script_paths_are_refused() {
    let repo = Repository::new();
    repo.write("scripts/check.py", "print('one')\n");
    let first = repo.commit();
    let created = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&first),
    );
    assert_eq!(created.status.code(), Some(1));
    assert_schema(&created);
    assert!(findings(&created).iter().any(
        |finding| finding["path"] == "scripts/check.py" && finding["kind"] == "executable-file"
    ));

    repo.write("scripts/check.py", "print('two')\n");
    let second = repo.commit();
    let expanded = guard(repo.root(), "quire-rs", Some(&first), Some(&second));
    assert_eq!(expanded.status.code(), Some(1));
    assert!(
        findings(&expanded)
            .iter()
            .any(|finding| finding["path"] == "scripts/check.py")
    );

    fs::rename(
        repo.root().join("scripts/check.py"),
        repo.root().join("scripts/other.py"),
    )
    .expect("rename");
    let third = repo.commit();
    let renamed = guard(repo.root(), "quire-rs", Some(&second), Some(&third));
    assert_eq!(renamed.status.code(), Some(1));
    assert!(
        findings(&renamed)
            .iter()
            .any(|finding| finding["path"] == "scripts/other.py")
    );
}

#[test]
#[trace("STD-30")]
fn extensionless_shebang_and_mode_are_refused() {
    let repo = Repository::new();
    repo.write("audit", "#!/usr/bin/env python3\nprint('x')\n");
    let shebang = repo.commit();
    let output = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&shebang),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        findings(&output)
            .iter()
            .any(|finding| finding["path"] == "audit")
    );

    repo.write("helper", "plain executable\n");
    #[cfg(unix)]
    fs::set_permissions(
        repo.root().join("helper"),
        fs::Permissions::from_mode(0o755),
    )
    .expect("executable fixture");
    let base = shebang;
    let head = repo.commit();
    let output = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(output.status.code(), Some(1));
    assert!(
        findings(&output)
            .iter()
            .any(|finding| finding["path"] == "helper")
    );
}

#[test]
#[cfg(unix)]
#[trace("STD-30")]
fn an_extensionless_script_symlink_is_refused() {
    let repo = Repository::new();
    fs::create_dir_all(repo.root().join("tools")).expect("tools directory");
    std::os::unix::fs::symlink("../src/helper.py", repo.root().join("tools/check"))
        .expect("symlink fixture");
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        findings(&output)
            .iter()
            .any(|finding| finding["path"] == "tools/check")
    );
}

#[test]
#[trace("STD-30")]
fn make_workflow_and_manifest_launches_are_refused() {
    let repo = Repository::new();
    repo.write("Makefile", "legacy:\n\tpython3 scripts/old.py\n");
    repo.write("scripts/old.py", "print('old')\n");
    let base = repo.commit();
    repo.write(".github/workflows/gate.yml", "name: Gate\non: pull_request\njobs:\n  check:\n    steps:\n      - run: |\n          make legacy\n          node tools/check.js\n");
    repo.write(
        "package.json",
        r#"{"scripts":{"check":"node check.js"},"bin":{"gate":"bin/gate.js"}}"#,
    );
    let head = repo.commit();
    let output = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(output.status.code(), Some(1));
    let found = findings(&output);
    assert!(
        found
            .iter()
            .any(|finding| finding["kind"] == "indirect-launch")
    );
    assert!(
        found
            .iter()
            .any(|finding| finding["kind"] == "inline-launch")
    );
    assert!(found.iter().any(|finding| finding["kind"] == "npm-script"));
    assert!(found.iter().any(|finding| finding["kind"] == "npm-bin"));
}

#[test]
#[trace("STD-30")]
fn rust_wrapper_and_generated_executable_template_are_refused() {
    let repo = Repository::new();
    repo.write("src/adapter.rs", "pub fn ready() -> bool { true }\n");
    let base = repo.commit();
    repo.write(
        "src/adapter.rs",
        "pub fn run() { let _ = std::process::Command::new(\"python3\"); }\n",
    );
    repo.write("templates/generated-check.py", "print('generated')\n");
    let head = repo.commit();
    let output = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(output.status.code(), Some(1));
    let found = findings(&output);
    assert!(
        found
            .iter()
            .any(|finding| finding["path"] == "src/adapter.rs"
                && finding["kind"] == "rust-wrapper-launch")
    );
    assert!(
        found
            .iter()
            .any(|finding| finding["path"] == "templates/generated-check.py"
                && finding["kind"] == "executable-file")
    );
}

#[test]
#[trace("STD-30")]
fn dynamic_rust_launch_and_misleading_shebang_are_refused() {
    let repo = Repository::new();
    repo.write("src/adapter.rs", "pub fn ready() -> bool { true }\n");
    let base = repo.commit();
    repo.write(
        "src/adapter.rs",
        "pub fn run(program: &str) { let _ = std::process::Command::new(program); }\n",
    );
    repo.write("runner", "#!/usr/bin/env rustpython\nprint('x')\n");
    let head = repo.commit();
    let output = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(output.status.code(), Some(1));
    let found = findings(&output);
    assert!(
        found
            .iter()
            .any(|finding| finding["path"] == "src/adapter.rs"
                && finding["kind"] == "dynamic-rust-launch")
    );
    assert!(
        found
            .iter()
            .any(|finding| finding["path"] == "runner" && finding["kind"] == "executable-file")
    );
}

#[test]
#[trace("STD-30")]
fn a_new_make_target_with_the_same_existing_command_is_refused() {
    let repo = Repository::new();
    repo.write("Makefile", "old:\n\tpython3 check.py\n");
    let base = repo.commit();
    repo.write(
        "Makefile",
        "old:\n\tpython3 check.py\nnew:\n\tpython3 check.py\n",
    );
    let head = repo.commit();
    let output = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(output.status.code(), Some(1));
    assert!(findings(&output).iter().any(|finding| {
        finding["path"] == "Makefile"
            && finding["evidence"]
                .as_str()
                .is_some_and(|evidence| evidence.starts_with("new:"))
    }));
}

#[test]
#[trace("STD-30")]
fn exact_exceptions_and_inert_ui_source_are_accepted() {
    let repo = Repository::new();
    repo.write("docs/images/figures.py", "#!/usr/bin/env python3\n");
    repo.write("tests/python/binding_test.py", "#!/usr/bin/env python3\n");
    repo.write(
        "tests/fixtures/case/input/sample.py",
        "#!/usr/bin/env python3\n",
    );
    repo.write("src/view.ts", "export const title = 'UI';\n");
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(output.status.code(), Some(0));
    assert_schema(&output);
    assert!(findings(&output).is_empty());
}

#[test]
#[trace("STD-30")]
fn driver_plugin_test_double_is_the_only_driver_python_exception() {
    let repo = Repository::new();
    repo.write(
        "tests/examples/sample-plugin.py",
        "#!/usr/bin/env python3\n",
    );
    let allowed = repo.commit();
    let output = guard(
        repo.root(),
        "quire-driver",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&allowed),
    );
    assert_eq!(output.status.code(), Some(0));
    repo.write("tests/examples/check.py", "#!/usr/bin/env python3\n");
    let disallowed = repo.commit();
    let output = guard(
        repo.root(),
        "quire-driver",
        Some(&allowed),
        Some(&disallowed),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        findings(&output)
            .iter()
            .any(|finding| finding["path"] == "tests/examples/check.py")
    );
}

#[test]
#[trace("STD-30")]
fn missing_or_unsafe_base_fails_closed() {
    let repo = Repository::new();
    repo.write("README.md", "changed\n");
    repo.commit();
    let invalid = guard(repo.root(), "quire-rs", Some("deadbeef"), None);
    assert_eq!(invalid.status.code(), Some(2));
    let excluded = guard(repo.root(), "build-chain", None, None);
    assert_eq!(excluded.status.code(), Some(2));
}

#[test]
#[trace("STD-30")]
fn shallow_history_and_invalid_manifest_fail_closed() {
    let repo = Repository::new();
    repo.write("README.md", "second revision\n");
    repo.commit();
    let clone = tempfile::tempdir().expect("clone root");
    let url = format!("file://{}", repo.root().display());
    let output = Command::new("git")
        .args(["clone", "-q", "--depth", "1", &url])
        .arg(clone.path().join("shallow"))
        .output()
        .expect("git clone starts");
    assert!(output.status.success());
    let shallow = guard(&clone.path().join("shallow"), "quire-rs", None, None);
    assert_eq!(shallow.status.code(), Some(2));

    let base = sha(repo.root(), "HEAD");
    repo.write("package.json", "{invalid json}");
    let head = repo.commit();
    let malformed = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(malformed.status.code(), Some(2));
}

#[test]
#[trace("STD-30")]
fn node_action_runtime_is_refused_without_a_run_step() {
    let repo = Repository::new();
    repo.write(
        ".github/actions/check/action.yml",
        "name: Check\nruns:\n  using: node20\n  main: index.js\n",
    );
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(findings(&output).iter().any(|finding| finding["path"]
        == ".github/actions/check/action.yml"
        && finding["kind"] == "inline-launch"));
}
