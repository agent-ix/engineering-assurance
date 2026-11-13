// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Git, `ssh-keygen` and `gitsign` adapter for the `signature-verify` command
//! (FR-036, FR-040).
//!
//! A CMS-signed object is never given to git: the adapter checks that the
//! repository sets no `gitsign.*` key, then runs `gitsign verify` or
//! `gitsign verify-tag` in the exit-status procedure the classifier drives.
//!
//! The adapter reads the allowed-signers file, then runs `git verify-commit`,
//! `git log --format=%G?%x00%GF%x00%GS`, `git verify-tag`, `git
//! interpret-trailers --parse` and, for a tag, `ssh-keygen -Y find-principals`
//! (git accepted it) or `ssh-keygen -Y check-novalidate` (git rejected it),
//! through the shared process host. It reads exit statuses
//! and fixed signals only, hands them to the pure classifier, and never writes
//! to the repository.

use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use engineering_assurance::signature_verify::{
    CommitObservation, GitGrade, GitsignIdentity, PrincipalLookup, Probe, Reason, Run,
    SignatureFormat, SignedOffMeaning, SplitTag, SshCheck, TagObservation, TargetKind, Verdict,
    classify_commit, classify_gitsign, classify_tag, commit_signature_format, is_plain_revision,
    object_message, signed_off_meaning, split_tag_object,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::process_host::{self, CompletedProcess, ProcessError, ProcessLimits};

const REQUEST_PROTOCOL: &str = "engineering-assurance.signature-verify-request/v1";
const RESULT_PROTOCOL: &str = "engineering-assurance.signature-verify-result/v1";
const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
/// Largest accepted allowed-signers file in bytes.
const MAX_ALLOWED_SIGNERS_BYTES: u64 = 1024 * 1024;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(60);

/// Repository-selecting and configuration variables git must not inherit.
const REMOVED_ENVIRONMENT: [&str; 12] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_EXTERNAL_DIFF",
    "GIT_DIFF_OPTS",
    "GIT_NAMESPACE",
    "GIT_COMMON_DIR",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_REPLACE_REF_BASE",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    repository: PathBuf,
    revision: String,
    kind: TargetKind,
    allowed_signers: PathBuf,
    gitsign: Option<GitsignRequest>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GitsignRequest {
    certificate_identity: String,
    certificate_oidc_issuer: String,
    principal: String,
}

#[derive(Serialize)]
struct ResultLine<'a> {
    protocol: &'static str,
    #[serde(flatten)]
    verdict: &'a Verdict,
}

/// Typed refusal that produces no verdict.
#[derive(Debug, Error)]
pub(crate) enum SignatureVerifyError {
    /// The request is not readable JSON.
    #[error("signature-verify request is not readable JSON")]
    Unreadable,
    /// The request is JSON that is not an exact request document.
    #[error("signature-verify request is not an exact signature-verify request")]
    Invalid,
    /// The revision is empty, too long, option-like or contains whitespace or controls.
    #[error("signature-verify revision is not a plain revision name")]
    Revision,
    /// The allowed-signers file is not a readable regular UTF-8 file of at most 1 MiB.
    #[error(
        "signature-verify allowed-signers file is not a readable UTF-8 regular file of at most 1 MiB"
    )]
    AllowedSigners,
    /// The verdict cannot be encoded.
    #[error("signature-verify result cannot be encoded")]
    ResultEncoding,
}

impl SignatureVerifyError {
    /// Stable machine error code.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "signature_verify_request_unreadable",
            Self::Invalid => "signature_verify_request_invalid",
            Self::Revision => "signature_verify_revision_invalid",
            Self::AllowedSigners => "signature_verify_allowed_signers_invalid",
            Self::ResultEncoding => "signature_verify_result_encoding_failed",
        }
    }
}

