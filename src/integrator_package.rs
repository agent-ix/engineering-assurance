// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The versioned integrator-package contract: a writer and a strict reader
//! (FR-030).
//!
//! An integrator package carries one source release candidate's facts to the
//! system that integrates it, and keeps the integrator's own validation open.
//! It has two disjoint parts:
//!
//! - **Source facts** ([`SourceFact`]): assumptions, limitations, exceptions,
//!   negative results, reviews, decisions, profiles, corpora, configurations,
//!   supersessions, licenses, reuse rights, provenance and verification
//!   artifacts, each with an explicit [`FactState`] and the authority that
//!   stated it. Open, negative, incomplete, conditional, conflicting and
//!   refused states are first-class values, never absences.
//! - **Adopter subjects** ([`AdopterSubject`]): the intended-use and
//!   configuration subjects the adopter owns, each with its obligations. An
//!   obligation is [`ObligationState::Open`] or separately attributed
//!   [`ObligationState::Completed`]. The package has no acceptance field of
//!   any kind, so neither the source release nor a completed obligation can
//!   state that the adopter's validation is accepted.
//!
//! The reader is strict: it accepts exactly the canonical bytes of exactly one
//! supported protocol version and refuses everything else. There is no
//! lenient mode and no digest; integrity is the byte-for-byte equality of the
//! input with the canonical re-encoding of what was parsed.
//!
//! This module performs no I/O. Candidate-bound publication is
//! [`crate::integrator_package_store`].

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The only protocol this release writes and reads.
pub const INTEGRATOR_PACKAGE_PROTOCOL: &str = "engineering-assurance.integrator-package/v1";
/// The identity of the protocol family; a version suffix `/v<N>` follows.
const PROTOCOL_FAMILY: &str = "engineering-assurance.integrator-package/v";
/// The lowest and only supported protocol version.
const SUPPORTED_VERSION: u64 = 1;

/// Maximum accepted package size in bytes.
pub const MAX_PACKAGE_BYTES: usize = 8_388_608;
/// Maximum number of source facts.
pub const MAX_FACTS: usize = 4_096;
/// Maximum number of adopter subjects.
pub const MAX_SUBJECTS: usize = 256;
/// Maximum number of obligations per adopter subject.
pub const MAX_OBLIGATIONS: usize = 1_024;
/// Maximum number of source references per fact.
pub const MAX_SOURCES: usize = 64;
/// Maximum identifier length in bytes.
pub const MAX_IDENTIFIER_BYTES: usize = 128;
/// Maximum length of any free-text field in bytes.
pub const MAX_TEXT_BYTES: usize = 16_384;

/// A malformed identifier.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error(
    "identifier must be 1..={MAX_IDENTIFIER_BYTES} bytes of ASCII lowercase letters, digits, '.', '_' or '-' and must not start or end with '.'"
)]
pub struct IdentifierError;

macro_rules! identifier {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Validates and wraps `value`.
            ///
            /// # Errors
            ///
            /// [`IdentifierError`] when `value` is empty, too long, starts or
            /// ends with `.` or holds a byte outside ASCII lowercase letters,
            /// digits, `.`, `_` and `-`. The restriction makes every identifier
            /// a single path component that cannot alias another on a
            /// case-insensitive filesystem. Windows reserved device names
            /// (`con`, `nul`, ...) are not excluded; do not choose them.
            pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
                let value = value.into();
                let valid = !value.is_empty()
                    && value.len() <= MAX_IDENTIFIER_BYTES
                    && !value.starts_with('.')
                    && !value.ends_with('.')
                    && value.bytes().all(|byte| {
                        byte.is_ascii_lowercase()
                            || byte.is_ascii_digit()
                            || matches!(byte, b'.' | b'_' | b'-')
                    });
                if valid { Ok(Self(value)) } else { Err(IdentifierError) }
            }

            /// The identifier text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

identifier! {
    /// The exact source release candidate a package is bound to (for example a
    /// commit identity chosen by the writer).
    CandidateId
}
identifier! {
    /// One publication attempt for a candidate. Reusing an attempt with
    /// different content is a conflict.
    AttemptId
}
identifier! {
    /// A source fact identity, unique within a package.
    FactId
}
identifier! {
    /// An adopter subject identity, unique within a package.
    SubjectId
}
identifier! {
    /// An adopter obligation identity, unique within its subject.
    ObligationId
}

