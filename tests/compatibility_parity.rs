// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Additive-migration parity with the retained Python compatibility classifier.

use std::process::Command;

use engineering_assurance::compatibility::{REQUEST_PROTOCOL, evaluate_request_bytes};
use ix_trace_rs::trace;

const PYTHON_REFERENCE: &str = r#"
import json
from engineering_assurance.compatibility import classify_all, load_matrix

observed = {
    "quire-cli": "0.31.0",
    "quoin": "0.22.5",
    "ix-flow": "99.0.0",
    "engineering-assurance": None,
}
items = classify_all(load_matrix(), observed)
print(json.dumps([
    {
        "component": item.component,
        "observed": item.observed,
        "expected": item.expected,
        "verdict": item.verdict,
        "reason": item.reason,
    }
    for item in items
], separators=(",", ":")))
"#;

#[trace("TC-100", "FR-015-AC-1")]
#[test]
fn tc_100_compatibility_classifications_match_the_retained_reference_bytes() {
    let request = serde_json::json!({
        "protocol": REQUEST_PROTOCOL,
        "observed": [
            {"component": "quire-cli", "version": "0.31.0"},
            {"component": "quoin", "version": "0.22.5"},
            {"component": "ix-flow", "version": "99.0.0"},
            {"component": "engineering-assurance", "version": null}
        ]
    });
    let rust =
        evaluate_request_bytes(&serde_json::to_vec(&request).expect("test request must serialize"))
            .expect("Rust classifier must accept the parity request");
    let rust_bytes =
        serde_json::to_vec(&rust.components).expect("Rust classifications must serialize");

    let python = Command::new("python3")
        .args(["-c", PYTHON_REFERENCE])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("retained Python reference must execute during additive migration");
    assert!(
        python.status.success(),
        "{}",
        String::from_utf8_lossy(&python.stderr)
    );
    assert_eq!(
        rust_bytes,
        python.stdout.strip_suffix(b"\n").unwrap_or(&python.stdout)
    );
}
