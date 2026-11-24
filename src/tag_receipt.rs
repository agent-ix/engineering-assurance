// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Tag receipts and the comparison that finds deleted tags (FR-081, FR-082).
//!
//! A tag receipt is one signed in-toto statement, written when a matching tag
//! is pushed and stored as one Sigstore bundle per line of a Git note on the
//! peeled commit. This module is pure: it builds and parses the statement,
//! verifies one note line through the delegated verifier (NFR-010), edits a
//! note's bytes and compares the receipts with the current tag set. Reading
//! git and signing live in the binary's adapter. A receipt records what was
//! pushed and never what the tag means.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::attestation_signing::{
    BundleVerifier, STATEMENT_V1_TYPE, SourceWorkflowPolicy, SubjectV1, VerificationFailure,
    WorkflowIdentityPolicy,
};

/// Git notes ref that stores tag receipts (FR-081). A different ref from the
/// change receipts' `refs/notes/assurance-receipts`.
pub const TAG_RECEIPT_NOTES_REF: &str = "refs/notes/assurance-tag-receipts";
/// Largest note a tag receipt may be stored in (FR-081).
pub const MAX_TAG_RECEIPT_NOTE_BYTES: usize = 1024 * 1024;
/// The predicate type of a tag receipt statement; an identity string only.
pub const TAG_RECEIPT_PREDICATE_TYPE: &str =
    concat!("https:", "//schemas.agent-ix.ai/attestation/tag-receipt/v1");
/// Most `Approved-Record-Id` values a readable receipt carries.
pub const MAX_RECORD_IDS: usize = 64;
/// Longest `Approved-Record-Id` value a readable receipt carries, in bytes.
pub const MAX_RECORD_ID_BYTES: usize = 256;

const TAG_REF_PREFIX: &str = "refs/tags/";
const MAX_PREFIXES: usize = 16;
const MAX_PREFIX_BYTES: usize = 128;
const MAX_ACCEPTED_TAGS: usize = 1000;
const MAX_ACCEPTED_BY_BYTES: usize = 256;
const MAX_REASON_BYTES: usize = 1024;

/// The `tag_receipts` member of the receipt policy is not a valid closed object.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("tag_receipt_policy_invalid")]
pub struct TagReceiptPolicyInvalid;

/// A person's acceptance of one specific finding (FR-082).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedTag {
    /// The full ref name, `refs/tags/<name>`.
    pub reference: String,
    /// The object id the acceptance names; `None` accepts only `tag_absent`.
    pub tag_object: Option<String>,
    /// The person who accepted the finding.
    pub accepted_by: String,
    /// Why the finding is accepted.
    pub reason: String,
}

/// The validated `tag_receipts` member of the receipt policy (FR-081).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TagReceiptPolicy {
    /// Literal prefixes of the tag names that get a receipt.
    pub prefixes: Vec<String>,
    /// Acceptances of specific findings, read as data.
    pub accepted_tags: Vec<AcceptedTag>,
}

impl TagReceiptPolicy {
    /// Reads the `tag_receipts` member out of a whole receipt policy document.
    /// `Ok(None)` when the document has no such member.
    ///
    /// # Errors
    ///
    /// [`TagReceiptPolicyInvalid`] when the document is not a JSON object or
    /// the member breaks its closed schema.
    pub fn from_receipt_policy(bytes: &[u8]) -> Result<Option<Self>, TagReceiptPolicyInvalid> {
        let document: Value = serde_json::from_slice(bytes).map_err(|_| TagReceiptPolicyInvalid)?;
        let object = document.as_object().ok_or(TagReceiptPolicyInvalid)?;
        object.get("tag_receipts").map(Self::parse).transpose()
    }