/// What a source fact is about. Every kind must be represented in a package,
/// so a kind that was never examined is stated as [`FactState::Incomplete`]
/// rather than left out.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FactKind {
    /// An assumption the release relies on.
    Assumption,
    /// A limitation of the release.
    Limitation,
    /// An exception to a rule or expectation.
    Exception,
    /// A negative result.
    NegativeResult,
    /// A review.
    Review,
    /// A release decision.
    Decision,
    /// A profile the release was assessed against.
    Profile,
    /// A corpus the release was exercised on.
    Corpus,
    /// A tool or build configuration.
    Configuration,
    /// A supersession of an earlier release or decision.
    Supersession,
    /// A license.
    License,
    /// A reuse right.
    ReuseRight,
    /// Provenance of the candidate or of a material.
    Provenance,
    /// A retained verification artifact.
    VerificationArtifact,
}

impl FactKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 14] = [
        Self::Assumption,
        Self::Limitation,
        Self::Exception,
        Self::NegativeResult,
        Self::Review,
        Self::Decision,
        Self::Profile,
        Self::Corpus,
        Self::Configuration,
        Self::Supersession,
        Self::License,
        Self::ReuseRight,
        Self::Provenance,
        Self::VerificationArtifact,
    ];
}

/// The state of a source fact. The states are unordered and none implies
/// another; the reader never converts one into another.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FactState {
    /// The authority established the fact, with sources.
    Established,
    /// The fact is still to be determined.
    Open,
    /// The fact is a negative finding.
    Negative,
    /// The material needed to state the fact is incomplete.
    Incomplete,
    /// The fact holds only under stated conditions.
    Conditional,
    /// Authorities disagree.
    Conflict,
    /// An authority or input could not be verified and the fact is refused.
    Refused,
}

/// Where a fact's statement comes from.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRef {
    /// The party that issued the source.
    pub issuer: String,
    /// The writer's reference to the source (a name, a path, an identity).
    pub reference: String,
}

/// One source-release fact. Its stage is always the source release.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFact {
    /// Unique identity within the package.
    pub id: FactId,
    /// What the fact is about.
    pub kind: FactKind,
    /// The fact's state.
    pub state: FactState,
    /// The party with authority over the fact.
    pub authority: String,
    /// The fact itself, including the condition, conflict or gap when the
    /// state is not [`FactState::Established`].
    pub statement: String,
    /// A normalized value such as one SPDX expression; required for an
    /// established license or reuse right.
    pub value: Option<String>,
    /// The sources the statement rests on; at least one when established.
    pub sources: Vec<SourceRef>,
}

/// The state of an adopter obligation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObligationState {
    /// Nothing has been supplied; the adopter still owns it.
    Open,
    /// The adopter supplied a response. The package records it as the
    /// adopter's own input; it is neither checked nor accepted here.
    Completed {
        /// The adopter party that supplied the response.
        attributed_to: String,
        /// What the adopter supplied.
        response: String,
    },
}

/// One obligation the adopter owns.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdopterObligation {
    /// Unique identity within the subject.
    pub id: ObligationId,
    /// What the adopter must establish.
    pub statement: String,
    /// Whether the adopter has supplied a response.
    pub status: ObligationState,
}

/// One intended-use and configuration subject the adopter owns.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdopterSubject {
    /// Unique identity within the package.
    pub id: SubjectId,
    /// The adopter's intended use.
    pub intended_use: String,
    /// The adopter's deployment configuration.
    pub configuration: String,
    /// The subject's obligations; at least one.
    pub obligations: Vec<AdopterObligation>,
}

/// One integrator package.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegratorPackage {
    /// Must equal [`INTEGRATOR_PACKAGE_PROTOCOL`].
    pub protocol: String,
    /// The exact release candidate the package is bound to.
    pub candidate: CandidateId,
    /// The publication attempt.
    pub attempt: AttemptId,
    /// The number of source facts, stated so a lost fact is detectable.
    pub fact_count: usize,
    /// The number of adopter subjects, stated for the same reason.
    pub subject_count: usize,
    /// Source facts, ordered by id.
    pub facts: Vec<SourceFact>,
    /// Adopter subjects, ordered by id.
    pub adopter_subjects: Vec<AdopterSubject>,
}

