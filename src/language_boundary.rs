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
use thiserror::Error;

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
        || executable_location(path)
        || [
            "py", "sh", "bash", "js", "mjs", "cjs", "ts", "tsx", "rb", "pl", "rs",
        ]
        .iter()
        .any(|suffix| has_extension(path, suffix))
        || !path.rsplit('/').next().unwrap_or(path).contains('.')
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
        if path == "package.json" || path.ends_with("/package.json") {
            classify_package(entry, &mut findings)?;
        }
        if has_extension(path, "rs")
            && let Some(text) = text
        {
            for line in text.lines() {
                let dynamic = line
                    .split_once("Command::new(")
                    .is_some_and(|(_, argument)| {
                        !argument.trim_start().starts_with('"')
                            && !argument.trim_start().starts_with("r#\"")
                    });
                if (line.contains("Command::new")
                    || line.contains("include_str!")
                    || line.contains("include_bytes!"))
                    && (non_rust_launch(line) || dynamic)
                {
                    findings.insert(Finding {
                        path: entry.path.clone(),
                        kind: if dynamic {
                            "dynamic-rust-launch"
                        } else {
                            "rust-wrapper-launch"
                        },
                        evidence: line.trim().to_owned(),
                    });
                }
            }
        }
        if is_makefile(path) || is_actions_file(path) {
            let text = text.ok_or_else(|| ClassificationError::NonUtf8(entry.path.clone()))?;
            let commands = if is_actions_file(path) {
                action_commands(text)
            } else {
                make_commands(text)
            };
            let mut occurrences = BTreeMap::new();
            for (context, command) in commands {
                let direct = context == "external-action" || non_rust_launch(&command);
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
    path.split('/')
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
        || path.starts_with("corpus/cases/"))
        && (path.contains("/input/") || path.contains("/samples/"))
}

fn is_makefile(path: &str) -> bool {
    path == "Makefile" || path.ends_with("/Makefile") || has_extension(path, "mk")
}

fn is_actions_file(path: &str) -> bool {
    (path.starts_with(".github/workflows/") || path.starts_with(".github/actions/"))
        && (has_extension(path, "yml") || has_extension(path, "yaml"))
}

fn make_command(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
        return None;
    }
    if line.starts_with('\t') {
        return Some(trimmed.trim_start_matches(['@', '-', '+']));
    }
    let (name, value) = trimmed.split_once('=')?;
    let name = name.trim_end_matches([':', '?', '+', ' ']).trim();
    if matches!(name, "PYTHON" | "NODE" | "SHELL" | "SCRIPT" | "RUNNER") {
        return Some(value);
    }
    None
}

fn make_commands(text: &str) -> Vec<(String, String)> {
    let mut commands = Vec::new();
    let mut target = String::new();
    for line in text.lines() {
        if !line.starts_with('\t')
            && let Some((name, _)) = line.split_once(':')
            && !name.contains('=')
            && !name.starts_with('.')
        {
            name.trim().clone_into(&mut target);
        }
        if let Some(command) = make_command(line) {
            let context = if line.starts_with('\t') {
                target.as_str()
            } else {
                "assignment"
            };
            commands.push((context.to_owned(), command.to_owned()));
        }
    }
    commands
}

fn action_commands(text: &str) -> Vec<(String, String)> {
    let mut commands = Vec::new();
    let mut block_indent = None;
    let mut run_number = 0usize;
    for line in text.lines() {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();
        if let Some(start) = block_indent {
            if trimmed.is_empty() {
                continue;
            }
            if indent > start {
                commands.push((format!("run-{run_number}"), trimmed.to_owned()));
                continue;
            }
            block_indent = None;
        }
        let run = trimmed
            .strip_prefix("run:")
            .or_else(|| trimmed.strip_prefix("- run:"));
        if let Some(command) = run {
            run_number += 1;
            if matches!(command.trim(), "|" | ">" | "|-") {
                block_indent = Some(indent);
            } else {
                commands.push((format!("run-{run_number}"), command.trim().to_owned()));
            }
        } else if let Some(using) = trimmed.strip_prefix("using:")
            && using.trim().starts_with("node")
        {
            commands.push(("action-runtime".to_owned(), using.trim().to_owned()));
        } else if let Some(shell) = trimmed.strip_prefix("shell:")
            && !matches!(shell.trim(), "bash" | "sh")
        {
            commands.push(("action-shell".to_owned(), shell.trim().to_owned()));
        } else if let Some(action) = trimmed
            .strip_prefix("uses:")
            .or_else(|| trimmed.strip_prefix("- uses:"))
            && [
                "actions/github-script",
                "actions/setup-python",
                "actions/setup-node",
                "pnpm/action-setup",
            ]
            .iter()
            .any(|prefix| action.trim().starts_with(prefix))
        {
            commands.push(("external-action".to_owned(), action.trim().to_owned()));
        }
    }
    commands
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

fn risky_make_targets(entries: &[Entry]) -> Result<BTreeSet<String>, ClassificationError> {
    let mut targets: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for entry in entries.iter().filter(|entry| is_makefile(&entry.path)) {
        let text = std::str::from_utf8(&entry.bytes)
            .map_err(|_| ClassificationError::NonUtf8(entry.path.clone()))?;
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
                }
                continue;
            }
            if line.starts_with('\t') {
                for target in &current {
                    targets
                        .entry(target.clone())
                        .or_default()
                        .1
                        .push(line.trim().to_owned());
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
    let value: serde_json::Value = serde_json::from_slice(&entry.bytes)
        .map_err(|_| ClassificationError::InvalidManifest(entry.path.clone()))?;
    if let Some(bin) = value.get("bin").and_then(serde_json::Value::as_str) {
        findings.insert(Finding {
            path: entry.path.clone(),
            kind: "npm-bin",
            evidence: format!("bin:{bin}"),
        });
    }
    for field in ["scripts", "bin"] {
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
            if field == "bin" || field == "scripts" {
                findings.insert(Finding {
                    path: entry.path.clone(),
                    kind: if field == "bin" {
                        "npm-bin"
                    } else {
                        "npm-script"
                    },
                    evidence: format!("{name}:{command}"),
                });
            }
        }
    }
    Ok(())
}
