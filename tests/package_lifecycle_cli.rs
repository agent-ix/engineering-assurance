// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Host-boundary tests for npm staging, cleanup, and publication refusal.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

use engineering_assurance::package_lifecycle::PACKAGE_LIFECYCLE_PROTOCOL;
use ix_trace_rs::trace;
use serde_json::Value;

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

const STAGED_NAMES: [&str; 6] = [
    "manifest.yaml",
    "compatibility-matrix.json",
    "contracts",
    "fixtures",
    "schemas",
    "skeletons",
];

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(name: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "engineering-assurance-package-{name}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("unique test directory must be creatable");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().expect("fixture path must have a parent"))
        .expect("fixture parent must be creatable");
    fs::write(path, bytes).expect("fixture must be writable");
}

fn complete_fixture(name: &str) -> TestDirectory {
    let directory = TestDirectory::new(name);
    let module = directory.path().join("engineering_assurance");
    write(&module.join("manifest.yaml"), b"name: fictional\n");
    write(
        &module.join("compatibility-matrix.json"),
        br#"{"state":"fictional"}"#,
    );
    for name in ["contracts", "fixtures", "schemas", "skeletons"] {
        write(
            &module.join(name).join("fictional.txt"),
            format!("{name} fixture\n").as_bytes(),
        );
    }
    directory
}

fn empty_fixture(name: &str) -> TestDirectory {
    let directory = TestDirectory::new(name);
    let module = directory.path().join("engineering_assurance");
    write(&module.join("manifest.yaml"), b"");
    write(&module.join("compatibility-matrix.json"), b"");
    for name in ["contracts", "fixtures", "schemas", "skeletons"] {
        fs::create_dir(module.join(name)).unwrap();
    }
    directory
}

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(arguments)
        .output()
        .expect("the Cargo-built CLI must terminate")
}

fn lifecycle(root: &Path, operation: &str) -> Output {
    run(&[
        "package-lifecycle",
        operation,
        "--root",
        root.to_str().expect("test path must be UTF-8"),
    ])
}

fn lifecycle_hook(root: &Path, operation: &str) -> Output {
    run(&[
        "package-lifecycle",
        operation,
        "--root",
        root.to_str().expect("test path must be UTF-8"),
        "--npm-hook",
    ])
}

fn result(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout must contain one JSON value")
}

fn assert_no_staged_destination(root: &Path) {
    for name in STAGED_NAMES {
        assert!(!root.join(name).exists(), "{name} must not be staged");
    }
}

#[test]
#[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
fn tc_111_stage_and_clean_preserve_exact_bytes_and_absence_is_idempotent() {
    let fixture = complete_fixture("round-trip");
    let staged = lifecycle(fixture.path(), "stage");
    assert!(
        staged.status.success(),
        "{}",
        String::from_utf8_lossy(&staged.stderr)
    );
    assert_eq!(
        result(&staged),
        serde_json::json!({
            "protocol": PACKAGE_LIFECYCLE_PROTOCOL,
            "operation": "stage",
            "outcome": "staged",
        })
    );
    for name in STAGED_NAMES {
        let source = fixture.path().join("engineering_assurance").join(name);
        let destination = fixture.path().join(name);
        if source.is_file() {
            assert_eq!(fs::read(source).unwrap(), fs::read(destination).unwrap());
        } else {
            assert_eq!(
                fs::read(source.join("fictional.txt")).unwrap(),
                fs::read(destination.join("fictional.txt")).unwrap()
            );
        }
    }

    let cleaned = lifecycle(fixture.path(), "clean");
    assert!(cleaned.status.success());
    assert_eq!(result(&cleaned)["outcome"], "cleaned");
    assert_no_staged_destination(fixture.path());

    let repeated = lifecycle(fixture.path(), "clean");
    assert!(repeated.status.success());
    assert_eq!(result(&repeated)["outcome"], "cleaned");
}

