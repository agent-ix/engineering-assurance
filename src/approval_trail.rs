// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Ordering of an approval trail and the test of whether a tag names a record
//! (FR-043).
//!
//! The module is pure. Every verdict comes from the `approval-verify` decision
//! (FR-041); this module only says which tags belong to the trail and in what
//! order they are listed.

use std::cmp::Ordering;
use std::fmt::Write as _;

use serde::Serialize;

use crate::approval_verify::UnverifiableReason;
use crate::signature_verify::Reason;

/// Most refs under `refs/tags` the trail examines.
pub const MAX_TAGS_CONSIDERED: usize = 10_000;

/// Whether a tag whose parsed `Approved-Record-Id` trailer values are
/// `record_id_values` names the record `record_id`: any value equals it byte for
/// byte. A tag repeating the trailer is still the record's, and is listed with
/// the binding `repeated`.
#[must_use]
pub fn names_record(record_id_values: &[String], record_id: &str) -> bool {
    record_id_values.iter().any(|value| value == record_id)
}

/// The place of one entry in the trail: the time the tag claims
/// (`YYYY-MM-DDTHH:MM:SSZ`, or `None` when unreadable) and the tag name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OrderKey<'a> {
    /// The claimed UTC instant; the signer's own claim.
    pub claimed_at: Option<&'a str>,
    /// The tag name.
    pub tag: &'a str,
}

/// Orders entries by claimed time ascending, entries with no time after all
/// entries with one, and equal or absent times by tag name in byte order. The
/// instants have a fixed width, so text order is time order.
#[must_use]
pub fn compare(left: &OrderKey<'_>, right: &OrderKey<'_>) -> Ordering {
    match (left.claimed_at, right.claimed_at) {
        (Some(a), Some(b)) => a.cmp(b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
    .then_with(|| left.tag.as_bytes().cmp(right.tag.as_bytes()))
}

/// The single value of the reason `tag_name_not_utf8`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TagNameNotUtf8 {
    /// The ref name is not valid UTF-8, so the tag is not read.
    TagNameNotUtf8,
}

/// Why an annotated tag that could name the record is not in the trail: the
/// FR-041 reason a message or tag cannot be read, or a name that is not UTF-8.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum UnreadableReason {
    /// A reason `approval-verify` gives for an `unverifiable` tag.
    Unverifiable(UnverifiableReason),
    /// The ref name is not UTF-8.
    Name(TagNameNotUtf8),
}

impl UnreadableReason {
    /// A failure to resolve or read the tag object, as `approval-verify` gives it.
    #[must_use]
    pub const fn read_failure(reason: Reason) -> Self {
        Self::Unverifiable(UnverifiableReason::Signature(reason))
    }
}

/// A tag name for display: UTF-8 control characters and every byte that is not
/// part of valid UTF-8 are written `\xNN` (a backslash cannot occur in a git ref
/// name, so the escape is unambiguous). The flag says whether anything was
/// escaped.
#[must_use]
pub fn escape_name(name: &[u8]) -> (String, bool) {
    let mut out = String::new();
    let mut escaped = false;
    for chunk in name.utf8_chunks() {
        for c in chunk.valid().chars() {
            if c.is_control() {
                escaped = true;
                let _ = write!(out, "\\x{:02x}", u32::from(c));
            } else {
                out.push(c);
            }
        }
        for byte in chunk.invalid() {
            escaped = true;
            let _ = write!(out, "\\x{byte:02x}");
        }
    }
    (out, escaped)
}

/// The bytes of `name` as lower-case hexadecimal.
#[must_use]
pub fn hex(name: &[u8]) -> String {
    name.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// Whether `c` is hidden or reorders text when printed: a control character, a
/// Unicode format character (bidirectional controls, zero-width characters,
/// joiners, the byte-order mark and tag characters), or a line or paragraph
/// separator.
#[must_use]
pub fn is_unprintable(c: char) -> bool {
    c.is_control()
        || matches!(
            c,
            '\u{ad}'
                | '\u{600}'..='\u{605}'
                | '\u{61c}'
                | '\u{6dd}'
                | '\u{70f}'
                | '\u{180e}'
                | '\u{200b}'..='\u{200f}'
                | '\u{2028}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
                | '\u{fff9}'..='\u{fffb}'
                | '\u{e0001}'
                | '\u{e0020}'..='\u{e007f}'
        )
}
