// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Additive-migration parity and adverse coverage for content-rights policy.

use engineering_assurance::content_rights::{
    ContentEntryKind, ContentRightsCategory, ContentRightsFinding, inspect_content,
};
use ix_trace_rs::trace;

fn findings(path: &str, bytes: &[u8], tokens: &[String]) -> Vec<ContentRightsFinding> {
    inspect_content(path, ContentEntryKind::File, bytes, tokens)
        .expect("repository-owned policy patterns must compile")
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn retained_text_finding_and_exception_correspondence_is_fixed() {
    let private_unix = format!("/{}/person/private/file.txt", "home");
    let private_root = format!("/{}/private/file.txt", "root");
    let private_windows = format!("C:{}Users{}person{}private.txt", '\\', '\\', '\\');
    let private_tilde = format!("{}private/file.txt", "~/");
    let identifier = ["IS", "O 1234"].concat();
    let external_url = ["https:", "//example.invalid/source"].concat();
    let registry_url = ["https:", "//github.com/rust-lang/crates.io-index"].concat();
    let git_dependency_url = ["https:", "//github.com/agent-ix/ix-trace-rs"].concat();
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
    let tokens = vec![protected];
    let observed = findings("candidate.md", ordinary.as_bytes(), &tokens)
        .into_iter()
        .map(|finding| (finding.line, finding.category))
        .collect::<Vec<_>>();
    assert_eq!(
        observed,
        vec![
            (1, ContentRightsCategory::UnixWorkstationLocation),
            (2, ContentRightsCategory::RootWorkstationLocation),
            (3, ContentRightsCategory::WindowsWorkstationLocation),
            (4, ContentRightsCategory::TildeWorkstationLocation),
            (5, ContentRightsCategory::ExternalPublicationIdentifier),
            (6, ContentRightsCategory::ExternalRuleInventory),
            (7, ContentRightsCategory::ApplicabilityMatrix),
            (8, ContentRightsCategory::ExternalCrosswalk),
            (9, ContentRightsCategory::LegalReviewMaterial),
            (10, ContentRightsCategory::UnapprovedExternalUrl),
            (11, ContentRightsCategory::EncodedPayload),
            (12, ContentRightsCategory::ProtectedLocalToken),
        ]
    );

    for (path, text) in [
        ("Cargo.lock", registry_url.as_str()),
        ("deny.toml", registry_url.as_str()),
        ("Cargo.toml", git_dependency_url.as_str()),
        ("Cargo.lock", git_dependency_url.as_str()),
        ("deny.toml", git_dependency_url.as_str()),
        ("candidate.json", schema_url.as_str()),
        ("LICENSE", license_url.as_str()),
        ("src/content_rights.rs", "clause inventory"),
        ("tests/content_rights_parity.rs", "legal advice"),
        ("AGENTS.md", "applicability matrix"),
        ("CONTENT_RIGHTS.md", "standard crosswalk"),
        ("content-rights.yaml", "clause inventory"),
    ] {
        assert!(
            findings(path, text.as_bytes(), &tokens).is_empty(),
            "{path}"
        );
    }
    // The git-dependency URL is approved only in the manifest/lock/policy
    // files it can legitimately name, not in arbitrary content.
    assert!(
        !findings("candidate.md", git_dependency_url.as_bytes(), &tokens).is_empty(),
        "an unqualified file must not inherit the manifest's git-dependency exemption"
    );
    // The manifest allowance is scoped to the exact root-level file names, not
    // any path that happens to end with one of them. A `path.ends_with(...)`
    // regression would wrongly exempt these.
    for path in ["vendor/x/Cargo.toml", "crates/sub/Cargo.toml"] {
        assert!(
            !findings(path, git_dependency_url.as_bytes(), &tokens).is_empty(),
            "{path} must not inherit the manifest exemption by suffix match"
        );
    }
    // The manifest allowance is an exact match on the single approved
    // dependency URL, not an org-wide prefix. A look-alike, squattable org
    // (e.g. `agent-ix-evil`) must still be refused in Cargo.toml.
    let look_alike_org_url = ["https:", "//github.com/agent-ix-evil/x"].concat();
    assert!(
        !findings("Cargo.toml", look_alike_org_url.as_bytes(), &tokens).is_empty(),
        "a look-alike agent-ix org URL must not be admitted by the manifest exemption"
    );
    assert_eq!(
        findings(
            "candidate.md",
            format!("{external_url}\n{external_url}").as_bytes(),
            &tokens,
        )
        .into_iter()
        .map(|finding| (finding.line, finding.category))
        .collect::<Vec<_>>(),
        vec![
            (1, ContentRightsCategory::UnapprovedExternalUrl),
            (2, ContentRightsCategory::UnapprovedExternalUrl),
        ]
    );
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn onboarding_report_url_exemption_is_scoped_to_its_exact_path() {
    // The assurance-onboarding skill's onboarding report (PLAT-924) may link
    // sibling first-party repos, by its exact path only.
    let tokens: Vec<String> = Vec::new();
    let onboarding_report_path =
        "engineering_assurance/skills/assurance-onboarding/scripts/onboard.js";
    let sibling_repo_url = ["https:", "//github.com/agent-ix/qa-corpus/tree/main"].concat();
    assert!(
        findings(onboarding_report_path, sibling_repo_url.as_bytes(), &tokens).is_empty(),
        "the onboarding report must be able to link a first-party sibling repository"
    );
    assert!(
        !findings("candidate.md", sibling_repo_url.as_bytes(), &tokens).is_empty(),
        "an unqualified file must not inherit the onboarding report's exemption"
    );
    assert!(
        !findings("other/onboard.js", sibling_repo_url.as_bytes(), &tokens).is_empty(),
        "a same-named file elsewhere must not inherit the exemption by suffix match"
    );
    // The exemption is an org-prefix match, not an exact single URL, but must
    // still refuse a look-alike, squattable org such as `agent-ix-evil`.
    let onboarding_look_alike_url = ["https:", "//github.com/agent-ix-evil/x"].concat();
    assert!(
        !findings(
            onboarding_report_path,
            onboarding_look_alike_url.as_bytes(),
            &tokens
        )
        .is_empty(),
        "a look-alike agent-ix org URL must not be admitted by the onboarding-report exemption"
    );
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn project_metadata_agent_ix_exemption_refuses_look_alike_orgs() {
    // No prior test exercised this branch at all. The bare org URL (the
    // plugin manifests' own `author.url`) and any URL under it must both be
    // admitted in project-metadata files, but a look-alike org must still be
    // refused — the same property the onboarding-report exemption already
    // had, that this one lacked until PLAT-924 added the sibling exemption
    // next to it and exposed the gap.
    let tokens: Vec<String> = Vec::new();
    let bare_org_url = ["https:", "//github.com/agent-ix"].concat();
    let org_prefixed_url = ["https:", "//github.com/agent-ix/quoin"].concat();
    let look_alike_org_url = ["https:", "//github.com/agent-ix-evil/x"].concat();
    for path in ["README.md", ".claude-plugin/plugin.json"] {
        assert!(
            findings(path, bare_org_url.as_bytes(), &tokens).is_empty(),
            "{path} must admit the bare agent-ix org URL"
        );
        assert!(
            findings(path, org_prefixed_url.as_bytes(), &tokens).is_empty(),
            "{path} must admit a URL under the agent-ix org"
        );
        assert!(
            !findings(path, look_alike_org_url.as_bytes(), &tokens).is_empty(),
            "{path} must refuse a look-alike agent-ix org URL"
        );
    }
    assert!(
        !findings("candidate.md", bare_org_url.as_bytes(), &tokens).is_empty(),
        "a non-project-metadata file must not inherit this exemption"
    );
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
