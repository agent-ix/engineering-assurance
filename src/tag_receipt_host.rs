// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Adapter for tag receipts (FR-081, FR-082).
//!
//! `tag-receipt-record` reads one pushed tag and signs its receipt;
//! `tag-receipt-note` stores the signed bundle in the notes ref and pushes it.
//! The reading half, shared by `approval-trail` and `receipt-missing`, lists the
//! receipt notes and the current tags and hands both to the one pure comparison
//! of `engineering_assurance::tag_receipt`. Verification is the delegated
//! Sigstore stack's (NFR-010); nothing here keeps a log of its own.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use engineering_assurance::{
    approval_verify::RECORD_ID_TRAILER,
    attestation_signing::{
        BundleVerifier, SigstoreBundleVerifier, SourceWorkflowPolicy, TransparencyRequirement,
        WorkflowIdentityDocument, WorkflowIdentityPolicy,
    },
    record_sink::{
        config::{SinkConfig, Visibility, resolve_sink_for_repository},
        form::{
            Hashes, PublicProtocol, PublicRecord, RecordId, RecordKind, Reference,
            References, Sha256Digest, Subject,
        },
        leak::{VisibilityLookup, leak_check_statement},
        sink::Signer,
    },
    signature_verify::trailer_values,
    tag_receipt::{
        Candidate, FindingStatus, InvalidReason, Judged, MissingTagReason, NoteUpdate,
        RecordTrailers, SignerSet, TAG_RECEIPT_NOTES_REF, TagFinding, TagReceiptError,
        TagReceiptPolicy, TagReceiptStatement, add_entry, compare, note_lines, verify_line,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::receipt_host::{
    GitError, GitHubVisibility, gh_api_json, git_command, git_output, git_show_base_file,
    git_with_input, is_full_object_id, run_host_tool, valid_branch, valid_github_repository,
};
use crate::signature_verify_host::{load_listed_tag, run_git};

const RECORD_REQUEST_PROTOCOL: &str = "engineering-assurance.tag-receipt-record-request/v1";
const RECORD_RESULT_PROTOCOL: &str = "engineering-assurance.tag-receipt-record-result/v1";
const NOTE_REQUEST_PROTOCOL: &str = "engineering-assurance.tag-receipt-note-request/v1";
const NOTE_RESULT_PROTOCOL: &str = "engineering-assurance.tag-receipt-note-result/v1";
const MAX_REQUEST_BYTES: usize = 2 * 1024 * 1024;
const MAX_POLICY_BYTES: usize = 1024 * 1024;
/// The first push and the five retries FR-081 allows.
const PUSH_ATTEMPTS: usize = 6;
/// Most notes one read examines.
const MAX_NOTES: usize = 10_000;
const REMOTE_NOTES_REF: &str = "refs/notes/tag-receipts-remote";
const NIL_RECORD_ID: &str = "00000000-0000-0000-0000-000000000000";
const POLICY_PATH: &str = ".engineering-assurance/receipt-policy.json";
const SINKS_PATH: &str = ".engineering-assurance/sinks.json";

/// A refusal that produces no receipt, with its stable code.
#[derive(Debug, Error)]
pub(crate) enum TagReceiptHostError {
    /// The request is not readable JSON.
    #[error("tag receipt request is not readable JSON")]
    Unreadable,
    /// The request is not an exact request of its command.
    #[error("tag receipt request is invalid")]
    Invalid,
    /// The `tag_receipts` policy member breaks its closed schema.
    #[error("the tag_receipts member of the receipt policy is invalid")]
    PolicyInvalid,
    /// The tag name is not valid UTF-8.
    #[error("the tag name is not valid UTF-8")]
    RefNameInvalid,
    /// The tag, its object or its commit does not match the trigger.
    #[error("the tag input does not match the repository")]
    InputMismatch,
    /// The signer is unavailable.
    #[error("the signer is unavailable")]
    SigningUnavailable,
    /// The note would exceed 1 MiB.
    #[error("the tag receipt note would exceed 1 MiB")]
    NoteTooLarge,
    /// Six pushes in a row were rejected.
    #[error("the notes ref could not be pushed after retries")]
    NoteConflict,
    /// The leak check refused a public write.
    #[error("the leak check refused the tag receipt")]
    LeakCheckRefused,
    /// Git could not be read or written.
    #[error("git could not be read or written")]
    Git,
    /// A result could not be encoded.
    #[error("the result could not be encoded")]
    Encoding,
}

impl TagReceiptHostError {
    /// Stable machine error code.
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Unreadable => "tag_receipt_request_unreadable",
            Self::Invalid => "tag_receipt_request_invalid",
            Self::PolicyInvalid => "tag_receipt_policy_invalid",
            Self::RefNameInvalid => "tag_receipt_ref_name_invalid",
            Self::InputMismatch => "tag_receipt_input_mismatch",
            Self::SigningUnavailable => "tag_receipt_signing_unavailable",
            Self::NoteTooLarge => "tag_receipt_note_too_large",
            Self::NoteConflict => "tag_receipt_note_conflict",
            Self::LeakCheckRefused => "tag_receipt_leak_check_refused",
            Self::Git => "tag_receipt_git_failed",
            Self::Encoding => "tag_receipt_result_encoding_failed",
        }
    }
}

