// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Bounded wheel and npm archive decoding for the package-audit binary.

use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    path::Path,
};

use engineering_assurance::package_membership::is_safe_package_member_path;
use flate2::read::GzDecoder;
use thiserror::Error;

pub(crate) const MAX_ARCHIVE_BYTES: u64 = 67_108_864;
pub(crate) const MAX_ARCHIVE_ENTRIES: usize = 65_536;
pub(crate) const MAX_ARCHIVE_FILE_BYTES: usize = 8_388_608;
pub(crate) const MAX_ARCHIVE_EXPANDED_BYTES: usize = 67_108_864;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArchiveFile {
    pub(crate) path: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ArchiveSnapshot {
    pub(crate) files: Vec<ArchiveFile>,
}

impl ArchiveSnapshot {
    pub(crate) fn names(&self) -> Vec<String> {
        self.files.iter().map(|file| file.path.clone()).collect()
    }

    pub(crate) const fn file_count(&self) -> usize {
        self.files.len()
    }

    pub(crate) fn file(&self, path: &str) -> Option<&[u8]> {
        self.files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.bytes.as_slice())
    }
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(crate) enum ArchiveError {
    #[error("selected package archive is not a bounded regular file")]
    ArchiveInvalid,
    #[error("package archive exceeds {MAX_ARCHIVE_BYTES} bytes")]
    ArchiveTooLarge,
    #[error("package archive cannot be decoded")]
    DecodeFailed,
    #[error("package archive exceeds {MAX_ARCHIVE_ENTRIES} entries")]
    EntryPopulationTooLarge,
    #[error("package archive contains an invalid member path")]
    MemberPathInvalid,
    #[error("package archive contains a duplicate file member")]
    MemberDuplicate,
    #[error("package archive contains an unsupported member kind")]
    MemberKindInvalid,
    #[error("package archive member exceeds {MAX_ARCHIVE_FILE_BYTES} bytes")]
    MemberTooLarge,
    #[error("package archive expands beyond {MAX_ARCHIVE_EXPANDED_BYTES} bytes")]
    ExpandedBytesTooLarge,
}

pub(crate) fn read_wheel(path: &Path) -> Result<ArchiveSnapshot, ArchiveError> {
    let file = archive_file(path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|_| ArchiveError::DecodeFailed)?;
    validate_entry_population(archive.len())?;
    let mut snapshot = SnapshotBuilder::default();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| ArchiveError::DecodeFailed)?;
        let raw_name = entry.name_raw();
        let name = std::str::from_utf8(raw_name).map_err(|_| ArchiveError::MemberPathInvalid)?;
        let directory = entry.is_dir();
        validate_zip_kind(entry.unix_mode(), directory)?;
        if directory {
            validate_directory_name(name, None)?;
            continue;
        }
        let path = validate_file_name(name, None)?;
        let declared = usize::try_from(entry.size()).map_err(|_| ArchiveError::MemberTooLarge)?;
        let bytes = read_member(&mut entry, declared)?;
        snapshot.push(path, bytes)?;
    }
    Ok(snapshot.finish())
}

pub(crate) fn read_npm(path: &Path) -> Result<ArchiveSnapshot, ArchiveError> {
    let file = archive_file(path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let entries = archive.entries().map_err(|_| ArchiveError::DecodeFailed)?;
    let mut snapshot = SnapshotBuilder::default();
    let mut entry_count = 0_usize;
    for entry in entries {
        entry_count = entry_count
            .checked_add(1)
            .ok_or(ArchiveError::EntryPopulationTooLarge)?;
        validate_entry_population(entry_count)?;
        let mut entry = entry.map_err(|_| ArchiveError::DecodeFailed)?;
        let raw_name = entry.path_bytes();
        let name = std::str::from_utf8(&raw_name).map_err(|_| ArchiveError::MemberPathInvalid)?;
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            validate_directory_name(name, Some("package/"))?;
            continue;
        }
        if !kind.is_file() {
            return Err(ArchiveError::MemberKindInvalid);
        }
        let path = validate_file_name(name, Some("package/"))?;
        let declared = usize::try_from(entry.size()).map_err(|_| ArchiveError::MemberTooLarge)?;
        let bytes = read_member(&mut entry, declared)?;
        snapshot.push(path, bytes)?;
    }
    Ok(snapshot.finish())
}

