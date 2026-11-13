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
            .any(|c| c.is_control() || c.is_whitespace() || is_glob(c) || "~^:".contains(c))
}

/// The `(record_id, revision)` of the git path `path` under `records_dir`, or
/// `None` when the path is not `<records_dir>/<record_id>/<revision>.json` with
/// the exact grammar of FR-052.
#[must_use]
pub fn record_of_path<'a>(records_dir: &str, path: &'a [u8]) -> Option<(&'a str, u64)> {
    let dir = records_dir.strip_suffix('/').unwrap_or(records_dir);
    let rest = path.strip_prefix(dir.as_bytes())?.strip_prefix(b"/")?;
    let rest = std::str::from_utf8(rest).ok()?;
    let (record_id, file) = rest.split_once('/')?;
    if record_id.is_empty()
        || record_id.len() > MAX_RECORD_ID_BYTES
        || matches!(record_id, "." | "..")
    {
        return None;
    }
    let digits = file.strip_suffix(".json")?;
    if !digits.bytes().all(|b| b.is_ascii_digit()) || digits.starts_with('0') {
        return None;
    }
    let revision: u64 = digits.parse().ok()?;
    Some((record_id, revision))
}
