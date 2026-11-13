// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for the `release-decision` workflow command (FR-048)
//! against a real `ix-flow`, real git repositories and fictional ed25519 keys
//! generated inside each test.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const OWNER: &str = "alice@example.test";
const BASELINE_PATH: &str = "baseline.md";
const RECORD_PATH: &str = "release/REL-1.md";
const SNAPSHOT_PATH: &str = "release/snapshot.json";
const SNAPSHOT_TEXT: &str = "{ \"evidence\" :  \"snapshot\" ,\n  not: [json\n\n";
const RESULT_PROTOCOL: &str = "engineering-assurance.release-decision-result/v1";
const REQUEST_PROTOCOL: &str = "engineering-assurance.release-decision-request/v1";

struct Fixture {
    dir: tempfile::TempDir,
    baseline: String,
}

impl Fixture {
    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn repo(&self) -> PathBuf {
        self.root().join("repo")
    }

    fn state(&self) -> PathBuf {
        self.root().join("state")
    }

    fn input(&self, name: &str, content: &str) -> String {
        let path = self.root().join(name);
        fs::write(&path, content).expect("input file");
        path.display().to_string()
    }
}

fn run_program(program: &str, directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new(program)
        .args(arguments)
        .current_dir(directory)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .output()
        .expect("program must start");
    assert!(
        output.status.success(),
        "{program} {arguments:?}: {output:?}"
    );
    String::from_utf8(output.stdout)
        .expect("utf-8")
        .trim_end()
        .to_owned()
}

fn git(repo: &Path, arguments: &[&str]) -> String {
    let mut all = vec!["-c", "user.name=t", "-c", "user.email=t@example.invalid"];
    all.extend_from_slice(arguments);
    run_program("git", repo, &all)
}

fn commit_files(repo: &Path, files: &[(&str, &str)], message: &str) -> String {
    for (path, content) in files {
        let target = repo.join(path);
        fs::create_dir_all(target.parent().expect("parent")).expect("directories");
        fs::write(target, content).expect("file");
        git(repo, &["add", "--", path]);
    }
    git(repo, &["commit", "-q", "-m", message]);
    git(repo, &["rev-parse", "HEAD"])
}

/// A repository whose baseline commit holds the Baseline file and its items,
/// and a key `a` mapped to the owner in an allowed-signers file.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    run_program(
        "ssh-keygen",
        root,
        &["-q", "-t", "ed25519", "-N", "", "-C", "a", "-f", "a"],
    );
    let public = fs::read_to_string(root.join("a.pub")).expect("key");
    fs::write(root.join("allowed_signers"), format!("{OWNER} {public}")).expect("signers");
    let repo = root.join("repo");
    fs::create_dir(&repo).expect("repo");
    fs::create_dir(root.join("state")).expect("state");
    git(&repo, &["init", "-q", "-b", "main"]);
    let baseline = "---\nid: BL-1\ntitle: Baseline one\ntype: Baseline\nname: Juniper service\nitems:\n  \
         - path: spec/a.md\n    kind: spec_artifact\n  - path: soup/s.md\n    kind: soup_record\n  \
         - path: evidence/e.txt\n    kind: evidence\n---\n\nThe baseline.\n";
    let commit = commit_files(
        &repo,
        &[
            ("spec/a.md", "spec a\n"),
            ("soup/s.md", "soup s\n"),
            ("evidence/e.txt", "evidence e\n"),
            (BASELINE_PATH, baseline),
        ],
        "baseline",
    );
    Fixture {
        dir,
        baseline: commit,
    }
}

/// A second Baseline file, committed after the first, that lists an item the
/// repository does not hold. Returns its path and the commit that holds it.
fn absent_item_baseline(fixture: &Fixture) -> (&'static str, String) {
    let baseline = "---\nid: BL-2\ntitle: Baseline two\ntype: Baseline\nname: Juniper service\nitems:\n  \
         - path: spec/a.md\n    kind: spec_artifact\n  - path: soup/s.md\n    kind: soup_record\n  \
         - path: evidence/missing.txt\n    kind: evidence\n---\n\nThe baseline.\n";
    let commit = commit_files(&fixture.repo(), &[("absent.md", baseline)], "absent item");
    ("absent.md", commit)
}

fn skill_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("engineering_assurance/skills/assurance-onboarding")
        .canonicalize()
        .expect("canonical skill")
}

fn base(fixture: &Fixture, run: &str, operation: &str) -> Value {
    json!({
        "protocol": REQUEST_PROTOCOL,
        "operation": operation,
        "state_dir": fixture.state(),
        "skill_root": skill_root(),
        "ix_flow_executable": "ix-flow",
        "run_id": run,
        "decision_owner": OWNER,
        "repository": fixture.repo(),
        "baseline_path": BASELINE_PATH,
        "baseline_commit": fixture.baseline,
        "snapshot_path": SNAPSHOT_PATH,
    })
}