/// Runs one request document and returns the result line.
pub(crate) fn execute(bytes: &[u8]) -> Result<Vec<u8>, SignatureVerifyError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| SignatureVerifyError::Unreadable)?;
    let request: Request =
        serde_json::from_value(value).map_err(|_| SignatureVerifyError::Invalid)?;
    if request.protocol != REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.allowed_signers.is_absolute()
    {
        return Err(SignatureVerifyError::Invalid);
    }
    if !is_plain_revision(&request.revision) {
        return Err(SignatureVerifyError::Revision);
    }
    let gitsign = request
        .gitsign
        .map(|g| {
            GitsignIdentity::new(
                g.certificate_identity,
                g.certificate_oidc_issuer,
                g.principal,
            )
            .ok_or(SignatureVerifyError::Invalid)
        })
        .transpose()?;
    check_allowed_signers(&request.allowed_signers)?;
    let git = Git {
        repository: &request.repository,
        allowed_signers: &request.allowed_signers,
        gitsign: gitsign.as_ref(),
    };
    let verdict = match request.kind {
        TargetKind::Commit => git.commit(&request.revision),
        TargetKind::Tag => git.tag(&request.revision),
    };
    let mut line = serde_json::to_vec(&ResultLine {
        protocol: RESULT_PROTOCOL,
        verdict: &verdict,
    })
    .map_err(|_| SignatureVerifyError::ResultEncoding)?;
    line.push(b'\n');
    Ok(line)
}

/// Refuses an allowed-signers file git should not be given: one that is not a
/// regular file, is over 1 MiB, cannot be read, or is not UTF-8. The grammar of
/// the file is git's and `ssh-keygen`'s to read.
fn check_allowed_signers(path: &Path) -> Result<(), SignatureVerifyError> {
    let invalid = |_| SignatureVerifyError::AllowedSigners;
    // The type is checked before the open: opening a FIFO would block.
    let before = std::fs::metadata(path).map_err(invalid)?;
    if !before.is_file() || before.len() > MAX_ALLOWED_SIGNERS_BYTES {
        return Err(SignatureVerifyError::AllowedSigners);
    }
    let file = File::open(path).map_err(invalid)?;
    let metadata = file.metadata().map_err(invalid)?;
    if !metadata.is_file() || metadata.len() > MAX_ALLOWED_SIGNERS_BYTES {
        return Err(SignatureVerifyError::AllowedSigners);
    }
    let mut bytes = Vec::new();
    file.take(MAX_ALLOWED_SIGNERS_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(invalid)?;
    if u64::try_from(bytes.len()).map_or(true, |len| len > MAX_ALLOWED_SIGNERS_BYTES)
        || std::str::from_utf8(&bytes).is_err()
    {
        return Err(SignatureVerifyError::AllowedSigners);
    }
    Ok(())
}

/// Maps a process-mechanics failure to the reason it makes the object
/// unverifiable; `not_started` is the reason for a program that cannot start.
const fn mechanics_reason(error: &ProcessError, not_started: Reason) -> Reason {
    match error {
        ProcessError::Unavailable { .. } => not_started,
        ProcessError::TimedOut { .. } => Reason::Timeout,
        ProcessError::OutputTooLarge { .. } => Reason::OutputExceeded,
        ProcessError::PipeUnavailable { .. }
        | ProcessError::Observation { .. }
        | ProcessError::OutputUnreadable { .. } => Reason::UnrecognizedOutput,
    }
}

struct Git<'a> {
    repository: &'a Path,
    allowed_signers: &'a Path,
    gitsign: Option<&'a GitsignIdentity>,
}

/// Name of the verifier program for CMS signatures; never taken from input.
const GITSIGN_PROGRAM: &str = "gitsign";

/// Environment variables the tools that read the repository's trust settings
/// are given.
const TOOL_ENVIRONMENT: [&str; 4] = ["PATH", "HOME", "GIT_CONFIG_NOSYSTEM", "GIT_CONFIG_GLOBAL"];