impl IntegratorPackage {
    /// Builds a package for the current protocol with counts and ordering
    /// filled in. The result is not validated; [`write_package`] validates.
    #[must_use]
    pub fn new(
        candidate: CandidateId,
        attempt: AttemptId,
        mut facts: Vec<SourceFact>,
        mut adopter_subjects: Vec<AdopterSubject>,
    ) -> Self {
        facts.sort_by(|a, b| a.id.cmp(&b.id));
        adopter_subjects.sort_by(|a, b| a.id.cmp(&b.id));
        for subject in &mut adopter_subjects {
            subject.obligations.sort_by(|a, b| a.id.cmp(&b.id));
        }
        Self {
            protocol: INTEGRATOR_PACKAGE_PROTOCOL.to_owned(),
            candidate,
            attempt,
            fact_count: facts.len(),
            subject_count: adopter_subjects.len(),
            facts,
            adopter_subjects,
        }
    }

    /// Every obligation still open, with its subject. Completed obligations are
    /// the adopter's own input and are not listed as accepted anywhere.
    pub fn open_obligations(&self) -> impl Iterator<Item = (&SubjectId, &AdopterObligation)> {
        self.adopter_subjects.iter().flat_map(|subject| {
            subject
                .obligations
                .iter()
                .filter(|obligation| obligation.status == ObligationState::Open)
                .map(move |obligation| (&subject.id, obligation))
        })
    }
}

/// A rule a package breaks.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum Violation {
    /// The stated fact or subject count differs from what is present.
    #[error("{what} count states {stated} but {present} are present")]
    CountMismatch {
        /// `fact` or `subject`.
        what: &'static str,
        /// The stated count.
        stated: usize,
        /// The number present.
        present: usize,
    },
    /// A collection exceeds its bound.
    #[error("{what} exceeds its bound of {bound}")]
    TooMany {
        /// The collection.
        what: &'static str,
        /// The bound.
        bound: usize,
    },
    /// An identity appears twice.
    #[error("duplicate {what} identity {id}")]
    Duplicate {
        /// The identity space.
        what: &'static str,
        /// The repeated identity.
        id: String,
    },
    /// A collection is not ordered by identity.
    #[error("{what} are not ordered by id")]
    Unordered {
        /// The collection.
        what: &'static str,
    },
    /// No fact of a required kind is present.
    #[error("no fact of kind {0:?}; state it as incomplete rather than omit it")]
    MissingKind(FactKind),
    /// A text field is empty, padded or too long.
    #[error("{field} must be non-empty, trimmed text of at most {MAX_TEXT_BYTES} bytes")]
    Text {
        /// The field.
        field: &'static str,
    },
    /// An established fact lacks the basis its state claims.
    #[error("established fact {id} lacks {missing}")]
    Unsupported {
        /// The fact.
        id: String,
        /// `sources` or `value`.
        missing: &'static str,
    },
    /// A fact that is not established carries a normalized value.
    #[error("fact {id} carries a value but is not established")]
    UnestablishedValue {
        /// The fact.
        id: String,
    },
    /// An adopter subject has no obligations.
    #[error("adopter subject {id} has no obligations")]
    EmptySubject {
        /// The subject.
        id: String,
    },
}

