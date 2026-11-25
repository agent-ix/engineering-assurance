// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

use std::process::Command;

use serde_json::{Value, json};

pub fn assert_producer(result: &Value) {
    assert_eq!(result["ea_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(result["ea_source_revision"], expected_source_revision());
    assert_eq!(result["generated_at_source"], "host_clock");
    let generated_at = result["generated_at"].as_str().expect("generated_at");
    assert_eq!(generated_at.len(), 20, "{generated_at}");
    assert!(generated_at.ends_with('Z'), "{generated_at}");
    assert!(
        generated_at
            .bytes()
            .enumerate()
            .all(|(index, byte)| match index {
                4 | 7 => byte == b'-',
                10 => byte == b'T',
                13 | 16 => byte == b':',
                19 => byte == b'Z',
                _ => byte.is_ascii_digit(),
            }),
        "{generated_at}"
    );
}

fn expected_source_revision() -> Value {
    let Some(status) = git(&["status", "--porcelain", "--untracked-files=normal"]) else {
        return Value::Null;
    };
    if !status.is_empty() {
        return Value::Null;
    }

    let Some(revision) = git(&["rev-parse", "HEAD"]) else {
        return Value::Null;
    };
    assert!(
        matches!(revision.len(), 40 | 64)
            && revision
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "invalid Git HEAD revision: {revision:?}"
    );
    json!(revision)
}

fn git(arguments: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_owned())
}
