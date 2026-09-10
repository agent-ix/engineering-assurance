// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Additive-migration parity and adverse coverage for content-rights policy.

use std::{
    io::Write,
    process::{Command, Stdio},
};

use engineering_assurance::content_rights::{
    ContentEntryKind, ContentRightsCategory, ContentRightsFinding, inspect_content,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

const PYTHON_REFERENCE: &str = r#"
import importlib.util
import json
import pathlib
import sys

root = pathlib.Path.cwd()
path = root / "scripts" / "check_content_rights.py"
spec = importlib.util.spec_from_file_location("retained_content_rights", path)
module = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = module
spec.loader.exec_module(module)

cases = json.load(sys.stdin)
results = []
for case in cases:
    findings = module.text_findings(case["path"], case["text"])
    results.append(sorted({(item.path, item.line, item.category) for item in findings}))
print(json.dumps(results, separators=(",", ":")))
"#;

fn findings(path: &str, bytes: &[u8], tokens: &[String]) -> Vec<ContentRightsFinding> {
    inspect_content(path, ContentEntryKind::File, bytes, tokens)
        .expect("repository-owned policy patterns must compile")
}

fn rendered(findings: &[ContentRightsFinding]) -> Vec<(String, usize, String)> {
    findings
        .iter()
        .map(|finding| {
            (
                finding.path.clone().expect("valid path finding"),
                finding.line,
                finding.category.to_string(),
            )
        })
        .collect()
}

fn python_findings(cases: &[Value], tokens: &[String]) -> Vec<Vec<(String, usize, String)>> {
    let mut child = Command::new("python3")
        .args(["-c", PYTHON_REFERENCE])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env("ASSURANCE_PROTECTED_TOKENS", tokens.join("\n"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("retained Python classifier must start during additive migration");
    child
        .stdin
        .take()
        .expect("piped stdin must exist")
        .write_all(&serde_json::to_vec(cases).expect("fixtures must serialize"))
        .expect("fixtures must be writable");
    let output = child.wait_with_output().expect("classifier must terminate");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("reference findings must be JSON")
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn every_retained_text_finding_and_exception_matches_python() {
    let private_unix = format!("/{}/person/private/file.txt", "home");
    let private_root = format!("/{}/private/file.txt", "root");
    let private_windows = format!("C:{}Users{}person{}private.txt", '\\', '\\', '\\');
    let private_tilde = format!("{}private/file.txt", "~/");
    let identifier = ["IS", "O 1234"].concat();
    let external_url = ["https:", "//example.invalid/source"].concat();
    let registry_url = ["https:", "//github.com/rust-lang/crates.io-index"].concat();
    let schema_url = ["http:", "//json-schema.org/draft-07/schema#"].concat();
    let license_url = ["https:", "//example.invalid/license"].concat();
    let protected = "Straße".to_owned();
    let ordinary = [
        private_unix,
        private_root,
        private_windows,
        private_tilde,
        identifier,
        "clause inventory".to_owned(),
        "applicability matrix".to_owned(),
        "standard crosswalk".to_owned(),
        "legal advice".to_owned(),
        external_url.clone(),
        "A".repeat(240),
        "STRASSE".to_owned(),
    ]
    .join("\n");
    let cases = vec![
        json!({"path": "candidate.md", "text": ordinary}),
        json!({"path": "Cargo.lock", "text": registry_url}),
        json!({"path": "deny.toml", "text": registry_url}),
        json!({"path": "candidate.json", "text": schema_url}),
        json!({"path": "LICENSE", "text": license_url}),
        json!({"path": "src/content_rights.rs", "text": "clause inventory"}),
        json!({"path": "tests/content_rights_parity.rs", "text": "legal advice"}),
        json!({"path": "AGENTS.md", "text": "applicability matrix"}),
        json!({"path": "CONTENT_RIGHTS.md", "text": "standard crosswalk"}),
        json!({"path": "content-rights.yaml", "text": "clause inventory"}),
        json!({"path": "scripts/check_content_rights.py", "text": "legal advice"}),
        json!({"path": "tests/test_content_rights.py", "text": "applicability table"}),
        json!({"path": "candidate.md", "text": format!("{}\n{}", external_url, external_url)}),
    ];
    let tokens = vec![protected];
    let expected = python_findings(&cases, &tokens);
    let observed = cases
        .iter()
        .map(|case| {
            rendered(&findings(
                case["path"].as_str().expect("path fixture"),
                case["text"].as_str().expect("text fixture").as_bytes(),
                &tokens,
            ))
        })
        .collect::<Vec<_>>();
    assert_eq!(observed, expected);
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn whole_file_boundaries_are_typed_and_short_circuit_content() {
    let cases = [
        (
            "candidate.md",
            ContentEntryKind::SymbolicLink,
            b"ignored".as_slice(),
            ContentRightsCategory::SymbolicLink,
        ),
        (
            "candidate.pdf",
            ContentEntryKind::File,
            b"ignored".as_slice(),
            ContentRightsCategory::ForbiddenFileType,
        ),
        (
            "candidate.bin",
            ContentEntryKind::File,
            b"ignored".as_slice(),
            ContentRightsCategory::UnreviewedFileType,
        ),
        (
            "candidate.md",
            ContentEntryKind::File,
            b"text\0payload".as_slice(),
            ContentRightsCategory::BinaryControlPayload,
        ),
        (
            "candidate.md",
            ContentEntryKind::File,
            &[0xff],
            ContentRightsCategory::NonUtf8Content,
        ),
    ];
    for (path, kind, bytes, expected) in cases {
        let observed = inspect_content(path, kind, bytes, &[])
            .expect("repository-owned policy patterns must compile");
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0].line, 0);
        assert_eq!(observed[0].category, expected);
    }

    for suffix in [
        "", ".cfg", ".css", ".html", ".js", ".json", ".lock", ".md", ".mjs", ".py", ".rs", ".sh",
        ".toml", ".ts", ".txt", ".yaml", ".yml",
    ] {
        let path = format!("candidate{suffix}");
        assert!(findings(&path, b"ordinary", &[]).is_empty(), "{suffix}");
    }
    for suffix in [
        ".doc", ".docx", ".epub", ".gif", ".jpg", ".jpeg", ".ods", ".odt", ".pdf", ".png", ".ppt",
        ".pptx", ".xls", ".xlsx", ".zip",
    ] {
        let path = format!("candidate{suffix}");
        assert_eq!(
            findings(&path, b"ordinary", &[])[0].category,
            ContentRightsCategory::ForbiddenFileType,
            "{suffix}"
        );
    }

    let at_limit = "x\n".repeat(256_000);
    assert!(findings("candidate.txt", at_limit.as_bytes(), &[]).is_empty());
    let oversized = vec![b'x'; 512_001];
    assert_eq!(
        findings("candidate.txt", &oversized, &[])[0].category,
        ContentRightsCategory::OversizedTextFile
    );
    let large_license = "x\n".repeat(256_001);
    assert!(findings("LICENSE", large_license.as_bytes(), &[]).is_empty());

    let lfs = ["version ", "https:", "//git-lfs.github.com/spec/v1\n"].concat();
    let lfs_finding = findings("candidate.txt", lfs.as_bytes(), &[]);
    assert_eq!(lfs_finding[0].line, 1);
    assert_eq!(
        lfs_finding[0].category,
        ContentRightsCategory::GitLfsPointer
    );
    assert!(findings("README.RS", b"ordinary", &[]).is_empty());
    assert!(findings(".gitignore", b"ordinary", &[]).is_empty());
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn unsafe_paths_refuse_before_classification_and_findings_leak_no_match() {
    let unsafe_paths = [
        "",
        "/absolute.md",
        "../escape.md",
        "nested/./candidate.md",
        "nested//candidate.md",
        "nested\\candidate.md",
        "C:/candidate.md",
        "candidate.md/",
        "candidate\nname.md",
    ];
    for path in unsafe_paths {
        let observed = findings(path, b"ordinary", &[]);
        assert_eq!(observed.len(), 1, "{path:?}");
        assert_eq!(observed[0].category, ContentRightsCategory::PathInvalid);
        assert_eq!(observed[0].line, 0);
        assert_eq!(observed[0].path, None);
    }

    let secret = format!("/{}/person/private-value", "home");
    let observed = findings("candidate.md", secret.as_bytes(), &[]);
    let encoded = serde_json::to_string(&observed).expect("findings must serialize");
    assert!(!encoded.contains(&secret));
    assert_eq!(
        observed[0].category,
        ContentRightsCategory::UnixWorkstationLocation
    );
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn python_line_boundaries_and_unicode_casefold_are_preserved() {
    let boundaries = [
        "\n", "\r", "\r\n", "\u{000b}", "\u{000c}", "\u{001c}", "\u{001d}", "\u{001e}", "\u{0085}",
        "\u{2028}", "\u{2029}",
    ];
    let private = format!("/{}/person/private", "home");
    for boundary in boundaries {
        let text = format!("ordinary{boundary}{private}");
        let observed = findings("candidate.md", text.as_bytes(), &[]);
        assert_eq!(observed.len(), 1, "boundary {boundary:?}");
        assert_eq!(observed[0].line, 2, "boundary {boundary:?}");
    }

    let tokens = vec!["Straße".to_owned(), "   ".to_owned()];
    let observed = findings("candidate.md", b"STRASSE", &tokens);
    assert_eq!(observed.len(), 1);
    assert_eq!(
        observed[0].category,
        ContentRightsCategory::ProtectedLocalToken
    );
}