/// Why a package could not be written.
#[derive(Debug, Error)]
pub enum WriteError {
    /// The package breaks a rule.
    #[error("invalid integrator package: {0}")]
    Invalid(#[from] Violation),
    /// The package has no canonical encoding.
    #[error("integrator package cannot be encoded canonically")]
    Encoding,
    /// The package names a protocol this writer does not write.
    #[error("this writer writes only {INTEGRATOR_PACKAGE_PROTOCOL}")]
    Protocol,
    /// The reader refused the writer's own output; this is a defect.
    #[error("the strict reader refused the writer's output: {0}")]
    Readback(#[source] ReadError),
    /// The encoding exceeds [`MAX_PACKAGE_BYTES`].
    #[error("integrator package exceeds {MAX_PACKAGE_BYTES} bytes")]
    TooLarge,
}

/// Why bytes were refused by the strict reader.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ReadError {
    /// The input exceeds [`MAX_PACKAGE_BYTES`].
    #[error("integrator package exceeds {MAX_PACKAGE_BYTES} bytes")]
    TooLarge,
    /// The input is not a JSON object with a string protocol.
    #[error("input is not an integrator package: {0}")]
    NotAPackage(String),
    /// The protocol is an older version of this family than is supported.
    #[error("protocol {0} is a downgrade below the supported version")]
    Downgraded(String),
    /// The protocol is a newer version of this family than is supported.
    #[error("protocol {0} is newer than this reader supports")]
    UnsupportedVersion(String),
    /// The protocol is not of this family.
    #[error("protocol {0} is not an integrator-package protocol")]
    UnknownProtocol(String),
    /// A field, variant or value the contract does not define, a duplicate or
    /// missing key, or a wrong type.
    #[error("input does not match the contract: {0}")]
    Malformed(String),
    /// The input parses but is not the canonical encoding of what it parses to.
    #[error("input is not the canonical encoding")]
    NotCanonical,
    /// The content breaks a rule.
    #[error("invalid integrator package: {0}")]
    Invalid(#[from] Violation),
}

/// Checks every package rule. Shared by the writer and the reader, so what can
/// be written is exactly what can be read.
///
/// # Errors
///
/// The first [`Violation`] found.
pub fn check_package(package: &IntegratorPackage) -> Result<(), Violation> {
    check_bounds(package)?;
    check_ordering_and_identity(package)?;
    check_kinds(package)?;
    for fact in &package.facts {
        check_fact(fact)?;
    }
    for subject in &package.adopter_subjects {
        check_text("subject intended_use", &subject.intended_use)?;
        check_text("subject configuration", &subject.configuration)?;
        if subject.obligations.is_empty() {
            return Err(Violation::EmptySubject {
                id: subject.id.as_str().to_owned(),
            });
        }
        for obligation in &subject.obligations {
            check_text("obligation statement", &obligation.statement)?;
            if let ObligationState::Completed {
                attributed_to,
                response,
            } = &obligation.status
            {
                check_text("obligation attributed_to", attributed_to)?;
                check_text("obligation response", response)?;
            }
        }
    }
    Ok(())
}

fn check_bounds(package: &IntegratorPackage) -> Result<(), Violation> {
    if package.facts.len() > MAX_FACTS {
        return Err(Violation::TooMany {
            what: "facts",
            bound: MAX_FACTS,
        });
    }
    if package.adopter_subjects.len() > MAX_SUBJECTS {
        return Err(Violation::TooMany {
            what: "adopter subjects",
            bound: MAX_SUBJECTS,
        });
    }
    if package
        .adopter_subjects
        .iter()
        .any(|subject| subject.obligations.len() > MAX_OBLIGATIONS)
    {
        return Err(Violation::TooMany {
            what: "obligations",
            bound: MAX_OBLIGATIONS,
        });
    }
    if package
        .facts
        .iter()
        .any(|fact| fact.sources.len() > MAX_SOURCES)
    {
        return Err(Violation::TooMany {
            what: "sources",
            bound: MAX_SOURCES,
        });
    }
    if package.fact_count != package.facts.len() {
        return Err(Violation::CountMismatch {
            what: "fact",
            stated: package.fact_count,
            present: package.facts.len(),
        });
    }
    if package.subject_count != package.adopter_subjects.len() {
        return Err(Violation::CountMismatch {
            what: "subject",
            stated: package.subject_count,
            present: package.adopter_subjects.len(),
        });
    }
    Ok(())
}

fn check_ordering_and_identity(package: &IntegratorPackage) -> Result<(), Violation> {
    check_identities("fact", package.facts.iter().map(|fact| fact.id.as_str()))?;
    check_identities(
        "subject",
        package
            .adopter_subjects
            .iter()
            .map(|subject| subject.id.as_str()),
    )?;
    for subject in &package.adopter_subjects {
        check_identities(
            "obligation",
            subject
                .obligations
                .iter()
                .map(|obligation| obligation.id.as_str()),
        )?;
    }
    Ok(())
}

/// Identities must be strictly ascending: that is both "ordered" and "unique".
fn check_identities<'a>(
    what: &'static str,
    ids: impl Iterator<Item = &'a str>,
) -> Result<(), Violation> {
    let mut previous: Option<&str> = None;
    for id in ids {
        match previous {
            Some(before) if before == id => {
                return Err(Violation::Duplicate {
                    what,
                    id: id.to_owned(),
                });
            }
            Some(before) if before > id => return Err(Violation::Unordered { what }),
            _ => previous = Some(id),
        }
    }
    Ok(())
}

fn check_kinds(package: &IntegratorPackage) -> Result<(), Violation> {
    let present: BTreeSet<FactKind> = package.facts.iter().map(|fact| fact.kind).collect();
    FactKind::ALL
        .into_iter()
        .find(|kind| !present.contains(kind))
        .map_or(Ok(()), |kind| Err(Violation::MissingKind(kind)))
}

fn check_fact(fact: &SourceFact) -> Result<(), Violation> {
    check_text("fact authority", &fact.authority)?;
    check_text("fact statement", &fact.statement)?;
    if let Some(value) = &fact.value {
        check_text("fact value", value)?;
    }
    for source in &fact.sources {
        check_text("source issuer", &source.issuer)?;
        check_text("source reference", &source.reference)?;
    }
    if fact.value.is_some() && fact.state != FactState::Established {
        return Err(Violation::UnestablishedValue {
            id: fact.id.as_str().to_owned(),
        });
    }
    if fact.state == FactState::Established {
        if fact.sources.is_empty() {
            return Err(Violation::Unsupported {
                id: fact.id.as_str().to_owned(),
                missing: "sources",
            });
        }
        if matches!(fact.kind, FactKind::License | FactKind::ReuseRight) && fact.value.is_none() {
            return Err(Violation::Unsupported {
                id: fact.id.as_str().to_owned(),
                missing: "value",
            });
        }
    }
    Ok(())
}

fn check_text(field: &'static str, text: &str) -> Result<(), Violation> {
    if text.is_empty() || text.len() > MAX_TEXT_BYTES || text.trim() != text {
        Err(Violation::Text { field })
    } else {
        Ok(())
    }
}

fn encode(package: &IntegratorPackage) -> Result<Vec<u8>, WriteError> {
    serde_json_canonicalizer::to_vec(package).map_err(|_| WriteError::Encoding)
}

/// Validates `package` and returns its canonical (RFC 8785) bytes.
///
/// The bytes are read back through [`read_package`] before they are returned,
/// so the writer cannot emit anything the reader refuses.
///
/// # Errors
///
/// [`WriteError`] when the package breaks a rule, has no canonical encoding,
/// is too large, or the protocol is not the supported one.
pub fn write_package(package: &IntegratorPackage) -> Result<Vec<u8>, WriteError> {
    if package.protocol != INTEGRATOR_PACKAGE_PROTOCOL {
        return Err(WriteError::Protocol);
    }
    check_package(package)?;
    let bytes = encode(package)?;
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(WriteError::TooLarge);
    }
    read_package(&bytes).map_err(WriteError::Readback)?;
    Ok(bytes)
}

