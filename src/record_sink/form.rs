// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The public and private record forms and their closed value grammars
//! (FR-056).
//!
//! Every public member is a validated newtype or a closed enumeration, so a
//! [`PublicRecord`] can hold only identifiers, hashes, verdicts, timestamps and
//! a signature by construction. No member's grammar admits free text.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The protocol literal of the public form.
pub const PUBLIC_PROTOCOL: &str = "engineering-assurance.record-public/v1";
/// The protocol literal of the private form.
pub const PRIVATE_PROTOCOL: &str = "engineering-assurance.record-private/v1";
/// Most references a public form may carry.
pub const MAX_REFERENCES: usize = 64;
/// Most characters of a signature bundle.
pub const MAX_SIGNATURE_CHARS: usize = 16_384;

/// A refusal of a value or a record by the form rules.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum FormError {
    /// A value does not match the grammar of the member it was offered for.
    #[error("value does not match the grammar of `{member}`")]
    Grammar {
        /// The member name.
        member: &'static str,
    },
    /// The public form lists more than [`MAX_REFERENCES`] references.
    #[error("a public form lists at most {MAX_REFERENCES} references")]
    TooManyReferences,
    /// The two forms of one record carry different record ids.
    #[error("the public and private forms carry different record ids")]
    RecordIdMismatch,
    /// A record was offered with a signature; only the signing step sets one.
    #[error("a record is not accepted with a signature")]
    SignatureNotAccepted,
    /// The document is not valid JSON of the closed schema.
    #[error("record document is not valid for its closed schema")]
    Schema,
}

/// Deserializes a present optional member, rejecting JSON `null`: `null` is in
/// no member's grammar.
pub(crate) fn no_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

/// As [`no_null`] for free-form JSON members, where `null` is a valid value.
fn no_null_value<'de, D>(deserializer: D) -> Result<Option<serde_json::Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    if value.is_null() {
        Err(serde::de::Error::custom("null is not a valid member value"))
    } else {
        Ok(Some(value))
    }
}

const fn grammar(member: &'static str) -> FormError {
    FormError::Grammar { member }
}

