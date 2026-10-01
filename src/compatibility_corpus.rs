// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Read-only access to the accepted compatibility-fixture corpus.
//!
//! The corpus is retained by `agent-ix/qa-corpus`, pinned here as a submodule
//! and read in place. This module parses and classifies the corpus index and
//! validates its retained paths. It reads no file: a caller supplies the
//! bytes, and the confined host adapter that obtains them belongs to the binary.
//!
//! The corpus answers one question — *does the reconciled contract read real
//! history and real producer output?* — and refuses to answer a second one. It
//! proves nothing about the live state of the source repositories; it proves
//! that these exact bytes behave this exact way.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::content_digest::ContentDigest;

/// The exact accepted corpus-index version.
pub const CORPUS_VERSION: &str = "engineering-assurance.compatibility-corpus/v1";

/// Every state the campaign must be able to carry through the contract.
///
/// A corpus missing one of these is not an accepted corpus. The gate names the
/// missing state rather than counting, because "7 of 8 kinds" tells nobody
/// which.
pub const REQUIRED_KINDS: [&str; 8] = [
    "current",
    "failed",
    "legacy",
    "malformed",
    "not_computed",
    "stale",
    "tampered",
    "unavailable",
];

/// The longest retained path this reader will accept.
pub const MAX_RETAINED_PATH_BYTES: usize = 4_096;

/// The largest corpus index this reader will parse.
pub const MAX_INDEX_BYTES: usize = 4_194_304;

/// A shared-model concept a producer's output feeds.
///
/// The vocabulary is closed on purpose. A producer case that fed a concept
/// nobody named would look like coverage while proving nothing about the
/// contract, so an unrecognised value is refused when the index is parsed
/// rather than carried through to a test that never checks it.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedConcept {
    /// What a verification is required to do.
    VerificationDefinition,
    /// One run of a defined verification.
    VerificationExecution,
    /// The outcome one check reached.
    CheckResult,
    /// Retained bytes a conclusion rests on.
    Evidence,
    /// A measured quantity.
    Measurement,
    /// An explanation of a failure.
    Diagnostic,
    /// A rendered summary for a reader.
    Report,
    /// A judgement a person made.
    HumanDecision,
}

impl SharedConcept {
    /// The concept's stable wire name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VerificationDefinition => "verification_definition",
            Self::VerificationExecution => "verification_execution",
            Self::CheckResult => "check_result",
            Self::Evidence => "evidence",
            Self::Measurement => "measurement",
            Self::Diagnostic => "diagnostic",
            Self::Report => "report",
            Self::HumanDecision => "human_decision",
        }
    }
}

/// Where a real retained record came from, and the digest that source recorded.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaseOrigin {
    /// Source repository that holds the immutable record.
    pub repository: String,
    /// Exact source revision, when one is recorded.
    pub revision: Option<String>,
    /// Path to the record within the source repository.
    pub path: String,
    /// Digest the source repository recorded for those bytes.
    pub recorded_sha256: Option<String>,
}

/// The exact edit that produced a constructed case, and why it was needed.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaseDerivation {
    /// The case this one was built from, when it was built from another.
    #[serde(rename = "from")]
    pub from_case: Option<String>,
    /// The edit that was applied.
    pub operation: String,
    /// Why the edit was necessary rather than found.
    pub reason: String,
}

/// One field mapping a case is required to preserve.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredMapping {
    /// Source JSON path the mapping must carry.
    pub source_path: String,
    /// Exact value the mapping must carry for that path.
    pub value: String,
}

/// What the accepted corpus records as this case's outcome.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaseExpectation {
    /// Expected mapping outcome.
    pub outcome: String,
    /// Reviewer note explaining the expectation.
    pub note: String,
    /// Mappings the view must preserve.
    #[serde(default)]
    pub required_mappings: Vec<RequiredMapping>,
    /// A reason the view must state for an unmapped field.
    #[serde(default)]
    pub required_unmapped_reason: Option<String>,
    /// Case whose bytes' digest is the expected identity for this one.
    #[serde(default)]
    pub verify_against_digest_of: Option<String>,
    /// Whether the view must carry no mapping at all.
    #[serde(default)]
    pub require_no_mappings: bool,
    /// Declared source schema, where the case names one.
    #[serde(default)]
    pub schema: Option<String>,
}

