// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure content-rights classification over caller-supplied paths and bytes.
//!
//! This module owns no repository enumeration, filesystem access, environment
//! access, child program, network, clock, package, or publication behavior.

use std::{fmt, sync::LazyLock};

use regex::Regex;
use serde::Serialize;
use thiserror::Error;
use unicode_casefold::UnicodeCaseFold;

const MAX_TEXT_BYTES: usize = 512_000;

const TEXT_SUFFIXES: &[&str] = &[
    "", ".cfg", ".css", ".html", ".js", ".json", ".lock", ".md", ".mjs", ".py", ".rs", ".sh",
    ".toml", ".ts", ".txt", ".yaml", ".yml",
];

const FORBIDDEN_SUFFIXES: &[&str] = &[
    ".doc", ".docx", ".epub", ".gif", ".jpg", ".jpeg", ".ods", ".odt", ".pdf", ".png", ".ppt",
    ".pptx", ".xls", ".xlsx", ".zip",
];

const SEMANTIC_POLICY_FILES: &[&str] = &[
    "AGENTS.md",
    "CONTENT_RIGHTS.md",
    "content-rights.yaml",
    "src/content_rights.rs",
    "tests/content_rights_parity.rs",
    "scripts/check_content_rights.py",
    "tests/test_content_rights.py",
];

/// Whether a candidate tree entry is a regular file or symbolic link.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentEntryKind {
    /// A regular file whose bytes can be classified.
    File,
    /// A symbolic link, which is rejected without following it.
    SymbolicLink,
}

/// Closed content-rights finding taxonomy.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContentRightsCategory {
    /// The caller supplied an unsafe or non-normalized relative path.
    PathInvalid,
    /// A candidate tree entry is a symbolic link.
    SymbolicLink,
    /// A suffix is explicitly forbidden by the publication policy.
    ForbiddenFileType,
    /// A suffix has no reviewed text or forbidden disposition.
    UnreviewedFileType,
    /// A non-license text file exceeds the fixed byte ceiling.
    OversizedTextFile,
    /// A candidate contains a NUL byte.
    BinaryControlPayload,
    /// A candidate is not valid UTF-8.
    NonUtf8Content,
    /// A candidate contains a Git LFS pointer rather than owned bytes.
    GitLfsPointer,
    /// A line exposes a Unix workstation location.
    UnixWorkstationLocation,
    /// A line exposes a root-user workstation location.
    RootWorkstationLocation,
    /// A line exposes a Windows workstation location.
    WindowsWorkstationLocation,
    /// A line exposes a tilde-relative workstation location.
    TildeWorkstationLocation,
    /// A line names an external publication identifier.
    ExternalPublicationIdentifier,
    /// A line appears to contain an external rule inventory.
    ExternalRuleInventory,
    /// A line appears to contain an applicability matrix.
    ApplicabilityMatrix,
    /// A line appears to contain an external crosswalk.
    ExternalCrosswalk,
    /// A line appears to contain legal-review material.
    LegalReviewMaterial,
    /// A line contains an external URL not admitted at that path.
    UnapprovedExternalUrl,
    /// A line contains a long encoded payload.
    EncodedPayload,
    /// A line contains a caller-supplied protected token.
    ProtectedLocalToken,
}

impl ContentRightsCategory {
    /// Return the retained human-readable category spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PathInvalid => "invalid path",
            Self::SymbolicLink => "symbolic link",
            Self::ForbiddenFileType => "forbidden file type",
            Self::UnreviewedFileType => "unreviewed file type",
            Self::OversizedTextFile => "oversized text file",
            Self::BinaryControlPayload => "binary control payload",
            Self::NonUtf8Content => "non-UTF-8 content",
            Self::GitLfsPointer => "Git LFS pointer",
            Self::UnixWorkstationLocation => "unix workstation location",
            Self::RootWorkstationLocation => "root workstation location",
            Self::WindowsWorkstationLocation => "windows workstation location",
            Self::TildeWorkstationLocation => "tilde workstation location",
            Self::ExternalPublicationIdentifier => "external publication identifier",
            Self::ExternalRuleInventory => "external rule inventory",
            Self::ApplicabilityMatrix => "applicability matrix",
            Self::ExternalCrosswalk => "external crosswalk",
            Self::LegalReviewMaterial => "legal review material",
            Self::UnapprovedExternalUrl => "unapproved external URL",
            Self::EncodedPayload => "encoded payload",
            Self::ProtectedLocalToken => "protected local token",
        }
    }
}

impl fmt::Display for ContentRightsCategory {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// One content-rights finding that never carries the rejected source bytes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContentRightsFinding {
    /// Normalized repository-relative path, absent when the supplied path is unsafe.
    pub path: Option<String>,
    /// One-based logical line number, or zero for a whole-file finding.
    pub line: usize,
    /// Closed reason the candidate cannot enter the publishable boundary.
    pub category: ContentRightsCategory,
}