fn present_request(fixture: &Fixture, run: &str) -> Value {
    let mut request = base(fixture, run, "present");
    request["release_id"] = json!("REL-1");
    request["record_path"] = json!(RECORD_PATH);
    request
}

fn decide_request(fixture: &Fixture, run: &str, choice: Option<&str>) -> Value {
    let mut request = base(fixture, run, "decide");
    if let Some(choice) = choice {
        request["choice"] = json!(choice);
    }
    if choice == Some("release") {
        request["meaning"] = json!("approved for release");
    }
    request
}

fn command(subcommand: &str, request: &Value) -> (Output, Value) {
    // `workflow-host` reads its request from stdin without a flag.
    let arguments: &[&str] = if subcommand == "workflow-host" {
        &[subcommand]
    } else {
        &[subcommand, "--request", "-"]
    };
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the Cargo-built CLI must start");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(request.to_string().as_bytes())
        .expect("request");
    let output = child.wait_with_output().expect("output");
    let value = serde_json::from_slice(&output.stdout).expect("one JSON line");
    (output, value)
}

fn ok(request: &Value) -> Value {
    let (output, value) = command("release-decision", request);
    assert_eq!(output.status.code(), Some(0), "{value}");
    assert_eq!(value["protocol"], RESULT_PROTOCOL);
    value
}

fn refused(request: &Value, code: &str) -> Value {
    let (output, value) = command("release-decision", request);
    assert_eq!(output.status.code(), Some(2), "{value}");
    assert_eq!(value["code"], code, "{value}");
    assert!(value.get("release").is_none() && value.get("decision").is_none());
    value
}

fn ix_flow(fixture: &Fixture, arguments: &[&str]) -> Value {
    let output = Command::new("ix-flow")
        .args(arguments)
        .args([
            "--state-dir",
            fixture.state().to_str().expect("utf-8"),
            "--json",
        ])
        .output()
        .expect("ix-flow must be installed");
    serde_json::from_slice(&output.stdout).expect("ix-flow JSON")
}

fn phase(fixture: &Fixture, run: &str) -> String {
    ix_flow(fixture, &["status", run])["data"]["phase"]
        .as_str()
        .expect("phase")
        .to_owned()
}

fn acknowledgements(fixture: &Fixture, run: &str) -> usize {
    ix_flow(fixture, &["status", run])["data"]["events"]
        .as_array()
        .expect("events")
        .iter()
        .filter(|event| event["kind"] == "gate.acknowledged")
        .count()
}

fn tree_snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_owned()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).expect("readable").flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let bytes = fs::read(&path).expect("readable");
            (path.strip_prefix(root).expect("relative").to_owned(), bytes)
        })
        .collect()
}

fn refs(fixture: &Fixture) -> String {
    git(&fixture.repo(), &["for-each-ref"])
}

const EVIDENCE: &str = r#"{"requirements":[{"id":"FR-1","criteria":[
  {"id":"FR-1-AC-1","evidence_status":"bound"},
  {"id":"FR-1-AC-2","evidence_status":"stale"},
  {"id":"FR-1-AC-3","evidence_status":"suspect"},
  {"id":"FR-1-AC-4","evidence_status":"undischarged"},
  {"id":"FR-1-AC-5","evidence_status":"no run evidence"}]}]}"#;
const SOUP: &str = r#"{"rows":[{"purl":"pkg:cargo/serde@1.0.0","id":"SOUP-1","unresolved_advisory":true,"policy_status":"policy findings not assessed: licence-policy not run"}],"scans":[{"tool":"grype","state":"not run"}]}"#;
const ANOMALIES: &str =
    r#"{"anomalies":[{"id":"ANOM-1","status":"open"},{"id":"ANOM-2","status":null}]}"#;

/// A run presented with every input supplied.
fn present_all(fixture: &Fixture, run: &str, anomalies: &str) -> Value {
    let mut request = present_request(fixture, run);
    request["evidence_status"] = json!(fixture.input("evidence.json", EVIDENCE));
    request["soup_report"] = json!(fixture.input("soup.json", SOUP));
    request["anomalies"] = json!(fixture.input("anomalies.json", anomalies));
    request["snapshot_file"] = json!(fixture.input("snapshot.json", SNAPSHOT_TEXT));
    ok(&request)
}

