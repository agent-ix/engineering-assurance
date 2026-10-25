// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Pure classification of non-Rust executable entrypoints in Git tree entries.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write as _,
    path::Path,
};

use serde::Serialize;
use sha2::{Digest, Sha256};
use syn::{Expr, ExprCall, Lit, UseTree, visit::Visit};
use thiserror::Error;

const MAX_MAKE_TARGETS: usize = 2_048;
const MAX_MAKE_VARIABLES: usize = 2_048;
const MAX_MAKE_EXPANSION: usize = 8_192;
const MAX_MAKE_DEPTH: usize = 8;

/// One bounded, regular Git tree entry selected by the host.
pub(crate) struct Entry {
    pub(crate) path: String,
    pub(crate) mode: String,
    pub(crate) bytes: Vec<u8>,
}

/// A stable, path-scoped executable surface in one revision.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub(crate) struct Finding {
    pub(crate) path: String,
    pub(crate) kind: &'static str,
    pub(crate) evidence: String,
}

/// A selected source cannot be classified safely.
#[derive(Debug, Error)]
pub(crate) enum ClassificationError {
    #[error("selected executable or launcher text is not UTF-8: {0}")]
    NonUtf8(String),
    #[error("selected package manifest is invalid JSON: {0}")]
    InvalidManifest(String),
    #[error("selected package manifest has an invalid scripts or bin map: {0}")]
    InvalidManifestShape(String),
    #[error("selected Make target population exceeds {MAX_MAKE_TARGETS}")]
    TooManyMakeTargets,
    #[error("selected Make variable population or expansion exceeds its bound")]
    MakeExpansionTooLarge,
    #[error("selected Actions environment population or expansion exceeds its bound")]
    ActionExpansionTooLarge,
    #[error("selected Rust source is invalid: {0}")]
    InvalidRustSource(String),
    #[error("selected Actions YAML is invalid: {0}")]
    InvalidActionYaml(String),
}

/// Whether a tree entry can hold a relevant executable or launcher surface.
pub(crate) fn needs_content(path: &str, mode: &str) -> bool {
    mode == "100755"
        || path == "Makefile"
        || path.ends_with("/Makefile")
        || has_extension(path, "mk")
        || path == "package.json"
        || path.ends_with("/package.json")
        || path.starts_with(".github/workflows/")
        || path.starts_with(".github/actions/")
        || is_actions_file(path)
        || executable_location(path)
        || [
            "py", "sh", "bash", "js", "mjs", "cjs", "ts", "tsx", "rb", "pl", "rs",
        ]
        .iter()
        .any(|suffix| has_extension(path, suffix))
        || !path.rsplit('/').next().unwrap_or(path).contains('.')
}

/// Compare complete revisions, retaining dependency findings except for a
/// proven version-only release of an existing repository-owned package family.
pub(crate) fn classify_delta(
    repository: &str,
    before: &[Entry],
    after: &[Entry],
) -> Result<Vec<Finding>, ClassificationError> {
    let base_findings = classify(repository, before)?;
    let head_findings = classify(repository, after)?;
    let base_packages: BTreeMap<_, _> = before
        .iter()
        .filter(|entry| entry.path == "package.json" || entry.path.ends_with("/package.json"))
        .map(|entry| (entry.path.as_str(), entry))
        .collect();
    let mut release_versions = BTreeSet::new();
    for head in after {
        if let Some(base) = base_packages.get(head.path.as_str()) {
            release_versions.extend(owned_release_versions(repository, base, head));
        }
    }
    Ok(head_findings
        .difference(&base_findings)
        .filter(|finding| !release_versions.contains(*finding))
        .cloned()
        .collect())
}

