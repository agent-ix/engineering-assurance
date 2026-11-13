// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Contract tests for the pure approval decision (FR-041).

use engineering_assurance::approval_verify::{
    ApprovalStatus, Binding, Bindings, MAX_RECORD_VALUE_BYTES, TrailerUnreadable,
    UnverifiableReason, bind, decide, is_record_value,
};
use engineering_assurance::signature_verify::{
    Mapping, Presence, Reason, SignatureKind, SignedOffMeaning, Signer, SignerSource, Status,
    TargetKind, Verdict, tagger_signed_at, trailer_values,
};
use ix_trace_rs::trace;
use serde_json::json;

const STATUSES: [Status; 5] = [
    Status::SignedValid,
    Status::SignedUntrusted,
    Status::SignedBad,
    Status::Unsigned,
    Status::Unverifiable,
];
const BINDINGS: [Binding; 4] = [
    Binding::Match,
    Binding::Mismatch,
    Binding::Missing,
    Binding::Repeated,
];

fn verdict(status: Status, meaning: Option<Presence>) -> Verdict {
    let signer = match status {
        Status::SignedValid => Some(Signer {
            source: SignerSource::AllowedSigners,
            mapping: Mapping::Mapped,
            principal: Some("alice".to_owned()),
        }),
        Status::SignedUntrusted => Some(Signer {
            source: SignerSource::AllowedSigners,
            mapping: Mapping::Unmapped,
            principal: None,
        }),
        _ => None,
    };
    Verdict {
        status,
        kind: TargetKind::Tag,
        signature_kind: Some(SignatureKind::Ssh),
        key_fingerprint: None,
        reason: (status == Status::Unverifiable).then_some(Reason::Timeout),
        signer,
        certificate: None,
        mismatch: None,
        signed_off_meaning: meaning.map(|presence| SignedOffMeaning {
            presence,
            values: if presence == Presence::Present {
                vec!["reviewed".to_owned(), "released".to_owned()]
            } else {
                Vec::new()
            },
        }),
    }
}

/// The one combination CON-1 allows `approved` for.
fn is_approval(status: Status, bindings: Option<Bindings>, meaning: Option<Presence>) -> bool {
    status == Status::SignedValid
        && bindings
            == Some(Bindings {
                record_id: Binding::Match,
                record_digest: Binding::Match,
            })
        && meaning == Some(Presence::Present)
}

#[test]
#[trace("TC-310", "FR-041-AC-9", "FR-041-CON-1")]
fn tc_310_every_combination_decides_and_only_one_is_approved() {
    let mut combinations = Vec::new();
    for id in BINDINGS {
        for digest in BINDINGS {
            combinations.push(Some(Bindings {
                record_id: id,
                record_digest: digest,
            }));
        }
    }
    combinations.push(None);
    let mut approved = 0;
    let mut total = 0;
    for status in STATUSES {
        for bindings in &combinations {
            for meaning in [None, Some(Presence::Missing), Some(Presence::Present)] {
                total += 1;
                let decision = decide(&verdict(status, meaning), *bindings, None);
                let expected = is_approval(status, *bindings, meaning);
                assert_eq!(
                    decision.status == ApprovalStatus::Approved,
                    expected,
                    "{status:?} {bindings:?} {meaning:?}"
                );
                assert_eq!(decision.approval.is_some(), expected);
                // The signing time fills an approval and changes no status.
                let timed = decide(
                    &verdict(status, meaning),
                    *bindings,
                    Some("2026-03-04T04:30:00Z".to_owned()),
                );
                assert_eq!(timed.status, decision.status);
                assert_eq!(timed.reason, decision.reason);
                assert_eq!(
                    decision.reason.is_some(),
                    decision.status == ApprovalStatus::Unverifiable
                );
                approved += usize::from(expected);
            }
        }
    }
    assert_eq!(total, 5 * 17 * 3);
    assert_eq!(approved, 1);
}

