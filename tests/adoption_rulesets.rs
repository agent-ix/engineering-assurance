// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The example ruleset payloads of the adoption documentation (FR-083).

use std::{fs, path::Path};

use ix_trace_rs::trace;
use serde_json::{Value, json};

fn payload(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name);
    serde_json::from_slice(&fs::read(path).expect("example payload exists"))
        .expect("example payload is JSON")
}

fn rule_types(payload: &Value) -> Vec<&str> {
    payload["rules"]
        .as_array()
        .expect("rules")
        .iter()
        .map(|rule| rule["type"].as_str().expect("rule type"))
        .collect()
}

#[test]
#[trace("TC-1124", "FR-083-AC-2")]
fn the_example_payloads_have_the_fields_the_requirement_names() {
    let tag = payload("tag-protection-ruleset.json");
    assert_eq!(tag["target"], "tag");
    assert_eq!(tag["enforcement"], "active");
    assert_eq!(
        tag["conditions"]["ref_name"]["include"],
        json!(["refs/tags/*"])
    );
    assert_eq!(rule_types(&tag), ["deletion", "update"]);
    assert_eq!(tag["bypass_actors"], json!([]));

    let branch = payload("private-records-branch-ruleset.json");
    assert_eq!(branch["target"], "branch");
    assert_eq!(branch["enforcement"], "active");
    assert_eq!(
        branch["conditions"]["ref_name"]["include"],
        json!(["refs/heads/engineering-assurance-private-records"])
    );
    assert_eq!(rule_types(&branch), ["deletion", "non_fast_forward"]);
    assert_eq!(branch["bypass_actors"], json!([]));

    for payload in [&tag, &branch] {
        assert!(!payload.to_string().contains("refs/notes/"));
    }
}