fn owned_release_versions(repository: &str, base: &Entry, head: &Entry) -> BTreeSet<Finding> {
    let mut versions = BTreeSet::new();
    if base.mode != "100644" || head.mode != base.mode {
        return versions;
    }
    let (Ok(before), Ok(mut after)) = (
        serde_json::from_slice::<serde_json::Value>(&base.bytes),
        serde_json::from_slice::<serde_json::Value>(&head.bytes),
    ) else {
        return versions;
    };
    let owner_name = format!("@agent-ix/{repository}");
    let owner_url = format!(
        "{}{repository}.git",
        concat!("git+https:", "//github.com/agent-ix/")
    );
    if before.get("name").and_then(serde_json::Value::as_str) != Some(&owner_name)
        || before
            .pointer("/repository/type")
            .and_then(serde_json::Value::as_str)
            != Some("git")
        || before
            .pointer("/repository/url")
            .and_then(serde_json::Value::as_str)
            != Some(&owner_url)
        || before
            .get("private")
            .is_some_and(|value| value != &serde_json::Value::Bool(false))
        || before
            .get("bin")
            .and_then(serde_json::Value::as_object)
            .is_none_or(serde_json::Map::is_empty)
        || before
            .pointer("/publishConfig/registry")
            .is_some_and(|registry| {
                registry.as_str() != Some(concat!("https:", "//registry.npmjs.org/"))
            })
    {
        return versions;
    }
    let (Some(old_version), Some(new_version)) = (
        before.get("version").and_then(serde_json::Value::as_str),
        after.get("version").and_then(serde_json::Value::as_str),
    ) else {
        return versions;
    };
    if old_version == new_version || !release_version(old_version) || !release_version(new_version)
    {
        return versions;
    }
    let new_version = new_version.to_owned();
    let Some(dependencies) = before
        .get("optionalDependencies")
        .and_then(serde_json::Value::as_object)
    else {
        return versions;
    };
    let prefix = format!("{owner_name}-");
    let family: Vec<_> = dependencies
        .iter()
        .filter(|(name, value)| {
            name.strip_prefix(&prefix)
                .is_some_and(|suffix| !suffix.is_empty())
                && value.as_str() == Some(old_version)
        })
        .map(|(name, _)| name)
        .collect();
    if family.is_empty() {
        return versions;
    }
    for name in family {
        let Some(value) = after
            .get_mut("optionalDependencies")
            .and_then(serde_json::Value::as_object_mut)
            .and_then(|dependencies| dependencies.get_mut(name))
        else {
            return BTreeSet::new();
        };
        if value.as_str() != Some(new_version.as_str()) {
            return BTreeSet::new();
        }
        versions.insert(Finding {
            path: head.path.clone(),
            kind: "npm-dependency",
            evidence: format!("optionalDependencies:{name}:{new_version}"),
        });
        *value = serde_json::Value::String(old_version.to_owned());
    }
    after["version"] = serde_json::Value::String(old_version.to_owned());
    if after != before {
        versions.clear();
    }
    versions
}

fn release_version(value: &str) -> bool {
    fn identifier(value: &str) -> bool {
        !value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    }
    fn number(value: &str) -> bool {
        !value.is_empty()
            && value.bytes().all(|byte| byte.is_ascii_digit())
            && (value.len() == 1 || !value.starts_with('0'))
    }
    let (version, build) = value
        .split_once('+')
        .map_or((value, None), |(version, build)| (version, Some(build)));
    if build.is_some_and(|build| !build.split('.').all(identifier)) {
        return false;
    }
    let (core, pre) = version
        .split_once('-')
        .map_or((version, None), |(core, pre)| (core, Some(pre)));
    if pre.is_some_and(|pre| {
        !pre.split('.').all(|part| {
            identifier(part) && (!part.bytes().all(|byte| byte.is_ascii_digit()) || number(part))
        })
    }) {
        return false;
    }
    let mut parts = core.split('.');
    matches!((parts.next(), parts.next(), parts.next(), parts.next()), (Some(major), Some(minor), Some(patch), None) if number(major) && number(minor) && number(patch))
}

