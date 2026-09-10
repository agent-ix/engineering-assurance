// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure parsed-Rust-source audits for library containment and test traces.
//!
//! The caller supplies source bytes and their role. This module parses Rust
//! syntax in memory and owns no filesystem, process, environment, network,
//! clock, or persistence capability.

use std::collections::BTreeMap;

use serde::Serialize;
use syn::{
    Attribute, Block, File, ImplItemFn, Item, ItemExternCrate, ItemFn, ItemUse, LitStr, Macro,
    Path, Stmt, TraitItemFn, UseTree, parse::Parser, punctuated::Punctuated, visit::Visit,
};
use thiserror::Error;

/// Maximum bytes accepted for one Rust source document.
pub const MAX_RUST_SOURCE_BYTES: usize = 2_097_152;

/// Closed purpose assigned to one caller-supplied Rust source document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RustSourceAuditRole {
    /// Inspect reusable library code for prohibited host capabilities.
    ReusableLibrary,
    /// Inspect first-party tests for canonical requirement traces.
    RequirementTests,
}

/// Host capability prohibited from reusable library source.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RustSourceCapability {
    /// Filesystem access.
    Filesystem,
    /// Child-program access.
    ChildProgram,
    /// Runtime environment access.
    Environment,
    /// Socket or network access.
    Network,
    /// Runtime clock access.
    Clock,
    /// Persistence-library access.
    Persistence,
    /// Standard output or diagnostic emission.
    Output,
}

/// Closed finding category emitted by parsed source inspection.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RustSourceFindingCategory {
    /// Reusable library source names a prohibited capability.
    ForbiddenCapability,
    /// A source containing tests lacks the exact trace import.
    TraceImportMissing,
    /// A trace import uses a rename.
    TraceImportAliased,
    /// A test uses a path-qualified trace attribute.
    TraceAttributeQualified,
    /// A bare trace attribute does not contain only string literals.
    TraceArgumentsInvalid,
    /// A test function has no bare trace attribute.
    TestTraceMissing,
    /// A test's trace attributes contain no canonical test-case identifier.
    TraceTestCaseMissing,
    /// A test's trace attributes contain no acceptance-criterion identifier.
    TraceAcceptanceCriterionMissing,
}

/// One deterministic finding from a single source document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RustSourceFinding {
    category: RustSourceFindingCategory,
    function: Option<Box<str>>,
    capability: Option<RustSourceCapability>,
}

impl RustSourceFinding {
    /// Return the closed finding category.
    #[must_use]
    pub const fn category(&self) -> RustSourceFindingCategory {
        self.category
    }

    /// Return the affected test or function name when one applies.
    #[must_use]
    pub fn function(&self) -> Option<&str> {
        self.function.as_deref()
    }

    /// Return the prohibited capability when one applies.
    #[must_use]
    pub const fn capability(&self) -> Option<RustSourceCapability> {
        self.capability
    }

    fn global(category: RustSourceFindingCategory) -> Self {
        Self {
            category,
            function: None,
            capability: None,
        }
    }

    fn for_function(category: RustSourceFindingCategory, function: &str) -> Self {
        Self {
            category,
            function: Some(function.into()),
            capability: None,
        }
    }

    fn for_capability(function: Option<&str>, capability: RustSourceCapability) -> Self {
        Self {
            category: RustSourceFindingCategory::ForbiddenCapability,
            function: function.map(Into::into),
            capability: Some(capability),
        }
    }
}

/// Refusal raised before a source document can be inspected.
#[derive(Debug, Error)]
pub enum RustSourceAuditError {
    /// The supplied source exceeds the fixed byte ceiling.
    #[error("Rust source is {actual} bytes; limit is {limit} bytes")]
    SourceTooLarge {
        /// Observed byte count.
        actual: usize,
        /// Maximum accepted byte count.
        limit: usize,
    },
    /// The supplied source is not UTF-8.
    #[error("Rust source is not UTF-8: {0}")]
    InvalidUtf8(std::str::Utf8Error),
    /// The supplied source is not valid Rust syntax.
    #[error("Rust source syntax is invalid: {0}")]
    InvalidSyntax(syn::Error),
}