    /// Validates one `tag_receipts` member.
    ///
    /// # Errors
    ///
    /// [`TagReceiptPolicyInvalid`] for an unknown or missing member, a value
    /// outside its bounds or a ref that is not a full tag ref name.
    pub fn parse(member: &Value) -> Result<Self, TagReceiptPolicyInvalid> {
        let object = member.as_object().ok_or(TagReceiptPolicyInvalid)?;
        if object
            .keys()
            .any(|key| !matches!(key.as_str(), "prefixes" | "accepted_tags"))
        {
            return Err(TagReceiptPolicyInvalid);
        }
        let prefixes = object
            .get("prefixes")
            .and_then(Value::as_array)
            .ok_or(TagReceiptPolicyInvalid)?
            .iter()
            .map(|prefix| prefix.as_str().map(str::to_owned))
            .collect::<Option<Vec<_>>>()
            .ok_or(TagReceiptPolicyInvalid)?;
        if prefixes.is_empty()
            || prefixes.len() > MAX_PREFIXES
            || prefixes
                .iter()
                .any(|prefix| prefix.is_empty() || prefix.len() > MAX_PREFIX_BYTES)
        {
            return Err(TagReceiptPolicyInvalid);
        }
        let accepted_tags = match object.get("accepted_tags") {
            None => Vec::new(),
            Some(list) => {
                let list = list.as_array().ok_or(TagReceiptPolicyInvalid)?;
                if list.len() > MAX_ACCEPTED_TAGS {
                    return Err(TagReceiptPolicyInvalid);
                }
                list.iter()
                    .map(parse_accepted_tag)
                    .collect::<Result<Vec<_>, _>>()?
            }
        };
        Ok(Self {
            prefixes,
            accepted_tags,
        })
    }

    /// Whether the tag `name` (without `refs/tags/`) starts with a policy
    /// prefix, compared as literal text.
    #[must_use]
    pub fn covers(&self, name: &str) -> bool {
        self.prefixes.iter().any(|prefix| name.starts_with(prefix))
    }

    /// The entry that accepts a finding for `reference` at `tag_object`: an
    /// exact match on both members, or, for an absent tag only, a null object.
    fn acceptance(
        &self,
        reference: &str,
        tag_object: Option<&str>,
        tag_absent: bool,
    ) -> Option<&AcceptedTag> {
        self.accepted_tags.iter().find(|entry| {
            entry.reference == reference
                && match (entry.tag_object.as_deref(), tag_object) {
                    (Some(entry_object), Some(object)) => entry_object == object,
                    (None, _) => tag_absent,
                    (Some(_), None) => false,
                }
        })
    }

    /// The entry that accepts the unreceipted current tag `reference` at its
    /// current object id (FR-082). A null entry never matches.
    #[must_use]
    pub fn accepts_unreceipted(&self, reference: &str, current_object: &str) -> Option<Acceptance> {
        self.acceptance(reference, Some(current_object), false)
            .map(Acceptance::from)
    }
}

fn parse_accepted_tag(entry: &Value) -> Result<AcceptedTag, TagReceiptPolicyInvalid> {
    let object = entry.as_object().ok_or(TagReceiptPolicyInvalid)?;
    if object.len() != 4 {
        return Err(TagReceiptPolicyInvalid);
    }
    let text = |key: &str, limit: usize| -> Result<String, TagReceiptPolicyInvalid> {
        object
            .get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty() && value.len() <= limit)
            .map(str::to_owned)
            .ok_or(TagReceiptPolicyInvalid)
    };
    let reference = text("ref", usize::MAX)?;
    if !reference.starts_with(TAG_REF_PREFIX) || reference.len() == TAG_REF_PREFIX.len() {
        return Err(TagReceiptPolicyInvalid);
    }
    let tag_object = match object.get("tag_object").ok_or(TagReceiptPolicyInvalid)? {
        Value::Null => None,
        Value::String(id) if is_object_id(id) => Some(id.clone()),
        _ => return Err(TagReceiptPolicyInvalid),
    };
    Ok(AcceptedTag {
        reference,
        tag_object,
        accepted_by: text("accepted_by", MAX_ACCEPTED_BY_BYTES)?,
        reason: text("reason", MAX_REASON_BYTES)?,
    })
}

fn is_object_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_utc_instant(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 20
        && bytes.iter().enumerate().all(|(index, byte)| match index {
            4 | 7 => *byte == b'-',
            10 => *byte == b'T',
            13 | 16 => *byte == b':',
            19 => *byte == b'Z',
            _ => byte.is_ascii_digit(),
        })
}

/// The members of a tag receipt predicate (FR-081): exactly these.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TagReceiptPredicate {
    /// The repository, exact `owner/name`.
    pub repository: String,
    /// The full ref name, equal to the subject name.
    #[serde(rename = "ref")]
    pub reference: String,
    /// The tag object id, equal to the subject digest.
    pub tag_object: String,
    /// The commit the tag peels to, which carries the note.
    pub commit: String,
    /// The platform's push time as a UTC instant.
    pub pushed_at: String,
    /// Whether the tag message trailers were read.
    pub trailers_readable: bool,
    /// Every `Approved-Record-Id` trailer value, byte for byte, in order.
    pub record_ids: Vec<String>,
}