/// Classify one complete selected revision. Existing findings are compared by
/// the caller against the base revision; no catalog or frozen baseline exists.
pub(crate) fn classify(
    repository: &str,
    entries: &[Entry],
) -> Result<BTreeSet<Finding>, ClassificationError> {
    let make_targets = risky_make_targets(entries)?;
    let mut findings = BTreeSet::new();
    for entry in entries {
        if exempt(repository, &entry.path) {
            continue;
        }
        if entry.mode == "160000" {
            findings.insert(Finding {
                path: entry.path.clone(),
                kind: "opaque-submodule",
                evidence: "gitlink".to_owned(),
            });
            continue;
        }
        let path = entry.path.as_str();
        let text = std::str::from_utf8(&entry.bytes).ok();
        if text.is_none() && entry.mode != "100755" {
            return Err(ClassificationError::NonUtf8(entry.path.clone()));
        }
        let shebang = text
            .and_then(|value| value.lines().next())
            .is_some_and(non_rust_shebang);
        let symlink_target = entry.mode == "120000"
            && text.is_some_and(|target| non_rust_launch(target) || has_script_directory(path));
        let executable =
            entry.mode == "100755" || shebang || symlink_target || executable_location(path);
        if executable && !has_extension(path, "rs") {
            findings.insert(Finding {
                path: entry.path.clone(),
                kind: "executable-file",
                evidence: format!("sha256:{}", hex_digest(&entry.bytes)),
            });
        }
        if ["py", "sh", "bash", "js", "mjs", "cjs", "ts", "tsx"]
            .iter()
            .any(|suffix| has_extension(path, suffix))
            && !executable
        {
            findings.insert(Finding {
                path: entry.path.clone(),
                kind: "foreign-source",
                evidence: format!("sha256:{}", hex_digest(&entry.bytes)),
            });
        }
        if path == "package.json" || path.ends_with("/package.json") {
            classify_package(entry, &mut findings)?;
        }
        classify_action_container(entry, text, &mut findings);
        if has_extension(path, "rs")
            && !rust_fixture_input(path)
            && let Some(text) = text
        {
            classify_rust(entry, text, &mut findings)?;
        }
        if is_makefile(path) || is_actions_file(path) {
            let text = text.ok_or_else(|| ClassificationError::NonUtf8(entry.path.clone()))?;
            let commands = if is_actions_file(path) {
                action_commands(path, text)?
            } else {
                make_commands(text)?
            };
            let mut occurrences = BTreeMap::new();
            for (context, command) in commands {
                let dynamic = (is_actions_file(path) && context.starts_with("run-"))
                    || (is_makefile(path) && !context.starts_with("assignment:"));
                let direct = context == "external-action"
                    || context == "action-runtime"
                    || non_rust_launch(&command)
                    || (dynamic && unresolved_command_position(&command, is_makefile(path)));
                let indirect = make_launches_risky_target(&command, &make_targets);
                if direct || indirect {
                    let identity = format!("{context}:{}", command.trim());
                    let ordinal = occurrences.entry(identity.clone()).or_insert(0usize);
                    *ordinal += 1;
                    findings.insert(Finding {
                        path: entry.path.clone(),
                        kind: if indirect {
                            "indirect-launch"
                        } else {
                            "inline-launch"
                        },
                        evidence: format!("{identity}#{ordinal}"),
                    });
                }
            }
        }
    }
    Ok(findings)
}

fn classify_action_container(entry: &Entry, text: Option<&str>, findings: &mut BTreeSet<Finding>) {
    if entry.path.starts_with(".github/actions/")
        && entry.path.rsplit('/').next() == Some("Dockerfile")
        && text.is_some_and(|body| {
            body.lines().any(|line| {
                let line = line.trim_start();
                line.starts_with("FROM ") || line.starts_with("RUN ")
            })
        })
    {
        findings.insert(Finding {
            path: entry.path.clone(),
            kind: "action-container",
            evidence: format!("sha256:{}", hex_digest(&entry.bytes)),
        });
    }
}

fn rust_fixture_input(path: &str) -> bool {
    fixture_location(path) && (path.contains("/input/") || path.contains("/samples/"))
}

fn fixture_location(path: &str) -> bool {
    path.starts_with("tests/fixtures/")
        || path.starts_with("fixtures/")
        || path.contains("/fixtures/")
        || path.starts_with("corpus/cases/")
        || path.starts_with("cases/")
}

fn executable_location(path: &str) -> bool {
    let at_root = !path.contains('/');
    (has_script_directory(path) || at_root)
        && [
            "py", "sh", "bash", "js", "mjs", "cjs", "ts", "tsx", "rb", "pl",
        ]
        .iter()
        .any(|suffix| has_extension(path, suffix))
}

fn has_script_directory(path: &str) -> bool {
    path.starts_with(".github/actions/")
        || path
            .split('/')
            .any(|part| matches!(part, "scripts" | "tools" | "bin" | "templates"))
}

fn non_rust_shebang(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("#!") else {
        return false;
    };
    let mut words = rest.split_whitespace();
    let Some(first) = words.next() else {
        return true;
    };
    let interpreter = if first.rsplit('/').next() == Some("env") {
        let next = words.next();
        if next == Some("-S") {
            words.next()
        } else {
            next
        }
    } else {
        Some(first)
    };
    !interpreter.is_some_and(|name| {
        matches!(
            name.rsplit('/').next(),
            Some("cargo" | "rustc" | "rust-script")
        )
    })
}

fn has_extension(path: &str, suffix: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(suffix))
}

fn hex_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing into a String cannot fail");
    }
    output
}