fn archive_file(path: &Path) -> Result<File, ArchiveError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ArchiveError::ArchiveInvalid)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ArchiveError::ArchiveInvalid);
    }
    if metadata.len() > MAX_ARCHIVE_BYTES {
        return Err(ArchiveError::ArchiveTooLarge);
    }
    File::open(path).map_err(|_| ArchiveError::ArchiveInvalid)
}

fn validate_entry_population(entries: usize) -> Result<(), ArchiveError> {
    if entries > MAX_ARCHIVE_ENTRIES {
        return Err(ArchiveError::EntryPopulationTooLarge);
    }
    Ok(())
}

fn validate_zip_kind(mode: Option<u32>, directory: bool) -> Result<(), ArchiveError> {
    let Some(mode) = mode else {
        return Ok(());
    };
    let file_type = mode & 0o170_000;
    let expected = if directory { 0o040_000 } else { 0o100_000 };
    if file_type != 0 && file_type != expected {
        return Err(ArchiveError::MemberKindInvalid);
    }
    Ok(())
}

fn validate_directory_name(name: &str, prefix: Option<&str>) -> Result<(), ArchiveError> {
    let without_slash = name.strip_suffix('/').unwrap_or(name);
    if prefix.is_some_and(|prefix| without_slash == prefix.trim_end_matches('/')) {
        return Ok(());
    }
    let relative = strip_prefix(without_slash, prefix)?;
    if !is_safe_package_member_path(relative) {
        return Err(ArchiveError::MemberPathInvalid);
    }
    Ok(())
}

fn validate_file_name(name: &str, prefix: Option<&str>) -> Result<String, ArchiveError> {
    let relative = strip_prefix(name, prefix)?;
    if !is_safe_package_member_path(relative) {
        return Err(ArchiveError::MemberPathInvalid);
    }
    Ok(relative.to_owned())
}

fn strip_prefix<'a>(name: &'a str, prefix: Option<&str>) -> Result<&'a str, ArchiveError> {
    match prefix {
        Some(prefix) => name
            .strip_prefix(prefix)
            .ok_or(ArchiveError::MemberPathInvalid),
        None => Ok(name),
    }
}

