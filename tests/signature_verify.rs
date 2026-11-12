// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Contract tests for the pure signature-verification classifier (FR-036).

use engineering_assurance::signature_verify::{
    CommitObservation, GitGrade, MAX_REVISION_BYTES, Reason, SignatureFormat, SshCheck, Status,
    TagObservation, TargetKind, classify_commit, classify_tag, commit_signature_format,
    is_plain_revision, split_tag_object,
};
use ix_trace_rs::trace;

fn commit(grade: char, verify_accepted: bool, fingerprint: Option<&str>) -> CommitObservation {
    CommitObservation {
        format: Some(SignatureFormat::Ssh),
        git: Some(GitGrade {
            verify_accepted,
            grade,
            fingerprint: fingerprint.map(str::to_owned),
        }),
    }
}

#[test]
#[trace("TC-248", "FR-036-AC-9", "FR-036-CON-1")]
fn tc_248_commit_grades_map_to_statuses_and_reasons() {
    let valid = classify_commit(&commit('G', true, Some("SHA256:key")));
    assert_eq!(valid.status, Status::SignedValid);
    assert_eq!(valid.kind, TargetKind::Commit);
    assert_eq!(valid.key_fingerprint.as_deref(), Some("SHA256:key"));
    assert_eq!(valid.reason, None);

    let untrusted = classify_commit(&commit('U', false, Some("SHA256:other")));
    assert_eq!(untrusted.status, Status::SignedUntrusted);
    assert_eq!(untrusted.key_fingerprint.as_deref(), Some("SHA256:other"));
    let bad = classify_commit(&commit('B', false, None));
    assert_eq!(bad.status, Status::SignedBad);
    assert_eq!(bad.key_fingerprint, None);
    let unsigned = classify_commit(&commit('N', false, None));
    assert_eq!(unsigned.status, Status::Unsigned);
    assert_eq!(unsigned.reason, None);

    let unchecked = classify_commit(&commit('E', false, None));
    assert_eq!(unchecked.status, Status::Unverifiable);
    assert_eq!(unchecked.reason, Some(Reason::SignatureUnchecked));
    for code in ['X', 'Y', 'R', 'Z', '?'] {
        let verdict = classify_commit(&commit(code, true, Some("SHA256:key")));
        assert_eq!(verdict.status, Status::Unverifiable, "grade {code}");
        assert_eq!(verdict.reason, Some(Reason::UnrecognizedOutput));
        assert_eq!(verdict.key_fingerprint, None);
    }

    // A good grade git's own verification did not accept is never valid.
    let disagreement = classify_commit(&commit('G', false, Some("SHA256:key")));
    assert_eq!(disagreement.status, Status::Unverifiable);
    assert_eq!(disagreement.reason, Some(Reason::UnrecognizedOutput));
    // Without a grade there is nothing to classify.
    let ungraded = classify_commit(&CommitObservation {
        format: Some(SignatureFormat::Ssh),
        git: None,
    });
    assert_eq!(ungraded.reason, Some(Reason::UnrecognizedOutput));

    // A non-SSH header is unsupported, whatever grade accompanies it.
    for grade in ['G', 'B', 'N'] {
        let other = classify_commit(&CommitObservation {
            format: Some(SignatureFormat::Other),
            git: Some(GitGrade {
                verify_accepted: true,
                grade,
                fingerprint: None,
            }),
        });
        assert_eq!(other.status, Status::Unverifiable);
        assert_eq!(other.reason, Some(Reason::UnsupportedSignatureFormat));
    }
}

fn signed(
    verify_accepted: bool,
    check: Option<SshCheck>,
    format: SignatureFormat,
) -> TagObservation {
    TagObservation::Signature {
        format,
        verify_accepted,
        check,
    }
}

#[test]
#[trace("TC-249", "FR-036-AC-9", "FR-036-CON-1")]
fn tc_249_tag_outcomes_map_in_grading_order() {
    let ssh = SignatureFormat::Ssh;
    let valid = classify_tag(&signed(true, None, ssh));
    assert_eq!(valid.status, Status::SignedValid);
    assert_eq!(valid.kind, TargetKind::Tag);
    assert_eq!(valid.key_fingerprint, None);

    let untrusted = classify_tag(&signed(false, Some(SshCheck::Accepted), ssh));
    assert_eq!(untrusted.status, Status::SignedUntrusted);
    let bad = classify_tag(&signed(false, Some(SshCheck::Rejected), ssh));
    assert_eq!(bad.status, Status::SignedBad);
    let missing = classify_tag(&signed(false, Some(SshCheck::NotStarted), ssh));
    assert_eq!(missing.status, Status::Unverifiable);
    assert_eq!(missing.reason, Some(Reason::SshKeygenNotStarted));
    let unchecked = classify_tag(&signed(false, None, ssh));
    assert_eq!(unchecked.reason, Some(Reason::UnrecognizedOutput));

    let unsigned = classify_tag(&TagObservation::NoSignature);
    assert_eq!(unsigned.status, Status::Unsigned);
    assert_eq!(unsigned.reason, None);
    let other = classify_tag(&signed(
        false,
        Some(SshCheck::Rejected),
        SignatureFormat::Other,
    ));
    assert_eq!(other.status, Status::Unverifiable);
    assert_eq!(other.reason, Some(Reason::UnsupportedSignatureFormat));
    let lightweight = classify_tag(&TagObservation::Lightweight);
    assert_eq!(lightweight.status, Status::Unsigned);
    assert_eq!(lightweight.reason, Some(Reason::LightweightTag));
}