impl RustSourceAuditError {
    /// Return the stable machine category for this refusal.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::SourceTooLarge { .. } => "rust_source_too_large",
            Self::InvalidUtf8(_) => "rust_source_not_utf8",
            Self::InvalidSyntax(_) => "rust_source_syntax_invalid",
        }
    }
}

/// Inspect one caller-supplied Rust source document without I/O.
///
/// # Errors
///
/// Returns [`RustSourceAuditError`] when the source exceeds the fixed ceiling,
/// is not UTF-8, or cannot be parsed as Rust syntax.
pub fn audit_rust_source(
    bytes: &[u8],
    role: RustSourceAuditRole,
) -> Result<Vec<RustSourceFinding>, RustSourceAuditError> {
    if bytes.len() > MAX_RUST_SOURCE_BYTES {
        return Err(RustSourceAuditError::SourceTooLarge {
            actual: bytes.len(),
            limit: MAX_RUST_SOURCE_BYTES,
        });
    }
    let source = std::str::from_utf8(bytes).map_err(RustSourceAuditError::InvalidUtf8)?;
    let syntax = syn::parse_file(source).map_err(RustSourceAuditError::InvalidSyntax)?;
    let mut findings = match role {
        RustSourceAuditRole::ReusableLibrary => audit_library(&syntax),
        RustSourceAuditRole::RequirementTests => audit_tests(&syntax),
    };
    findings.sort_by(|left, right| {
        left.function
            .cmp(&right.function)
            .then_with(|| left.category.cmp(&right.category))
            .then_with(|| left.capability.cmp(&right.capability))
    });
    findings.dedup();
    Ok(findings)
}

#[derive(Clone, Debug)]
struct Import {
    target: Vec<String>,
    local: Option<String>,
    renamed: bool,
}

#[derive(Default)]
struct ImportCollector {
    imports: Vec<Import>,
    exact_trace_import: bool,
}

impl<'ast> Visit<'ast> for ImportCollector {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        self.exact_trace_import |= is_exact_trace_import(item);
        flatten_use_tree(&[], &item.tree, &mut self.imports);
        syn::visit::visit_item_use(self, item);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast ItemExternCrate) {
        self.imports.push(Import {
            target: vec![item.ident.to_string()],
            local: Some(
                item.rename
                    .as_ref()
                    .map_or_else(|| item.ident.to_string(), |(_, name)| name.to_string()),
            ),
            renamed: item.rename.is_some(),
        });
        syn::visit::visit_item_extern_crate(self, item);
    }
}

fn flatten_use_tree(prefix: &[String], tree: &UseTree, imports: &mut Vec<Import>) {
    match tree {
        UseTree::Path(path) => {
            let mut nested = prefix.to_vec();
            nested.push(path.ident.to_string());
            flatten_use_tree(&nested, &path.tree, imports);
        }
        UseTree::Name(name) => {
            let mut target = prefix.to_vec();
            if name.ident != "self" {
                target.push(name.ident.to_string());
            }
            imports.push(Import {
                local: target.last().cloned(),
                target,
                renamed: false,
            });
        }
        UseTree::Rename(rename) => {
            let mut target = prefix.to_vec();
            if rename.ident != "self" {
                target.push(rename.ident.to_string());
            }
            imports.push(Import {
                target,
                local: Some(rename.rename.to_string()),
                renamed: true,
            });
        }
        UseTree::Glob(_) => imports.push(Import {
            target: prefix.to_vec(),
            local: None,
            renamed: false,
        }),
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use_tree(prefix, item, imports);
            }
        }
    }
}

fn collect_imports(syntax: &File) -> ImportCollector {
    let mut collector = ImportCollector::default();
    collector.visit_file(syntax);
    collector
}

fn is_exact_trace_import(item: &ItemUse) -> bool {
    if item.leading_colon.is_some() || !matches!(item.vis, syn::Visibility::Inherited) {
        return false;
    }
    let UseTree::Path(root) = &item.tree else {
        return false;
    };
    root.ident == "ix_trace_rs"
        && matches!(root.tree.as_ref(), UseTree::Name(name) if name.ident == "trace")
}

