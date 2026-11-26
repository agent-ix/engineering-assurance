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

const FR_017_SPEC: &str =
    include_str!("../spec/functional/FR-017-rust-evaluation-and-qualification.md");

#[derive(Debug)]
struct ClauseSourceRow {
    scheme: String,
    host: String,
    url_path: String,
    query: Option<String>,
    wheel_member: String,
    npm_member: String,
}

fn clause_source_rows_from_spec() -> Vec<ClauseSourceRow> {
    const TABLE_HEADER: &str =
        "| Scheme | Host | URL path | Query | Wheel member path | npm member path |";

    let mut in_table = false;
    let mut rows = Vec::new();
    for line in FR_017_SPEC.lines().map(str::trim) {
        if !in_table {
            in_table = line == TABLE_HEADER;
            continue;
        }
        if !line.starts_with('|') {
            break;
        }
        if line.contains("---") {
            continue;
        }

        let cells = markdown_table_cells(line).expect("FR-017 clause-source row must be valid");
        assert_eq!(cells.len(), 6, "FR-017 clause-source row column count");
        rows.push(ClauseSourceRow {
            scheme: cells[0].to_owned(),
            host: cells[1].to_owned(),
            url_path: cells[2].to_owned(),
            query: (cells[3] != "none").then(|| cells[3].to_owned()),
            wheel_member: cells[4].to_owned(),
            npm_member: cells[5].to_owned(),
        });
    }
    assert!(
        !rows.is_empty(),
        "FR-017 clause-source table must have rows"
    );
    rows
}

fn markdown_table_cells(line: &str) -> Option<Vec<&str>> {
    let mut cells = line.split('|').collect::<Vec<_>>();
    if cells.first()?.trim().is_empty() && cells.last()?.trim().is_empty() {
        cells.remove(0);
        cells.pop();
    } else {
        return None;
    }
    Some(
        cells
            .into_iter()
            .map(|cell| cell.trim().trim_matches(char::from(96)))
            .collect(),
    )
}

