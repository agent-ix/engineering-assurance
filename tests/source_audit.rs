// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Repository and adverse coverage for the parsed Rust source audit.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use engineering_assurance::source_audit::{
    MAX_RUST_SOURCE_BYTES, RustSourceAuditError, RustSourceAuditRole, RustSourceCapability,
    RustSourceFinding, RustSourceFindingCategory, audit_rust_source,
};
use ix_trace_rs::trace;
use syn::Item;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", directory.display()))
        .collect::<Result<_, _>>()
        .unwrap_or_else(|error| panic!("cannot enumerate {}: {error}", directory.display()));
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let file_type = entry
            .file_type()
            .unwrap_or_else(|error| panic!("cannot classify {}: {error}", entry.path().display()));
        if file_type.is_dir() {
            rust_files(&entry.path(), files);
        } else if file_type.is_file() && entry.path().extension().is_some_and(|value| value == "rs")
        {
            files.push(entry.path());
        }
    }
}

fn library_module_files() -> Vec<PathBuf> {
    let mut pending = vec![repository_root().join("src/lib.rs")];
    let mut selected = BTreeSet::new();
    while let Some(path) = pending.pop() {
        if !selected.insert(path.clone()) {
            continue;
        }
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let syntax = syn::parse_file(&source)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()));
        let base = if matches!(
            path.file_name().and_then(|name| name.to_str()),
            Some("lib.rs" | "mod.rs")
        ) {
            path.parent()
                .expect("module source has a parent")
                .to_owned()
        } else {
            path.with_extension("")
        };
        enqueue_external_modules(&syntax.items, &base, &path, &mut pending);
    }
    selected.into_iter().collect()
}

fn enqueue_external_modules(
    items: &[Item],
    base: &Path,
    source_path: &Path,
    pending: &mut Vec<PathBuf>,
) {
    for module in items.iter().filter_map(|item| match item {
        Item::Mod(module) => Some(module),
        _ => None,
    }) {
        assert!(
            !module
                .attrs
                .iter()
                .any(|attribute| attribute.path().is_ident("path")),
            "explicit #[path] module {} in {} must be added to the closed module resolver",
            module.ident,
            source_path.display()
        );
        let name = module.ident.to_string();
        if let Some((_, nested)) = &module.content {
            enqueue_external_modules(nested, &base.join(name), source_path, pending);
            continue;
        }
        let candidates = [
            base.join(format!("{name}.rs")),
            base.join(name).join("mod.rs"),
        ];
        let resolved: Vec<_> = candidates
            .into_iter()
            .filter(|item| item.is_file())
            .collect();
        assert_eq!(
            resolved.len(),
            1,
            "module {} in {} must resolve to exactly one source",
            module.ident,
            source_path.display()
        );
        pending.push(resolved[0].clone());
    }
}

fn categories(source: &str, role: RustSourceAuditRole) -> Vec<RustSourceFindingCategory> {
    audit_rust_source(source.as_bytes(), role)
        .expect("synthetic Rust source must parse")
        .into_iter()
        .map(|finding| finding.category())
        .collect()
}

fn synthetic_source(template: &str) -> String {
    template.replace("TRACE", "trace")
}

#[test]
#[trace("TC-086", "FR-012-AC-8")]
#[trace("TC-101", "FR-014-AC-4", "FR-014-CON-1", "FR-014-CON-2")]
fn tc_101_every_library_module_is_capability_confined() {
    let producer_execution = repository_root().join("src/producer_execution.rs");
    for path in library_module_files() {
        if path == producer_execution {
            continue;
        }
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let findings = audit_rust_source(&bytes, RustSourceAuditRole::ReusableLibrary)
            .unwrap_or_else(|error| panic!("cannot audit {}: {error}", path.display()));
        assert!(findings.is_empty(), "{}: {findings:?}", path.display());
    }
}

