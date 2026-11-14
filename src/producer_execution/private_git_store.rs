// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Git-backed private record retention at the process-capable boundary.
//!
//! The domain sink remains I/O-free. This adapter resolves the configured
//! `owner/name` destination, stores canonical record forms on the one fixed
//! branch, and delegates all outcome signing and verification to the supplied
//! Sigstore ports.

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    attestation_signing::{
        BundleVerifier, TransparencyRequirement, WorkflowIdentityDocument, WorkflowIdentityPolicy,
        WorkflowRole,
    },
    record_sink::{
        config::{PolicySource, SinkKind},
        form::{PrivateRecord, PublicRecord, RecordId},
        sink::{
            ApprovalStatementLookup, ApprovalStatementPublishOutcome, ApprovalStatementReadError,
            ApprovalStatementReader, PrivateStore, RetainedApprovalStatement, Signer, StoreError,
            StorePreparation,
        },
    },
};

/// The sole private-store ref required by FR-054.
pub const PRIVATE_RECORDS_REF: &str = "refs/heads/engineering-assurance-private-records";
/// Protected-base workflow-identity policy path.
pub const WORKFLOW_IDENTITIES_PATH: &str = ".engineering-assurance/workflow-identities.json";
const BRANCH: &str = "engineering-assurance-private-records";
const PROTOCOL: &str = "engineering-assurance.private-store-outcome/v1";
const ENVELOPE_PROTOCOL: &str = "engineering-assurance.private-store-signed-outcome/v1";
const STATEMENT_TYPE: &str = concat!("https:", "//in-toto.io/Statement/v1");
const APPROVAL_PREDICATE_TYPE: &str =
    concat!("https:", "//schemas.agent-ix.ai/attestation/approval/v1");
const MAX_GIT_OUTPUT: usize = 16 * 1024 * 1024;
const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;
const MAX_PUSH_ATTEMPTS: usize = 5;
const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(120);

/// Resolves the configured `owner/name` to an authenticated Git remote URL.
///
/// The resolver owns hosting-service selection and credential setup. It must
/// return a URL without embedded user information; authentication is supplied
/// by the Git credential mechanism.
pub trait GitDestinationResolver {
    /// Resolve one validated destination repository.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the destination cannot be resolved.
    fn resolve(&self, destination: &str) -> Result<String, StoreError>;
}

/// Resolver for an HTTPS Git hosting service whose repository URLs end in
/// `.git` (for example GitHub).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HttpsGitDestinationResolver {
    host: String,
}

impl HttpsGitDestinationResolver {
    /// Construct a resolver for an HTTPS Git hosting domain.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] for a malformed host or embedded credentials.
    pub fn new(host: &str) -> Result<Self, StoreError> {
        let host = host.trim_end_matches('/');
        let Some(authority) = host.strip_prefix("https://") else {
            return Err(StoreError);
        };
        if authority.is_empty()
            || authority.contains(['/', '@', '?', '#'])
            || host.chars().any(char::is_whitespace)
        {
            return Err(StoreError);
        }
        Ok(Self {
            host: host.to_owned(),
        })
    }
}

impl GitDestinationResolver for HttpsGitDestinationResolver {
    fn resolve(&self, destination: &str) -> Result<String, StoreError> {
        if !valid_repository(destination) {
            return Err(StoreError);
        }
        Ok(format!("{}/{destination}.git", self.host))
    }
}

/// Identity coordinates for the `record_publisher` workflow on protected
/// base-branch policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordPublisherIdentity {
    /// Repository containing the signer workflow.
    pub repository: String,
    /// Exact workflow path.
    pub workflow_path: String,
    /// Exact protected-base-branch ref.
    pub reference: String,
}

/// Git private store with delegated signing and verification.
pub struct GitPrivateStore<'a> {
    resolver: &'a dyn GitDestinationResolver,
    signer: &'a dyn Signer,
    verifier: &'a dyn BundleVerifier,
    publisher_policy: WorkflowIdentityPolicy,
    publishing_repository: String,
    git_executable: PathBuf,
}

impl<'a> GitPrivateStore<'a> {
    /// Build a private-store adapter from protected-base policy.
    ///
    /// The identity document must have been read from the publishing
    /// repository's protected base branch. The constructor refuses unless it
    /// contains the exact configured `record_publisher` identity.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the base branch is unprotected, policy is
    /// missing or malformed, or the configured publisher is not authorized.
    pub fn new(
        resolver: &'a dyn GitDestinationResolver,
        signer: &'a dyn Signer,
        verifier: &'a dyn BundleVerifier,
        policy_source: &dyn PolicySource,
        publisher: &RecordPublisherIdentity,
        publishing_repository: &str,
        git_executable: impl Into<PathBuf>,
    ) -> Result<Self, StoreError> {
        if !valid_repository(publishing_repository) {
            return Err(StoreError);
        }
        if policy_source.base_branch_protected() != Some(true) {
            return Err(StoreError);
        }
        let identity_bytes = policy_source
            .read_from_base_branch(WORKFLOW_IDENTITIES_PATH)
            .ok_or(StoreError)?;
        let identity_document =
            WorkflowIdentityDocument::parse(&identity_bytes).map_err(|_| StoreError)?;
        let publisher_policy = identity_document
            .signer_policy(
                WorkflowRole::RecordPublisher,
                &publisher.repository,
                &publisher.workflow_path,
                &publisher.reference,
                TransparencyRequirement::TimestampOnly,
            )
            .ok_or(StoreError)?;
        Ok(Self {
            resolver,
            signer,
            verifier,
            publisher_policy,
            publishing_repository: publishing_repository.to_owned(),
            git_executable: git_executable.into(),
        })
    }

    fn checked_remote(&self, destination: &str) -> Result<String, StoreError> {
        if !valid_repository(destination)
            || canonical_repository(destination)
                == canonical_repository(&self.publishing_repository)
        {
            return Err(StoreError);
        }
        let remote = self.resolver.resolve(destination)?;
        let valid_scheme = remote.starts_with("https://")
            || remote.starts_with("ssh://")
            || remote.starts_with("git@")
            || remote.starts_with("file://");
        if !valid_scheme || remote.contains('@') && remote.starts_with("https://") {
            return Err(StoreError);
        }
        Ok(remote)
    }