#[test]
#[trace("TC-111", "FR-017-AC-3")]
fn tc_111_stage_preflight_refuses_missing_or_preexisting_destinations_without_writes() {
    let missing = complete_fixture("missing");
    fs::remove_dir_all(missing.path().join("engineering_assurance/schemas")).unwrap();
    let output = lifecycle(missing.path(), "stage");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(result(&output)["code"], "package_source_missing");
    assert_no_staged_destination(missing.path());

    let existing = complete_fixture("existing");
    write(&existing.path().join("manifest.yaml"), b"owner bytes\n");
    let output = lifecycle(existing.path(), "stage");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(result(&output)["code"], "package_destination_exists");
    assert_eq!(
        fs::read(existing.path().join("manifest.yaml")).unwrap(),
        b"owner bytes\n"
    );
    for name in STAGED_NAMES.into_iter().skip(1) {
        assert!(!existing.path().join(name).exists());
    }
}

#[test]
#[trace("TC-111", "FR-017-AC-3")]
fn tc_111_cleanup_refuses_partial_or_changed_population_without_deleting_any_member() {
    let partial = complete_fixture("partial");
    assert!(lifecycle(partial.path(), "stage").status.success());
    fs::remove_file(partial.path().join("manifest.yaml")).unwrap();
    let output = lifecycle(partial.path(), "clean");
    assert_eq!(
        result(&output)["code"],
        "package_destination_population_incomplete"
    );
    for name in STAGED_NAMES.into_iter().skip(1) {
        assert!(partial.path().join(name).exists());
    }

    let changed = complete_fixture("changed");
    assert!(lifecycle(changed.path(), "stage").status.success());
    write(&changed.path().join("schemas/added.txt"), b"not selected\n");
    let output = lifecycle(changed.path(), "clean");
    assert_eq!(result(&output)["code"], "package_destination_mismatch");
    for name in STAGED_NAMES {
        assert!(changed.path().join(name).exists());
    }
}

#[cfg(unix)]
#[test]
#[trace("TC-111", "FR-017-AC-3")]
fn tc_111_source_links_special_files_and_nonportable_names_fail_before_writes() {
    use std::os::unix::fs::symlink;

    let linked = complete_fixture("linked");
    symlink(
        linked.path().join("engineering_assurance/manifest.yaml"),
        linked
            .path()
            .join("engineering_assurance/fixtures/linked.txt"),
    )
    .unwrap();
    let output = lifecycle(linked.path(), "stage");
    assert_eq!(result(&output)["code"], "package_entry_kind_invalid");
    assert_no_staged_destination(linked.path());

    let linked_root = complete_fixture("linked-root");
    fs::rename(
        linked_root.path().join("engineering_assurance"),
        linked_root.path().join("actual-module"),
    )
    .unwrap();
    symlink(
        linked_root.path().join("actual-module"),
        linked_root.path().join("engineering_assurance"),
    )
    .unwrap();
    let output = lifecycle(linked_root.path(), "stage");
    assert_eq!(result(&output)["code"], "package_entry_kind_invalid");
    assert_no_staged_destination(linked_root.path());

    let special = complete_fixture("special");
    let fifo = special
        .path()
        .join("engineering_assurance/contracts/special.fifo");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo must be available for the Unix special-file case")
            .success()
    );
    let output = lifecycle(special.path(), "stage");
    assert_eq!(result(&output)["code"], "package_entry_kind_invalid");
    assert_no_staged_destination(special.path());

    let nonportable = complete_fixture("nonportable");
    write(
        &nonportable
            .path()
            .join("engineering_assurance/schemas/not\\portable.txt"),
        b"fixture\n",
    );
    let output = lifecycle(nonportable.path(), "stage");
    assert_eq!(result(&output)["code"], "package_path_invalid");
    assert_no_staged_destination(nonportable.path());
}

#[test]
#[trace("TC-111", "FR-017-AC-3")]
fn tc_111_file_and_entry_resource_ceilings_fail_before_writes() {
    let oversized = complete_fixture("oversized");
    let file = fs::File::create(
        oversized
            .path()
            .join("engineering_assurance/fixtures/oversized.bin"),
    )
    .unwrap();
    file.set_len(8_388_609).unwrap();
    let output = lifecycle(oversized.path(), "stage");
    assert_eq!(result(&output)["code"], "package_file_too_large");
    assert_no_staged_destination(oversized.path());

    let total = complete_fixture("total-bytes");
    for index in 0..3 {
        let file = fs::File::create(
            total
                .path()
                .join(format!("engineering_assurance/fixtures/total-{index}.bin")),
        )
        .unwrap();
        file.set_len(8_388_608).unwrap();
    }
    let output = lifecycle(total.path(), "stage");
    assert_eq!(result(&output)["code"], "package_total_bytes_too_large");
    assert_no_staged_destination(total.path());

    let excessive = complete_fixture("excessive");
    let root = excessive.path().join("engineering_assurance/fixtures/many");
    fs::create_dir(&root).unwrap();
    for index in 0..4_091 {
        fs::create_dir(root.join(format!("entry-{index:04}"))).unwrap();
    }
    let output = lifecycle(excessive.path(), "stage");
    assert_eq!(
        result(&output)["code"],
        "package_entry_population_too_large"
    );
    assert_no_staged_destination(excessive.path());
}