/// Typed failure from the pure content-rights boundary.
#[derive(Debug, Error)]
pub enum ContentRightsError {
    /// One repository-owned regular expression failed to compile.
    #[error("content-rights policy pattern is invalid")]
    InvalidPolicyPattern,
    /// A closed content-rights result could not be serialized.
    #[error("content-rights result serialization failed: {0}")]
    Serialization(serde_json::Error),
}

impl ContentRightsError {
    /// Return the stable machine category for this library failure.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidPolicyPattern => "content_rights_policy_invalid",
            Self::Serialization(_) => "content_rights_result_serialization_failed",
        }
    }
}

/// Protocol discriminator for a complete selected-tree result.
pub const CONTENT_RIGHTS_TREE_PROTOCOL: &str =
    "engineering-assurance.content-rights-tree-result/v1";

/// Closed selected-tree content-rights outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContentRightsTreeOutcome {
    /// No selected entry produced a finding.
    Accepted,
    /// At least one selected entry produced a finding.
    Withheld,
}

/// Deterministic result for one complete selected repository tree.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContentRightsTreeResult {
    protocol: &'static str,
    outcome: ContentRightsTreeOutcome,
    inspected_entries: usize,
    findings: Vec<ContentRightsFinding>,
}

impl ContentRightsTreeResult {
    /// Construct a result and canonicalize its finding population.
    #[must_use]
    pub fn new(inspected_entries: usize, mut findings: Vec<ContentRightsFinding>) -> Self {
        sort_findings(&mut findings);
        let outcome = if findings.is_empty() {
            ContentRightsTreeOutcome::Accepted
        } else {
            ContentRightsTreeOutcome::Withheld
        };
        Self {
            protocol: CONTENT_RIGHTS_TREE_PROTOCOL,
            outcome,
            inspected_entries,
            findings,
        }
    }

    /// Return the fixed result protocol discriminator.
    #[must_use]
    pub const fn protocol(&self) -> &'static str {
        self.protocol
    }

    /// Return the closed accepted or withheld outcome.
    #[must_use]
    pub const fn outcome(&self) -> ContentRightsTreeOutcome {
        self.outcome
    }

    /// Return the number of regular files and symbolic links inspected.
    #[must_use]
    pub const fn inspected_entries(&self) -> usize {
        self.inspected_entries
    }

    /// Return the canonical finding population.
    #[must_use]
    pub fn findings(&self) -> &[ContentRightsFinding] {
        &self.findings
    }

    /// Encode one newline-terminated machine result.
    ///
    /// # Errors
    ///
    /// Returns a typed error if serialization unexpectedly fails.
    pub fn to_json_line(&self) -> Result<Vec<u8>, ContentRightsError> {
        let mut encoded = serde_json::to_vec(self).map_err(ContentRightsError::Serialization)?;
        encoded.push(b'\n');
        Ok(encoded)
    }
}

struct Patterns {
    unix_workstation: Regex,
    root_workstation: Regex,
    windows_workstation: Regex,
    external_identifier: Regex,
    semantic: Vec<(ContentRightsCategory, Regex)>,
    url: Regex,
}