fn clause_source_url(row: &ClauseSourceRow) -> String {
    let query = row
        .query
        .as_deref()
        .map(|query| format!("?{query}"))
        .unwrap_or_default();
    format!("{}://{}{}{}", row.scheme, row.host, row.url_path, query)
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
        external_url,
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
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn retained_path_exceptions_are_exact() {
    let tokens: Vec<String> = Vec::new();
    let registry_url = ["https:", "//github.com/rust-lang/crates.io-index"].concat();
    let git_dependency_url = ["https:", "//github.com/agent-ix/ix-trace-rs"].concat();
    let schema_url = ["http:", "//json-schema.org/draft-07/schema#"].concat();
    let license_url = ["https:", "//example.invalid/license"].concat();
    for (path, text) in [
        ("Cargo.lock", registry_url.as_str()),
        ("deny.toml", registry_url.as_str()),
        ("Cargo.toml", git_dependency_url.as_str()),
        ("Cargo.lock", git_dependency_url.as_str()),
        ("deny.toml", git_dependency_url.as_str()),
        ("candidate.json", schema_url.as_str()),
        ("LICENSE", license_url.as_str()),
        ("package/LICENSE", license_url.as_str()),
        (
            "engineering_assurance-0.8.2.dist-info/licenses/LICENSE",
            license_url.as_str(),
        ),
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
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn unapproved_url_findings_keep_line_numbers() {
    let tokens: Vec<String> = Vec::new();
    let external_url = ["https:", "//example.invalid/source"].concat();
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
#[trace("FR-079-AC-1")]
fn attestation_predicate_uris_are_allowed_in_markdown_and_rust() {
    let tokens: Vec<String> = Vec::new();
    let approval = ["https:", "//schemas.agent-ix.ai/attestation/approval/v1"].concat();
    let change_record = [
        "https:",
        "//schemas.agent-ix.ai/attestation/change-record/v1",
    ]
    .concat();

    for path in ["approval.md", "src/predicate.rs"] {
        let text = format!("{approval}\n{change_record}\n");
        assert!(
            findings(path, text.as_bytes(), &tokens).is_empty(),
            "the organization predicate URIs must be allowed in {path}"
        );
    }
}

#[test]
#[trace("FR-079-AC-2")]
fn attestation_schema_allowlist_rejects_lookalike_hosts() {
    let tokens: Vec<String> = Vec::new();
    let subdomain = [
        "https:",
        "//schemas.agent-ix.ai.attacker.invalid/attestation/approval/v1",
    ]
    .concat();
    let credential = [
        "https:",
        "//schemas.agent-ix.ai@attacker.invalid/attestation/approval/v1",
    ]
    .concat();

    for url in [subdomain, credential] {
        let observed = findings("candidate.md", url.as_bytes(), &tokens);
        assert_eq!(observed.len(), 1, "lookalike URL must be refused: {url}");
        assert_eq!(
            observed[0].category,
            ContentRightsCategory::UnapprovedExternalUrl,
            "lookalike URL must be refused by the external URL rule: {url}"
        );
    }
}

#[test]
#[trace("FR-079-AC-3")]
fn attestation_schema_allowlist_requires_https_and_attestation_path() {
    let tokens: Vec<String> = Vec::new();
    let other_path = ["https:", "//schemas.agent-ix.ai/other/approval/v1"].concat();
    let insecure = ["http:", "//schemas.agent-ix.ai/attestation/approval/v1"].concat();

    for url in [other_path, insecure] {
        let observed = findings("candidate.md", url.as_bytes(), &tokens);
        assert_eq!(observed.len(), 1, "URL must be refused: {url}");
        assert_eq!(
            observed[0].category,
            ContentRightsCategory::UnapprovedExternalUrl
        );
    }
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn removed_onboarding_script_path_has_no_url_exemption() {
    // The retired JavaScript path must no longer have a special exemption.
    let tokens: Vec<String> = Vec::new();
    let onboarding_report_path =
        "engineering_assurance/skills/assurance-onboarding/scripts/onboard.js";
    let sibling_repo_url = ["https:", "//github.com/agent-ix/qa-corpus/tree/main"].concat();
    assert!(
        !findings(onboarding_report_path, sibling_repo_url.as_bytes(), &tokens).is_empty(),
        "the retired script path must not retain a URL exemption"
    );
    assert!(
        !findings("candidate.md", sibling_repo_url.as_bytes(), &tokens).is_empty(),
        "an unqualified file must not inherit the onboarding report's exemption"
    );
    assert!(
        !findings("other/onboard.js", sibling_repo_url.as_bytes(), &tokens).is_empty(),
        "a same-named file elsewhere must not inherit the exemption by suffix match"
    );
    // A look-alike, squattable org must also be refused.
    let onboarding_look_alike_url = ["https:", "//github.com/agent-ix-evil/x"].concat();
    assert!(
        !findings(
            onboarding_report_path,
            onboarding_look_alike_url.as_bytes(),
            &tokens
        )
        .is_empty(),
        "a look-alike agent-ix org URL must be refused"
    );
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn project_metadata_agent_ix_exemption_refuses_look_alike_orgs() {
    // No prior test exercised this branch at all. The bare org URL (the
    // plugin manifests' own `author.url`) and any URL under it must both be
    // admitted in project-metadata files, but a look-alike org must still be
    // refused — the same property the onboarding-report exemption already
    // had, that this one lacked.
    let tokens: Vec<String> = Vec::new();
    let bare_org_url = ["https:", "//github.com/agent-ix"].concat();
    let org_prefixed_url = ["https:", "//github.com/agent-ix/quoin"].concat();
    let look_alike_org_url = ["https:", "//github.com/agent-ix-evil/x"].concat();
    for path in [
        "README.md",
        ".claude-plugin/plugin.json",
        "package.json",
        "package/package.json",
    ] {
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
    for path in [
        "nested/package.json",
        "package/child/package.json",
        "other/package/package.json",
    ] {
        assert!(
            !findings(path, org_prefixed_url.as_bytes(), &tokens).is_empty(),
            "{path} must not inherit package metadata context by suffix"
        );
    }
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn official_clause_source_urls_match_spec_at_archive_paths() {
    let tokens: Vec<String> = Vec::new();
    for row in clause_source_rows_from_spec() {
        let source_url = clause_source_url(&row);
        for path in [&row.wheel_member, &row.npm_member] {
            assert!(
                findings(path, source_url.as_bytes(), &tokens).is_empty(),
                "{path}"
            );
        }
        assert!(
            !findings("candidate.json", source_url.as_bytes(), &tokens).is_empty(),
            "an unrelated path must refuse {source_url}"
        );
    }
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn clause_source_paths_refuse_unrelated_lookalike_and_standards_urls() {
    let tokens: Vec<String> = Vec::new();
    let rows = clause_source_rows_from_spec();
    let paths = rows
        .iter()
        .flat_map(|row| [&row.wheel_member, &row.npm_member])
        .collect::<std::collections::BTreeSet<_>>();
    let unrelated = ["https:", "//example.invalid/unrelated"].concat();
    let standards = ["https:", "//standards.example.invalid/catalog/1234"].concat();

    for path in paths {
        let source = rows
            .iter()
            .find(|row| row.wheel_member == *path || row.npm_member == *path)
            .expect("each table path must have a source row");
        let lookalike = format!(
            "{}://{}.example.invalid{}",
            source.scheme, source.host, source.url_path
        );
        for rejected in [&unrelated, &lookalike, &standards] {
            assert!(
                !findings(path, rejected.as_bytes(), &tokens).is_empty(),
                "{path}"
            );
        }
    }
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn eur_lex_legal_notice_is_limited_to_gdpr_and_ai_act_paths() {
    let tokens: Vec<String> = Vec::new();
    let rows = clause_source_rows_from_spec();
    let legal_notice_row = rows
        .iter()
        .find(|row| row.url_path == "/content/legal-notice/legal-notice.html")
        .expect("FR-017 must scope the EUR-Lex legal notice");
    let legal_notice = clause_source_url(legal_notice_row);

    for path in rows
        .iter()
        .flat_map(|row| [&row.wheel_member, &row.npm_member])
        .collect::<std::collections::BTreeSet<_>>()
    {
        let explicitly_attributed = rows.iter().any(|row| {
            row.url_path == "/content/legal-notice/legal-notice.html"
                && (row.wheel_member == *path || row.npm_member == *path)
        });
        let observed = findings(path, legal_notice.as_bytes(), &tokens).is_empty();
        assert_eq!(observed, explicitly_attributed, "{path}");
    }
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn query_bearing_clause_sources_refuse_missing_or_altered_queries() {
    let tokens: Vec<String> = Vec::new();
    for row in clause_source_rows_from_spec()
        .into_iter()
        .filter(|row| row.query.is_some())
    {
        let query = row
            .query
            .as_deref()
            .expect("filtered query must be present");
        let url_without_query = format!("{}://{}{}", row.scheme, row.host, row.url_path);
        let url_with_altered_query = format!("{url_without_query}?{query}-changed");

        for path in [&row.wheel_member, &row.npm_member] {
            for rejected in [&url_without_query, &url_with_altered_query] {
                assert!(
                    !findings(path, rejected.as_bytes(), &tokens).is_empty(),
                    "query mismatch must be refused at {path}"
                );
            }
        }
    }
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
        "", ".cfg", ".css", ".html", ".js", ".json", ".lock", ".md", ".mod", ".mjs", ".py", ".rs",
        ".sh", ".toml", ".ts", ".txt", ".yaml", ".yml",
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
    for path in [
        "LICENSE",
        "package/LICENSE",
        "engineering_assurance-0.8.2.dist-info/licenses/LICENSE",
    ] {
        assert!(
            findings(path, large_license.as_bytes(), &[]).is_empty(),
            "{path}"
        );
    }

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

#[test]
#[trace("TC-119", "FR-028-AC-4")]
fn shared_cli_dependency_allowance_is_exact_and_metadata_only() {
    let dependency = ["https:", "//github.com/agent-ix/ix-cli-kit"].concat();
    let tokens = Vec::new();
    for path in ["Cargo.toml", "Cargo.lock", "deny.toml"] {
        for suffix in ["", "?rev=fictional#fictional"] {
            assert!(findings(path, format!("{dependency}{suffix}").as_bytes(), &tokens).is_empty());
        }
        for suffix in ["-evil", "/unrelated"] {
            assert!(
                !findings(path, format!("{dependency}{suffix}").as_bytes(), &tokens).is_empty()
            );
        }
    }
    for path in ["spec/candidate.md", "nested/Cargo.toml", "nested/deny.toml"] {
        assert!(!findings(path, dependency.as_bytes(), &tokens).is_empty());
    }
}

#[test]
#[trace("TC-119", "FR-017-AC-5", "FR-017-CON-3")]
fn gmp_annex_11_urls_are_admitted_only_at_their_member_paths_and_query() {
    let tokens: Vec<String> = Vec::new();
    let pdf = [
        "https:",
        "//health.ec.europa.eu/document/download/8d305550-dd22-4dad-8463-2ddb4a1345f1_en",
        "?filename=annex11_01-2011_en.pdf",
    ]
    .concat();
    let notice = ["https:", "//commission.europa.eu/legal-notice_en"].concat();
    let pdf_without_query = pdf.split('?').next().expect("split yields one part");
    let pdf_altered_query = format!("{pdf}-changed");
    let annex_paths = [
        "spec_clauses_eu_law/clauses/eu-eu-gmp-annex-11-2011-01.json",
        "package/clauses/eu-eu-gmp-annex-11-2011-01.json",
    ];
    let other_paths = [
        "spec_clauses_eu_law/clauses/eu-mdr-2017-745-2026-07-19.json",
        "package/clauses/eu-gdpr-2016-679-2016-05-04.json",
        "candidate.json",
    ];

    for path in annex_paths {
        for accepted in [&pdf, &notice] {
            assert!(
                findings(path, accepted.as_bytes(), &tokens).is_empty(),
                "{path} must admit {accepted}"
            );
        }
        for refused in [pdf_without_query, pdf_altered_query.as_str()] {
            assert!(
                !findings(path, refused.as_bytes(), &tokens).is_empty(),
                "{path} must refuse {refused}"
            );
        }
    }
    for path in other_paths {
        for refused in [&pdf, &notice] {
            assert!(
                !findings(path, refused.as_bytes(), &tokens).is_empty(),
                "{path} must refuse {refused}"
            );
        }
    }
}
