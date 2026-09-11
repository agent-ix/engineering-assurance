// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Compatibility-classification parity against checked-in reference bytes.
//!
//! The reference is a file, not a process. It was captured once from the
//! retained Python classifier at the candidate revision that cut this capability
//! over, and it is committed here so the cutover could delete that classifier
//! without deleting the evidence that Rust reproduces it. Executing the retained
//! implementation as an oracle would have made the two inseparable.

use engineering_assurance::compatibility::{REQUEST_PROTOCOL, evaluate_request_bytes};
use ix_trace_rs::trace;

/// Classifications the retained Python implementation produced for the request
/// below, captured verbatim and compared byte for byte.
const REFERENCE_CLASSIFICATIONS: &[u8] =
    include_bytes!("fixtures/compatibility-classifications.json");

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

    let reference = REFERENCE_CLASSIFICATIONS
        .strip_suffix(b"\n")
        .unwrap_or(REFERENCE_CLASSIFICATIONS);
    assert_eq!(
        String::from_utf8_lossy(&rust_bytes),
        String::from_utf8_lossy(reference),
        "Rust classifications drifted from the captured reference bytes"
    );

    // The reference is only evidence while it still describes this matrix. A
    // pin that moved without the reference moving with it would otherwise pass
    // silently, comparing Rust against a record of a matrix that no longer
    // exists.
    let captured: Vec<serde_json::Value> =
        serde_json::from_slice(reference).expect("the captured reference must be JSON");
    assert_eq!(captured.len(), rust.components.len());
    for (recorded, produced) in captured.iter().zip(&rust.components) {
        assert_eq!(
            recorded["component"].as_str(),
            Some(produced.component.as_str())
        );
        assert_eq!(
            recorded["expected"].as_str(),
            Some(produced.expected.as_str())
        );
    }
}
