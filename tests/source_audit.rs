// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Repository and adverse coverage for the parsed Rust source audit.

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use engineering_assurance::source_audit::{
    MAX_RUST_SOURCE_BYTES, RustSourceAuditError, RustSourceAuditRole, RustSourceCapability,
    RustSourceFinding, RustSourceFindingCategory, TraceGrammar, audit_rust_source,
};
use ix_trace_rs::trace;
use syn::{Item, UseTree, visit::Visit};

const ATTRIBUTE_TESTS: RustSourceAuditRole = RustSourceAuditRole::RequirementTests {
    grammar: TraceGrammar::Attribute,
};

const DOC_COMMENT_TESTS: RustSourceAuditRole = RustSourceAuditRole::RequirementTests {
    grammar: TraceGrammar::DocComment,
};

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

fn direct_dependencies_for(features: &[&str]) -> BTreeSet<String> {
    let mut command = Command::new(env!("CARGO"));
    command.args([
        "tree",
        "--offline",
        "--no-default-features",
        "--edges",
        "normal",
        "--depth",
        "1",
        "--prefix",
        "none",
        "--target",
        "all",
    ]);
    command
        .arg("--manifest-path")
        .arg(repository_root().join("Cargo.toml"));
    for feature in features {
        command.args(["--features", feature]);
    }
    let output = command.output().expect("Cargo dependency tree must launch");
    assert!(
        output.status.success(),
        "Cargo dependency tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("Cargo dependency tree must be UTF-8")
        .lines()
        .skip(1)
        .map(|line| {
            line.split_whitespace()
                .next()
                .expect("dependency tree row must have a crate name")
                .to_owned()
        })
        .collect()
}

/// The direct dependencies in the package's `full` graph, including feature edges
/// and dependencies enabled for any target.
fn full_feature_direct_dependencies() -> BTreeSet<String> {
    let root = repository_root();
    let output = Command::new(env!("CARGO"))
        .args([
            "tree",
            "--offline",
            "--locked",
            "--no-default-features",
            "--features",
            "full",
            "--edges",
            "normal,features",
            "--target",
            "all",
            "--manifest-path",
        ])
        .arg(root.join("Cargo.toml"))
        .output()
        .expect("Cargo full dependency tree must launch");
    assert!(
        output.status.success(),
        "Cargo full dependency tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8(output.stdout).expect("Cargo full dependency tree must be UTF-8");
    direct_package_nodes(&tree)
}

fn direct_package_nodes(tree: &str) -> BTreeSet<String> {
    let mut lines = tree.lines();
    let root = lines.next().expect("Cargo dependency tree has a root");
    assert!(
        root.starts_with("engineering-assurance v"),
        "Cargo dependency tree root must be engineering-assurance: {root}"
    );

    lines
        .filter_map(|line| {
            let edge = line
                .strip_prefix("├── ")
                .or_else(|| line.strip_prefix("└── "))?;
            let mut fields = edge.split_whitespace();
            let name = fields.next()?;
            let kind = fields.next()?;
            (kind.starts_with('v') || kind == "feature").then(|| name.to_owned())
        })
        .collect()
}

const NFR_010_ALLOWED_DIRECT_DELEGATES: [&str; 3] =
    ["sigstore-sign", "sigstore-types", "sigstore-verify"];

fn signature_or_crypto_dependency_violations(
    direct_dependencies: &BTreeSet<String>,
) -> BTreeSet<String> {
    let allowed_delegates = NFR_010_ALLOWED_DIRECT_DELEGATES
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    direct_dependencies
        .iter()
        .filter(|name| {
            let normalized = name.replace('_', "-").to_ascii_lowercase();
            let is_delegate_namespace =
                normalized.starts_with("sigstore-") || normalized.starts_with("in-toto");
            let is_crypto_primitive = [
                "crypto",
                "signature",
                "signing",
                "signer",
                "certificate",
                "cert",
                "x509",
                "openssl",
                "aws-lc",
                "boring",
                "ring",
                "rsa",
                "ed25519",
                "curve25519",
                "ecdsa",
                "p256",
                "k256",
                "hmac",
                "blake3",
                "md5",
                "md-5",
                "sha1",
                "sha2",
                "sha3",
                "digest",
                "rustls",
                "cms",
                "spki",
                "pkcs",
                "der",
                "pem",
                "zeroize",
            ]
            .iter()
            .any(|marker| normalized == *marker || normalized.starts_with(&format!("{marker}-")));
            (is_delegate_namespace || is_crypto_primitive) && !allowed_delegates.contains(*name)
        })
        .cloned()
        .collect()
}

#[derive(Default)]
struct SignatureVerifierAudit {
    functions: Vec<String>,
    resolver_calls: Vec<(String, String)>,
    tool_calls: Vec<(String, String)>,
    process_host_calls: Vec<String>,
    gitsign_verdict_calls: Vec<String>,
    gitsign_run_calls: Vec<String>,
    cms_branch_functions: BTreeSet<String>,
    direct_process_api: bool,
    dependency_roots: BTreeSet<String>,
    imported_roots: BTreeSet<String>,
    external_roots: BTreeSet<String>,
}

impl SignatureVerifierAudit {
    fn function(&self) -> &str {
        self.functions.last().map_or("<module>", String::as_str)
    }
}

impl<'ast> Visit<'ast> for SignatureVerifierAudit {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.functions.push(item.sig.ident.to_string());
        syn::visit::visit_item_fn(self, item);
        self.functions.pop();
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.functions.push(item.sig.ident.to_string());
        syn::visit::visit_impl_item_fn(self, item);
        self.functions.pop();
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if item.attrs.iter().any(|attribute| {
            attribute.path().is_ident("cfg")
                && matches!(
                    &attribute.meta,
                    syn::Meta::List(list) if list.tokens.to_string().contains("test")
                )
        }) {
            return;
        }
        syn::visit::visit_item_mod(self, item);
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.inspect_use_tree(&item.tree, &mut Vec::new());
        syn::visit::visit_item_use(self, item);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        let segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        if segments.starts_with(&["std".to_owned(), "process".to_owned()]) {
            self.direct_process_api = true;
        }
        if segments.ends_with(&["SignatureFormat".to_owned(), "Cms".to_owned()]) {
            self.cms_branch_functions.insert(self.function().to_owned());
        }
        if let Some(root) = segments.first() {
            self.add_external_root(root.clone());
        }
        syn::visit::visit_path(self, path);
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = call.func.as_ref() {
            let name = function
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string());
            if function
                .path
                .segments
                .iter()
                .any(|segment| segment.ident == "process_host")
                && let Some(name) = name.as_ref()
            {
                self.process_host_calls.push(name.clone());
            }
            let argument = call.args.first().and_then(|argument| match argument {
                syn::Expr::Lit(literal) => match &literal.lit {
                    syn::Lit::Str(value) => Some(value.value()),
                    _ => None,
                },
                syn::Expr::Path(path) => path.path.get_ident().map(ToString::to_string),
                _ => None,
            });
            match (name.as_deref(), argument) {
                (Some("resolve_program"), Some(program)) => {
                    self.resolver_calls
                        .push((self.function().to_owned(), program));
                }
                (Some("run_tool"), Some(program)) => {
                    self.tool_calls.push((self.function().to_owned(), program));
                }
                _ => {}
            }
        }
        syn::visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        match call.method.to_string().as_str() {
            "gitsign_verdict" => self.gitsign_verdict_calls.push(self.function().to_owned()),
            "gitsign_run" => self.gitsign_run_calls.push(self.function().to_owned()),
            _ => {}
        }
        syn::visit::visit_expr_method_call(self, call);
    }
}