/// The exact environment of `gitsign` and of the configuration check: the
/// inherited `PATH` and `HOME`, no system or global git configuration, and
/// nothing else, so the returned removal list holds every other inherited name.
fn tool_environment() -> (Vec<(OsString, OsString)>, Vec<OsString>) {
    let set = vec![
        (OsString::from("GIT_CONFIG_NOSYSTEM"), OsString::from("1")),
        (
            OsString::from("GIT_CONFIG_GLOBAL"),
            OsString::from("/dev/null"),
        ),
    ];
    let removed = std::env::vars_os()
        .map(|(name, _)| name)
        .filter(|name| !TOOL_ENVIRONMENT.iter().any(|kept| OsStr::new(kept) == name))
        .collect();
    (set, removed)
}

/// What the process host observed of a delegated run, as the classifier reads it.
fn observed(result: &Result<CompletedProcess, ProcessError>) -> Run {
    match result {
        Ok(output) => Run::Exit(output.status.code().unwrap_or(-1)),
        Err(ProcessError::Unavailable { .. }) => Run::NotStarted,
        Err(ProcessError::TimedOut { .. }) => Run::TimedOut,
        Err(ProcessError::OutputTooLarge { .. }) => Run::CutShort,
        Err(
            ProcessError::PipeUnavailable { .. }
            | ProcessError::Observation { .. }
            | ProcessError::OutputUnreadable { .. },
        ) => Run::Unobserved,
    }
}