#[test]
#[trace("TC-450", "FR-048-AC-1")]
fn tc_450_every_terminal_transition_is_hitl_and_an_automatic_override_fails_closed() {
    let bytes = fs::read(skill_root().join("workflows/release-decision/def.yaml"))
        .expect("the canonical definition must exist");
    let definition: Value = yaml_serde::from_slice(&bytes).expect("definition");
    let terminal: Vec<&str> = definition["phases"]
        .as_array()
        .expect("phases")
        .iter()
        .filter(|phase| phase["terminal"] == true)
        .map(|phase| phase["name"].as_str().expect("name"))
        .collect();
    assert_eq!(terminal, ["released", "rejected"]);
    let into_terminal: Vec<&Value> = definition["transitions"]
        .as_array()
        .expect("transitions")
        .iter()
        .filter(|t| terminal.contains(&t["to"].as_str().expect("to")))
        .collect();
    assert_eq!(into_terminal.len(), 2);
    for transition in into_terminal {
        assert_eq!(transition["defaultGate"], "hitl", "{transition}");
    }

    let fixture = fixture();
    let created = ix_flow(
        &fixture,
        &[
            "run",
            "release-decision",
            "--path",
            skill_root().to_str().expect("utf-8"),
            "--id",
            "override-run",
            "--gate",
            "decision_ready->released=auto",
        ],
    );
    assert_eq!(created["ok"], true, "{created}");
    let before = tree_snapshot(&fixture.state());
    refused(
        &present_request(&fixture, "override-run"),
        "workflow_binding_invalid",
    );
    assert_eq!(tree_snapshot(&fixture.state()), before);
    assert_eq!(phase(&fixture, "override-run"), "capture");
}

#[test]
#[trace("TC-451", "FR-048-AC-2")]
fn tc_451_with_no_owner_choice_the_run_stays_at_the_gate_with_no_document_and_no_tag() {
    let fixture = fixture();
    let refs_before = refs(&fixture);
    let presented = present_all(&fixture, "run-gate", ANOMALIES);
    assert_eq!(presented["kind"], "presentation");
    assert_eq!(presented["snapshot"]["phase"], "decision_ready");

    let waiting = ok(&decide_request(&fixture, "run-gate", None));
    assert_eq!(waiting["kind"], "waiting");
    assert_eq!(waiting["snapshot"]["phase"], "decision_ready");
    assert_eq!(waiting["snapshot"]["open_gates"], json!([]));
    assert!(waiting.get("release").is_none() && waiting.get("decision").is_none());
    assert_eq!(acknowledgements(&fixture, "run-gate"), 0);
    assert_eq!(phase(&fixture, "run-gate"), "decision_ready");
    assert!(git(&fixture.repo(), &["tag", "-l"]).is_empty());
    assert_eq!(refs(&fixture), refs_before);
    assert!(!fixture.repo().join(RECORD_PATH).exists());
}