impl SignatureVerifierAudit {
    fn inspect_use_tree(&mut self, tree: &UseTree, prefix: &mut Vec<String>) {
        match tree {
            UseTree::Path(path) => {
                if prefix.is_empty() {
                    self.imported_roots.insert(path.ident.to_string());
                    self.add_external_root(path.ident.to_string());
                }
                prefix.push(path.ident.to_string());
                self.inspect_process_prefix(prefix);
                self.inspect_use_tree(&path.tree, prefix);
                prefix.pop();
            }
            UseTree::Name(name) => {
                if prefix.is_empty() {
                    self.imported_roots.insert(name.ident.to_string());
                    self.add_external_root(name.ident.to_string());
                }
                prefix.push(name.ident.to_string());
                self.inspect_process_prefix(prefix);
                prefix.pop();
            }
            UseTree::Rename(rename) => {
                if prefix.is_empty() {
                    self.imported_roots.insert(rename.ident.to_string());
                    self.add_external_root(rename.ident.to_string());
                }
                prefix.push(rename.ident.to_string());
                self.inspect_process_prefix(prefix);
                prefix.pop();
            }
            UseTree::Group(group) => {
                for item in &group.items {
                    self.inspect_use_tree(item, prefix);
                }
            }
            UseTree::Glob(_) => self.inspect_process_prefix(prefix),
        }
    }

