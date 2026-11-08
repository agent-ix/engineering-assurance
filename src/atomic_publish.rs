// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Publish a file without ever replacing an existing one (FR-027).
//!
//! The bytes go to a sidecar created exclusively beside the destination and
//! are committed with `link(2)`, which fails with `EEXIST` rather than
//! replacing. `rename` is deliberately not used: it replaces silently. A
//! destination that already holds identical bytes is success
//! ([`Published::AlreadyPresent`]); anything else is the caller's typed
//! collision. The sidecar is removed on every path.
//!
//! Durability: the sidecar is `fsync`ed before it is linked, and the parent
//! directory is `fsync`ed after the name is in place, also when the name was
//! already present, so success means the name survives a crash. Windows has no
//! directory `fsync`; there the directory step is a no-op and the hard link
//! (`CreateHardLinkW`) gives the same no-replace semantics.
//!
//! This is a filesystem module (like `producer_execution`), exempt from the
//! FR-014-AC-4 I/O-freedom audit.

use std::{
    fs,
    io::{self, Read as _, Write as _},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use rustix::fs::{Mode, OFlags};
use thiserror::Error;

/// The outcome of a successful publish.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Published {
    /// This call created the destination with the given bytes.
    Written,
    /// The destination already held exactly the given bytes and was left
    /// untouched.
    AlreadyPresent,
}

/// The filesystem step that failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublishStep {
    /// Creating the exclusive sidecar.
    CreateSidecar,
    /// Writing the bytes into the sidecar.
    WriteSidecar,
    /// Flushing the sidecar to stable storage.
    SyncSidecar,
    /// Committing the sidecar with a hard link.
    Link,
    /// Inspecting or reading the destination that already exists.
    ReadDestination,
    /// Removing the sidecar.
    RemoveSidecar,
    /// Flushing the parent directory to stable storage.
    SyncDirectory,
}