    fn record_path(record_id: &RecordId, repository: &str) -> Result<String, StoreError> {
        let (owner, name) = repository.split_once('/').ok_or(StoreError)?;
        Ok(format!(
            "records/{}/{}/{}/",
            hex_component(owner),
            hex_component(name),
            record_id.as_str()
        ))
    }

    fn with_repository<T>(
        &self,
        destination: &str,
        mut operation: impl FnMut(&Path) -> Result<T, StoreError>,
    ) -> Result<T, StoreError> {
        let remote = self.checked_remote(destination)?;
        let temp = tempfile::Builder::new()
            .prefix("ea-private-store-")
            .tempdir()
            .map_err(|_| StoreError)?;
        let repository = temp.path().join("store");
        let cloned = self.git(
            None,
            &[
                "clone",
                "--no-checkout",
                "--",
                &remote,
                path_arg(&repository)?,
            ],
        )?;
        require_success(&cloned)?;
        self.checkout_store_ref(&repository)?;
        operation(&repository)
    }

    fn checkout_store_ref(&self, repository: &Path) -> Result<(), StoreError> {
        let listed = self.git(
            Some(repository),
            &["ls-remote", "--exit-code", "origin", PRIVATE_RECORDS_REF],
        )?;
        match listed.status.code() {
            Some(0) => {
                let fetched = self.git(
                    Some(repository),
                    &[
                        "fetch",
                        "--no-tags",
                        "origin",
                        &format!("{PRIVATE_RECORDS_REF}:refs/remotes/origin/{BRANCH}"),
                    ],
                )?;
                require_success(&fetched)?;
                let checkout = self.git(
                    Some(repository),
                    &[
                        "checkout",
                        "--force",
                        "--track",
                        "-B",
                        BRANCH,
                        &format!("origin/{BRANCH}"),
                    ],
                )?;
                require_success(&checkout)
            }
            Some(2) => {
                let checkout = self.git(Some(repository), &["checkout", "--orphan", BRANCH])?;
                require_success(&checkout)
            }
            _ => Err(StoreError),
        }
    }

    fn git(&self, repository: Option<&Path>, args: &[&str]) -> Result<Output, StoreError> {
        let mut command = Command::new(&self.git_executable);
        if let Some(repository) = repository {
            command.arg("-C").arg(repository);
        }
        run_bounded_git(command.args(args).env("GIT_TERMINAL_PROMPT", "0"))
    }

    fn read_file(&self, repository: &Path, path: &str) -> Result<Option<Vec<u8>>, StoreError> {
        let reference = self.git(
            Some(repository),
            &["rev-parse", "--verify", &format!("{BRANCH}^{{commit}}")],
        )?;
        match reference.status.code() {
            Some(0) => {}
            Some(128) => return Ok(None),
            _ => return Err(StoreError),
        }
        let listed = self.git(
            Some(repository),
            &["ls-tree", "--name-only", BRANCH, "--", path],
        )?;
        require_success(&listed)?;
        let shown = self.git(Some(repository), &["show", &format!("{BRANCH}:{path}")])?;
        match shown.status.code() {
            Some(0) if shown.stdout.len() <= MAX_FILE_BYTES => Ok(Some(shown.stdout)),
            Some(128 | 1) => Ok(None),
            _ => Err(StoreError),
        }
    }

    fn write_file(&self, repository: &Path, path: &str, bytes: &[u8]) -> Result<(), StoreError> {
        let absolute = repository.join(path);
        let parent = absolute.parent().ok_or(StoreError)?;
        fs::create_dir_all(parent).map_err(|_| StoreError)?;
        fs::write(&absolute, bytes).map_err(|_| StoreError)?;
        let added = self.git(Some(repository), &["add", "--", path])?;
        require_success(&added)?;
        let committed = self.git(
            Some(repository),
            &[
                "-c",
                "user.name=Engineering Assurance",
                "-c",
                "user.email=engineering-assurance@agent-ix.invalid",
                "commit",
                "--quiet",
                "-m",
                "Retain private assurance record",
            ],
        )?;
        require_success(&committed)
    }

    fn push(&self, repository: &Path) -> Result<(), StoreError> {
        let pushed = self.git(
            Some(repository),
            &["push", "origin", &format!("HEAD:{PRIVATE_RECORDS_REF}")],
        )?;
        require_success(&pushed)
    }

    fn commit_new_file(
        &self,
        destination: &str,
        path: &str,
        bytes: &[u8],
        identical_is_success: bool,
    ) -> Result<(), StoreError> {
        for _ in 0..MAX_PUSH_ATTEMPTS {
            let attempt = self.with_repository(destination, |repository| {
                if let Some(stored) = self.read_file(repository, path)? {
                    if identical_is_success && stored == bytes {
                        return Ok(true);
                    }
                    return Err(StoreError);
                }
                self.write_file(repository, path, bytes)?;
                self.push(repository)?;
                Ok(false)
            });
            if attempt.is_ok() {
                return Ok(());
            }
        }
        Err(StoreError)
    }

    fn verified_outcome(
        &self,
        repository: &Path,
        record_path: &str,
        record_id: &RecordId,
    ) -> Result<Option<OutcomeStatement>, StoreError> {
        let path = format!("{record_path}outcome.json");
        let Some(bytes) = self.read_file(repository, &path)? else {
            return Ok(None);
        };
        let envelope: SignedOutcome = serde_json::from_slice(&bytes).map_err(|_| StoreError)?;
        if envelope.protocol != ENVELOPE_PROTOCOL {
            return Err(StoreError);
        }
        if serde_json_canonicalizer::to_vec(&envelope).map_err(|_| StoreError)? != bytes {
            return Err(StoreError);
        }
        let statement_bytes = envelope.statement.as_bytes();
        let statement: OutcomeStatement =
            serde_json::from_slice(statement_bytes).map_err(|_| StoreError)?;
        if statement.protocol != PROTOCOL
            || statement.record_id != record_id.as_str()
            || statement.sink == SinkKind::None
        {
            return Err(StoreError);
        }
        let canonical = serde_json_canonicalizer::to_vec(&statement).map_err(|_| StoreError)?;
        if canonical != statement_bytes
            || self
                .verifier
                .verify(
                    statement_bytes,
                    &envelope.signature_bundle,
                    &self.publisher_policy,
                )
                .is_err()
        {
            return Err(StoreError);
        }
        Ok(Some(statement))
    }

