// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Static checks for the first versioned native protocol schemas.

use engineering_assurance::compatibility::{MAX_REQUEST_BYTES, MAX_RESULT_BYTES};
use ix_trace_rs::trace;

const REQUEST_SCHEMA: &str =
    include_str!("../engineering_assurance/schemas/compatibility-request-v1.schema.json");
const RESULT_SCHEMA: &str =
    include_str!("../engineering_assurance/schemas/compatibility-result-v1.schema.json");
const ERROR_SCHEMA: &str = include_str!("../engineering_assurance/schemas/error-v1.schema.json");

#[trace("TC-121", "FR-014-AC-5")]
#[test]
fn tc_121_protocol_schemas_declare_the_enforced_byte_limits() {
    for (encoded, enforced) in [
        (REQUEST_SCHEMA, MAX_REQUEST_BYTES),
        (RESULT_SCHEMA, MAX_RESULT_BYTES),
        (ERROR_SCHEMA, MAX_RESULT_BYTES),
    ] {
        let schema: serde_json::Value =
            serde_json::from_str(encoded).expect("embedded protocol schema must be JSON");
        assert_eq!(
            schema["x-engineering-assurance-max-encoded-bytes"].as_u64(),
            u64::try_from(enforced).ok()
        );
        assert_eq!(
            schema["x-engineering-assurance-downstream-deadline-required"],
            false
        );
    }
}