/// Reads one package, refusing anything but the canonical encoding of a valid
/// package of the supported protocol.
///
/// Unknown fields and variants, duplicate keys, a different protocol version,
/// unordered or duplicated identities, a missing fact kind, a changed count,
/// an established fact without its basis and non-canonical bytes are all
/// refused. Nothing is repaired, defaulted or improved.
///
/// # Errors
///
/// The [`ReadError`] naming the first reason.
pub fn read_package(bytes: &[u8]) -> Result<IntegratorPackage, ReadError> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(ReadError::TooLarge);
    }
    let probe: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| ReadError::NotAPackage(error.to_string()))?;
    let protocol = probe
        .get("protocol")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| ReadError::NotAPackage("no string protocol".to_owned()))?;
    check_protocol(protocol)?;
    let package: IntegratorPackage =
        serde_json::from_slice(bytes).map_err(|error| ReadError::Malformed(error.to_string()))?;
    check_package(&package)?;
    let canonical = encode(&package).map_err(|_| ReadError::NotCanonical)?;
    if canonical != bytes {
        return Err(ReadError::NotCanonical);
    }
    Ok(package)
}

fn check_protocol(protocol: &str) -> Result<(), ReadError> {
    if protocol == INTEGRATOR_PACKAGE_PROTOCOL {
        return Ok(());
    }
    let Some(version) = protocol.strip_prefix(PROTOCOL_FAMILY) else {
        return Err(ReadError::UnknownProtocol(protocol.to_owned()));
    };
    let canonical = !version.is_empty()
        && version.bytes().all(|byte| byte.is_ascii_digit())
        && (version == "0" || !version.starts_with('0'));
    match version.parse::<u64>().ok().filter(|_| canonical).ok_or(()) {
        Ok(number) if number < SUPPORTED_VERSION => Err(ReadError::Downgraded(protocol.to_owned())),
        Ok(number) if number > SUPPORTED_VERSION => {
            Err(ReadError::UnsupportedVersion(protocol.to_owned()))
        }
        // `v01`, `v1x`, `v+1` and the like name no version.
        _ => Err(ReadError::UnknownProtocol(protocol.to_owned())),
    }
}