/// The closed in-toto statement of a tag receipt (FR-081).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TagReceiptStatement {
    /// The in-toto statement type.
    #[serde(rename = "_type")]
    pub type_: String,
    /// The one subject, the tag ref and its object id.
    pub subject: Vec<SubjectV1>,
    /// The predicate type.
    #[serde(rename = "predicateType")]
    pub predicate_type: String,
    /// The predicate.
    pub predicate: TagReceiptPredicate,
}

/// The record trailers of one tag as a receipt carries them (FR-081).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordTrailers {
    /// Whether the trailers were read.
    pub readable: bool,
    /// The values, empty when they were not read.
    pub record_ids: Vec<String>,
}

impl RecordTrailers {
    /// The trailers of a tag message whose parsed `Approved-Record-Id` values
    /// are `values`: unreadable above 64 values or when a value is over 256
    /// bytes.
    #[must_use]
    pub fn from_values(values: Vec<String>) -> Self {
        if values.len() > MAX_RECORD_IDS
            || values.iter().any(|value| value.len() > MAX_RECORD_ID_BYTES)
        {
            return Self::unreadable();
        }
        Self {
            readable: true,
            record_ids: values,
        }
    }

    /// A tag message that cannot be read for any reason of FR-043.
    #[must_use]
    pub const fn unreadable() -> Self {
        Self {
            readable: false,
            record_ids: Vec::new(),
        }
    }
}

impl TagReceiptStatement {
    /// Builds the statement for the tag `reference` at `tag_object`.
    #[must_use]
    pub fn new(
        repository: &str,
        reference: &str,
        tag_object: &str,
        commit: &str,
        pushed_at: &str,
        trailers: RecordTrailers,
    ) -> Self {
        Self {
            type_: STATEMENT_V1_TYPE.to_owned(),
            subject: vec![SubjectV1 {
                name: reference.to_owned(),
                digest: BTreeMap::from([("gitTag".to_owned(), tag_object.to_owned())]),
            }],
            predicate_type: TAG_RECEIPT_PREDICATE_TYPE.to_owned(),
            predicate: TagReceiptPredicate {
                repository: repository.to_owned(),
                reference: reference.to_owned(),
                tag_object: tag_object.to_owned(),
                commit: commit.to_owned(),
                pushed_at: pushed_at.to_owned(),
                trailers_readable: trailers.readable,
                record_ids: trailers.record_ids,
            },
        }
    }

    /// The RFC 8785 bytes that are signed.
    ///
    /// # Errors
    ///
    /// [`TagReceiptError::Encoding`] when the statement cannot be encoded.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, TagReceiptError> {
        serde_json_canonicalizer::to_vec(self).map_err(|_| TagReceiptError::Encoding)
    }

    /// Whether the statement keeps every rule of the closed schema that the
    /// type system does not: the types, the one `gitTag` digest, the shapes of
    /// the ids and the instant, and the bounds of the trailer values.
    fn is_schema_valid(&self) -> bool {
        let predicate = &self.predicate;
        self.type_ == STATEMENT_V1_TYPE
            && self.predicate_type == TAG_RECEIPT_PREDICATE_TYPE
            && self.subject.len() == 1
            && self.subject[0].digest.len() == 1
            && self.subject[0].digest.contains_key("gitTag")
            && predicate.reference.starts_with(TAG_REF_PREFIX)
            && is_object_id(&predicate.tag_object)
            && is_object_id(&predicate.commit)
            && is_utc_instant(&predicate.pushed_at)
            && !predicate.repository.is_empty()
            && predicate.record_ids.len() <= MAX_RECORD_IDS
            && predicate
                .record_ids
                .iter()
                .all(|value| value.len() <= MAX_RECORD_ID_BYTES)
            && (predicate.trailers_readable || predicate.record_ids.is_empty())
    }
}

/// A failure that is not a verdict on a receipt.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum TagReceiptError {
    /// A statement or note line could not be encoded.
    #[error("tag receipt cannot be encoded")]
    Encoding,
    /// The note would exceed 1 MiB.
    #[error("tag_receipt_note_too_large")]
    NoteTooLarge,
    /// The note or the new bundle is not valid JSON Lines of bundle objects.
    #[error("tag receipt note is malformed")]
    NoteInvalid,
}

