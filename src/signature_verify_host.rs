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
    SignatureFormat, SignedAt, SignedOffMeaning, SplitTag, SshCheck, TagObservation, TargetKind,
    Verdict, classify_commit, classify_gitsign, classify_tag, commit_signature_format,
    config_sets_gitsign_key, is_plain_revision, object_message, signed_off_meaning,
    split_tag_object, tagger_signed_at,
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

/// The `gitsign` member of a request, as the request documents spell it.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GitsignRequest {
    pub(crate) certificate_identity: String,
    pub(crate) certificate_oidc_issuer: String,
    pub(crate) principal: String,
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
pub(crate) fn check_allowed_signers(path: &Path) -> Result<(), SignatureVerifyError> {
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

/// A tag's verdict and the trailer block its message was read to hold.
pub(crate) struct CheckedTag {
    /// The FR-036, FR-037 and FR-040 verdict.
    pub(crate) verdict: Verdict,
    /// The output of `git interpret-trailers --parse` for the tag message;
    /// `None` when the tag has no message to read or git could not read it.
    pub(crate) trailers: Option<String>,
    /// The UTC instant the tag's tagger line claims; `None` when the tag has no
    /// readable tagger line.
    pub(crate) tagger_signed_at: Option<SignedAt>,
}

/// Verifies the tag `revision` exactly as `signature-verify` does for kind
/// `tag`, and returns the parsed trailer block read for the same verdict.
pub(crate) fn check_tag(
    repository: &Path,
    revision: &str,
    allowed_signers: &Path,
    gitsign: Option<&GitsignIdentity>,
) -> CheckedTag {
    Git {
        repository,
        allowed_signers,
        gitsign,
    }
    .tag_checked(revision)
}

struct Git<'a> {
    repository: &'a Path,
    allowed_signers: &'a Path,
    gitsign: Option<&'a GitsignIdentity>,
}

/// Name of the verifier program for CMS signatures; never taken from input.
const GITSIGN_PROGRAM: &str = "gitsign";

/// Environment variables `gitsign` and the configuration check keep from the
/// parent; `GIT_CONFIG_NOSYSTEM` and `GIT_CONFIG_GLOBAL` are set, not kept.
const KEPT_ENVIRONMENT: [&str; 2] = ["PATH", "HOME"];

/// The absolute entries of `path`, which are the only directories a verifier
/// program is looked up in: a relative entry would resolve inside the
/// repository under verification, the working directory of the run.
fn absolute_search_path(path: &OsStr) -> OsString {
    let kept: Vec<PathBuf> = std::env::split_paths(path)
        .filter(|entry| entry.is_absolute())
        .collect();
    std::env::join_paths(kept).unwrap_or_default()
}