    fn inspect_process_prefix(&mut self, prefix: &[String]) {
        if prefix.starts_with(&["std".to_owned(), "process".to_owned()]) {
            self.direct_process_api = true;
        }
    }

    fn add_external_root(&mut self, root: String) {
        if self.dependency_roots.contains(&root) || root == "engineering_assurance" {
            self.external_roots.insert(root);
        }
    }
}

fn repository_source(relative_path: &str) -> String {
    let path = repository_root().join(relative_path);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    syn::parse_file(&source)
        .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()));
    source
}

fn assert_import_roots_are_known(imported: &BTreeSet<String>, dependencies: &BTreeSet<String>) {
    let standard_or_local = ["alloc", "core", "crate", "self", "std", "super"]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let unknown = imported
        .difference(&standard_or_local)
        .filter(|root| *root != "engineering_assurance" && !dependencies.contains(*root))
        .cloned()
        .collect::<BTreeSet<_>>();
    assert!(
        unknown.is_empty(),
        "unrecognized imported crate roots must be reviewed explicitly: {unknown:?}"
    );
}

fn gitsign_program_constant(source: &syn::File) -> Option<String> {
    source.items.iter().find_map(|item| match item {
        Item::Const(item) if item.ident == "GITSIGN_PROGRAM" => match &*item.expr {
            syn::Expr::Lit(value) => match &value.lit {
                syn::Lit::Str(value) => Some(value.value()),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    })
}

fn use_tree_imports_child_program(tree: &UseTree, prefix: &mut Vec<String>) -> bool {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            let found = use_tree_imports_child_program(&path.tree, prefix);
            prefix.pop();
            found
        }
        UseTree::Name(name) => {
            let mut path = prefix.clone();
            path.push(name.ident.to_string());
            path == ["std", "process"]
                || (path.starts_with(&["std".to_owned(), "process".to_owned()])
                    && matches!(
                        path.last().map(String::as_str),
                        Some("Command" | "Child" | "Stdio")
                    ))
        }
        UseTree::Rename(rename) => {
            let mut path = prefix.clone();
            path.push(rename.ident.to_string());
            path == ["std"]
                || path == ["std", "self"]
                || path == ["std", "process"]
                || (path.starts_with(&["std".to_owned(), "process".to_owned()])
                    && matches!(
                        path.last().map(String::as_str),
                        Some("Command" | "Child" | "Stdio")
                    ))
        }
        UseTree::Group(group) => group
            .items
            .iter()
            .any(|item| use_tree_imports_child_program(item, prefix)),
        UseTree::Glob(_) => prefix == &["std", "process"],
    }
}

#[derive(Default)]
struct ChildProgramUse {
    found: bool,
    capture_calls: usize,
    paths: Vec<Vec<String>>,
}

impl<'ast> Visit<'ast> for ChildProgramUse {
    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.found |= use_tree_imports_child_program(&item.tree, &mut Vec::new());
        syn::visit::visit_item_use(self, item);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        let segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        if segments.starts_with(&["std".to_owned(), "process".to_owned()])
            && segments
                .get(2)
                .is_some_and(|name| matches!(name.as_str(), "Command" | "Child" | "Stdio"))
        {
            self.found = true;
        }
        self.paths.push(segments);
        syn::visit::visit_path(self, path);
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = call.func.as_ref()
            && function.path.is_ident("capture_command")
        {
            self.capture_calls += 1;
        }
        syn::visit::visit_expr_call(self, call);
    }
}

