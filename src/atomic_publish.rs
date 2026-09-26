// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Publish a file without ever replacing an existing one (FR-027).
//!
//! The bytes go to a sidecar created exclusively beside the destination and
//! are committed with `link(2)`, which fails with `EEXIST` rather than
//! replacing. `rename` is deliberately not used: it replaces silently. A
//! destination that already holds identical bytes is success; anything else is
//! the caller's typed collision. The sidecar is removed on every path.
//!
//! This is a filesystem module (like `producer_execution`), exempt from the
//! FR-014-AC-4 I/O-freedom audit.

use std::{
    fs,
    io::{self, Read as _, Write as _},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use rustix::fs::{Mode, OFlags};
use thiserror::Error;

/// The filesystem step that failed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublishStep {
    /// Creating the exclusive sidecar.
    CreateSidecar,
    /// Writing the bytes into the sidecar.
    WriteSidecar,
    /// Committing the sidecar with a hard link.
    Link,
    /// Inspecting or reading the destination that already exists.
    ReadDestination,
    /// Removing the sidecar.
    RemoveSidecar,
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

/// Writes `bytes` to `path` unless a different file is already there.
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
) -> Result<(), PublishError<C>> {
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
) -> Result<(), PublishError<C>> {
    let outcome = match fs::hard_link(tmp, path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            existing_matches(path, bytes, collision)
        }
        Err(source) => Err(PublishError::Io {
            step: PublishStep::Link,
            path: path.to_owned(),
            source,
        }),
    };
    match (fs::remove_file(tmp), outcome) {
        (_, Err(error)) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
        (Err(source), Ok(())) => Err(PublishError::Io {
            step: PublishStep::RemoveSidecar,
            path: tmp.to_owned(),
            source,
        }),
    }
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
    // One O_NOFOLLOW descriptor supplies both the type check and the bytes, so
    // a destination swapped for a symlink cannot be read as identical.
    let opened = rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    );
    let Ok(fd) = opened else {
        return Err(PublishError::Collision {
            path: path.to_owned(),
            collision,
        });
    };
    let mut file = fs::File::from(fd);
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

#[cfg(test)]
mod tests {
    use std::fs;

    use ix_trace_rs::trace;

    use super::{PublishError, publish_file_no_replace, write_file_atomic_no_replace};

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
        write_file_atomic_no_replace(&path, b"one", Taken).expect("identical is idempotent");

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
        publish_file_no_replace(&tmp, &path, b"mine", Taken).expect("idempotent");
        assert_eq!(entries(dir.path()), ["out"]);

        fs::write(&tmp, b"mine").expect("sidecar");
        fs::write(&path, b"theirs").expect("racing writer, other bytes");
        let error = publish_file_no_replace(&tmp, &path, b"mine", Taken).expect_err("collision");
        assert!(matches!(error, PublishError::Collision { .. }));
        assert_eq!(fs::read(&path).expect("read"), b"theirs");
        assert_eq!(entries(dir.path()), ["out"]);
    }
}