#[test]
#[trace("TC-101", "FR-014-AC-4", "FR-014-CON-1", "FR-014-CON-2")]
fn tc_101_aliases_are_resolved_while_comments_and_literals_are_inert() {
    let safe = r#"
        // std::process::Command is prose, not a capability.
        const DESCRIPTION: &str = "std::fs::read and println!";
        fn classify() -> &'static str { DESCRIPTION }
    "#;
    assert!(
        audit_rust_source(safe.as_bytes(), RustSourceAuditRole::ReusableLibrary)
            .expect("valid source")
            .is_empty()
    );

    let aliased = r#"
        use std as platform;
        use std::process::Command as Runner;
        fn inspect() { let _ = platform::fs::read("candidate"); }
    "#;
    let findings = audit_rust_source(aliased.as_bytes(), RustSourceAuditRole::ReusableLibrary)
        .expect("valid source");
    let capabilities: BTreeSet<_> = findings
        .iter()
        .filter_map(RustSourceFinding::capability)
        .collect();
    assert_eq!(
        capabilities,
        [
            RustSourceCapability::Filesystem,
            RustSourceCapability::ChildProgram,
        ]
        .into_iter()
        .collect()
    );

    let scoped = r#"
        mod alias_owner {
            use std as platform;
            fn inspect() { let _ = platform::fs::read("candidate"); }
        }
        mod independent {
            struct platform;
            impl platform { fn fs() {} }
            fn inspect() { platform::fs(); }
        }
    "#;
    let findings = audit_rust_source(scoped.as_bytes(), RustSourceAuditRole::ReusableLibrary)
        .expect("valid source");
    assert_eq!(
        findings
            .iter()
            .filter_map(RustSourceFinding::capability)
            .collect::<Vec<_>>(),
        [RustSourceCapability::Filesystem]
    );
}

#[test]
#[trace("TC-101", "FR-014-AC-4")]
fn tc_101_only_exact_cargo_package_metadata_environment_macros_are_admitted() {
    let admitted = r#"
        const NAME: &str = env!("CARGO_PKG_NAME");
        const VERSION: &str = env!("CARGO_PKG_VERSION");
    "#;
    assert!(
        audit_rust_source(admitted.as_bytes(), RustSourceAuditRole::ReusableLibrary)
            .expect("valid source")
            .is_empty()
    );

    for rejected in [
        r#"const VALUE: &str = env!("HOME");"#,
        r#"const VALUE: Option<&str> = option_env!("TOKEN");"#,
        r#"fn emit() { println!("result"); }"#,
        r"use std::println as emit;",
    ] {
        assert_eq!(
            audit_rust_source(rejected.as_bytes(), RustSourceAuditRole::ReusableLibrary,)
                .expect("valid source")
                .len(),
            1
        );
    }
}

#[test]
#[trace("TC-101", "FR-014-AC-4")]
fn tc_101_findings_are_ordered_by_function_category_and_capability() {
    let source = r#"
        fn zeta() { let _ = std::fs::read("candidate"); }
        fn alpha() { let _ = std::env::var("TOKEN"); }
    "#;
    let findings = audit_rust_source(source.as_bytes(), RustSourceAuditRole::ReusableLibrary)
        .expect("valid source");
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].function(), Some("alpha"));
    assert_eq!(
        findings[0].capability(),
        Some(RustSourceCapability::Environment)
    );
    assert_eq!(findings[1].function(), Some("zeta"));
    assert_eq!(
        findings[1].capability(),
        Some(RustSourceCapability::Filesystem)
    );
}

#[test]
#[trace("TC-117", "NFR-005-AC-2")]
fn tc_117_every_first_party_rust_test_uses_canonical_trace_syntax() {
    let root = repository_root();
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    rust_files(&root.join("tests"), &mut files);
    files.sort();
    for path in files {
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let findings = audit_rust_source(&bytes, RustSourceAuditRole::RequirementTests)
            .unwrap_or_else(|error| panic!("cannot audit {}: {error}", path.display()));
        assert!(findings.is_empty(), "{}: {findings:?}", path.display());
    }
}