/// Why a note line is not a trusted tag receipt (FR-081).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidReason {
    /// The delegated verification rejected the signature.
    SignatureInvalid,
    /// The signer is not a listed tag receipt workflow identity.
    UntrustedSigner,
    /// The line is not a bundle of a statement that keeps the closed schema.
    PredicateInvalid,
    /// The subject, ref, tag object, commit or repository disagree.
    WrongSubject,
}

/// A tag receipt that verified.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TagReceipt {
    /// The full ref name, `refs/tags/<name>`.
    pub reference: String,
    /// The tag object id the ref named when pushed.
    pub tag_object: String,
    /// The commit that carries the note.
    pub commit: String,
    /// The platform's push time.
    pub pushed_at: String,
    /// Whether the trailers were read.
    pub trailers_readable: bool,
    /// The captured `Approved-Record-Id` values.
    pub record_ids: Vec<String>,
}

/// A note line that did not verify.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidReceipt {
    /// The commit whose note holds the line.
    pub commit: String,
    /// Why it is not trusted.
    pub reason: InvalidReason,
}

/// One candidate receipt: every line of the note is one (FR-082).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Candidate {
    /// A receipt that verified under FR-081.
    Verified(TagReceipt),
    /// A line that did not.
    Invalid(InvalidReceipt),
}

/// The signers one verifier is tried against.
pub struct SignerSet<'a> {
    /// The delegated bundle verifier, built from one trusted root.
    pub verifier: &'a dyn BundleVerifier,
    /// The `receipt_writer` identities with `source_ref_prefix`, each with its
    /// caller policy.
    pub signers: &'a [(WorkflowIdentityPolicy, SourceWorkflowPolicy)],
}

/// The lines of a note, in order; `None` for a line that is not a JSON object.
/// A note over 1 MiB is one `None`: nothing in it can be trusted.
#[must_use]
pub fn note_lines(note: &[u8]) -> Vec<Option<Value>> {
    if note.len() > MAX_TAG_RECEIPT_NOTE_BYTES {
        return vec![None];
    }
    let mut lines: Vec<&[u8]> = note.split(|byte| *byte == b'\n').collect();
    if lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
        .into_iter()
        .map(|line| {
            serde_json::from_slice::<Value>(line)
                .ok()
                .filter(Value::is_object)
        })
        .collect()
}

fn bundle_payload(bundle: &Value) -> Option<(Vec<u8>, String)> {
    let bundle_json = serde_json::to_string(bundle).ok()?;
    let parsed = sigstore_types::Bundle::from_json(&bundle_json).ok()?;
    let sigstore_types::SignatureContent::DsseEnvelope(envelope) = parsed.content else {
        return None;
    };
    Some((envelope.payload.as_ref().to_vec(), bundle_json))
}

/// Verifies one note line of the note on `commit` (FR-081).
///
/// Verification is the delegated stack's; this checks which listed identity
/// signed it, that the certificate's source ref equals the statement's `ref`,
/// the closed schema and that the subject, `ref`, `tag_object`, `commit` and
/// `repository` agree.
#[must_use]
pub fn verify_line(line: Option<&Value>, commit: &str, sets: &[SignerSet<'_>]) -> Candidate {
    let invalid = |reason| {
        Candidate::Invalid(InvalidReceipt {
            commit: commit.to_owned(),
            reason,
        })
    };
    let Some((payload, bundle_json)) = line.and_then(bundle_payload) else {
        return invalid(InvalidReason::PredicateInvalid);
    };
    let mut saw_untrusted = false;
    let mut matched = None;
    'sets: for set in sets {
        for (signer, source) in set.signers {
            match set.verifier.verify_with_claims(&payload, &bundle_json, signer) {
                Ok(Some(claims)) if source.matches(&claims) => {
                    matched = Some((claims, source));
                    break 'sets;
                }
                Ok(_) | Err(VerificationFailure::UntrustedSigner) => saw_untrusted = true,
                Err(_) => {}
            }
        }
    }
    let Some((claims, source)) = matched else {
        return invalid(if saw_untrusted {
            InvalidReason::UntrustedSigner
        } else {
            InvalidReason::SignatureInvalid
        });
    };
    let Ok(statement) = serde_json::from_slice::<TagReceiptStatement>(&payload) else {
        return invalid(InvalidReason::PredicateInvalid);
    };
    if !statement.is_schema_valid() {
        return invalid(InvalidReason::PredicateInvalid);
    }
    let predicate = statement.predicate;
    if claims.source_repository_ref.as_deref() != Some(predicate.reference.as_str()) {
        return invalid(InvalidReason::UntrustedSigner);
    }
    let subject = &statement.subject[0];
    if subject.name != predicate.reference
        || subject.digest.get("gitTag") != Some(&predicate.tag_object)
        || predicate.commit != commit
        || predicate.repository != source.repository
    {
        return invalid(InvalidReason::WrongSubject);
    }
    Candidate::Verified(TagReceipt {
        reference: predicate.reference,
        tag_object: predicate.tag_object,
        commit: predicate.commit,
        pushed_at: predicate.pushed_at,
        trailers_readable: predicate.trailers_readable,
        record_ids: predicate.record_ids,
    })
}

