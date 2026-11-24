// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Process-boundary tests for `signature-verify` on gitsign-signed objects
//! (FR-040). `gitsign` is not installed on the build machines, so every test
//! but the ignored live one runs a stand-in executable named `gitsign` that
//! records its arguments, working directory and environment and exits with a
//! scripted status. The CMS signature blocks are forged with `git hash-object`
//! and the identities are fictional.

use std::{
    fs,
    io::Write,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use ix_trace_rs::trace;
use serde_json::{Value, json};

const IDENTITY: &str = "alice@example.test";
const ISSUER: &str = concat!("https:", "//issuer.example.test");
const OTHER_ISSUER: &str = concat!("https:", "//other.example.test");
const PRINCIPAL: &str = "Alice Example";
/// The committed identity registry every request names: branch `registry`, file
/// `registry.json`.
const REGISTRY: &str = "registry:registry.json";
const CMS_BLOCK: &str = "-----BEGIN SIGNED MESSAGE-----\nAAAA\n-----END SIGNED MESSAGE-----\n";

/// Records every call as `args_N`, `cwd_N` and `env_N` (the environment the
/// process was started with) in its own directory, then exits with the status
/// scripted for the identity and issuer flavor of the call.
const STAND_IN: &str = r#"#!/bin/sh
d=${0%/*}
n=0
[ -e "$d/count" ] && read -r n < "$d/count"
n=$((n + 1))
echo "$n" > "$d/count"
for a in "$@"; do printf '%s\n' "$a"; done > "$d/args_$n"
pwd > "$d/cwd_$n"
/usr/bin/tr '\0' '\n' < /proc/$$/environ > "$d/env_$n"
id=exact
iss=exact
for a in "$@"; do
  case "$a" in
    --certificate-identity-regexp) id=any ;;
    --certificate-oidc-issuer-regexp) iss=any ;;
  esac
done
[ -e "$d/flood" ] && /usr/bin/head -c 9000000 /dev/zero
status=0
if [ -e "$d/exit_${id}_${iss}" ]; then read -r status < "$d/exit_${id}_${iss}"
elif [ -e "$d/exit_default" ]; then read -r status < "$d/exit_default"; fi
exit "$status"
"#;

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn repo(&self) -> PathBuf {
        self.root().join("repo")
    }

    fn bin(&self) -> PathBuf {
        self.root().join("bin")
    }

    fn home(&self) -> PathBuf {
        self.root().join("home")
    }

    fn allowed(&self) -> PathBuf {
        self.root().join("allowed_signers")
    }

    /// Scripts the stand-in's exit status for one flavor, such as `exact_exact`.
    fn script(&self, flavor: &str, status: i32) {
        fs::write(
            self.bin().join(format!("exit_{flavor}")),
            format!("{status}\n"),
        )
        .expect("script");
    }

    fn calls(&self) -> usize {
        fs::read_to_string(self.bin().join("count"))
            .map_or(0, |text| text.trim().parse().expect("count"))
    }

    fn lines(&self, name: &str, call: usize) -> Vec<String> {
        fs::read_to_string(self.bin().join(format!("{name}_{call}")))
            .expect("record")
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

fn run_program(
    directory: &Path,
    program: &str,
    arguments: &[&str],
    input: Option<&[u8]>,
) -> Vec<u8> {
    let mut child = Command::new(program)
        .args(arguments)
        .current_dir(directory)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env_remove("GIT_DIR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("program must start");
    let mut stdin = child.stdin.take().expect("stdin");
    if let Some(input) = input {
        stdin.write_all(input).expect("input");
    }
    drop(stdin);
    let output = child.wait_with_output().expect("output");
    assert!(
        output.status.success(),
        "{program} {arguments:?}: {output:?}"
    );
    output.stdout
}

fn git(repo: &Path, arguments: &[&str]) -> String {
    let mut all = vec!["-c", "user.name=t", "-c", "user.email=t@example.test"];
    all.extend_from_slice(arguments);
    String::from_utf8(run_program(repo, "git", &all, None)).expect("utf-8")
}

fn which(program: &str) -> PathBuf {
    std::env::split_paths(&std::env::var_os("PATH").expect("PATH"))
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
        .expect("program on PATH")
}

/// A repository with one plain commit, an empty allowed-signers file and a
/// `bin` directory holding `git` and the stand-in `gitsign`.
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    fs::create_dir(root.join("home")).expect("home");
    fs::write(root.join("allowed_signers"), "").expect("allowed signers");
    let bin = root.join("bin");
    fs::create_dir(&bin).expect("bin");
    // A `git` that logs its arguments, then runs the real one.
    let wrapper = format!(
        "#!/bin/sh\necho \"$*\" >> \"${{0%/*}}/gitlog\"\nexec {} \"$@\"\n",
        which("git").display()
    );
    fs::write(bin.join("git"), wrapper).expect("git wrapper");
    fs::set_permissions(bin.join("git"), fs::Permissions::from_mode(0o755)).expect("mode");
    fs::write(bin.join("gitsign"), STAND_IN).expect("stand-in");
    fs::set_permissions(bin.join("gitsign"), fs::Permissions::from_mode(0o755)).expect("mode");
    let repo = root.join("repo");
    fs::create_dir(&repo).expect("repo dir");
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "base"]);
    commit_registry(&repo, "registry", &default_registry());
    Fixture { dir }
}

fn store(repo: &Path, kind: &str, object: &str) -> String {
    store_bytes(repo, kind, object.as_bytes())
}

fn store_bytes(repo: &Path, kind: &str, object: &[u8]) -> String {
    let id = run_program(
        repo,
        "git",
        &["hash-object", "-t", kind, "-w", "--stdin"],
        Some(object),
    );
    String::from_utf8(id).expect("utf-8").trim().to_owned()
}

/// The registry document with the given `(identity, issuer, principal)` entries.
fn registry_json(entries: &[(&str, &str, &str)]) -> String {
    let entries: Vec<Value> = entries
        .iter()
        .map(|(identity, issuer, principal)| {
            json!({
                "certificate_identity": identity,
                "certificate_oidc_issuer": issuer,
                "principal": principal,
            })
        })
        .collect();
    json!({
        "protocol": "engineering-assurance.gitsign-identity-registry/v1",
        "entries": entries,
    })
    .to_string()
}

/// The registry that maps the fixture identity to [`PRINCIPAL`].
fn default_registry() -> String {
    registry_json(&[(IDENTITY, ISSUER, PRINCIPAL)])
}

/// Commits `content` as `registry.json` in a root commit that `branch` names,
/// without touching HEAD, the index or the working tree. Returns the commit id.
fn commit_registry(repo: &Path, branch: &str, content: &str) -> String {
    commit_registry_bytes(repo, branch, content.as_bytes())
}

fn commit_registry_bytes(repo: &Path, branch: &str, content: &[u8]) -> String {
    let blob = store_bytes(repo, "blob", content);
    commit_tree_listing(
        repo,
        branch,
        &format!("100644 blob {blob}\tregistry.json\n"),
    )
}

/// Commits the tree `git mktree` builds from `listing` in a root commit that
/// `branch` names. Returns the commit id.
fn commit_tree_listing(repo: &Path, branch: &str, listing: &str) -> String {
    let tree = String::from_utf8(run_program(
        repo,
        "git",
        &["mktree", "--missing"],
        Some(listing.as_bytes()),
    ))
    .expect("utf-8")
    .trim()
    .to_owned();
    let commit = git(repo, &["commit-tree", &tree, "-m", "registry"])
        .trim()
        .to_owned();
    git(
        repo,
        &["update-ref", &format!("refs/heads/{branch}"), &commit],
    );
    commit
}

/// A continued header value: every line after the first starts with a space.
fn header_block(block: &str) -> String {
    block.trim_end_matches('\n').replace('\n', "\n ")
}

/// A commit whose message is `message` and whose `gpgsig` header is `block`.
fn forged_commit(fixture: &Fixture, message: &str, block: &str) -> String {
    forged_commit_by(
        fixture,
        message,
        block,
        "t <t@example.test> 1700000000 +0000",
    )
}

/// As [`forged_commit`], with `who` as author and committer line.
fn forged_commit_by(fixture: &Fixture, message: &str, block: &str, who: &str) -> String {
    let repo = fixture.repo();
    let tree = git(&repo, &["rev-parse", "HEAD^{tree}"]).trim().to_owned();
    let parent = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let object = format!(
        "tree {tree}\nparent {parent}\nauthor {who}\ncommitter {who}\ngpgsig {}\n\n{message}\n",
        header_block(block)
    );
    let id = store(&repo, "commit", &object);
    git(&repo, &["update-ref", "refs/heads/signed", &id]);
    id
}

/// An annotated tag named `name` whose message ends in `block`.
fn forged_tag(fixture: &Fixture, name: &str, message: &str, block: &str) -> String {
    let repo = fixture.repo();
    let commit = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let object = format!(
        "object {commit}\ntype commit\ntag {name}\ntagger t <t@example.test> 1700000000 +0000\n\n{message}\n{block}"
    );
    let id = store(&repo, "tag", &object);
    git(&repo, &["update-ref", &format!("refs/tags/{name}"), &id]);
    id
}

fn request(fixture: &Fixture, revision: &str, kind: &str, gitsign: bool) -> Value {
    let mut value = json!({
        "protocol": "engineering-assurance.signature-verify-request/v1",
        "repository": fixture.repo().display().to_string(),
        "revision": revision,
        "kind": kind,
        "allowed_signers": fixture.allowed().display().to_string(),
    });
    if gitsign {
        value["gitsign"] = json!({
            "certificate_identity": IDENTITY,
            "certificate_oidc_issuer": ISSUER,
        });
        value["identity_registry"] = json!(REGISTRY);
    }
    value
}

fn run(fixture: &Fixture, body: &[u8], path: &Path) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_engineering-assurance"))
        .args(["signature-verify", "--request", "-"])
        .env_clear()
        .env("PATH", path)
        .env("HOME", fixture.home())
        .env("SIGSTORE_ROOT_FILE", "no-such-root")
        .env("GITSIGN_FULCIO_ROOT", "no-such-fulcio")
        .env("TUF_ROOT", "no-such-tuf")
        .env("GIT_DIR", "no-such-git-dir")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the Cargo-built CLI must start");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(body)
        .expect("request");
    child.wait_with_output().expect("output")
}

fn verdict(fixture: &Fixture, request: &Value) -> Value {
    let bin = fixture.bin();
    verdict_on(fixture, request, &bin)
}

fn verdict_on(fixture: &Fixture, request: &Value, path: &Path) -> Value {
    let output = run(fixture, request.to_string().as_bytes(), path);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    serde_json::from_slice(&output.stdout).expect("json line")
}

#[test]
#[trace("TC-278", "FR-040-AC-1", "FR-040-CON-1")]
fn tc_278_an_accepted_commit_and_tag_are_signed_valid_with_the_request_identity() {
    let fixture = fixture();
    let id = forged_commit(&fixture, "signed", CMS_BLOCK);
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    for (revision, kind) in [(id.as_str(), "commit"), ("v-cms", "tag")] {
        let result = verdict(&fixture, &request(&fixture, revision, kind, true));
        assert_eq!(result["status"], "signed_valid", "{result}");
        assert_eq!(result["signature_kind"], "gitsign");
        assert_eq!(
            result["certificate"],
            json!({"identity": IDENTITY, "oidc_issuer": ISSUER})
        );
        assert_eq!(
            result["signer"],
            json!({"source": "gitsign_registry", "mapping": "mapped", "principal": PRINCIPAL})
        );
        assert!(result.get("reason").is_none());
        assert!(result.get("key_fingerprint").is_none());
    }
    assert_eq!(fixture.calls(), 2);
}

#[test]
#[trace("TC-279", "FR-040-AC-2")]
fn tc_279_gitsign_gets_the_commit_id_or_the_snapshot_tag_ref_in_an_isolated_repository() {
    let fixture = fixture();
    let id = forged_commit(&fixture, "signed", CMS_BLOCK);
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    // A branch name resolves to the id; the request text is never passed.
    verdict(&fixture, &request(&fixture, "signed", "commit", true));
    verdict(&fixture, &request(&fixture, "v-cms", "tag", true));
    verdict(&fixture, &request(&fixture, "refs/tags/v-cms", "tag", true));
    assert_eq!(fixture.calls(), 3);
    let flags = |target: &str, sub: &str| {
        vec![
            sub.to_owned(),
            "--certificate-identity".to_owned(),
            IDENTITY.to_owned(),
            "--certificate-oidc-issuer".to_owned(),
            ISSUER.to_owned(),
            "--".to_owned(),
            target.to_owned(),
        ]
    };
    assert_eq!(fixture.lines("args", 1), flags(&id, "verify"));
    // verify-tag gets the bare name of the snapshot's tag ref, never the object id.
    assert_eq!(fixture.lines("args", 2), flags("verified", "verify-tag"));
    assert_eq!(fixture.lines("args", 3), flags("verified", "verify-tag"));
    for call in 1..=3 {
        let cwd = PathBuf::from(&fixture.lines("cwd", call)[0]);
        if call == 1 {
            assert_eq!(cwd, fixture.repo());
        } else {
            assert_ne!(cwd, fixture.repo());
            assert!(cwd.starts_with(std::env::temp_dir()));
        }
    }
    // Git is never asked to verify the CMS object.
    let log = fs::read_to_string(fixture.bin().join("gitlog")).expect("git log");
    assert!(
        !log.contains("verify-commit") && !log.contains("verify-tag"),
        "{log}"
    );
}

#[test]
#[trace("TC-1011", "FR-041-AC-25")]
fn tc_293_gitsign_uses_an_isolated_config_snapshot_when_repository_config_changes() {
    let fixture = fixture();
    let tag_id = forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    let replacement_id = forged_tag(&fixture, "v-replacement", "replacement", CMS_BLOCK);
    let repository = fixture.repo();
    let real_git_path = which("git");
    let real_git = real_git_path.display();
    let stand_in = STAND_IN.replace(
        "exit \"$status\"",
        &format!(
            "{real_git} -C '{}' update-ref refs/tags/v-cms '{replacement_id}'\n{real_git} -C '{}' config gitsign.program /changed/during/verification\ngit rev-parse refs/tags/verified > \"$d/snapshot-tag\"\ngit config --get-regexp 'gitsign\\..*' > \"$d/snapshot-gitsign-config\" 2>&1 || true\nexit \"$status\"",
            repository.display(),
            repository.display(),
        ),
    );
    fs::write(fixture.bin().join("gitsign"), stand_in).expect("gitsign stand-in");
    let verified = verdict(&fixture, &request(&fixture, "v-cms", "tag", true));

    assert_eq!(verified["status"], "signed_valid", "{verified}");
    assert_eq!(fixture.lines("args", 1)[6], "verified");
    assert_eq!(
        fs::read_to_string(fixture.bin().join("snapshot-tag"))
            .expect("snapshot tag")
            .trim(),
        tag_id
    );
    assert_eq!(
        git(&repository, &["config", "--get", "gitsign.program"]),
        "/changed/during/verification\n"
    );
    assert_eq!(
        git(&repository, &["rev-parse", "refs/tags/v-cms"]).trim(),
        replacement_id
    );
    let cwd = PathBuf::from(&fixture.lines("cwd", 1)[0]);
    assert_ne!(cwd, repository);
    assert!(
        fs::read_to_string(fixture.bin().join("snapshot-gitsign-config"))
            .expect("snapshot config")
            .is_empty()
    );
}

#[test]
#[trace("TC-1033", "FR-040-AC-21", "FR-041-AC-23")]
fn tc_279_gitsign_verify_tag_runs_in_a_non_bare_repository_bound_to_the_resolved_tag_object() {
    let fixture = fixture();
    let tag_id = forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    let stand_in = STAND_IN.replace(
        "exit \"$status\"",
        "{ git rev-parse --is-bare-repository; git rev-parse --git-dir; git rev-parse \"refs/tags/$7\"; git cat-file -t \"refs/tags/$7\"; test ! -e .git/objects/info/alternates && echo no-alternates; } > \"$d/snapshot_state_$n\" 2>&1\ngit cat-file --batch-all-objects --batch-check='%(objectname) %(objecttype)' > \"$d/snapshot_objects_$n\" 2>&1\nexit \"$status\"",
    );
    fs::write(fixture.bin().join("gitsign"), stand_in).expect("gitsign stand-in");
    let result = verdict(&fixture, &request(&fixture, "v-cms", "tag", true));
    assert_eq!(result["status"], "signed_valid", "{result}");
    assert_eq!(fixture.lines("args", 1)[6], "verified");
    assert_eq!(
        fixture.lines("snapshot_state", 1),
        ["false", ".git", tag_id.as_str(), "tag", "no-alternates"]
    );
    // Every object loose or packed in the snapshot: the resolved tag and nothing else.
    assert_eq!(
        fixture.lines("snapshot_objects", 1),
        [format!("{tag_id} tag")]
    );
}

/// Replaces the fixture's `git` with a wrapper that runs `branch` first for
/// the arguments it matches, so a test can make one snapshot step misbehave.
fn git_misbehaving_when(fixture: &Fixture, pattern: &str, branch: &str) {
    let wrapper = format!(
        "#!/bin/sh\necho \"$*\" >> \"${{0%/*}}/gitlog\"\ncase \"$*\" in\n  {pattern}) {branch} ;;\nesac\nexec {} \"$@\"\n",
        which("git").display()
    );
    fs::write(fixture.bin().join("git"), wrapper).expect("git wrapper");
}

#[test]
#[trace("TC-1034", "FR-040-AC-22")]
fn tc_1034_a_snapshot_object_that_does_not_rehash_to_the_resolved_id_is_refused() {
    let fixture = fixture();
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    // The object read for the snapshot gains a line, so it stores under another id.
    let real_git = which("git");
    git_misbehaving_when(
        &fixture,
        "*\"cat-file tag\"*",
        &format!("{} \"$@\"; echo extra; exit 0", real_git.display()),
    );
    let result = verdict(&fixture, &request(&fixture, "v-cms", "tag", true));
    assert_eq!(result["status"], "unverifiable", "{result}");
    assert_eq!(result["reason"], "snapshot_object_mismatch", "{result}");
    assert_eq!(fixture.calls(), 0, "the stand-in must never start");
}

#[test]
#[trace("TC-1034", "FR-040-AC-22")]
fn tc_1034_a_snapshot_that_cannot_be_made_is_unavailable() {
    let fixture = fixture();
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    git_misbehaving_when(&fixture, "*hash-object*", "exit 1");
    let result = verdict(&fixture, &request(&fixture, "v-cms", "tag", true));
    assert_eq!(result["status"], "unverifiable", "{result}");
    assert_eq!(result["reason"], "snapshot_unavailable", "{result}");
    assert_eq!(fixture.calls(), 0, "the stand-in must never start");
}

#[test]
#[trace("TC-1035", "FR-040-AC-23", "FR-041-AC-23")]
fn tc_1035_a_snapshot_ref_that_changes_during_verification_fails_closed() {
    for name in ["moved", "deleted"] {
        let fixture = fixture();
        let other_id = forged_tag(&fixture, "v-other", "other", CMS_BLOCK);
        forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
        let change = if name == "moved" {
            format!("echo '{other_id}' > .git/refs/tags/verified")
        } else {
            "/usr/bin/rm .git/refs/tags/verified".to_owned()
        };
        let stand_in = STAND_IN.replace("exit \"$status\"", &format!("{change}\nexit \"$status\""));
        fs::write(fixture.bin().join("gitsign"), stand_in).expect("gitsign stand-in");
        let result = verdict(&fixture, &request(&fixture, "v-cms", "tag", true));
        assert_eq!(result["status"], "unverifiable", "{name}: {result}");
        assert_eq!(result["reason"], "snapshot_ref_changed", "{name}: {result}");
    }
}

#[test]
#[trace("TC-1036", "FR-040-AC-24", "FR-040-CON-1")]
fn tc_1036_the_snapshot_holds_no_template_include_or_gitsign_configuration() {
    let fixture = fixture();
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    // A global init template with a gitsign root and a hook, named from the
    // caller's HOME configuration.
    let template = fixture.root().join("template");
    fs::create_dir_all(template.join("hooks")).expect("template hooks");
    fs::write(template.join("hooks/post-checkout"), "#!/bin/sh\n").expect("hook");
    fs::write(template.join("config"), "[gitsign]\n\tfulcioRoot = /evil\n").expect("template");
    fs::write(
        fixture.home().join(".gitconfig"),
        format!(
            "[init]\n\ttemplateDir = {}\n[gitsign]\n\tfulcioRoot = /evil\n",
            template.display()
        ),
    )
    .expect("global config");
    // Source include and includeIf keys that pull in only a harmless setting.
    let git_dir = fixture.repo().join(".git");
    fs::write(git_dir.join("extra.cfg"), "[user]\n\tname = included\n").expect("include");
    let mut config = fs::read_to_string(git_dir.join("config")).expect("config");
    config
        .push_str("[include]\n\tpath = extra.cfg\n[includeIf \"gitdir:**\"]\n\tpath = extra.cfg\n");
    fs::write(git_dir.join("config"), config).expect("config");
    let stand_in = STAND_IN.replace(
        "exit \"$status\"",
        "git config --list > \"$d/snapshot_config_$n\" 2>&1\nfor f in * .[!.]* .git/* .git/.[!.]*; do [ -e \"$f\" ] && echo \"$f\"; done > \"$d/snapshot_files_$n\"\nexit \"$status\"",
    );
    fs::write(fixture.bin().join("gitsign"), stand_in).expect("gitsign stand-in");

    let result = verdict(&fixture, &request(&fixture, "v-cms", "tag", true));
    assert_eq!(result["status"], "signed_valid", "{result}");
    let mut config = fixture.lines("snapshot_config", 1);
    config.sort();
    assert_eq!(
        config,
        ["core.bare=false", "core.repositoryformatversion=0"]
    );
    let mut files = fixture.lines("snapshot_files", 1);
    files.sort();
    assert_eq!(
        files,
        [
            ".git",
            ".git/HEAD",
            ".git/config",
            ".git/objects",
            ".git/refs"
        ]
    );

    // A gitsign key in the source repository still stops the run before any
    // snapshot is made.
    let refused = {
        let fixture = fixture_with_tag_and_source_key();
        let result = verdict(&fixture, &request(&fixture, "v-cms", "tag", true));
        assert_eq!(result["status"], "unverifiable", "{result}");
        assert_eq!(result["reason"], "gitsign_repository_config", "{result}");
        assert_eq!(fixture.calls(), 0);
        fs::read_to_string(fixture.bin().join("gitlog")).expect("git log")
    };
    assert!(!refused.contains("hash-object"), "{refused}");
}

fn fixture_with_tag_and_source_key() -> Fixture {
    let fixture = fixture();
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    git(&fixture.repo(), &["config", "gitsign.fulcioRoot", "/x"]);
    fixture
}

#[test]
#[trace("TC-1012", "FR-041-AC-23", "FR-041-AC-24")]
fn tc_294_gitsign_keeps_verification_on_the_loaded_object_after_its_tag_ref_is_deleted() {
    let fixture = fixture();
    forged_tag(&fixture, "v-control", "control", CMS_BLOCK);
    forged_tag(&fixture, "v-deleted", "deleted", CMS_BLOCK);
    let control_id = git(&fixture.repo(), &["rev-parse", "v-control^{tag}"])
        .trim()
        .to_owned();
    let deleted_id = git(&fixture.repo(), &["rev-parse", "v-deleted^{tag}"])
        .trim()
        .to_owned();
    let real_git_path = which("git");
    let real_git = real_git_path.display();
    let stand_in = STAND_IN.replace(
        "exit \"$status\"",
        &format!(
            "if [ \"$(git rev-parse refs/tags/verified)\" = '{deleted_id}' ]; then {real_git} -C '{}' update-ref -d refs/tags/v-deleted; : > \"$d/source-ref-deleted\"; fi\nexit \"$status\"",
            fixture.repo().display(),
        ),
    );
    fs::write(fixture.bin().join("gitsign"), stand_in).expect("gitsign stand-in");

    let control = verdict(&fixture, &request(&fixture, "v-control", "tag", true));
    assert_eq!(control["status"], "signed_valid", "{control}");
    assert_eq!(fixture.lines("args", 1)[6], "verified");
    assert!(!fixture.bin().join("source-ref-deleted").exists());
    assert_ne!(control_id, deleted_id);

    let deleted = verdict(&fixture, &request(&fixture, "v-deleted", "tag", true));
    assert_eq!(deleted["status"], "signed_valid", "{deleted}");
    assert_eq!(fixture.lines("args", 2)[6], "verified");
    assert!(fixture.bin().join("source-ref-deleted").exists());
    let deleted_ref = Command::new("git")
        .args([
            "-C",
            fixture.repo().to_str().expect("repo path"),
            "show-ref",
            "--verify",
            "refs/tags/v-deleted",
        ])
        .output()
        .expect("show deleted ref");
    assert!(!deleted_ref.status.success());
}

#[test]
#[trace("TC-280", "FR-040-AC-3")]
fn tc_280_a_wildcard_only_acceptance_is_untrusted_with_the_mismatch() {
    let cases = [
        // (exact+exact, exact id/any issuer, any id/exact issuer) -> mismatch
        ("exact_any", 0, "any_exact", 1, "issuer", 3),
        ("exact_any", 1, "any_exact", 0, "identity", 4),
        ("exact_any", 1, "any_exact", 1, "identity_and_issuer", 4),
    ];
    for (issuer_flavor, issuer_status, identity_flavor, identity_status, mismatch, calls) in cases {
        let fixture = fixture_with_commit();
        fixture.script("exact_exact", 1);
        fixture.script("any_any", 0);
        fixture.script(issuer_flavor, issuer_status);
        fixture.script(identity_flavor, identity_status);
        let result = verdict(&fixture, &request(&fixture, "signed", "commit", true));
        assert_eq!(result["status"], "signed_untrusted", "{result}");
        assert_eq!(result["mismatch"], mismatch);
        assert_eq!(
            result["signer"],
            json!({"source": "gitsign_registry", "mapping": "unmapped"})
        );
        assert!(result.get("certificate").is_none());
        assert_eq!(result["signature_kind"], "gitsign");
        assert_eq!(fixture.calls(), calls);
    }
}

fn fixture_with_commit() -> Fixture {
    let fixture = fixture();
    forged_commit(&fixture, "signed", CMS_BLOCK);
    fixture
}

#[test]
#[trace("TC-281", "FR-040-AC-4")]
fn tc_281_failures_are_unverifiable_and_never_bad_or_untrusted() {
    let fixture = fixture_with_commit();
    fixture.script("default", 1);
    let result = verdict(&fixture, &request(&fixture, "signed", "commit", true));
    assert_eq!(result["status"], "unverifiable", "{result}");
    assert_eq!(result["reason"], "gitsign_verification_failed");
    assert_eq!(fixture.calls(), 2);

    // Exit 2, or any other status, at each step of the procedure.
    for status in [2, 3, 127] {
        for step in ["exact_exact", "any_any", "exact_any", "any_exact"] {
            let fixture = fixture_with_commit();
            fixture.script("exact_exact", 1);
            fixture.script("any_any", 0);
            fixture.script("exact_any", 1);
            fixture.script("any_exact", 1);
            fixture.script(step, status);
            let result = verdict(&fixture, &request(&fixture, "signed", "commit", true));
            assert_eq!(
                result["status"], "unverifiable",
                "{step} {status}: {result}"
            );
            assert_eq!(result["reason"], "gitsign_verification_failed");
        }
    }
}

#[test]
#[trace("TC-282", "FR-040-AC-5")]
fn tc_282_a_cms_object_without_a_gitsign_member_is_missing_its_identity() {
    let fixture = fixture_with_commit();
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    for (revision, kind) in [("signed", "commit"), ("v-cms", "tag")] {
        let result = verdict(&fixture, &request(&fixture, revision, kind, false));
        assert_eq!(result["status"], "unverifiable", "{result}");
        assert_eq!(result["reason"], "gitsign_identity_missing");
        assert_eq!(result["signature_kind"], "gitsign");
    }
    assert_eq!(fixture.calls(), 0);
}

#[test]
#[trace("TC-283", "TC-1016", "FR-040-AC-6", "FR-040-AC-20")]
fn tc_283_a_missing_gitsign_executable_is_not_started() {
    let fixture = fixture_with_commit();
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    let bare = fixture.root().join("bare");
    fs::create_dir(&bare).expect("bare");
    symlink(which("git"), bare.join("git")).expect("git link");
    for (revision, kind) in [("signed", "commit"), ("v-cms", "tag")] {
        let result = verdict_on(&fixture, &request(&fixture, revision, kind, true), &bare);
        assert_eq!(result["status"], "unverifiable", "{result}");
        assert_eq!(result["reason"], "gitsign_not_started");
    }
}

#[test]
#[trace("TC-284", "FR-040-AC-7")]
fn tc_284_output_above_the_capture_bound_is_output_exceeded() {
    let fixture = fixture_with_commit();
    fs::write(fixture.bin().join("flood"), "").expect("flood");
    let result = verdict(&fixture, &request(&fixture, "signed", "commit", true));
    assert_eq!(result["status"], "unverifiable", "{result}");
    assert_eq!(result["reason"], "output_exceeded");
}

#[test]
#[trace("TC-285", "FR-040-AC-8")]
fn tc_285_unsigned_and_openpgp_objects_are_unchanged_with_or_without_gitsign() {
    let fixture = fixture();
    git(&fixture.repo(), &["tag", "-a", "-m", "plain", "v-plain"]);
    let pgp = "-----BEGIN PGP SIGNATURE-----\nAAAA\n-----END PGP SIGNATURE-----\n";
    forged_commit(&fixture, "pgp", pgp);
    forged_tag(&fixture, "v-pgp", "tag message", pgp);
    for gitsign in [false, true] {
        for (revision, kind) in [("main", "commit"), ("v-plain", "tag")] {
            let result = verdict(&fixture, &request(&fixture, revision, kind, gitsign));
            assert_eq!(result["status"], "unsigned", "{result}");
            assert!(result.get("signature_kind").is_none());
            assert!(result.get("reason").is_none());
        }
        for (revision, kind) in [("signed", "commit"), ("v-pgp", "tag")] {
            let result = verdict(&fixture, &request(&fixture, revision, kind, gitsign));
            assert_eq!(result["status"], "unverifiable", "{result}");
            assert_eq!(result["reason"], "unsupported_signature_format");
            assert!(result.get("signature_kind").is_none());
        }
    }
    assert_eq!(fixture.calls(), 0);
}

#[test]
#[trace("TC-286", "FR-040-AC-11")]
fn tc_286_a_malformed_gitsign_member_is_refused_before_git_starts() {
    let fixture = fixture_with_commit();
    let empty = fixture.root().join("empty");
    fs::create_dir(&empty).expect("empty");
    let long = "x".repeat(257);
    let bad: Vec<Value> = vec![
        json!({"certificate_oidc_issuer": ISSUER}),
        json!({"certificate_identity": IDENTITY}),
        json!({"certificate_identity": "", "certificate_oidc_issuer": ISSUER}),
        json!({"certificate_identity": IDENTITY, "certificate_oidc_issuer": ""}),
        json!({"certificate_identity": long, "certificate_oidc_issuer": ISSUER}),
        json!({"certificate_identity": IDENTITY, "certificate_oidc_issuer": long}),
        json!({"certificate_identity": "-alice@example.test", "certificate_oidc_issuer": ISSUER}),
        json!({"certificate_identity": IDENTITY, "certificate_oidc_issuer": concat!("-https:", "//x.test")}),
        json!({"certificate_identity": "alice\n@example.test", "certificate_oidc_issuer": ISSUER}),
        json!({"certificate_identity": IDENTITY, "certificate_oidc_issuer": ISSUER, "extra": 1}),
        // A principal beside an invalid member does not validate, so it is a
        // request error and not the principal refusal.
        json!({"certificate_identity": "", "certificate_oidc_issuer": ISSUER, "principal": PRINCIPAL}),
        json!("alice"),
    ];
    for gitsign in bad {
        let mut body = request(&fixture, "signed", "commit", false);
        body["gitsign"] = gitsign.clone();
        body["identity_registry"] = json!(REGISTRY);
        assert_request_invalid(&fixture, &body, &empty);
    }
    // `gitsign` without `identity_registry`, with or without a `principal`,
    // `identity_registry` without `gitsign`, and a malformed address.
    let identity = json!({"certificate_identity": IDENTITY, "certificate_oidc_issuer": ISSUER});
    let with_principal = json!({
        "certificate_identity": IDENTITY,
        "certificate_oidc_issuer": ISSUER,
        "principal": PRINCIPAL,
    });
    let mut bodies = Vec::new();
    for gitsign in [identity.clone(), with_principal] {
        let mut body = request(&fixture, "signed", "commit", false);
        body["gitsign"] = gitsign;
        bodies.push(body);
    }
    let mut body = request(&fixture, "signed", "commit", false);
    body["identity_registry"] = json!(REGISTRY);
    bodies.push(body);
    for address in [
        "registry.json",
        ":registry.json",
        "registry:",
        "-x:registry.json",
        "registry:/registry.json",
        "registry:../registry.json",
        "registry:a//b",
        "registry:./registry.json",
        "registry:registry.json/",
        "reg\u{7}:registry.json",
        "registry:reg\u{7}.json",
    ] {
        let mut body = request(&fixture, "signed", "commit", false);
        body["gitsign"] = identity.clone();
        body["identity_registry"] = json!(address);
        bodies.push(body);
    }
    let mut body = request(&fixture, "signed", "commit", false);
    body["gitsign"] = identity;
    body["identity_registry"] = json!(7);
    bodies.push(body);
    for body in bodies {
        assert_request_invalid(&fixture, &body, &empty);
    }
    assert_eq!(fixture.calls(), 0);
}

/// An empty PATH could not start git: exit 2 proves git was not started.
fn assert_request_invalid(fixture: &Fixture, body: &Value, empty: &Path) {
    let output = run(fixture, body.to_string().as_bytes(), empty);
    assert_eq!(output.status.code(), Some(2), "{body}: {output:?}");
    let error: Value = serde_json::from_slice(&output.stdout).expect("machine error");
    assert_eq!(error["code"], "signature_verify_request_invalid", "{body}");
}

/// Executables that record that they ran, then exit 0.
fn install_hooks(directory: &Path, marker: &Path, programs: &[&str]) {
    for program in programs {
        let script = format!("#!/bin/sh\n: > {}\nexit 0\n", marker.display());
        fs::write(directory.join(program), script).expect("hook");
        fs::set_permissions(directory.join(program), fs::Permissions::from_mode(0o755))
            .expect("mode");
    }
}

fn ssh_fixture() -> (Fixture, String) {
    let fixture = fixture();
    let root = fixture.root();
    let key = root.join("key");
    run_program(
        root,
        "ssh-keygen",
        &[
            "-q",
            "-t",
            "ed25519",
            "-N",
            "",
            "-C",
            "k",
            "-f",
            key.to_str().expect("utf-8"),
        ],
        None,
    );
    let public = fs::read_to_string(root.join("key.pub")).expect("public key");
    fs::write(fixture.allowed(), format!("signer@example.test {public}")).expect("allowed");
    let key = root.join("key.pub").display().to_string();
    (fixture, key)
}

#[test]
#[trace("TC-287", "FR-040-AC-12")]
fn tc_287_ssh_objects_keep_the_ssh_verdict_and_never_start_gitsign() {
    let (fixture, key) = ssh_fixture();
    let repo = fixture.repo();
    let sign = |extra: &[&str]| {
        let mut all = vec!["-c", "gpg.format=ssh", "-c"];
        let setting = format!("user.signingkey={key}");
        all.push(&setting);
        all.extend_from_slice(extra);
        git(&repo, &all);
    };
    sign(&["commit", "-q", "-S", "--allow-empty", "-m", "ssh signed"]);
    sign(&["tag", "-s", "-m", "ssh tag", "v-ssh"]);
    // The real git and ssh-keygen are needed for the SSH path.
    let path = fixture.root().join("ssh-bin");
    fs::create_dir(&path).expect("ssh-bin");
    symlink(which("git"), path.join("git")).expect("git link");
    symlink(which("ssh-keygen"), path.join("ssh-keygen")).expect("ssh-keygen link");
    symlink(fixture.bin().join("gitsign"), path.join("gitsign")).expect("gitsign link");
    for (revision, kind) in [("HEAD", "commit"), ("v-ssh", "tag")] {
        let result = verdict_on(&fixture, &request(&fixture, revision, kind, true), &path);
        assert_eq!(result["status"], "signed_valid", "{result}");
        assert_eq!(result["signature_kind"], "ssh");
        assert_eq!(
            result["signer"],
            json!({"source": "allowed_signers", "mapping": "mapped", "principal": "signer@example.test"})
        );
        assert!(result.get("certificate").is_none());
        assert!(result.get("mismatch").is_none());
    }
    assert_eq!(fixture.calls(), 0);
}

#[test]
#[trace("TC-288", "FR-040-AC-13")]
fn tc_288_the_signed_off_meaning_is_reported_for_every_gitsign_status() {
    let fixture = fixture();
    let message = "work\n\nSigned-off-meaning: reviewed";
    forged_commit(&fixture, message, CMS_BLOCK);
    forged_tag(&fixture, "v-cms", message, CMS_BLOCK);
    let expect = json!({"presence": "present", "values": ["reviewed"]});
    for (revision, kind) in [("signed", "commit"), ("v-cms", "tag")] {
        let valid = verdict(&fixture, &request(&fixture, revision, kind, true));
        assert_eq!(valid["status"], "signed_valid");
        assert_eq!(valid["signed_off_meaning"], expect);
        let missing = verdict(&fixture, &request(&fixture, revision, kind, false));
        assert_eq!(missing["status"], "unverifiable");
        assert_eq!(missing["signed_off_meaning"], expect);
    }
    fixture.script("default", 1);
    let failed = verdict(&fixture, &request(&fixture, "signed", "commit", true));
    assert_eq!(failed["reason"], "gitsign_verification_failed");
    assert_eq!(failed["signed_off_meaning"], expect);
}

#[test]
#[trace("TC-289", "FR-040-AC-17", "FR-040-CON-1")]
fn tc_289_a_gitsign_key_in_any_repository_configuration_stops_the_run() {
    let local = |f: &Fixture| {
        git(&f.repo(), &["config", "gitsign.fulcioRoot", "/x"]);
    };
    let included = |f: &Fixture| {
        let file = f.root().join("included.cfg");
        fs::write(&file, "[gitsign]\n\trekorURL = rekor-log\n").expect("include");
        git(
            &f.repo(),
            &["config", "include.path", file.to_str().expect("utf-8")],
        );
    };
    let worktree = |f: &Fixture| {
        git(&f.repo(), &["config", "extensions.worktreeConfig", "true"]);
        git(
            &f.repo(),
            &["config", "--worktree", "gitsign.connectorID", "x"],
        );
    };
    // A newline inside another key's value puts a `gitsign.` line in the output
    // of `git config --get-regexp .*`, which is how gitsign reads its settings.
    let injected = |f: &Fixture| {
        let config = f.repo().join(".git").join("config");
        let mut text = fs::read_to_string(&config).expect("config");
        text.push_str("[foo]\n\tbar = \"x\\ngitsign.fulcioRoot evil\"\n");
        fs::write(&config, text).expect("config");
        let listed = git(&f.repo(), &["config", "--get-regexp", ".*"]);
        assert!(
            listed
                .lines()
                .any(|l| l.to_ascii_lowercase().starts_with("gitsign.fulcioroot ")),
            "{listed}"
        );
        let scoped = Command::new("git")
            .current_dir(f.repo())
            .args(["config", "--get-regexp", "^gitsign\\."])
            .output()
            .expect("git");
        assert_eq!(
            scoped.status.code(),
            Some(1),
            "a scoped query misses the injection"
        );
    };
    // gitsign folds case with Unicode simple folding, so a long s or a Kelvin
    // sign in an injected key is read as `gitsign.`; the check fails closed.
    let folded = |name: &'static str| {
        move |f: &Fixture| {
            let config = f.repo().join(".git").join("config");
            let mut text = fs::read_to_string(&config).expect("config");
            text.extend(["[foo]\n\tbar = \"x\\n", name, ".fulcioRoot evil\"\n"]);
            fs::write(&config, text).expect("config");
            let listed = git(&f.repo(), &["config", "--get-regexp", ".*"]);
            assert!(listed.contains(name), "{listed}");
        }
    };
    let long_s = folded("git\u{17f}ign");
    let kelvin = folded("gitsign\u{212a}");
    let setters: [&dyn Fn(&Fixture); 6] =
        [&local, &included, &worktree, &injected, &long_s, &kelvin];
    for set in setters {
        let fixture = fixture_with_commit();
        forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
        set(&fixture);
        for (revision, kind) in [("signed", "commit"), ("v-cms", "tag")] {
            let result = verdict(&fixture, &request(&fixture, revision, kind, true));
            assert_eq!(result["status"], "unverifiable", "{result}");
            assert_eq!(result["reason"], "gitsign_repository_config");
        }
        assert_eq!(fixture.calls(), 0);
    }
    let clean = fixture_with_commit();
    git(&clean.repo(), &["config", "gitsignfoo.other", "unrelated"]);
    let result = verdict(&clean, &request(&clean, "signed", "commit", true));
    assert_eq!(result["status"], "signed_valid", "{result}");
    assert_eq!(clean.calls(), 1);
}

#[test]
#[trace("TC-290", "FR-040-AC-18")]
fn tc_290_gitsign_gets_exactly_four_environment_variables() {
    let fixture = fixture_with_commit();
    verdict(&fixture, &request(&fixture, "signed", "commit", true));
    let mut names: Vec<String> = fixture
        .lines("env", 1)
        .iter()
        .filter_map(|line| line.split_once('=').map(|(name, _)| name.to_owned()))
        .collect();
    names.sort();
    assert_eq!(
        names,
        ["GIT_CONFIG_GLOBAL", "GIT_CONFIG_NOSYSTEM", "HOME", "PATH"]
    );
    let env = fixture.lines("env", 1);
    assert!(env.contains(&"GIT_CONFIG_NOSYSTEM=1".to_owned()));
    assert!(env.contains(&"GIT_CONFIG_GLOBAL=/dev/null".to_owned()));
    assert!(env.contains(&format!("HOME={}", fixture.home().display())));
}

#[test]
#[trace("TC-291", "FR-040-AC-19")]
fn tc_291_a_tag_that_is_not_an_annotated_tag_ref_never_starts_gitsign() {
    let fixture = fixture_with_commit();
    git(&fixture.repo(), &["tag", "light"]);
    let id = forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    forged_tag(&fixture, "--help", "tag message", CMS_BLOCK);
    let light = verdict(&fixture, &request(&fixture, "light", "tag", true));
    assert_eq!(light["status"], "unsigned", "{light}");
    assert_eq!(light["reason"], "lightweight_tag");
    // The tag object's own id, a branch and an unknown name are not tag refs.
    for revision in [
        id.as_str(),
        "main",
        "refs/tags/nope",
        "v-cms^{tag}",
        "refs/tags/--help",
    ] {
        let result = verdict(&fixture, &request(&fixture, revision, "tag", true));
        assert_eq!(result["status"], "unverifiable", "{revision}: {result}");
        assert_eq!(result["reason"], "revision_unresolved", "{revision}");
    }
    assert_eq!(fixture.calls(), 0);
}

#[test]
#[trace("TC-297", "FR-040-AC-6", "FR-040-CON-1")]
fn tc_297_a_relative_path_entry_never_supplies_gitsign_or_git() {
    let fixture = fixture_with_commit();
    // Programs inside the repository that would report a valid signature.
    let hook = fixture.repo().join("hook");
    fs::create_dir(&hook).expect("hook dir");
    let marker = fixture.root().join("hook-ran");
    install_hooks(&hook, &marker, &["gitsign", "git"]);
    let request = request(&fixture, "signed", "commit", true);
    // The absolute `gitsign` is used, never the one in the relative entry.
    let path = std::env::join_paths([Path::new("hook"), fixture.bin().as_path()]).expect("path");
    let result = verdict_on(&fixture, &request, Path::new(&path));
    assert_eq!(result["status"], "signed_valid", "{result}");
    assert_eq!(fixture.calls(), 1);
    // With no absolute `gitsign` the relative one is not found either.
    let bare = fixture.root().join("bare");
    fs::create_dir(&bare).expect("bare");
    symlink(which("git"), bare.join("git")).expect("git link");
    let path = std::env::join_paths([Path::new("hook"), bare.as_path()]).expect("path");
    let result = verdict_on(&fixture, &request, Path::new(&path));
    assert_eq!(result["reason"], "gitsign_not_started", "{result}");
    assert!(!marker.exists(), "a program from the repository ran");
}

#[test]
#[trace("TC-299", "FR-036-AC-1", "FR-036-CON-1")]
fn tc_299_a_relative_path_entry_never_supplies_git_or_ssh_keygen_on_the_ssh_path() {
    let (fixture, key) = ssh_fixture();
    let repo = fixture.repo();
    let setting = format!("user.signingkey={key}");
    git(
        &repo,
        &[
            "-c",
            "gpg.format=ssh",
            "-c",
            &setting,
            "commit",
            "-q",
            "-S",
            "--allow-empty",
            "-m",
            "s",
        ],
    );
    git(
        &repo,
        &[
            "-c",
            "gpg.format=ssh",
            "-c",
            &setting,
            "tag",
            "-s",
            "-m",
            "t",
            "v-ssh",
        ],
    );
    let hook = repo.join("hook");
    fs::create_dir(&hook).expect("hook dir");
    let marker = fixture.root().join("hook-ran");
    install_hooks(&hook, &marker, &["git", "ssh-keygen"]);
    let real = fixture.root().join("real-bin");
    fs::create_dir(&real).expect("real-bin");
    for program in ["git", "ssh-keygen"] {
        symlink(which(program), real.join(program)).expect("link");
    }
    let path = std::env::join_paths([Path::new("hook"), real.as_path()]).expect("path");
    for (revision, kind) in [("HEAD", "commit"), ("v-ssh", "tag")] {
        let result = verdict_on(
            &fixture,
            &request(&fixture, revision, kind, false),
            Path::new(&path),
        );
        assert_eq!(result["status"], "signed_valid", "{result}");
    }
    // Only the relative entry holds the programs: they are not found at all.
    let result = verdict_on(
        &fixture,
        &request(&fixture, "HEAD", "commit", false),
        Path::new("hook"),
    );
    assert_eq!(result["reason"], "git_not_started", "{result}");
    assert!(!marker.exists(), "a program from the repository ran");
}

#[test]
#[ignore = "needs the real gitsign, network access and a public gitsign-signed commit and tag named by FR_040_LIVE_*"]
#[trace(
    "TC-292",
    "TC-1013",
    "TC-1014",
    "TC-1015",
    "TC-1016",
    "FR-040-AC-15",
    "FR-040-AC-20"
)]
fn tc_292_real_gitsign_verifies_a_public_signed_commit_and_tag() {
    let (Ok(repository), Ok(commit), Ok(tag), Ok(identity), Ok(issuer)) = (
        std::env::var("FR_040_LIVE_REPOSITORY"),
        std::env::var("FR_040_LIVE_COMMIT"),
        std::env::var("FR_040_LIVE_TAG"),
        std::env::var("FR_040_LIVE_IDENTITY"),
        std::env::var("FR_040_LIVE_ISSUER"),
    ) else {
        panic!(
            "UNAVAILABLE: set FR_040_LIVE_REPOSITORY, _COMMIT, _TAG, _IDENTITY and _ISSUER; this is not live interoperability evidence"
        );
    };
    let version = Command::new("gitsign")
        .arg("--version")
        .output()
        .unwrap_or_else(|error| {
            panic!("LIVE_CASE=UNAVAILABLE gitsign_not_started error={error}; no interoperability pass recorded")
        });
    assert!(version.status.success(), "gitsign --version: {version:?}");
    let version_text = String::from_utf8_lossy(&version.stdout).into_owned();
    assert_gitsign_018_version(&version_text);
    let live_checkout = tempfile::tempdir().expect("live checkout");
    let live_repository = live_checkout.path().join("repository");
    let clone = Command::new("git")
        .args(["clone", "--no-hardlinks", "--no-checkout", &repository])
        .arg(&live_repository)
        .output()
        .expect("clone public live repository");
    assert!(clone.status.success(), "git clone: {clone:?}");
    let commit_id = git(
        &live_repository,
        &["rev-parse", &format!("{commit}^{{commit}}")],
    );
    let tag_id = git(
        &live_repository,
        &["rev-parse", &format!("refs/tags/{tag}^{{tag}}")],
    );
    eprintln!(
        "LIVE_CASE=STARTED gitsign_version={:?} commit_object={} tag_object={} identity={identity:?} issuer={issuer:?} trust_service=probe_pending",
        version_text.trim(),
        commit_id.trim(),
        tag_id.trim()
    );
    let fixture = fixture();
    fs::write(fixture.allowed(), "").expect("allowed");
    for (revision, kind) in [(&commit, "commit"), (&tag, "tag")] {
        assert_live_identity_cases(
            &fixture,
            &live_repository,
            revision,
            kind,
            &identity,
            &issuer,
        );
    }
    let (tag_object, changed_id) =
        mutate_live_tag(&fixture, &live_repository, &tag, &identity, &issuer);
    assert_ne!(
        tag_object, changed_id,
        "tag payload mutation must change immutable object id"
    );
    eprintln!(
        "LIVE_CASE=INTEROP_PASS gitsign_version={:?} commit_object={} original_tag_object={} changed_tag_object={} identity={identity:?} issuer={issuer:?} trust_service=available",
        version_text.trim(),
        commit_id.trim(),
        tag_object,
        changed_id
    );
}

fn assert_gitsign_018_version(version: &str) {
    let token = version
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .last()
        .unwrap_or_default();
    assert!(
        matches!(token, "v0.18.0" | "0.18.0"),
        "expected exact gitsign v0.18.0 version token, got {version:?}"
    );
}

fn assert_live_identity_cases(
    fixture: &Fixture,
    repository: &Path,
    revision: &str,
    kind: &str,
    identity: &str,
    issuer: &str,
) {
    let accepted = live_verdict(fixture, repository, revision, kind, identity, issuer);
    assert_ne!(
        accepted["status"], "unverifiable",
        "LIVE_CASE=UNAVAILABLE kind={kind} reason={} trust_service=unavailable_or_unverifiable; no interoperability pass recorded",
        accepted["reason"]
    );
    assert_eq!(accepted["status"], "signed_valid", "{accepted}");
    let wrong_identity = live_verdict(
        fixture,
        repository,
        revision,
        kind,
        "wrong-identity@example.test",
        issuer,
    );
    assert_eq!(
        wrong_identity["status"], "signed_untrusted",
        "{wrong_identity}"
    );
    assert_eq!(wrong_identity["mismatch"], "identity", "{wrong_identity}");
    let wrong_issuer = live_verdict(fixture, repository, revision, kind, identity, OTHER_ISSUER);
    assert_eq!(wrong_issuer["status"], "signed_untrusted", "{wrong_issuer}");
    assert_eq!(wrong_issuer["mismatch"], "issuer", "{wrong_issuer}");
}

fn live_verdict(
    fixture: &Fixture,
    repository: &Path,
    revision: &str,
    kind: &str,
    identity: &str,
    issuer: &str,
) -> Value {
    let mut body = request(fixture, revision, kind, true);
    commit_registry(
        repository,
        "registry",
        &registry_json(&[(identity, issuer, PRINCIPAL)]),
    );
    body["repository"] = json!(repository.display().to_string());
    body["gitsign"]["certificate_identity"] = json!(identity);
    body["gitsign"]["certificate_oidc_issuer"] = json!(issuer);
    let path = std::env::var_os("PATH").expect("PATH");
    verdict_on(fixture, &body, Path::new(&path))
}

fn mutate_live_tag(
    fixture: &Fixture,
    repository: &Path,
    tag: &str,
    identity: &str,
    issuer: &str,
) -> (String, String) {
    let refname = format!("refs/tags/{tag}");
    let original_id = git(repository, &["rev-parse", &format!("{refname}^{{tag}}")])
        .trim()
        .to_owned();
    let raw = git(repository, &["cat-file", "tag", &original_id]);
    let (headers, signed_message) = raw.split_once("\n\n").expect("annotated tag headers");
    let signature_at = signed_message
        .find("-----BEGIN SIGNED MESSAGE-----")
        .expect("gitsign CMS block in live tag");
    let (_, signature) = signed_message.split_at(signature_at);
    let changed = format!("{headers}\n\nchanged-by-ea-129\n{signature}");
    let changed_id = git_store_tag(repository.to_str().expect("repository path"), &changed);
    let update = git(repository, &["update-ref", &refname, &changed_id]);
    assert!(update.is_empty(), "update-ref: {update}");
    eprintln!(
        "LIVE_CASE=TAG_MUTATION original_tag_object={original_id} changed_tag_object={changed_id}"
    );
    let mutation = live_verdict(fixture, repository, tag, "tag", identity, issuer);
    assert_eq!(mutation["status"], "unverifiable", "{mutation}");
    assert_eq!(
        mutation["reason"], "gitsign_verification_failed",
        "{mutation}"
    );
    (original_id, changed_id)
}

fn git_store_tag(repository: &str, object: &str) -> String {
    let output = Command::new("git")
        .args([
            "-C",
            repository,
            "hash-object",
            "-t",
            "tag",
            "-w",
            "--stdin",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("git hash-object");
    let mut child = output;
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(object.as_bytes())
        .expect("tag object");
    let output = child.wait_with_output().expect("hash-object output");
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout)
        .expect("object id")
        .trim()
        .to_owned()
}

// ---------------------------------------------------------------------------
// Identity registry (FR-037 trust-source reading, FR-040 mapping)
// ---------------------------------------------------------------------------

/// The registry commit a branch names.
fn tip(repo: &Path, branch: &str) -> String {
    git(repo, &["rev-parse", &format!("refs/heads/{branch}")])
        .trim()
        .to_owned()
}

/// The request for a CMS commit with the registry address `address`.
fn request_with_registry(fixture: &Fixture, address: &str) -> Value {
    let mut body = request(fixture, "signed", "commit", true);
    body["identity_registry"] = json!(address);
    body
}

/// Runs a request and returns its machine error, asserting exit status 2.
fn machine_error(fixture: &Fixture, body: &Value, path: &Path) -> Value {
    let output = run(fixture, body.to_string().as_bytes(), path);
    assert_eq!(output.status.code(), Some(2), "{body}: {output:?}");
    serde_json::from_slice(&output.stdout).expect("machine error")
}

#[test]
#[trace("TC-1041", "FR-040-AC-25", "FR-040-CON-3")]
fn tc_1041_only_an_exact_registry_entry_maps_the_accepted_identity() {
    let fixture = fixture_with_commit();
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    let repo = fixture.repo();
    let other_issuer = OTHER_ISSUER;
    let mapped = |result: &Value| {
        assert_eq!(result["status"], "signed_valid", "{result}");
        result["signer"].clone()
    };
    for (revision, kind) in [("signed", "commit"), ("v-cms", "tag")] {
        let run = |fixture: &Fixture| {
            let mut body = request(fixture, revision, kind, true);
            body["identity_registry"] = json!(REGISTRY);
            verdict(fixture, &body)
        };
        assert_eq!(
            mapped(&run(&fixture)),
            json!({"source": "gitsign_registry", "mapping": "mapped", "principal": PRINCIPAL})
        );
        // None of these registries holds an entry for exactly this identity and issuer.
        let near_misses = [
            registry_json(&[]),
            registry_json(&[("bob@example.test", ISSUER, "Bob")]),
            registry_json(&[(IDENTITY, other_issuer, "Wrong issuer")]),
            registry_json(&[("bob@example.test", ISSUER, "Wrong identity")]),
            registry_json(&[("ALICE@example.test", ISSUER, "Case")]),
            registry_json(&[(IDENTITY, &ISSUER.to_uppercase(), "Case issuer")]),
            registry_json(&[("alice@example", ISSUER, "Prefix")]),
            registry_json(&[("alice@example.test.evil", ISSUER, "Longer")]),
            registry_json(&[("alice@.*", ISSUER, "Pattern")]),
            registry_json(&[(".*", ".*", "Wildcard")]),
        ];
        for registry in near_misses {
            commit_registry(&repo, "registry", &registry);
            let result = run(&fixture);
            assert_eq!(
                mapped(&result),
                json!({"source": "gitsign_registry", "mapping": "unmapped"}),
                "{registry}"
            );
            assert!(result["signer"].get("principal").is_none(), "{registry}");
            // The accepted identity is still reported.
            assert_eq!(result["certificate"]["identity"], IDENTITY);
        }
        commit_registry(&repo, "registry", &default_registry());
    }
}

#[test]
#[trace("TC-1042", "FR-040-AC-26")]
fn tc_1042_a_principal_in_gitsign_is_refused_before_git_starts_and_never_echoed() {
    let fixture = fixture_with_commit();
    let empty = fixture.root().join("empty");
    fs::create_dir(&empty).expect("empty");
    let secret = "Mallory Principal";
    for registry in [
        default_registry(),
        registry_json(&[("bob@example.test", ISSUER, "Bob")]),
    ] {
        commit_registry(&fixture.repo(), "registry", &registry);
        let mut body = request(&fixture, "signed", "commit", true);
        body["gitsign"]["principal"] = json!(secret);
        // No git on this PATH: exit 2 with the refusal proves git never started.
        let output = run(&fixture, body.to_string().as_bytes(), &empty);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let error: Value = serde_json::from_slice(&output.stdout).expect("machine error");
        assert_eq!(error["code"], "signature_verify_gitsign_principal_refused");
        let text = format!("{output:?}");
        assert!(!text.contains(secret), "{text}");
        // And with a working git, still no git call and no gitsign run.
        let output = run(&fixture, body.to_string().as_bytes(), &fixture.bin());
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(!format!("{output:?}").contains(secret));
    }
    assert!(!fixture.bin().join("gitlog").exists());
    assert_eq!(fixture.calls(), 0);
}

/// Every way an identity registry is unusable, as `(name, address)`;
/// [`break_registry`] builds the state that makes the registry unusable.
const UNUSABLE_REGISTRIES: [(&str, &str); 19] = [
    ("unresolved revision", "no-such-branch:registry.json"),
    ("a revision holding a colon", "HEAD@{12:00}"),
    ("absent path", "registry:missing.json"),
    ("symbolic link", REGISTRY),
    ("directory", REGISTRY),
    ("submodule", REGISTRY),
    ("over 1 MiB", REGISTRY),
    ("not UTF-8", REGISTRY),
    ("not JSON", REGISTRY),
    ("wrong protocol", REGISTRY),
    ("unknown top-level member", REGISTRY),
    ("unknown entry member", REGISTRY),
    ("empty value", REGISTRY),
    ("oversized value", REGISTRY),
    ("leading dash identity", REGISTRY),
    ("control character", REGISTRY),
    ("more than 4096 entries", REGISTRY),
    ("duplicate identity and issuer", REGISTRY),
    ("duplicate with the same principal", REGISTRY),
];

/// A registry document with one entry for the fixture identity and `extra`
/// members appended to the entry.
fn single_entry_registry(extra: &str) -> String {
    format!(
        r#"{{"protocol":"engineering-assurance.gitsign-identity-registry/v1","entries":[{{"certificate_identity":"alice@example.test","certificate_oidc_issuer":"{ISSUER}","principal":"Alice"{extra}}}]}}"#
    )
}

/// Makes the registry that [`UNUSABLE_REGISTRIES`] names `name` unusable.
fn break_registry(repo: &Path, name: &str) {
    let content = |text: &str| {
        commit_registry(repo, "registry", text);
    };
    match name {
        "symbolic link" => {
            // The link target is a valid registry document, so only the entry
            // mode can make the source unusable.
            let target = store(repo, "blob", &default_registry());
            commit_tree_listing(
                repo,
                "registry",
                &format!("120000 blob {target}\tregistry.json\n"),
            );
        }
        "directory" => {
            let blob = store(repo, "blob", "x");
            let subtree = String::from_utf8(run_program(
                repo,
                "git",
                &["mktree"],
                Some(format!("100644 blob {blob}\tinner\n").as_bytes()),
            ))
            .expect("utf-8")
            .trim()
            .to_owned();
            commit_tree_listing(
                repo,
                "registry",
                &format!("040000 tree {subtree}\tregistry.json\n"),
            );
        }
        "submodule" => {
            let head = git(repo, &["rev-parse", "HEAD"]).trim().to_owned();
            commit_tree_listing(
                repo,
                "registry",
                &format!("160000 commit {head}\tregistry.json\n"),
            );
        }
        "over 1 MiB" => {
            let mut bytes = default_registry().into_bytes();
            bytes.resize(1024 * 1024 + 1, b' ');
            commit_registry_bytes(repo, "registry", &bytes);
        }
        "not UTF-8" => {
            // A valid registry document except for invalid UTF-8 inside a value.
            let text = registry_json(&[(IDENTITY, ISSUER, "@@")]);
            let mut bytes = Vec::new();
            for (index, part) in text.split("@@").enumerate() {
                if index > 0 {
                    bytes.extend([0xff, 0xfe]);
                }
                bytes.extend(part.as_bytes());
            }
            commit_registry_bytes(repo, "registry", &bytes);
        }
        "not JSON" => content("entries: []"),
        "wrong protocol" => content(
            r#"{"protocol":"engineering-assurance.gitsign-identity-registry/v2","entries":[]}"#,
        ),
        "unknown top-level member" => content(
            r#"{"protocol":"engineering-assurance.gitsign-identity-registry/v1","entries":[],"extra":1}"#,
        ),
        "unknown entry member" => content(&single_entry_registry(r#","email":"a@example.test""#)),
        "empty value" => content(&single_entry_registry("").replace("\"Alice\"", "\"\"")),
        "oversized value" => {
            let long = format!("\"{}\"", "x".repeat(257));
            content(&single_entry_registry("").replace("\"Alice\"", &long));
        }
        "leading dash identity" => {
            content(&single_entry_registry("").replace("\"alice@", "\"-alice@"));
        }
        "control character" => {
            content(&single_entry_registry("").replace("\"Alice\"", "\"Al\\u0007ice\""));
        }
        "more than 4096 entries" => {
            let users: Vec<String> = (0..4097).map(|n| format!("user{n}@example.test")).collect();
            let entries: Vec<(&str, &str, &str)> = users
                .iter()
                .map(|user| (user.as_str(), ISSUER, "P"))
                .collect();
            content(&registry_json(&entries));
        }
        "duplicate identity and issuer" => content(&registry_json(&[
            (IDENTITY, ISSUER, "One"),
            (IDENTITY, ISSUER, "Two"),
        ])),
        "duplicate with the same principal" => content(&registry_json(&[
            (IDENTITY, ISSUER, "One"),
            (IDENTITY, ISSUER, "One"),
        ])),
        // The address alone is unusable.
        _ => {}
    }
}

/// A repository whose `v-ssh` tag and HEAD commit carry real SSH signatures, and
/// the PATH (real git and ssh-keygen plus the stand-in gitsign) that verifies them.
fn ssh_signed_fixture() -> (Fixture, PathBuf) {
    let (fixture, key) = ssh_fixture();
    let repo = fixture.repo();
    let setting = format!("user.signingkey={key}");
    git(
        &repo,
        &[
            "-c",
            "gpg.format=ssh",
            "-c",
            &setting,
            "commit",
            "-q",
            "-S",
            "--allow-empty",
            "-m",
            "s",
        ],
    );
    git(
        &repo,
        &[
            "-c",
            "gpg.format=ssh",
            "-c",
            &setting,
            "tag",
            "-s",
            "-m",
            "t",
            "v-ssh",
        ],
    );
    let path = fixture.root().join("ssh-bin");
    fs::create_dir(&path).expect("ssh-bin");
    // Recording wrappers: `git` logs its arguments to `gitlog` and `ssh-keygen`
    // to `sshlog`, both beside the wrappers, then run the real programs.
    for (program, log) in [("git", "gitlog"), ("ssh-keygen", "sshlog")] {
        let wrapper = format!(
            "#!/bin/sh\necho \"$*\" >> \"${{0%/*}}/{log}\"\nexec {} \"$@\"\n",
            which(program).display()
        );
        fs::write(path.join(program), wrapper).expect("wrapper");
        fs::set_permissions(path.join(program), fs::Permissions::from_mode(0o755)).expect("mode");
    }
    symlink(fixture.bin().join("gitsign"), path.join("gitsign")).expect("gitsign link");
    (fixture, path)
}

#[test]
#[trace("TC-1043", "TC-1038", "FR-040-AC-27", "FR-037-AC-20")]
fn tc_1043_an_unusable_identity_registry_is_refused_and_starts_no_verifier() {
    let cms = fixture_with_commit();
    forged_tag(&cms, "v-cms", "tag message", CMS_BLOCK);
    let (ssh, ssh_path) = ssh_signed_fixture();
    for (name, address) in UNUSABLE_REGISTRIES {
        // A CMS-signed commit and tag.
        break_registry(&cms.repo(), name);
        for (revision, kind) in [("signed", "commit"), ("v-cms", "tag")] {
            let mut body = request(&cms, revision, kind, true);
            body["identity_registry"] = json!(address);
            let error = machine_error(&cms, &body, &cms.bin());
            assert_eq!(
                error["code"], "signature_verify_identity_registry_invalid",
                "{name} {kind}"
            );
        }
        assert_eq!(cms.calls(), 0, "{name}: gitsign must not start");
        commit_registry(&cms.repo(), "registry", &default_registry());
        // An SSH-signed commit and tag in a request that also carries gitsign.
        break_registry(&ssh.repo(), name);
        for (revision, kind) in [("HEAD", "commit"), ("v-ssh", "tag")] {
            let mut body = request(&ssh, revision, kind, true);
            body["identity_registry"] = json!(address);
            let error = machine_error(&ssh, &body, &ssh_path);
            assert_eq!(
                error["code"], "signature_verify_identity_registry_invalid",
                "{name} ssh {kind}"
            );
        }
        commit_registry(&ssh.repo(), "registry", &default_registry());
    }
    assert_eq!(ssh.calls(), 0);
    // No SSH verifier started either: no git verification and no ssh-keygen.
    let git_log = fs::read_to_string(ssh_path.join("gitlog")).expect("git log");
    assert!(
        !git_log.contains("verify-commit") && !git_log.contains("verify-tag"),
        "{git_log}"
    );
    assert!(!ssh_path.join("sshlog").exists(), "ssh-keygen started");
}

#[test]
#[trace("TC-1044", "TC-1037", "FR-040-AC-28", "FR-037-AC-19")]
fn tc_1044_the_registry_is_read_from_the_committed_revision_not_the_working_tree() {
    let fixture = fixture_with_commit();
    let repo = fixture.repo();
    let address = "main:registry.json";
    let registry = repo.join("registry.json");
    let result = |fixture: &Fixture| verdict(fixture, &request_with_registry(fixture, address));
    let principal = |result: &Value| result["signer"]["mapping"].clone();

    // The committed registry lacks the entry; the working tree and the index hold it.
    fs::write(&registry, registry_json(&[])).expect("registry");
    git(&repo, &["add", "registry.json"]);
    git(&repo, &["commit", "-q", "-m", "registry without alice"]);
    fs::write(&registry, default_registry()).expect("edited registry");
    git(&repo, &["add", "registry.json"]);
    assert_eq!(principal(&result(&fixture)), "unmapped");

    // The committed registry holds the entry; the working tree and the index drop it.
    git(&repo, &["commit", "-q", "-m", "registry with alice"]);
    fs::write(&registry, registry_json(&[])).expect("edited registry");
    git(&repo, &["add", "registry.json"]);
    fs::write(&registry, "not even json").expect("edited again");
    let mapped = result(&fixture);
    assert_eq!(principal(&mapped), "mapped");
    assert_eq!(mapped["signer"]["principal"], PRINCIPAL);

    // A replace ref naming the commit does not change the content read.
    let committed = git(&repo, &["rev-parse", "main"]).trim().to_owned();
    let decoy = commit_registry(&repo, "decoy", &registry_json(&[]));
    git(
        &repo,
        &["update-ref", &format!("refs/replace/{committed}"), &decoy],
    );
    let replaced = result(&fixture);
    assert_eq!(replaced["signer"]["principal"], PRINCIPAL);
    assert_eq!(replaced["identity_registry_commit"], committed);
    git(&repo, &["replace", "-d", &committed]);

    // A graft changes neither the commit nor the content a revision that goes
    // through ancestry names: `main~1` is the registry without Alice, and a graft
    // gives `main` a parent whose registry names her. Honouring the graft would
    // resolve `main~1` to that decoy and map the signer.
    let ancestor = git(&repo, &["rev-parse", "main~1"]).trim().to_owned();
    let naming_alice = commit_registry(&repo, "decoy-with-alice", &default_registry());
    fs::write(
        repo.join(".git/info/grafts"),
        format!("{committed} {naming_alice}\n"),
    )
    .expect("grafts");
    let through_ancestry = verdict(
        &fixture,
        &request_with_registry(&fixture, "main~1:registry.json"),
    );
    assert_eq!(
        through_ancestry["signer"]["mapping"], "unmapped",
        "{through_ancestry}"
    );
    assert_eq!(through_ancestry["identity_registry_commit"], ancestor);
}

#[test]
#[trace("TC-1045", "FR-040-AC-29")]
fn tc_1045_the_mapped_principal_depends_only_on_the_registry_revision() {
    let fixture = fixture_with_commit();
    let repo = fixture.repo();
    commit_registry(
        &repo,
        "first",
        &registry_json(&[(IDENTITY, ISSUER, "First Person")]),
    );
    commit_registry(
        &repo,
        "second",
        &registry_json(&[(IDENTITY, ISSUER, "Second Person")]),
    );
    let principal = |address: &str| {
        let result = verdict(&fixture, &request_with_registry(&fixture, address));
        result["signer"]["principal"].clone()
    };
    assert_eq!(principal("first:registry.json"), "First Person");
    assert_eq!(principal("second:registry.json"), "Second Person");
    // An author, committer and message that name another registered person
    // change nothing.
    let impostor = forged_commit_by(
        &fixture,
        "Second Person <second@example.test>",
        CMS_BLOCK,
        "Second Person <second@example.test> 1700000000 +0000",
    );
    let mut body = request(&fixture, &impostor, "commit", true);
    body["identity_registry"] = json!("first:registry.json");
    let result = verdict(&fixture, &body);
    assert_eq!(result["signer"]["principal"], "First Person", "{result}");

    // A tag's tagger, and the certificate text of the signature block, change
    // nothing either.
    let repo = fixture.repo();
    let commit = git(&repo, &["rev-parse", "HEAD"]).trim().to_owned();
    for (name, tagger, block) in [
        ("v-a", "t <t@example.test> 1700000000 +0000", CMS_BLOCK),
        (
            "v-b",
            "Second Person <second@example.test> 1600000000 -0700",
            CMS_BLOCK,
        ),
        (
            "v-c",
            "t <t@example.test> 1700000000 +0000",
            "-----BEGIN SIGNED MESSAGE-----\nBBBBCCCCDDDD\n-----END SIGNED MESSAGE-----\n",
        ),
    ] {
        let object = format!(
            "object {commit}\ntype commit\ntag {name}\ntagger {tagger}\n\nSecond Person\n{block}"
        );
        let id = store(&repo, "tag", &object);
        git(&repo, &["update-ref", &format!("refs/tags/{name}"), &id]);
        let mut body = request(&fixture, name, "tag", true);
        body["identity_registry"] = json!("first:registry.json");
        let result = verdict(&fixture, &body);
        assert_eq!(
            result["signer"]["principal"], "First Person",
            "{name}: {result}"
        );
    }
}

#[test]
#[trace("TC-1046", "TC-1040", "FR-040-AC-30", "FR-037-AC-22")]
fn tc_1046_a_git_failure_while_reading_the_registry_is_unverifiable_not_invalid() {
    // Not a repository.
    let fixture = fixture_with_commit();
    let plain = fixture.root().join("plain");
    fs::create_dir(&plain).expect("plain directory");
    let mut body = request_with_registry(&fixture, REGISTRY);
    body["repository"] = json!(plain.display().to_string());
    let result = verdict(&fixture, &body);
    assert_eq!(result["status"], "unverifiable", "{result}");
    assert_eq!(result["reason"], "repository_unavailable");
    assert!(result.get("identity_registry_commit").is_none());

    // A git that cannot start.
    let nogit = fixture.root().join("nogit");
    fs::create_dir(&nogit).expect("empty PATH directory");
    let result = verdict_on(&fixture, &request_with_registry(&fixture, REGISTRY), &nogit);
    assert_eq!(result["status"], "unverifiable", "{result}");
    assert_eq!(result["reason"], "git_not_started");
    assert!(result.get("identity_registry_commit").is_none());
    assert_eq!(fixture.calls(), 0, "no verifier starts");

    // A git that exceeds its output bound while the registry is read.
    git_misbehaving_when(
        &fixture,
        "*ls-tree*",
        "/usr/bin/head -c 9000000 /dev/zero; exit 0",
    );
    let result = verdict(&fixture, &request_with_registry(&fixture, REGISTRY));
    assert_eq!(result["status"], "unverifiable", "{result}");
    assert_eq!(result["reason"], "output_exceeded");
    assert!(result.get("identity_registry_commit").is_none());

    assert_eq!(fixture.calls(), 0, "no verifier starts");
}

#[test]
#[trace("TC-1046", "TC-1040", "FR-040-AC-30", "FR-037-AC-22")]
fn tc_1046_a_git_that_times_out_while_reading_the_registry_is_unverifiable() {
    // The real 60 second process bound: no production knob shortens it.
    let fixture = fixture_with_commit();
    git_misbehaving_when(&fixture, "*ls-tree*", "exec /usr/bin/sleep 120");
    let result = verdict(&fixture, &request_with_registry(&fixture, REGISTRY));
    assert_eq!(result["status"], "unverifiable", "{result}");
    assert_eq!(result["reason"], "timeout");
    assert!(result.get("identity_registry_commit").is_none());
    assert_eq!(fixture.calls(), 0, "no verifier starts");
}

#[test]
#[trace("TC-1047", "TC-1039", "FR-040-AC-31", "FR-037-AC-21")]
fn tc_1047_every_verdict_carries_the_commit_the_registry_revision_resolved_to() {
    let fixture = fixture_with_commit();
    let repo = fixture.repo();
    forged_tag(&fixture, "v-cms", "tag message", CMS_BLOCK);
    git(&repo, &["tag", "light"]);
    git(&repo, &["tag", "-a", "-m", "plain", "v-plain"]);
    let registry_commit = tip(&repo, "registry");
    // An annotated tag as the revision is peeled to the commit it names.
    git(
        &repo,
        &[
            "tag",
            "-a",
            "-m",
            "registry release",
            "registry-tag",
            "registry",
        ],
    );

    let cases = [
        ("signed", "commit"),
        ("main", "commit"),
        ("nope", "commit"),
        ("v-cms", "tag"),
        ("v-plain", "tag"),
        ("light", "tag"),
        ("refs/tags/nope", "tag"),
    ];
    for address in [REGISTRY, "registry-tag:registry.json"] {
        for (revision, kind) in cases {
            let mut body = request(&fixture, revision, kind, true);
            body["identity_registry"] = json!(address);
            let result = verdict(&fixture, &body);
            assert_eq!(
                result["identity_registry_commit"], registry_commit,
                "{address} {revision}: {result}"
            );
        }
    }
    // Also with the gitsign member's absent object, SSH and failed verdicts.
    fixture.script("default", 1);
    let failed = verdict(&fixture, &request(&fixture, "signed", "commit", true));
    assert_eq!(failed["reason"], "gitsign_verification_failed");
    assert_eq!(failed["identity_registry_commit"], registry_commit);
    fixture.script("default", 0);

    // The commit and the mapping come from the revision as it resolved at the
    // reading, even when the branch moves while gitsign runs.
    let moved = commit_registry(&repo, "other-registry", &registry_json(&[]));
    let real_git_path = which("git");
    let real_git = real_git_path.display();
    let stand_in = STAND_IN.replace(
        "exit \"$status\"",
        &format!(
            "{real_git} -C '{}' update-ref refs/heads/registry '{moved}'\nexit \"$status\"",
            repo.display()
        ),
    );
    fs::write(fixture.bin().join("gitsign"), stand_in).expect("gitsign stand-in");
    let result = verdict(&fixture, &request(&fixture, "signed", "commit", true));
    assert_eq!(result["signer"]["principal"], PRINCIPAL, "{result}");
    assert_eq!(result["identity_registry_commit"], registry_commit);
    assert_eq!(
        tip(&repo, "registry"),
        moved,
        "the stand-in moved the branch"
    );
}

#[test]
#[trace("TC-1047", "FR-040-AC-31", "FR-040-AC-12")]
fn tc_1047_an_ssh_verdict_carries_the_registry_commit_and_its_unchanged_signer() {
    let (fixture, path) = ssh_signed_fixture();
    let registry_commit = tip(&fixture.repo(), "registry");
    for (revision, kind) in [("HEAD", "commit"), ("v-ssh", "tag")] {
        let result = verdict_on(&fixture, &request(&fixture, revision, kind, true), &path);
        assert_eq!(result["status"], "signed_valid", "{result}");
        assert_eq!(result["signature_kind"], "ssh");
        assert_eq!(result["signer"]["source"], "allowed_signers");
        assert_eq!(result["identity_registry_commit"], registry_commit);
    }
}

#[test]
#[trace("TC-1038", "FR-037-AC-20")]
fn tc_1038_an_ill_formed_address_never_starts_git_and_a_colon_revision_addresses_nothing() {
    let fixture = fixture_with_commit();
    let empty = fixture.root().join("empty");
    fs::create_dir(&empty).expect("empty");
    for address in [
        "",
        "registry",
        ":registry.json",
        "registry:",
        "-x:registry.json",
        "registry:/registry.json",
        "registry:a/../registry.json",
    ] {
        let body = request_with_registry(&fixture, address);
        assert_request_invalid(&fixture, &body, &empty);
    }
    // `HEAD@{12:00}` splits at its first colon: revision `HEAD@{12`, path `00}`.
    let body = request_with_registry(&fixture, "HEAD@{12:00}");
    let error = machine_error(&fixture, &body, &fixture.bin());
    assert_eq!(error["code"], "signature_verify_identity_registry_invalid");
}
