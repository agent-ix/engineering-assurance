// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

pub(super) fn is_structural_external_action_reference(reference: &str) -> bool {
    let Some((action, revision)) = reference.split_once('@') else {
        return false;
    };
    if revision.contains('@') || !non_empty_slash_segments(revision) {
        return false;
    }

    let action_segments = action.split('/').collect::<Vec<_>>();
    action_segments.len() >= 2
        && action_segments
            .iter()
            .all(|segment| valid_action_segment(segment))
}

fn non_empty_slash_segments(value: &str) -> bool {
    !value.is_empty() && value.split('/').all(valid_action_segment)
}

fn valid_action_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.chars().any(char::is_whitespace)
        && !segment.contains('@')
}