impl Patterns {
    fn compile() -> Result<Self, regex::Error> {
        Ok(Self {
            unix_workstation: Regex::new(r#"/(?:home|Users)/[^/\s"'`]+"#)?,
            root_workstation: Regex::new(concat!("/", r"root(?:/|\b)"))?,
            windows_workstation: Regex::new(
                r"(?i)\b[A-Z]:[\\/](?:Users|Documents and Settings)[\\/]",
            )?,
            external_identifier: Regex::new(
                r"(?i)\b(?:ISO(?:[ /_-]*IEC)?|IEC|IEEE|NIST|NPR|ECSS|DO)[ /_:-]*\d+[A-Za-z]?(?:[-.:/]\d+)*\b",
            )?,
            semantic: vec![
                (
                    ContentRightsCategory::ExternalRuleInventory,
                    Regex::new(r"(?i)\b(?:rule|clause) inventory\b")?,
                ),
                (
                    ContentRightsCategory::ApplicabilityMatrix,
                    Regex::new(r"(?i)\bapplicability (?:matrix|table)\b")?,
                ),
                (
                    ContentRightsCategory::ExternalCrosswalk,
                    Regex::new(r"(?i)\b(?:standard|clause) crosswalk\b")?,
                ),
                (
                    ContentRightsCategory::LegalReviewMaterial,
                    Regex::new(r"(?i)\b(?:counsel package|legal advice)\b")?,
                ),
            ],
            url: Regex::new(r#"https?://[^\s)\]>"']+"#)?,
        })
    }
}

static PATTERNS: LazyLock<Result<Patterns, regex::Error>> = LazyLock::new(Patterns::compile);

/// Classify one caller-supplied candidate without performing I/O.
///
/// Findings are sorted and deduplicated by path, line, and retained category
/// spelling. Rejected source text and protected token values are never returned.
///
/// # Errors
///
/// Returns [`ContentRightsError::InvalidPolicyPattern`] if a built-in policy
/// expression cannot initialize.
pub fn inspect_content(
    path: &str,
    kind: ContentEntryKind,
    bytes: &[u8],
    protected_tokens: &[String],
) -> Result<Vec<ContentRightsFinding>, ContentRightsError> {
    let patterns = PATTERNS
        .as_ref()
        .map_err(|_| ContentRightsError::InvalidPolicyPattern)?;
    if !is_normalized_relative_path(path) {
        return Ok(vec![ContentRightsFinding {
            path: None,
            line: 0,
            category: ContentRightsCategory::PathInvalid,
        }]);
    }
    if let Some(category) = whole_file_finding(path, kind, bytes) {
        return Ok(vec![finding(path, 0, category)]);
    }
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Ok(vec![finding(
            path,
            0,
            ContentRightsCategory::NonUtf8Content,
        )]);
    };
    let lfs_pointer = concat!("version ", "https:", "//git-lfs.github.com/spec/v1");
    if text.starts_with(lfs_pointer) {
        return Ok(vec![finding(path, 1, ContentRightsCategory::GitLfsPointer)]);
    }

    let folded_tokens = protected_tokens
        .iter()
        .filter(|token| !token.trim().is_empty())
        .map(|token| token.case_fold().collect::<String>())
        .collect::<Vec<_>>();
    Ok(inspect_text_lines(path, text, &folded_tokens, patterns))
}

/// Split a protected-token environment value with the retained logical-line
/// boundaries while preserving each nonblank token's exact spelling.
#[must_use]
pub fn split_protected_tokens(raw: &str) -> Vec<String> {
    LogicalLines::new(raw)
        .filter(|token| !token.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

fn whole_file_finding(
    path: &str,
    kind: ContentEntryKind,
    bytes: &[u8],
) -> Option<ContentRightsCategory> {
    if kind == ContentEntryKind::SymbolicLink {
        return Some(ContentRightsCategory::SymbolicLink);
    }
    let suffix = suffix(path);
    if FORBIDDEN_SUFFIXES.contains(&suffix.as_str()) {
        return Some(ContentRightsCategory::ForbiddenFileType);
    }
    if !TEXT_SUFFIXES.contains(&suffix.as_str()) {
        return Some(ContentRightsCategory::UnreviewedFileType);
    }
    if bytes.len() > MAX_TEXT_BYTES && path != "LICENSE" {
        return Some(ContentRightsCategory::OversizedTextFile);
    }
    bytes
        .contains(&0)
        .then_some(ContentRightsCategory::BinaryControlPayload)
}

fn inspect_text_lines(
    path: &str,
    text: &str,
    folded_tokens: &[String],
    patterns: &Patterns,
) -> Vec<ContentRightsFinding> {
    let semantic_checks = !SEMANTIC_POLICY_FILES.contains(&path);
    let mut findings = Vec::new();
    for (index, line) in LogicalLines::new(text).enumerate() {
        let line_number = index.saturating_add(1);
        append_location_findings(&mut findings, path, line_number, line, patterns);
        append_policy_findings(
            &mut findings,
            path,
            line_number,
            line,
            folded_tokens,
            semantic_checks,
            patterns,
        );
    }
    sort_findings(&mut findings);
    findings
}

fn sort_findings(findings: &mut Vec<ContentRightsFinding>) {
    findings.sort_by(|left, right| {
        (left.path.as_deref(), left.line, left.category.as_str()).cmp(&(
            right.path.as_deref(),
            right.line,
            right.category.as_str(),
        ))
    });
    findings.dedup();
}

fn append_location_findings(
    findings: &mut Vec<ContentRightsFinding>,
    path: &str,
    line_number: usize,
    line: &str,
    patterns: &Patterns,
) {
    for (category, found) in [
        (
            ContentRightsCategory::UnixWorkstationLocation,
            patterns.unix_workstation.is_match(line),
        ),
        (
            ContentRightsCategory::RootWorkstationLocation,
            patterns.root_workstation.is_match(line),
        ),
        (
            ContentRightsCategory::WindowsWorkstationLocation,
            patterns.windows_workstation.is_match(line),
        ),
        (
            ContentRightsCategory::TildeWorkstationLocation,
            contains_tilde_workstation_path(line),
        ),
    ] {
        if found {
            findings.push(finding(path, line_number, category));
        }
    }
}

fn append_policy_findings(
    findings: &mut Vec<ContentRightsFinding>,
    path: &str,
    line_number: usize,
    line: &str,
    folded_tokens: &[String],
    semantic_checks: bool,
    patterns: &Patterns,
) {
    if patterns.external_identifier.is_match(line) {
        findings.push(finding(
            path,
            line_number,
            ContentRightsCategory::ExternalPublicationIdentifier,
        ));
    }
    if semantic_checks {
        for (category, pattern) in &patterns.semantic {
            if pattern.is_match(line) {
                findings.push(finding(path, line_number, *category));
            }
        }
    }
    if path != "LICENSE"
        && patterns
            .url
            .find_iter(line)
            .any(|matched| !url_is_allowed(path, matched.as_str()))
    {
        findings.push(finding(
            path,
            line_number,
            ContentRightsCategory::UnapprovedExternalUrl,
        ));
    }
    if contains_encoded_payload(line) {
        findings.push(finding(
            path,
            line_number,
            ContentRightsCategory::EncodedPayload,
        ));
    }
    if !folded_tokens.is_empty() {
        let folded_line = line.case_fold().collect::<String>();
        if folded_tokens
            .iter()
            .any(|token| folded_line.contains(token))
        {
            findings.push(finding(
                path,
                line_number,
                ContentRightsCategory::ProtectedLocalToken,
            ));
        }
    }
}

fn finding(path: &str, line: usize, category: ContentRightsCategory) -> ContentRightsFinding {
    ContentRightsFinding {
        path: Some(path.to_owned()),
        line,
        category,
    }
}

fn suffix(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rfind('.') {
        Some(index) if index > 0 && index + 1 < name.len() => name[index..].to_ascii_lowercase(),
        Some(_) | None => String::new(),
    }
}

fn is_normalized_relative_path(path: &str) -> bool {
    if path.is_empty()
        || path.starts_with('/')
        || path.ends_with('/')
        || path.contains('\\')
        || path.chars().any(char::is_control)
    {
        return false;
    }
    let mut components = path.split('/');
    let Some(first) = components.next() else {
        return false;
    };
    if is_drive_prefix(first) || !is_safe_component(first) {
        return false;
    }
    components.all(is_safe_component)
}

fn is_drive_prefix(component: &str) -> bool {
    let bytes = component.as_bytes();
    bytes.len() == 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

fn is_safe_component(component: &str) -> bool {
    !component.is_empty() && component != "." && component != ".."
}

fn contains_tilde_workstation_path(line: &str) -> bool {
    line.match_indices("~/").any(|(index, _)| {
        let boundary = index == 0
            || line[..index]
                .chars()
                .next_back()
                .is_some_and(char::is_whitespace);
        let after = index.saturating_add(2);
        boundary
            && line[after..]
                .chars()
                .next()
                .is_some_and(|character| !character.is_whitespace())
    })
}

fn url_is_allowed(path: &str, url: &str) -> bool {
    let schema_prefix = concat!("http:", "//json-schema.org/draft-07/schema#");
    let cargo_registry = concat!("https:", "//github.com/rust-lang/crates.io-index");
    url.starts_with(schema_prefix)
        || matches!(path, "Cargo.lock" | "deny.toml") && url == cargo_registry
}

fn contains_encoded_payload(line: &str) -> bool {
    let mut run = 0usize;
    for byte in line.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'/') {
            run = run.saturating_add(1);
            if run >= 240 {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

struct LogicalLines<'a> {
    text: &'a str,
    cursor: usize,
}

impl<'a> LogicalLines<'a> {
    const fn new(text: &'a str) -> Self {
        Self { text, cursor: 0 }
    }
}

impl<'a> Iterator for LogicalLines<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor >= self.text.len() {
            return None;
        }
        let start = self.cursor;
        for (relative, character) in self.text[start..].char_indices() {
            if is_line_boundary(character) {
                let boundary = start.saturating_add(relative);
                let mut next = boundary.saturating_add(character.len_utf8());
                if character == '\r' && self.text.as_bytes().get(next) == Some(&b'\n') {
                    next = next.saturating_add(1);
                }
                self.cursor = next;
                return Some(&self.text[start..boundary]);
            }
        }
        self.cursor = self.text.len();
        Some(&self.text[start..])
    }
}

const fn is_line_boundary(character: char) -> bool {
    matches!(
        character,
        '\n' | '\r'
            | '\u{000b}'
            | '\u{000c}'
            | '\u{001c}'
            | '\u{001d}'
            | '\u{001e}'
            | '\u{0085}'
            | '\u{2028}'
            | '\u{2029}'
    )
}