    fn commit_outcome(
        &self,
        destination: &str,
        record_path: &str,
        expected: &OutcomeStatement,
        bytes: &[u8],
    ) -> Result<(), StoreError> {
        let path = format!("{record_path}outcome.json");
        for _ in 0..MAX_PUSH_ATTEMPTS {
            let result = self.with_repository(destination, |repository| {
                let private_path = format!("{record_path}private.json");
                if self.read_file(repository, &private_path)?.is_none() {
                    return Err(StoreError);
                }
                self.validate_public_state(repository, record_path, expected)?;
                if let Some(current) = self.verified_outcome(
                    repository,
                    record_path,
                    &RecordId::try_from(expected.record_id.clone()).map_err(|_| StoreError)?,
                )? {
                    self.validate_public_state(repository, record_path, &current)?;
                    return if current == *expected {
                        Ok(true)
                    } else {
                        Err(StoreError)
                    };
                }
                self.write_file(repository, &path, bytes)?;
                self.push(repository)?;
                Ok(false)
            });
            if result.is_ok() {
                return Ok(());
            }
        }
        Err(StoreError)
    }

    fn validate_public_state(
        &self,
        repository: &Path,
        record_path: &str,
        outcome: &OutcomeStatement,
    ) -> Result<(), StoreError> {
        let public_path = format!("{record_path}public.json");
        let public = self.read_file(repository, &public_path)?;
        match (outcome.sink, outcome.outcome, public) {
            (SinkKind::PublicLog, _, None)
            | (SinkKind::PrivateStore, PublishOutcome::Refused, None) => Ok(()),
            (SinkKind::PublicLog, _, Some(_))
            | (SinkKind::None, _, _)
            | (SinkKind::PrivateStore, PublishOutcome::Published, None) => Err(StoreError),
            (SinkKind::PrivateStore, PublishOutcome::Published, Some(bytes)) => {
                let record = PublicRecord::from_json(&bytes).map_err(|_| StoreError)?;
                let canonical = record.entry_bytes().map_err(|_| StoreError)?;
                if canonical == bytes
                    && record.record_id.as_str() == outcome.record_id
                    && record.subject.reference().id() == self.publishing_repository
                {
                    Ok(())
                } else {
                    Err(StoreError)
                }
            }
            (SinkKind::PrivateStore, PublishOutcome::Refused, Some(bytes)) => {
                let record = PublicRecord::from_json(&bytes).map_err(|_| StoreError)?;
                if record.entry_bytes().map_err(|_| StoreError)? == bytes
                    && record.record_id.as_str() == outcome.record_id
                    && record.subject.reference().id() == self.publishing_repository
                {
                    Ok(())
                } else {
                    Err(StoreError)
                }
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SignedOutcome {
    protocol: String,
    statement: String,
    signature_bundle: String,
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum PublishOutcome {
    Published,
    Refused,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct OutcomeStatement {
    protocol: String,
    record_id: String,
    sink: SinkKind,
    outcome: PublishOutcome,
}

impl PrivateStore for GitPrivateStore<'_> {
    fn write_private(&self, destination: &str, private: &PrivateRecord) -> Result<(), StoreError> {
        let repository = Self::record_path(&private.record_id, &self.publishing_repository)?;
        let path = format!("{repository}private.json");
        let bytes = serde_json_canonicalizer::to_vec(private).map_err(|_| StoreError)?;
        self.commit_new_file(destination, &path, &bytes, true)
    }

    fn prepare_record(
        &self,
        destination: &str,
        private: &PrivateRecord,
        public: &PublicRecord,
        sink: SinkKind,
    ) -> Result<StorePreparation, StoreError> {
        if sink == SinkKind::None
            || private.record_id != public.record_id
            || public.subject.reference().id() != self.publishing_repository
        {
            return Err(StoreError);
        }
        let repository = Self::record_path(&private.record_id, public.subject.reference().id())?;
        let private_path = format!("{repository}private.json");
        let bytes = serde_json_canonicalizer::to_vec(private).map_err(|_| StoreError)?;
        for attempt in 0..MAX_PUSH_ATTEMPTS {
            let mut preparation = StorePreparation::OutcomeIncomplete;
            let result = self.with_repository(destination, |repo| {
                match self.read_file(repo, &private_path)? {
                    None => {
                        self.write_file(repo, &private_path, &bytes)?;
                        self.push(repo)?;
                        preparation = StorePreparation::NewRecord;
                    }
                    Some(existing) if existing == bytes => {
                        let Some(outcome) =
                            self.verified_outcome(repo, &repository, &private.record_id)?
                        else {
                            preparation = StorePreparation::OutcomeIncomplete;
                            return Ok(());
                        };
                        self.validate_public_state(repo, &repository, &outcome)?;
                        if outcome.sink != sink {
                            return Err(StoreError);
                        }
                        preparation = match outcome.outcome {
                            PublishOutcome::Published => StorePreparation::AlreadyPublished,
                            PublishOutcome::Refused => StorePreparation::AlreadyRefused,
                        };
                    }
                    Some(_) => return Err(StoreError),
                }
                Ok(())
            });
            match result {
                Ok(()) => return Ok(preparation),
                Err(StoreError) if attempt + 1 < MAX_PUSH_ATTEMPTS => {}
                Err(StoreError) => return Err(StoreError),
            }
        }
        Err(StoreError)
    }

    fn write_public(&self, destination: &str, public: &PublicRecord) -> Result<(), StoreError> {
        if public.subject.reference().id() != self.publishing_repository {
            return Err(StoreError);
        }
        let repository = Self::record_path(&public.record_id, public.subject.reference().id())?;
        let path = format!("{repository}public.json");
        let bytes = public.entry_bytes().map_err(|_| StoreError)?;
        self.commit_new_file(destination, &path, &bytes, true)
    }

    fn write_publish_outcome(
        &self,
        destination: &str,
        record_id: &RecordId,
        sink: SinkKind,
        outcome: ApprovalStatementPublishOutcome,
    ) -> Result<(), StoreError> {
        if sink == SinkKind::None {
            return Err(StoreError);
        }
        let statement = OutcomeStatement {
            protocol: PROTOCOL.to_owned(),
            record_id: record_id.as_str().to_owned(),
            sink,
            outcome: match outcome {
                ApprovalStatementPublishOutcome::Published => PublishOutcome::Published,
                ApprovalStatementPublishOutcome::Refused => PublishOutcome::Refused,
            },
        };
        let statement_bytes =
            serde_json_canonicalizer::to_vec(&statement).map_err(|_| StoreError)?;
        let signature_bundle = self
            .signer
            .sign_without_public_log(&statement_bytes)
            .map_err(|_| StoreError)?;
        if self
            .signer
            .signed_payload(&signature_bundle)
            .map_err(|_| StoreError)?
            != statement_bytes
            || self
                .verifier
                .verify(&statement_bytes, &signature_bundle, &self.publisher_policy)
                .is_err()
        {
            return Err(StoreError);
        }
        let envelope = SignedOutcome {
            protocol: ENVELOPE_PROTOCOL.to_owned(),
            statement: String::from_utf8(statement_bytes).map_err(|_| StoreError)?,
            signature_bundle,
        };
        let record_path = Self::record_path(record_id, &self.publishing_repository)?;
        let bytes = serde_json_canonicalizer::to_vec(&envelope).map_err(|_| StoreError)?;
        self.commit_outcome(destination, &record_path, &statement, &bytes)
    }
}

impl ApprovalStatementReader for GitPrivateStore<'_> {
    fn lookup_approval_statements(
        &self,
        destination: &str,
        lookup: ApprovalStatementLookup<'_>,
    ) -> Result<Vec<RetainedApprovalStatement>, ApprovalStatementReadError> {
        let result = self.with_repository(destination, |repository| {
            let prefix = Self::record_path_for_repository(lookup.repository)?;
            let listed = self.git(
                Some(repository),
                &["ls-tree", "-r", "--name-only", BRANCH, "--", &prefix],
            )?;
            require_success(&listed)?;
            let output = String::from_utf8(listed.stdout).map_err(|_| StoreError)?;
            if output.len() > MAX_GIT_OUTPUT {
                return Err(StoreError);
            }
            let mut files: Vec<String> = output
                .lines()
                .filter(|path| path.ends_with("/private.json"))
                .map(str::to_owned)
                .collect();
            files.sort();
            let mut statements = Vec::new();
            for private_path in files {
                let bytes = self
                    .read_file(repository, &private_path)?
                    .ok_or(StoreError)?;
                let private = PrivateRecord::from_json(&bytes).map_err(|_| StoreError)?;
                if serde_json_canonicalizer::to_vec(&private).map_err(|_| StoreError)? != bytes {
                    return Err(StoreError);
                }
                let record_directory = private_path
                    .strip_prefix(&prefix)
                    .and_then(|tail| tail.strip_suffix("/private.json"))
                    .ok_or(StoreError)?;
                if record_directory != private.record_id.as_str() {
                    return Err(StoreError);
                }
                let record_path = private_path
                    .strip_suffix("private.json")
                    .ok_or(StoreError)?;
                let outcome = self
                    .verified_outcome(repository, record_path, &private.record_id)?
                    .ok_or(StoreError)?;
                self.validate_public_state(repository, record_path, &outcome)?;
                let Some(envelope_value) = private.statement.as_ref() else {
                    continue;
                };
                let envelope: ApprovalEnvelope =
                    serde_json::from_value(envelope_value.clone()).map_err(|_| StoreError)?;
                let statement_bytes = serde_json_canonicalizer::to_vec(&envelope.statement)
                    .map_err(|_| StoreError)?;
                if statement_matches(
                    &statement_bytes,
                    lookup.repository,
                    lookup.merged_pull_request,
                    lookup.merged_commit,
                )? {
                    statements.push((
                        private.record_id.as_str().to_owned(),
                        RetainedApprovalStatement {
                            statement_bytes,
                            signature_bundle: envelope.signature_bundle,
                            publish_outcome: match outcome.outcome {
                                PublishOutcome::Published => {
                                    ApprovalStatementPublishOutcome::Published
                                }
                                PublishOutcome::Refused => ApprovalStatementPublishOutcome::Refused,
                            },
                        },
                    ));
                }
            }
            statements.sort_by(|left, right| left.0.cmp(&right.0));
            Ok(statements.into_iter().map(|(_, item)| item).collect())
        });
        result.map_err(|_| ApprovalStatementReadError)
    }
}

impl GitPrivateStore<'_> {
    fn record_path_for_repository(repository: &str) -> Result<String, StoreError> {
        let (owner, name) = repository.split_once('/').ok_or(StoreError)?;
        Ok(format!(
            "records/{}/{}/",
            hex_component(owner),
            hex_component(name)
        ))
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalEnvelope {
    statement: Value,
    signature_bundle: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalStatement {
    #[serde(rename = "_type")]
    type_: String,
    #[serde(rename = "predicateType")]
    predicate_type: String,
    subject: Vec<ApprovalSubject>,
    predicate: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ApprovalSubject {
    name: String,
    digest: std::collections::BTreeMap<String, String>,
}

fn statement_matches(
    bytes: &[u8],
    repository: &str,
    pull_request: &str,
    merged_commit: &str,
) -> Result<bool, StoreError> {
    let statement: ApprovalStatement = serde_json::from_slice(bytes).map_err(|_| StoreError)?;
    if statement.type_ != STATEMENT_TYPE || statement.predicate_type != APPROVAL_PREDICATE_TYPE {
        return Ok(false);
    }
    let subject_matches = matches!(statement.subject.as_slice(), [subject]
        if subject.name == repository
            && subject.digest.get("gitCommit").is_some_and(|value| value == merged_commit));
    let predicate = statement.predicate.as_object().ok_or(StoreError)?;
    let predicate_matches = predicate
        .get("pull_request")
        .and_then(Value::as_str)
        .is_some_and(|value| value == pull_request)
        && predicate
            .get("merged_commit")
            .and_then(Value::as_str)
            .is_some_and(|value| value == merged_commit);
    Ok(subject_matches && predicate_matches)
}

fn valid_repository(value: &str) -> bool {
    let Some((owner, name)) = value.split_once('/') else {
        return false;
    };
    let valid_component = |part: &str| {
        (1..=100).contains(&part.len())
            && part
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    };
    valid_component(owner) && valid_component(name)
}

fn canonical_repository(value: &str) -> String {
    value.to_ascii_lowercase()
}

fn hex_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len().saturating_mul(2));
    for byte in value.bytes() {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02x}");
    }
    encoded
}

fn require_success(output: &Output) -> Result<(), StoreError> {
    if output.status.success() {
        Ok(())
    } else {
        Err(StoreError)
    }
}

fn run_bounded_git(command: &mut Command) -> Result<Output, StoreError> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| StoreError)?;
    let mut stdout = child.stdout.take().ok_or(StoreError)?;
    let mut stderr = child.stderr.take().ok_or(StoreError)?;
    let stdout_reader = thread::spawn(move || read_capped(&mut stdout));
    let stderr_reader = thread::spawn(move || read_capped(&mut stderr));
    let deadline = Instant::now() + GIT_COMMAND_TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|_| StoreError)? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(StoreError);
        }
        thread::sleep(Duration::from_millis(10));
    };
    let stdout = stdout_reader.join().map_err(|_| StoreError)??;
    let stderr = stderr_reader.join().map_err(|_| StoreError)??;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn read_capped(reader: &mut impl Read) -> Result<Vec<u8>, StoreError> {
    let max_plus_one = u64::try_from(MAX_GIT_OUTPUT)
        .ok()
        .and_then(|value| value.checked_add(1))
        .ok_or(StoreError)?;
    let mut bytes = Vec::new();
    reader
        .take(max_plus_one)
        .read_to_end(&mut bytes)
        .map_err(|_| StoreError)?;
    if bytes.len() > MAX_GIT_OUTPUT {
        return Err(StoreError);
    }
    Ok(bytes)
}

