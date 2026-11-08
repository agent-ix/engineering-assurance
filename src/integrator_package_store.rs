// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Candidate-bound, atomic and idempotent publication of integrator packages
//! (FR-031).
//!
//! A package lives at `<root>/<candidate>/<attempt>.json`. The path is the
//! binding: the reader refuses an object whose own candidate and attempt
//! differ from the path that holds it.
//!
//! - **Temporary until verified.** The bytes are produced by
//!   [`write_package`], which has
//!   already read them back strictly, before anything touches the store. They
//!   then reach the destination only through
//!   [`write_file_atomic_no_replace`]:
//!   a sidecar that no reader looks at, committed by a hard link that cannot
//!   replace. After the commit the object is read back from the store.
//! - **Idempotent.** Publishing identical bytes again is
//!   [`Published::AlreadyPresent`].
//! - **Conflicts, not winners.** Different bytes for an attempt that already
//!   exists are never written. The refused writer records a conflict marker
//!   beside the object, and from then on the attempt reads as a conflict for
//!   everyone: neither writer's bytes is presented. A reader that looks in the
//!   instant between the winner's commit and the loser's marker can still see
//!   the winner's object; a consumer that must exclude that window waits for
//!   the writers it knows about to return. Likewise the creating writer's own
//!   read-back reports the conflict if a loser's marker is already there, so
//!   with non-identical writers "at most one `Written`" is the guarantee, not
//!   "exactly one".
//!
//! This is a filesystem module (like `atomic_publish`), exempt from the
//! FR-014-AC-4 I/O-freedom audit.

use std::{
    fs,
    io::{self, Read as _},
    path::{Path, PathBuf},
};

use thiserror::Error;

use crate::{
    atomic_publish::{PublishError, Published, write_file_atomic_no_replace},
    integrator_package::{
        AttemptId, CandidateId, IntegratorPackage, MAX_PACKAGE_BYTES, ReadError, WriteError,
        read_package, write_package,
    },
};

/// The fixed content of a conflict marker; the attempt's identity is its path.
const CONFLICT_MARKER: &[u8] = b"engineering-assurance.integrator-package-conflict/v1\n";

/// The collision value handed to the no-replace publisher.
#[derive(Clone, Copy, Debug)]
pub struct Occupied;

/// A successful publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishedPackage {
    /// Where the verified object is.
    pub path: PathBuf,
    /// Whether this call created it or it was already there, byte for byte.
    pub outcome: Published,
}

