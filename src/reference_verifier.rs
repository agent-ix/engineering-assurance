// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Re-verify declared file references against the bytes now on disk (FR-027).
//!
//! Each declared `{path, size_bytes, digest}` is re-resolved beneath a
//! capability root with `openat2(BENEATH | NO_SYMLINKS)`, so no path component
//! may be a symbolic link or leave the root, then re-digested from that one
//! descriptor (size and digest describe the same file). Only Linux is supported. Every mismatch is
//! reported, not just the first, as a finding at a JSON Pointer such as
//! `/raw_evidence/3/digest`.
//!
//! This is a filesystem module (like `producer_execution`), exempt from the
//! FR-014-AC-4 I/O-freedom audit.

use std::{
    fmt,
    fs::File,
    path::{Component, Path},
};

use rustix::{
    fs::{Mode, OFlags, ResolveFlags, openat2},
    io::Errno,
};

use crate::content_digest::{ContentDigest, DigestError, hash_reader};

/// One declared file reference to re-verify.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeclaredReference {
    /// Path relative to the capability root.
    pub path: String,
    /// Declared length in bytes.
    pub size_bytes: u64,
    /// Declared content digest.
    pub digest: ContentDigest,
}

/// What was wrong with one declared reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FindingKind {
    /// The path is absolute or contains a parent or root component.
    PathNotRelative,
    /// The file is absent, linked, not regular, too large or unreadable.
    Unreadable {
        /// The refusal from [`ContentDigest::of_file`].
        error: DigestError,
    },
    /// The file's length differs from the declared `size_bytes`.
    SizeMismatch {
        /// Declared length.
        expected: u64,
        /// Observed length.
        observed: u64,
    },
    /// The file's digest differs from the declared `digest`.
    DigestMismatch {
        /// Declared digest.
        expected: ContentDigest,
        /// Observed digest.
        observed: ContentDigest,
    },
}

/// One mismatch, located by JSON Pointer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceFinding {
    /// Pointer to the offending field, e.g. `/raw_evidence/3/digest`.
    pub pointer: String,
    /// What was wrong.
    pub kind: FindingKind,
}

impl fmt::Display for ReferenceFinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pointer = &self.pointer;
        match &self.kind {
            FindingKind::PathNotRelative => write!(formatter, "{pointer}: path is not relative"),
            FindingKind::Unreadable { error } => write!(formatter, "{pointer}: {error}"),
            FindingKind::SizeMismatch { expected, observed } => {
                write!(
                    formatter,
                    "{pointer}: expected {expected}, observed {observed}"
                )
            }
            FindingKind::DigestMismatch { expected, observed } => write!(
                formatter,
                "{pointer}: expected {}, observed {}",
                expected.as_str(),
                observed.as_str()
            ),
        }
    }
}

/// Opens `relative` beneath `root` without following any link, and returns the
/// digest and length of the regular file's bytes read from that descriptor.
fn read_beneath(root: &File, relative: &Path) -> Result<(ContentDigest, u64), DigestError> {
    let fd = openat2(
        root,
        relative,
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_MAGICLINKS | ResolveFlags::NO_SYMLINKS,
    )
    .map_err(|errno| match errno {
        Errno::NOENT => DigestError::Unavailable,
        Errno::LOOP | Errno::XDEV => DigestError::NotRegular,
        _ => DigestError::Unreadable,
    })?;
    let mut file = File::from(fd);
    let metadata = file.metadata().map_err(|_| DigestError::Unreadable)?;
    if !metadata.is_file() {
        return Err(DigestError::NotRegular);
    }
    let (digest, total) = hash_reader(&mut file, metadata.len(), || false)?;
    Ok((digest, total))
}

/// Verifies every reference beneath `root` and returns every finding.
///
/// `pointer_prefix` names the collection in the caller's document (for example
/// `/raw_evidence`); findings are located at `{prefix}/{index}/{field}`. An
/// empty result means every reference matched.
#[must_use]
pub fn verify_references(
    root: &Path,
    pointer_prefix: &str,
    references: &[DeclaredReference],
) -> Vec<ReferenceFinding> {
    let mut findings = Vec::new();
    let Ok(root_dir) = File::open(root) else {
        // No reference can be resolved without the root: report each one.
        return references
            .iter()
            .enumerate()
            .map(|(index, _)| ReferenceFinding {
                pointer: format!("{pointer_prefix}/{index}/path"),
                kind: FindingKind::Unreadable {
                    error: DigestError::Unavailable,
                },
            })
            .collect();
    };
    for (index, reference) in references.iter().enumerate() {
        let at = |field: &str| format!("{pointer_prefix}/{index}/{field}");
        let relative = Path::new(&reference.path);
        if !relative
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
        {
            findings.push(ReferenceFinding {
                pointer: at("path"),
                kind: FindingKind::PathNotRelative,
            });
            continue;
        }
        let (observed, length) = match read_beneath(&root_dir, relative) {
            Ok(read) => read,
            Err(error) => {
                findings.push(ReferenceFinding {
                    pointer: at("path"),
                    kind: FindingKind::Unreadable { error },
                });
                continue;
            }
        };
        if length != reference.size_bytes {
            findings.push(ReferenceFinding {
                pointer: at("size_bytes"),
                kind: FindingKind::SizeMismatch {
                    expected: reference.size_bytes,
                    observed: length,
                },
            });
        }
        if observed != reference.digest {
            findings.push(ReferenceFinding {
                pointer: at("digest"),
                kind: FindingKind::DigestMismatch {
                    expected: reference.digest.clone(),
                    observed,
                },
            });
        }
    }
    findings
}