fn classify_rust(
    entry: &Entry,
    text: &str,
    findings: &mut BTreeSet<Finding>,
) -> Result<(), ClassificationError> {
    let syntax = match syn::parse_file(text) {
        Ok(syntax) => syntax,
        Err(_) if fixture_location(&entry.path) => return Ok(()),
        Err(_) => return Err(ClassificationError::InvalidRustSource(entry.path.clone())),
    };
    let mut names = RustProcessNames::default();
    names.visit_file(&syntax);
    let safe_cargo_names = names
        .safe_cargo_candidates
        .into_iter()
        .filter(|name| names.binding_counts.get(name) == Some(&1))
        .collect();
    let mut visitor = RustLauncherVisitor {
        path: &entry.path,
        findings,
        process_names: names.process_names,
        process_modules: names.process_modules,
        safe_cargo_names,
        occurrences: BTreeMap::new(),
    };
    visitor.visit_file(&syntax);
    Ok(())
}

#[derive(Default)]
struct RustProcessNames {
    process_names: BTreeSet<String>,
    process_modules: BTreeSet<String>,
    binding_counts: BTreeMap<String, usize>,
    safe_cargo_candidates: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for RustProcessNames {
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        collect_process_import(
            &node.tree,
            &mut Vec::new(),
            &mut self.process_names,
            &mut self.process_modules,
        );
    }

    fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
        *self
            .binding_counts
            .entry(node.ident.to_string())
            .or_default() += 1;
        syn::visit::visit_pat_ident(self, node);
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        if let syn::Pat::Ident(binding) = &node.pat
            && binding.mutability.is_none()
            && node
                .init
                .as_ref()
                .is_some_and(|init| is_cargo_environment_fallback(&init.expr))
        {
            self.safe_cargo_candidates.insert(binding.ident.to_string());
        }
        syn::visit::visit_local(self, node);
    }
}

fn is_cargo_environment_fallback(expression: &Expr) -> bool {
    let Expr::MethodCall(fallback) = expression else {
        return false;
    };
    if fallback.method != "unwrap_or_else" || fallback.args.len() != 1 {
        return false;
    }
    let Expr::Call(environment) = fallback.receiver.as_ref() else {
        return false;
    };
    let Expr::Path(function) = environment.func.as_ref() else {
        return false;
    };
    let segments: Vec<_> = function
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    if !segments.ends_with(&["env".to_owned(), "var_os".to_owned()])
        || !environment.args.first().is_some_and(|arg| {
            matches!(arg, Expr::Lit(value) if matches!(&value.lit, Lit::Str(name) if name.value() == "CARGO"))
        })
    {
        return false;
    }
    let Some(Expr::Closure(default)) = fallback.args.first() else {
        return false;
    };
    matches!(default.body.as_ref(), Expr::MethodCall(value)
        if value.method == "into" && value.args.is_empty()
        && matches!(value.receiver.as_ref(), Expr::Lit(text)
            if matches!(&text.lit, Lit::Str(name) if name.value() == "cargo")))
}

fn collect_process_import(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    names: &mut BTreeSet<String>,
    modules: &mut BTreeSet<String>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_process_import(&path.tree, prefix, names, modules);
            prefix.pop();
        }
        UseTree::Name(name)
            if prefix.ends_with(&["std".to_owned(), "process".to_owned()])
                && name.ident == "Command" =>
        {
            names.insert("Command".to_owned());
        }
        UseTree::Rename(rename)
            if prefix.ends_with(&["std".to_owned(), "process".to_owned()])
                && rename.ident == "Command" =>
        {
            names.insert(rename.rename.to_string());
        }
        UseTree::Name(name) if prefix == &["std"] && name.ident == "process" => {
            modules.insert("process".to_owned());
        }
        UseTree::Rename(rename) if prefix == &["std"] && rename.ident == "process" => {
            modules.insert(rename.rename.to_string());
        }
        UseTree::Group(group) => {
            for member in &group.items {
                collect_process_import(member, prefix, names, modules);
            }
        }
        _ => {}
    }
}

struct RustLauncherVisitor<'a> {
    path: &'a str,
    findings: &'a mut BTreeSet<Finding>,
    process_names: BTreeSet<String>,
    process_modules: BTreeSet<String>,
    safe_cargo_names: BTreeSet<String>,
    occurrences: BTreeMap<(&'static str, String), usize>,
}

impl RustLauncherVisitor<'_> {
    fn record(&mut self, kind: &'static str, evidence: &str) {
        let ordinal = self
            .occurrences
            .entry((kind, evidence.to_owned()))
            .or_default();
        *ordinal += 1;
        self.findings.insert(Finding {
            path: self.path.to_owned(),
            kind,
            evidence: format!("{evidence}#{ordinal}"),
        });
    }
}

