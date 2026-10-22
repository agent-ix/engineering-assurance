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
#[trace("TC-209", "NFR-005-AC-6", "TC-211", "NFR-005-AC-8")]
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
#[trace("TC-210", "NFR-005-AC-7")]
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
#[trace("TC-210", "NFR-005-AC-7")]
#[cfg(unix)]
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
#[trace("TC-210", "NFR-005-AC-7")]
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
            .any(|finding| finding["kind"] == "inline-launch"),
        "{found:?}"
    );
    assert!(found.iter().any(|finding| finding["kind"] == "npm-script"));
    assert!(found.iter().any(|finding| finding["kind"] == "npm-bin"));
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
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
#[trace("TC-210", "NFR-005-AC-7")]
fn qualified_dynamic_rust_launch_and_misleading_shebang_are_refused() {
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
    assert!(found.iter().any(|finding| {
        finding["path"] == "src/adapter.rs" && finding["kind"] == "dynamic-rust-launch"
    }));
    assert!(
        found
            .iter()
            .any(|finding| finding["path"] == "runner" && finding["kind"] == "executable-file")
    );
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn imported_process_command_dynamic_launch_is_refused() {
    let repo = Repository::new();
    repo.write(
        "src/host.rs",
        "use std::process::Command;\npub fn run(program: &str) { let _ = Command::new(program); }\n",
    );
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(findings(&output).iter().any(|finding| {
        finding["path"] == "src/host.rs" && finding["kind"] == "dynamic-rust-launch"
    }));
}

#[test]
#[trace("TC-209", "NFR-005-AC-6")]
fn another_identical_rust_process_launch_is_new_surface() {
    let repo = Repository::new();
    repo.write(
        "src/host.rs",
        "pub fn one() { let _ = std::process::Command::new(\"python3\"); }\n",
    );
    let base = repo.commit();
    repo.write("src/host.rs", "pub fn one() { let _ = std::process::Command::new(\"python3\"); }\npub fn two() { let _ = std::process::Command::new(\"python3\"); }\n");
    let head = repo.commit();
    let output = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(output.status.code(), Some(1));
    assert!(
        findings(&output).iter().any(|finding| {
            finding["path"] == "src/host.rs" && finding["evidence"] == "python3#2"
        })
    );
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn make_variables_cannot_hide_a_new_script_launcher() {
    let repo = Repository::new();
    repo.write(
        "Makefile",
        "CHECKER = python3\nGATE = src/check.py\ncheck:\n\t$(CHECKER) $(GATE)\n",
    );
    repo.write("src/check.py", "print('check')\n");
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(findings(&output).iter().any(|finding| {
        finding["path"] == "Makefile"
            && finding["evidence"]
                .as_str()
                .is_some_and(|value| value.contains("check:python3 src/check.py"))
    }));
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn exported_make_variable_cannot_hide_extensionless_python_launcher() {
    let repo = Repository::new();
    repo.write(
        "Makefile",
        "export CHECKER=python3\nGATE=src/check\ncheck:\n\t$(CHECKER) $(GATE)\n",
    );
    repo.write("src/check", "print('check')\n");
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(findings(&output).iter().any(|finding| {
        finding["path"] == "Makefile"
            && finding["evidence"]
                .as_str()
                .is_some_and(|value| value.contains("check:python3 src/check"))
    }));
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn folded_action_scalar_is_scanned() {
    let repo = Repository::new();
    repo.write(
        ".github/workflows/check.yml",
        "jobs:\n  check:\n    steps:\n      - run: >-\n          python3 -c 'print(1)'\n",
    );
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
            .any(|finding| finding["path"] == ".github/workflows/check.yml")
    );
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn action_env_interpreter_and_new_run_reference_are_refused() {
    let repo = Repository::new();
    repo.write(".github/workflows/check.yml", "env:\n  CHECKER: python3\n");
    let base = repo.commit();
    repo.write(
        ".github/workflows/check.yml",
        "env:\n  CHECKER: python3\njobs:\n  check:\n    steps:\n      - run: $CHECKER -c 'print(1)'\n",
    );
    let head = repo.commit();
    let output = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(output.status.code(), Some(1));
    assert!(findings(&output).iter().any(|finding| {
        finding["path"] == ".github/workflows/check.yml"
            && finding["evidence"]
                .as_str()
                .is_some_and(|value| value.contains("run-1:python3 -c"))
    }));
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn unresolved_actions_expression_in_command_position_is_refused() {
    let repo = Repository::new();
    repo.write(
        ".github/workflows/check.yml",
        "jobs:\n  check:\n    steps:\n      - run: ${{ vars.CHECKER }} -c 'print(1)'\n",
    );
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(findings(&output).iter().any(|finding| {
        finding["path"] == ".github/workflows/check.yml"
            && finding["evidence"]
                .as_str()
                .is_some_and(|value| value.contains("run-1:${{ vars.CHECKER }}"))
    }));
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn unresolved_make_command_variable_is_refused() {
    let repo = Repository::new();
    repo.write("Makefile", "check:\n\t$(CHECKER) src/check\n");
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
            .any(|finding| finding["path"] == "Makefile")
    );
}

#[test]
#[trace("TC-211", "NFR-005-AC-8")]
fn rust_fixture_strings_and_dynamic_rust_hosts_are_not_launches() {
    let repo = Repository::new();
    repo.write("src/host.rs", "use clap::Command;\npub fn run(program: &str) { let _ = Command::new(program); let _ = Command::new(\"python3\"); let _fixture = \"Command::new(\\\"python3\\\")\"; }\n");
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(output.status.code(), Some(0));
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn a_safe_const_name_cannot_hide_a_shadowed_process_argument() {
    let repo = Repository::new();
    repo.write("src/host.rs", "use std::process::Command;\nconst TOOL: &str = \"cargo\";\npub fn run(TOOL: &str) { let _ = Command::new(TOOL); }\n");
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
            .any(|finding| finding["kind"] == "dynamic-rust-launch")
    );
}

#[test]
#[trace("TC-211", "NFR-005-AC-8")]
fn cargo_environment_fallback_is_a_proven_rust_launcher() {
    let repo = Repository::new();
    repo.write("examples/default_dependencies.rs", "use std::{env, process::Command};\npub fn run() { let cargo = env::var_os(\"CARGO\").unwrap_or_else(|| \"cargo\".into()); let _ = Command::new(cargo).args([\"tree\"]).output(); }\n");
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "tl-syntax",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(output.status.code(), Some(0));
}

#[test]
#[trace("TC-211", "NFR-005-AC-8")]
fn this_guards_rust_sources_do_not_refuse_the_guard_itself() {
    let repo = Repository::new();
    repo.write("src/main.rs", include_str!("../src/main.rs"));
    repo.write(
        "src/language_boundary.rs",
        include_str!("../src/language_boundary.rs"),
    );
    repo.write(
        "tests/language_boundary_cli.rs",
        include_str!("language_boundary_cli.rs"),
    );
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "engineering-assurance",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
#[trace("TC-209", "NFR-005-AC-6")]
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
#[trace("TC-211", "NFR-005-AC-8")]
fn exact_fixture_and_test_double_exceptions_are_accepted() {
    let repo = Repository::new();
    repo.write("docs/images/figures.py", "#!/usr/bin/env python3\n");
    repo.write("tests/python/binding_test.py", "#!/usr/bin/env python3\n");
    repo.write(
        "tests/fixtures/case/input/sample.py",
        "#!/usr/bin/env python3\n",
    );
    repo.write("cases/example/input/sample.py", "print('fixture')\n");
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
#[trace("TC-211", "NFR-005-AC-8")]
fn malformed_rust_fixture_input_is_data_but_adjacent_python_helper_is_governed() {
    let repo = Repository::new();
    repo.write("tests/fixtures/symbols/broken/truncated.rs", "fn broken(\n");
    repo.write(
        "fixtures/resilience/arbitrary-bytes/rust/input/src/junk.rs",
        "not rust at all\n",
    );
    let fixture_head = repo.commit();
    let fixture = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&fixture_head),
    );
    assert_eq!(fixture.status.code(), Some(0));
    repo.write(
        "tests/fixtures/symbols/broken/check.py",
        "#!/usr/bin/env python3\nprint('check')\n",
    );
    let helper_head = repo.commit();
    let helper = guard(
        repo.root(),
        "quire-rs",
        Some(&fixture_head),
        Some(&helper_head),
    );
    assert_eq!(helper.status.code(), Some(1));
    assert!(
        findings(&helper)
            .iter()
            .any(|finding| { finding["path"] == "tests/fixtures/symbols/broken/check.py" })
    );
}

#[test]
#[trace("TC-211", "NFR-005-AC-8")]
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
#[trace("TC-209", "NFR-005-AC-6")]
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
#[trace("TC-209", "NFR-005-AC-6")]
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
#[trace("TC-210", "NFR-005-AC-7")]
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

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn existing_node_action_source_expansion_is_refused() {
    let repo = Repository::new();
    repo.write(
        ".github/actions/check/action.yml",
        "name: Check\nruns:\n  using: node20\n  main: index.js\n",
    );
    repo.write(".github/actions/check/index.js", "console.log('old');\n");
    let base = repo.commit();
    repo.write(".github/actions/check/index.js", "console.log('new');\n");
    let head = repo.commit();
    let output = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(output.status.code(), Some(1));
    assert!(findings(&output).iter().any(|finding| finding["path"]
        == ".github/actions/check/index.js"
        && finding["kind"] == "executable-file"));
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn yaml_alias_run_and_external_java_action_are_refused() {
    let repo = Repository::new();
    repo.write(
        ".github/workflows/audit.yml",
        "name: Audit\non: pull_request\njobs:\n  audit:\n    steps:\n      - run: &audit python3 -c 'print(1)'\n      - run: *audit\n      - uses: actions/setup-java@v4\n",
    );
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(output.status.code(), Some(1));
    assert_schema(&output);
    let found = findings(&output);
    assert_eq!(
        found
            .iter()
            .filter(|item| item["path"] == ".github/workflows/audit.yml"
                && item["kind"] == "inline-launch")
            .count(),
        3
    );
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn process_module_alias_and_valid_fixture_helper_are_refused() {
    let repo = Repository::new();
    repo.write(
        "src/check.rs",
        "use std::process as proc;\nfn check() { proc::Command::new(\"python3\"); }\n",
    );
    repo.write(
        "tests/fixtures/helper.rs",
        "fn check() { std::process::Command::new(\"python3\"); }\n",
    );
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(output.status.code(), Some(1));
    assert_schema(&output);
    let found = findings(&output);
    assert!(
        found
            .iter()
            .any(|item| item["path"] == "src/check.rs" && item["kind"] == "rust-wrapper-launch")
    );
    assert!(
        found
            .iter()
            .any(|item| item["path"] == "tests/fixtures/helper.rs"
                && item["kind"] == "rust-wrapper-launch")
    );
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn docker_action_and_dependency_only_make_alias_are_refused() {
    let repo = Repository::new();
    repo.write("Makefile", "old:\n\tpython3 scripts/old.py\n");
    let base = repo.commit();
    repo.write("Makefile", "old:\n\tpython3 scripts/old.py\nci: old\n");
    repo.write(
        ".github/actions/check/action.yml",
        "name: Check\nruns:\n  using: docker\n  image: Dockerfile\n",
    );
    repo.write(".github/actions/check/Dockerfile", "FROM node:20\n");
    let head = repo.commit();
    let output = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(output.status.code(), Some(1));
    assert_schema(&output);
    let found = findings(&output);
    assert!(found.iter().any(|item| {
        item["path"] == "Makefile"
            && item["evidence"]
                .as_str()
                .is_some_and(|text| text.contains("target:ci"))
    }));
    assert!(
        found
            .iter()
            .any(|item| item["path"] == ".github/actions/check/action.yml"
                && item["kind"] == "inline-launch")
    );
    assert!(
        found
            .iter()
            .any(|item| item["path"] == ".github/actions/check/Dockerfile"
                && item["kind"] == "action-container")
    );
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn imported_python_and_node_helpers_and_new_ui_ts_are_refused() {
    let repo = Repository::new();
    repo.write("scripts/audit.py", "from src import helper\n");
    repo.write(
        "scripts/audit.js",
        "const helper = require('../lib/helper.js');\n",
    );
    let base = repo.commit();
    repo.write("src/helper.py", "print('new')\n");
    repo.write("lib/helper.js", "module.exports = 1;\n");
    repo.write("ui/widget.ts", "export const widget = 1;\n");
    let head = repo.commit();
    let output = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(output.status.code(), Some(1));
    assert_schema(&output);
    let found = findings(&output);
    assert!(
        found
            .iter()
            .any(|item| item["path"] == "src/helper.py" && item["kind"] == "foreign-source")
    );
    assert!(
        found
            .iter()
            .any(|item| item["path"] == "lib/helper.js" && item["kind"] == "foreign-source")
    );
    assert!(
        found
            .iter()
            .any(|item| item["path"] == "ui/widget.ts" && item["kind"] == "foreign-source")
    );
}

#[test]
#[trace("TC-211", "NFR-005-AC-8")]
fn templated_package_manifest_is_digested_without_json_parsing() {
    let repo = Repository::new();
    let path = "templates/{{cookiecutter.repo_name}}/package.json";
    repo.write(
        path,
        "{%- if cookiecutter.node %}\n{\"scripts\":{\"build\":\"node build.js\"}}\n{%- endif %}\n",
    );
    let base = repo.commit();
    let unchanged = guard(
        repo.root(),
        "quoin",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&base),
    );
    assert_eq!(unchanged.status.code(), Some(1));
    assert_schema(&unchanged);
    assert!(
        findings(&unchanged)
            .iter()
            .any(|item| item["path"] == path && item["kind"] == "templated-npm-manifest")
    );
    repo.write(path, "{%- if cookiecutter.node %}\n{\"scripts\":{\"build\":\"python3 build.py\"}}\n{%- endif %}\n");
    let head = repo.commit();
    let changed = guard(repo.root(), "quoin", Some(&base), Some(&head));
    assert_eq!(changed.status.code(), Some(1));
    assert_schema(&changed);
    assert!(
        findings(&changed)
            .iter()
            .any(|item| item["path"] == path && item["kind"] == "templated-npm-manifest")
    );
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn local_docker_action_and_unrecognized_external_action_are_refused() {
    let repo = Repository::new();
    repo.write(
        ".github/workflows/check.yml",
        "name: Check\non: pull_request\njobs:\n  check:\n    steps:\n      - uses: ./tools/check\n      - uses: vendor/run-python@v1\n",
    );
    repo.write(
        "tools/check/action.yml",
        "name: Check\nruns:\n  using: docker\n  image: docker://node:22\n",
    );
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "quire-rs",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(output.status.code(), Some(1));
    assert_schema(&output);
    let found = findings(&output);
    assert!(found.iter().any(|item| item["path"] == "tools/check/action.yml" && item["kind"] == "inline-launch"));
    assert_eq!(
        found
            .iter()
            .filter(|item| item["path"] == ".github/workflows/check.yml"
                && item["kind"] == "inline-launch")
            .count(),
        2
    );
}

#[test]
#[trace("TC-211", "NFR-005-AC-8")]
fn exact_trusted_rust_checker_workflow_refs_are_accepted() {
    let repo = Repository::new();
    repo.write(
        ".github/workflows/language-boundary.yml",
        "name: Language boundary\non: pull_request_target\njobs:\n  sha:\n    uses: agent-ix/engineering-assurance/.github/workflows/language-boundary-reusable.yml@71dbe634937a528673db1ce6ca5bb2f0d4ff06ce\n  main:\n    uses: agent-ix/engineering-assurance/.github/workflows/language-boundary-reusable.yml@main\n",
    );
    let head = repo.commit();
    let output = guard(
        repo.root(),
        "qa-corpus",
        Some(&sha(repo.root(), "HEAD^")),
        Some(&head),
    );
    assert_eq!(output.status.code(), Some(0));
    assert_schema(&output);
    assert!(findings(&output).is_empty());
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn imported_typescript_helper_and_unrelated_ui_module_are_refused() {
    let repo = Repository::new();
    repo.write(
        "scripts/run.ts",
        "import { check } from '../lib/helper';\ncheck();\n",
    );
    let base = repo.commit();
    repo.write("lib/helper.ts", "export const check = () => 1;\n");
    repo.write("ui/widget.ts", "export const widget = 1;\n");
    let head = repo.commit();
    let output = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(output.status.code(), Some(1));
    assert_schema(&output);
    let found = findings(&output);
    assert!(
        found
            .iter()
            .any(|item| item["path"] == "lib/helper.ts" && item["kind"] == "foreign-source")
    );
    assert!(
        found
            .iter()
            .any(|item| item["path"] == "ui/widget.ts" && item["kind"] == "foreign-source")
    );
}

#[test]
#[trace("TC-210", "NFR-005-AC-7", "TC-211", "NFR-005-AC-8")]
fn new_runtime_dependency_is_a_structured_manifest_finding() {
    let repo = Repository::new();
    repo.write("package.json", r#"{"scripts":{"check":"node check.js"}}"#);
    repo.write("check.js", "require('electron');\n");
    let base = repo.commit();
    repo.write(
        "package.json",
        r#"{"scripts":{"check":"node check.js"},"dependencies":{"electron":"^33.0.0"}}"#,
    );
    let head = repo.commit();
    let output = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(output.status.code(), Some(1));
    assert_schema(&output);
    assert!(
        findings(&output)
            .iter()
            .any(|item| item["path"] == "package.json"
                && item["kind"] == "npm-dependency"
                && item["evidence"] == "dependencies:electron:^33.0.0")
    );
}

#[test]
#[trace("TC-210", "NFR-005-AC-7")]
fn new_shell_source_helper_is_a_structured_finding() {
    let repo = Repository::new();
    repo.write("scripts/run.sh", "source lib/helper.sh\n");
    let base = repo.commit();
    repo.write("lib/helper.sh", "python3 -c 'print(1)'\n");
    let head = repo.commit();
    let output = guard(repo.root(), "quire-rs", Some(&base), Some(&head));
    assert_eq!(output.status.code(), Some(1));
    assert_schema(&output);
    assert!(
        findings(&output)
            .iter()
            .any(|item| item["path"] == "lib/helper.sh" && item["kind"] == "foreign-source")
    );
}