#[test]
#[trace("TC-111", "FR-017-AC-3")]
fn tc_111_exact_file_total_and_entry_resource_ceilings_are_admitted() {
    let file_limit = empty_fixture("exact-file-bytes");
    let file = fs::File::create(
        file_limit
            .path()
            .join("engineering_assurance/fixtures/exact.bin"),
    )
    .unwrap();
    file.set_len(8_388_608).unwrap();
    assert!(lifecycle(file_limit.path(), "stage").status.success());
    assert!(lifecycle(file_limit.path(), "clean").status.success());

    let total_limit = empty_fixture("exact-total-bytes");
    for index in 0..2 {
        let file = fs::File::create(
            total_limit
                .path()
                .join(format!("engineering_assurance/fixtures/exact-{index}.bin")),
        )
        .unwrap();
        file.set_len(8_388_608).unwrap();
    }
    assert!(lifecycle(total_limit.path(), "stage").status.success());
    assert!(lifecycle(total_limit.path(), "clean").status.success());

    let entry_limit = empty_fixture("exact-entries");
    let root = entry_limit
        .path()
        .join("engineering_assurance/fixtures/many");
    fs::create_dir(&root).unwrap();
    for index in 0..4_089 {
        fs::create_dir(root.join(format!("entry-{index:04}"))).unwrap();
    }
    assert!(lifecycle(entry_limit.path(), "stage").status.success());
    assert!(lifecycle(entry_limit.path(), "clean").status.success());
}

#[test]
#[trace("TC-111", "FR-017-AC-3")]
fn tc_111_publication_refusal_is_unconditional_and_machine_readable() {
    let output = run(&["package-lifecycle", "refuse-publication"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        result(&output),
        serde_json::json!({
            "protocol": PACKAGE_LIFECYCLE_PROTOCOL,
            "operation": "refuse-publication",
            "outcome": "publication-refused",
        })
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("publication is disabled"));

    let hook = run(&["package-lifecycle", "refuse-publication", "--npm-hook"]);
    assert_eq!(hook.status.code(), Some(1));
    assert!(hook.stdout.is_empty());
    assert!(String::from_utf8_lossy(&hook.stderr).contains("publication is disabled"));
}

#[test]
#[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
fn tc_111_npm_hook_mode_preserves_the_package_managers_stdout() {
    let fixture = complete_fixture("hook-mode");
    let staged = lifecycle_hook(fixture.path(), "stage");
    assert!(staged.status.success());
    assert!(staged.stdout.is_empty());
    assert!(staged.stderr.is_empty());

    let cleaned = lifecycle_hook(fixture.path(), "clean");
    assert!(cleaned.status.success());
    assert!(cleaned.stdout.is_empty());
    assert!(cleaned.stderr.is_empty());
}

#[test]
#[trace("TC-112", "FR-017-AC-4", "FR-017-CON-2")]
fn tc_112_npm_lifecycle_hooks_are_exact_declarative_rust_dispatch() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let package: Value = serde_json::from_slice(
        &fs::read(root.join("package.json")).expect("package manifest must be readable"),
    )
    .expect("package manifest must be JSON");
    assert_eq!(package["private"], true);
    assert_eq!(
        package["scripts"],
        serde_json::json!({
            "prepack": "cargo run --locked --quiet -- package-lifecycle stage --root . --npm-hook",
            "postpack": "cargo run --locked --quiet -- package-lifecycle clean --root . --npm-hook",
            "prepublishOnly": "cargo run --locked --quiet -- package-lifecycle refuse-publication --npm-hook",
        })
    );
    assert!(!root.join("scripts/stage-npm.mjs").exists());
    assert!(!root.join("scripts/refuse-publication.mjs").exists());
}