/// One retained compatibility case.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusCase {
    /// Case identity, unique within the corpus.
    pub id: String,
    /// The state this case carries.
    pub kind: String,
    /// Record family the case belongs to.
    pub family: String,
    /// Corpus-relative path to the retained bytes.
    pub retained_path: String,
    /// Source provenance, when the case is found rather than constructed.
    pub origin: Option<CaseOrigin>,
    /// The edit that produced the case, when it was constructed.
    pub derivation: Option<CaseDerivation>,
    /// The recorded expectation for this case.
    pub expected: CaseExpectation,
}

impl CorpusCase {
    /// Whether this case was built rather than found.
    #[must_use]
    pub const fn constructed(&self) -> bool {
        self.derivation.is_some()
    }
}

/// One real producer's retained output.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerCase {
    /// Case identity, unique within the corpus.
    pub id: String,
    /// Implementation language of the producer.
    pub language: String,
    /// Producing repository.
    pub producer: String,
    /// Exact producer revision.
    pub revision: String,
    /// Path to the output within the producing repository.
    pub path: String,
    /// Shared-model concept the output feeds.
    pub feeds: SharedConcept,
    /// Reviewer note.
    pub note: String,
    /// Digest of the bytes as the producing repository holds them.
    pub source_sha256: String,
    /// Corpus-relative path to the retained bytes.
    pub retained_path: String,
}

/// The subject the retained chain was produced about.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChainSubject {
    /// Subject repository.
    pub repository: String,
    /// Exact subject revision.
    pub revision: String,
}

/// One tool pinned by the retained chain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChainTool {
    /// Released version.
    pub version: String,
    /// Release the version was installed from, where one is named.
    #[serde(default)]
    pub release: Option<String>,
    /// Source revision that produced the release, where one is recorded.
    #[serde(default)]
    pub source_revision: Option<String>,
    /// Revision of the CLI surface, where the tool separates the two.
    #[serde(default)]
    pub cli_source_revision: Option<String>,
    /// Engine version, where the tool separates the two.
    #[serde(default)]
    pub engine_version: Option<String>,
    /// Engine source revision, where the tool separates the two.
    #[serde(default)]
    pub engine_source_revision: Option<String>,
    /// Reviewer note about the tool's provenance.
    #[serde(default)]
    pub note: Option<String>,
}

/// One retained chain artifact.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChainArtifact {
    /// Role this artifact plays in the chain.
    pub role: String,
    /// What produced it.
    pub produced_by: String,
    /// Corpus-relative path to the retained bytes.
    pub retained_path: String,
}

/// The retained evidence chain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Chain {
    /// What the chain demonstrates.
    pub purpose: String,
    /// The subject it was produced about.
    pub subject: ChainSubject,
    /// The exact tools that produced it.
    pub tools: BTreeMap<String, ChainTool>,
    /// Retained chain artifacts.
    pub artifacts: Vec<ChainArtifact>,
}

/// The parsed and validated accepted-corpus index.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusIndex {
    /// Exact corpus-version discriminator.
    pub corpus_version: String,
    /// What the corpus is for.
    pub purpose: String,
    /// The limitations the corpus states about itself.
    pub limitations: Vec<String>,
    /// Retained compatibility cases.
    pub cases: Vec<CorpusCase>,
    /// Real producer outputs.
    pub producer_cases: Vec<ProducerCase>,
    /// The retained evidence chain.
    pub chain: Chain,
}