fn path_arg(path: &Path) -> Result<&str, StoreError> {
    path.to_str().ok_or(StoreError)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        path::Path,
        process::Command,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use ix_trace_rs::trace;
    use serde_json::json;

    use super::*;
    use crate::{
        attestation_signing::VerificationFailure,
        record_sink::form::{Record, Signature},
    };

    const RECORD_ID: &str = "0b9d6a3e-4f1c-4a52-9d0e-6c1f7a2b8e34";
    const MERGED_COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
    const REPOSITORY: &str = "acme/ledger";
    const PULL_REQUEST: &str = "acme/ledger#42";
    static NEXT_CHECKOUT: AtomicUsize = AtomicUsize::new(0);
    const OIDC_ISSUER: &str = concat!("https:", "//token.actions.githubusercontent.com");

    fn identity_policy() -> Vec<u8> {
        format!(
            r#"{{"protocol":"engineering-assurance.workflow-identities/v1","identities":[{{"role":"record_publisher","issuer":"{OIDC_ISSUER}","repository":"acme/ledger","workflow_path":".github/workflows/publish.yml","ref":"refs/heads/main"}}]}}"#
        )
        .into_bytes()
    }

    struct FixturePolicySource {
        protected: bool,
    }

    impl Default for FixturePolicySource {
        fn default() -> Self {
            Self { protected: true }
        }
    }

    impl PolicySource for FixturePolicySource {
        fn base_branch_protected(&self) -> Option<bool> {
            Some(self.protected)
        }

        fn read_from_base_branch(&self, path: &str) -> Option<Vec<u8>> {
            (path == WORKFLOW_IDENTITIES_PATH).then(identity_policy)
        }
    }

    #[derive(Clone)]
    struct LocalResolver(String);

    impl GitDestinationResolver for LocalResolver {
        fn resolve(&self, destination: &str) -> Result<String, StoreError> {
            if destination == "agent-ix/private-records" {
                Ok(self.0.clone())
            } else {
                Err(StoreError)
            }
        }
    }

    #[derive(Clone, Default)]
    struct FixtureSignatures {
        next: Arc<AtomicUsize>,
        payloads: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    }

    impl Signer for FixtureSignatures {
        fn sign_for_public_log(
            &self,
            _proof: &crate::record_sink::leak::PublicPathProof<'_>,
        ) -> Result<String, crate::record_sink::sink::SigningFailed> {
            Err(crate::record_sink::sink::SigningFailed)
        }

        fn sign_without_public_log(
            &self,
            canonical: &[u8],
        ) -> Result<String, crate::record_sink::sink::SigningFailed> {
            let index = self.next.fetch_add(1, Ordering::SeqCst);
            let bundle = format!("fixture-outcome-{index}");
            self.payloads
                .lock()
                .map_err(|_| crate::record_sink::sink::SigningFailed)?
                .insert(bundle.clone(), canonical.to_vec());
            Ok(bundle)
        }

        fn signed_payload(
            &self,
            bundle: &str,
        ) -> Result<Vec<u8>, crate::record_sink::sink::SigningFailed> {
            self.payloads
                .lock()
                .map_err(|_| crate::record_sink::sink::SigningFailed)?
                .get(bundle)
                .cloned()
                .ok_or(crate::record_sink::sink::SigningFailed)
        }
    }

    impl BundleVerifier for FixtureSignatures {
        fn verify(
            &self,
            payload: &[u8],
            bundle_json: &str,
            policy: &WorkflowIdentityPolicy,
        ) -> Result<(), VerificationFailure> {
            let payloads = self
                .payloads
                .lock()
                .map_err(|_| VerificationFailure::InvalidSignature)?;
            if policy.repository == "acme/ledger"
                && payloads
                    .get(bundle_json)
                    .is_some_and(|signed| signed == payload)
            {
                Ok(())
            } else {
                Err(VerificationFailure::InvalidSignature)
            }
        }
    }

    fn git(executable: &Path, directory: Option<&Path>, args: &[&str]) {
        let mut command = Command::new(executable);
        if let Some(directory) = directory {
            command.arg("-C").arg(directory);
        }
        let output = command.args(args).output().expect("fixture git runs");
        assert!(
            output.status.success(),
            "git fixture command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_stdout(executable: &Path, args: &[&str]) -> String {
        let output = Command::new(executable)
            .args(args)
            .output()
            .expect("fixture git runs");
        assert!(output.status.success(), "git query succeeds");
        String::from_utf8(output.stdout)
            .expect("git object id is UTF-8")
            .trim()
            .to_owned()
    }

    fn git_output(executable: &Path, args: &[&str]) -> Vec<u8> {
        let output = Command::new(executable)
            .args(args)
            .output()
            .expect("fixture git runs");
        assert!(output.status.success(), "git query succeeds");
        output.stdout
    }

    fn update_remote_file(executable: &Path, bare: &Path, path: &str, bytes: &[u8], root: &Path) {
        let checkout = root.join(format!(
            "tamper-checkout-{}",
            NEXT_CHECKOUT.fetch_add(1, Ordering::SeqCst)
        ));
        git(
            executable,
            None,
            &[
                "clone",
                "--branch",
                BRANCH,
                path_arg(bare).expect("bare path"),
                path_arg(&checkout).expect("checkout path"),
            ],
        );
        fs::write(checkout.join(path), bytes).expect("fixture edit writes");
        git(executable, Some(&checkout), &["add", "--", path]);
        git(
            executable,
            Some(&checkout),
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-m",
                "tamper private fixture",
            ],
        );
        git(
            executable,
            Some(&checkout),
            &["push", "origin", &format!("HEAD:{PRIVATE_RECORDS_REF}")],
        );
    }

    fn fixture_git() -> PathBuf {
        ["/usr/bin/git", "/usr/local/bin/git"]
            .into_iter()
            .map(PathBuf::from)
            .find(|path| path.is_file())
            .expect("git executable is installed")
    }

    fn initialized_remote(root: &Path, executable: &Path) -> PathBuf {
        let bare = root.join("private-records.git");
        let seed = root.join("seed");
        git(
            executable,
            None,
            &["init", "--bare", path_arg(&bare).unwrap()],
        );
        git(
            executable,
            None,
            &["clone", path_arg(&bare).unwrap(), path_arg(&seed).unwrap()],
        );
        git(
            executable,
            Some(&seed),
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "checkout",
                "-b",
                "main",
            ],
        );
        fs::write(seed.join("README.md"), b"private fixture\n").expect("fixture file writes");
        git(executable, Some(&seed), &["add", "README.md"]);
        git(
            executable,
            Some(&seed),
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-m",
                "seed private fixture",
            ],
        );
        git(executable, Some(&seed), &["push", "-u", "origin", "main"]);
        git(
            executable,
            None,
            &[
                "--git-dir",
                path_arg(&bare).unwrap(),
                "symbolic-ref",
                "HEAD",
                "refs/heads/main",
            ],
        );
        bare
    }

    fn approval_record(repository: &str) -> Record {
        approval_record_with_id(repository, RECORD_ID)
    }

    fn approval_record_with_id(repository: &str, record_id: &str) -> Record {
        let statement = json!({
            "_type": STATEMENT_TYPE,
            "subject": [{"name": REPOSITORY, "digest": {"gitCommit": MERGED_COMMIT}}],
            "predicateType": APPROVAL_PREDICATE_TYPE,
            "predicate": {
                "source": "github",
                "approver": "github:1234",
                "approver_name": "fixture person",
                "role": "release_lead",
                "meaning": "approve release",
                "signed_at": "2026-11-14T10:00:00Z",
                "head_commit": "0123456789abcdef0123456789abcdef01234567",
                "pull_request": PULL_REQUEST,
                "merged_commit": MERGED_COMMIT
            }
        });
        let public = PublicRecord::from_json(
            json!({
                "protocol":"engineering-assurance.record-public/v1",
                "record_id":record_id,
                "kind":"approval",
                "subject":{"kind":"repo","id":repository},
                "references":[],
                "hashes":{"statement":"sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"},
                "verdicts":{"approval":"satisfied"},
                "timestamps":{"created":"2026-11-14T10:00:00Z"}
            })
            .to_string()
            .as_bytes(),
        )
        .expect("valid public form");
        let mut private = PrivateRecord::new(public.record_id.clone());
        private.statement = Some(json!({
            "statement": statement,
            "signature_bundle": "exact-approval-bundle"
        }));
        Record::new(public, private).expect("record forms share id")
    }

    fn store<'a>(
        resolver: &'a LocalResolver,
        signatures: &'a FixtureSignatures,
        git: &Path,
    ) -> GitPrivateStore<'a> {
        GitPrivateStore::new(
            resolver,
            signatures,
            signatures,
            &FixturePolicySource::default(),
            &RecordPublisherIdentity {
                repository: REPOSITORY.to_owned(),
                workflow_path: ".github/workflows/publish.yml".to_owned(),
                reference: "refs/heads/main".to_owned(),
            },
            REPOSITORY,
            git,
        )
        .expect("publisher identity is in protected policy")
    }

    #[trace(
        "TC-973",
        "FR-054-AC-7",
        "FR-054-AC-8",
        "TC-974",
        "FR-054-AC-8",
        "TC-975",
        "FR-054-AC-9",
        "TC-980",
        "FR-054-AC-14",
        "TC-982",
        "FR-054-AC-16"
    )]
    #[test]
    fn git_store_writes_canonical_forms_only_to_the_fixed_private_ref() {
        let temp = tempfile::tempdir().expect("temp dir");
        let git_executable = fixture_git();
        let bare = initialized_remote(temp.path(), &git_executable);
        let source_root = temp.path().join("source");
        fs::create_dir_all(&source_root).expect("source fixture directory");
        let source_bare = initialized_remote(&source_root, &git_executable);
        let remote = format!("file://{}", bare.display());
        let resolver = LocalResolver(remote);
        let signatures = FixtureSignatures::default();
        let unprotected = FixturePolicySource { protected: false };
        assert!(
            GitPrivateStore::new(
                &resolver,
                &signatures,
                &signatures,
                &unprotected,
                &RecordPublisherIdentity {
                    repository: REPOSITORY.to_owned(),
                    workflow_path: ".github/workflows/publish.yml".to_owned(),
                    reference: "refs/heads/main".to_owned(),
                },
                REPOSITORY,
                &git_executable,
            )
            .is_err()
        );
        let store = store(&resolver, &signatures, &git_executable);
        let record = approval_record(REPOSITORY);
        assert_eq!(
            store
                .prepare_record(
                    "agent-ix/private-records",
                    record.private(),
                    record.public(),
                    SinkKind::PrivateStore,
                )
                .expect("private form is prepared"),
            StorePreparation::NewRecord
        );
        assert_eq!(
            store
                .prepare_record(
                    "agent-ix/private-records",
                    record.private(),
                    record.public(),
                    SinkKind::PrivateStore,
                )
                .expect("an unfinished prior record fails closed"),
            StorePreparation::OutcomeIncomplete
        );
        let mut public = record.public().clone();
        public.signature = Some(Signature::try_from("dGVzdA==").expect("fixture signature"));
        store
            .write_public("agent-ix/private-records", &public)
            .expect("linked public form is stored");
        store
            .write_publish_outcome(
                "agent-ix/private-records",
                record.id(),
                SinkKind::PrivateStore,
                ApprovalStatementPublishOutcome::Published,
            )
            .expect("signed outcome is stored");
        let private_path = format!("records/61636d65/6c6564676572/{RECORD_ID}/private.json");
        let public_path = format!("records/61636d65/6c6564676572/{RECORD_ID}/public.json");
        let outcome_path = format!("records/61636d65/6c6564676572/{RECORD_ID}/outcome.json");
        let git_dir = ["--git-dir", bare.to_str().unwrap()];
        let stored_private = git_output(
            &git_executable,
            &[
                &git_dir[0],
                &git_dir[1],
                "show",
                &format!("{PRIVATE_RECORDS_REF}:{private_path}"),
            ],
        );
        assert_eq!(
            stored_private,
            serde_json_canonicalizer::to_vec(record.private()).expect("canonical private form")
        );
        let stored_public = git_output(
            &git_executable,
            &[
                &git_dir[0],
                &git_dir[1],
                "show",
                &format!("{PRIVATE_RECORDS_REF}:{public_path}"),
            ],
        );
        assert_eq!(
            stored_public,
            public.entry_bytes().expect("canonical public form")
        );
        let stored_outcome: SignedOutcome = serde_json::from_slice(&git_output(
            &git_executable,
            &[
                &git_dir[0],
                &git_dir[1],
                "show",
                &format!("{PRIVATE_RECORDS_REF}:{outcome_path}"),
            ],
        ))
        .expect("signed outcome envelope");
        assert_eq!(stored_outcome.protocol, ENVELOPE_PROTOCOL);
        let source_refs = git_output(
            &git_executable,
            &[
                "--git-dir",
                source_bare.to_str().unwrap(),
                "for-each-ref",
                "--format=%(refname)",
                "refs/heads/",
            ],
        );
        assert_eq!(source_refs, b"refs/heads/main\n");
        assert!(
            !Command::new(&git_executable)
                .args([
                    "--git-dir",
                    source_bare.to_str().unwrap(),
                    "show-ref",
                    "--verify",
                    "refs/notes/engineering-assurance-receipts"
                ])
                .output()
                .expect("source notes lookup runs")
                .status
                .success()
        );
        let private_path = format!("records/61636d65/6c6564676572/{RECORD_ID}/private.json");
        let private_blob = git_stdout(
            &git_executable,
            &[
                "--git-dir",
                bare.to_str().unwrap(),
                "rev-parse",
                &format!("{PRIVATE_RECORDS_REF}:{private_path}"),
            ],
        );
        store
            .write_private("agent-ix/private-records", record.private())
            .expect("identical private writes are idempotent");
        assert_eq!(
            git_stdout(
                &git_executable,
                &[
                    "--git-dir",
                    bare.to_str().unwrap(),
                    "rev-parse",
                    &format!("{PRIVATE_RECORDS_REF}:{private_path}"),
                ],
            ),
            private_blob
        );
        let mut changed_private = record.private().clone();
        changed_private.ticket_text = Some("different ticket".to_owned());
        assert!(
            store
                .write_private("agent-ix/private-records", &changed_private)
                .is_err()
        );
        assert_eq!(
            git_stdout(
                &git_executable,
                &[
                    "--git-dir",
                    bare.to_str().unwrap(),
                    "rev-parse",
                    &format!("{PRIVATE_RECORDS_REF}:{private_path}"),
                ],
            ),
            private_blob
        );
        let outcome_path = format!("records/61636d65/6c6564676572/{RECORD_ID}/outcome.json");
        let outcome_blob = git_stdout(
            &git_executable,
            &[
                "--git-dir",
                bare.to_str().unwrap(),
                "rev-parse",
                &format!("{PRIVATE_RECORDS_REF}:{outcome_path}"),
            ],
        );
        store
            .write_publish_outcome(
                "agent-ix/private-records",
                record.id(),
                SinkKind::PrivateStore,
                ApprovalStatementPublishOutcome::Published,
            )
            .expect("same final outcome is idempotent");
        assert!(
            store
                .write_publish_outcome(
                    "agent-ix/private-records",
                    record.id(),
                    SinkKind::PrivateStore,
                    ApprovalStatementPublishOutcome::Refused,
                )
                .is_err()
        );
        assert_eq!(
            git_stdout(
                &git_executable,
                &[
                    "--git-dir",
                    bare.to_str().unwrap(),
                    "rev-parse",
                    &format!("{PRIVATE_RECORDS_REF}:{outcome_path}"),
                ],
            ),
            outcome_blob
        );
        assert_eq!(
            store
                .prepare_record(
                    "agent-ix/private-records",
                    record.private(),
                    record.public(),
                    SinkKind::PrivateStore,
                )
                .expect("replay verifies the outcome"),
            StorePreparation::AlreadyPublished
        );
        let branch = Command::new(&git_executable)
            .args([
                "--git-dir",
                bare.to_str().unwrap(),
                "show-ref",
                "--verify",
                PRIVATE_RECORDS_REF,
            ])
            .output()
            .expect("show-ref runs");
        assert!(branch.status.success());
        let public_log_record =
            approval_record_with_id(REPOSITORY, "1b9d6a3e-4f1c-4a52-9d0e-6c1f7a2b8e35");
        assert_eq!(
            store
                .prepare_record(
                    "agent-ix/private-records",
                    public_log_record.private(),
                    public_log_record.public(),
                    SinkKind::PublicLog,
                )
                .expect("public-log record is prepared privately"),
            StorePreparation::NewRecord
        );
        store
            .write_publish_outcome(
                "agent-ix/private-records",
                public_log_record.id(),
                SinkKind::PublicLog,
                ApprovalStatementPublishOutcome::Published,
            )
            .expect("public-log outcome is retained without a public form");
        let public_log_path = format!(
            "records/61636d65/6c6564676572/{}/public.json",
            public_log_record.id().as_str()
        );
        assert!(
            store
                .with_repository("agent-ix/private-records", |repository| {
                    store.read_file(repository, &public_log_path)
                })
                .expect("public-log private store can be read")
                .is_none()
        );
    }

    #[trace("TC-977", "FR-054-AC-11")]
    #[test]
    fn path_components_encode_dot_and_dotdot_as_reversible_hex() {
        assert_eq!(hex_component("."), "2e");
        assert_eq!(hex_component(".."), "2e2e");
        assert_eq!(hex_component("a.b"), "612e62");
        let record_id = RecordId::try_from(RECORD_ID).expect("fixture record id");
        assert_eq!(
            GitPrivateStore::record_path(&record_id, "./..").expect("encoded path"),
            "records/2e/2e2e/0b9d6a3e-4f1c-4a52-9d0e-6c1f7a2b8e34/"
        );
    }

    #[trace(
        "TC-978",
        "FR-054-AC-12",
        "TC-989",
        "FR-064-AC-21",
        "TC-990",
        "FR-064-AC-22",
        "TC-991",
        "FR-064-AC-23",
        "TC-992",
        "FR-064-AC-24"
    )]
    #[test]
    fn approval_lookup_reads_exact_statement_bytes_bundle_and_signed_outcome() {
        let temp = tempfile::tempdir().expect("temp dir");
        let git_executable = fixture_git();
        let bare = initialized_remote(temp.path(), &git_executable);
        let resolver = LocalResolver(format!("file://{}", bare.display()));
        let signatures = FixtureSignatures::default();
        let store = store(&resolver, &signatures, &git_executable);
        let record = approval_record(REPOSITORY);
        store
            .prepare_record(
                "agent-ix/private-records",
                record.private(),
                record.public(),
                SinkKind::PrivateStore,
            )
            .expect("record prepared");
        let mut public = record.public().clone();
        public.signature = Some(Signature::try_from("dGVzdA==").expect("fixture signature"));
        store
            .write_public("agent-ix/private-records", &public)
            .expect("public form stored");
        store
            .write_publish_outcome(
                "agent-ix/private-records",
                record.id(),
                SinkKind::PrivateStore,
                ApprovalStatementPublishOutcome::Published,
            )
            .expect("signed outcome stored");

        let retained = store
            .lookup_approval_statements(
                "agent-ix/private-records",
                ApprovalStatementLookup {
                    repository: REPOSITORY,
                    merged_pull_request: PULL_REQUEST,
                    merged_commit: MERGED_COMMIT,
                },
            )
            .expect("exact approval statement is readable");
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].signature_bundle, "exact-approval-bundle");
        assert_eq!(
            serde_json_canonicalizer::to_vec(
                &serde_json::from_value::<Value>(
                    record.private().statement.as_ref().unwrap()["statement"].clone()
                )
                .expect("nested statement")
            )
            .expect("canonical nested statement"),
            retained[0].statement_bytes
        );
        assert_eq!(
            retained[0].publish_outcome,
            ApprovalStatementPublishOutcome::Published
        );
        assert_eq!(
            serde_json_canonicalizer::to_vec(
                &serde_json::from_slice::<Value>(&retained[0].statement_bytes,)
                    .expect("statement JSON")
            )
            .expect("canonical statement"),
            retained[0].statement_bytes
        );
        signatures
            .payloads
            .lock()
            .expect("fixture signature map")
            .insert(
                retained[0].signature_bundle.clone(),
                retained[0].statement_bytes.clone(),
            );
        assert!(
            store
                .verifier
                .verify(
                    &retained[0].statement_bytes,
                    &retained[0].signature_bundle,
                    &store.publisher_policy,
                )
                .is_ok()
        );
        let mut changed_statement = retained[0].statement_bytes.clone();
        if let Some(first) = changed_statement.first_mut() {
            *first = b' ';
        }
        assert!(
            store
                .verifier
                .verify(
                    &changed_statement,
                    &retained[0].signature_bundle,
                    &store.publisher_policy,
                )
                .is_err()
        );
        assert!(
            store
                .lookup_approval_statements(
                    "agent-ix/private-records",
                    ApprovalStatementLookup {
                        repository: REPOSITORY,
                        merged_pull_request: "acme/ledger#43",
                        merged_commit: MERGED_COMMIT,
                    },
                )
                .expect("different pull request returns no match")
                .is_empty()
        );
        assert!(
            store
                .lookup_approval_statements(
                    "agent-ix/private-records",
                    ApprovalStatementLookup {
                        repository: REPOSITORY,
                        merged_pull_request: PULL_REQUEST,
                        merged_commit: "ffffffffffffffffffffffffffffffffffffffff",
                    },
                )
                .expect("different merged commit returns no match")
                .is_empty()
        );
        let outcome_path = format!("records/61636d65/6c6564676572/{RECORD_ID}/outcome.json");
        let mut altered: SignedOutcome = serde_json::from_slice(&git_output(
            &git_executable,
            &[
                "--git-dir",
                bare.to_str().unwrap(),
                "show",
                &format!("{PRIVATE_RECORDS_REF}:{outcome_path}"),
            ],
        ))
        .expect("stored outcome parses");
        altered.signature_bundle = "changed-bundle".to_owned();
        let altered_bytes =
            serde_json_canonicalizer::to_vec(&altered).expect("canonical altered outcome");
        update_remote_file(
            &git_executable,
            &bare,
            &outcome_path,
            &altered_bytes,
            temp.path(),
        );
        assert!(
            store
                .lookup_approval_statements(
                    "agent-ix/private-records",
                    ApprovalStatementLookup {
                        repository: REPOSITORY,
                        merged_pull_request: PULL_REQUEST,
                        merged_commit: MERGED_COMMIT,
                    },
                )
                .is_err()
        );
        update_remote_file(
            &git_executable,
            &bare,
            &outcome_path,
            b"{ malformed",
            temp.path(),
        );
        assert!(
            store
                .lookup_approval_statements(
                    "agent-ix/private-records",
                    ApprovalStatementLookup {
                        repository: REPOSITORY,
                        merged_pull_request: PULL_REQUEST,
                        merged_commit: MERGED_COMMIT,
                    },
                )
                .is_err()
        );
    }
}