fn is_lower_hex(text: &str) -> bool {
    text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

macro_rules! text_newtype {
    ($(#[$doc:meta])* $name:ident, $member:literal, $check:expr) => {
        $(#[$doc])*
        #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            /// The validated text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = FormError;

            fn try_from(value: String) -> Result<Self, FormError> {
                let valid: fn(&str) -> bool = $check;
                if valid(&value) {
                    Ok(Self(value))
                } else {
                    Err(grammar($member))
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = FormError;

            fn try_from(value: &str) -> Result<Self, FormError> {
                Self::try_from(value.to_owned())
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

fn is_uuid(text: &str) -> bool {
    text.len() == 36
        && text.char_indices().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => c == '-',
            _ => matches!(c, '0'..='9' | 'a'..='f'),
        })
}

fn is_sha256(text: &str) -> bool {
    text.strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && is_lower_hex(hex))
}

fn is_git_object_id(text: &str) -> bool {
    matches!(text.len(), 40 | 64) && is_lower_hex(text)
}

fn is_timestamp(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 20 {
        return false;
    }
    let digits = |range: std::ops::Range<usize>| -> Option<u32> {
        let part = &bytes[range];
        part.iter()
            .all(u8::is_ascii_digit)
            .then(|| part.iter().fold(0, |acc, d| acc * 10 + u32::from(d - b'0')))
    };
    let separators = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'Z'),
    ];
    separators.iter().all(|&(at, sep)| bytes[at] == sep)
        && digits(0..4).is_some()
        && digits(5..7).is_some_and(|m| (1..=12).contains(&m))
        && digits(8..10).is_some_and(|d| (1..=31).contains(&d))
        && digits(11..13).is_some_and(|h| h < 24)
        && digits(14..16).is_some_and(|m| m < 60)
        && digits(17..19).is_some_and(|s| s < 60)
}

fn is_base64_bundle(text: &str) -> bool {
    let padding = text.bytes().rev().take_while(|&b| b == b'=').count();
    !text.is_empty()
        && text.len() <= MAX_SIGNATURE_CHARS
        && text.len().is_multiple_of(4)
        && padding <= 2
        && text[..text.len() - padding]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/')
}

fn is_repo_id(text: &str) -> bool {
    let part_ok = |part: &str| {
        (1..=100).contains(&part.len())
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    };
    text.split_once('/')
        .is_some_and(|(owner, name)| part_ok(owner) && part_ok(name))
}

/// Splits a Linear issue id `TEAM-N` into its team key and number, or `None`
/// when it does not match the grammar.
#[must_use]
pub fn split_linear_issue(text: &str) -> Option<(&str, &str)> {
    let (team, number) = text.split_once('-')?;
    let team_ok = (1..=10).contains(&team.len())
        && team.as_bytes()[0].is_ascii_uppercase()
        && team
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    let number_ok = (1..=9).contains(&number.len()) && number.bytes().all(|b| b.is_ascii_digit());
    (team_ok && number_ok).then_some((team, number))
}

text_newtype!(
    /// A lowercase canonical UUID string naming one record.
    RecordId, "record_id", is_uuid
);
text_newtype!(
    /// `sha256:` followed by 64 lowercase hexadecimal characters.
    Sha256Digest, "hashes", is_sha256
);
text_newtype!(
    /// A git object id: 40 or 64 lowercase hexadecimal characters.
    GitObjectId, "hashes.subject_commit", is_git_object_id
);
text_newtype!(
    /// A UTC instant of the form `YYYY-MM-DDTHH:MM:SSZ`.
    Timestamp, "timestamps", is_timestamp
);
text_newtype!(
    /// A signature bundle in standard padded base64, at most 16384 characters.
    Signature, "signature", is_base64_bundle
);

impl RecordId {
    /// Builds a version-4 UUID record id from 16 random bytes. The id depends
    /// on the bytes alone, never on the record's content, ticket or repository.
    #[must_use]
    pub fn from_random_bytes(mut bytes: [u8; 16]) -> Self {
        bytes[6] = bytes[6] & 0x0f | 0x40;
        bytes[8] = bytes[8] & 0x3f | 0x80;
        let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Self(format!(
            "{}-{}-{}-{}-{}",
            hex[0..4].concat(),
            hex[4..6].concat(),
            hex[6..8].concat(),
            hex[8..10].concat(),
            hex[10..16].concat()
        ))
    }
}

/// The kind of a reference.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    /// A repository, `owner/name`.
    Repo,
    /// A Linear issue, `TEAM-N`.
    LinearIssue,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawReference {
    kind: RefKind,
    id: String,
}

/// One reference `{kind, id}` whose id matches the grammar of its kind.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawReference")]
pub struct Reference {
    kind: RefKind,
    id: String,
}

impl Reference {
    /// A `repo` reference.
    ///
    /// # Errors
    ///
    /// [`FormError::Grammar`] when `id` is not `owner/name`.
    pub fn repo(id: &str) -> Result<Self, FormError> {
        Self::new(RefKind::Repo, id)
    }

    /// A `linear_issue` reference.
    ///
    /// # Errors
    ///
    /// [`FormError::Grammar`] when `id` is not `TEAM-N`.
    pub fn linear_issue(id: &str) -> Result<Self, FormError> {
        Self::new(RefKind::LinearIssue, id)
    }

    fn new(kind: RefKind, id: &str) -> Result<Self, FormError> {
        let valid = match kind {
            RefKind::Repo => is_repo_id(id),
            RefKind::LinearIssue => split_linear_issue(id).is_some(),
        };
        if valid {
            Ok(Self {
                kind,
                id: id.to_owned(),
            })
        } else {
            Err(grammar("references"))
        }
    }

    /// The reference kind.
    #[must_use]
    pub const fn kind(&self) -> RefKind {
        self.kind
    }

    /// The reference id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
}

impl TryFrom<RawReference> for Reference {
    type Error = FormError;

    fn try_from(raw: RawReference) -> Result<Self, FormError> {
        Self::new(raw.kind, &raw.id)
    }
}

/// The repository a record is about: a `repo` reference.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Reference")]
pub struct Subject(Reference);

impl Subject {
    /// The subject for repository `id`.
    ///
    /// # Errors
    ///
    /// [`FormError::Grammar`] when `id` is not `owner/name`.
    pub fn repo(id: &str) -> Result<Self, FormError> {
        Reference::repo(id).map(Self)
    }

    /// The subject as a reference.
    #[must_use]
    pub const fn reference(&self) -> &Reference {
        &self.0
    }
}

impl TryFrom<Reference> for Subject {
    type Error = FormError;

    fn try_from(reference: Reference) -> Result<Self, FormError> {
        if reference.kind == RefKind::Repo {
            Ok(Self(reference))
        } else {
            Err(grammar("subject"))
        }
    }
}

/// At most [`MAX_REFERENCES`] references.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<Reference>")]
pub struct References(Vec<Reference>);

impl References {
    /// Collects the references.
    ///
    /// # Errors
    ///
    /// [`FormError::TooManyReferences`] above the limit.
    pub fn new(items: Vec<Reference>) -> Result<Self, FormError> {
        Self::try_from(items)
    }

    /// The references in order.
    #[must_use]
    pub fn as_slice(&self) -> &[Reference] {
        &self.0
    }
}

impl TryFrom<Vec<Reference>> for References {
    type Error = FormError;

    fn try_from(items: Vec<Reference>) -> Result<Self, FormError> {
        if items.len() > MAX_REFERENCES {
            Err(FormError::TooManyReferences)
        } else {
            Ok(Self(items))
        }
    }
}

/// The kind of record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    /// An approval statement.
    Approval,
    /// A change record.
    ChangeRecord,
    /// A release attestation.
    ReleaseAttestation,
}

/// A verdict value.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// An approval is satisfied.
    Satisfied,
    /// An approval is unsatisfied.
    Unsatisfied,
    /// A release is released.
    Released,
    /// A release is not released.
    NotReleased,
    /// A change record was recorded.
    Recorded,
    /// A change record records a failure.
    Failed,
}

/// The hashes of a public form; the names are the closed set.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hashes {
    /// The digest of the private form.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null"
    )]
    pub private_form: Option<Sha256Digest>,
    /// The digest of the statement.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null"
    )]
    pub statement: Option<Sha256Digest>,
    /// The subject commit.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null"
    )]
    pub subject_commit: Option<GitObjectId>,
}