impl From<TagReceiptError> for TagReceiptHostError {
    fn from(error: TagReceiptError) -> Self {
        match error {
            TagReceiptError::NoteTooLarge => Self::NoteTooLarge,
            TagReceiptError::NoteInvalid => Self::Invalid,
            TagReceiptError::Encoding => Self::Encoding,
        }
    }
}

// ---------------------------------------------------------------------------
// tag-receipt-record
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordRequest {
    protocol: String,
    repository: PathBuf,
    github_repository: String,
    base_branch: String,
    /// The raw bytes of the tag name in lower-case hexadecimal, so a name that
    /// is not UTF-8 reaches the tool and is refused there.
    tag_hex: String,
    tag_object: String,
    /// The platform's push time, Unix seconds, from the trigger context.
    pushed_at_unix: i64,
}

#[derive(Serialize)]
struct RecordResult {
    protocol: &'static str,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit: Option<String>,
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag_object: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bundle: Option<Value>,
}

/// What the protected base branch says about tag receipts for one repository.
pub(crate) struct Protected {
    policy: Option<TagReceiptPolicy>,
    private: bool,
    public_linear_teams: Vec<String>,
}

impl Protected {
    /// From the bytes of the receipt policy on the protected base branch, or
    /// `None` when the file is absent (a repository with no tag receipts).
    ///
    /// # Errors
    ///
    /// [`TagReceiptHostError::PolicyInvalid`] when the document or its
    /// `tag_receipts` member is not valid.
    pub(crate) fn from_policy_bytes(
        policy: Option<&[u8]>,
        private: bool,
        public_linear_teams: Vec<String>,
    ) -> Result<Self, TagReceiptHostError> {
        let policy = policy
            .map(TagReceiptPolicy::from_receipt_policy)
            .transpose()
            .map_err(|_| TagReceiptHostError::PolicyInvalid)?
            .flatten();
        Ok(Self {
            policy,
            private,
            public_linear_teams,
        })
    }