/// The result of adding an entry to a note.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NoteUpdate {
    /// The note already holds an entry with the same `ref` and `tag_object`.
    Unchanged,
    /// The new note bytes: every earlier line unchanged, then the new bundle.
    Updated(Vec<u8>),
}

/// The closed statement a bundle carries, read without verifying it; `None`
/// when the bundle holds no statement that keeps the closed schema.
#[must_use]
pub fn parse_statement(bundle: &Value) -> Option<TagReceiptStatement> {
    let (payload, _) = bundle_payload(bundle)?;
    let statement: TagReceiptStatement = serde_json::from_slice(&payload).ok()?;
    statement.is_schema_valid().then_some(statement)
}

/// The `ref` and `tag_object` of a bundle's statement, read without verifying.
fn entry_key(bundle: &Value) -> Option<(String, String)> {
    let (payload, _) = bundle_payload(bundle)?;
    let statement: Value = serde_json::from_slice(&payload).ok()?;
    let predicate = statement.get("predicate")?;
    Some((
        predicate.get("ref")?.as_str()?.to_owned(),
        predicate.get("tag_object")?.as_str()?.to_owned(),
    ))
}

/// Adds the signed `bundle` to the note `existing` (FR-081): no second entry
/// for a `ref` and `tag_object` the note holds, a new entry for a different
/// object id, and no earlier line removed or replaced.
///
/// # Errors
///
/// [`TagReceiptError::NoteInvalid`] when `bundle` is not a bundle of a tag
/// receipt statement, [`TagReceiptError::NoteTooLarge`] when the result would
/// exceed 1 MiB.
pub fn add_entry(existing: Option<&[u8]>, bundle: &Value) -> Result<NoteUpdate, TagReceiptError> {
    let key = entry_key(bundle).ok_or(TagReceiptError::NoteInvalid)?;
    let mut note = existing.map(<[u8]>::to_vec).unwrap_or_default();
    if note.len() > MAX_TAG_RECEIPT_NOTE_BYTES {
        return Err(TagReceiptError::NoteTooLarge);
    }
    if note_lines(&note)
        .iter()
        .flatten()
        .any(|line| entry_key(line).as_ref() == Some(&key))
    {
        return Ok(NoteUpdate::Unchanged);
    }
    if !note.is_empty() && note.last() != Some(&b'\n') {
        note.push(b'\n');
    }
    serde_json::to_writer(&mut note, bundle).map_err(|_| TagReceiptError::Encoding)?;
    note.push(b'\n');
    if note.len() > MAX_TAG_RECEIPT_NOTE_BYTES {
        return Err(TagReceiptError::NoteTooLarge);
    }
    Ok(NoteUpdate::Updated(note))
}

/// The status of a tag finding.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    /// A receipted tag is absent or changed.
    MissingTag,
    /// A current tag with a policy prefix has no receipt for its object.
    TagNoReceipt,
    /// A receipt that does not verify.
    InvalidReceipt,
}

/// Why a receipted tag is missing (FR-082).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingTagReason {
    /// The ref does not exist.
    TagAbsent,
    /// The ref names a different object.
    TagChanged,
}

/// One finding of the comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TagFinding {
    /// A verified receipt whose tag is absent or changed.
    MissingTag {
        /// The receipt.
        receipt: TagReceipt,
        /// Absent or changed.
        reason: MissingTagReason,
        /// The object the ref names now, for `tag_changed`.
        current_object: Option<String>,
    },
    /// A current tag with a policy prefix and no verified receipt for it.
    TagNoReceipt {
        /// The full ref name.
        reference: String,
        /// The object the ref names now.
        current_object: String,
    },
    /// A receipt that does not verify.
    InvalidReceipt(InvalidReceipt),
}