impl<'ast> Visit<'ast> for RustLauncherVisitor<'_> {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(function) = node.func.as_ref() {
            let parts: Vec<_> = function
                .path
                .segments
                .iter()
                .map(|part| part.ident.to_string())
                .collect();
            let qualified_process =
                parts.ends_with(&["process".to_owned(), "Command".to_owned(), "new".to_owned()]);
            let imported_process =
                parts.len() == 2 && parts[1] == "new" && self.process_names.contains(&parts[0]);
            let module_process = parts.len() == 3
                && parts[1] == "Command"
                && parts[2] == "new"
                && self.process_modules.contains(&parts[0]);
            if qualified_process || imported_process || module_process {
                if let Some(Expr::Lit(literal)) = node.args.first()
                    && let Lit::Str(program) = &literal.lit
                {
                    if non_rust_launch(&program.value()) {
                        self.record("rust-wrapper-launch", &program.value());
                    }
                } else if (qualified_process || imported_process || module_process)
                    && !node.args.first().is_some_and(|argument| match argument {
                        Expr::Macro(value) if value.mac.path.is_ident("env") => {
                            syn::parse2::<syn::LitStr>(value.mac.tokens.clone())
                                .is_ok_and(|name| name.value().starts_with("CARGO_BIN_EXE_"))
                        }
                        Expr::Path(path) => path
                            .path
                            .get_ident()
                            .is_some_and(|name| self.safe_cargo_names.contains(&name.to_string())),
                        _ => false,
                    })
                {
                    self.record(
                        "dynamic-rust-launch",
                        "qualified process::Command::new(dynamic)",
                    );
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'ast syn::ExprMacro) {
        if (node.mac.path.is_ident("include_str") || node.mac.path.is_ident("include_bytes"))
            && let Ok(path) = syn::parse2::<syn::LitStr>(node.mac.tokens.clone())
            && non_rust_launch(&path.value())
        {
            self.record("rust-wrapper-launch", &path.value());
        }
        syn::visit::visit_expr_macro(self, node);
    }
}

fn exempt(repository: &str, path: &str) -> bool {
    if path == "docs/images/figures.py" {
        return true;
    }
    if repository == "quire-rs" && path.starts_with("tests/python/") {
        return true;
    }
    if repository == "quire-driver"
        && (path.starts_with("tests/") || path.contains("/tests/"))
        && path
            .rsplit('/')
            .next()
            .is_some_and(|name| name.ends_with("-plugin.py"))
    {
        return true;
    }
    // Only fixture *inputs* are data. A helper beside an input remains governed.
    (path.starts_with("tests/fixtures/")
        || path.contains("/fixtures/")
        || path.starts_with("corpus/cases/")
        || path.starts_with("cases/"))
        && (path.contains("/input/") || path.contains("/samples/"))
}

fn is_makefile(path: &str) -> bool {
    path == "Makefile" || path.ends_with("/Makefile") || has_extension(path, "mk")
}

fn is_actions_file(path: &str) -> bool {
    ((path.starts_with(".github/workflows/") || path.starts_with(".github/actions/"))
        && (has_extension(path, "yml") || has_extension(path, "yaml")))
        || matches!(path.rsplit('/').next(), Some("action.yml" | "action.yaml"))
}

fn make_assignment(line: &str) -> Option<(&str, &str)> {
    if line.starts_with('\t') || line.trim_start().starts_with('#') {
        return None;
    }
    let mut assignment = line.trim_start();
    while let Some(rest) = ["export ", "override ", "private ", "unexport "]
        .iter()
        .find_map(|prefix| assignment.strip_prefix(prefix))
    {
        assignment = rest.trim_start();
    }
    let (name, value) = assignment.split_once('=')?;
    let name = name.trim_end_matches([':', '?', '+', ' ']).trim();
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return None;
    }
    Some((name, value.trim()))
}

fn make_variables(text: &str) -> Result<BTreeMap<String, String>, ClassificationError> {
    let mut variables = BTreeMap::new();
    for line in text.lines() {
        if let Some((name, value)) = make_assignment(line) {
            variables.insert(name.to_owned(), value.to_owned());
            if variables.len() > MAX_MAKE_VARIABLES {
                return Err(ClassificationError::MakeExpansionTooLarge);
            }
        }
    }
    Ok(variables)
}