#[test]
#[trace("TC-249", "FR-036-AC-9")]
fn tc_249_a_tag_object_splits_at_the_signature_header_line() {
    let object = b"object 1\ntype commit\ntag t\n\nmessage\n-----BEGIN SSH SIGNATURE-----\nAAAA\n-----END SSH SIGNATURE-----\n";
    let split = split_tag_object(object).expect("a signature block");
    assert_eq!(split.format, SignatureFormat::Ssh);
    assert_eq!(split.payload, b"object 1\ntype commit\ntag t\n\nmessage\n");
    assert_eq!(
        split.signature,
        b"-----BEGIN SSH SIGNATURE-----\nAAAA\n-----END SSH SIGNATURE-----\n"
    );
    assert_eq!([split.payload, split.signature].concat(), object);

    let pgp = b"tag t\n\nm\n-----BEGIN PGP SIGNATURE-----\nAA\n-----END PGP SIGNATURE-----\n";
    assert_eq!(
        split_tag_object(pgp).expect("block").format,
        SignatureFormat::Other
    );
    let x509 = b"tag t\n\nm\n-----BEGIN SIGNED MESSAGE-----\nAA\n";
    assert_eq!(
        split_tag_object(x509).expect("block").format,
        SignatureFormat::Other
    );
    assert_eq!(split_tag_object(b"tag t\n\nplain message\n"), None);
    // A header that is only the prefix of a longer line is message text.
    assert_eq!(
        split_tag_object(b"tag t\n\n-----BEGIN SSH SIGNATURE----- not a header\n"),
        None
    );
    // Message text that looks like a header does not start the signature.
    let quoted = b"tag t\n\n-----BEGIN PGP SIGNATURE-----\nq\n-----BEGIN SSH SIGNATURE-----\nAA\n";
    let split = split_tag_object(quoted).expect("block");
    assert_eq!(split.format, SignatureFormat::Ssh);
    assert!(split.payload.ends_with(b"q\n"));
}

#[test]
#[trace("TC-248", "FR-036-AC-9")]
fn tc_248_commit_signature_header_names_its_format() {
    let ssh = b"tree 1\nauthor a\ngpgsig -----BEGIN SSH SIGNATURE-----\n AAAA\n -----END SSH SIGNATURE-----\n\nbody\n";
    assert_eq!(commit_signature_format(ssh), Some(SignatureFormat::Ssh));
    let pgp = b"tree 1\ngpgsig -----BEGIN PGP SIGNATURE-----\n AA\n\nbody\n";
    assert_eq!(commit_signature_format(pgp), Some(SignatureFormat::Other));
    let x509 = b"tree 1\ngpgsig -----BEGIN SIGNED MESSAGE-----\n AA\n\nbody\n";
    assert_eq!(commit_signature_format(x509), Some(SignatureFormat::Other));
    let sha256 = b"tree 1\ngpgsig-sha256 -----BEGIN SSH SIGNATURE-----\n AA\n\nbody\n";
    assert_eq!(commit_signature_format(sha256), Some(SignatureFormat::Ssh));
    assert_eq!(commit_signature_format(b"tree 1\nauthor a\n\nbody\n"), None);
    // A header-looking line in the message is not a header.
    let message = b"tree 1\n\ngpgsig -----BEGIN SSH SIGNATURE-----\n";
    assert_eq!(commit_signature_format(message), None);
}

#[test]
#[trace("TC-256", "FR-036-AC-7")]
fn tc_256_plain_revisions_are_the_only_accepted_ones() {
    assert!(is_plain_revision("HEAD~1"));
    assert!(is_plain_revision(&"a".repeat(MAX_REVISION_BYTES)));
    assert!(!is_plain_revision(&"a".repeat(MAX_REVISION_BYTES + 1)));
    for bad in [
        "",
        "-x",
        "--output=x",
        "a b",
        "a\tb",
        "a\nb",
        "a\u{7}b",
        "a\u{a0}b",
    ] {
        assert!(!is_plain_revision(bad), "{bad:?}");
    }
}
