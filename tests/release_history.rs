// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Unit tests for the `release-history` request rules and record path grammar
//! (FR-052).

use engineering_assurance::release_history::{
    RecordPath, is_commit_hash, is_records_dir, is_tag_name, record_of_path,
};
use ix_trace_rs::trace;

#[trace("TC-513", "FR-052-AC-11", "FR-052-AC-17")]
#[test]
fn tc_513_the_record_id_is_one_to_256_bytes_and_the_revision_is_an_unsigned_integer_from_one() {
    let long = "k".repeat(256);
    let too_long = "k".repeat(257);
    assert_eq!(
        record_of_path("records", format!("records/{long}/1.json").as_bytes()),
        RecordPath::Record(long.as_str(), 1)
    );
    assert_eq!(
        record_of_path("records/", b"records/a/18446744073709551615.json"),
        RecordPath::Record("a", u64::MAX)
    );
    for path in [
        format!("records/{too_long}/1.json"),
        "records/../1.json".to_owned(),
        "records/./1.json".to_owned(),
        "records//1.json".to_owned(),
        "records/a/0.json".to_owned(),
        "records/a/01.json".to_owned(),
        "records/a/-1.json".to_owned(),
        "records/a/.json".to_owned(),
        "other/a/1.json".to_owned(),
        "records-x/a/1.json".to_owned(),
    ] {
        assert_eq!(
            record_of_path("records", path.as_bytes()),
            RecordPath::NotARecord,
            "{path}"
        );
    }
    assert_eq!(
        record_of_path("records", b"records/a/18446744073709551616.json"),
        RecordPath::Unrepresentable
    );
    assert_eq!(
        record_of_path("records", b"records/\xff/1.json"),
        RecordPath::Unrepresentable
    );
    assert_eq!(
        record_of_path("records", b"records/\xff/x.json"),
        RecordPath::NotARecord
    );
}

#[trace("TC-514", "FR-052-AC-12")]
#[test]
fn tc_514_request_values_outside_the_grammar_are_not_accepted() {
    assert!(is_commit_hash(&"a".repeat(40)));
    assert!(is_commit_hash(&"0".repeat(64)));
    for hash in [
        "",
        "main",
        &"a".repeat(39),
        &"A".repeat(40),
        &"g".repeat(40),
    ] {
        assert!(!is_commit_hash(hash), "{hash}");
    }
    assert!(is_records_dir("records") && is_records_dir("a/b/"));
    for dir in ["", "/a", "a/../b", "a/*", "-a", "a//b", "."] {
        assert!(!is_records_dir(dir), "{dir}");
    }
    assert!(is_tag_name("v1.0") && is_tag_name("rel/1"));
    assert!(is_tag_name("v{1}"));
    for tag in [
        "", "-v", "v^{tree}", "a..b", "a b", "v~1", "a:b", "v1/", "a@{1}", "a\u{7}",
    ] {
        assert!(!is_tag_name(tag), "{tag}");
    }
}
