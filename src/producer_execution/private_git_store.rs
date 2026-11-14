// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Git-backed private record retention at the process-capable boundary.
//!
//! The domain sink remains I/O-free. This adapter resolves the configured
//! `owner/name` destination, stores canonical record forms on the one fixed
//! branch, and delegates all outcome signing and verification to the supplied
//! Sigstore ports.

use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
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
        config::{PolicySource, SinkKind, valid_repository},
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
const TEMP_STORE_DIRECTORY_ATTEMPTS: usize = 32;
static NEXT_TEMP_STORE_DIRECTORY: AtomicU64 = AtomicU64::new(0);

/// One exclusively created, permission-restricted scratch directory per Git operation.
/// This avoids adding a general-purpose temporary-file crate to the standalone
/// private-store feature's runtime dependency graph.
struct OwnedTempDirectory {
    path: PathBuf,
}

impl OwnedTempDirectory {
    fn create() -> Result<Self, StoreError> {
        let root = std::env::temp_dir();
        for _ in 0..TEMP_STORE_DIRECTORY_ATTEMPTS {
            let sequence = NEXT_TEMP_STORE_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = root.join(format!(
                "ea-private-store-{}-{sequence}",
                std::process::id()
            ));
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            match builder.create(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(StoreError),
            }
        }
        Err(StoreError)
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for OwnedTempDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

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
        let temp = OwnedTempDirectory::create()?;
        let repository = temp.path().join("store");
        let initialized = self.git(None, &["init", "--quiet", path_arg(&repository)?])?;
        require_success(&initialized)?;
        let origin = self.git(Some(&repository), &["remote", "add", "origin", &remote])?;
        require_success(&origin)?;
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
        create_checkout_parents(repository, parent)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&absolute)
            .map_err(|_| StoreError)?;
        file.write_all(bytes).map_err(|_| StoreError)?;
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
    ) -> Result<Option<OutcomeStatement>, OutcomeReadError> {
        let path = format!("{record_path}outcome.json");
        let Some(bytes) = self
            .read_file(repository, &path)
            .map_err(|_| OutcomeReadError::Store)?
        else {
            return Ok(None);
        };
        let envelope: SignedOutcome =
            serde_json::from_slice(&bytes).map_err(|_| OutcomeReadError::Invalid)?;
        if envelope.protocol != ENVELOPE_PROTOCOL {
            return Err(OutcomeReadError::Invalid);
        }
        if serde_json_canonicalizer::to_vec(&envelope).map_err(|_| OutcomeReadError::Invalid)?
            != bytes
        {
            return Err(OutcomeReadError::Invalid);
        }
        let statement_bytes = envelope.statement.as_bytes();
        let statement: OutcomeStatement =
            serde_json::from_slice(statement_bytes).map_err(|_| OutcomeReadError::Invalid)?;
        if statement.protocol != PROTOCOL
            || statement.record_id != record_id.as_str()
            || statement.sink == SinkKind::None
        {
            return Err(OutcomeReadError::Invalid);
        }
        let canonical =
            serde_json_canonicalizer::to_vec(&statement).map_err(|_| OutcomeReadError::Invalid)?;
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
            return Err(OutcomeReadError::Invalid);
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
                if let Some(current) = self
                    .verified_outcome(
                        repository,
                        record_path,
                        &RecordId::try_from(expected.record_id.clone()).map_err(|_| StoreError)?,
                    )
                    .map_err(|_| StoreError)?
                {
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

#[derive(Clone, Copy)]
enum OutcomeReadError {
    Invalid,
    Store,
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
                        let outcome =
                            match self.verified_outcome(repo, &repository, &private.record_id) {
                                Ok(Some(outcome)) => outcome,
                                Ok(None) | Err(OutcomeReadError::Invalid) => {
                                    preparation = StorePreparation::OutcomeIncomplete;
                                    return Ok(());
                                }
                                Err(OutcomeReadError::Store) => return Err(StoreError),
                            };
                        if self
                            .validate_public_state(repo, &repository, &outcome)
                            .is_err()
                        {
                            preparation = StorePreparation::OutcomeIncomplete;
                            return Ok(());
                        }
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
                    .verified_outcome(repository, record_path, &private.record_id)
                    .map_err(|_| StoreError)?
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

fn create_checkout_parents(root: &Path, parent: &Path) -> Result<(), StoreError> {
    let relative = parent.strip_prefix(root).map_err(|_| StoreError)?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(StoreError);
        };
        current.push(name);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&current).map_err(|_| StoreError)?;
            }
            Ok(_) | Err(_) => return Err(StoreError),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        hash::{Hash, Hasher},
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
    type FixturePayloads = Arc<Mutex<BTreeMap<String, (Vec<u8>, String)>>>;

    fn identity_policy(repository: &str) -> Vec<u8> {
        format!(
            r#"{{"protocol":"engineering-assurance.workflow-identities/v1","identities":[{{"role":"record_publisher","issuer":"{OIDC_ISSUER}","repository":"{repository}","workflow_path":".github/workflows/publish.yml","ref":"refs/heads/main"}}]}}"#
        )
        .into_bytes()
    }

    struct FixturePolicySource {
        protected: bool,
        publisher_repository: String,
    }

    impl Default for FixturePolicySource {
        fn default() -> Self {
            Self {
                protected: true,
                publisher_repository: REPOSITORY.to_owned(),
            }
        }
    }

    impl PolicySource for FixturePolicySource {
        fn base_branch_protected(&self) -> Option<bool> {
            Some(self.protected)
        }

        fn read_from_base_branch(&self, path: &str) -> Option<Vec<u8>> {
            (path == WORKFLOW_IDENTITIES_PATH).then(|| identity_policy(&self.publisher_repository))
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

    #[derive(Clone)]
    struct FixtureSignatures {
        payloads: FixturePayloads,
        signing_repository: String,
    }

    impl Default for FixtureSignatures {
        fn default() -> Self {
            Self::for_repository(REPOSITORY)
        }
    }

    impl FixtureSignatures {
        fn for_repository(repository: &str) -> Self {
            Self {
                payloads: Arc::default(),
                signing_repository: repository.to_owned(),
            }
        }

        fn delegated_to(&self, repository: &str) -> Self {
            Self {
                payloads: self.payloads.clone(),
                signing_repository: repository.to_owned(),
            }
        }

        fn bundle_for(payload: &[u8], repository: &str) -> String {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            repository.hash(&mut hasher);
            payload.hash(&mut hasher);
            format!("AA{:016x}==", hasher.finish())
        }
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
            let bundle = Self::bundle_for(canonical, &self.signing_repository);
            self.payloads
                .lock()
                .map_err(|_| crate::record_sink::sink::SigningFailed)?
                .insert(
                    bundle.clone(),
                    (canonical.to_vec(), self.signing_repository.clone()),
                );
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
                .map(|(payload, _)| payload.clone())
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
            if bundle_json == Self::bundle_for(payload, &policy.repository) {
                Ok(())
            } else {
                Err(VerificationFailure::InvalidSignature)
            }
        }
    }

    struct CountingStore<'a> {
        inner: &'a GitPrivateStore<'a>,
        sink_calls: &'a Path,
    }

    impl ApprovalStatementReader for CountingStore<'_> {
        fn lookup_approval_statements(
            &self,
            destination: &str,
            lookup: ApprovalStatementLookup<'_>,
        ) -> Result<Vec<RetainedApprovalStatement>, ApprovalStatementReadError> {
            self.inner.lookup_approval_statements(destination, lookup)
        }
    }

    impl PrivateStore for CountingStore<'_> {
        fn write_private(
            &self,
            destination: &str,
            private: &PrivateRecord,
        ) -> Result<(), StoreError> {
            self.inner.write_private(destination, private)
        }

        fn prepare_record(
            &self,
            destination: &str,
            private: &PrivateRecord,
            public: &PublicRecord,
            sink: SinkKind,
        ) -> Result<StorePreparation, StoreError> {
            self.inner
                .prepare_record(destination, private, public, sink)
        }

        fn write_public(&self, destination: &str, public: &PublicRecord) -> Result<(), StoreError> {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.sink_calls)
                .map_err(|_| StoreError)?;
            file.write_all(b"sink\n").map_err(|_| StoreError)?;
            self.inner.write_public(destination, public)
        }

        fn write_publish_outcome(
            &self,
            destination: &str,
            record_id: &RecordId,
            sink: SinkKind,
            outcome: ApprovalStatementPublishOutcome,
        ) -> Result<(), StoreError> {
            self.inner
                .write_publish_outcome(destination, record_id, sink, outcome)
        }
    }

    struct PublicVisibility;

    impl crate::record_sink::leak::VisibilityLookup for PublicVisibility {
        fn repo_visibility(
            &self,
            _repo: &str,
        ) -> Result<crate::record_sink::config::Visibility, crate::record_sink::leak::LookupFailed>
        {
            Ok(crate::record_sink::config::Visibility::Public)
        }
    }

    struct UnusedPublicLog;

    impl crate::record_sink::sink::PublicLogClient for UnusedPublicLog {
        fn send(
            &self,
            _endpoint: &str,
            _entry: &crate::record_sink::sink::CheckedEntry,
        ) -> Result<(), crate::record_sink::sink::LogRejected> {
            Err(crate::record_sink::sink::LogRejected)
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

    fn remove_remote_file(executable: &Path, bare: &Path, path: &str, root: &Path) {
        let checkout = root.join(format!(
            "remove-checkout-{}",
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
        git(executable, Some(&checkout), &["rm", "--", path]);
        git(
            executable,
            Some(&checkout),
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-m",
                "remove private fixture file",
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

    fn add_remote_branch(root: &Path, bare: &Path, executable: &Path, name: &str) {
        let checkout = root.join(format!("branch-{name}"));
        git(
            executable,
            None,
            &[
                "clone",
                "--branch",
                "main",
                path_arg(bare).unwrap(),
                path_arg(&checkout).unwrap(),
            ],
        );
        git(executable, Some(&checkout), &["checkout", "-b", name]);
        fs::write(checkout.join("unrelated.txt"), b"not fetched\n")
            .expect("unrelated branch fixture writes");
        git(executable, Some(&checkout), &["add", "unrelated.txt"]);
        git(
            executable,
            Some(&checkout),
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-m",
                "add unrelated branch",
            ],
        );
        git(
            executable,
            Some(&checkout),
            &["push", "origin", &format!("HEAD:refs/heads/{name}")],
        );
    }

    #[cfg(unix)]
    fn add_owner_symlink_branch(root: &Path, bare: &Path, executable: &Path, outside: &Path) {
        use std::os::unix::fs::symlink;

        let checkout = root.join("symlink-seed");
        git(
            executable,
            None,
            &[
                "clone",
                "--branch",
                "main",
                path_arg(bare).unwrap(),
                path_arg(&checkout).unwrap(),
            ],
        );
        fs::create_dir_all(checkout.join("records")).expect("records fixture directory");
        symlink(outside, checkout.join("records/61636d65"))
            .expect("fixture uses an absolute symlink outside the checkout");
        git(executable, Some(&checkout), &["add", "records"]);
        git(
            executable,
            Some(&checkout),
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "-m",
                "seed malicious symlink",
            ],
        );
        git(
            executable,
            Some(&checkout),
            &["push", "origin", &format!("HEAD:{PRIVATE_RECORDS_REF}")],
        );
    }

    fn approval_record(signatures: &FixtureSignatures, repository: &str) -> Record {
        approval_record_with_id(signatures, repository, RECORD_ID)
    }

    fn approval_record_with_id(
        signatures: &FixtureSignatures,
        repository: &str,
        record_id: &str,
    ) -> Record {
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
        let statement_bytes =
            serde_json_canonicalizer::to_vec(&statement).expect("canonical approval statement");
        let signature_bundle = signatures
            .sign_without_public_log(&statement_bytes)
            .expect("fixture signer returns the approval bundle");
        let mut private = PrivateRecord::new(public.record_id.clone());
        private.statement = Some(json!({
            "statement": statement,
            "signature_bundle": signature_bundle
        }));
        Record::new(public, private).expect("record forms share id")
    }

    fn signed_approval_fixture(
        record: &Record,
        signatures: &FixtureSignatures,
    ) -> (Vec<u8>, String) {
        let private_statement = record.private().statement.as_ref().expect("approval form");
        let statement_bytes = serde_json_canonicalizer::to_vec(&private_statement["statement"])
            .expect("canonical approval statement");
        let signature_bundle = private_statement["signature_bundle"]
            .as_str()
            .expect("approval bundle is a string")
            .to_owned();
        assert!(
            signatures
                .payloads
                .lock()
                .expect("fixture signature map")
                .get(&signature_bundle)
                .is_some_and(|(payload, repository)| {
                    payload == &statement_bytes && repository == REPOSITORY
                }),
            "fixture signer established this trusted bundle before storage"
        );
        (statement_bytes, signature_bundle)
    }

    fn persist_record_outcome(
        store: &GitPrivateStore<'_>,
        signatures: &FixtureSignatures,
        record: &Record,
        sink: SinkKind,
        outcome: ApprovalStatementPublishOutcome,
    ) {
        assert_eq!(
            store
                .prepare_record(
                    "agent-ix/private-records",
                    record.private(),
                    record.public(),
                    sink,
                )
                .expect("record prepares"),
            StorePreparation::NewRecord
        );
        if sink == SinkKind::PrivateStore && outcome == ApprovalStatementPublishOutcome::Published {
            let bytes = record.public().signed_bytes().expect("public bytes");
            let bundle = signatures
                .sign_without_public_log(&bytes)
                .expect("fixture signature");
            let mut public = record.public().clone();
            public.signature = Some(Signature::try_from(bundle).expect("valid fixture bundle"));
            store
                .write_public("agent-ix/private-records", &public)
                .expect("public form is linked");
        }
        store
            .write_publish_outcome("agent-ix/private-records", record.id(), sink, outcome)
            .expect("signed terminal outcome is retained");
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

    fn install_push_hook(bare: &Path, counter: &Path, wait_for: Option<usize>, reject: bool) {
        use std::os::unix::fs::PermissionsExt;

        let counter = path_arg(counter).expect("counter path");
        assert!(!counter.contains('\''), "temp paths have no shell quote");
        let barrier = wait_for.map_or_else(String::new, |target| {
            format!("while [ \"$(wc -l < '{counter}')\" -lt {target} ]; do sleep 0.01; done\n")
        });
        let reject = if reject { "exit 1" } else { "exit 0" };
        let script = format!("#!/bin/sh\nprintf 'push\\n' >> '{counter}'\n{barrier}{reject}\n");
        let hook = bare.join("hooks/pre-receive");
        fs::write(&hook, script).expect("pre-receive hook is installed");
        let mut permissions = fs::metadata(&hook).expect("hook metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&hook, permissions).expect("hook is executable");
    }

    fn spawn_test_worker(
        test_name: &str,
        bare: &Path,
        record_id: &str,
        counter: &Path,
    ) -> std::process::Child {
        let test = format!("producer_execution::private_git_store::tests::{test_name}");
        Command::new(std::env::current_exe().expect("current test executable"))
            .args(["--exact", &test, "--nocapture"])
            .env(
                "EA_PRIVATE_STORE_REMOTE",
                format!("file://{}", bare.display()),
            )
            .env("EA_PRIVATE_STORE_RECORD_ID", record_id)
            .env("EA_PRIVATE_STORE_COUNTER", counter)
            .spawn()
            .expect("test worker starts")
    }

    fn private_write_worker_if_requested() -> bool {
        let Ok(remote) = std::env::var("EA_PRIVATE_STORE_REMOTE") else {
            return false;
        };
        let record_id = std::env::var("EA_PRIVATE_STORE_RECORD_ID").expect("worker record id");
        let resolver = LocalResolver(remote);
        let signatures = FixtureSignatures::default();
        let store = store(&resolver, &signatures, &fixture_git());
        let record = approval_record_with_id(&signatures, REPOSITORY, &record_id);
        store
            .write_private("agent-ix/private-records", record.private())
            .expect("concurrent private write succeeds after bounded retry");
        true
    }

    fn same_id_sink_worker_if_requested() -> bool {
        let Ok(remote) = std::env::var("EA_PRIVATE_STORE_REMOTE") else {
            return false;
        };
        let record_id = std::env::var("EA_PRIVATE_STORE_RECORD_ID").expect("worker record id");
        let counter =
            PathBuf::from(std::env::var("EA_PRIVATE_STORE_COUNTER").expect("worker call counter"));
        let resolver = LocalResolver(remote);
        let signatures = FixtureSignatures::default();
        let git_executable = fixture_git();
        let private_store = store(&resolver, &signatures, &git_executable);
        let counted_store = CountingStore {
            inner: &private_store,
            sink_calls: &counter,
        };
        let lookup = PublicVisibility;
        let client = UnusedPublicLog;
        let ports = crate::record_sink::sink::Ports {
            store: &counted_store,
            lookup: &lookup,
            signer: &signatures,
            client: &client,
        };
        let selection = crate::record_sink::config::SinkSelection {
            sink: SinkKind::PrivateStore,
            source: crate::record_sink::config::SelectionSource::Configured,
            public_log_endpoint: None,
            private_store_destination: Some("agent-ix/private-records".to_owned()),
            public_linear_teams: Vec::new(),
        };
        let layer = crate::record_sink::sink::SinkLayer::new(&selection, REPOSITORY, ports);
        let record = approval_record_with_id(&signatures, REPOSITORY, &record_id);
        let result = crate::record_sink::sink::RecordSink::publish(&layer, &record);
        fs::write(
            counter.with_extension(format!("{}.result", std::process::id())),
            format!("{:?}", result.status),
        )
        .expect("worker result is retained");
        true
    }

    fn assert_worker_succeeded(worker: std::process::Child) {
        let output = worker.wait_with_output().expect("worker completes");
        assert!(
            output.status.success(),
            "worker failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[trace("TC-976", "FR-054-AC-10")]
    #[test]
    fn concurrent_distinct_record_writes_retry_without_losing_either_file() {
        if private_write_worker_if_requested() {
            return;
        }
        let temp = tempfile::tempdir().expect("temp dir");
        let git_executable = fixture_git();
        let bare = initialized_remote(temp.path(), &git_executable);
        let resolver = LocalResolver(format!("file://{}", bare.display()));
        let signatures = FixtureSignatures::default();
        let private_store = store(&resolver, &signatures, &git_executable);
        let seed = approval_record_with_id(
            &signatures,
            REPOSITORY,
            "1b9d6a3e-4f1c-4a52-9d0e-6c1f7a2b8e30",
        );
        private_store
            .write_private("agent-ix/private-records", seed.private())
            .expect("private branch is seeded before the concurrent writes");
        let pushes = temp.path().join("concurrent-pushes");
        install_push_hook(&bare, &pushes, Some(2), false);
        let ids = [
            "1b9d6a3e-4f1c-4a52-9d0e-6c1f7a2b8e31",
            "1b9d6a3e-4f1c-4a52-9d0e-6c1f7a2b8e32",
        ];
        let workers: Vec<_> = ids
            .iter()
            .map(|id| {
                spawn_test_worker(
                    "concurrent_distinct_record_writes_retry_without_losing_either_file",
                    &bare,
                    id,
                    &pushes,
                )
            })
            .collect();
        for worker in workers {
            assert_worker_succeeded(worker);
        }
        let refs = git_output(
            &git_executable,
            &[
                "--git-dir",
                path_arg(&bare).unwrap(),
                "ls-tree",
                "-r",
                "--name-only",
                PRIVATE_RECORDS_REF,
            ],
        );
        let refs = String::from_utf8(refs).expect("private tree filenames are UTF-8");
        assert!(refs.contains(&format!("{}/private.json", ids[0])));
        assert!(refs.contains(&format!("{}/private.json", ids[1])));
        assert!(
            fs::read_to_string(pushes)
                .expect("push attempts are logged")
                .lines()
                .count()
                >= 3,
            "the non-fast-forward writer refreshed and retried"
        );
    }

    #[trace("TC-975", "FR-054-AC-9")]
    #[test]
    fn concurrent_reapplication_of_identical_private_bytes_survives_non_fast_forward() {
        if private_write_worker_if_requested() {
            return;
        }
        let temp = tempfile::tempdir().expect("temp dir");
        let git_executable = fixture_git();
        let bare = initialized_remote(temp.path(), &git_executable);
        let pushes = temp.path().join("same-bytes-pushes");
        install_push_hook(&bare, &pushes, Some(2), false);
        let workers: Vec<_> = (0..2)
            .map(|_| {
                spawn_test_worker(
                    "concurrent_reapplication_of_identical_private_bytes_survives_non_fast_forward",
                    &bare,
                    RECORD_ID,
                    &pushes,
                )
            })
            .collect();
        for worker in workers {
            assert_worker_succeeded(worker);
        }
        let private_path = format!(
            "{}private.json",
            GitPrivateStore::record_path(
                &RecordId::try_from(RECORD_ID).expect("record id"),
                REPOSITORY
            )
            .expect("record path")
        );
        let stored = git_output(
            &git_executable,
            &[
                "--git-dir",
                path_arg(&bare).unwrap(),
                "show",
                &format!("{PRIVATE_RECORDS_REF}:{private_path}"),
            ],
        );
        let signatures = FixtureSignatures::default();
        let record = approval_record(&signatures, REPOSITORY);
        assert_eq!(
            stored,
            serde_json_canonicalizer::to_vec(record.private()).expect("canonical private form")
        );
        assert!(
            fs::read_to_string(pushes)
                .expect("push attempts are logged")
                .lines()
                .count()
                >= 2,
            "one identical writer had to retry the conflicting push"
        );
    }

    #[trace("TC-976", "FR-054-AC-10")]
    #[test]
    fn five_rejected_pushes_leave_the_remote_ref_unchanged() {
        let temp = tempfile::tempdir().expect("temp dir");
        let git_executable = fixture_git();
        let bare = initialized_remote(temp.path(), &git_executable);
        let pushes = temp.path().join("rejected-pushes");
        install_push_hook(&bare, &pushes, None, true);
        let resolver = LocalResolver(format!("file://{}", bare.display()));
        let signatures = FixtureSignatures::default();
        let store = store(&resolver, &signatures, &git_executable);
        let record = approval_record(&signatures, REPOSITORY);
        assert!(
            store
                .write_private("agent-ix/private-records", record.private())
                .is_err(),
            "the bounded five-attempt write fails closed"
        );
        assert_eq!(
            fs::read_to_string(pushes)
                .expect("rejected pushes are logged")
                .lines()
                .count(),
            MAX_PUSH_ATTEMPTS
        );
        assert!(
            !Command::new(&git_executable)
                .args([
                    "--git-dir",
                    path_arg(&bare).unwrap(),
                    "show-ref",
                    "--verify",
                    PRIVATE_RECORDS_REF,
                ])
                .output()
                .expect("fixed-ref query runs")
                .status
                .success()
        );
    }

    #[trace("TC-984", "FR-054-AC-18")]
    #[test]
    fn concurrent_same_id_publish_invokes_the_selected_sink_at_most_once() {
        if same_id_sink_worker_if_requested() {
            return;
        }
        let temp = tempfile::tempdir().expect("temp dir");
        let git_executable = fixture_git();
        let bare = initialized_remote(temp.path(), &git_executable);
        let pushes = temp.path().join("same-id-pushes");
        let sink_calls = temp.path().join("selected-sink-calls");
        install_push_hook(&bare, &pushes, Some(2), false);
        let workers: Vec<_> = (0..2)
            .map(|_| {
                spawn_test_worker(
                    "concurrent_same_id_publish_invokes_the_selected_sink_at_most_once",
                    &bare,
                    RECORD_ID,
                    &sink_calls,
                )
            })
            .collect();
        for worker in workers {
            assert_worker_succeeded(worker);
        }
        assert_eq!(
            fs::read_to_string(&sink_calls).expect("the winning sink invocation is logged"),
            "sink\n",
            "only the process that prepared a new record invokes the sink"
        );
        let worker_results: Vec<_> = fs::read_dir(temp.path())
            .expect("worker result directory")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("selected-sink-calls.")
            })
            .map(|entry| fs::read_to_string(entry.path()).expect("worker result readable"))
            .collect();
        assert_eq!(
            worker_results.len(),
            2,
            "worker statuses: {worker_results:?}"
        );
        let resolver = LocalResolver(format!("file://{}", bare.display()));
        let signatures = FixtureSignatures::default();
        let store = store(&resolver, &signatures, &git_executable);
        let record = approval_record(&signatures, REPOSITORY);
        assert_eq!(
            store
                .prepare_record(
                    "agent-ix/private-records",
                    record.private(),
                    record.public(),
                    SinkKind::PrivateStore,
                )
                .expect("winning terminal outcome is readable"),
            StorePreparation::AlreadyPublished,
            "worker statuses: {worker_results:?}"
        );
    }

    #[trace("TC-993", "FR-055-AC-9")]
    #[test]
    fn private_store_fetches_only_its_fixed_ref_and_rejects_ref_configuration() {
        let temp = tempfile::tempdir().expect("temp dir");
        let git_executable = fixture_git();
        let bare = initialized_remote(temp.path(), &git_executable);
        let resolver = LocalResolver(format!("file://{}", bare.display()));
        let signatures = FixtureSignatures::default();
        let store = store(&resolver, &signatures, &git_executable);
        let record = approval_record(&signatures, REPOSITORY);
        store
            .write_private("agent-ix/private-records", record.private())
            .expect("fixed private branch is created");
        add_remote_branch(temp.path(), &bare, &git_executable, "unrelated-secret");

        let tracking_refs = store
            .with_repository("agent-ix/private-records", |repository| {
                let output = store.git(
                    Some(repository),
                    &["for-each-ref", "--format=%(refname)", "refs/remotes/origin"],
                )?;
                require_success(&output)?;
                String::from_utf8(output.stdout).map_err(|_| StoreError)
            })
            .expect("only configured store ref is fetched");
        assert_eq!(tracking_refs, format!("refs/remotes/origin/{BRANCH}\n"));
        assert!(
            crate::record_sink::config::SinkConfig::parse(
                br#"{"protocol":"engineering-assurance.sink-config/v1","private_store":{"destination":"agent-ix/private-records","ref":"refs/heads/other"}}"#
            )
            .is_err(),
            "an alternative ref member is rejected by the closed sink config"
        );
    }

    #[trace("TC-987", "FR-056-AC-11")]
    #[test]
    fn approval_envelope_keeps_exact_statement_and_bundle_and_rejects_unknown_members() {
        let signatures = FixtureSignatures::default();
        let record = approval_record(&signatures, REPOSITORY);
        let (signed_statement, signed_bundle) = signed_approval_fixture(&record, &signatures);
        let private_envelope = record.private().statement.as_ref().expect("private form");
        let decoded: ApprovalEnvelope = serde_json::from_value(private_envelope.clone())
            .expect("exact statement envelope is accepted");
        assert_eq!(
            serde_json_canonicalizer::to_vec(&decoded.statement).expect("canonical statement"),
            signed_statement
        );
        assert_eq!(decoded.signature_bundle, signed_bundle);

        let mut unknown = private_envelope.clone();
        unknown
            .as_object_mut()
            .expect("statement envelope object")
            .insert("unexpected".to_owned(), json!(true));
        assert!(
            serde_json::from_value::<ApprovalEnvelope>(unknown).is_err(),
            "unknown statement-envelope members fail closed"
        );
    }

    #[trace("TC-990", "FR-064-AC-22")]
    #[test]
    fn malformed_private_form_and_git_read_failure_return_no_partial_lookup() {
        let temp = tempfile::tempdir().expect("temp dir");
        let git_executable = fixture_git();
        let bare = initialized_remote(temp.path(), &git_executable);
        let resolver = LocalResolver(format!("file://{}", bare.display()));
        let signatures = FixtureSignatures::default();
        let private_store = store(&resolver, &signatures, &git_executable);
        let first = approval_record_with_id(
            &signatures,
            REPOSITORY,
            "0a9d6a3e-4f1c-4a52-9d0e-6c1f7a2b8e34",
        );
        let second = approval_record_with_id(
            &signatures,
            REPOSITORY,
            "1b9d6a3e-4f1c-4a52-9d0e-6c1f7a2b8e34",
        );
        persist_record_outcome(
            &private_store,
            &signatures,
            &first,
            SinkKind::PrivateStore,
            ApprovalStatementPublishOutcome::Published,
        );
        persist_record_outcome(
            &private_store,
            &signatures,
            &second,
            SinkKind::PrivateStore,
            ApprovalStatementPublishOutcome::Published,
        );
        let missing_outcome = approval_record_with_id(
            &signatures,
            REPOSITORY,
            "2b9d6a3e-4f1c-4a52-9d0e-6c1f7a2b8e34",
        );
        assert_eq!(
            private_store
                .prepare_record(
                    "agent-ix/private-records",
                    missing_outcome.private(),
                    missing_outcome.public(),
                    SinkKind::PublicLog,
                )
                .expect("missing-outcome fixture is prepared"),
            StorePreparation::NewRecord
        );
        let lookup = ApprovalStatementLookup {
            repository: REPOSITORY,
            merged_pull_request: PULL_REQUEST,
            merged_commit: MERGED_COMMIT,
        };
        assert!(
            private_store
                .lookup_approval_statements("agent-ix/private-records", lookup)
                .is_err(),
            "a missing signed outcome makes the whole lookup unreadable"
        );
        private_store
            .write_publish_outcome(
                "agent-ix/private-records",
                missing_outcome.id(),
                SinkKind::PublicLog,
                ApprovalStatementPublishOutcome::Refused,
            )
            .expect("fixture now has a signed refusal outcome");
        let second_private = format!(
            "{}private.json",
            GitPrivateStore::record_path(second.id(), REPOSITORY).expect("record path")
        );
        update_remote_file(
            &git_executable,
            &bare,
            &second_private,
            b"{ malformed private form",
            temp.path(),
        );
        assert!(
            private_store
                .lookup_approval_statements("agent-ix/private-records", lookup)
                .is_err(),
            "the first matching record is not returned partially when a later form is malformed"
        );

        let missing_resolver = LocalResolver(format!(
            "file://{}",
            temp.path().join("missing-private-store.git").display()
        ));
        let missing_store = store(&missing_resolver, &signatures, &git_executable);
        assert!(
            missing_store
                .lookup_approval_statements("agent-ix/private-records", lookup)
                .is_err(),
            "failed Git tree access is input-unreadable"
        );
    }

    #[trace("TC-983", "FR-054-AC-17", "TC-992", "FR-064-AC-24")]
    #[test]
    fn structurally_valid_changed_outcome_field_fails_lookup() {
        let temp = tempfile::tempdir().expect("temp dir");
        let git_executable = fixture_git();
        let bare = initialized_remote(temp.path(), &git_executable);
        let resolver = LocalResolver(format!("file://{}", bare.display()));
        let signatures = FixtureSignatures::default();
        let private_store = store(&resolver, &signatures, &git_executable);
        let record = approval_record(&signatures, REPOSITORY);
        persist_record_outcome(
            &private_store,
            &signatures,
            &record,
            SinkKind::PrivateStore,
            ApprovalStatementPublishOutcome::Published,
        );
        let outcome_path = format!(
            "{}outcome.json",
            GitPrivateStore::record_path(record.id(), REPOSITORY).expect("record path")
        );
        let original = git_output(
            &git_executable,
            &[
                "--git-dir",
                path_arg(&bare).unwrap(),
                "show",
                &format!("{PRIVATE_RECORDS_REF}:{outcome_path}"),
            ],
        );
        let mut envelope: SignedOutcome =
            serde_json::from_slice(&original).expect("signed outcome parses");
        let mut statement: OutcomeStatement =
            serde_json::from_str(&envelope.statement).expect("outcome statement parses");
        statement.outcome = PublishOutcome::Refused;
        envelope.statement = String::from_utf8(
            serde_json_canonicalizer::to_vec(&statement).expect("canonical changed outcome"),
        )
        .expect("canonical outcome is UTF-8");
        update_remote_file(
            &git_executable,
            &bare,
            &outcome_path,
            &serde_json_canonicalizer::to_vec(&envelope).expect("canonical envelope"),
            temp.path(),
        );
        assert_eq!(
            private_store
                .prepare_record(
                    "agent-ix/private-records",
                    record.private(),
                    record.public(),
                    SinkKind::PrivateStore,
                )
                .expect("invalid retained outcome is incomplete"),
            StorePreparation::OutcomeIncomplete
        );
        assert!(
            private_store
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

    #[trace("TC-983", "FR-054-AC-17")]
    #[test]
    fn valid_signature_from_foreign_record_publisher_fails_lookup() {
        let temp = tempfile::tempdir().expect("temp dir");
        let git_executable = fixture_git();
        let bare = initialized_remote(temp.path(), &git_executable);
        let resolver = LocalResolver(format!("file://{}", bare.display()));
        let signatures = FixtureSignatures::default();
        let private_store = store(&resolver, &signatures, &git_executable);
        let record = approval_record(&signatures, REPOSITORY);
        persist_record_outcome(
            &private_store,
            &signatures,
            &record,
            SinkKind::PrivateStore,
            ApprovalStatementPublishOutcome::Published,
        );
        let outcome_path = format!(
            "{}outcome.json",
            GitPrivateStore::record_path(record.id(), REPOSITORY).expect("record path")
        );
        let original = git_output(
            &git_executable,
            &[
                "--git-dir",
                path_arg(&bare).unwrap(),
                "show",
                &format!("{PRIVATE_RECORDS_REF}:{outcome_path}"),
            ],
        );
        let mut envelope: SignedOutcome =
            serde_json::from_slice(&original).expect("signed outcome parses");
        let foreign_signer = signatures.delegated_to("acme/other-publisher");
        envelope.signature_bundle = foreign_signer
            .sign_without_public_log(envelope.statement.as_bytes())
            .expect("foreign identity fixture signs the outcome");
        let foreign_document =
            WorkflowIdentityDocument::parse(&identity_policy("acme/other-publisher"))
                .expect("foreign identity policy parses");
        let foreign_policy = foreign_document
            .signer_policy(
                WorkflowRole::RecordPublisher,
                "acme/other-publisher",
                ".github/workflows/publish.yml",
                "refs/heads/main",
                TransparencyRequirement::TimestampOnly,
            )
            .expect("foreign publisher is trusted by its own policy fixture");
        assert!(
            foreign_signer
                .verify(
                    envelope.statement.as_bytes(),
                    &envelope.signature_bundle,
                    &foreign_policy,
                )
                .is_ok()
        );
        update_remote_file(
            &git_executable,
            &bare,
            &outcome_path,
            &serde_json_canonicalizer::to_vec(&envelope).expect("canonical foreign envelope"),
            temp.path(),
        );
        assert!(
            private_store
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

    #[cfg(unix)]
    #[trace("TC-973", "FR-054-AC-7")]
    #[test]
    fn checked_out_symlink_cannot_redirect_private_write_outside_the_clone() {
        let temp = tempfile::tempdir().expect("temp dir");
        let git_executable = fixture_git();
        let bare = initialized_remote(temp.path(), &git_executable);
        let outside = temp.path().join("outside-checkout");
        fs::create_dir(&outside).expect("outside fixture directory");
        add_owner_symlink_branch(temp.path(), &bare, &git_executable, &outside);

        let resolver = LocalResolver(format!("file://{}", bare.display()));
        let signatures = FixtureSignatures::default();
        let store = store(&resolver, &signatures, &git_executable);
        let record = approval_record(&signatures, REPOSITORY);
        assert!(
            store
                .write_private("agent-ix/private-records", record.private())
                .is_err(),
            "a remote-controlled symlink in an ancestor component refuses the write"
        );
        assert!(
            !outside
                .join("6c6564676572")
                .join(RECORD_ID)
                .join("private.json")
                .exists(),
            "no file is written through the symlink outside the cloned checkout"
        );
    }

    fn assert_persisted_forms_are_canonical(
        record: &Record,
        public: &PublicRecord,
        bare: &Path,
        git_executable: &Path,
    ) -> (String, String) {
        let private_path = format!(
            "{}private.json",
            GitPrivateStore::record_path(record.id(), REPOSITORY).expect("record path")
        );
        let public_path = format!(
            "{}public.json",
            GitPrivateStore::record_path(record.id(), REPOSITORY).expect("record path")
        );
        let outcome_path = format!(
            "{}outcome.json",
            GitPrivateStore::record_path(record.id(), REPOSITORY).expect("record path")
        );
        assert_eq!(
            git_output(
                git_executable,
                &[
                    "--git-dir",
                    path_arg(bare).unwrap(),
                    "show",
                    &format!("{PRIVATE_RECORDS_REF}:{private_path}")
                ],
            ),
            serde_json_canonicalizer::to_vec(record.private()).expect("canonical private form")
        );
        assert_eq!(
            git_output(
                git_executable,
                &[
                    "--git-dir",
                    path_arg(bare).unwrap(),
                    "show",
                    &format!("{PRIVATE_RECORDS_REF}:{public_path}")
                ],
            ),
            public.entry_bytes().expect("canonical public form")
        );
        let outcome: SignedOutcome = serde_json::from_slice(&git_output(
            git_executable,
            &[
                "--git-dir",
                path_arg(bare).unwrap(),
                "show",
                &format!("{PRIVATE_RECORDS_REF}:{outcome_path}"),
            ],
        ))
        .expect("signed outcome envelope");
        assert_eq!(outcome.protocol, ENVELOPE_PROTOCOL);
        (private_path, public_path)
    }

    fn assert_source_repository_is_unchanged(source_bare: &Path, git_executable: &Path) {
        assert_eq!(
            git_output(
                git_executable,
                &[
                    "--git-dir",
                    path_arg(source_bare).unwrap(),
                    "for-each-ref",
                    "--format=%(refname)",
                    "refs/heads/"
                ],
            ),
            b"refs/heads/main\n"
        );
        assert!(
            !Command::new(git_executable)
                .args([
                    "--git-dir",
                    path_arg(source_bare).unwrap(),
                    "show-ref",
                    "--verify",
                    "refs/notes/engineering-assurance-receipts",
                ])
                .output()
                .expect("source notes lookup runs")
                .status
                .success()
        );
    }

    fn assert_private_form_round_trip(
        store: &GitPrivateStore<'_>,
        record: &Record,
        bare: &Path,
        source_bare: &Path,
        git_executable: &Path,
    ) -> (String, String) {
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
                .expect("unfinished prior record fails closed"),
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
        let paths = assert_persisted_forms_are_canonical(record, &public, bare, git_executable);
        assert_source_repository_is_unchanged(source_bare, git_executable);
        paths
    }

    fn assert_signed_outcome_is_immutable(
        store: &GitPrivateStore<'_>,
        record: &Record,
        bare: &Path,
        outcome_path: &str,
        git_executable: &Path,
    ) {
        let outcome_blob = git_stdout(
            git_executable,
            &[
                "--git-dir",
                path_arg(bare).unwrap(),
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
            .expect("same signed outcome is idempotent");
        for (sink, outcome) in [
            (
                SinkKind::PrivateStore,
                ApprovalStatementPublishOutcome::Refused,
            ),
            (
                SinkKind::PublicLog,
                ApprovalStatementPublishOutcome::Published,
            ),
        ] {
            assert!(
                store
                    .write_publish_outcome("agent-ix/private-records", record.id(), sink, outcome)
                    .is_err()
            );
        }
        assert_eq!(
            git_stdout(
                git_executable,
                &[
                    "--git-dir",
                    path_arg(bare).unwrap(),
                    "rev-parse",
                    &format!("{PRIVATE_RECORDS_REF}:{outcome_path}")
                ],
            ),
            outcome_blob
        );
    }

    fn assert_private_writes_are_idempotent_and_outcomes_are_immutable(
        store: &GitPrivateStore<'_>,
        record: &Record,
        bare: &Path,
        public_path: &str,
        temp_root: &Path,
        git_executable: &Path,
    ) {
        let private_path = format!(
            "{}private.json",
            GitPrivateStore::record_path(record.id(), REPOSITORY).expect("record path")
        );
        let outcome_path = format!(
            "{}outcome.json",
            GitPrivateStore::record_path(record.id(), REPOSITORY).expect("record path")
        );
        let private_blob = git_stdout(
            git_executable,
            &[
                "--git-dir",
                path_arg(bare).unwrap(),
                "rev-parse",
                &format!("{PRIVATE_RECORDS_REF}:{private_path}"),
            ],
        );
        store
            .write_private("agent-ix/private-records", record.private())
            .expect("identical private writes are idempotent");
        let mut changed_private = record.private().clone();
        changed_private.ticket_text = Some("different ticket".to_owned());
        assert!(
            store
                .write_private("agent-ix/private-records", &changed_private)
                .is_err()
        );
        assert_eq!(
            git_stdout(
                git_executable,
                &[
                    "--git-dir",
                    path_arg(bare).unwrap(),
                    "rev-parse",
                    &format!("{PRIVATE_RECORDS_REF}:{private_path}")
                ],
            ),
            private_blob
        );
        assert_signed_outcome_is_immutable(store, record, bare, &outcome_path, git_executable);
        assert_eq!(
            store
                .prepare_record(
                    "agent-ix/private-records",
                    record.private(),
                    record.public(),
                    SinkKind::PrivateStore,
                )
                .expect("replay verifies outcome"),
            StorePreparation::AlreadyPublished
        );
        remove_remote_file(git_executable, bare, public_path, temp_root);
        assert_eq!(
            store
                .prepare_record(
                    "agent-ix/private-records",
                    record.private(),
                    record.public(),
                    SinkKind::PrivateStore,
                )
                .expect("inconsistent public state is incomplete"),
            StorePreparation::OutcomeIncomplete
        );
    }

    fn assert_public_log_and_refused_replay(
        store: &GitPrivateStore<'_>,
        signatures: &FixtureSignatures,
    ) {
        let public_log_record = approval_record_with_id(
            signatures,
            REPOSITORY,
            "1b9d6a3e-4f1c-4a52-9d0e-6c1f7a2b8e35",
        );
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
            .expect("public-log outcome is retained without public form");
        let public_path = format!(
            "{}public.json",
            GitPrivateStore::record_path(public_log_record.id(), REPOSITORY)
                .expect("public-log path")
        );
        assert!(
            store
                .with_repository("agent-ix/private-records", |repository| {
                    store.read_file(repository, &public_path)
                })
                .expect("public-log private store can be read")
                .is_none()
        );
        let refused_record = approval_record_with_id(
            signatures,
            REPOSITORY,
            "1b9d6a3e-4f1c-4a52-9d0e-6c1f7a2b8e36",
        );
        persist_record_outcome(
            store,
            signatures,
            &refused_record,
            SinkKind::PublicLog,
            ApprovalStatementPublishOutcome::Refused,
        );
        assert_eq!(
            store
                .prepare_record(
                    "agent-ix/private-records",
                    refused_record.private(),
                    refused_record.public(),
                    SinkKind::PublicLog,
                )
                .expect("refused outcome replays"),
            StorePreparation::AlreadyRefused
        );
    }

    fn assert_unprotected_policy_is_refused(
        resolver: &LocalResolver,
        signatures: &FixtureSignatures,
        git_executable: &Path,
    ) {
        let unprotected = FixturePolicySource {
            protected: false,
            publisher_repository: REPOSITORY.to_owned(),
        };
        assert!(
            GitPrivateStore::new(
                resolver,
                signatures,
                signatures,
                &unprotected,
                &RecordPublisherIdentity {
                    repository: REPOSITORY.to_owned(),
                    workflow_path: ".github/workflows/publish.yml".to_owned(),
                    reference: "refs/heads/main".to_owned(),
                },
                REPOSITORY,
                git_executable,
            )
            .is_err()
        );
    }

    #[trace(
        "TC-973",
        "FR-054-AC-7",
        "FR-054-AC-8",
        "TC-974",
        "FR-054-AC-8",
        "TC-975",
        "FR-054-AC-9",
        "TC-979",
        "FR-054-AC-13",
        "TC-980",
        "FR-054-AC-14",
        "TC-981",
        "FR-054-AC-15",
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
        let resolver = LocalResolver(format!("file://{}", bare.display()));
        let signatures = FixtureSignatures::default();
        assert_unprotected_policy_is_refused(&resolver, &signatures, &git_executable);
        let store = store(&resolver, &signatures, &git_executable);
        let record = approval_record(&signatures, REPOSITORY);
        let (private_path, public_path) =
            assert_private_form_round_trip(&store, &record, &bare, &source_bare, &git_executable);
        assert_private_writes_are_idempotent_and_outcomes_are_immutable(
            &store,
            &record,
            &bare,
            &public_path,
            temp.path(),
            &git_executable,
        );
        assert_public_log_and_refused_replay(&store, &signatures);
        assert!(private_path.ends_with("/private.json"));
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

    fn exact_lookup() -> ApprovalStatementLookup<'static> {
        ApprovalStatementLookup {
            repository: REPOSITORY,
            merged_pull_request: PULL_REQUEST,
            merged_commit: MERGED_COMMIT,
        }
    }

    fn assert_lookup_mismatches_are_omitted(store: &GitPrivateStore<'_>) {
        for lookup in [
            ApprovalStatementLookup {
                repository: REPOSITORY,
                merged_pull_request: "acme/ledger#43",
                merged_commit: MERGED_COMMIT,
            },
            ApprovalStatementLookup {
                repository: "different/repository",
                merged_pull_request: PULL_REQUEST,
                merged_commit: MERGED_COMMIT,
            },
            ApprovalStatementLookup {
                repository: REPOSITORY,
                merged_pull_request: PULL_REQUEST,
                merged_commit: "ffffffffffffffffffffffffffffffffffffffff",
            },
        ] {
            assert!(
                store
                    .lookup_approval_statements("agent-ix/private-records", lookup)
                    .expect("nonmatching query is readable")
                    .is_empty()
            );
        }
    }

    fn assert_tampered_outcome_bundle_is_rejected(
        store: &GitPrivateStore<'_>,
        git_executable: &Path,
        bare: &Path,
        root: &Path,
        record: &Record,
    ) {
        let outcome_path = format!(
            "{}outcome.json",
            GitPrivateStore::record_path(record.id(), REPOSITORY).expect("record path")
        );
        let mut altered: SignedOutcome = serde_json::from_slice(&git_output(
            git_executable,
            &[
                "--git-dir",
                path_arg(bare).unwrap(),
                "show",
                &format!("{PRIVATE_RECORDS_REF}:{outcome_path}"),
            ],
        ))
        .expect("stored outcome parses");
        altered.signature_bundle = "changed-bundle".to_owned();
        update_remote_file(
            git_executable,
            bare,
            &outcome_path,
            &serde_json_canonicalizer::to_vec(&altered).expect("canonical altered outcome"),
            root,
        );
        assert!(
            store
                .lookup_approval_statements("agent-ix/private-records", exact_lookup())
                .is_err()
        );
        update_remote_file(git_executable, bare, &outcome_path, b"{ malformed", root);
        assert!(
            store
                .lookup_approval_statements("agent-ix/private-records", exact_lookup())
                .is_err()
        );
    }

    #[trace(
        "TC-978",
        "FR-054-AC-12",
        "TC-988",
        "FR-058-AC-10",
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
        let record = approval_record(&signatures, REPOSITORY);
        let (expected_statement_bytes, expected_bundle) =
            signed_approval_fixture(&record, &signatures);
        persist_record_outcome(
            &store,
            &signatures,
            &record,
            SinkKind::PrivateStore,
            ApprovalStatementPublishOutcome::Published,
        );
        let earlier = approval_record_with_id(
            &signatures,
            REPOSITORY,
            "0a9d6a3e-4f1c-4a52-9d0e-6c1f7a2b8e34",
        );
        let (_, earlier_bundle) = signed_approval_fixture(&earlier, &signatures);
        persist_record_outcome(
            &store,
            &signatures,
            &earlier,
            SinkKind::PrivateStore,
            ApprovalStatementPublishOutcome::Published,
        );

        let retained = store
            .lookup_approval_statements("agent-ix/private-records", exact_lookup())
            .expect("exact approval statements are readable");
        assert_eq!(retained.len(), 2);
        assert_eq!(retained[0].statement_bytes, expected_statement_bytes);
        assert_eq!(retained[1].statement_bytes, expected_statement_bytes);
        assert_eq!(retained[0].signature_bundle, earlier_bundle);
        assert_eq!(retained[1].signature_bundle, expected_bundle);
        assert!(retained.iter().all(|item| {
            item.publish_outcome == ApprovalStatementPublishOutcome::Published
                && serde_json_canonicalizer::to_vec(
                    &serde_json::from_slice::<Value>(&item.statement_bytes)
                        .expect("retained statement parses"),
                )
                .is_ok_and(|canonical| canonical == item.statement_bytes)
        }));
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
        changed_statement[0] = b' ';
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
        assert_lookup_mismatches_are_omitted(&store);
        assert_tampered_outcome_bundle_is_rejected(
            &store,
            &git_executable,
            &bare,
            temp.path(),
            &record,
        );
    }
}