    fn load(request: &RecordRequest) -> Result<Self, TagReceiptHostError> {
        let (private, base_commit) = protected_base(
            &request.github_repository,
            &request.base_branch,
        )
        .ok_or(TagReceiptHostError::Invalid)?;
        let present = run_host_tool(
            std::ffi::OsStr::new("git"),
            &[
                std::ffi::OsStr::new("-C"),
                request.repository.as_os_str(),
                std::ffi::OsStr::new("cat-file"),
                std::ffi::OsStr::new("-e"),
                std::ffi::OsStr::new(&format!("{base_commit}:{POLICY_PATH}")),
            ],
            None,
            MAX_POLICY_BYTES,
        )
        .is_ok();
        let policy = if present {
            Some(
                git_show_base_file(&request.repository, &base_commit, POLICY_PATH)
                    .map_err(|()| TagReceiptHostError::Git)?,
            )
        } else {
            None
        };
        // Fail closed: when the sink configuration cannot be read no Linear team
        // is public, so a public receipt naming a ticket is refused.
        let visibility = if private {
            Visibility::Private
        } else {
            Visibility::Public
        };
        let public_linear_teams = git_show_base_file(&request.repository, &base_commit, SINKS_PATH)
            .ok()
            .and_then(|bytes| SinkConfig::parse(&bytes).ok())
            .and_then(|config| {
                resolve_sink_for_repository(Some(&config), visibility, &request.github_repository)
                    .ok()
            })
            .map(|selection| selection.public_linear_teams)
            .unwrap_or_default();
        Self::from_policy_bytes(policy.as_deref(), private, public_linear_teams)
    }
}

/// Whether the repository is private and the commit its protected default
/// branch points at; `None` when either cannot be established.
fn protected_base(github_repository: &str, base_branch: &str) -> Option<(bool, String)> {
    let repository = gh_api_json(&format!("repos/{github_repository}")).ok()?;
    if repository.get("default_branch")?.as_str()? != base_branch {
        return None;
    }
    let private = repository.get("private")?.as_bool()?;
    let branch = gh_api_json(&format!(
        "repos/{github_repository}/branches/{}",
        crate::receipt_host::encode_path_segment(base_branch)
    ))
    .ok()?;
    if branch.get("protected")?.as_bool()? != true {
        return None;
    }
    let commit = branch.pointer("/commit/sha")?.as_str()?;
    is_full_object_id(commit).then(|| (private, commit.to_owned()))
}

/// Reads one pushed tag, signs its receipt and returns the signed bundle.
pub(crate) fn execute_record(bytes: &[u8]) -> Result<Vec<u8>, TagReceiptHostError> {
    let request = parse_record_request(bytes)?;
    let protected = Protected::load(&request)?;
    let signer = engineering_assurance::attestation_signing::SigstoreSigner::new();
    record_with(&request, &protected, &signer, &GitHubVisibility)
}

fn parse_record_request(bytes: &[u8]) -> Result<RecordRequest, TagReceiptHostError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(TagReceiptHostError::Invalid);
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| TagReceiptHostError::Unreadable)?;
    let request: RecordRequest =
        serde_json::from_value(value).map_err(|_| TagReceiptHostError::Invalid)?;
    if request.protocol != RECORD_REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.repository.is_dir()
        || !valid_github_repository(&request.github_repository)
        || !valid_branch(&request.base_branch)
        || !is_full_object_id(&request.tag_object)
        || request.tag_hex.is_empty()
        || request.pushed_at_unix < 0
    {
        return Err(TagReceiptHostError::Invalid);
    }
    Ok(request)
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    text.as_bytes()
        .chunks(2)
        .map(|pair| {
            let digit = |byte: u8| char::from(byte).to_digit(16);
            Some(u8::try_from(digit(pair[0])? * 16 + digit(pair[1])?).ok()?)
        })
        .collect()
}

/// The pushed tag as git holds it.
struct ReadTag {
    object: String,
    commit: String,
    trailers: RecordTrailers,
}

fn rev_parse(repository: &Path, expression: &str) -> Result<String, TagReceiptHostError> {
    let output = run_git(
        repository,
        &["rev-parse", "--verify", "--quiet", "--end-of-options", expression],
    )
    .map_err(|_| TagReceiptHostError::Git)?;
    let id = output
        .status
        .success()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
        .map(|text| text.trim_end().to_owned())
        .filter(|id| is_full_object_id(id))
        .ok_or(TagReceiptHostError::InputMismatch)?;
    Ok(id)
}