fn read_member(reader: &mut impl Read, declared: usize) -> Result<Vec<u8>, ArchiveError> {
    if declared > MAX_ARCHIVE_FILE_BYTES {
        return Err(ArchiveError::MemberTooLarge);
    }
    let limit = u64::try_from(MAX_ARCHIVE_FILE_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut bytes = Vec::with_capacity(declared.min(MAX_ARCHIVE_FILE_BYTES));
    reader
        .take(limit)
        .read_to_end(&mut bytes)
        .map_err(|_| ArchiveError::DecodeFailed)?;
    if bytes.len() > MAX_ARCHIVE_FILE_BYTES {
        return Err(ArchiveError::MemberTooLarge);
    }
    if bytes.len() != declared {
        return Err(ArchiveError::DecodeFailed);
    }
    Ok(bytes)
}

#[derive(Default)]
struct SnapshotBuilder {
    files: Vec<ArchiveFile>,
    names: BTreeSet<String>,
    expanded_bytes: usize,
}

impl SnapshotBuilder {
    fn push(&mut self, path: String, bytes: Vec<u8>) -> Result<(), ArchiveError> {
        if !self.names.insert(path.clone()) {
            return Err(ArchiveError::MemberDuplicate);
        }
        self.expanded_bytes = self
            .expanded_bytes
            .checked_add(bytes.len())
            .ok_or(ArchiveError::ExpandedBytesTooLarge)?;
        if self.expanded_bytes > MAX_ARCHIVE_EXPANDED_BYTES {
            return Err(ArchiveError::ExpandedBytesTooLarge);
        }
        self.files.push(ArchiveFile { path, bytes });
        Ok(())
    }

    fn finish(mut self) -> ArchiveSnapshot {
        self.files.sort_by(|left, right| left.path.cmp(&right.path));
        ArchiveSnapshot { files: self.files }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use ix_trace_rs::trace;

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn directory_and_file_names_share_the_safe_path_grammar() {
        assert!(validate_directory_name("package/", Some("package/")).is_ok());
        assert!(validate_directory_name("package/safe/", Some("package/")).is_ok());
        assert_eq!(
            validate_directory_name("package/../escape/", Some("package/")),
            Err(ArchiveError::MemberPathInvalid)
        );
        assert_eq!(
            validate_file_name("other/file", Some("package/")),
            Err(ArchiveError::MemberPathInvalid)
        );
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn snapshot_builder_rejects_duplicates_and_expansion_overflow() {
        let mut duplicate = SnapshotBuilder::default();
        duplicate.push("one".to_owned(), Vec::new()).unwrap();
        assert_eq!(
            duplicate.push("one".to_owned(), Vec::new()),
            Err(ArchiveError::MemberDuplicate)
        );

        let mut expanded = SnapshotBuilder {
            expanded_bytes: MAX_ARCHIVE_EXPANDED_BYTES,
            ..SnapshotBuilder::default()
        };
        assert_eq!(
            expanded.push("two".to_owned(), vec![0]),
            Err(ArchiveError::ExpandedBytesTooLarge)
        );
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn archive_file_entry_and_member_byte_boundaries_are_exact() {
        let directory = tempfile::tempdir().unwrap();
        let exact_path = directory.path().join("exact.whl");
        let exact = File::create(&exact_path).unwrap();
        exact.set_len(MAX_ARCHIVE_BYTES).unwrap();
        assert!(archive_file(&exact_path).is_ok());

        let over_path = directory.path().join("over.whl");
        let over = File::create(&over_path).unwrap();
        over.set_len(MAX_ARCHIVE_BYTES + 1).unwrap();
        assert!(matches!(
            archive_file(&over_path),
            Err(ArchiveError::ArchiveTooLarge)
        ));

        assert!(validate_entry_population(MAX_ARCHIVE_ENTRIES).is_ok());
        assert_eq!(
            validate_entry_population(MAX_ARCHIVE_ENTRIES + 1),
            Err(ArchiveError::EntryPopulationTooLarge)
        );

        let exact_bytes = vec![0; MAX_ARCHIVE_FILE_BYTES];
        assert_eq!(
            read_member(&mut Cursor::new(&exact_bytes), MAX_ARCHIVE_FILE_BYTES)
                .unwrap()
                .len(),
            MAX_ARCHIVE_FILE_BYTES
        );
        let over_bytes = vec![0; MAX_ARCHIVE_FILE_BYTES + 1];
        assert_eq!(
            read_member(&mut Cursor::new(&over_bytes), MAX_ARCHIVE_FILE_BYTES + 1),
            Err(ArchiveError::MemberTooLarge)
        );
    }

    #[test]
    #[trace("TC-111", "FR-017-AC-3", "FR-017-CON-3")]
    fn decoded_wheel_rejects_unsafe_file_members() {
        use std::io::Write;
        use zip::{ZipWriter, write::SimpleFileOptions};

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("unsafe.whl");
        let file = File::create(&path).unwrap();
        let mut writer = ZipWriter::new(file);
        writer
            .start_file("../unsafe.txt", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"one").unwrap();
        writer.finish().unwrap();
        assert!(matches!(
            read_wheel(&path),
            Err(ArchiveError::MemberPathInvalid)
        ));
    }
}