fn expand_make(
    command: &str,
    variables: &BTreeMap<String, String>,
    depth: usize,
) -> Result<String, ClassificationError> {
    if depth > MAX_MAKE_DEPTH || command.len() > MAX_MAKE_EXPANSION {
        return Err(ClassificationError::MakeExpansionTooLarge);
    }
    let mut expanded = String::new();
    let mut rest = command;
    while let Some(index) = rest.find('$') {
        expanded.push_str(&rest[..index]);
        rest = &rest[index..];
        let (open, close) = if rest.starts_with("$(") {
            (2, ')')
        } else if rest.starts_with("${") {
            (2, '}')
        } else {
            expanded.push('$');
            rest = &rest[1..];
            continue;
        };
        let Some(end) = rest[open..].find(close) else {
            expanded.push('$');
            rest = &rest[1..];
            continue;
        };
        let name = &rest[open..open + end];
        if let Some(value) = variables.get(name) {
            expanded.push_str(&expand_make(value, variables, depth + 1)?);
        } else {
            expanded.push_str(&rest[..=(open + end)]);
        }
        rest = &rest[open + end + 1..];
        if expanded.len() > MAX_MAKE_EXPANSION {
            return Err(ClassificationError::MakeExpansionTooLarge);
        }
    }
    expanded.push_str(rest);
    if expanded.len() > MAX_MAKE_EXPANSION {
        return Err(ClassificationError::MakeExpansionTooLarge);
    }
    Ok(expanded)
}

fn make_commands(text: &str) -> Result<Vec<(String, String)>, ClassificationError> {
    let variables = make_variables(text)?;
    let mut commands = Vec::new();
    let mut target = String::new();
    for line in text.lines() {
        if !line.starts_with('\t')
            && let Some((name, _)) = line.split_once(':')
            && !name.contains('=')
            && !name.starts_with('.')
        {
            name.trim().clone_into(&mut target);
            for name in name.split_whitespace() {
                if !name.is_empty() {
                    commands.push((format!("target:{name}"), format!("make {name}")));
                }
            }
        }
        if line.starts_with('\t') {
            let command = line.trim().trim_start_matches(['@', '-', '+']);
            commands.push((target.clone(), expand_make(command, &variables, 0)?));
        } else if let Some((name, value)) = make_assignment(line) {
            commands.push((
                format!("assignment:{name}"),
                expand_make(value, &variables, 0)?,
            ));
        }
    }
    Ok(commands)
}

fn action_env_values(text: &str) -> Result<BTreeMap<String, String>, ClassificationError> {
    let mut values = BTreeMap::new();
    let mut env_indent = None;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();
        if trimmed == "env:" {
            env_indent = Some(indent);
            continue;
        }
        if let Some(start) = env_indent {
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if indent <= start {
                env_indent = None;
                continue;
            }
            if let Some((name, value)) = trimmed.split_once(':') {
                let name = name.trim();
                let value = value.trim().trim_matches(['"', '\'']);
                if !name.is_empty()
                    && name.bytes().all(|byte| {
                        byte.is_ascii_uppercase() || byte == b'_' || byte.is_ascii_digit()
                    })
                    && non_rust_launch(value)
                {
                    values.insert(name.to_owned(), value.to_owned());
                    if values.len() > MAX_MAKE_VARIABLES {
                        return Err(ClassificationError::ActionExpansionTooLarge);
                    }
                }
            }
        }
    }
    Ok(values)
}

fn expand_action_run(
    command: &str,
    variables: &BTreeMap<String, String>,
) -> Result<String, ClassificationError> {
    if command.len() > MAX_MAKE_EXPANSION {
        return Err(ClassificationError::ActionExpansionTooLarge);
    }
    let mut expanded = command.to_owned();
    for (name, value) in variables {
        for reference in [format!("${name}"), format!("${{{name}}}")] {
            let count = expanded.matches(&reference).count();
            if count > 0 {
                let growth = value.len().saturating_mul(count);
                if expanded.len().saturating_add(growth) > MAX_MAKE_EXPANSION {
                    return Err(ClassificationError::ActionExpansionTooLarge);
                }
                expanded = expanded.replace(&reference, value);
            }
        }
    }
    Ok(expanded)
}

fn action_commands(path: &str, text: &str) -> Result<Vec<(String, String)>, ClassificationError> {
    let yaml: yaml_serde::Value = yaml_serde::from_str(text)
        .map_err(|_| ClassificationError::InvalidActionYaml(path.to_owned()))?;
    let variables = action_env_values(text)?;
    let mut commands = Vec::new();
    for (name, value) in &variables {
        commands.push((format!("env:{name}"), value.clone()));
    }
    let mut run_number = 0usize;
    collect_action_commands(&yaml, &variables, &mut run_number, &mut commands)?;
    Ok(commands)
}

