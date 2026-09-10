// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Git/tree host-boundary tests for the Rust content-rights command.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

use engineering_assurance::content_rights::CONTENT_RIGHTS_TREE_PROTOCOL;
use ix_trace_rs::trace;
use serde::Deserialize;
use serde_json::Value;

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TestRepository {
    path: PathBuf,
}

impl TestRepository {
    fn new(name: &str) -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "engineering-assurance-rights-{name}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("unique test directory must be creatable");
        let status = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&path)
            .status()
            .expect("git must start");
        assert!(status.success());
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.path.join(relative);
        fs::create_dir_all(path.parent().expect("fixture path must have a parent"))
            .expect("fixture parent must be creatable");
        fs::write(path, bytes).expect("fixture must be writable");
    }

    fn add(&self, relative: &str) {
        let status = Command::new("git")
            .args(["add", "--", relative])
            .current_dir(&self.path)
            .status()
            .expect("git add must start");
        assert!(status.success());
    }
}

impl Drop for TestRepository {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn run(root: &Path, protected_tokens: Option<&str>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"));
    command.args([
        "content-rights-tree",
        "--root",
        root.to_str().expect("test path must be UTF-8"),
    ]);
    if let Some(tokens) = protected_tokens {
        command.env("ASSURANCE_PROTECTED_TOKENS", tokens);
    } else {
        command.env_remove("ASSURANCE_PROTECTED_TOKENS");
    }
    command.output().expect("content-rights command must run")
}

fn run_retained(root: &Path, protected_tokens: Option<&str>) -> Output {
    let mut command = Command::new("python3");
    command
        .arg(root.join("scripts/check_content_rights.py"))
        .arg("--tree")
        .current_dir(root);
    if let Some(tokens) = protected_tokens {
        command.env("ASSURANCE_PROTECTED_TOKENS", tokens);
    } else {
        command.env_remove("ASSURANCE_PROTECTED_TOKENS");
    }
    command.output().expect("retained checker must run")
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout must contain one JSON value")
}

#[test]
#[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
fn tc_111_git_population_inspects_tracked_untracked_and_links_but_not_ignored_files() {
    let repository = TestRepository::new("population");
    repository.write(".gitignore", b"ignored.md\n");
    repository.write("tracked.md", b"ordinary\n");
    let protected_and_url = ["STRASSE\n", "https:", "//example.invalid/source\n"].concat();
    repository.write("untracked.md", protected_and_url.as_bytes());
    let ignored_location = ["/", "home", "/private/ignored\n"].concat();
    repository.write("ignored.md", ignored_location.as_bytes());
    repository.add(".gitignore");
    repository.add("tracked.md");

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("tracked.md", repository.path().join("linked.md"))
            .expect("fixture link must be creatable");
        repository.add("linked.md");
    }

    let output = run(repository.path(), Some("Straße"));
    assert_eq!(output.status.code(), Some(1));
    let result = json(&output);
    assert_eq!(result["protocol"], CONTENT_RIGHTS_TREE_PROTOCOL);
    assert_eq!(result["outcome"], "withheld");
    #[cfg(unix)]
    assert_eq!(result["inspected_entries"], 4);
    #[cfg(not(unix))]
    assert_eq!(result["inspected_entries"], 3);
    let findings = result["findings"].as_array().expect("findings array");
    assert!(findings.iter().any(|finding| {
        finding["path"] == "untracked.md" && finding["category"] == "protected-local-token"
    }));
    assert!(findings.iter().any(|finding| {
        finding["path"] == "untracked.md" && finding["category"] == "unapproved-external-url"
    }));
    #[cfg(unix)]
    assert!(findings.iter().any(|finding| {
        finding["path"] == "linked.md" && finding["category"] == "symbolic-link"
    }));
    assert!(!output.stdout.windows(7).any(|window| window == b"STRASSE"));
    assert!(
        !output
            .stdout
            .windows(7)
            .any(|window| window == b"Stra\xC3\x9Fe")
    );
    assert!(
        !output
            .stdout
            .windows(10)
            .any(|window| window == b"ignored.md")
    );
}