/// Why one retained path is not a safe corpus-relative descendant.
///
/// A closed enum rather than a `&'static str`, because the nine refusals were
/// distinguishable only by prose: every one produced the same diagnostic code
/// and the same variant, so a test could assert nothing finer than "refused"
/// and swapping two of the messages broke nothing. A caller's response to all
/// nine is the same — fix the path — which is why they stay one error variant
/// and one code rather than becoming nine; what they must not share is the
/// discriminant, because that is what makes each refusal individually
/// verifiable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsafePathReason {
    /// The path names nothing at all.
    Empty,
    /// The path is longer than [`MAX_RETAINED_PATH_BYTES`].
    TooLong,
    /// The path contains a NUL byte.
    ContainsNul,
    /// The path contains a backslash, which separates components on some hosts.
    ContainsBackslash,
    /// The path is rooted and so escapes the corpus root entirely.
    Absolute,
    /// The path carries a drive prefix, which is absolute on some hosts.
    DrivePrefix,
    /// The path contains an empty component, as in a doubled separator.
    EmptyComponent,
    /// The path contains a current-directory component.
    CurrentDirectoryComponent,
    /// The path traverses above the corpus root.
    ParentTraversal,
}

impl std::fmt::Display for UnsafePathReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Empty => "retained path is empty",
            Self::TooLong => "retained path is longer than the accepted bound",
            Self::ContainsNul => "retained path contains a NUL byte",
            Self::ContainsBackslash => "retained path contains a backslash",
            Self::Absolute => "retained path is absolute",
            Self::DrivePrefix => "retained path carries a drive prefix",
            Self::EmptyComponent => "retained path contains an empty component",
            Self::CurrentDirectoryComponent => {
                "retained path contains a current-directory component"
            }
            Self::ParentTraversal => "retained path traverses above the corpus root",
        })
    }
}

/// Stable failures at the accepted-corpus boundary.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CorpusError {
    /// The corpus index exceeds [`MAX_INDEX_BYTES`].
    #[error("compatibility corpus index exceeds {MAX_INDEX_BYTES} bytes")]
    IndexTooLarge,
    /// The corpus index is not valid strict JSON for [`CorpusIndex`].
    #[error("invalid compatibility corpus index: {detail}")]
    InvalidIndex {
        /// Parser or structural-validation detail.
        detail: String,
    },
    /// The index names a corpus version this implementation does not accept.
    #[error("unknown compatibility corpus version {observed:?}")]
    UnknownVersion {
        /// Version discriminator received from the index.
        observed: String,
    },
    /// A required index member is absent or empty.
    #[error("compatibility corpus is missing {member}")]
    MissingMember {
        /// Name of the absent or empty member.
        member: &'static str,
    },
    /// The corpus does not carry one of the required states.
    #[error("the accepted corpus is missing state {kind:?}")]
    MissingRequiredKind {
        /// The state the corpus does not carry.
        kind: &'static str,
    },
    /// Two entries claim the same identity.
    #[error("duplicate compatibility corpus {member} identity {identity:?}")]
    DuplicateIdentity {
        /// Which member carries the duplicate.
        member: &'static str,
        /// The duplicated identity.
        identity: String,
    },
    /// A retained path is absolute, escaping, or otherwise unsafe.
    #[error("unsafe retained path for {identity:?}: {reason}")]
    UnsafeRetainedPath {
        /// Identity of the entry that carries the path.
        identity: String,
        /// Closed reason the path is refused.
        reason: UnsafePathReason,
    },
    /// A recorded digest is not a lowercase hexadecimal SHA-256 value.
    #[error("invalid recorded digest for {identity:?}")]
    InvalidRecordedDigest {
        /// Identity of the entry that carries the digest.
        identity: String,
    },
    /// An entry leaves a member empty that has to name something.
    ///
    /// A producer case with no producer or no source path is not evidence that
    /// a real implementation was read; it is a row that satisfies a count.
    #[error("compatibility corpus entry {identity:?} leaves {member} empty")]
    EmptyMember {
        /// Identity of the entry that leaves the member empty.
        identity: String,
        /// Name of the empty member.
        member: &'static str,
    },
    /// No entry carries the requested identity.
    #[error("no such compatibility corpus {member}: {identity:?}")]
    UnknownIdentity {
        /// Which member was searched.
        member: &'static str,
        /// The identity that was not found.
        identity: String,
    },
}

