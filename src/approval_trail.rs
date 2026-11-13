// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Ordering of an approval trail and the test of whether a tag names a record
//! (FR-043).
//!
//! The module is pure. Every verdict comes from the `approval-verify` decision
//! (FR-041); this module only says which tags belong to the trail and in what
//! order they are listed.

use std::cmp::Ordering;

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