#[test]
#[trace("TC-111", "FR-017-AC-3", "FR-018-AC-2")]
fn tc_111_same_revision_retained_tree_status_and_findings_match() {
    let repository = TestRepository::new("retained-parity");
    repository.write(
        "scripts/check_content_rights.py",
        include_bytes!("../scripts/check_content_rights.py"),
    );
    repository.write("ordinary.md", b"ordinary\n");

    let old = run_retained(repository.path(), None);
    let new = run(repository.path(), None);
    assert!(
        old.status.success(),
        "{}",
        String::from_utf8_lossy(&old.stderr)
    );
    assert!(
        new.status.success(),
        "{}",
        String::from_utf8_lossy(&new.stderr)
    );
    assert_eq!(json(&new)["outcome"], "accepted");

    let protected_and_url = ["STRASSE\n", "https:", "//example.invalid/source\n"].concat();
    repository.write("candidate.md", protected_and_url.as_bytes());
    #[cfg(unix)]
    std::os::unix::fs::symlink("ordinary.md", repository.path().join("linked.md"))
        .expect("fixture link must be creatable");

    let old = run_retained(repository.path(), Some("Straße"));
    let new = run(repository.path(), Some("Straße"));
    assert_eq!(old.status.code(), Some(1));
    assert_eq!(new.status.code(), Some(1));
    let stderr = String::from_utf8(old.stderr).expect("retained diagnostics must be UTF-8");
    let retained = stderr.lines().skip(1).collect::<Vec<_>>();
    #[cfg(unix)]
    assert_eq!(
        retained,
        vec![
            "candidate.md:1: protected local token",
            "candidate.md:2: unapproved external URL",
            "linked.md:0: symbolic link",
        ]
    );
    #[cfg(not(unix))]
    assert_eq!(
        retained,
        vec![
            "candidate.md:1: protected local token",
            "candidate.md:2: unapproved external URL",
        ]
    );
    let result = json(&new);
    let findings = result["findings"].as_array().expect("findings array");
    assert!(findings.iter().any(|finding| {
        finding["path"] == "candidate.md"
            && finding["line"] == 1
            && finding["category"] == "protected-local-token"
    }));
    assert!(findings.iter().any(|finding| {
        finding["path"] == "candidate.md"
            && finding["line"] == 2
            && finding["category"] == "unapproved-external-url"
    }));
    #[cfg(unix)]
    assert!(findings.iter().any(|finding| {
        finding["path"] == "linked.md"
            && finding["line"] == 0
            && finding["category"] == "symbolic-link"
    }));
    assert_eq!(findings.len(), retained.len());
}

#[test]
#[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
fn tc_111_root_identity_and_unsupported_entries_fail_with_typed_errors() {
    let non_repository = TestRepository::new("not-a-repository");
    fs::remove_dir_all(non_repository.path().join(".git")).unwrap();
    let output = run(non_repository.path(), None);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(json(&output)["code"], "content_rights_git_command_failed");

    let repository = TestRepository::new("root-and-kind");
    repository.write("nested/ordinary.md", b"ordinary\n");
    let nested = run(&repository.path().join("nested"), None);
    assert_eq!(nested.status.code(), Some(2));
    assert_eq!(json(&nested)["code"], "content_rights_root_not_top_level");

    #[cfg(unix)]
    {
        repository.write("special", b"");
        repository.add("special");
        fs::remove_file(repository.path().join("special")).unwrap();
        assert!(
            Command::new("mkfifo")
                .arg(repository.path().join("special"))
                .status()
                .expect("mkfifo must start")
                .success()
        );
        let special = run(repository.path(), None);
        assert_eq!(special.status.code(), Some(2));
        assert_eq!(json(&special)["code"], "content_rights_entry_invalid");
    }
}

#[test]
#[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
fn tc_111_file_and_total_byte_boundaries_are_enforced() {
    let exact_file = TestRepository::new("exact-file");
    fs::File::create(exact_file.path().join("large.md"))
        .unwrap()
        .set_len(8_388_608)
        .unwrap();
    let exact = run(exact_file.path(), None);
    assert_eq!(exact.status.code(), Some(1));
    assert_eq!(json(&exact)["outcome"], "withheld");

    let over_file = TestRepository::new("over-file");
    fs::File::create(over_file.path().join("large.md"))
        .unwrap()
        .set_len(8_388_609)
        .unwrap();
    let over = run(over_file.path(), None);
    assert_eq!(over.status.code(), Some(2));
    assert_eq!(json(&over)["code"], "content_rights_file_too_large");

    let total = TestRepository::new("total");
    for index in 0..8 {
        fs::File::create(total.path().join(format!("large-{index}.md")))
            .unwrap()
            .set_len(8_388_608)
            .unwrap();
    }
    let exact = run(total.path(), None);
    assert_eq!(exact.status.code(), Some(1));
    total.write("over.md", b"x");
    let over = run(total.path(), None);
    assert_eq!(over.status.code(), Some(2));
    assert_eq!(json(&over)["code"], "content_rights_total_bytes_too_large");
}

#[test]
#[trace("TC-111", "FR-017-AC-3")]
fn tc_111_public_repository_rights_metadata_remains_consistent() {
    #[derive(Deserialize)]
    struct Policy {
        repository: Repository,
    }
    #[derive(Deserialize)]
    struct Repository {
        visibility: String,
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let policy: Policy = yaml_serde::from_slice(
        &fs::read(root.join("content-rights.yaml")).expect("policy must be readable"),
    )
    .expect("policy must be valid YAML");
    assert_eq!(policy.repository.visibility, "public");
    assert!(
        fs::read_to_string(root.join("CONTENT_RIGHTS.md"))
            .unwrap()
            .contains("This public repository")
    );
    assert!(
        fs::read_to_string(root.join("AGENTS.md"))
            .unwrap()
            .contains("Keep the repository public")
    );
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    assert!(readme.contains("The repository is public"));
    assert!(readme.contains("Registry packages remain private and unpublished"));
}

#[test]
#[trace("TC-112", "FR-017-AC-4", "FR-017-CON-2")]
fn tc_112_make_dispatch_is_declarative_rust_invocation() {
    let makefile = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Makefile"))
        .expect("Makefile must be readable");
    assert!(!makefile.contains("scripts/check_content_rights.py --tree"));
    assert_eq!(makefile.matches("content-rights-tree --root .").count(), 1);
}