impl CorpusError {
    /// Stable machine-readable diagnostic code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::IndexTooLarge => "compatibility_corpus_index_too_large",
            Self::InvalidIndex { .. } => "invalid_compatibility_corpus_index",
            Self::UnknownVersion { .. } => "unknown_compatibility_corpus_version",
            Self::MissingMember { .. } => "compatibility_corpus_missing_member",
            Self::MissingRequiredKind { .. } => "compatibility_corpus_missing_required_kind",
            Self::DuplicateIdentity { .. } => "duplicate_compatibility_corpus_identity",
            Self::UnsafeRetainedPath { .. } => "unsafe_compatibility_corpus_path",
            Self::InvalidRecordedDigest { .. } => "invalid_compatibility_corpus_digest",
            Self::EmptyMember { .. } => "empty_compatibility_corpus_member",
            Self::UnknownIdentity { .. } => "unknown_compatibility_corpus_identity",
        }
    }
}

/// Refuse any retained path that is not a bounded, corpus-relative descendant.
///
/// The corpus is read through one explicit root. A path that is absolute, that
/// traverses upward, or that carries an empty or current-directory component is
/// refused here rather than resolved, so a confined host adapter never has to
/// decide whether a resolved path is still inside its root.
fn validate_retained_path(identity: &str, path: &str) -> Result<(), CorpusError> {
    let refuse = |reason: UnsafePathReason| {
        Err(CorpusError::UnsafeRetainedPath {
            identity: identity.to_owned(),
            reason,
        })
    };
    if path.is_empty() {
        return refuse(UnsafePathReason::Empty);
    }
    if path.len() > MAX_RETAINED_PATH_BYTES {
        return refuse(UnsafePathReason::TooLong);
    }
    if path.contains('\0') {
        return refuse(UnsafePathReason::ContainsNul);
    }
    if path.contains('\\') {
        return refuse(UnsafePathReason::ContainsBackslash);
    }
    if path.starts_with('/') {
        return refuse(UnsafePathReason::Absolute);
    }
    // A Windows drive or UNC prefix is absolute on some hosts and relative on
    // others. It is refused on every host, so the corpus reads the same bytes
    // everywhere.
    if path.len() >= 2 && path.as_bytes()[1] == b':' {
        return refuse(UnsafePathReason::DrivePrefix);
    }
    for component in path.split('/') {
        match component {
            "" => return refuse(UnsafePathReason::EmptyComponent),
            "." => return refuse(UnsafePathReason::CurrentDirectoryComponent),
            ".." => return refuse(UnsafePathReason::ParentTraversal),
            _ => {}
        }
    }
    Ok(())
}

impl CorpusIndex {
    /// Parse and validate the accepted-corpus index.
    ///
    /// # Errors
    ///
    /// Returns a stable [`CorpusError`] for an oversized or malformed index, an
    /// unknown corpus version, an absent required member, a duplicate identity,
    /// an unsafe retained path, or an invalid recorded digest.
    pub fn parse(bytes: &[u8]) -> Result<Self, CorpusError> {
        if bytes.len() > MAX_INDEX_BYTES {
            return Err(CorpusError::IndexTooLarge);
        }
        let index: Self =
            serde_json::from_slice(bytes).map_err(|error| CorpusError::InvalidIndex {
                detail: error.to_string(),
            })?;
        index.validate()?;
        Ok(index)
    }