/// The verdicts of a public form; the names are the closed set.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verdicts {
    /// The approval verdict.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null"
    )]
    pub approval: Option<Verdict>,
    /// The change-record verdict.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null"
    )]
    pub change_record: Option<Verdict>,
    /// The release verdict.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null"
    )]
    pub release: Option<Verdict>,
}

/// The timestamps of a public form; the names are the closed set.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Timestamps {
    /// When the record was created.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null"
    )]
    pub created: Option<Timestamp>,
    /// When it was signed.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null"
    )]
    pub signed: Option<Timestamp>,
    /// When it was published.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null"
    )]
    pub published: Option<Timestamp>,
}

/// The only protocol literal of the public form.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PublicProtocol {
    /// `engineering-assurance.record-public/v1`.
    #[serde(rename = "engineering-assurance.record-public/v1")]
    V1,
}

/// The only protocol literal of the private form.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PrivateProtocol {
    /// `engineering-assurance.record-private/v1`.
    #[serde(rename = "engineering-assurance.record-private/v1")]
    V1,
}

/// The public form, `engineering-assurance.record-public/v1`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicRecord {
    /// The protocol literal.
    pub protocol: PublicProtocol,
    /// The record id shared with the private form.
    pub record_id: RecordId,
    /// The record kind.
    pub kind: RecordKind,
    /// The repository the record is about.
    pub subject: Subject,
    /// The other repositories and issues the record names.
    pub references: References,
    /// The hashes.
    #[serde(default)]
    pub hashes: Hashes,
    /// The verdicts.
    #[serde(default)]
    pub verdicts: Verdicts,
    /// The timestamps.
    #[serde(default)]
    pub timestamps: Timestamps,
    /// The signature bundle; set only by the signing step.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null"
    )]
    pub signature: Option<Signature>,
}