/// Why a publish did not complete. `C` is the caller's collision value.
#[derive(Debug, Error)]
pub enum PublishError<C: std::fmt::Debug> {
    /// The destination exists and does not hold exactly the published bytes.
    #[error("destination {path} already exists with different content")]
    Collision {
        /// The occupied destination.
        path: PathBuf,
        /// The caller-supplied collision value.
        collision: C,
    },
    /// The destination path has no file name to derive a sidecar from.
    #[error("destination {path} has no file name")]
    NoFileName {
        /// The offending destination.
        path: PathBuf,
    },
    /// A filesystem step failed.
    #[error("{step:?} failed for {path}: {source}")]
    Io {
        /// The step that failed.
        step: PublishStep,
        /// The path the step acted on.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
}

const SIDECAR_ATTEMPTS: u32 = 8;
static SIDECAR_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Writes `bytes` to `path` unless a different file is already there, durably.
///
/// Returns [`Published::Written`] when this call created the file and
/// [`Published::AlreadyPresent`] when it already held exactly `bytes`.
///
/// # Errors
///
/// [`PublishError::Collision`] when the destination exists with different
/// bytes or is not a regular file; [`PublishError::Io`] for any filesystem
/// failure; [`PublishError::NoFileName`] for a destination such as `/`.
pub fn write_file_atomic_no_replace<C: std::fmt::Debug>(
    path: &Path,
    bytes: &[u8],
    collision: C,
) -> Result<Published, PublishError<C>> {
    let file_name = path.file_name().ok_or_else(|| PublishError::NoFileName {
        path: path.to_owned(),
    })?;
    let (tmp, mut file) = create_sidecar(path, file_name)?;
    if let Err(error) = file.write_all(bytes) {
        drop(file);
        let _ = fs::remove_file(&tmp);
        return Err(PublishError::Io {
            step: PublishStep::WriteSidecar,
            path: tmp,
            source: error,
        });
    }
    if let Err(source) = file.sync_all() {
        drop(file);
        let _ = fs::remove_file(&tmp);
        return Err(PublishError::Io {
            step: PublishStep::SyncSidecar,
            path: tmp,
            source,
        });
    }
    drop(file);
    publish_file_no_replace(&tmp, path, bytes, collision)
}

/// Creates a uniquely named sidecar exclusively, retrying a bounded number of
/// times when the name is taken (a stale sidecar, or another process sharing
/// this directory and process id).
fn create_sidecar<C: std::fmt::Debug>(
    path: &Path,
    file_name: &std::ffi::OsStr,
) -> Result<(PathBuf, fs::File), PublishError<C>> {
    let mut last = None;
    for _ in 0..SIDECAR_ATTEMPTS {
        let counter = SIDECAR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut name = std::ffi::OsString::from(format!(".tmp-{}-{counter}-", std::process::id()));
        name.push(file_name);
        let tmp = path.with_file_name(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
        {
            Ok(file) => return Ok((tmp, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => last = Some((tmp, error)),
            Err(source) => {
                return Err(PublishError::Io {
                    step: PublishStep::CreateSidecar,
                    path: tmp,
                    source,
                });
            }
        }
    }
    let (path, source) = last.expect("SIDECAR_ATTEMPTS is non-zero");
    Err(PublishError::Io {
        step: PublishStep::CreateSidecar,
        path,
        source,
    })
}

/// Commits the already-written sidecar `tmp` to `path` and removes `tmp`.
///
/// A separate step so a destination that appears before the commit can be
/// tested without threads or sleeps; private because it trusts that `tmp` is a
/// distinct regular file holding exactly `bytes`. If the sidecar cannot be
/// removed after a successful link, the publish still happened, but the error
/// is reported and a retry succeeds as identical content.
fn publish_file_no_replace<C: std::fmt::Debug>(
    tmp: &Path,
    path: &Path,
    bytes: &[u8],
    collision: C,
) -> Result<Published, PublishError<C>> {
    let outcome = match fs::hard_link(tmp, path) {
        Ok(()) => Ok(Published::Written),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            existing_matches(path, bytes, collision).map(|()| Published::AlreadyPresent)
        }
        Err(source) => Err(PublishError::Io {
            step: PublishStep::Link,
            path: path.to_owned(),
            source,
        }),
    };
    let removed = fs::remove_file(tmp);
    let published = outcome?;
    removed.map_err(|source| PublishError::Io {
        step: PublishStep::RemoveSidecar,
        path: tmp.to_owned(),
        source,
    })?;
    // Also on `AlreadyPresent`: the writer that created the name may have
    // crashed before its own directory fsync.
    sync_parent(path)?;
    Ok(published)
}

/// Flushes the directory entry for `path` to stable storage.
#[cfg(unix)]
fn sync_parent<C: std::fmt::Debug>(path: &Path) -> Result<(), PublishError<C>> {
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| PublishError::Io {
            step: PublishStep::SyncDirectory,
            path: parent.to_owned(),
            source,
        })
}

/// Windows cannot open a directory as a file to flush it; NTFS journals the
/// link, so there is nothing further to do.
#[cfg(not(unix))]
fn sync_parent<C: std::fmt::Debug>(_path: &Path) -> Result<(), PublishError<C>> {
    Ok(())
}

fn existing_matches<C: std::fmt::Debug>(
    path: &Path,
    bytes: &[u8],
    collision: C,
) -> Result<(), PublishError<C>> {
    let read_error = |source| PublishError::Io {
        step: PublishStep::ReadDestination,
        path: path.to_owned(),
        source,
    };
    let Some(mut file) = open_regular_nofollow(path) else {
        return Err(PublishError::Collision {
            path: path.to_owned(),
            collision,
        });
    };
    let metadata = file.metadata().map_err(read_error)?;
    let mut observed = Vec::new();
    let identical = metadata.is_file()
        && u64::try_from(bytes.len()).is_ok_and(|length| length == metadata.len())
        && file.read_to_end(&mut observed).map_err(read_error)? == bytes.len()
        && observed == bytes;
    if identical {
        Ok(())
    } else {
        Err(PublishError::Collision {
            path: path.to_owned(),
            collision,
        })
    }
}

/// One `O_NOFOLLOW` descriptor supplies both the type check and the bytes, so
/// a destination swapped for a symlink cannot be read as identical.
#[cfg(unix)]
fn open_regular_nofollow(path: &Path) -> Option<fs::File> {
    rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .ok()
    .map(fs::File::from)
}

/// Windows has no `O_NOFOLLOW`: refuse any link or reparse point by its own
/// metadata, then open and let the caller re-check the opened file is regular.
#[cfg(not(unix))]
fn open_regular_nofollow(path: &Path) -> Option<fs::File> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    fs::File::open(path).ok()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use ix_trace_rs::trace;

    use super::{PublishError, Published, publish_file_no_replace, write_file_atomic_no_replace};

    #[derive(Debug, PartialEq)]
    struct Taken;

    fn entries(dir: &std::path::Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .expect("read dir")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        names.sort();
        names
    }

    #[trace("TC-206", "FR-027-AC-1")]
    #[test]
    fn tc_206_publish_creates_exact_bytes_and_leaves_no_sidecar() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("out.json");
        write_file_atomic_no_replace(&path, b"{\"a\":1}", Taken).expect("publish");
        assert_eq!(fs::read(&path).expect("read"), b"{\"a\":1}");
        assert_eq!(entries(dir.path()), ["out.json"]);
    }

    #[trace("TC-207", "FR-027-AC-2")]
    #[test]
    fn tc_207_identical_succeeds_and_different_collides_leaving_destination_untouched() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("out");
        write_file_atomic_no_replace(&path, b"one", Taken).expect("first");
        let again =
            write_file_atomic_no_replace(&path, b"one", Taken).expect("identical is idempotent");
        assert_eq!(again, Published::AlreadyPresent);