/// Why a store operation failed.
#[derive(Debug, Error)]
pub enum StoreError {
    /// The package could not be written.
    #[error(transparent)]
    Write(#[from] WriteError),
    /// The stored object is not a valid package.
    #[error("stored package {path} was refused: {source}")]
    Read {
        /// The object.
        path: PathBuf,
        /// The reader's refusal.
        source: ReadError,
    },
    /// The attempt is in conflict: it was published with different content.
    #[error("attempt {attempt} of candidate {candidate} is in conflict")]
    Conflict {
        /// The candidate.
        candidate: String,
        /// The attempt.
        attempt: String,
    },
    /// The stored object names a different candidate or attempt than its path.
    #[error("stored package {path} is not bound to the candidate and attempt of its path")]
    Unbound {
        /// The object.
        path: PathBuf,
    },
    /// The stored object is not a regular file within the size bound.
    #[error("stored object {path} is not a regular file within the size bound")]
    NotRegular {
        /// The object.
        path: PathBuf,
    },
    /// The store root is not a directory.
    #[error("store root {path} is not a directory")]
    RootInvalid {
        /// The root.
        path: PathBuf,
    },
    /// A filesystem step failed.
    #[error("store I/O failed for {path}: {source}")]
    Io {
        /// The path acted on.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// The atomic publisher failed for a reason other than a collision. The
    /// object may already be committed when the step is a cleanup or flush.
    #[error("atomic publication failed: {0}")]
    Publish(#[source] PublishError<Occupied>),
}

fn object_path(root: &Path, candidate: &CandidateId, attempt: &AttemptId) -> PathBuf {
    root.join(candidate.as_str())
        .join(format!("{}.json", attempt.as_str()))
}

fn marker_path(root: &Path, candidate: &CandidateId, attempt: &AttemptId) -> PathBuf {
    root.join(candidate.as_str())
        .join(format!("{}.conflict", attempt.as_str()))
}

fn conflict(candidate: &CandidateId, attempt: &AttemptId) -> StoreError {
    StoreError::Conflict {
        candidate: candidate.as_str().to_owned(),
        attempt: attempt.as_str().to_owned(),
    }
}

fn exists(path: &Path) -> Result<bool, StoreError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(StoreError::Io {
            path: path.to_owned(),
            source,
        }),
    }
}

/// Publishes `package` under `root`, bound to its candidate and attempt.
///
/// `root` must be an existing directory; the candidate directory is created.
///
/// # Errors
///
/// [`StoreError::Write`] when the package is invalid (nothing is written);
/// [`StoreError::Conflict`] when the attempt already exists with different
/// bytes or is marked in conflict; [`StoreError::Read`] or
/// [`StoreError::Unbound`] if the committed object does not verify;
/// [`StoreError::Io`] or [`StoreError::Publish`] for filesystem failures.
pub fn publish_package(
    root: &Path,
    package: &IntegratorPackage,
) -> Result<PublishedPackage, StoreError> {
    let bytes = write_package(package)?;
    let root_metadata = fs::metadata(root).map_err(|source| StoreError::Io {
        path: root.to_owned(),
        source,
    })?;
    if !root_metadata.is_dir() {
        return Err(StoreError::RootInvalid {
            path: root.to_owned(),
        });
    }
    let (candidate, attempt) = (&package.candidate, &package.attempt);
    let directory = root.join(candidate.as_str());
    fs::create_dir_all(&directory).map_err(|source| StoreError::Io {
        path: directory.clone(),
        source,
    })?;
    let (path, marker) = (
        object_path(root, candidate, attempt),
        marker_path(root, candidate, attempt),
    );
    if exists(&marker)? {
        return Err(conflict(candidate, attempt));
    }
    match write_file_atomic_no_replace(&path, &bytes, Occupied) {
        Ok(outcome) => {
            let stored = read_published(root, candidate, attempt)?;
            if stored != *package {
                return Err(StoreError::Unbound { path });
            }
            Ok(PublishedPackage { path, outcome })
        }
        Err(PublishError::Collision { .. }) => {
            // Only a valid package of this attempt with different content is a
            // conflict. A link, directory or corrupt object is reported as
            // what it is and marks nothing.
            read_object(&path, candidate, attempt)?;
            record_conflict(&marker)?;
            Err(conflict(candidate, attempt))
        }
        Err(error) => Err(StoreError::Publish(error)),
    }
}

fn record_conflict(marker: &Path) -> Result<(), StoreError> {
    match write_file_atomic_no_replace(marker, CONFLICT_MARKER, Occupied) {
        // A collision means another refused writer recorded it first, or
        // something else holds the name; the attempt reads as a conflict
        // either way.
        Ok(_) | Err(PublishError::Collision { .. }) => Ok(()),
        Err(error) => Err(StoreError::Publish(error)),
    }
}

/// Reads the package published for `candidate` and `attempt`.
///
/// # Errors
///
/// [`StoreError::Conflict`] when the attempt is marked in conflict;
/// [`StoreError::NotRegular`] for a link, directory or oversize object;
/// [`StoreError::Read`] when the strict reader refuses the bytes;
/// [`StoreError::Unbound`] when the object is not bound to its path;
/// [`StoreError::Io`] otherwise.
pub fn read_published(
    root: &Path,
    candidate: &CandidateId,
    attempt: &AttemptId,
) -> Result<IntegratorPackage, StoreError> {
    if exists(&marker_path(root, candidate, attempt))? {
        return Err(conflict(candidate, attempt));
    }
    read_object(&object_path(root, candidate, attempt), candidate, attempt)
}

/// Opens the object without following a link, checks the opened file, and
/// reads it strictly.
fn read_object(
    path: &Path,
    candidate: &CandidateId,
    attempt: &AttemptId,
) -> Result<IntegratorPackage, StoreError> {
    let io_error = |source| StoreError::Io {
        path: path.to_owned(),
        source,
    };
    let not_regular = || StoreError::NotRegular {
        path: path.to_owned(),
    };
    // A link or directory is named before it is opened; the opened file is
    // checked again below, so a swap between the two cannot be followed.
    if !fs::symlink_metadata(path).map_err(io_error)?.is_file() {
        return Err(not_regular());
    }
    let file = open_regular(path).map_err(io_error)?;
    let metadata = file.metadata().map_err(io_error)?;
    let within_bound = usize::try_from(metadata.len()).is_ok_and(|size| size <= MAX_PACKAGE_BYTES);
    if !metadata.is_file() || !within_bound {
        return Err(not_regular());
    }
    let mut bytes = Vec::new();
    file.take(MAX_PACKAGE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    let package = read_package(&bytes).map_err(|source| StoreError::Read {
        path: path.to_owned(),
        source,
    })?;
    if package.candidate != *candidate || package.attempt != *attempt {
        return Err(StoreError::Unbound {
            path: path.to_owned(),
        });
    }
    Ok(package)
}

#[cfg(unix)]
fn open_regular(path: &Path) -> io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let flags = rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK;
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(i32::try_from(flags.bits()).unwrap_or(0))
        .open(path)
}

#[cfg(not(unix))]
fn open_regular(path: &Path) -> io::Result<fs::File> {
    fs::File::open(path)
}