    fn validate(&self) -> Result<(), CorpusError> {
        if self.corpus_version != CORPUS_VERSION {
            return Err(CorpusError::UnknownVersion {
                observed: self.corpus_version.clone(),
            });
        }
        for (member, empty) in [
            ("cases", self.cases.is_empty()),
            ("producer_cases", self.producer_cases.is_empty()),
            ("limitations", self.limitations.is_empty()),
            ("chain artifacts", self.chain.artifacts.is_empty()),
        ] {
            if empty {
                return Err(CorpusError::MissingMember { member });
            }
        }

        let mut case_ids = BTreeSet::new();
        for case in &self.cases {
            if !case_ids.insert(case.id.as_str()) {
                return Err(CorpusError::DuplicateIdentity {
                    member: "case",
                    identity: case.id.clone(),
                });
            }
            validate_retained_path(&case.id, &case.retained_path)?;
        }

        let mut producer_ids = BTreeSet::new();
        for producer in &self.producer_cases {
            if !producer_ids.insert(producer.id.as_str()) {
                return Err(CorpusError::DuplicateIdentity {
                    member: "producer case",
                    identity: producer.id.clone(),
                });
            }
            // A producer case exists to prove a real implementation's output was
            // read. One that names no producer, or no path within it, records
            // nothing anybody could go back to, so it is refused here rather
            // than counted as coverage.
            for (member, value) in [
                ("producer", producer.producer.as_str()),
                ("path", producer.path.as_str()),
            ] {
                if value.trim().is_empty() {
                    return Err(CorpusError::EmptyMember {
                        identity: producer.id.clone(),
                        member,
                    });
                }
            }
            validate_retained_path(&producer.id, &producer.retained_path)?;
            if ContentDigest::parse(&producer.source_sha256).is_err() {
                return Err(CorpusError::InvalidRecordedDigest {
                    identity: producer.id.clone(),
                });
            }
        }

        let mut roles = BTreeSet::new();
        for artifact in &self.chain.artifacts {
            if !roles.insert(artifact.role.as_str()) {
                return Err(CorpusError::DuplicateIdentity {
                    member: "chain artifact",
                    identity: artifact.role.clone(),
                });
            }
            validate_retained_path(&artifact.role, &artifact.retained_path)?;
        }
        Ok(())
    }

    /// The first required state the corpus does not carry, if any.
    #[must_use]
    pub fn missing_required_kind(&self) -> Option<&'static str> {
        let present: BTreeSet<&str> = self.cases.iter().map(|case| case.kind.as_str()).collect();
        REQUIRED_KINDS
            .into_iter()
            .find(|kind| !present.contains(kind))
    }

    /// Refuse a corpus that does not carry every required state.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError::MissingRequiredKind`] naming the first absent
    /// state, so a reader is told which one rather than how many.
    pub fn require_all_kinds(&self) -> Result<(), CorpusError> {
        self.missing_required_kind().map_or(Ok(()), |kind| {
            Err(CorpusError::MissingRequiredKind { kind })
        })
    }

    /// One case by identity.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError::UnknownIdentity`] when no case carries `id`.
    pub fn case(&self, id: &str) -> Result<&CorpusCase, CorpusError> {
        self.cases
            .iter()
            .find(|case| case.id == id)
            .ok_or_else(|| CorpusError::UnknownIdentity {
                member: "case",
                identity: id.to_owned(),
            })
    }

    /// Every case carrying one state, in corpus order.
    #[must_use]
    pub fn cases_by_kind(&self, kind: &str) -> Vec<&CorpusCase> {
        self.cases.iter().filter(|case| case.kind == kind).collect()
    }

    /// One retained chain artifact by role.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError::UnknownIdentity`] when no artifact carries `role`.
    pub fn chain_artifact(&self, role: &str) -> Result<&ChainArtifact, CorpusError> {
        self.chain
            .artifacts
            .iter()
            .find(|artifact| artifact.role == role)
            .ok_or_else(|| CorpusError::UnknownIdentity {
                member: "chain artifact",
                identity: role.to_owned(),
            })
    }
}

/// One artifact this reader may be asked for bytes of.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetainedArtifact<'a> {
    /// Identity used in refusals.
    pub identity: &'a str,
    /// Corpus-relative path to the retained bytes.
    pub retained_path: &'a str,
}

impl<'a> From<&'a CorpusCase> for RetainedArtifact<'a> {
    fn from(case: &'a CorpusCase) -> Self {
        Self {
            identity: &case.id,
            retained_path: &case.retained_path,
        }
    }
}

impl<'a> From<&'a ChainArtifact> for RetainedArtifact<'a> {
    fn from(artifact: &'a ChainArtifact) -> Self {
        Self {
            identity: &artifact.role,
            retained_path: &artifact.retained_path,
        }
    }
}

impl<'a> From<&'a ProducerCase> for RetainedArtifact<'a> {
    fn from(producer: &'a ProducerCase) -> Self {
        Self {
            identity: &producer.id,
            retained_path: &producer.retained_path,
        }
    }
}