fn audit_library(syntax: &File) -> Vec<RustSourceFinding> {
    let mut visitor = LibraryVisitor {
        scopes: Vec::new(),
        function: None,
        findings: Vec::new(),
    };
    visitor.visit_file(syntax);
    visitor.findings
}

#[derive(Default)]
struct ImportScope {
    aliases: BTreeMap<String, Vec<String>>,
    declarations: Vec<String>,
}

struct LibraryVisitor {
    scopes: Vec<ImportScope>,
    function: Option<String>,
    findings: Vec<RustSourceFinding>,
}

impl LibraryVisitor {
    fn enter_function(&mut self, name: String, visit: impl FnOnce(&mut Self)) {
        let previous = self.function.replace(name);
        visit(self);
        self.function = previous;
    }

    fn enter_scope(&mut self, items: &[&Item], visit: impl FnOnce(&mut Self)) {
        let mut imports = Vec::new();
        let mut declarations = Vec::new();
        for item in items {
            match item {
                Item::Use(item) => flatten_use_tree(&[], &item.tree, &mut imports),
                Item::ExternCrate(item) => imports.push(extern_crate_import(item)),
                _ => {
                    if let Some(name) = item_declaration_name(item) {
                        declarations.push(name);
                    }
                }
            }
        }
        let aliases = imports
            .iter()
            .filter_map(|import| {
                import
                    .local
                    .as_ref()
                    .map(|local| (local.clone(), import.target.clone()))
            })
            .collect();
        self.scopes.push(ImportScope {
            aliases,
            declarations,
        });
        for import in imports {
            let resolved = self.resolve_segments(import.target);
            if let Some(capability) = classify_path(&resolved) {
                self.findings.push(RustSourceFinding::for_capability(
                    self.function.as_deref(),
                    capability,
                ));
            }
        }
        visit(self);
        self.scopes.pop();
    }

    fn resolve_segments(&self, mut segments: Vec<String>) -> Vec<String> {
        let mut expanded_names = Vec::new();
        while let Some(first) = segments.first().cloned() {
            if expanded_names.contains(&first) {
                break;
            }
            expanded_names.push(first.clone());
            let mut replacement = None;
            for scope in self.scopes.iter().rev() {
                if scope.declarations.contains(&first) {
                    return segments;
                }
                if let Some(target) = scope.aliases.get(&first) {
                    replacement = Some(target.clone());
                    break;
                }
            }
            let Some(target) = replacement else {
                break;
            };
            segments.splice(0..1, target);
        }
        segments
    }

    fn inspect_path(&mut self, path: &Path) {
        let segments = self.resolve_segments(
            path.segments
                .iter()
                .map(|item| item.ident.to_string())
                .collect(),
        );
        if let Some(capability) = classify_path(&segments) {
            self.findings.push(RustSourceFinding::for_capability(
                self.function.as_deref(),
                capability,
            ));
        }
    }

    fn inspect_macro(&mut self, value: &Macro) {
        let Some(name) = value
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
        else {
            return;
        };
        let capability = match name.as_str() {
            "print" | "println" | "eprint" | "eprintln" | "dbg" => {
                Some(RustSourceCapability::Output)
            }
            "option_env" => Some(RustSourceCapability::Environment),
            "env" if !allowed_package_metadata(value) => Some(RustSourceCapability::Environment),
            _ => None,
        };
        if let Some(capability) = capability {
            self.findings.push(RustSourceFinding::for_capability(
                self.function.as_deref(),
                capability,
            ));
        }
    }
}

impl<'ast> Visit<'ast> for LibraryVisitor {
    fn visit_file(&mut self, file: &'ast File) {
        let items: Vec<_> = file.items.iter().collect();
        self.enter_scope(&items, |visitor| {
            for item in &file.items {
                visitor.visit_item(item);
            }
        });
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if let Some((_, items)) = &item.content {
            let scoped: Vec<_> = items.iter().collect();
            self.enter_scope(&scoped, |visitor| {
                for child in items {
                    visitor.visit_item(child);
                }
            });
        }
    }

