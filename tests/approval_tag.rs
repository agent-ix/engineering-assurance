// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Contract tests for the pure approval-tag message and preconditions (FR-042).

use engineering_assurance::approval_tag::{
    MAX_MEANING_BYTES, SigningSetting, compose_message, is_meaning, is_record_digest,
    is_trailer_record_value, signing_refusal, tag_is_signed,
};
use engineering_assurance::approval_verify::{RECORD_DIGEST_TRAILER, RECORD_ID_TRAILER};
use engineering_assurance::signature_verify::{MEANING_TOKEN, trailer_values};
use ix_trace_rs::trace;

const DIGEST: &str = "sha256:0123abcd";

#[test]
#[trace("TC-338", "FR-042-AC-3", "FR-042-AC-12")]
fn tc_338_the_message_is_a_subject_then_the_three_trailers_read_back_exactly() {
    let message = compose_message("change-record-17", DIGEST, "reviewed for release");
    let paragraphs: Vec<&str> = message.trim_end_matches('\n').split("\n\n").collect();
    assert_eq!(paragraphs.len(), 2, "{message:?}");
    assert!(!paragraphs[0].contains('\n'), "the subject is one line");
    assert!(message.ends_with('\n'));
    let trailers = paragraphs[1];
    assert_eq!(
        trailers.lines().collect::<Vec<_>>(),
        [
            "Approved-Record-Id: change-record-17",
            "Approved-Record-Digest: sha256:0123abcd",
            "Signed-off-meaning: reviewed for release",
        ]
    );
    assert_eq!(
        trailer_values(trailers, RECORD_ID_TRAILER),
        ["change-record-17"]
    );
    assert_eq!(trailer_values(trailers, RECORD_DIGEST_TRAILER), [DIGEST]);
    assert_eq!(
        trailer_values(trailers, MEANING_TOKEN),
        ["reviewed for release"]
    );
}

#[test]
#[trace("TC-339", "FR-042-AC-4", "FR-042-AC-12")]
fn tc_339_a_meaning_and_a_record_value_must_read_back_unchanged() {
    assert!(is_meaning("reviewed"));
    assert!(is_meaning("approved for release: v1"));
    assert!(is_meaning(&"m".repeat(MAX_MEANING_BYTES)));
    for refused in [
        "",
        " ",
        " reviewed",
        "reviewed ",
        "re\nviewed",
        "re\tviewed",
        "re\u{7f}viewed",
    ] {
        assert!(!is_meaning(refused), "{refused:?}");
    }
    assert!(!is_meaning(&"m".repeat(MAX_MEANING_BYTES + 1)));

    let hex = "0123456789abcdef".repeat(4);
    assert!(is_record_digest(&format!("sha256:{hex}")));
    for refused in [
        hex.clone(),
        format!("sha256:{}", &hex[1..]),
        format!("sha256:{hex}0"),
        format!("sha256:{}", hex.to_uppercase()),
        format!("sha1:{hex}"),
        format!(" sha256:{hex}"),
        format!("sha256:{hex}\n"),
        format!("sha256:{}g", &hex[1..]),
        String::new(),
    ] {
        assert!(!is_record_digest(&refused), "{refused:?}");
    }

    assert!(is_trailer_record_value("change-record-17"));
    assert!(is_trailer_record_value(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    ));
    for refused in ["", "-id", " id", "id ", "i\nd"] {
        assert!(!is_trailer_record_value(refused), "{refused:?}");
    }
}

#[test]
#[trace("TC-340", "FR-042-AC-6", "FR-042-AC-12")]
fn tc_340_only_ssh_with_a_key_and_x509_can_sign_a_verifiable_tag() {
    let formats = [
        None,
        Some(""),
        Some("openpgp"),
        Some("SSH"),
        Some("ssh"),
        Some("x509"),
    ];
    let values = [None, Some(""), Some("key")];
    let mut accepted = 0;
    for format in formats {
        for key in values {
            for command in values {
                for program in values {
                    let refusal = signing_refusal(format, key, command, program);
                    match format {
                        Some("x509") if program == Some("key") => assert_eq!(refusal, None),
                        Some("x509") => assert_eq!(refusal, Some(SigningSetting::X509Program)),
                        Some("ssh") if key == Some("key") || command == Some("key") => {
                            assert_eq!(refusal, None);
                        }
                        Some("ssh") => assert_eq!(refusal, Some(SigningSetting::UserSigningKey)),
                        _ => assert_eq!(refusal, Some(SigningSetting::GpgFormat)),
                    }
                    accepted += usize::from(refusal.is_none());
                }
            }
        }
    }
    // x509 with a program (9 key combinations), ssh with a key (5, times 3 programs).
    assert_eq!(accepted, 9 + 5 * 3);
    assert_eq!(SigningSetting::X509Program.key(), "gpg.x509.program");
    assert_eq!(SigningSetting::GpgFormat.key(), "gpg.format");
    assert_eq!(SigningSetting::UserSigningKey.key(), "user.signingkey");
}

#[test]
#[trace("TC-340", "FR-042-AC-11")]
fn tc_340_only_a_tag_with_a_supported_signature_block_counts_as_signed() {
    let head = "object 0123456789abcdef0123456789abcdef01234567\ntype commit\ntag t\ntagger a <a@example.test> 1 +0000\n\nmsg\n";
    assert!(!tag_is_signed(head.as_bytes()));
    for block in [
        "-----BEGIN SSH SIGNATURE-----\nAAAA\n-----END SSH SIGNATURE-----\n",
        "-----BEGIN SIGNED MESSAGE-----\nAAAA\n-----END SIGNED MESSAGE-----\n",
    ] {
        assert!(tag_is_signed(format!("{head}{block}").as_bytes()));
    }
    let pgp = "-----BEGIN PGP SIGNATURE-----\nAAAA\n-----END PGP SIGNATURE-----\n";
    assert!(!tag_is_signed(format!("{head}{pgp}").as_bytes()));
}