impl TagFinding {
    /// The status the finding has before any acceptance.
    #[must_use]
    pub const fn status(&self) -> FindingStatus {
        match self {
            Self::MissingTag { .. } => FindingStatus::MissingTag,
            Self::TagNoReceipt { .. } => FindingStatus::TagNoReceipt,
            Self::InvalidReceipt(_) => FindingStatus::InvalidReceipt,
        }
    }
}

/// The person and reason of an acceptance, as FR-082 reports them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Acceptance {
    /// The entry's `accepted_by`.
    pub accepted_by: String,
    /// The entry's `reason`.
    pub accepted_reason: String,
}

impl From<&AcceptedTag> for Acceptance {
    fn from(entry: &AcceptedTag) -> Self {
        Self {
            accepted_by: entry.accepted_by.clone(),
            accepted_reason: entry.reason.clone(),
        }
    }
}

/// A finding and the acceptance that matches it, if any.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Judged {
    /// The finding.
    pub finding: TagFinding,
    /// The matching `accepted_tags` entry; never present for an invalid receipt.
    pub accepted: Option<Acceptance>,
}

impl Judged {
    /// Whether the finding counts against a result: not accepted.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.accepted.is_none()
    }
}

/// Whether a verified receipt names `reference` at exactly `object`.
#[must_use]
pub fn is_receipted(candidates: &[Candidate], reference: &str, object: &str) -> bool {
    candidates.iter().any(|candidate| {
        matches!(candidate, Candidate::Verified(receipt)
            if receipt.reference == reference && receipt.tag_object == object)
    })
}

/// The one comparison of FR-082, a pure function of the candidate receipts,
/// the current tag set (full ref name to object id) and the policy.
///
/// Each receipt is judged on its own object id. A receipt that does not
/// verify is listed and never judged missing. The findings are in a fixed
/// order, so two callers with equal inputs get equal outputs.
#[must_use]
pub fn compare(
    candidates: &[Candidate],
    current_tags: &BTreeMap<String, String>,
    policy: &TagReceiptPolicy,
) -> Vec<Judged> {
    let mut judged = Vec::new();
    for candidate in candidates {
        match candidate {
            Candidate::Invalid(invalid) => judged.push(Judged {
                finding: TagFinding::InvalidReceipt(invalid.clone()),
                accepted: None,
            }),
            Candidate::Verified(receipt) => {
                let (reason, current_object) = match current_tags.get(&receipt.reference) {
                    None => (MissingTagReason::TagAbsent, None),
                    Some(object) if *object == receipt.tag_object => continue,
                    Some(object) => (MissingTagReason::TagChanged, Some(object.clone())),
                };
                let accepted = policy
                    .acceptance(
                        &receipt.reference,
                        Some(&receipt.tag_object),
                        reason == MissingTagReason::TagAbsent,
                    )
                    .map(Acceptance::from);
                judged.push(Judged {
                    finding: TagFinding::MissingTag {
                        receipt: receipt.clone(),
                        reason,
                        current_object,
                    },
                    accepted,
                });
            }
        }
    }
    for (reference, object) in current_tags {
        let name = reference.strip_prefix(TAG_REF_PREFIX).unwrap_or(reference);
        if policy.covers(name) && !is_receipted(candidates, reference, object) {
            judged.push(Judged {
                finding: TagFinding::TagNoReceipt {
                    reference: reference.clone(),
                    current_object: object.clone(),
                },
                accepted: policy.accepts_unreceipted(reference, object),
            });
        }
    }
    judged.sort_by(|left, right| sort_key(&left.finding).cmp(&sort_key(&right.finding)));
    judged
}

fn sort_key(finding: &TagFinding) -> (u8, &str, &str, &str) {
    match finding {
        TagFinding::MissingTag { receipt, .. } => (
            0,
            &receipt.reference,
            &receipt.tag_object,
            &receipt.commit,
        ),
        TagFinding::TagNoReceipt {
            reference,
            current_object,
        } => (1, reference, current_object, ""),
        TagFinding::InvalidReceipt(invalid) => (2, "", "", &invalid.commit),
    }
}