#[test]
#[trace("TC-117", "NFR-005-AC-2")]
fn tc_117_missing_aliased_qualified_and_malformed_traces_fail_closed() {
    let qualified = synthetic_source(
        r#"
        #[test]
        #[ix_trace_rs::TRACE("TC-117", "NFR-005-AC-2")]
        fn qualified() {}
    "#,
    );
    assert_eq!(
        categories(&qualified, RustSourceAuditRole::RequirementTests),
        [
            RustSourceFindingCategory::TraceImportMissing,
            RustSourceFindingCategory::TraceAttributeQualified,
            RustSourceFindingCategory::TestTraceMissing,
        ]
    );

    let aliased = synthetic_source(
        r#"
        use ix_trace_rs::TRACE as tracked;
        #[test]
        #[tracked("TC-117", "NFR-005-AC-2")]
        fn aliased() {}
    "#,
    );
    assert_eq!(
        categories(&aliased, RustSourceAuditRole::RequirementTests),
        [
            RustSourceFindingCategory::TraceImportMissing,
            RustSourceFindingCategory::TraceImportAliased,
            RustSourceFindingCategory::TestTraceMissing,
        ]
    );

    let grouped_import = synthetic_source(
        r#"
        use ix_trace_rs::{TRACE};
        #[test]
        #[TRACE("TC-117", "NFR-005-AC-2")]
        fn grouped_import() {}
    "#,
    );
    assert_eq!(
        categories(&grouped_import, RustSourceAuditRole::RequirementTests),
        [RustSourceFindingCategory::TraceImportMissing]
    );

    let malformed = synthetic_source(
        r"
        use ix_trace_rs::TRACE;
        #[test]
        #[TRACE(TC_117)]
        fn malformed() {}
    ",
    );
    assert_eq!(
        categories(&malformed, RustSourceAuditRole::RequirementTests),
        [
            RustSourceFindingCategory::TraceArgumentsInvalid,
            RustSourceFindingCategory::TraceTestCaseMissing,
            RustSourceFindingCategory::TraceAcceptanceCriterionMissing,
        ]
    );
}

#[test]
#[trace("TC-117", "NFR-005-AC-2")]
fn tc_117_trace_requires_both_test_case_and_acceptance_identifiers() {
    let missing_test = synthetic_source(
        r#"
        use ix_trace_rs::TRACE;
        #[test]
        #[TRACE("NFR-005-AC-2")]
        fn missing_test() {}
    "#,
    );
    assert_eq!(
        categories(&missing_test, RustSourceAuditRole::RequirementTests),
        [RustSourceFindingCategory::TraceTestCaseMissing]
    );

    let missing_acceptance = synthetic_source(
        r#"
        use ix_trace_rs::TRACE;
        #[test]
        #[TRACE("TC-117", "not-an-acceptance-id")]
        fn missing_acceptance() {}
    "#,
    );
    assert_eq!(
        categories(&missing_acceptance, RustSourceAuditRole::RequirementTests),
        [RustSourceFindingCategory::TraceAcceptanceCriterionMissing]
    );
}

#[test]
#[trace("TC-101", "FR-014-AC-4")]
#[trace("TC-117", "NFR-005-AC-2")]
fn source_size_encoding_and_syntax_boundaries_refuse_before_inspection() {
    let exact = vec![b' '; MAX_RUST_SOURCE_BYTES];
    assert!(
        audit_rust_source(&exact, RustSourceAuditRole::ReusableLibrary)
            .expect("exact source ceiling is admitted")
            .is_empty()
    );
    let oversized = audit_rust_source(
        &vec![b' '; MAX_RUST_SOURCE_BYTES + 1],
        RustSourceAuditRole::ReusableLibrary,
    )
    .expect_err("over-limit source must be refused");
    assert!(matches!(
        oversized,
        RustSourceAuditError::SourceTooLarge { .. }
    ));
    assert_eq!(oversized.code(), "rust_source_too_large");

    let invalid_utf8 = audit_rust_source(&[0xff], RustSourceAuditRole::ReusableLibrary)
        .expect_err("non-UTF-8 source must be refused");
    assert!(matches!(invalid_utf8, RustSourceAuditError::InvalidUtf8(_)));
    assert_eq!(invalid_utf8.code(), "rust_source_not_utf8");

    let invalid_syntax = audit_rust_source(b"fn incomplete(", RustSourceAuditRole::ReusableLibrary)
        .expect_err("invalid syntax must be refused");
    assert!(matches!(
        invalid_syntax,
        RustSourceAuditError::InvalidSyntax(_)
    ));
    assert_eq!(invalid_syntax.code(), "rust_source_syntax_invalid");
}

/// Repository-relative source that actually opens and reads the pinned corpus.
///
/// The pure index module `src/compatibility_corpus.rs` is already covered by
/// the library capability audit above, but it performs no I/O by construction,
/// so holding *it* to a capability contract proves nothing that its type
/// signatures do not already prove. The reader below is where directories are
/// opened and bytes are read, and it is the only place a corpus write, a
/// socket, or a persistence handle could actually appear.
const CORPUS_READER: &str = "tests/compatibility_corpus.rs";

