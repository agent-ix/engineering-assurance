// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Message and preconditions of a signed approval tag (FR-042).
//!
//! The module is pure. It composes the tag message that `approval-verify`
//! (FR-041) reads back, decides whether a value can be carried in a trailer
//! unchanged, and decides from the values of the caller's git signing
//! configuration whether git can sign a tag that FR-036 and FR-040 can verify.
//! It signs nothing and starts nothing.

use crate::approval_verify::{RECORD_DIGEST_TRAILER, RECORD_ID_TRAILER, is_record_value};
use crate::signature_verify::MEANING_TOKEN;

/// Longest accepted `Signed-off-meaning` value in bytes.
pub const MAX_MEANING_BYTES: usize = 256;

/// Whether `value` survives a trailer round trip byte for byte: git trims
/// ASCII whitespace around a trailer value, so a value that starts or ends with
/// one would read back different.
fn survives_trailer(value: &str) -> bool {
    value.trim_matches(|c: char| c.is_ascii_whitespace()) == value
}

/// Whether `value` is an acceptable `record_id`, or a record digest read from a
/// record: a [`is_record_value`] that a trailer reads back unchanged.
#[must_use]
pub fn is_trailer_record_value(value: &str) -> bool {
    is_record_value(value) && survives_trailer(value)
}

/// Whether `value` is an acceptable record digest: an [`is_record_value`] of
/// the form `sha256:` and 64 lowercase hexadecimal characters, which is the
/// form the record's owner defines and `approval-verify` compares.
#[must_use]
pub fn is_record_digest(value: &str) -> bool {
    is_record_value(value)
        && value.strip_prefix("sha256:").is_some_and(|hex| {
            hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
        })
}

/// Whether `value` is an acceptable approval meaning: not empty, at most
/// [`MAX_MEANING_BYTES`] bytes, free of control characters, and read back
/// unchanged by a trailer. There is no default meaning.
#[must_use]
pub fn is_meaning(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_MEANING_BYTES
        && !value.chars().any(char::is_control)
        && survives_trailer(value)
}

/// The tag message: a subject paragraph, then the three trailers in order. The
/// subject paragraph is required because git reads no trailers from a message's
/// only paragraph.
#[must_use]
pub fn compose_message(record_id: &str, record_digest: &str, meaning: &str) -> String {
    format!(
        "Approve change-assurance record {record_id}\n\n\
         {RECORD_ID_TRAILER}: {record_id}\n\
         {RECORD_DIGEST_TRAILER}: {record_digest}\n\
         {MEANING_TOKEN}: {meaning}\n"
    )
}

/// A git setting whose absence or unsupported value makes git unable to sign a
/// tag that Engineering Assurance can verify.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SigningSetting {
    /// `gpg.format`: unset, or neither `ssh` nor `x509`.
    GpgFormat,
    /// `user.signingkey`: unset under `gpg.format=ssh`, with no default key
    /// command either.
    UserSigningKey,
}

impl SigningSetting {
    /// The git configuration key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::GpgFormat => "gpg.format",
            Self::UserSigningKey => "user.signingkey",
        }
    }
}

/// Whether a git setting has a non-empty value.
fn set(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.is_empty())
}

/// The setting that keeps git from signing a verifiable tag, or `None`.
///
/// `format` is `gpg.format`. The format must be `ssh` (FR-036) or `x509`, which
/// is how `gitsign` signs (FR-040); an unset format means `OpenPGP` to git, which
/// FR-036 does not verify. Under `ssh` git needs a key: `signing_key` is
/// `user.signingkey` and `default_key_command` is `gpg.ssh.defaultKeyCommand`.
/// Under `x509` the configured program chooses the key, so none is required.
#[must_use]
pub fn signing_refusal(
    format: Option<&str>,
    signing_key: Option<&str>,
    default_key_command: Option<&str>,
) -> Option<SigningSetting> {
    match format {
        Some("ssh") => (!set(signing_key) && !set(default_key_command))
            .then_some(SigningSetting::UserSigningKey),
        Some("x509") => None,
        _ => Some(SigningSetting::GpgFormat),
    }
}
