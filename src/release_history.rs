// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Request rules and the record path grammar of `release-history` (FR-052).
//!
//! The module is pure: it decides which strings are acceptable inputs and which
//! git paths name a change-assurance record. History walking stays in the
//! binary's git adapter.

/// Most bytes a `record_id` path segment may have.
pub const MAX_RECORD_ID_BYTES: usize = 256;

/// Whether `value` is a full lowercase hexadecimal commit hash of 40 or 64 digits.
#[must_use]
pub fn is_commit_hash(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// Whether `dir` is an acceptable `records_dir`: a non-empty, repository-relative
/// directory with no empty, `.` or `..` segment, no glob character and no
/// leading `-`. One trailing `/` is allowed.
#[must_use]
pub fn is_records_dir(dir: &str) -> bool {
    let dir = dir.strip_suffix('/').unwrap_or(dir);
    !dir.starts_with('-')
        && !dir.is_empty()
        && !dir.chars().any(|c| c.is_control() || is_glob(c))
        && dir
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | ".."))
}

fn is_glob(c: char) -> bool {
    matches!(c, '*' | '?' | '[' | ']' | '{' | '}' | '\\')
}

/// Whether `name` is usable as a tag name under `refs/tags/`: no revision
/// syntax, so that `refs/tags/<name>` can only name a ref.
#[must_use]
pub fn is_tag_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(['-', '/'])
        && !name.ends_with(['/', '.'])
        && !name.contains("..")
        && !name.contains("@{")
        && !name.contains("//")
        && !name
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "*?[\\~^:".contains(c))
}

/// What a git path is, relative to `records_dir`.
#[derive(Debug, Eq, PartialEq)]
pub enum RecordPath<'a> {
    /// Not `<records_dir>/<record_id>/<revision>.json`: ignored.
    NotARecord,
    /// A record, with its `record_id` and its `revision`.
    Record(&'a str, u64),
    /// Fits the grammar but cannot be reported: the revision exceeds `u64::MAX`
    /// or the `record_id` is not UTF-8.
    Unrepresentable,
}

/// Classifies the git path `path` under `records_dir` by the exact grammar of FR-052.
#[must_use]
pub fn record_of_path<'a>(records_dir: &str, path: &'a [u8]) -> RecordPath<'a> {
    let dir = records_dir.strip_suffix('/').unwrap_or(records_dir);
    let Some(rest) = path
        .strip_prefix(dir.as_bytes())
        .and_then(|rest| rest.strip_prefix(b"/"))
    else {
        return RecordPath::NotARecord;
    };
    let Some(slash) = rest.iter().position(|byte| *byte == b'/') else {
        return RecordPath::NotARecord;
    };
    let (record_id, file) = (&rest[..slash], &rest[slash + 1..]);
    if record_id.is_empty()
        || record_id.len() > MAX_RECORD_ID_BYTES
        || matches!(record_id, b"." | b"..")
    {
        return RecordPath::NotARecord;
    }
    let Some(digits) = file.strip_suffix(b".json") else {
        return RecordPath::NotARecord;
    };
    if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) || digits[0] == b'0' {
        return RecordPath::NotARecord;
    }
    let revision = std::str::from_utf8(digits)
        .ok()
        .and_then(|digits| digits.parse::<u64>().ok());
    match (std::str::from_utf8(record_id), revision) {
        (Ok(record_id), Some(revision)) => RecordPath::Record(record_id, revision),
        _ => RecordPath::Unrepresentable,
    }
}