#[test]
#[trace("TC-311", "FR-041-AC-9")]
fn tc_311_signature_status_decides_before_the_binding_and_the_binding_before_the_meaning() {
    let wrong = Some(Bindings {
        record_id: Binding::Mismatch,
        record_digest: Binding::Repeated,
    });
    let status =
        |status, bindings, meaning| decide(&verdict(status, meaning), bindings, None).status;
    // Rule 1 wins over any binding.
    assert_eq!(
        status(Status::Unsigned, wrong, None),
        ApprovalStatus::Unsigned
    );
    assert_eq!(
        status(Status::SignedBad, wrong, None),
        ApprovalStatus::SignedBad
    );
    assert_eq!(
        status(Status::SignedUntrusted, wrong, None),
        ApprovalStatus::UntrustedSigner
    );
    assert_eq!(
        status(Status::Unverifiable, wrong, None),
        ApprovalStatus::Unverifiable
    );
    // A valid tag with a wrong binding is wrong_record even without a meaning.
    assert_eq!(
        status(Status::SignedValid, wrong, Some(Presence::Missing)),
        ApprovalStatus::WrongRecord
    );
    let right = Some(Bindings {
        record_id: Binding::Match,
        record_digest: Binding::Match,
    });
    assert_eq!(
        status(Status::SignedValid, right, Some(Presence::Missing)),
        ApprovalStatus::MeaningMissing
    );
    let approved = decide(
        &verdict(Status::SignedValid, Some(Presence::Present)),
        right,
        None,
    );
    let approval = approved.approval.expect("approval members");
    assert_eq!(approval.approver, "alice");
    assert_eq!(approval.meaning, ["reviewed", "released"]);
    assert_eq!(approval.signature_kind, SignatureKind::Ssh);
    assert_eq!(approved.reason, None);
}

#[test]
#[trace("TC-312", "FR-041-AC-15", "FR-041-AC-6")]
fn tc_312_an_unreadable_message_is_unverifiable_and_a_verdict_reason_is_carried() {
    let decision = decide(&verdict(Status::SignedValid, None), None, None);
    assert_eq!(decision.status, ApprovalStatus::Unverifiable);
    assert_eq!(
        decision.reason,
        Some(UnverifiableReason::Trailer(
            TrailerUnreadable::TrailerUnreadable
        ))
    );
    let carried = decide(&verdict(Status::Unverifiable, None), None, None);
    assert_eq!(
        carried.reason,
        Some(UnverifiableReason::Signature(Reason::Timeout))
    );
}

#[test]
#[trace("TC-313", "FR-041-AC-3", "FR-041-AC-2")]
fn tc_313_a_trailer_is_compared_by_exact_bytes_and_counted_per_occurrence() {
    let values = |items: &[&str]| items.iter().map(|v| (*v).to_owned()).collect::<Vec<_>>();
    assert_eq!(bind(&values(&[]), "x"), Binding::Missing);
    assert_eq!(bind(&values(&["x"]), "x"), Binding::Match);
    assert_eq!(bind(&values(&["X"]), "x"), Binding::Mismatch);
    assert_eq!(bind(&values(&["x "]), "x"), Binding::Mismatch);
    assert_eq!(
        bind(&values(&["sha256:abc1"]), "sha256:abc2"),
        Binding::Mismatch
    );
    assert_eq!(bind(&values(&["x", "x"]), "x"), Binding::Repeated);
    assert_eq!(bind(&values(&["x", "y"]), "x"), Binding::Repeated);
}

#[test]
#[trace("TC-314", "FR-041-AC-3")]
fn tc_314_trailer_values_match_the_token_case_insensitively_and_trim_ascii_only() {
    // The shape `git interpret-trailers --parse` prints: one unfolded
    // `token: value` line per trailer.
    let parsed = "Approved-Record-Id: one\nAPPROVED-record-id: two\nOther: x\n\
                  Approved-Record-Digest: abc def\nEmpty:\n";
    assert_eq!(trailer_values(parsed, "Approved-Record-Id"), ["one", "two"]);
    assert_eq!(
        trailer_values(parsed, "Approved-Record-Digest"),
        ["abc def"]
    );
    assert_eq!(trailer_values(parsed, "Empty"), [""]);
    assert!(trailer_values(parsed, "Absent").is_empty());
    // Only ASCII whitespace is trimmed: a no-break space stays in the value.
    assert_eq!(trailer_values("K: v\u{a0}\n", "K"), ["v\u{a0}"]);
    assert_eq!(trailer_values("K:\t v \n", "K"), ["v"]);
}

#[test]
#[trace("TC-315", "FR-041-AC-10")]
fn tc_315_a_record_value_is_bounded_plain_and_not_option_like() {
    assert!(is_record_value("sha256:0123"));
    assert!(is_record_value(&"a".repeat(MAX_RECORD_VALUE_BYTES)));
    assert!(!is_record_value(""));
    assert!(!is_record_value(&"a".repeat(MAX_RECORD_VALUE_BYTES + 1)));
    assert!(!is_record_value("-digest"));
    assert!(!is_record_value("a\nb"));
    assert!(!is_record_value("a\u{7}b"));
}

const COMMIT: &str = "object 0123456789012345678901234567890123456789\ntype commit\ntag v1\n";

fn tag_with_tagger(tagger: &str) -> Vec<u8> {
    format!("{COMMIT}tagger {tagger}\n\nsubject\n").into_bytes()
}