#[test]
#[trace("TC-101", "FR-014-AC-4", "FR-014-CON-1", "FR-014-CON-2")]
fn tc_101_every_library_module_is_capability_confined() {
    // FR-014-AC-4 admits the FR-019 module wholesale (it has its own check
    // below); the FR-027 module is admitted for filesystem access only.
    let producer_execution = repository_root().join("src/producer_execution.rs");
    let filesystem_only = ["src/atomic_publish.rs", "src/integrator_package_store.rs"]
        .map(|module| repository_root().join(module));
    for path in library_module_files() {
        if path == producer_execution {
            continue;
        }
        let bytes = fs::read(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let mut findings = audit_rust_source(&bytes, RustSourceAuditRole::ReusableLibrary)
            .unwrap_or_else(|error| panic!("cannot audit {}: {error}", path.display()));
        if filesystem_only.contains(&path) {
            findings.retain(|finding| finding.capability != Some(RustSourceCapability::Filesystem));
        }
        assert!(findings.is_empty(), "{}: {findings:?}", path.display());
    }
}

#[trace(
    "TC-127",
    "FR-014-AC-4",
    "FR-019-AC-5",
    "FR-019-CON-2",
    "FR-019-CON-3",
    "FR-027-CON-1",
    "NFR-004-AC-2"
)]
#[test]
fn tc_127_producer_execution_owns_the_only_first_party_process_capability() {
    let root = repository_root();
    let producer_path = root.join("src/producer_execution.rs");
    let mut sources = Vec::new();
    rust_files(&root.join("src"), &mut sources);
    for path in sources.iter().filter(|path| **path != producer_path) {
        let source = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let syntax = syn::parse_file(&source)
            .unwrap_or_else(|error| panic!("cannot parse {}: {error}", path.display()));
        let mut process_use = ChildProgramUse::default();
        process_use.visit_file(&syntax);
        assert!(
            !process_use.found,
            "{} owns a child-program capability outside producer execution",
            path.display()
        );
    }

    let producer = fs::read_to_string(&producer_path)
        .expect("producer-execution source must remain inspectable");
    let compatibility = fs::read_to_string(root.join("src/process_host.rs"))
        .expect("legacy host compatibility facade must remain inspectable");
    let producer_syntax = syn::parse_file(&producer).expect("producer source must be valid Rust");
    let compatibility_syntax =
        syn::parse_file(&compatibility).expect("compatibility facade must be valid Rust");
    let mut producer_ownership = ChildProgramUse::default();
    producer_ownership.visit_file(&producer_syntax);
    let mut compatibility_ownership = ChildProgramUse::default();
    compatibility_ownership.visit_file(&compatibility_syntax);
    assert_eq!(producer_ownership.capture_calls, 2);
    assert!(compatibility_ownership.paths.iter().any(|path| {
        path.windows(2)
            .any(|pair| pair == ["producer_execution", "__private"])
    }));
    for path in &producer_ownership.paths {
        assert!(
            !matches!(
                path.as_slice(),
                [crate_name, parser, ..]
                    if crate_name == "serde_json"
                        && matches!(parser.as_str(), "from_reader" | "from_slice" | "from_str")
            ) && !matches!(
                path.first().map(String::as_str),
                Some("rusqlite" | "sqlx" | "quire")
            ),
            "producer execution contains a forbidden parser, persistence, or Quoin path: {path:?}"
        );
    }
    assert!(
        !root.join("src/producer_execution/Cargo.toml").exists(),
        "producer execution must remain in the existing crate"
    );

    for mutant in [
        "use std::process::Command; fn escape() { let _ = Command::new(\"tool\"); }",
        "mod nested { use std::process::Command as Runner; fn escape() { let _ = Runner::new(\"tool\"); } }",
        "use std as platform; fn escape() { let _ = platform::process::Command::new(\"tool\"); }",
    ] {
        let syntax = syn::parse_file(mutant).expect("ownership mutant must be valid Rust");
        let mut process_use = ChildProgramUse::default();
        process_use.visit_file(&syntax);
        assert!(process_use.found, "child-process ownership mutant escaped");
    }
    let safe = syn::parse_file(
        "use std::process::ExitCode; fn complete() -> ExitCode { ExitCode::SUCCESS }",
    )
    .expect("safe process-status source must be valid Rust");
    let mut process_use = ChildProgramUse::default();
    process_use.visit_file(&safe);
    assert!(!process_use.found, "process status is not child execution");
}

#[test]
#[trace("TC-653", "FR-036-AC-12")]
fn tc_653_signature_verification_delegates_to_system_verifiers() {
    audit_signature_feature_dependencies();
    audit_full_feature_direct_signature_dependencies();
    audit_pure_signature_classifier();
    audit_signature_adapter_delegation();
}

#[test]
#[trace("TC-653", "FR-036-AC-12")]
fn tc_653_rejects_a_direct_signature_primitive_dependency() {
    let forbidden = BTreeSet::from(["ring".to_owned()]);
    assert_eq!(
        signature_or_crypto_dependency_violations(&forbidden),
        forbidden,
        "direct signature and cryptographic primitives must be rejected"
    );
}

fn audit_signature_feature_dependencies() {
    let signature_dependencies = direct_dependencies_for(&["signature-verify"]);
    assert_eq!(
        signature_dependencies,
        BTreeSet::from(["serde".to_owned()]),
        "the signature-verify feature must add only its data-serialization dependency"
    );
}