/// The executable `name` in the first directory of `search` that holds one.
fn find_executable(name: &str, search: &OsStr) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    std::env::split_paths(search)
        .map(|directory| directory.join(name))
        .find(|candidate| {
            std::fs::metadata(candidate)
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
}

/// Finds `program` for a run: its absolute path and the `PATH` to pass on. Both
/// come from the absolute entries of the inherited `PATH` alone, so a program
/// in the verified repository (the working directory) is never found, whether
/// we start it or a program we start does.
fn resolve_program(program: &str) -> Result<(PathBuf, OsString), ProcessError> {
    let unavailable = || ProcessError::Unavailable {
        detail: format!("{program} is not in an absolute PATH entry"),
    };
    let search = std::env::var_os("PATH")
        .map(|path| absolute_search_path(&path))
        .ok_or_else(unavailable)?;
    let executable = find_executable(program, &search).ok_or_else(unavailable)?;
    Ok((executable, search))
}

/// Runs `program` as `gitsign` and the configuration check need it run: found
/// by absolute path in the absolute entries of the inherited `PATH`, in the
/// repository, with exactly `PATH` (those entries), `HOME`,
/// `GIT_CONFIG_NOSYSTEM=1` and `GIT_CONFIG_GLOBAL=/dev/null`.
fn run_tool(
    program: &str,
    arguments: &[&OsStr],
    repository: &Path,
) -> Result<CompletedProcess, ProcessError> {
    let (executable, search) = resolve_program(program)?;
    let set: Vec<(OsString, OsString)> = vec![
        ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
        ("GIT_CONFIG_GLOBAL".into(), "/dev/null".into()),
        ("PATH".into(), search),
    ];
    let removed: Vec<OsString> = std::env::vars_os()
        .map(|(name, _)| name)
        .filter(|name| !KEPT_ENVIRONMENT.iter().any(|kept| OsStr::new(kept) == name))
        .filter(|name| !set.iter().any(|(set_name, _)| set_name == name))
        .collect();
    let set: Vec<(&OsStr, &OsStr)> = set
        .iter()
        .map(|(name, value)| (name.as_os_str(), value.as_os_str()))
        .collect();
    let removed: Vec<&OsStr> = removed.iter().map(OsString::as_os_str).collect();
    process_host::run_configured(
        executable.as_os_str(),
        arguments,
        Some(repository),
        &set,
        &removed,
        limits(),
    )
}

/// What the process host observed of a delegated run, as the classifier reads it.
fn observed(result: &Result<CompletedProcess, ProcessError>) -> Run {
    match result {
        Ok(output) => output.status.code().map_or(Run::Signaled, Run::Exit),
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
        let Some(identity) = self.gitsign else {
            // The classifier asks for no probe without an identity.
            return Ok(classify_gitsign(kind, None, false, |_| Run::Unobserved));
        };
        let sets_key = self.repository_sets_gitsign_key()?;
        Ok(classify_gitsign(kind, Some(identity), sets_key, |probe| {
            self.gitsign_run(identity, kind, target, probe)
        }))
    }

    /// Whether the configuration `gitsign` will see sets any `gitsign.*` key,
    /// asked as gitsign asks it, `git config --get-regexp .*` with no setting
    /// added, in `gitsign`'s own environment, and read as gitsign reads it.
    fn repository_sets_gitsign_key(&self) -> Result<bool, Reason> {
        let arguments = ["config", "--get-regexp", ".*"].map(OsStr::new);
        let output = run_tool("git", &arguments, self.repository)
            .map_err(|error| mechanics_reason(&error, Reason::GitNotStarted))?;
        match output.status.code() {
            Some(0) => Ok(config_sets_gitsign_key(&output.stdout)),
            Some(1) => Ok(false),
            _ => Err(Reason::UnrecognizedOutput),
        }
    }

    /// One `gitsign` invocation of the procedure, as separate arguments and
    /// never through a shell.
    fn gitsign_run(
        &self,
        identity: &GitsignIdentity,
        kind: TargetKind,
        target: &str,
        probe: Probe,
    ) -> Run {
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
            "--",
            target,
        ]
        .map(OsStr::new);
        observed(&run_tool(GITSIGN_PROGRAM, &arguments, self.repository))
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
        self.tag_checked(revision).verdict
    }

    fn tag_checked(&self, revision: &str) -> CheckedTag {
        let kind = TargetKind::Tag;
        let without_message = |verdict| CheckedTag {
            verdict,
            trailers: None,
            tagger_signed_at: None,
        };
        let loaded = self.load_tag(revision);
        let (id, object) = match loaded {
            Ok(Some(loaded)) => loaded,
            Ok(None) => return without_message(classify_tag(&TagObservation::Lightweight)),
            Err(reason) => return without_message(Verdict::unverifiable(kind, reason)),
        };
        let split = split_tag_object(&object);
        let payload = split.as_ref().map_or(object.as_slice(), |s| s.payload);
        let trailers = self.parsed_trailers(object_message(payload));
        let meaning = trailers.as_deref().map(signed_off_meaning);
        let format = split.as_ref().map(|s| s.format);
        let verdict = self
            .tag_verdict(revision, &id, split.as_ref())
            .unwrap_or_else(|reason| {
                Verdict::unverifiable(kind, reason)
                    .with_signature_kind(format.and_then(SignatureFormat::signature_kind))
            })
            .with_meaning(meaning);
        CheckedTag {
            verdict,
            trailers,
            tagger_signed_at: tagger_signed_at(payload),
        }
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
        // `gitsign` would read such a name as an option.
        if name.starts_with('-') {
            return Err(Reason::RevisionUnresolved);
        }
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
        let result = resolve_program("ssh-keygen").and_then(|(executable, path)| {
            process_host::run_configured(
                executable.as_os_str(),
                &arguments,
                Some(directory.path()),
                &[(OsStr::new("PATH"), path.as_os_str())],
                &[],
                limits(),
            )
        });
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
        self.parsed_trailers(message)
            .as_deref()
            .map(signed_off_meaning)
    }

    /// The output of `git interpret-trailers --parse --no-divider` for
    /// `message`, read in the isolated directory `meaning` describes; `None` when
    /// git could not read it.
    fn parsed_trailers(&self, message: &[u8]) -> Option<String> {
        let directory = tempfile::tempdir().ok()?;
        let path = directory.path().join("message");
        if message.contains(&0) {
            return None;
        }
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
        String::from_utf8(output.stdout).ok()
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
        let (git, path) = resolve_program("git")
            .map_err(|error| mechanics_reason(&error, Reason::GitNotStarted))?;
        environment.push((OsStr::new("PATH"), path.as_os_str()));
        match stdin {
            Some(stdin) => process_host::run_configured_stdin(
                git.as_os_str(),
                &all,
                Some(directory),
                &environment,
                &removed,
                stdin,
                limits(),
            ),
            None => process_host::run_configured(
                git.as_os_str(),
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
    let result = resolve_program("ssh-keygen").and_then(|(executable, path)| {
        process_host::run_configured_stdin(
            executable.as_os_str(),
            &arguments,
            Some(directory.path()),
            &[(OsStr::new("PATH"), path.as_os_str())],
            &[],
            stdin,
            limits(),
        )
    });
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
        {
            use std::os::unix::process::ExitStatusExt;
            let killed = CompletedProcess {
                status: std::process::ExitStatus::from_raw(9),
                stdout: Vec::new(),
                stderr: Vec::new(),
            };
            assert_eq!(observed(&Ok(killed)), Run::Signaled);
        }
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