#[test]
#[trace("TC-331", "FR-041-AC-16", "FR-041-AC-17")]
fn tc_331_the_tagger_line_gives_a_utc_instant_and_anything_else_gives_none() {
    let at = |tagger: &str| tagger_signed_at(&tag_with_tagger(tagger));
    // The epoch is the instant; the zone does not move it.
    assert_eq!(
        at("A B <a@x.test> 1772598600 +0530").as_deref(),
        Some("2026-03-04T04:30:00Z")
    );
    assert_eq!(
        at("A B <a@x.test> 1772598600 -0800").as_deref(),
        Some("2026-03-04T04:30:00Z")
    );
    assert_eq!(
        at("A <a@x.test> 0 +0000").as_deref(),
        Some("1970-01-01T00:00:00Z")
    );
    assert_eq!(
        at("A <a@x.test> 951782400 +0000").as_deref(),
        Some("2000-02-29T00:00:00Z")
    );
    assert_eq!(
        at("A <a@x.test> 253402300799 +0000").as_deref(),
        Some("9999-12-31T23:59:59Z")
    );
    // A name holding `>` is read from the last `>`.
    assert_eq!(
        at("A > B <a@x.test> 1700000000 +0000").as_deref(),
        Some("2023-11-14T22:13:20Z")
    );
    for bad in [
        "A <a@x.test> 253402300800 +0000",
        "A <a@x.test> 1700000000",
        "A <a@x.test> 1700000000 +0000 extra",
        "A <a@x.test> 17e8 +0000",
        "A <a@x.test> -1 +0000",
        "A <a@x.test> 1700000000 0000",
        "A <a@x.test> 1700000000 +0060",
        "A <a@x.test> 1700000000 +00",
        "A <a@x.test> 1234567890123 +0000",
        "A <a@x.test> 999999999999 +0000",
        "A <a@x.test>  1700000000 +0000",
        "A <a@x.test>",
        "no email 1700000000 +0000",
    ] {
        assert_eq!(at(bad), None, "{bad}");
    }
}

#[test]
#[trace("TC-332", "FR-041-AC-17")]
fn tc_332_only_the_one_header_tagger_line_counts_never_message_text() {
    let none = |object: &str| tagger_signed_at(object.as_bytes());
    // No tagger header, though the message holds a tagger-shaped line.
    assert_eq!(
        none(&format!(
            "{COMMIT}\nsubject\n\ntagger A <a@x.test> 1700000000 +0000\n"
        )),
        None
    );
    // Two tagger headers are ambiguous.
    assert_eq!(
        none(&format!(
            "{COMMIT}tagger A <a@x.test> 1700000000 +0000\ntagger B <b@x.test> 1700000001 +0000\n\nm\n"
        )),
        None
    );
    // A header-only object still reads its tagger.
    assert_eq!(
        none(&format!("{COMMIT}tagger A <a@x.test> 1700000000 +0000\n")).as_deref(),
        Some("2023-11-14T22:13:20Z")
    );
    assert_eq!(tagger_signed_at(b"\xff\xfe"), None);
    assert_eq!(tagger_signed_at(b""), None);
}

#[test]
#[trace("TC-333", "FR-041-AC-16", "FR-041-AC-18", "FR-041-AC-19")]
fn tc_333_a_time_is_reported_only_on_approved_and_a_missing_one_is_explicit() {
    let right = Some(Bindings {
        record_id: Binding::Match,
        record_digest: Binding::Match,
    });
    let approved = |time: Option<&str>| {
        let decision = decide(
            &verdict(Status::SignedValid, Some(Presence::Present)),
            right,
            time.map(str::to_owned),
        );
        serde_json::to_value(decision.approval.expect("approval")).expect("json")
    };
    assert_eq!(
        approved(Some("2026-03-04T04:30:00Z")),
        json!({
            "approver": "alice",
            "meaning": ["reviewed", "released"],
            "signature_kind": "ssh",
            "signed_at": "2026-03-04T04:30:00Z",
            "signed_at_source": "tagger_date",
        })
    );
    assert_eq!(
        approved(None),
        json!({
            "approver": "alice",
            "meaning": ["reviewed", "released"],
            "signature_kind": "ssh",
            "signed_at_missing": "tagger_date_unreadable",
        })
    );
    // Every other status carries no approval members, so no time.
    let wrong = decide(
        &verdict(Status::SignedValid, Some(Presence::Present)),
        Some(Bindings {
            record_id: Binding::Match,
            record_digest: Binding::Mismatch,
        }),
        Some("2026-03-04T04:30:00Z".to_owned()),
    );
    assert_eq!(wrong.approval, None);
}