impl Git<'_> {
    fn commit(&self, revision: &str) -> Verdict {
        let kind = TargetKind::Commit;
        let (id, object) = match self.load_commit(revision) {
            Ok(loaded) => loaded,
            Err(reason) => return Verdict::unverifiable(kind, reason),
        };
        let meaning = self.meaning(object_message(&object));
        let format = commit_signature_format(&object);
        self.commit_verdict(&id, format)
            .unwrap_or_else(|reason| {
                Verdict::unverifiable(kind, reason)
                    .with_signature_kind(format.and_then(SignatureFormat::signature_kind))
            })
            .with_meaning(meaning)
    }

    fn load_commit(&self, revision: &str) -> Result<(String, Vec<u8>), Reason> {
        self.require_repository()?;
        let id = self.resolve(&format!("{revision}^{{commit}}"))?;
        let object = self.successful(&["cat-file", "commit", &id])?;
        Ok((id, object))
    }

    fn commit_verdict(&self, id: &str, format: Option<SignatureFormat>) -> Result<Verdict, Reason> {
        let git = match format {
            Some(SignatureFormat::Cms) => {
                return self.gitsign_verdict(TargetKind::Commit, id);
            }
            Some(SignatureFormat::Other) => None,
            Some(SignatureFormat::Ssh) | None => Some(self.grade(id)?),
        };
        Ok(classify_commit(&CommitObservation { format, git }))
    }

    /// Delegates a CMS-signed object to `gitsign`; `target` is the commit id or
    /// the bare tag name. Without a supplied identity nothing is started.
    fn gitsign_verdict(&self, kind: TargetKind, target: &str) -> Result<Verdict, Reason> {
        let sets_key = match self.gitsign {
            Some(_) => self.repository_sets_gitsign_key()?,
            None => false,
        };
        Ok(classify_gitsign(kind, self.gitsign, sets_key, |probe| {
            self.gitsign_run(kind, target, probe)
        }))
    }

    /// Whether the configuration `gitsign` will see sets any `gitsign.*` key,
    /// asked of git with no setting added, in `gitsign`'s own environment.
    fn repository_sets_gitsign_key(&self) -> Result<bool, Reason> {
        let (set, removed) = tool_environment();
        let set: Vec<(&OsStr, &OsStr)> = set
            .iter()
            .map(|(k, v)| (k.as_os_str(), v.as_os_str()))
            .collect();
        let removed: Vec<&OsStr> = removed.iter().map(OsString::as_os_str).collect();
        let arguments = [
            OsStr::new("config"),
            OsStr::new("--get-regexp"),
            OsStr::new("^gitsign\\."),
        ];
        let output = process_host::run_configured(
            OsStr::new("git"),
            &arguments,
            Some(self.repository),
            &set,
            &removed,
            limits(),
        )
        .map_err(|error| mechanics_reason(&error, Reason::GitNotStarted))?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(Reason::UnrecognizedOutput),
        }
    }

    /// One `gitsign` invocation of the procedure, as separate arguments and
    /// never through a shell.
    fn gitsign_run(&self, kind: TargetKind, target: &str, probe: Probe) -> Run {
        let Some(identity) = self.gitsign else {
            return Run::NotStarted;
        };
        let (identity_flag, identity_value) = match probe {
            Probe::Exact | Probe::ExactIdentityAnyIssuer => {
                ("--certificate-identity", identity.certificate_identity())
            }
            Probe::AnyIdentityAnyIssuer | Probe::AnyIdentityExactIssuer => {
                ("--certificate-identity-regexp", ".*")
            }
        };
        let (issuer_flag, issuer_value) = match probe {
            Probe::Exact | Probe::AnyIdentityExactIssuer => (
                "--certificate-oidc-issuer",
                identity.certificate_oidc_issuer(),
            ),
            Probe::AnyIdentityAnyIssuer | Probe::ExactIdentityAnyIssuer => {
                ("--certificate-oidc-issuer-regexp", ".*")
            }
        };
        let subcommand = match kind {
            TargetKind::Commit => "verify",
            TargetKind::Tag => "verify-tag",
        };
        let arguments = [
            subcommand,
            identity_flag,
            identity_value,
            issuer_flag,
            issuer_value,
            target,
        ]
        .map(OsStr::new);
        let (set, removed) = tool_environment();
        let set: Vec<(&OsStr, &OsStr)> = set
            .iter()
            .map(|(k, v)| (k.as_os_str(), v.as_os_str()))
            .collect();
        let removed: Vec<&OsStr> = removed.iter().map(OsString::as_os_str).collect();
        observed(&process_host::run_configured(
            OsStr::new(GITSIGN_PROGRAM),
            &arguments,
            Some(self.repository),
            &set,
            &removed,
            limits(),
        ))
    }

    fn grade(&self, id: &str) -> Result<GitGrade, Reason> {
        let verify_accepted = self.run(&["verify-commit", id])?.status.success();
        let log = self.successful(&[
            "log",
            "-1",
            "--no-show-signature",
            "--format=%G?%x00%GF%x00%GS",
            id,
        ])?;
        let text = String::from_utf8(log).map_err(|_| Reason::UnrecognizedOutput)?;
        let mut fields = text.trim_end_matches('\n').split('\0');
        let (Some(grade), Some(fingerprint), Some(principal), None) =
            (fields.next(), fields.next(), fields.next(), fields.next())
        else {
            return Err(Reason::UnrecognizedOutput);
        };
        let mut codes = grade.chars();
        let (Some(grade), None) = (codes.next(), codes.next()) else {
            return Err(Reason::UnrecognizedOutput);
        };
        Ok(GitGrade {
            verify_accepted,
            grade,
            fingerprint: (!fingerprint.is_empty()).then(|| fingerprint.to_owned()),
            principal: (!principal.is_empty()).then(|| principal.to_owned()),
        })
    }

    fn tag(&self, revision: &str) -> Verdict {
        let kind = TargetKind::Tag;
        let loaded = self.load_tag(revision);
        let (id, object) = match loaded {
            Ok(Some(loaded)) => loaded,
            Ok(None) => return classify_tag(&TagObservation::Lightweight),
            Err(reason) => return Verdict::unverifiable(kind, reason),
        };
        let split = split_tag_object(&object);
        let payload = split.as_ref().map_or(object.as_slice(), |s| s.payload);
        let meaning = self.meaning(object_message(payload));
        let format = split.as_ref().map(|s| s.format);
        self.tag_verdict(revision, &id, split.as_ref())
            .unwrap_or_else(|reason| {
                Verdict::unverifiable(kind, reason)
                    .with_signature_kind(format.and_then(SignatureFormat::signature_kind))
            })
            .with_meaning(meaning)
    }

    /// The id and object of an annotated tag; `None` for a lightweight tag.
    fn load_tag(&self, revision: &str) -> Result<Option<(String, Vec<u8>)>, Reason> {
        self.require_repository()?;
        let id = self.resolve(revision)?;
        let kind = self.successful(&["cat-file", "-t", &id])?;
        match kind.as_slice() {
            b"commit\n" if self.names_tag_ref(revision)? => Ok(None),
            b"tag\n" => {
                let object = self.successful(&["cat-file", "tag", &id])?;
                Ok(Some((id, object)))
            }
            _ => Err(Reason::RevisionUnresolved),
        }
    }

    fn tag_verdict(
        &self,
        revision: &str,
        id: &str,
        split: Option<&SplitTag<'_>>,
    ) -> Result<Verdict, Reason> {
        if split.is_some_and(|split| split.format == SignatureFormat::Cms) {
            let name = self.tag_name(revision, id)?;
            return self.gitsign_verdict(TargetKind::Tag, name);
        }
        let observation = match split {
            Some(split) => self.annotated_tag(id, split)?,
            None => TagObservation::NoSignature,
        };
        Ok(classify_tag(&observation))
    }

    /// The bare name of the tag `revision` names: `refs/tags/<name>` must be a
    /// valid ref that resolves to the tag object `id`.
    fn tag_name<'r>(&self, revision: &'r str, id: &str) -> Result<&'r str, Reason> {
        let name = revision.strip_prefix("refs/tags/").unwrap_or(revision);
        let full = format!("refs/tags/{name}");
        let valid = self.run(&["check-ref-format", &full])?.status.success();
        if valid && self.resolve(&full).is_ok_and(|resolved| resolved == id) {
            Ok(name)
        } else {
            Err(Reason::RevisionUnresolved)
        }
    }

    /// Whether `revision` names a ref under `refs/tags/`.
    fn names_tag_ref(&self, revision: &str) -> Result<bool, Reason> {
        // The revision cannot begin with `-` (checked before git starts), and git
        // 2.34 echoes `--end-of-options` under `--symbolic-full-name`.
        let output = self.successful(&["rev-parse", "--symbolic-full-name", revision])?;
        Ok(output.starts_with(b"refs/tags/"))
    }

    fn annotated_tag(&self, id: &str, split: &SplitTag<'_>) -> Result<TagObservation, Reason> {
        if split.format == SignatureFormat::Other {
            return Ok(TagObservation::Signature {
                format: SignatureFormat::Other,
                verify_accepted: false,
                check: None,
                principal: PrincipalLookup::NotRun,
            });
        }
        let verify_accepted = self.run(&["verify-tag", id])?.status.success();
        let (check, principal) = if verify_accepted {
            (None, self.find_principal(split.signature)?)
        } else {
            (
                Some(ssh_check(split.signature, split.payload)?),
                PrincipalLookup::NotRun,
            )
        };
        Ok(TagObservation::Signature {
            format: SignatureFormat::Ssh,
            verify_accepted,
            check,
            principal,
        })
    }

    /// The first principal `ssh-keygen -Y find-principals` prints for `signature`.
    fn find_principal(&self, signature: &[u8]) -> Result<PrincipalLookup, Reason> {
        let directory = tempfile::tempdir().map_err(|_| Reason::UnrecognizedOutput)?;
        let signature_path = directory.path().join("signature");
        std::fs::write(&signature_path, signature).map_err(|_| Reason::UnrecognizedOutput)?;
        let arguments = [
            OsStr::new("-Y"),
            OsStr::new("find-principals"),
            OsStr::new("-s"),
            signature_path.as_os_str(),
            OsStr::new("-f"),
            self.allowed_signers.as_os_str(),
        ];
        let result = process_host::run_configured(
            OsStr::new("ssh-keygen"),
            &arguments,
            Some(directory.path()),
            &[],
            &[],
            limits(),
        );
        match result {
            Ok(output) if output.status.success() => {
                let first = std::str::from_utf8(&output.stdout)
                    .ok()
                    .and_then(|text| text.lines().next())
                    .filter(|line| !line.is_empty());
                Ok(first.map_or(PrincipalLookup::Rejected, |line| {
                    PrincipalLookup::Found(line.to_owned())
                }))
            }
            Ok(_) => Ok(PrincipalLookup::Rejected),
            Err(ProcessError::Unavailable { .. }) => Ok(PrincipalLookup::NotStarted),
            Err(error) => Err(mechanics_reason(&error, Reason::UnrecognizedOutput)),
        }
    }

    /// The `Signed-off-meaning:` trailers of `message`, read by
    /// `git interpret-trailers --parse --no-divider`; `None` when git could not
    /// read them. Git runs in an empty directory it may not leave, so the
    /// verified repository's configuration cannot change what is read.
    fn meaning(&self, message: &[u8]) -> Option<SignedOffMeaning> {
        let directory = tempfile::tempdir().ok()?;
        let path = directory.path().join("message");
        std::fs::write(&path, message).ok()?;
        let output = self
            .run_with_stdin(
                &["interpret-trailers", "--parse", "--no-divider"],
                Some(File::open(&path).ok()?),
                Some(directory.path()),
            )
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        Some(signed_off_meaning(&text))
    }

    fn require_repository(&self) -> Result<(), Reason> {
        if !self.repository.is_dir() {
            return Err(Reason::RepositoryUnavailable);
        }
        if self.run(&["rev-parse", "--git-dir"])?.status.success() {
            Ok(())
        } else {
            Err(Reason::RepositoryUnavailable)
        }
    }

    /// Resolves `expression` to a full object id.
    fn resolve(&self, expression: &str) -> Result<String, Reason> {
        let output = self.run(&[
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            expression,
        ])?;
        let id = output
            .status
            .success()
            .then(|| String::from_utf8(output.stdout).ok())
            .flatten()
            .map(|text| text.trim_end_matches('\n').to_owned())
            .filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_hexdigit()));
        id.ok_or(Reason::RevisionUnresolved)
    }

    /// Standard output of a git command that must exit with status 0.
    fn successful(&self, arguments: &[&str]) -> Result<Vec<u8>, Reason> {
        let output = self.run(arguments)?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(Reason::UnrecognizedOutput)
        }
    }

    /// Runs git with only the allowed-signers setting from the request.
    fn run(&self, arguments: &[&str]) -> Result<CompletedProcess, Reason> {
        self.run_with_stdin(arguments, None, None)
    }

    /// Runs git with `stdin` as standard input when given. With `isolated`, git
    /// runs in that directory and, with the ceiling on its parent, cannot
    /// discover a repository there or above it (git ignores a ceiling equal to
    /// the working directory itself).
    fn run_with_stdin(
        &self,
        arguments: &[&str],
        stdin: Option<File>,
        isolated: Option<&Path>,
    ) -> Result<CompletedProcess, Reason> {
        let mut setting = OsString::from("gpg.ssh.allowedSignersFile=");
        setting.push(self.allowed_signers);
        let mut all: Vec<&OsStr> = vec![
            OsStr::new("-c"),
            &setting,
            OsStr::new("-c"),
            OsStr::new("gpg.ssh.program=ssh-keygen"),
            OsStr::new("-c"),
            OsStr::new("gpg.program=false"),
            OsStr::new("-c"),
            OsStr::new("gpg.openpgp.program=false"),
            OsStr::new("-c"),
            OsStr::new("gpg.x509.program=false"),
            OsStr::new("-c"),
            OsStr::new("trailer.separators=:"),
        ];
        all.extend(arguments.iter().map(OsStr::new));
        let removed: Vec<&OsStr> = REMOVED_ENVIRONMENT.iter().map(OsStr::new).collect();
        let mut environment: Vec<(&OsStr, &OsStr)> = vec![
            (OsStr::new("GIT_CONFIG_NOSYSTEM"), OsStr::new("1")),
            (OsStr::new("GIT_CONFIG_GLOBAL"), OsStr::new("/dev/null")),
            (OsStr::new("GIT_OPTIONAL_LOCKS"), OsStr::new("0")),
        ];
        if let Some(parent) = isolated.and_then(Path::parent) {
            environment.push((OsStr::new("GIT_CEILING_DIRECTORIES"), parent.as_os_str()));
        }
        let directory = isolated.unwrap_or(self.repository);
        match stdin {
            Some(stdin) => process_host::run_configured_stdin(
                OsStr::new("git"),
                &all,
                Some(directory),
                &environment,
                &removed,
                stdin,
                limits(),
            ),
            None => process_host::run_configured(
                OsStr::new("git"),
                &all,
                Some(directory),
                &environment,
                &removed,
                limits(),
            ),
        }
        .map_err(|error| mechanics_reason(&error, Reason::GitNotStarted))
    }
}

