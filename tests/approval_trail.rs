// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Contract tests for the pure approval-trail ordering and record test (FR-043).

use engineering_assurance::approval_trail::{OrderKey, compare, names_record};
use ix_trace_rs::trace;

fn key<'a>(claimed_at: Option<&'a str>, tag: &'a str) -> OrderKey<'a> {
    OrderKey { claimed_at, tag }
}

fn ordered(mut keys: Vec<OrderKey<'static>>) -> Vec<&'static str> {
    keys.sort_by(compare);
    keys.iter().map(|k| k.tag).collect()
}

#[test]
#[trace("TC-351", "FR-043-AC-12", "FR-043-AC-5")]
fn tc_351_the_order_is_time_then_absent_times_then_tag_name() {
    let early = "2020-01-01T00:00:00Z";
    let late = "2026-06-01T12:00:00Z";
    let keys = vec![
        key(None, "z-none"),
        key(Some(late), "a-late"),
        key(None, "b-none"),
        key(Some(early), "z-early"),
        key(Some(late), "0-late"),
    ];
    let expected = vec!["z-early", "0-late", "a-late", "b-none", "z-none"];
    assert_eq!(ordered(keys.clone()), expected);
    let mut reversed = keys;
    reversed.reverse();
    assert_eq!(ordered(reversed), expected);
}

#[test]
#[trace("TC-351", "FR-043-AC-12")]
fn tc_351_a_tag_names_the_record_when_any_id_value_is_byte_equal() {
    let values = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert!(names_record(&values(&["r-1"]), "r-1"));
    assert!(names_record(&values(&["r-2", "r-1"]), "r-1"));
    assert!(!names_record(&values(&["r-2"]), "r-1"));
    assert!(!names_record(&values(&["R-1"]), "r-1"));
    assert!(!names_record(&values(&[" r-1"]), "r-1"));
    assert!(!names_record(&[], "r-1"));
}
