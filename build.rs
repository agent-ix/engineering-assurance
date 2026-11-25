// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Embeds the source commit used to build Engineering Assurance.

use std::process::Command;

fn main() {
    if let Some(tracked_files) = git_output(&["ls-files"]) {
        for path in tracked_files.lines().filter(|path| !path.is_empty()) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
    if let Some(head) = git_output(&["rev-parse", "--git-path", "HEAD"]) {
        println!("cargo:rerun-if-changed={head}");
    }
    if let Some(reference) = git_output(&["symbolic-ref", "-q", "HEAD"])
        && let Some(path) = git_output(&["rev-parse", "--git-path", &reference])
    {
        println!("cargo:rerun-if-changed={path}");
    }
    let clean = git_output(&["status", "--porcelain", "--untracked-files=normal"])
        .is_some_and(|status| status.is_empty());
    let revision = clean
        .then(|| git_output(&["rev-parse", "HEAD"]))
        .flatten()
        .filter(|revision| {
            matches!(revision.len(), 40 | 64)
                && revision
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        .unwrap_or_default();
    println!("cargo:rustc-env=EA_SOURCE_REVISION={revision}");
}

fn git_output(arguments: &[&str]) -> Option<String> {
    Command::new("git")
        .args(arguments)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|output| output.trim().to_owned())
}