fn audit_full_feature_direct_signature_dependencies() {
    let direct_dependencies = full_feature_direct_dependencies();
    let signing_dependencies = direct_dependencies
        .iter()
        .filter(|name| name.starts_with("sigstore-") || name.starts_with("in-toto"))
        .cloned()
        .collect::<BTreeSet<_>>();
    assert_eq!(
        signing_dependencies,
        NFR_010_ALLOWED_DIRECT_DELEGATES
            .into_iter()
            .map(str::to_owned)
            .collect(),
        "NFR-010 delegates signing, verification, and statement types to the specified Sigstore crates"
    );
    assert_eq!(
        signature_or_crypto_dependency_violations(&direct_dependencies),
        BTreeSet::new(),
        "NFR-010 allows cryptographic implementation behind the direct Sigstore delegates; direct signature, certificate, or crypto primitives are forbidden"
    );
}

fn audit_pure_signature_classifier() {
    let signature_dependencies = direct_dependencies_for(&["signature-verify"]);
    let pure_source = repository_source("src/signature_verify.rs");
    let pure_syntax = syn::parse_file(&pure_source).expect("classifier parses");
    let mut pure_audit = SignatureVerifierAudit {
        dependency_roots: signature_dependencies
            .iter()
            .map(|name| name.replace('-', "_"))
            .collect(),
        ..SignatureVerifierAudit::default()
    };
    pure_audit.visit_file(&pure_syntax);
    assert_import_roots_are_known(&pure_audit.imported_roots, &pure_audit.dependency_roots);
    assert_eq!(
        pure_audit.external_roots,
        BTreeSet::from(["serde".to_owned()]),
        "the classifier may depend only on its serialization crate"
    );
    assert!(
        !pure_audit.direct_process_api,
        "the signature classifier must not invoke child programs"
    );
}

fn audit_signature_adapter_delegation() {
    let full_dependency_roots = direct_dependencies_for(&["full"])
        .into_iter()
        .map(|name| name.replace('-', "_"))
        .collect::<BTreeSet<_>>();
    let adapter_source = repository_source("src/signature_verify_host.rs");
    let adapter_syntax = syn::parse_file(&adapter_source).expect("adapter parses");
    let mut adapter_audit = SignatureVerifierAudit {
        dependency_roots: full_dependency_roots.clone(),
        ..SignatureVerifierAudit::default()
    };
    adapter_audit.visit_file(&adapter_syntax);
    assert_import_roots_are_known(&adapter_audit.imported_roots, &full_dependency_roots);
    assert_eq!(
        adapter_audit.external_roots,
        BTreeSet::from([
            "engineering_assurance".to_owned(),
            "serde".to_owned(),
            "serde_json".to_owned(),
            "tempfile".to_owned(),
            "thiserror".to_owned(),
        ]),
        "the signature adapter must not import a signature or cryptography crate"
    );
    assert!(
        !adapter_audit.direct_process_api,
        "signature checks must use the bounded process host"
    );
    assert_eq!(
        adapter_audit
            .process_host_calls
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["run_configured", "run_configured_stdin"]),
        "all adapter child processes must cross the bounded process host"
    );
    assert_eq!(
        adapter_audit
            .resolver_calls
            .iter()
            .filter(|(_, program)| program.as_str() != "program")
            .map(|(_, program)| program.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["git", "ssh-keygen"]),
        "the SSH path may resolve only git and ssh-keygen"
    );
    assert!(
        adapter_audit
            .resolver_calls
            .iter()
            .filter(|(_, program)| program == "program")
            .all(|(function, _)| function == "run_tool"),
        "only the fixed gitsign/git tool adapter may resolve a supplied program"
    );
    assert_eq!(
        adapter_audit.tool_calls,
        vec![
            ("repository_sets_gitsign_key".to_owned(), "git".to_owned()),
            ("gitsign_run".to_owned(), "GITSIGN_PROGRAM".to_owned()),
        ],
        "gitsign may run only in the CMS verification procedure"
    );
    assert_eq!(
        adapter_audit.gitsign_run_calls,
        vec!["gitsign_verdict".to_owned()],
        "the gitsign process is reached only through its CMS classifier"
    );
    assert_eq!(
        adapter_audit.gitsign_verdict_calls,
        vec!["commit_verdict".to_owned(), "tag_verdict".to_owned()],
        "only the commit and tag CMS branches may delegate to gitsign"
    );
    assert_eq!(
        adapter_audit.cms_branch_functions,
        BTreeSet::from(["commit_verdict".to_owned(), "tag_verdict".to_owned()]),
        "the gitsign call sites must be restricted to explicit CMS branches"
    );
    assert_eq!(
        gitsign_program_constant(&adapter_syntax).as_deref(),
        Some("gitsign"),
        "the CMS verifier executable must be fixed to gitsign"
    );
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
        let findings = audit_rust_source(&bytes, ATTRIBUTE_TESTS)
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
        categories(&qualified, ATTRIBUTE_TESTS),
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
        categories(&aliased, ATTRIBUTE_TESTS),
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
        categories(&grouped_import, ATTRIBUTE_TESTS),
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
        categories(&malformed, ATTRIBUTE_TESTS),
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
        categories(&missing_test, ATTRIBUTE_TESTS),
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
        categories(&missing_acceptance, ATTRIBUTE_TESTS),
        [RustSourceFindingCategory::TraceAcceptanceCriterionMissing]
    );
}