fn read_tag(repository: &Path, name: &str) -> Result<ReadTag, TagReceiptHostError> {
    let reference = format!("refs/tags/{name}");
    let (object, trailers) = match load_listed_tag(repository, name, Path::new("/dev/null")) {
        Ok(Some(loaded)) => (
            loaded.id().to_owned(),
            loaded.trailers().map_or_else(RecordTrailers::unreadable, |parsed| {
                RecordTrailers::from_values(trailer_values(parsed, RECORD_ID_TRAILER))
            }),
        ),
        // A lightweight tag has no message: its object is the commit.
        Ok(None) => (
            rev_parse(repository, &reference)?,
            RecordTrailers::from_values(Vec::new()),
        ),
        Err(_) => return Err(TagReceiptHostError::InputMismatch),
    };
    let commit = rev_parse(repository, &format!("{reference}^{{commit}}"))?;
    Ok(ReadTag {
        object,
        commit,
        trailers,
    })
}

fn push_time(unix: i64) -> Result<String, TagReceiptHostError> {
    let instant = time::OffsetDateTime::from_unix_timestamp(unix)
        .map_err(|_| TagReceiptHostError::Invalid)?;
    Ok(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        instant.year(),
        u8::from(instant.month()),
        instant.day(),
        instant.hour(),
        instant.minute(),
        instant.second()
    ))
}

/// The record that the leak check reads for the text members of a statement:
/// the repository and every record id that names a Linear issue. It is checked
/// and dropped; the receipt itself is the statement.
fn leak_check_record(
    repository: &str,
    statement: &[u8],
    record_ids: &[String],
) -> Result<PublicRecord, TagReceiptHostError> {
    let refused = |_| TagReceiptHostError::LeakCheckRefused;
    let references = record_ids
        .iter()
        .filter_map(|id| Reference::linear_issue(id).ok())
        .collect();
    let digest = format!(
        "sha256:{}",
        sigstore_verify::crypto::sha256(statement).to_hex()
    );
    Ok(PublicRecord {
        protocol: PublicProtocol::V1,
        record_id: RecordId::try_from(NIL_RECORD_ID.to_owned()).map_err(refused)?,
        kind: RecordKind::Approval,
        subject: Subject::repo(repository).map_err(refused)?,
        references: References::new(references).map_err(refused)?,
        hashes: Hashes {
            statement: Some(Sha256Digest::try_from(digest).map_err(refused)?),
            ..Hashes::default()
        },
        verdicts: engineering_assurance::record_sink::form::Verdicts::default(),
        timestamps: engineering_assurance::record_sink::form::Timestamps::default(),
        signature: None,
    })
}

/// As [`execute_record`], with the protected policy, the signer and the
/// visibility lookup supplied.
fn record_with(
    request: &RecordRequest,
    protected: &Protected,
    signer: &dyn Signer,
    visibility: &dyn VisibilityLookup,
) -> Result<Vec<u8>, TagReceiptHostError> {
    let skipped = || {
        encode(&RecordResult {
            protocol: RECORD_RESULT_PROTOCOL,
            status: "skipped",
            commit: None,
            reference: None,
            tag_object: None,
            bundle: None,
        })
    };
    let Some(policy) = &protected.policy else {
        return skipped();
    };
    let name = decode_hex(&request.tag_hex)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .ok_or(TagReceiptHostError::RefNameInvalid)?;
    if !policy.covers(&name) {
        return skipped();
    }
    let tag = read_tag(&request.repository, &name)?;
    if tag.object != request.tag_object {
        return Err(TagReceiptHostError::InputMismatch);
    }
    let reference = format!("refs/tags/{name}");
    let statement = TagReceiptStatement::new(
        &request.github_repository,
        &reference,
        &tag.object,
        &tag.commit,
        &push_time(request.pushed_at_unix)?,
        tag.trailers.clone(),
    );
    let canonical = statement.canonical_bytes()?;
    let bundle = if protected.private {
        signer
            .sign_without_public_log(&canonical)
            .map_err(|_| TagReceiptHostError::SigningUnavailable)?
    } else {
        let record = leak_check_record(
            &request.github_repository,
            &canonical,
            &tag.trailers.record_ids,
        )?;
        let proof = leak_check_statement(
            &request.github_repository,
            &record,
            &canonical,
            &protected.public_linear_teams,
            visibility,
        )
        .map_err(|_| TagReceiptHostError::LeakCheckRefused)?;
        signer
            .sign_statement_for_public_log(&proof)
            .map_err(|_| TagReceiptHostError::SigningUnavailable)?
    };
    let bundle: Value =
        serde_json::from_str(&bundle).map_err(|_| TagReceiptHostError::SigningUnavailable)?;
    encode(&RecordResult {
        protocol: RECORD_RESULT_PROTOCOL,
        status: "signed",
        commit: Some(tag.commit),
        reference: Some(reference),
        tag_object: Some(tag.object),
        bundle: Some(bundle),
    })
}

fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, TagReceiptHostError> {
    let mut out = serde_json::to_vec(value).map_err(|_| TagReceiptHostError::Encoding)?;
    out.push(b'\n');
    Ok(out)
}

// ---------------------------------------------------------------------------
// tag-receipt-note
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoteRequest {
    protocol: String,
    repository: PathBuf,
    /// The remote the notes ref is fetched from and pushed to.
    remote: String,
    commit: String,
    bundle: Value,
}

#[derive(Serialize)]
struct NoteResult {
    protocol: &'static str,
    status: &'static str,
    commit: String,
}

/// Stores one signed bundle in the notes ref and pushes it, merging and
/// retrying a rejected push (FR-081, FR-076).
pub(crate) fn execute_note(bytes: &[u8]) -> Result<Vec<u8>, TagReceiptHostError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(TagReceiptHostError::Invalid);
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| TagReceiptHostError::Unreadable)?;
    let request: NoteRequest =
        serde_json::from_value(value).map_err(|_| TagReceiptHostError::Invalid)?;
    if request.protocol != NOTE_REQUEST_PROTOCOL
        || !request.repository.is_absolute()
        || !request.repository.is_dir()
        || !is_full_object_id(&request.commit)
        || request.remote.is_empty()
        || request.remote.starts_with('-')
        || !request
            .remote
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(TagReceiptHostError::Invalid);
    }
    // The entry must be a bundle of a closed tag receipt statement for this commit.
    let statement = engineering_assurance::tag_receipt::parse_statement(&request.bundle)
        .ok_or(TagReceiptHostError::Invalid)?;
    if statement.predicate.commit != request.commit {
        return Err(TagReceiptHostError::Invalid);
    }
    let status = store(&request)?;
    encode(&NoteResult {
        protocol: NOTE_RESULT_PROTOCOL,
        status,
        commit: request.commit,
    })
}

fn ref_value(repository: &Path, reference: &str) -> Option<String> {
    git_output(
        repository,
        &["rev-parse", "--verify", "--quiet", "--end-of-options", reference],
    )
    .ok()
    .and_then(|out| String::from_utf8(out).ok())
    .map(|text| text.trim_end().to_owned())
}

fn git_ok(repository: &Path, arguments: &[&str]) -> bool {
    git_command(repository, arguments, None).is_ok_and(|output| output.status.success())
}

fn store(request: &NoteRequest) -> Result<&'static str, TagReceiptHostError> {
    let repository = request.repository.as_path();
    let before = ref_value(repository, TAG_RECEIPT_NOTES_REF);
    let outcome = store_attempts(request, repository);
    if outcome.is_err() {
        // Leave no partial local note behind after a failure.
        match &before {
            Some(value) => {
                git_ok(repository, &["update-ref", TAG_RECEIPT_NOTES_REF, value]);
            }
            None => {
                git_ok(repository, &["update-ref", "-d", TAG_RECEIPT_NOTES_REF]);
            }
        }
    }
    git_ok(repository, &["update-ref", "-d", REMOTE_NOTES_REF]);
    outcome
}

