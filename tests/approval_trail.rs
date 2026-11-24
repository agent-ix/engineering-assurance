// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Contract tests for the pure approval-trail ordering and record test (FR-043).

use engineering_assurance::approval_trail::{
    OrderKey, compare, escape_name, hex, is_unprintable, names_record,
};
use ix_trace_rs::trace;

fn key<'a>(claimed_at: Option<&'a str>, tag: &'a str) -> OrderKey<'a> {
    OrderKey {
        claimed_at,
        tag: Some(tag),
        object: "",
        commit: "",
    }
}

fn ordered(mut keys: Vec<OrderKey<'static>>) -> Vec<&'static str> {
    keys.sort_by(compare);
    keys.iter().map(|k| k.tag.expect("a tag")).collect()
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

#[test]
#[trace("TC-363", "FR-043-AC-14")]
fn tc_363_names_are_escaped_byte_exactly() {
    assert_eq!(escape_name(b"plain"), ("plain".to_owned(), false));
    assert_eq!(escape_name(b"caf\xe9"), ("caf\\xe9".to_owned(), true));
    assert_eq!(
        escape_name("caf\u{e9}".as_bytes()),
        ("caf\u{e9}".to_owned(), false)
    );
    assert_eq!(escape_name(b"a\x1bb"), ("a\\x1bb".to_owned(), true));
    assert_eq!(hex(b"caf\xe9"), "636166e9");
}

#[test]
#[trace("TC-367", "FR-043-AC-18")]
fn tc_367_unprintable_covers_controls_and_format_characters() {
    for c in [
        '\u{1b}', '\u{85}', '\u{7f}', '\u{200e}', '\u{202e}', '\u{2066}', '\u{2069}', '\u{200b}',
        '\u{feff}', '\u{2028}',
    ] {
        assert!(is_unprintable(c), "{c:?}");
    }
    for c in ['a', ' ', '\u{e9}', '\u{4e2d}'] {
        assert!(!is_unprintable(c), "{c:?}");
    }
}

#[test]
#[trace("TC-351", "FR-043-AC-5")]
fn tc_351_equal_times_and_tags_order_by_object_then_commit_and_tagless_entries_last() {
    let at = Some("2026-06-01T12:00:00Z");
    let entry = |claimed_at, tag, object, commit| OrderKey {
        claimed_at,
        tag,
        object,
        commit,
    };
    let mut keys = [
        entry(None, None, "", "c2"),
        entry(None, Some("t"), "o2", "c1"),
        entry(None, Some("t"), "o1", "c9"),
        entry(None, Some("t"), "o1", "c1"),
        entry(None, None, "", "c1"),
        entry(at, Some("z"), "o0", ""),
    ];
    keys.sort_by(compare);
    let order: Vec<_> = keys.iter().map(|k| (k.tag, k.object, k.commit)).collect();
    assert_eq!(
        order,
        vec![
            (Some("z"), "o0", ""),
            (Some("t"), "o1", "c1"),
            (Some("t"), "o1", "c9"),
            (Some("t"), "o2", "c1"),
            (None, "", "c1"),
            (None, "", "c2"),
        ]
    );
}