#[test]
#[trace("TC-452", "FR-048-AC-3")]
fn tc_452_the_presentation_shows_each_source_as_data_or_not_provided() {
    let fixture = fixture();
    let (absent_path, absent_commit) = absent_item_baseline(&fixture);
    let fixture = Fixture {
        baseline: absent_commit,
        ..fixture
    };
    let mut request = present_request(&fixture, "run-all");
    request["baseline_path"] = json!(absent_path);
    request["evidence_status"] = json!(fixture.input("evidence.json", EVIDENCE));
    request["soup_report"] = json!(fixture.input("soup.json", SOUP));
    request["anomalies"] = json!(fixture.input("anomalies.json", ANOMALIES));
    let all = ok(&request)["presentation"].clone();
    assert_eq!(all["baseline"]["commit"], fixture.baseline);
    assert_eq!(all["baseline"]["name"], "Juniper service");
    assert_eq!(
        all["baseline"]["items"],
        json!([
            {"path": "spec/a.md", "kind": "spec_artifact", "present": true},
            {"path": "soup/s.md", "kind": "soup_record", "present": true},
            {"path": "evidence/missing.txt", "kind": "evidence", "present": false},
        ])
    );
    assert_eq!(all["evidence_status"]["state"], "supplied");
    let statuses: Vec<&str> = all["evidence_status"]["criteria"]
        .as_array()
        .expect("criteria")
        .iter()
        .map(|c| c["evidence_status"].as_str().expect("status"))
        .collect();
    assert_eq!(
        statuses,
        [
            "bound",
            "stale",
            "suspect",
            "undischarged",
            "no run evidence"
        ]
    );
    assert_eq!(
        all["soup"]["baseline_records"],
        json!([{"path": "soup/s.md", "present": true}])
    );
    // The supplied reports are carried verbatim, so a member the presentation
    // does not select is still shown.
    assert_eq!(
        all["evidence_status"]["document"],
        serde_json::from_str::<Value>(EVIDENCE).expect("fixture")
    );
    assert_eq!(
        all["soup"]["report"]["document"],
        serde_json::from_str::<Value>(SOUP).expect("fixture")
    );
    assert_eq!(
        all["anomalies"]["document"],
        serde_json::from_str::<Value>(ANOMALIES).expect("fixture")
    );
    assert_eq!(
        all["anomalies"]["items"],
        json!([{"id": "ANOM-1", "status": "open"}, {"id": "ANOM-2", "status": null}])
    );

    let empty = present_all(&fixture, "run-empty", r#"{"anomalies":[]}"#)["presentation"].clone();
    assert_eq!(empty["anomalies"]["state"], "supplied");
    assert_eq!(empty["anomalies"]["items"], json!([]));

    let none = ok(&present_request(&fixture, "run-none"))["presentation"].clone();
    for member in [
        &none["evidence_status"],
        &none["soup"]["report"],
        &none["anomalies"],
        &none["snapshot"]["file"],
    ] {
        assert_eq!(member, &json!({"state": "not provided"}));
    }
}

#[test]
#[trace("TC-453", "FR-048-AC-4")]
fn tc_453_a_release_names_each_unresolved_anomaly_or_writes_no_document() {
    let fixture = fixture();
    present_all(&fixture, "run-anom", ANOMALIES);
    let mut request = decide_request(&fixture, "run-anom", Some("release"));
    let value = refused(&request, "anomaly_not_accepted");
    assert!(
        value["message"]
            .as_str()
            .expect("message")
            .contains("ANOM-1")
    );
    request["accepted_anomalies"] = json!(["ANOM-1"]);
    let value = refused(&request, "anomaly_not_accepted");
    assert!(
        value["message"]
            .as_str()
            .expect("message")
            .contains("ANOM-2")
    );
    assert_eq!(phase(&fixture, "run-anom"), "decision_ready");
    assert_eq!(acknowledgements(&fixture, "run-anom"), 0);

    request["accepted_anomalies"] = json!(["ANOM-2", "ANOM-1"]);
    let decided = ok(&request);
    let document = decided["release"]["document"].as_str().expect("document");
    assert!(
        document.contains("accepted_anomalies:\n  - \"ANOM-2\"\n  - \"ANOM-1\"\n---"),
        "{document}"
    );
}

#[test]
#[trace("TC-454", "FR-048-AC-5")]
fn tc_454_a_reject_retains_the_rejection_and_writes_no_document_and_no_tag() {
    let fixture = fixture();
    present_all(&fixture, "run-reject", ANOMALIES);
    let refs_before = refs(&fixture);
    let decided = ok(&decide_request(&fixture, "run-reject", Some("reject")));
    assert_eq!(decided["decision"]["outcome"], "rejected");
    assert_eq!(decided["decision"]["choice"], "reject");
    assert!(decided.get("release").is_none());
    assert_eq!(phase(&fixture, "run-reject"), "rejected");
    assert!(git(&fixture.repo(), &["tag", "-l"]).is_empty());
    assert_eq!(refs(&fixture), refs_before);
    assert!(!fixture.repo().join(RECORD_PATH).exists());
}

#[test]
#[trace("TC-455", "FR-048-AC-6")]
fn tc_455_a_release_emits_one_terminal_event_and_runs_no_git_write() {
    let fixture = fixture();
    present_all(&fixture, "run-event", r#"{"anomalies":[]}"#);
    let refs_before = refs(&fixture);
    let head_before = git(&fixture.repo(), &["rev-parse", "HEAD"]);
    let request = decide_request(&fixture, "run-event", Some("release"));
    let decided = ok(&request);
    let event = &decided["decision"];
    assert_eq!(event["run_id"], "run-event");
    assert_eq!(event["workflow"], "release-decision");
    assert_eq!(event["workflow_version"], "0.1.0");
    assert_eq!(event["owner"], OWNER);
    assert_eq!(event["choice"], "release");
    assert_eq!(event["outcome"], "released");
    assert!(
        event["timestamp"]
            .as_str()
            .expect("timestamp")
            .ends_with('Z')
    );
    assert_eq!(acknowledgements(&fixture, "run-event"), 1);
    // A repeated choice returns the same event and acknowledges nothing more.
    assert_eq!(ok(&request)["decision"], *event);
    assert_eq!(acknowledgements(&fixture, "run-event"), 1);

    assert_eq!(git(&fixture.repo(), &["rev-parse", "HEAD"]), head_before);
    assert_eq!(refs(&fixture), refs_before);
    assert!(git(&fixture.repo(), &["tag", "-l"]).is_empty());
    assert!(git(&fixture.repo(), &["status", "--porcelain"]).is_empty());
}

#[test]
#[trace("TC-456", "FR-048-AC-7")]
fn tc_456_no_evidence_state_an_empty_list_or_a_failed_read_produces_a_release() {
    let fixture = fixture();
    let all_bound = fixture.input(
        "bound.json",
        r#"{"requirements":[{"id":"FR-1","criteria":[{"id":"FR-1-AC-1","evidence_status":"bound"}]}]}"#,
    );
    let mut request = present_request(&fixture, "run-bound");
    request["evidence_status"] = json!(all_bound);
    request["anomalies"] = json!(fixture.input("none.json", r#"{"anomalies":[]}"#));
    ok(&request);
    let waiting = ok(&decide_request(&fixture, "run-bound", None));
    assert_eq!(waiting["snapshot"]["phase"], "decision_ready");
    assert_eq!(acknowledgements(&fixture, "run-bound"), 0);

    let mut failed = present_request(&fixture, "run-failed");
    failed["evidence_status"] = json!(fixture.root().join("absent.json"));
    refused(&failed, "release_input_unreadable");
    assert_eq!(phase(&fixture, "run-failed"), "capture");
    refused(
        &decide_request(&fixture, "run-failed", Some("release")),
        "release_presentation_missing",
    );
    assert_eq!(acknowledgements(&fixture, "run-failed"), 0);
}

#[test]
#[trace("TC-457", "FR-048-AC-8")]
fn tc_457_a_reused_run_id_is_refused_unchanged_and_an_interrupted_run_resumes() {
    let fixture = fixture();
    present_all(&fixture, "run-id", ANOMALIES);
    let before = tree_snapshot(&fixture.state());
    let other_commit = commit_files(&fixture.repo(), &[("later.md", "later\n")], "later");
    for (member, value) in [
        ("baseline_commit", json!(other_commit)),
        ("baseline_path", json!("other.md")),
        ("snapshot_path", json!("release/other.json")),
        ("decision_owner", json!("bob@example.test")),
        ("repository", json!(fixture.root().join("elsewhere"))),
    ] {
        let mut request = decide_request(&fixture, "run-id", Some("reject"));
        request[member] = value;
        refused(&request, "workflow_binding_invalid");
        assert_eq!(tree_snapshot(&fixture.state()), before, "{member}");
    }
    assert_eq!(phase(&fixture, "run-id"), "decision_ready");

    // Interrupted before the presentation: the run exists at `capture`, and the
    // next call completes it without a second binding.
    let mut failed = present_request(&fixture, "run-resume");
    failed["soup_report"] = json!(fixture.root().join("absent.json"));
    refused(&failed, "release_input_unreadable");
    assert_eq!(phase(&fixture, "run-resume"), "capture");
    let resumed = ok(&present_request(&fixture, "run-resume"));
    assert_eq!(resumed["snapshot"]["phase"], "decision_ready");
    let status = ix_flow(&fixture, &["status", "run-resume"]);
    assert_eq!(
        status["data"]["items"]["run_binding"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    // Resuming a presented run returns the retained presentation unchanged.
    assert_eq!(
        ok(&present_request(&fixture, "run-resume"))["presentation"],
        resumed["presentation"]
    );
}

#[test]
#[trace("TC-458", "FR-048-AC-9")]
fn tc_458_an_unknown_accepted_id_or_an_unreadable_input_stops_before_the_gate() {
    let fixture = fixture();
    present_all(&fixture, "run-unknown", ANOMALIES);
    let mut request = decide_request(&fixture, "run-unknown", Some("release"));
    request["accepted_anomalies"] = json!(["ANOM-1", "ANOM-2", "ANOM-9"]);
    let value = refused(&request, "accepted_anomaly_unknown");
    assert!(
        value["message"]
            .as_str()
            .expect("message")
            .contains("ANOM-9")
    );
    assert_eq!(phase(&fixture, "run-unknown"), "decision_ready");
    assert_eq!(acknowledgements(&fixture, "run-unknown"), 0);

    let cases = [
        ("evidence_status", "{\"requirements\":[{\"id\":\"FR-1\"}]}"),
        (
            "evidence_status",
            "{\"requirements\":[{\"criteria\":[{\"id\":\"x\",\"evidence_status\":\"none\"}]}]}",
        ),
        ("soup_report", "{\"rows\":[{\"id\":\"SOUP-1\"}]}"),
        ("anomalies", "{\"anomalies\":[{\"id\":\"A\"}]}"),
        ("anomalies", "[1]"),
        ("anomalies", "not json"),
    ];
    for (index, (member, content)) in cases.into_iter().enumerate() {
        let run = format!("run-bad-{index}");
        let path = fixture.input(&format!("bad-{index}.json"), content);
        let mut request = present_request(&fixture, &run);
        request[member] = json!(path);
        let value = refused(&request, "release_input_unreadable");
        assert!(
            value["message"].as_str().expect("message").contains(&path),
            "{value}"
        );
        assert_eq!(phase(&fixture, &run), "capture", "{member}: {content}");
    }
}

/// Mints a release commit's worth of files for the document the workflow emitted.
fn commit_release(fixture: &Fixture, release: &Value, snapshot: &str) -> String {
    let record_path = release["record_path"].as_str().expect("record_path");
    let document = release["document"].as_str().expect("document");
    commit_files(
        &fixture.repo(),
        &[(record_path, document), (SNAPSHOT_PATH, snapshot)],
        "release",
    )
}

#[test]
#[trace("TC-459", "FR-048-AC-10")]
fn tc_459_the_emitted_record_and_message_yield_released_after_the_owner_commits_and_signs() {
    let fixture = fixture();
    let mut request = present_request(&fixture, "run-e2e");
    request["snapshot_file"] = json!(fixture.input("snapshot.json", SNAPSHOT_TEXT));
    request["anomalies"] = json!(fixture.input("anomalies.json", ANOMALIES));
    ok(&request);
    let head_before = git(&fixture.repo(), &["rev-parse", "HEAD"]);

    let mut decide = decide_request(&fixture, "run-e2e", Some("release"));
    decide["accepted_anomalies"] = json!(["ANOM-1", "ANOM-2"]);
    let release = ok(&decide)["release"].clone();
    assert_eq!(git(&fixture.repo(), &["rev-parse", "HEAD"]), head_before);
    assert_eq!(release["record_path"], RECORD_PATH);

    // The owner commits the record and the snapshot as one child of the
    // baseline commit, completes the digest line and signs with `git tag -s`.
    let commit = commit_release(&fixture, &release, SNAPSHOT_TEXT);
    let id = git(
        &fixture.repo(),
        &["rev-parse", &format!("{commit}:{RECORD_PATH}")],
    );
    let message = release["tag_message"]
        .as_str()
        .expect("tag message")
        .replace(
            "Approved-Record-Digest: git-blob:",
            &format!("Approved-Record-Digest: git-blob:{id}"),
        );
    let public = fixture.root().join("a.pub");
    git(
        &fixture.repo(),
        &[
            "-c",
            "gpg.format=ssh",
            "-c",
            &format!("user.signingkey={}", public.display()),
            "tag",
            "-s",
            "-m",
            &message,
            "v1",
            &commit,
        ],
    );
    let (output, verdict) = command(
        "release-verify",
        &json!({
            "protocol": "engineering-assurance.release-verify-request/v1",
            "repository": fixture.repo(),
            "tag": "v1",
            "record_path": RECORD_PATH,
            "allowed_signers": fixture.root().join("allowed_signers"),
        }),
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(verdict["status"], "released", "{verdict}");
    assert_eq!(verdict["approver"], OWNER);
    assert_eq!(verdict["meaning"], json!(["approved for release"]));
    assert_eq!(verdict["accepted_anomalies"], json!(["ANOM-1", "ANOM-2"]));
    assert_eq!(verdict["release"]["snapshot_path"], SNAPSHOT_PATH);
}

#[test]
#[trace("TC-460", "FR-048-AC-11")]
fn tc_460_a_snapshot_path_at_the_baseline_or_breaking_the_path_rules_is_refused() {
    let fixture = fixture();
    for (index, path) in [BASELINE_PATH, "spec/a.md", "spec"].into_iter().enumerate() {
        let run = format!("run-present-{index}");
        let mut request = present_request(&fixture, &run);
        request["snapshot_path"] = json!(path);
        ok(&request);
        let mut decide = decide_request(&fixture, &run, Some("release"));
        decide["snapshot_path"] = json!(path);
        refused(&decide, "snapshot_path_present_at_baseline");
        assert_eq!(phase(&fixture, &run), "decision_ready", "{path}");
        assert_eq!(acknowledgements(&fixture, &run), 0, "{path}");
    }
    for (index, path) in [
        "/abs/snapshot.json",
        "a/../b.json",
        "a/*.json",
        "a/\u{1}b",
        " lead/x.json",
    ]
    .into_iter()
    .enumerate()
    {
        let run = format!("run-invalid-{index}");
        let mut request = present_request(&fixture, &run);
        request["snapshot_path"] = json!(path);
        ok(&request);
        let mut decide = decide_request(&fixture, &run, Some("release"));
        decide["snapshot_path"] = json!(path);
        refused(&decide, "release_snapshot_path_invalid");
        assert_eq!(phase(&fixture, &run), "decision_ready", "{path:?}");
        assert_eq!(acknowledgements(&fixture, &run), 0, "{path:?}");
    }
}

#[test]
#[trace("TC-461", "FR-048-AC-12")]
fn tc_461_a_supplied_snapshot_is_shown_verbatim_with_its_path_and_an_unsupplied_one_is_not_provided()
 {
    let fixture = fixture();
    let shown = present_all(&fixture, "run-snap", ANOMALIES)["presentation"]["snapshot"].clone();
    assert_eq!(shown["snapshot_path"], SNAPSHOT_PATH);
    assert_eq!(shown["file"]["state"], "supplied");
    assert_eq!(shown["file"]["text"], SNAPSHOT_TEXT);
    let retained = ix_flow(&fixture, &["status", "run-snap"])["data"]["items"]["release_presentation"][0]["snapshot"]["file"]["text"].clone();
    assert_eq!(retained, SNAPSHOT_TEXT);

    let none = ok(&present_request(&fixture, "run-nosnap"))["presentation"]["snapshot"].clone();
    assert_eq!(none["snapshot_path"], SNAPSHOT_PATH);
    assert_eq!(none["file"], json!({"state": "not provided"}));
}

#[test]
#[trace("TC-462", "FR-048-AC-13")]
fn tc_462_the_choice_cannot_change_after_the_gate_is_acknowledged() {
    let fixture = fixture();
    present_all(&fixture, "run-fixed", ANOMALIES);
    let mut request = decide_request(&fixture, "run-fixed", Some("release"));
    request["accepted_anomalies"] = json!(["ANOM-1", "ANOM-2"]);
    let first = ok(&request);
    let before = tree_snapshot(&fixture.state());

    // The same choice returns the stored document and message unchanged.
    let repeated = ok(&request);
    assert_eq!(repeated["release"], first["release"]);
    assert_eq!(repeated["decision"], first["decision"]);

    let mut other_meaning = request.clone();
    other_meaning["meaning"] = json!("a different meaning");
    refused(&other_meaning, "release_decision_changed");
    let mut other_order = request.clone();
    other_order["accepted_anomalies"] = json!(["ANOM-2", "ANOM-1"]);
    refused(&other_order, "release_decision_changed");
    assert_eq!(tree_snapshot(&fixture.state()), before);
}

#[test]
#[trace("TC-463", "FR-048-AC-14")]
fn tc_463_the_generic_workflow_host_cannot_reach_released_without_a_presentation() {
    let fixture = fixture();
    let binding = json!({
        "run_id": "run-generic",
        "repository_id": fixture.repo(),
        "workflow": "release-decision",
        "workflow_version": "0.1.0",
        "decision_boundary": "generic",
        "decision_owner": OWNER,
    });
    let host = |operation: &str, choice: Option<&str>| {
        let mut request = json!({
            "protocol": "engineering-assurance.workflow-host/v1",
            "operation": operation,
            "state_dir": fixture.state(),
            "skill_root": skill_root(),
            "ix_flow_executable": "ix-flow",
            "binding": binding,
        });
        if let Some(choice) = choice {
            request["choice"] = json!(choice);
        }
        command("workflow-host", &request)
    };
    let (output, _) = host("start_or_resume", None);
    assert_eq!(output.status.code(), Some(0));
    let answers = r#"{"release_id":"REL-1","baseline_path":"b.md","baseline_commit":"c","snapshot_path":"s","record_path":"r","owner":"o"}"#;
    ix_flow(
        &fixture,
        &[
            "record-answers",
            "run-generic",
            "release",
            "--answers",
            answers,
        ],
    );
    assert_eq!(
        ix_flow(&fixture, &["advance", "run-generic", "evidence_ready"])["ok"],
        true
    );
    let blocked = ix_flow(&fixture, &["advance", "run-generic", "decision_ready"]);
    assert_eq!(blocked["state"], "invariant_failed", "{blocked}");
    assert_eq!(
        blocked["error"]["details"]["invariantCode"],
        "release_presentation_missing"
    );

    let (output, value) = host("decide", Some("release"));
    assert_eq!(output.status.code(), Some(2), "{value}");
    assert_eq!(phase(&fixture, "run-generic"), "evidence_ready");
    assert_eq!(acknowledgements(&fixture, "run-generic"), 0);
}

#[test]
#[trace("TC-464", "FR-048-AC-15")]
fn tc_464_the_escaped_presentation_is_bounded_so_no_input_can_wedge_a_run() {
    let fixture = fixture();
    // 700 KiB of U+0001 is within the file limit and 4.2 MiB once escaped.
    let heavy = "\u{1}".repeat(700 * 1024);
    let mut request = present_request(&fixture, "run-heavy");
    request["snapshot_file"] = json!(fixture.input("heavy.txt", &heavy));
    refused(&request, "release_presentation_too_large");
    assert_eq!(phase(&fixture, "run-heavy"), "capture");
    // A file over the raw limit is refused naming the file.
    let huge = fixture.input("huge.txt", &"\u{1}".repeat(4 * 1024 * 1024));
    request["snapshot_file"] = json!(huge);
    let value = refused(&request, "release_input_unreadable");
    assert!(value["message"].as_str().expect("message").contains(&huge));

    // Just under the bound the run is presented and still decidable.
    let allowed = "\u{1}".repeat(480 * 1024);
    let mut request = present_request(&fixture, "run-near");
    request["snapshot_file"] = json!(fixture.input("near.txt", &allowed));
    ok(&request);
    let decided = ok(&decide_request(&fixture, "run-near", Some("reject")));
    assert_eq!(decided["decision"]["outcome"], "rejected");
}

#[test]
#[trace("TC-465", "FR-048-AC-15")]
fn tc_465_a_fifo_or_non_utf8_input_is_unreadable_and_never_hangs() {
    let fixture = fixture();
    let fifo = fixture.root().join("fifo.json");
    run_program("mkfifo", fixture.root(), &[fifo.to_str().expect("utf-8")]);
    let mut request = present_request(&fixture, "run-fifo");
    request["evidence_status"] = json!(fifo);
    let value = refused(&request, "release_input_unreadable");
    assert!(
        value["message"]
            .as_str()
            .expect("message")
            .contains("fifo.json")
    );

    let binary = fixture.root().join("snapshot.bin");
    fs::write(&binary, [0xff, 0xfe, 0x00]).expect("binary file");
    let mut request = present_request(&fixture, "run-binary");
    request["snapshot_file"] = json!(binary);
    let value = refused(&request, "release_input_unreadable");
    assert!(
        value["message"]
            .as_str()
            .expect("message")
            .contains("snapshot.bin")
    );
    assert_eq!(phase(&fixture, "run-binary"), "capture");
}

#[test]
#[trace("TC-466", "FR-048-AC-15")]
fn tc_466_deciding_an_unknown_run_makes_no_run() {
    let fixture = fixture();
    let before = tree_snapshot(&fixture.state());
    for choice in [None, Some("reject"), Some("release")] {
        refused(
            &decide_request(&fixture, "run-unknown-id", choice),
            "workflow_run_not_found",
        );
    }
    assert_eq!(tree_snapshot(&fixture.state()), before);
}

#[test]
#[trace("TC-467", "FR-048-AC-16")]
fn tc_467_a_record_path_at_the_baseline_or_an_absent_baseline_item_refuses_the_release() {
    let fixture = fixture();
    let mut request = present_request(&fixture, "run-record");
    request["record_path"] = json!("spec/a.md");
    ok(&request);
    refused(
        &decide_request(&fixture, "run-record", Some("release")),
        "record_path_present_at_baseline",
    );
    assert_eq!(acknowledgements(&fixture, "run-record"), 0);

    let (absent_path, absent_commit) = absent_item_baseline(&fixture);
    let fixture = Fixture {
        baseline: absent_commit,
        ..fixture
    };
    let mut request = present_request(&fixture, "run-absent");
    request["baseline_path"] = json!(absent_path);
    ok(&request);
    let mut decide = decide_request(&fixture, "run-absent", Some("release"));
    decide["baseline_path"] = json!(absent_path);
    let value = refused(&decide, "baseline_item_absent");
    assert!(
        value["message"]
            .as_str()
            .expect("message")
            .contains("evidence/missing.txt")
    );
    assert_eq!(acknowledgements(&fixture, "run-absent"), 0);
    // The person can still reject a candidate whose Baseline is incomplete.
    let mut reject = decide_request(&fixture, "run-absent", Some("reject"));
    reject["baseline_path"] = json!(absent_path);
    assert_eq!(ok(&reject)["decision"]["outcome"], "rejected");
}

#[test]
#[trace("TC-468", "FR-048-AC-8")]
fn tc_468_an_interrupted_run_resumes_from_evidence_ready() {
    let fixture = fixture();
    let mut failed = present_request(&fixture, "run-mid");
    failed["soup_report"] = json!(fixture.root().join("absent.json"));
    refused(&failed, "release_input_unreadable");
    let answers = json!({
        "release_id": "REL-1",
        "baseline_path": BASELINE_PATH,
        "baseline_commit": fixture.baseline,
        "snapshot_path": SNAPSHOT_PATH,
        "record_path": RECORD_PATH,
        "owner": OWNER,
    })
    .to_string();
    ix_flow(
        &fixture,
        &[
            "record-answers",
            "run-mid",
            "release",
            "--answers",
            &answers,
        ],
    );
    ix_flow(&fixture, &["advance", "run-mid", "evidence_ready"]);
    assert_eq!(phase(&fixture, "run-mid"), "evidence_ready");

    let resumed = ok(&present_request(&fixture, "run-mid"));
    assert_eq!(resumed["snapshot"]["phase"], "decision_ready");
    let status = ix_flow(&fixture, &["status", "run-mid"]);
    assert_eq!(
        status["data"]["items"]["release_presentation"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        status["data"]["items"]["release_request"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
}

#[test]
#[trace("TC-469", "FR-048-AC-17")]
fn tc_469_an_unreadable_anomaly_document_is_shown_and_must_be_named_as_accepted() {
    let fixture = fixture();
    let report = r#"{"anomalies":[],"unreadable":[{"path":"spec/anomalies/AN-9.md","code":"anomaly.no-record"}]}"#;
    let presented = present_all(&fixture, "run-unreadable", report);
    assert_eq!(
        presented["presentation"]["anomalies"]["unreadable"],
        json!([{"path": "spec/anomalies/AN-9.md", "code": "anomaly.no-record"}])
    );
    assert_eq!(
        presented["presentation"]["anomalies"]["document"],
        serde_json::from_str::<Value>(report).expect("fixture")
    );
    let mut request = decide_request(&fixture, "run-unreadable", Some("release"));
    let value = refused(&request, "anomaly_not_accepted");
    assert!(
        value["message"]
            .as_str()
            .expect("message")
            .contains("AN-9.md")
    );
    request["accepted_anomalies"] = json!(["spec/anomalies/AN-9.md"]);
    let decided = ok(&request);
    assert!(
        decided["release"]["document"]
            .as_str()
            .expect("document")
            .contains("accepted_anomalies:\n  - \"spec/anomalies/AN-9.md\"\n")
    );
}
