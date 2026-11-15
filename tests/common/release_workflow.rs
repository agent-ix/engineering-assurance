// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

use crate::common::is_structural_external_action_reference;

pub(super) fn assert_action_references_and_checkouts(workflow: &serde_json::Value) {
    let jobs = workflow["jobs"].as_object().expect("workflow jobs");
    for (job_name, job) in jobs {
        let Some(steps) = job["steps"].as_array() else {
            continue;
        };
        for step in steps {
            let Some(reference) = step["uses"].as_str() else {
                continue;
            };
            if reference.starts_with("./") {
                continue;
            }
            assert!(
                is_structural_external_action_reference(reference),
                "external action reference has a valid owner/repository/path@ref shape in {job_name}: {reference}"
            );
            if reference.starts_with("actions/checkout@") {
                assert_eq!(
                    step["with"]["persist-credentials"], false,
                    "checkout does not retain credentials in {job_name}"
                );
            }
        }
    }
}
