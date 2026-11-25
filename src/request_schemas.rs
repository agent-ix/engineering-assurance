// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Published JSON Schemas for the CLI request contracts covered by FR-089.

const APPROVAL_TAG: &str = include_str!("request_schemas/approval-tag-request-v1.schema.json");
const RELEASE_DECISION: &str =
    include_str!("request_schemas/release-decision-request-v1.schema.json");
const RELEASE_HISTORY: &str =
    include_str!("request_schemas/release-history-request-v1.schema.json");
const RELEASE_VERIFY: &str = include_str!("request_schemas/release-verify-request-v1.schema.json");

/// Returns the request schema document for a command that publishes one.
pub(crate) fn for_command(command: &str) -> Option<&'static [u8]> {
    let document = match command {
        "approval-tag" => APPROVAL_TAG,
        "release-decision" => RELEASE_DECISION,
        "release-history" => RELEASE_HISTORY,
        "release-verify" => RELEASE_VERIFY,
        _ => return None,
    };
    Some(document.as_bytes())
}