impl PublicRecord {
    /// Parses and validates a public form.
    ///
    /// # Errors
    ///
    /// [`FormError::Schema`] for an unknown member, a member outside its
    /// enumeration or a value outside its grammar.
    pub fn from_json(bytes: &[u8]) -> Result<Self, FormError> {
        serde_json::from_slice(bytes).map_err(|_| FormError::Schema)
    }

    /// The RFC 8785 canonical bytes of the public form without its `signature`
    /// member: the only payload a signature bundle may sign.
    ///
    /// # Errors
    ///
    /// [`FormError::Schema`] if the form cannot be serialized.
    pub fn signed_bytes(&self) -> Result<Vec<u8>, FormError> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        serde_json_canonicalizer::to_vec(&unsigned).map_err(|_| FormError::Schema)
    }

    /// The RFC 8785 canonical bytes of the whole form, signature included: the
    /// entry a public log receives.
    ///
    /// # Errors
    ///
    /// [`FormError::Schema`] if the form cannot be serialized.
    pub fn entry_bytes(&self) -> Result<Vec<u8>, FormError> {
        serde_json_canonicalizer::to_vec(self).map_err(|_| FormError::Schema)
    }
}

/// The private form, `engineering-assurance.record-private/v1`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivateRecord {
    /// The protocol literal.
    pub protocol: PrivateProtocol,
    /// The record id shared with the public form.
    pub record_id: RecordId,
    /// The ticket text.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null"
    )]
    pub ticket_text: Option<String>,
    /// The criteria text.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null"
    )]
    pub criteria_text: Option<String>,
    /// The model names.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null"
    )]
    pub model_names: Option<Vec<String>>,
    /// The full approval statement.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null_value"
    )]
    pub statement: Option<serde_json::Value>,
    /// The full change record.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null_value"
    )]
    pub change_record: Option<serde_json::Value>,
    /// The agent bill of materials.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "no_null_value"
    )]
    pub agent_bom: Option<serde_json::Value>,
}

impl PrivateRecord {
    /// A private form holding no content yet.
    #[must_use]
    pub const fn new(record_id: RecordId) -> Self {
        Self {
            protocol: PrivateProtocol::V1,
            record_id,
            ticket_text: None,
            criteria_text: None,
            model_names: None,
            statement: None,
            change_record: None,
            agent_bom: None,
        }
    }

    /// Parses a private form.
    ///
    /// # Errors
    ///
    /// [`FormError::Schema`] for an unknown member or a malformed value.
    pub fn from_json(bytes: &[u8]) -> Result<Self, FormError> {
        serde_json::from_slice(bytes).map_err(|_| FormError::Schema)
    }
}

/// One record: its two forms, linked only by an equal `record_id`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Record {
    public: PublicRecord,
    private: PrivateRecord,
}

impl Record {
    /// Pairs the two forms.
    ///
    /// # Errors
    ///
    /// [`FormError::RecordIdMismatch`] when the ids differ and
    /// [`FormError::SignatureNotAccepted`] when the public form already carries
    /// a signature.
    pub fn new(public: PublicRecord, private: PrivateRecord) -> Result<Self, FormError> {
        if public.record_id != private.record_id {
            return Err(FormError::RecordIdMismatch);
        }
        if public.signature.is_some() {
            return Err(FormError::SignatureNotAccepted);
        }
        Ok(Self { public, private })
    }

    /// The shared record id.
    #[must_use]
    pub const fn id(&self) -> &RecordId {
        &self.public.record_id
    }

    /// The public form, unsigned.
    #[must_use]
    pub const fn public(&self) -> &PublicRecord {
        &self.public
    }

    /// The private form.
    #[must_use]
    pub const fn private(&self) -> &PrivateRecord {
        &self.private
    }
}
