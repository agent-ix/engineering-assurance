// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Historical `AssuranceProfile` version spelling stays retired-only.

use std::{fs, path::PathBuf};

use ix_trace_rs::trace;
use serde_json::{Value, json};

fn validator() -> jsonschema::Validator {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("engineering_assurance/schemas/assurance-profile-frontmatter.schema.json");
    let schema: Value = serde_json::from_slice(&fs::read(path).expect("packaged schema exists"))
        .expect("packaged schema is JSON");
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(&schema)
        .expect("packaged schema compiles")
}

#[trace("TC-173", "FR-025-AC-8")]
#[test]
fn retired_profile_accepts_historical_spelling_only() {
    let validator = validator();
    let old = json!({
        "id": "AP-001",
        "title": "Historical source-release profile",
        "type": "AssuranceProfile",
        "status": "retired",
        "owner": "release-owner",
        "profile_version": 0.2,
        "profile_kind": "general",
        "scope": "one old source candidate",
        "relationships": []
    });
    assert!(validator.is_valid(&old));

    for status in ["proposed", "active"] {
        let mut changed = old.clone();
        changed["status"] = json!(status);
        assert!(
            !validator.is_valid(&changed),
            "{status} cannot use legacy spelling"
        );
    }

    let mut invalid_version = old.clone();
    invalid_version["profile_version"] = json!(0.1);
    assert!(!validator.is_valid(&invalid_version));

    let mut ambiguous = old.clone();
    ambiguous["schema_version"] = json!(0.2);
    assert!(!validator.is_valid(&ambiguous));

    let mut current = old;
    current.as_object_mut().unwrap().remove("profile_version");
    current["schema_version"] = json!(0.2);
    assert!(validator.is_valid(&current), "retired current spelling");
    current["status"] = json!("active");
    assert!(validator.is_valid(&current), "active current spelling");
}