const fn limits() -> ProcessLimits {
    ProcessLimits {
        timeout: PROCESS_TIMEOUT,
        max_output_bytes: MAX_OUTPUT_BYTES,
    }
}

/// Checks `signature` over `payload` with `ssh-keygen -Y check-novalidate`,
/// passing both parts unchanged.
fn ssh_check(signature: &[u8], payload: &[u8]) -> Result<SshCheck, Reason> {
    let directory = tempfile::tempdir().map_err(|_| Reason::UnrecognizedOutput)?;
    let signature_path = directory.path().join("signature");
    let payload_path = directory.path().join("payload");
    std::fs::write(&signature_path, signature).map_err(|_| Reason::UnrecognizedOutput)?;
    std::fs::write(&payload_path, payload).map_err(|_| Reason::UnrecognizedOutput)?;
    let stdin = File::open(&payload_path).map_err(|_| Reason::UnrecognizedOutput)?;
    let arguments = [
        OsStr::new("-Y"),
        OsStr::new("check-novalidate"),
        OsStr::new("-n"),
        OsStr::new("git"),
        OsStr::new("-s"),
        signature_path.as_os_str(),
    ];
    let result = process_host::run_configured_stdin(
        OsStr::new("ssh-keygen"),
        &arguments,
        Some(directory.path()),
        &[],
        &[],
        stdin,
        limits(),
    );
    match result {
        Ok(output) if output.status.success() => Ok(SshCheck::Accepted),
        Ok(_) => Ok(SshCheck::Rejected),
        Err(ProcessError::Unavailable { .. }) => Ok(SshCheck::NotStarted),
        Err(error) => Err(mechanics_reason(&error, Reason::UnrecognizedOutput)),
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    #[test]
    #[trace("TC-295", "FR-040-AC-7")]
    fn gitsign_process_failures_are_observed_as_runs() {
        use engineering_assurance::signature_verify::Run;
        let failure = |error| observed(&Err(error));
        assert_eq!(
            failure(ProcessError::TimedOut {
                timeout: PROCESS_TIMEOUT
            }),
            Run::TimedOut
        );
        assert_eq!(
            failure(ProcessError::OutputTooLarge {
                stream: "stdout",
                limit: MAX_OUTPUT_BYTES
            }),
            Run::CutShort
        );
        assert_eq!(
            failure(ProcessError::Unavailable {
                detail: "missing".to_owned()
            }),
            Run::NotStarted
        );
        assert_eq!(
            failure(ProcessError::PipeUnavailable { stream: "stdout" }),
            Run::Unobserved
        );
    }

    #[test]
    #[trace("TC-255", "FR-036-AC-6")]
    fn timeout_and_oversized_output_are_unverifiable_reasons() {
        let timed_out = mechanics_reason(
            &ProcessError::TimedOut {
                timeout: PROCESS_TIMEOUT,
            },
            Reason::GitNotStarted,
        );
        assert_eq!(timed_out, Reason::Timeout);
        let oversized = mechanics_reason(
            &ProcessError::OutputTooLarge {
                stream: "stdout",
                limit: MAX_OUTPUT_BYTES,
            },
            Reason::GitNotStarted,
        );
        assert_eq!(oversized, Reason::OutputExceeded);
        let not_started = mechanics_reason(
            &ProcessError::Unavailable {
                detail: "missing".to_owned(),
            },
            Reason::GitNotStarted,
        );
        assert_eq!(not_started, Reason::GitNotStarted);
    }
}