impl RetainedArtifact<'_> {
    /// The corpus-relative path whose bytes this artifact retains.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError::UnsafeRetainedPath`] when the path is not a
    /// bounded, corpus-relative descendant.
    pub fn require_retained_path(&self) -> Result<&str, CorpusError> {
        validate_retained_path(self.identity, self.retained_path)?;
        Ok(self.retained_path)
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use serde_json::{Value, json};

    use super::*;

    /// A minimal index that satisfies every structural rule.
    ///
    /// Synthetic rather than a copy of the accepted corpus: this module is the
    /// reusable library, and embedding qualification corpus bytes here would put
    /// them in the production artifact. The real corpus is read by the confined
    /// host adapter, which is where the accepted-corpus assertions live.
    fn valid_index() -> Value {
        let digest = ContentDigest::of_bytes(b"retained bytes").into_hex();
        let cases: Vec<Value> = REQUIRED_KINDS
            .iter()
            .enumerate()
            .map(|(position, kind)| {
                json!({
                    "id": format!("case-{kind}"),
                    "kind": kind,
                    "family": "pgm01-v1",
                    "retained_path": format!("records/{kind}.json"),
                    "origin": null,
                    "derivation": if position == 0 {
                        Value::Null
                    } else {
                        json!({
                            "from": "case-current",
                            "operation": "set /checks/0/status to the named state",
                            "reason": "no real record carries this state",
                        })
                    },
                    "expected": {"outcome": "lossy", "note": "a fictional expectation"},
                })
            })
            .collect();
        json!({
            "corpus_version": CORPUS_VERSION,
            "purpose": "a fictional index for library-boundary tests",
            "limitations": [
                "Every non-current state is derived, and each records the edit that produced it.",
                "PGM-01 v1 has no not-computed status.",
            ],
            "cases": cases,
            "producer_cases": [
                {
                    "id": "producer-rust",
                    "language": "rust",
                    "producer": "agent-ix/fictional-producer",
                    "revision": "0".repeat(40),
                    "path": "tests/golden/fixture.json",
                    "feeds": "check_result",
                    "note": "a fictional producer case",
                    "source_sha256": digest,
                    "retained_path": "producers/retained.json",
                },
                {
                    "id": "producer-typescript",
                    "language": "typescript",
                    "producer": "agent-ix/fictional-consumer",
                    "revision": "1".repeat(40),
                    "path": "dist/result.json",
                    "feeds": "measurement",
                    "note": "a second fictional producer case",
                    "source_sha256": digest,
                    "retained_path": "producers/typescript.json",
                },
            ],
            "chain": {
                "purpose": "a fictional chain",
                "subject": {"repository": "agent-ix/fictional", "revision": "2".repeat(40)},
                "tools": {"quire": {"version": "0.0.0"}},
                "artifacts": [
                    {
                        "role": "change_assurance_record",
                        "produced_by": "quoin",
                        "retained_path": "chain/record.json",
                    },
                    {
                        "role": "receipt_schema",
                        "produced_by": "quoin",
                        "retained_path": "chain/receipt.schema.json",
                    },
                ],
            },
        })
    }

    fn parse(raw: &Value) -> Result<CorpusIndex, CorpusError> {
        CorpusIndex::parse(&serde_json::to_vec(raw).expect("index must serialize"))
    }

    fn index() -> CorpusIndex {
        parse(&valid_index()).expect("the valid index must parse")
    }

    fn mutated(mutate: impl FnOnce(&mut Value)) -> CorpusError {
        let mut raw = valid_index();
        mutate(&mut raw);
        parse(&raw).expect_err("the mutated index must be refused")
    }

    #[trace("TC-070", "FR-011-AC-2", "FR-015-AC-5")]
    #[test]
    fn tc_070_names_the_missing_state_and_labels_constructions() {
        let index = index();
        assert_eq!(index.missing_required_kind(), None);
        index
            .require_all_kinds()
            .expect("an index carrying every state must be accepted");

        for case in &index.cases {
            let Some(derivation) = case.derivation.as_ref() else {
                continue;
            };
            assert!(
                case.constructed(),
                "{} is not labelled constructed",
                case.id
            );
            assert!(!derivation.operation.trim().is_empty());
            assert!(!derivation.reason.trim().is_empty());
        }

        // The refusal names the absent state rather than a count, because
        // "7 of 8 kinds" tells a reader nothing about which one is missing.
        let mut raw = valid_index();
        raw["cases"]
            .as_array_mut()
            .expect("cases must be an array")
            .retain(|case| case["kind"] != "tampered");
        let index = parse(&raw).expect("a structurally valid index must still parse");
        assert_eq!(index.missing_required_kind(), Some("tampered"));
        let error = index
            .require_all_kinds()
            .expect_err("an incomplete corpus must refuse");
        assert_eq!(error.code(), "compatibility_corpus_missing_required_kind");
        assert!(error.to_string().contains("tampered"));
    }

    #[trace("TC-069", "FR-011-AC-1", "FR-015-AC-5")]
    #[test]
    fn tc_069_resolves_a_retained_case_to_its_validated_path() {
        let index = index();
        let case = index.case("case-legacy").expect("case must exist");
        let artifact = RetainedArtifact::from(case);

        assert_eq!(artifact.require_retained_path(), Ok("records/legacy.json"));

        let unsafe_artifact = RetainedArtifact {
            identity: "unsafe",
            retained_path: "../outside.json",
        };
        assert_eq!(
            unsafe_artifact
                .require_retained_path()
                .expect_err("an escaping path must refuse")
                .code(),
            "unsafe_compatibility_corpus_path"
        );
    }

    #[trace("TC-075", "FR-011-AC-7", "FR-015-AC-5")]
    #[test]
    fn tc_075_exposes_producer_language_and_concept() {
        let index = index();
        let languages: BTreeSet<&str> = index
            .producer_cases
            .iter()
            .map(|producer| producer.language.as_str())
            .collect();
        let concepts: BTreeSet<SharedConcept> = index
            .producer_cases
            .iter()
            .map(|producer| producer.feeds)
            .collect();
        assert!(languages.contains("rust") && languages.contains("typescript"));
        assert!(
            concepts.contains(&SharedConcept::CheckResult)
                && concepts.contains(&SharedConcept::Measurement)
        );

        let producer = &index.producer_cases[1];
        assert_eq!(producer.retained_path, "producers/typescript.json");
        assert_eq!(producer.source_sha256.len(), 64);

        assert_eq!(
            index
                .chain_artifact("receipt_schema")
                .expect("the receipt schema must resolve")
                .retained_path,
            "chain/receipt.schema.json"
        );
    }

    #[trace("TC-103", "FR-015-AC-3", "FR-015-AC-5", "FR-015-CON-4")]
    #[test]
    fn tc_103_refuses_every_malformed_unsafe_or_incomplete_index() {
        assert_eq!(
            CorpusIndex::parse(b"{")
                .expect_err("malformed JSON must refuse")
                .code(),
            "invalid_compatibility_corpus_index"
        );
        assert_eq!(
            CorpusIndex::parse(&vec![b' '; MAX_INDEX_BYTES + 1])
                .expect_err("an oversized index must refuse")
                .code(),
            "compatibility_corpus_index_too_large"
        );
        assert_eq!(
            mutated(|raw| raw["corpus_version"] = json!("engineering-assurance.corpus/v2")).code(),
            "unknown_compatibility_corpus_version"
        );
        assert_eq!(
            mutated(|raw| {
                raw.as_object_mut()
                    .expect("the index must be an object")
                    .remove("chain");
            })
            .code(),
            "invalid_compatibility_corpus_index"
        );
        for empty in ["cases", "producer_cases", "limitations"] {
            assert_eq!(
                mutated(|raw| raw[empty] = json!([])).code(),
                "compatibility_corpus_missing_member",
                "an empty {empty} member was not refused"
            );
        }
        assert_eq!(
            mutated(|raw| raw["chain"]["artifacts"] = json!([])).code(),
            "compatibility_corpus_missing_member"
        );

        for member in ["cases", "producer_cases"] {
            assert_eq!(
                mutated(|raw| {
                    let entries = raw[member].as_array_mut().expect("member must be an array");
                    let duplicate = entries[0].clone();
                    entries.push(duplicate);
                })
                .code(),
                "duplicate_compatibility_corpus_identity",
                "a duplicate {member} identity was not refused"
            );
        }
        assert_eq!(
            mutated(|raw| {
                raw["chain"]["artifacts"][1]["role"] = raw["chain"]["artifacts"][0]["role"].clone();
            })
            .code(),
            "duplicate_compatibility_corpus_identity"
        );

        for digest in ["", "not-a-digest", &"A".repeat(64), &"a".repeat(63)] {
            assert_eq!(
                mutated(|raw| raw["producer_cases"][0]["source_sha256"] = json!(digest)).code(),
                "invalid_compatibility_corpus_digest",
                "{digest:?} was accepted as a recorded digest"
            );
        }
    }

    #[trace("TC-103", "FR-015-AC-3", "FR-015-AC-5", "FR-015-CON-4")]
    #[test]
    fn tc_103_refuses_every_unsafe_path_and_empty_producer_member() {
        // Every unsafe retained-path shape is refused here, so a confined host
        // never has to decide whether a resolved path is still inside its root.
        // Each unsafe shape is paired with the reason it must produce. All nine
        // share one diagnostic code because a caller's response to all nine is
        // the same — fix the path — but they must not share the discriminant:
        // while the reason was a `&'static str` inside one variant, nothing
        // distinguished a traversal from a doubled separator, and swapping two
        // of the messages left every assertion here passing.
        let long = format!("records/{}.json", "a".repeat(MAX_RETAINED_PATH_BYTES));
        for (unsafe_path, expected) in [
            ("", UnsafePathReason::Empty),
            ("/etc/passwd", UnsafePathReason::Absolute),
            ("../outside.json", UnsafePathReason::ParentTraversal),
            (
                "records/../../outside.json",
                UnsafePathReason::ParentTraversal,
            ),
            (
                "records/./here.json",
                UnsafePathReason::CurrentDirectoryComponent,
            ),
            ("records//here.json", UnsafePathReason::EmptyComponent),
            ("records\\here.json", UnsafePathReason::ContainsBackslash),
            ("C:/records/here.json", UnsafePathReason::DrivePrefix),
            ("records/\u{0}here.json", UnsafePathReason::ContainsNul),
            (long.as_str(), UnsafePathReason::TooLong),
        ] {
            let error = mutated(|raw| raw["cases"][0]["retained_path"] = json!(unsafe_path));
            assert_eq!(
                error.code(),
                "unsafe_compatibility_corpus_path",
                "{unsafe_path:?} was not refused"
            );
            assert!(
                matches!(
                    &error,
                    CorpusError::UnsafeRetainedPath { reason, .. } if *reason == expected
                ),
                "{unsafe_path:?} was refused as {error:?}, not {expected:?}"
            );
        }

        // A producer case that names no producer or no source path is refused
        // when the index is parsed. Such a row satisfies a count without
        // recording an implementation anyone could go back and read.
        for member in ["producer", "path"] {
            assert_eq!(
                mutated(|raw| raw["producer_cases"][0][member] = json!("   ")).code(),
                "empty_compatibility_corpus_member",
                "an empty {member} was accepted"
            );
        }

        // The shared-model vocabulary is closed. A case feeding a concept
        // nobody named would look like cross-cutting coverage while proving
        // nothing, so an unrecognised value never parses in the first place.
        assert_eq!(
            mutated(|raw| raw["producer_cases"][0]["feeds"] = json!("guesswork")).code(),
            "invalid_compatibility_corpus_index"
        );

        // Unknown identities are their own answer, not an empty result.
        let index = index();
        for error in [
            index.case("not-a-case").expect_err("unknown case"),
            index
                .chain_artifact("not-a-role")
                .expect_err("unknown role"),
        ] {
            assert_eq!(error.code(), "unknown_compatibility_corpus_identity");
        }
        assert!(index.cases_by_kind("not-a-kind").is_empty());
        assert_eq!(index.cases_by_kind("legacy").len(), 1);
    }
}