/// Identifiers that would mutate, replace, or remove something on disk.
///
/// Matched against parsed syntax rather than raw text, so prose in a doc
/// comment that happens to say "writes nothing" cannot satisfy or trip the
/// check, and a capability introduced inside a string literal cannot hide.
const MUTATING_IDENTIFIERS: [&str; 14] = [
    "OpenOptions",
    "create",
    "create_dir",
    "create_dir_all",
    "create_new",
    "hard_link",
    "remove_dir",
    "remove_dir_all",
    "remove_file",
    "rename",
    "set_len",
    "set_permissions",
    "symlink",
    "write_all",
];

#[derive(Default)]
struct NamedIdentifiers(BTreeSet<String>);

impl<'ast> syn::visit::Visit<'ast> for NamedIdentifiers {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        for segment in &path.segments {
            self.0.insert(segment.ident.to_string());
        }
        syn::visit::visit_path(self, path);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        self.0.insert(call.method.to_string());
        syn::visit::visit_expr_method_call(self, call);
    }
}

#[test]
#[trace("TC-076", "FR-011-AC-8", "FR-011-CON-1", "FR-011-CON-4")]
fn tc_076_the_corpus_reader_holds_a_bounded_capability_contract() {
    // What this audit proves: the source that reads the corpus reaches for no
    // network, persistence, clock, or diagnostic-output capability, and names
    // no identifier that would create, replace, truncate, or delete a file.
    //
    // What it does not prove: that the reader behaves read-only at run time.
    // Static inspection cannot see through a helper, and a blanket capability
    // ban would be dishonest here anyway — this reader legitimately holds a
    // confined directory capability, reads metadata, canonicalizes one root,
    // and runs `git` to check the corpus gitlink. The behavioral half of the
    // property is TC-076 in `tests/compatibility_corpus.rs`, which enumerates
    // the corpus before and after mapping it and refuses any changed byte. The
    // two halves are complementary: this one fails when a capability appears in
    // the source, that one fails when a byte moves.
    let path = repository_root().join(CORPUS_READER);
    let bytes = fs::read(&path)
        .unwrap_or_else(|error| panic!("the corpus reader must be readable: {error}"));

    let capabilities: BTreeSet<_> = audit_rust_source(&bytes, RustSourceAuditRole::ReusableLibrary)
        .unwrap_or_else(|error| panic!("cannot audit {}: {error}", path.display()))
        .iter()
        .filter_map(RustSourceFinding::capability)
        .collect();

    // A reader that reached for neither a filesystem nor a child program is not
    // this reader. Asserting the capabilities it is supposed to have keeps the
    // exclusions below from passing against a file that was renamed, emptied,
    // or reduced to re-exports, which is the failure mode that let this
    // property be checked where it could not fail in the first place.
    for expected in [
        RustSourceCapability::Filesystem,
        RustSourceCapability::ChildProgram,
    ] {
        assert!(
            capabilities.contains(&expected),
            "{CORPUS_READER} no longer reads the corpus directly: {capabilities:?}"
        );
    }

    // The corpus is evidence. A reader that could open a socket could reach a
    // source the retained bytes were supposed to replace; one that could open a
    // persistence handle could carry state between runs; a clock would make the
    // qualification non-deterministic. None of the three is needed to read a
    // pinned directory, so none of the three is admitted.
    for forbidden in [
        RustSourceCapability::Network,
        RustSourceCapability::Persistence,
        RustSourceCapability::Clock,
        RustSourceCapability::Output,
    ] {
        assert!(
            !capabilities.contains(&forbidden),
            "{CORPUS_READER} reaches for {forbidden:?}"
        );
    }

    let source = std::str::from_utf8(&bytes).expect("the corpus reader must be UTF-8");
    let syntax = syn::parse_file(source).expect("the corpus reader must parse");
    let mut named = NamedIdentifiers::default();
    syn::visit::Visit::visit_file(&mut named, &syntax);
    for identifier in MUTATING_IDENTIFIERS {
        assert!(
            !named.0.contains(identifier),
            "{CORPUS_READER} names the mutating operation {identifier}"
        );
    }

    // The identifier list has to be able to fire, or it is decoration. A source
    // that does write is rejected by the same walk that clears the reader.
    let mut mutating = NamedIdentifiers::default();
    syn::visit::Visit::visit_file(
        &mut mutating,
        &syn::parse_file(r#"fn edit() { std::fs::remove_file("corpus/corpus.json").ok(); }"#)
            .expect("mutant must parse"),
    );
    assert!(
        MUTATING_IDENTIFIERS
            .iter()
            .any(|identifier| mutating.0.contains(*identifier)),
        "a writing mutant escaped the identifier walk"
    );
}