fn store_attempts(
    request: &NoteRequest,
    repository: &Path,
) -> Result<&'static str, TagReceiptHostError> {
    let refspec = format!("+{TAG_RECEIPT_NOTES_REF}:{REMOTE_NOTES_REF}");
    let push = format!("{TAG_RECEIPT_NOTES_REF}:{TAG_RECEIPT_NOTES_REF}");
    for _ in 0..PUSH_ATTEMPTS {
        // A remote without the ref is an empty remote set, not an error.
        git_ok(repository, &["update-ref", "-d", REMOTE_NOTES_REF]);
        git_ok(
            repository,
            &["fetch", "--no-tags", &request.remote, &refspec],
        );
        if let Some(remote) = ref_value(repository, REMOTE_NOTES_REF) {
            match ref_value(repository, TAG_RECEIPT_NOTES_REF) {
                None => {
                    if !git_ok(
                        repository,
                        &["update-ref", TAG_RECEIPT_NOTES_REF, &remote],
                    ) {
                        return Err(TagReceiptHostError::Git);
                    }
                }
                Some(local) if local != remote => {
                    if !git_ok(
                        repository,
                        &[
                            "-c",
                            "user.name=engineering-assurance",
                            "-c",
                            "user.email=engineering-assurance@localhost",
                            "notes",
                            "--ref",
                            TAG_RECEIPT_NOTES_REF,
                            "merge",
                            "-s",
                            "cat_sort_uniq",
                            REMOTE_NOTES_REF,
                        ],
                    ) {
                        return Err(TagReceiptHostError::Git);
                    }
                }
                Some(_) => {}
            }
        }
        let existing = match git_output(
            repository,
            &["notes", "--ref", TAG_RECEIPT_NOTES_REF, "show", &request.commit],
        ) {
            Ok(note) => Some(note),
            Err(GitError::NotFound) => None,
            Err(GitError::Failed) => return Err(TagReceiptHostError::Git),
        };
        let note = match add_entry(existing.as_deref(), &request.bundle)? {
            NoteUpdate::Unchanged => return Ok("already_recorded"),
            NoteUpdate::Updated(note) => note,
        };
        git_with_input(
            repository,
            &[
                "-c",
                "user.name=engineering-assurance",
                "-c",
                "user.email=engineering-assurance@localhost",
                "notes",
                "--ref",
                TAG_RECEIPT_NOTES_REF,
                "add",
                "-f",
                "-F",
                "-",
                &request.commit,
            ],
            &note,
        )
        .map_err(|_| TagReceiptHostError::Git)?;
        if git_ok(repository, &["push", &request.remote, &push]) {
            return Ok("stored");
        }
    }
    Err(TagReceiptHostError::NoteConflict)
}

// ---------------------------------------------------------------------------
// Reading the receipts and the tag set (approval-trail, receipt-missing)
// ---------------------------------------------------------------------------

/// One verifier built from one trusted root, with the log evidence it needs.
pub(crate) struct Attempt<'a> {
    pub(crate) verifier: &'a dyn BundleVerifier,
    pub(crate) transparency: TransparencyRequirement,
}

/// Where the verifiers come from; tests supply their own.
pub(crate) trait VerifierSource {
    /// The verifiers to try, strongest evidence first.
    ///
    /// # Errors
    ///
    /// [`TagReceiptHostError::Git`] is not used; a trust root that cannot be
    /// read is [`TagReceiptHostError::SigningUnavailable`].
    fn attempts(&self) -> Result<Vec<Attempt<'_>>, TagReceiptHostError>;
}

/// The production verifiers: the public-good root with log inclusion required,
/// then the platform's private-instance root with timestamp-only evidence. A
/// receipt carries no word on which signed it, and the identity policy pins
/// the repository either way.
#[derive(Default)]
pub(crate) struct ProductionVerifiers {
    cell: OnceLock<Option<(SigstoreBundleVerifier, SigstoreBundleVerifier)>>,
}