#[test]
#[trace("TC-189", "FR-014-AC-6")]
fn tc_189_doc_comment_grammar_accepts_trace_lines_and_flags_absent_or_malformed_ones() {
    let conforming = r"
        /// Trace: FR-901, NFR-902
        /// Provenance: fictional#1
        #[test]
        fn tc_901_widgets_are_sorted() {}

        /// Trace: FR-903-AC-2
        #[test]
        fn tc_902_gadgets_are_counted() {}
    ";
    assert_eq!(categories(conforming, DOC_COMMENT_TESTS), []);

    let cases = [
        (
            "#[test] fn tc_903_untagged() {}",
            RustSourceFindingCategory::DocTraceMissing,
        ),
        (
            "/// Explains the case without tracing it.\n#[test] fn tc_904_prose_only() {}",
            RustSourceFindingCategory::DocTraceMissing,
        ),
        (
            "/// Trace:\n#[test] fn tc_905_empty_list() {}",
            RustSourceFindingCategory::DocTraceIdInvalid,
        ),
        (
            "/// Trace: FR-901, not an id\n#[test] fn tc_906_bad_entry() {}",
            RustSourceFindingCategory::DocTraceIdInvalid,
        ),
        (
            "/// Trace: FR-901,, NFR-902\n#[test] fn tc_907_empty_entry() {}",
            RustSourceFindingCategory::DocTraceIdInvalid,
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(
            categories(source, DOC_COMMENT_TESTS),
            [expected],
            "{source}"
        );
    }
}

#[test]
#[trace("TC-189", "FR-014-AC-6")]
fn tc_189_each_grammar_is_held_exclusively_and_import_findings_are_attribute_only() {
    let doc_only = "/// Trace: FR-901\n#[test] fn tc_908_doc_form() {}";
    // The attribute grammar does not accept the doc form, and the doc grammar
    // reports no import-shape finding for a source with no trace import.
    assert_eq!(
        categories(doc_only, ATTRIBUTE_TESTS),
        [
            RustSourceFindingCategory::TraceImportMissing,
            RustSourceFindingCategory::TestTraceMissing,
        ]
    );
    assert_eq!(categories(doc_only, DOC_COMMENT_TESTS), []);

    let attribute_only = synthetic_source(
        r#"
        use ix_trace_rs::TRACE;
        #[test]
        #[TRACE("TC-908", "FR-901-AC-1")]
        fn tc_909_attribute_form() {}
    "#,
    );
    assert_eq!(categories(&attribute_only, ATTRIBUTE_TESTS), []);
    assert_eq!(
        categories(&attribute_only, DOC_COMMENT_TESTS),
        [RustSourceFindingCategory::DocTraceMissing]
    );
}

#[test]
#[trace("TC-189", "FR-014-AC-6")]
fn tc_189_only_requirement_family_ids_satisfy_a_doc_trace() {
    for accepted in [
        "FR-901",
        "NFR-902",
        "StR-903",
        "US-904",
        "FR-901-AC-2",
        "FR-901-CON-3",
        "FR-901, NFR-902-AC-1",
        "FR-901 ,NFR-902",
    ] {
        let source = format!("/// Trace: {accepted}\n#[test] fn tc_910_accepted() {{}}");
        assert_eq!(categories(&source, DOC_COMMENT_TESTS), [], "{accepted}");
    }
    for rejected in [
        "TODO-LATER",
        "X-X",
        "TC-901",
        "fr-901",
        "FR-",
        "FR-9x",
        "FR-901-AC",
        "FR-901-XX-1",
        "FR-901-AC-1-2",
        "FR-901,",
        "FR-901, TC-901",
    ] {
        let source = format!("/// Trace: {rejected}\n#[test] fn tc_911_rejected() {{}}");
        assert_eq!(
            categories(&source, DOC_COMMENT_TESTS),
            [RustSourceFindingCategory::DocTraceIdInvalid],
            "{rejected}"
        );
    }
}

#[test]
#[trace("TC-189", "FR-014-AC-6")]
fn tc_189_doc_trace_carriers_are_outer_docs_and_are_read_line_by_line() {
    let accepted = [
        "#[doc = \" Trace: FR-901\"]\n#[test] fn tc_912_doc_attribute() {}",
        "/** Trace: FR-901 */\n#[test] fn tc_913_one_line_block() {}",
        "/**\n * Summary of the case.\n * Trace: FR-901, NFR-902\n */\n#[test] fn tc_914_block() {}",
        "/**\n * Trace: FR-901\n * prose on its own line is not part of the list\n */\n#[test] fn tc_917_prose_after_block_trace() {}",
        "/// Trace: FR-901\n/// Trace: NFR-902\n#[test] fn tc_915_two_lines() {}",
        "/// Trace: FR-901\n#[test]\n#[trace(\"TC-901\", \"FR-901-AC-1\")]\nfn tc_916_both_forms() {}",
    ];
    for source in accepted {
        assert_eq!(categories(source, DOC_COMMENT_TESTS), [], "{source}");
    }

    let cases = [
        (
            "/// Trace: FR-901\n/// Trace: TODO-LATER\n#[test] fn tc_918_one_bad_line() {}",
            RustSourceFindingCategory::DocTraceIdInvalid,
        ),
        (
            "#[test] fn tc_919_inner_doc() {\n    //! Trace: FR-901\n}",
            RustSourceFindingCategory::DocTraceMissing,
        ),
        (
            "/// trace: FR-901\n#[test] fn tc_920_lowercase_key() {}",
            RustSourceFindingCategory::DocTraceMissing,
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(
            categories(source, DOC_COMMENT_TESTS),
            [expected],
            "{source}"
        );
    }
}

#[test]
#[trace("TC-189", "FR-014-AC-6")]
fn tc_189_doc_grammar_reports_no_import_finding_for_an_aliased_trace_import() {
    let source = synthetic_source(
        "use ix_trace_rs::TRACE as tracked;\n/// Trace: FR-901\n#[test] fn tc_921_aliased() {}",
    );
    assert_eq!(categories(&source, DOC_COMMENT_TESTS), []);
    assert_eq!(
        categories(&source, ATTRIBUTE_TESTS),
        [
            RustSourceFindingCategory::TraceImportMissing,
            RustSourceFindingCategory::TraceImportAliased,
            RustSourceFindingCategory::TestTraceMissing,
        ]
    );
}

#[test]
#[trace("TC-189", "FR-014-AC-6")]
fn tc_189_doc_finding_categories_have_stable_wire_names() {
    let findings = audit_rust_source(
        b"#[test] fn tc_922_missing() {}\n/// Trace: TODO-LATER\n#[test] fn tc_923_invalid() {}",
        DOC_COMMENT_TESTS,
    )
    .expect("valid source");
    let wire = serde_json::to_value(&findings).expect("findings serialize");
    assert_eq!(
        wire,
        serde_json::json!([
            {"category": "doc_trace_missing", "function": "tc_922_missing", "capability": null},
            {"category": "doc_trace_id_invalid", "function": "tc_923_invalid", "capability": null},
        ])
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

/// A consumer that wants only the source audit must not be forced to accept
/// `serde_json/arbitrary_precision` -- a global, workspace-wide change to
/// `serde_json`'s number representation -- just to reach `source_audit`.
/// `source_audit` used to be reachable only through the `full` feature, which
/// bundles `arbitrary_precision` in with it; this is the regression test for
/// the narrower `source-audit` feature that splits them, mirroring the
/// `producer-execution` minimal-consumer check this repository already runs
/// (`tc_128_minimal_downstream_compiles_only_producer_execution_feature` in
/// `tests/producer_execution.rs`) rather than inventing a second pattern for
/// the same property.
#[test]
#[trace("TC-138", "FR-014-AC-5")]
fn tc_138_a_minimal_downstream_compiles_only_the_source_audit_feature() {
    let consumer = tempfile::tempdir().expect("consumer root");
    fs::create_dir(consumer.path().join("src")).expect("consumer source directory");
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::write(
        consumer.path().join("Cargo.toml"),
        format!(
            "[package]\nname='source-audit-consumer-fixture'\nversion='0.0.0'\nedition='2024'\n[dependencies]\nengineering-assurance={{path={manifest_dir:?},default-features=false,features=['source-audit']}}\n"
        ),
    )
    .expect("consumer manifest");
    fs::write(
        consumer.path().join("src/main.rs"),
        "use engineering_assurance::source_audit::audit_rust_source;\nfn main(){let _ = audit_rust_source;}\n",
    )
    .expect("consumer source");
    let status = Command::new(env!("CARGO"))
        .args(["check", "--offline", "--manifest-path"])
        .arg(consumer.path().join("Cargo.toml"))
        .env("CARGO_BUILD_JOBS", "2")
        .env("CARGO_TARGET_DIR", consumer.path().join("target"))
        .status()
        .expect("consumer cargo check must launch");
    assert!(
        status.success(),
        "minimal source-audit consumer must compile"
    );

    let metadata = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--offline",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(consumer.path().join("Cargo.toml"))
        .output()
        .expect("consumer metadata must launch");
    assert!(metadata.status.success());
    let graph: serde_json::Value = serde_json::from_slice(&metadata.stdout).expect("metadata JSON");
    let assurance_package = graph["packages"]
        .as_array()
        .expect("metadata packages")
        .iter()
        .find(|package| package["name"] == "engineering-assurance")
        .expect("Engineering Assurance package");
    let assurance_id = assurance_package["id"]
        .as_str()
        .expect("Engineering Assurance package id");
    let direct_dependencies = graph["resolve"]["nodes"]
        .as_array()
        .expect("metadata resolve nodes")
        .iter()
        .find(|node| node["id"] == assurance_id)
        .and_then(|node| node["deps"].as_array())
        .expect("Engineering Assurance resolve node")
        .iter()
        .filter_map(|dependency| dependency["name"].as_str())
        .collect::<BTreeSet<_>>();
    // Cargo's resolve-node dependency names are the lib name (hyphens become
    // underscores), not the crate name as written in Cargo.toml.
    for forbidden in [
        "cap_std",
        "clap",
        "flate2",
        "jsonschema",
        "regex",
        "rustix",
        "serde_json_canonicalizer",
        "tar",
        "tempfile",
        "time",
        "unicode_casefold",
        "yaml_serde",
        "zip",
    ] {
        assert!(
            !direct_dependencies.contains(forbidden),
            "unexpected activated direct dependency {forbidden}"
        );
    }
    // Whole-graph, not just direct: a source-audit-only consumer must not
    // resolve `serde_json` at all, from any transitive path, so it cannot
    // inherit an `arbitrary_precision` flip from anywhere else in a
    // downstream workspace's feature unification.
    let resolved_packages = graph["packages"]
        .as_array()
        .expect("metadata packages")
        .iter()
        .filter_map(|package| package["name"].as_str())
        .collect::<BTreeSet<_>>();
    assert!(
        !resolved_packages.contains("serde_json"),
        "a source-audit-only consumer must not resolve serde_json anywhere \
         in its dependency graph: {resolved_packages:?}"
    );

    // The `full` feature must still expose `source_audit` and still
    // carry `arbitrary_precision` unchanged -- this split must not weaken the
    // existing consumer's guarantee while adding the narrow one.
    let full_feature_tree = cargo_feature_tree(&manifest_dir.join("Cargo.toml"), &["full"]);
    assert!(
        full_feature_tree.contains("arbitrary_precision"),
        "full Engineering Assurance feature set must retain serde_json arbitrary_precision"
    );
}

fn cargo_feature_tree(manifest_path: &Path, features: &[&str]) -> String {
    let feature_tree = Command::new(env!("CARGO"))
        .args([
            "tree",
            "--offline",
            "--edges",
            "features",
            "--invert",
            "serde_json",
            "--manifest-path",
        ])
        .arg(manifest_path)
        .args(features.iter().flat_map(|f| ["--features", f]))
        .output()
        .expect("feature tree must launch");
    assert!(
        feature_tree.status.success(),
        "feature tree failed: {}",
        String::from_utf8_lossy(&feature_tree.stderr)
    );
    String::from_utf8(feature_tree.stdout).expect("feature tree must be UTF-8")
}