fn collect_action_commands(
    node: &yaml_serde::Value,
    variables: &BTreeMap<String, String>,
    run_number: &mut usize,
    commands: &mut Vec<(String, String)>,
) -> Result<(), ClassificationError> {
    match node {
        yaml_serde::Value::Mapping(mapping) => {
            for (key, value) in mapping {
                let Some(key) = key.as_str() else { continue };
                match key {
                    "run" => {
                        let Some(command) = value.as_str() else {
                            continue;
                        };
                        *run_number += 1;
                        let multiline = command.lines().count() > 1;
                        for (line, command) in command.lines().enumerate() {
                            let context = if multiline {
                                format!("run-{run_number}-line-{line}")
                            } else {
                                format!("run-{run_number}")
                            };
                            commands.push((context, expand_action_run(command, variables)?));
                        }
                    }
                    "using"
                        if value.as_str().is_some_and(|using| {
                            using.starts_with("node") || using == "docker"
                        }) =>
                    {
                        commands.push((
                            "action-runtime".to_owned(),
                            value.as_str().unwrap_or_default().to_owned(),
                        ));
                    }
                    "image"
                        if value
                            .as_str()
                            .is_some_and(|image| image.starts_with("docker://")) =>
                    {
                        commands.push((
                            "action-runtime".to_owned(),
                            value.as_str().unwrap_or_default().to_owned(),
                        ));
                    }
                    "shell"
                        if value
                            .as_str()
                            .is_some_and(|shell| !matches!(shell, "bash" | "sh")) =>
                    {
                        commands.push((
                            "action-shell".to_owned(),
                            value.as_str().unwrap_or_default().to_owned(),
                        ));
                    }
                    "uses" if value.as_str().is_some_and(external_non_rust_action) => {
                        commands.push((
                            "external-action".to_owned(),
                            value.as_str().unwrap_or_default().to_owned(),
                        ));
                    }
                    _ => collect_action_commands(value, variables, run_number, commands)?,
                }
            }
        }
        yaml_serde::Value::Sequence(items) => {
            for value in items {
                collect_action_commands(value, variables, run_number, commands)?;
            }
        }
        yaml_serde::Value::Tagged(tagged) => {
            collect_action_commands(&tagged.value, variables, run_number, commands)?;
        }
        _ => {}
    }
    Ok(())
}

fn external_non_rust_action(action: &str) -> bool {
    if action.starts_with("actions/checkout@") || action.starts_with("./.github/workflows/") {
        return false;
    }
    let trusted_workflow =
        "agent-ix/engineering-assurance/.github/workflows/language-boundary-reusable.yml@";
    if let Some(revision) = action.strip_prefix(trusted_workflow)
        && (revision == "main"
            || (revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit())))
    {
        return false;
    }
    true
}

fn non_rust_launch(command: &str) -> bool {
    if ["$(PYTHON)", "$(NODE)", "$(SHELL)", "${PYTHON}", "${NODE}"]
        .iter()
        .any(|alias| command.contains(alias))
    {
        return true;
    }
    let normalized = command.replace(['"', '\'', ';', '|', '(', ')', ':', '='], " ");
    normalized.split_whitespace().any(|token| {
        let token = token.trim_start_matches(['@', '-', '+']);
        let basename = token.rsplit('/').next().unwrap_or(token);
        matches!(
            basename,
            "python"
                | "python3"
                | "python2"
                | "node"
                | "nodejs"
                | "npx"
                | "npm"
                | "bun"
                | "deno"
                | "tsx"
                | "ts-node"
                | "bash"
                | "sh"
                | "zsh"
                | "ruby"
                | "perl"
                | "php"
                | "pwsh"
                | "powershell"
                | "java"
                | "javac"
                | "mvn"
                | "gradle"
                | "electron"
                | "jupyter"
                | "pytest"
        ) || basename.starts_with("python3.")
            || basename.strip_prefix("node").is_some_and(|tail| {
                !tail.is_empty() && tail.bytes().all(|byte| byte.is_ascii_digit())
            })
            || ["py", "sh", "js", "mjs", "cjs"]
                .iter()
                .any(|suffix| has_extension(basename, suffix))
    })
}

