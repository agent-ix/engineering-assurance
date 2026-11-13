// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The fail-closed pre-publish leak check (FR-057, NFR-009).
//!
//! The check decides from the public form alone and resolves the visibility of
//! the publishing repository, the subject and every reference. An item that is
//! private or whose visibility cannot be determined is an offender. A passing
//! check yields a [`PublicPathProof`], the only value the public signing path
//! and the public-log send accept.

use serde::{Deserialize, Serialize};

use super::config::Visibility;
use super::form::{PublicRecord, RefKind, split_linear_issue};

/// A visibility lookup failed, timed out or answered something unrecognized.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[error("visibility lookup failed")]
pub struct LookupFailed;

/// Resolves the visibility of a repository through the hosting service.
pub trait VisibilityLookup {
    /// The visibility of repository `owner/name`.
    ///
    /// # Errors
    ///
    /// [`LookupFailed`] when the lookup errors, times out or returns an
    /// unrecognized value.
    fn repo_visibility(&self, repo: &str) -> Result<Visibility, LookupFailed>;
}

/// Why an item is offending.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OffenseReason {
    /// The item is private.
    Private,
    /// Its visibility could not be determined.
    VisibilityUnknown,
}

/// One item the check refused.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offending {
    /// The item kind.
    pub kind: RefKind,
    /// The item id.
    pub id: String,
    /// Why it offends.
    pub reason: OffenseReason,
}

/// Proof that the leak check passed for one public form. Only [`leak_check`]
/// makes one, and it carries the RFC 8785 bytes of the checked form, so the
/// public signing path can receive nothing but the bytes that were checked.
#[derive(Debug)]
pub struct PublicPathProof<'a> {
    record: &'a PublicRecord,
    signed_bytes: Vec<u8>,
}

impl<'a> PublicPathProof<'a> {
    /// The public form the check passed for.
    #[must_use]
    pub const fn record(&self) -> &'a PublicRecord {
        self.record
    }

    /// The canonical bytes of the checked public form: the only payload the
    /// public signing path may sign.
    #[must_use]
    pub fn signed_bytes(&self) -> &[u8] {
        &self.signed_bytes
    }
}

/// Why the check did not pass.
#[derive(Debug, Eq, PartialEq)]
pub enum CheckRefusal {
    /// At least one item is private or of unknown visibility.
    Offending(Vec<Offending>),
    /// The public form could not be encoded, so nothing can be signed.
    Unencodable,
}

/// Checks the publishing repository, the subject and every reference.
///
/// # Errors
///
/// [`CheckRefusal::Offending`] lists every offending item, each once, in the
/// order publishing repository, subject, references.
pub fn leak_check<'a>(
    publishing_repo: &str,
    record: &'a PublicRecord,
    public_linear_teams: &[String],
    lookup: &dyn VisibilityLookup,
) -> Result<PublicPathProof<'a>, CheckRefusal> {
    let mut items: Vec<(RefKind, &str)> = vec![
        (RefKind::Repo, publishing_repo),
        (RefKind::Repo, record.subject.reference().id()),
    ];
    items.extend(
        record
            .references
            .as_slice()
            .iter()
            .map(|r| (r.kind(), r.id())),
    );

    let mut seen = Vec::new();
    let mut offending = Vec::new();
    for item in items {
        if seen.contains(&item) {
            continue;
        }
        seen.push(item);
        let (kind, id) = item;
        let visibility = match kind {
            RefKind::Repo => lookup.repo_visibility(id).unwrap_or(Visibility::Unknown),
            RefKind::LinearIssue => linear_visibility(id, public_linear_teams),
        };
        let reason = match visibility {
            Visibility::Public => continue,
            Visibility::Private => OffenseReason::Private,
            Visibility::Unknown => OffenseReason::VisibilityUnknown,
        };
        offending.push(Offending {
            kind,
            id: id.to_owned(),
            reason,
        });
    }
    if !offending.is_empty() {
        return Err(CheckRefusal::Offending(offending));
    }
    let signed_bytes = record
        .signed_bytes()
        .map_err(|_| CheckRefusal::Unencodable)?;
    Ok(PublicPathProof {
        record,
        signed_bytes,
    })
}

fn linear_visibility(issue: &str, public_teams: &[String]) -> Visibility {
    match split_linear_issue(issue) {
        Some((team, _)) if public_teams.iter().any(|t| t == team) => Visibility::Public,
        _ => Visibility::Private,
    }
}