#[cfg(test)]
mod tests {
    use std::fs;

    use ix_trace_rs::trace;

    use super::{DeclaredReference, FindingKind, verify_references};
    use crate::content_digest::{ContentDigest, DigestError};

    fn declared(path: &str, bytes: &[u8]) -> DeclaredReference {
        DeclaredReference {
            path: path.to_owned(),
            size_bytes: u64::try_from(bytes.len()).expect("length"),
            digest: ContentDigest::of_bytes(bytes),
        }
    }

    #[trace("TC-208", "FR-027-AC-3")]
    #[test]
    fn tc_208_every_mismatch_is_reported_at_its_own_pointer() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("good"), b"good").expect("write");
        fs::write(dir.path().join("changed"), b"XXXXX").expect("write");
        fs::write(dir.path().join("same-size"), b"abcd").expect("write");
        fs::create_dir(dir.path().join("dir")).expect("mkdir");
        std::os::unix::fs::symlink(dir.path().join("good"), dir.path().join("link")).expect("link");
        // An intermediate directory symlink must not be followed out of the root.
        let outside = tempfile::tempdir().expect("outside");
        fs::write(outside.path().join("secret"), b"secret").expect("write");
        std::os::unix::fs::symlink(outside.path(), dir.path().join("sub")).expect("dir link");

        let references = [
            declared("good", b"good"),
            declared("changed", b"abc"),
            declared("same-size", b"wxyz"),
            declared("missing", b"m"),
            declared("link", b"good"),
            declared("dir", b"d"),
            declared("../escape", b"e"),
            declared("/abs", b"a"),
            declared("sub/secret", b"secret"),
        ];
        let findings = verify_references(dir.path(), "/raw_evidence", &references);
        let summary: Vec<(&str, &FindingKind)> = findings
            .iter()
            .map(|finding| (finding.pointer.as_str(), &finding.kind))
            .collect();

        assert_eq!(summary.len(), 9, "{findings:?}");
        assert!(matches!(
            summary[0],
            (
                "/raw_evidence/1/size_bytes",
                FindingKind::SizeMismatch {
                    expected: 3,
                    observed: 5
                }
            )
        ));
        assert!(matches!(
            summary[1],
            ("/raw_evidence/1/digest", FindingKind::DigestMismatch { .. })
        ));
        assert!(matches!(
            summary[2],
            ("/raw_evidence/2/digest", FindingKind::DigestMismatch { .. })
        ));
        assert!(matches!(
            summary[3],
            (
                "/raw_evidence/3/path",
                FindingKind::Unreadable {
                    error: DigestError::Unavailable
                }
            )
        ));
        assert!(matches!(
            summary[4],
            (
                "/raw_evidence/4/path",
                FindingKind::Unreadable {
                    error: DigestError::NotRegular
                }
            )
        ));
        assert!(matches!(
            summary[5],
            (
                "/raw_evidence/5/path",
                FindingKind::Unreadable {
                    error: DigestError::NotRegular
                }
            )
        ));
        assert!(matches!(
            summary[6],
            ("/raw_evidence/6/path", FindingKind::PathNotRelative)
        ));
        assert!(matches!(
            summary[7],
            ("/raw_evidence/7/path", FindingKind::PathNotRelative)
        ));
    }

    #[trace("TC-208", "FR-027-AC-3")]
    #[test]
    fn tc_208_matching_references_report_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::create_dir(dir.path().join("sub")).expect("mkdir");
        fs::write(dir.path().join("sub/a"), b"alpha").expect("write");
        fs::write(dir.path().join("b"), b"").expect("write");
        let references = [declared("sub/a", b"alpha"), declared("b", b"")];
        assert!(verify_references(dir.path(), "/raw_evidence", &references).is_empty());
    }
}