        // Same length, different bytes: exercises the content comparison.
        let error = write_file_atomic_no_replace(&path, b"two", Taken).expect_err("collision");
        assert!(matches!(
            error,
            PublishError::Collision {
                collision: Taken,
                ..
            }
        ));
        assert_eq!(fs::read(&path).expect("read"), b"one");
        assert_eq!(entries(dir.path()), ["out"]);
    }

    #[cfg(unix)]
    #[trace("TC-207", "FR-027-AC-2")]
    #[test]
    fn tc_207_directory_and_symlink_destinations_collide() {
        let dir = tempfile::tempdir().expect("tempdir");
        let occupied_dir = dir.path().join("d");
        fs::create_dir(&occupied_dir).expect("mkdir");
        let target = dir.path().join("target");
        fs::write(&target, b"x").expect("write");
        let link = dir.path().join("l");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");

        for path in [&occupied_dir, &link] {
            let error = write_file_atomic_no_replace(path, b"x", Taken).expect_err("collision");
            assert!(matches!(error, PublishError::Collision { .. }), "{path:?}");
        }
        assert_eq!(entries(dir.path()), ["d", "l", "target"]);
    }

    #[trace("TC-207", "FR-027-AC-2")]
    #[test]
    fn tc_207_destination_appearing_before_commit_is_handled_and_sidecar_removed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("out");
        let tmp = dir.path().join(".tmp-test");

        fs::write(&tmp, b"mine").expect("sidecar");
        fs::write(&path, b"mine").expect("racing writer, same bytes");
        let outcome = publish_file_no_replace(&tmp, &path, b"mine", Taken).expect("idempotent");
        assert_eq!(outcome, Published::AlreadyPresent);
        assert_eq!(entries(dir.path()), ["out"]);

        fs::write(&tmp, b"mine").expect("sidecar");
        fs::write(&path, b"theirs").expect("racing writer, other bytes");
        let error = publish_file_no_replace(&tmp, &path, b"mine", Taken).expect_err("collision");
        assert!(matches!(error, PublishError::Collision { .. }));
        assert_eq!(fs::read(&path).expect("read"), b"theirs");
        assert_eq!(entries(dir.path()), ["out"]);
    }

    #[trace("TC-211", "FR-027-AC-4")]
    #[test]
    fn tc_211_outcome_distinguishes_written_from_already_present() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("out");
        assert_eq!(
            write_file_atomic_no_replace(&path, b"bytes", Taken).expect("first"),
            Published::Written
        );
        assert_eq!(
            write_file_atomic_no_replace(&path, b"bytes", Taken).expect("second"),
            Published::AlreadyPresent
        );
        // The pre-commit race path reports the same outcomes.
        let tmp = dir.path().join(".tmp-test");
        fs::write(&tmp, b"fresh").expect("sidecar");
        let other = dir.path().join("other");
        assert_eq!(
            publish_file_no_replace(&tmp, &other, b"fresh", Taken).expect("commit"),
            Published::Written
        );
        assert_eq!(fs::read(&other).expect("read"), b"fresh");
        assert_eq!(entries(dir.path()), ["other", "out"]);
    }

    #[trace("TC-212", "FR-027-AC-5")]
    #[test]
    fn tc_212_durable_publish_succeeds_for_bare_relative_and_nested_destinations() {
        // The sidecar and parent-directory flushes run on every success path,
        // including AlreadyPresent; a flush failure would surface as `Io`.
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("a").join("b");
        fs::create_dir_all(&nested).expect("mkdir");
        let path = nested.join("out");
        for expected in [Published::Written, Published::AlreadyPresent] {
            let outcome = write_file_atomic_no_replace(&path, &vec![7_u8; 65_536], Taken)
                .expect("durable publish");
            assert_eq!(outcome, expected);
        }
        assert_eq!(fs::read(&path).expect("read").len(), 65_536);
        assert_eq!(entries(&nested), ["out"]);
    }

    #[trace("TC-213", "FR-027-AC-2")]
    #[test]
    fn tc_213_concurrent_writers_of_differing_bytes_refuse_rather_than_clobber() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("output.bin");
        let outcomes: Vec<_> = (0_u8..8)
            .map(|writer| {
                let path = path.clone();
                std::thread::spawn(move || {
                    write_file_atomic_no_replace(&path, &vec![writer; 4096], Taken)
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().expect("no writer panicked"))
            .collect();
        let written = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Ok(Published::Written)))
            .count();
        let refused = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Err(PublishError::Collision { .. })))
            .count();
        assert_eq!(written, 1, "exactly one publisher");
        assert_eq!(refused, 7, "the rest are refused");
        let bytes = fs::read(&path).expect("read");
        assert_eq!(bytes.len(), 4096);
        assert!(bytes.iter().all(|byte| *byte == bytes[0]));
        assert_eq!(entries(dir.path()), ["output.bin"]);
    }

    #[trace("TC-213", "FR-027-AC-2")]
    #[test]
    fn tc_213_concurrent_writers_of_identical_bytes_all_succeed_once_written() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("same.bin");
        let outcomes: Vec<_> = (0..8)
            .map(|_| {
                let path = path.clone();
                std::thread::spawn(move || write_file_atomic_no_replace(&path, b"same", Taken))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().expect("no writer panicked").expect("publish"))
            .collect();
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| **outcome == Published::Written)
                .count(),
            1
        );
        assert_eq!(entries(dir.path()), ["same.bin"]);
    }
}