fn production_verifier(root: &str) -> Option<SigstoreBundleVerifier> {
    let root = sigstore_verify::trust_root::TrustedRoot::from_json(root).ok()?;
    SigstoreBundleVerifier::new(&root).ok()
}

impl VerifierSource for ProductionVerifiers {
    fn attempts(&self) -> Result<Vec<Attempt<'_>>, TagReceiptHostError> {
        let pair = self.cell.get_or_init(|| {
            Some((
                production_verifier(sigstore_verify::trust_root::SIGSTORE_PRODUCTION_TRUSTED_ROOT)?,
                production_verifier(sigstore_verify::trust_root::SIGSTORE_GITHUB_TRUSTED_ROOT)?,
            ))
        });
        let (public, github) = pair
            .as_ref()
            .ok_or(TagReceiptHostError::SigningUnavailable)?;
        Ok(vec![
            Attempt {
                verifier: public,
                transparency: TransparencyRequirement::PublicLog,
            },
            Attempt {
                verifier: github,
                transparency: TransparencyRequirement::TimestampOnly,
            },
        ])
    }
}

/// The listed tag receipt identities of a policy, paired with their verifiers.
pub(crate) struct Trust<'a> {
    entries: Vec<(
        &'a dyn BundleVerifier,
        Vec<(WorkflowIdentityPolicy, SourceWorkflowPolicy)>,
    )>,
}

impl<'a> Trust<'a> {
    pub(crate) fn new(
        identities: &WorkflowIdentityDocument,
        source: &'a dyn VerifierSource,
    ) -> Result<Self, TagReceiptHostError> {
        let entries = source
            .attempts()?
            .into_iter()
            .map(|attempt| {
                (
                    attempt.verifier,
                    identities.tag_receipt_signers(attempt.transparency),
                )
            })
            .collect();
        Ok(Self { entries })
    }

    fn sets(&self) -> Vec<SignerSet<'_>> {
        self.entries
            .iter()
            .map(|(verifier, signers)| SignerSet {
                verifier: *verifier,
                signers,
            })
            .collect()
    }
}

/// Every line of every note under the tag receipt notes ref, each verified.
/// An absent notes ref is an empty set.
pub(crate) fn read_candidates(
    repository: &Path,
    trust: &Trust<'_>,
) -> Result<Vec<Candidate>, TagReceiptHostError> {
    let listing = run_git(repository, &["notes", "--ref", TAG_RECEIPT_NOTES_REF, "list"])
        .map_err(|_| TagReceiptHostError::Git)?;
    if !listing.status.success() {
        return Err(TagReceiptHostError::Git);
    }
    let listing = String::from_utf8(listing.stdout).map_err(|_| TagReceiptHostError::Git)?;
    let notes: Vec<(&str, &str)> = listing
        .lines()
        .map(|line| line.split_once(' ').ok_or(TagReceiptHostError::Git))
        .collect::<Result<_, _>>()?;
    if notes.len() > MAX_NOTES {
        return Err(TagReceiptHostError::Git);
    }
    let sets = trust.sets();
    let mut candidates = Vec::new();
    for (blob, commit) in notes {
        if !is_full_object_id(blob) || !is_full_object_id(commit) {
            return Err(TagReceiptHostError::Git);
        }
        let content = run_git(repository, &["cat-file", "blob", blob])
            .map_err(|_| TagReceiptHostError::Git)?;
        if !content.status.success() {
            return Err(TagReceiptHostError::Git);
        }
        for line in note_lines(&content.stdout) {
            candidates.push(verify_line(line.as_ref(), commit, &sets));
        }
    }
    Ok(candidates)
}