fn unresolved_command_position(command: &str, make: bool) -> bool {
    let mut command = command.trim_start().trim_start_matches(['"', '\'']);
    if let Some(rest) = command
        .strip_prefix("env ")
        .or_else(|| command.strip_prefix("sudo "))
    {
        command = rest.trim_start();
    }
    if make && command.starts_with("$(MAKE)") {
        return false;
    }
    command.starts_with('$')
}

fn risky_make_targets(entries: &[Entry]) -> Result<BTreeSet<String>, ClassificationError> {
    let mut targets: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for entry in entries.iter().filter(|entry| is_makefile(&entry.path)) {
        let text = std::str::from_utf8(&entry.bytes)
            .map_err(|_| ClassificationError::NonUtf8(entry.path.clone()))?;
        let variables = make_variables(text)?;
        let mut current = Vec::new();
        for line in text.lines() {
            if let Some((name, deps)) = line.split_once(':')
                && !line.starts_with('\t')
                && !name.contains('=')
                && !name.starts_with('.')
            {
                current = name.split_whitespace().map(str::to_owned).collect();
                for target in &current {
                    targets
                        .entry(target.clone())
                        .or_default()
                        .0
                        .extend(deps.split_whitespace().map(str::to_owned));
                    if targets.len() > MAX_MAKE_TARGETS {
                        return Err(ClassificationError::TooManyMakeTargets);
                    }
                }
                continue;
            }
            if line.starts_with('\t') {
                for target in &current {
                    targets
                        .entry(target.clone())
                        .or_default()
                        .1
                        .push(expand_make(line.trim(), &variables, 0)?);
                }
            }
        }
    }
    let mut risky: BTreeSet<String> = targets
        .iter()
        .filter(|(_, (_, commands))| commands.iter().any(|command| non_rust_launch(command)))
        .map(|(name, _)| name.clone())
        .collect();
    loop {
        let next: BTreeSet<String> = targets
            .iter()
            .filter(|(_, (deps, commands))| {
                deps.iter().any(|dep| risky.contains(dep))
                    || commands
                        .iter()
                        .any(|line| make_launches_risky_target(line, &risky))
            })
            .map(|(name, _)| name.clone())
            .collect();
        if next.is_subset(&risky) {
            break;
        }
        risky.extend(next);
    }
    Ok(risky)
}

fn make_launches_risky_target(command: &str, risky: &BTreeSet<String>) -> bool {
    let tokens: Vec<_> = command.split_whitespace().collect();
    tokens
        .windows(2)
        .any(|pair| matches!(pair[0], "make" | "$(MAKE)" | "gmake") && risky.contains(pair[1]))
}

fn classify_package(
    entry: &Entry,
    findings: &mut BTreeSet<Finding>,
) -> Result<(), ClassificationError> {
    if entry.path.split('/').any(|part| part == "templates")
        && entry
            .bytes
            .windows(2)
            .any(|window| window == b"{{" || window == b"{%")
    {
        findings.insert(Finding {
            path: entry.path.clone(),
            kind: "templated-npm-manifest",
            evidence: format!("sha256:{}", hex_digest(&entry.bytes)),
        });
        return Ok(());
    }
    let value: serde_json::Value = serde_json::from_slice(&entry.bytes)
        .map_err(|_| ClassificationError::InvalidManifest(entry.path.clone()))?;
    if let Some(bin) = value.get("bin").and_then(serde_json::Value::as_str) {
        findings.insert(Finding {
            path: entry.path.clone(),
            kind: "npm-bin",
            evidence: format!("bin:{bin}"),
        });
    }
    for field in [
        "scripts",
        "bin",
        "dependencies",
        "devDependencies",
        "optionalDependencies",
    ] {
        let Some(map) = value.get(field) else {
            continue;
        };
        if field == "bin" && map.is_string() {
            continue;
        }
        let map = map
            .as_object()
            .ok_or_else(|| ClassificationError::InvalidManifestShape(entry.path.clone()))?;
        for (name, value) in map {
            let command = value
                .as_str()
                .ok_or_else(|| ClassificationError::InvalidManifestShape(entry.path.clone()))?;
            findings.insert(Finding {
                path: entry.path.clone(),
                kind: match field {
                    "bin" => "npm-bin",
                    "scripts" => "npm-script",
                    _ => "npm-dependency",
                },
                evidence: if matches!(field, "bin" | "scripts") {
                    format!("{name}:{command}")
                } else {
                    format!("{field}:{name}:{command}")
                },
            });
        }
    }
    Ok(())
}