    fn visit_block(&mut self, block: &'ast Block) {
        let items: Vec<_> = block
            .stmts
            .iter()
            .filter_map(|statement| match statement {
                Stmt::Item(item) => Some(item),
                _ => None,
            })
            .collect();
        self.enter_scope(&items, |visitor| {
            for statement in &block.stmts {
                visitor.visit_stmt(statement);
            }
        });
    }

    fn visit_item_use(&mut self, _item: &'ast ItemUse) {}

    fn visit_item_extern_crate(&mut self, _item: &'ast ItemExternCrate) {}

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        self.enter_function(item.sig.ident.to_string(), |visitor| {
            syn::visit::visit_item_fn(visitor, item);
        });
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        self.enter_function(item.sig.ident.to_string(), |visitor| {
            syn::visit::visit_impl_item_fn(visitor, item);
        });
    }

    fn visit_trait_item_fn(&mut self, item: &'ast TraitItemFn) {
        self.enter_function(item.sig.ident.to_string(), |visitor| {
            syn::visit::visit_trait_item_fn(visitor, item);
        });
    }

    fn visit_path(&mut self, path: &'ast Path) {
        self.inspect_path(path);
        syn::visit::visit_path(self, path);
    }

    fn visit_macro(&mut self, value: &'ast Macro) {
        self.inspect_macro(value);
        syn::visit::visit_macro(self, value);
    }
}

fn extern_crate_import(item: &ItemExternCrate) -> Import {
    Import {
        target: vec![item.ident.to_string()],
        local: Some(
            item.rename
                .as_ref()
                .map_or_else(|| item.ident.to_string(), |(_, name)| name.to_string()),
        ),
        renamed: item.rename.is_some(),
    }
}

fn item_declaration_name(item: &Item) -> Option<String> {
    let ident = match item {
        Item::Const(item) => &item.ident,
        Item::Enum(item) => &item.ident,
        Item::Fn(item) => &item.sig.ident,
        Item::Mod(item) => &item.ident,
        Item::Static(item) => &item.ident,
        Item::Struct(item) => &item.ident,
        Item::Trait(item) => &item.ident,
        Item::TraitAlias(item) => &item.ident,
        Item::Type(item) => &item.ident,
        Item::Union(item) => &item.ident,
        _ => return None,
    };
    Some(ident.to_string())
}

fn classify_path(segments: &[String]) -> Option<RustSourceCapability> {
    let values: Vec<_> = segments.iter().map(String::as_str).collect();
    match values.as_slice() {
        ["std" | "tokio" | "async_std", "fs", ..] | ["cap_std", ..] => {
            Some(RustSourceCapability::Filesystem)
        }
        ["std" | "tokio" | "async_std", "process", ..] => Some(RustSourceCapability::ChildProgram),
        ["std", "env", ..] => Some(RustSourceCapability::Environment),
        ["std" | "tokio" | "async_std", "net", ..] | ["reqwest" | "ureq", ..] => {
            Some(RustSourceCapability::Network)
        }
        ["std", "time", "Instant" | "SystemTime", ..]
        | ["time", "Instant", ..]
        | ["time", "OffsetDateTime", "now_utc", ..]
        | ["chrono", "Utc" | "Local", "now", ..] => Some(RustSourceCapability::Clock),
        [
            "rusqlite" | "sqlx" | "diesel" | "sled" | "rocksdb" | "redb",
            ..,
        ] => Some(RustSourceCapability::Persistence),
        ["std", "io", "stdin" | "stdout" | "stderr", ..]
        | [
            "std",
            "print" | "println" | "eprint" | "eprintln" | "dbg",
            ..,
        ] => Some(RustSourceCapability::Output),
        _ => None,
    }
}

fn allowed_package_metadata(value: &Macro) -> bool {
    let Ok(variable) = syn::parse2::<LitStr>(value.tokens.clone()) else {
        return false;
    };
    matches!(
        variable.value().as_str(),
        "CARGO_PKG_NAME" | "CARGO_PKG_VERSION"
    )
}

fn audit_tests(syntax: &File) -> Vec<RustSourceFinding> {
    let imports = collect_imports(syntax);
    let exact_import = imports.exact_trace_import;
    let aliased_import = imports
        .imports
        .iter()
        .any(|import| import.target == ["ix_trace_rs", "trace"] && import.renamed);
    let mut visitor = TestVisitor {
        findings: Vec::new(),
        test_count: 0,
    };
    visitor.visit_file(syntax);
    if visitor.test_count > 0 && !exact_import {
        visitor.findings.push(RustSourceFinding::global(
            RustSourceFindingCategory::TraceImportMissing,
        ));
    }
    if visitor.test_count > 0 && aliased_import {
        visitor.findings.push(RustSourceFinding::global(
            RustSourceFindingCategory::TraceImportAliased,
        ));
    }
    visitor.findings
}

#[derive(Default)]
struct TestVisitor {
    findings: Vec<RustSourceFinding>,
    test_count: usize,
}

impl TestVisitor {
    fn inspect_function(&mut self, name: &str, attributes: &[Attribute]) {
        if !attributes.iter().any(is_test_attribute) {
            return;
        }
        self.test_count = self.test_count.saturating_add(1);
        let mut bare_trace = false;
        let mut has_test_case = false;
        let mut has_acceptance = false;
        for attribute in attributes
            .iter()
            .filter(|attribute| is_trace_attribute(attribute))
        {
            if !attribute.path().is_ident("trace") {
                self.findings.push(RustSourceFinding::for_function(
                    RustSourceFindingCategory::TraceAttributeQualified,
                    name,
                ));
                continue;
            }
            bare_trace = true;
            let Ok(list) = attribute.meta.require_list() else {
                self.findings.push(RustSourceFinding::for_function(
                    RustSourceFindingCategory::TraceArgumentsInvalid,
                    name,
                ));
                continue;
            };
            let parser = Punctuated::<LitStr, syn::Token![,]>::parse_terminated;
            let Ok(arguments) = parser.parse2(list.tokens.clone()) else {
                self.findings.push(RustSourceFinding::for_function(
                    RustSourceFindingCategory::TraceArgumentsInvalid,
                    name,
                ));
                continue;
            };
            has_test_case |= arguments
                .iter()
                .any(|value| is_test_case_id(&value.value()));
            has_acceptance |= arguments
                .iter()
                .any(|value| is_acceptance_id(&value.value()));
        }
        if !bare_trace {
            self.findings.push(RustSourceFinding::for_function(
                RustSourceFindingCategory::TestTraceMissing,
                name,
            ));
            return;
        }
        if !has_test_case {
            self.findings.push(RustSourceFinding::for_function(
                RustSourceFindingCategory::TraceTestCaseMissing,
                name,
            ));
        }
        if !has_acceptance {
            self.findings.push(RustSourceFinding::for_function(
                RustSourceFindingCategory::TraceAcceptanceCriterionMissing,
                name,
            ));
        }
    }
}

impl<'ast> Visit<'ast> for TestVisitor {
    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        self.inspect_function(&item.sig.ident.to_string(), &item.attrs);
        syn::visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        self.inspect_function(&item.sig.ident.to_string(), &item.attrs);
        syn::visit::visit_impl_item_fn(self, item);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast TraitItemFn) {
        self.inspect_function(&item.sig.ident.to_string(), &item.attrs);
        syn::visit::visit_trait_item_fn(self, item);
    }
}

fn is_test_attribute(attribute: &Attribute) -> bool {
    attribute
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "test")
}

fn is_trace_attribute(attribute: &Attribute) -> bool {
    attribute
        .path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "trace")
}

fn is_test_case_id(value: &str) -> bool {
    value.len() == 6
        && value.starts_with("TC-")
        && value.as_bytes()[3..].iter().all(u8::is_ascii_digit)
}

fn is_acceptance_id(value: &str) -> bool {
    let Some((parent, number)) = value.rsplit_once("-AC-") else {
        return false;
    };
    !parent.is_empty()
        && parent
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
        && !number.is_empty()
        && number.bytes().all(|byte| byte.is_ascii_digit())
}