/// The current `refs/tags/*` with the object id each names. A name that is not
/// UTF-8 can be neither receipted nor match a prefix, so it is left out.
pub(crate) fn current_tags(
    repository: &Path,
) -> Result<BTreeMap<String, String>, TagReceiptHostError> {
    let output = run_git(
        repository,
        &["for-each-ref", "--format=%(refname)%00%(objectname)", "refs/tags"],
    )
    .map_err(|_| TagReceiptHostError::Git)?;
    if !output.status.success() {
        return Err(TagReceiptHostError::Git);
    }
    let mut tags = BTreeMap::new();
    for line in output.stdout.split(|byte| *byte == b'\n') {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, |byte| *byte == 0);
        let (Some(name), Some(object)) = (parts.next(), parts.next()) else {
            return Err(TagReceiptHostError::Git);
        };
        if let (Ok(name), Ok(object)) = (std::str::from_utf8(name), std::str::from_utf8(object)) {
            tags.insert(name.to_owned(), object.to_owned());
        }
    }
    Ok(tags)
}

/// The tag findings of FR-082 for the whole repository.
pub(crate) fn repository_findings(
    repository: &Path,
    identities: &WorkflowIdentityDocument,
    policy: &TagReceiptPolicy,
    source: &dyn VerifierSource,
) -> Result<Vec<Judged>, TagReceiptHostError> {
    let trust = Trust::new(identities, source)?;
    let candidates = read_candidates(repository, &trust)?;
    let tags = current_tags(repository)?;
    Ok(compare(&candidates, &tags, policy))
}

/// The reason of a tag finding: why a receipted tag is missing, or why a
/// receipt is invalid.
#[derive(Clone, Copy, Serialize)]
#[serde(untagged)]
pub(crate) enum FindingReason {
    Missing(MissingTagReason),
    Invalid(InvalidReason),
}

/// One tag finding of `receipt-missing` (FR-082): `kind` is always `tag`.
#[derive(Serialize)]
pub(crate) struct TagFindingLine {
    kind: &'static str,
    /// `missing_tag`, `tag_no_receipt`, `invalid_receipt` or `accepted`.
    status: &'static str,
    /// Absent for an invalid receipt, whose content is not trusted.
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag_object: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<FindingReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    current_object: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit: Option<String>,
    /// The status an acceptance replaced.
    #[serde(skip_serializing_if = "Option::is_none")]
    finding: Option<FindingStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accepted_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accepted_reason: Option<String>,
}

impl From<&Judged> for TagFindingLine {
    fn from(judged: &Judged) -> Self {
        let status = judged.finding.status();
        let mut line = Self {
            kind: "tag",
            status: match status {
                FindingStatus::MissingTag => "missing_tag",
                FindingStatus::TagNoReceipt => "tag_no_receipt",
                FindingStatus::InvalidReceipt => "invalid_receipt",
            },
            reference: None,
            tag_object: None,
            reason: None,
            current_object: None,
            commit: None,
            finding: None,
            accepted_by: None,
            accepted_reason: None,
        };
        match &judged.finding {
            TagFinding::MissingTag {
                receipt,
                reason,
                current_object,
            } => {
                line.reference = Some(receipt.reference.clone());
                line.tag_object = Some(receipt.tag_object.clone());
                line.reason = Some(FindingReason::Missing(*reason));
                line.current_object.clone_from(current_object);
                line.commit = Some(receipt.commit.clone());
            }
            TagFinding::TagNoReceipt {
                reference,
                current_object,
            } => {
                line.reference = Some(reference.clone());
                line.tag_object = Some(current_object.clone());
            }
            TagFinding::InvalidReceipt(invalid) => {
                line.reason = Some(FindingReason::Invalid(invalid.reason));
                line.commit = Some(invalid.commit.clone());
            }
        }
        if let Some(acceptance) = &judged.accepted {
            line.status = "accepted";
            line.finding = Some(status);
            line.accepted_by = Some(acceptance.accepted_by.clone());
            line.accepted_reason = Some(acceptance.accepted_reason.clone());
        }
        line
    }
}

#[cfg(test)]
// Child-process test setup lives outside `src` so the production capability scan
// does not classify it as a production host capability.
#[path = "../tests/support/tag_receipt_host.rs"]
mod tests;
