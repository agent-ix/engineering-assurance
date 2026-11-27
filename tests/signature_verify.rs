// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Contract tests for the pure signature-verification classifier (FR-036).

use engineering_assurance::signature_verify::{
    CommitObservation, GitGrade, GitsignIdentity, GitsignTrust, IdentityRegistry,
    MAX_REGISTRY_ENTRIES, MAX_REVISION_BYTES, Mapping, Mismatch, Presence, PrincipalLookup, Probe,
    Reason, RegistryEntry, Run, SignatureFormat, SignatureKind, SignerSource, SshCheck, Status,
    TagObservation, TargetKind, TrustSourceAddress, classify_commit, classify_gitsign,
    classify_tag, commit_signature_format, config_sets_gitsign_key, is_plain_revision,
    object_message, signed_off_meaning, split_tag_object,
};
use ix_trace_rs::trace;

fn commit(grade: char, verify_accepted: bool, fingerprint: Option<&str>) -> CommitObservation {
    CommitObservation {
        format: Some(SignatureFormat::Ssh),
        git: Some(GitGrade {
            verify_accepted,
            grade,
            fingerprint: fingerprint.map(str::to_owned),
            principal: Some("p".to_owned()),
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
                principal: None,
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
        principal: if verify_accepted {
            PrincipalLookup::Found("p".to_owned())
        } else {
            PrincipalLookup::NotRun
        },
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
#[trace("TC-249", "FR-036-AC-9", "FR-036-CON-1")]
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
        SignatureFormat::Cms
    );
    assert_eq!(split_tag_object(b"tag t\n\nplain message\n"), None);
    // Git matches a header by prefix, so a longer line still starts the block.
    let prefixed =
        split_tag_object(b"tag t\n\nm\n-----BEGIN PGP SIGNATURE----- \nAA\n").expect("block");
    assert_eq!(prefixed.format, SignatureFormat::Other);
    assert_eq!(prefixed.payload, b"tag t\n\nm\n");
    // Message text that looks like a header does not start the signature.
    let quoted = b"tag t\n\n-----BEGIN PGP SIGNATURE-----\nq\n-----BEGIN SSH SIGNATURE-----\nAA\n";
    let split = split_tag_object(quoted).expect("block");
    assert_eq!(split.format, SignatureFormat::Ssh);
    assert!(split.payload.ends_with(b"q\n"));
}

#[test]
#[trace("TC-248", "FR-036-AC-9", "FR-036-CON-1")]
fn tc_248_commit_signature_header_names_its_format() {
    let ssh = b"tree 1\nauthor a\ngpgsig -----BEGIN SSH SIGNATURE-----\n AAAA\n -----END SSH SIGNATURE-----\n\nbody\n";
    assert_eq!(commit_signature_format(ssh), Some(SignatureFormat::Ssh));
    let pgp = b"tree 1\ngpgsig -----BEGIN PGP SIGNATURE-----\n AA\n\nbody\n";
    assert_eq!(commit_signature_format(pgp), Some(SignatureFormat::Other));
    let x509 = b"tree 1\ngpgsig -----BEGIN SIGNED MESSAGE-----\n AA\n\nbody\n";
    assert_eq!(commit_signature_format(x509), Some(SignatureFormat::Cms));
    let sha256 = b"tree 1\ngpgsig-sha256 -----BEGIN SSH SIGNATURE-----\n AA\n\nbody\n";
    assert_eq!(commit_signature_format(sha256), Some(SignatureFormat::Ssh));
    assert_eq!(commit_signature_format(b"tree 1\nauthor a\n\nbody\n"), None);
    // Any non-SSH signature header makes the commit unsupported, in either order.
    let mixed = b"tree 1\ngpgsig-sha256 -----BEGIN SSH SIGNATURE-----\n AA\ngpgsig -----BEGIN PGP SIGNATURE-----\n AA\n\nbody\n";
    assert_eq!(commit_signature_format(mixed), Some(SignatureFormat::Other));
    let reversed = b"tree 1\ngpgsig -----BEGIN PGP SIGNATURE-----\n AA\ngpgsig-sha256 -----BEGIN SSH SIGNATURE-----\n AA\n\nbody\n";
    assert_eq!(
        commit_signature_format(reversed),
        Some(SignatureFormat::Other)
    );
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

#[test]
#[trace("TC-273", "FR-037-AC-1", "FR-037-AC-2", "FR-037-CON-1")]
fn tc_273_only_an_accepted_key_with_a_principal_is_mapped() {
    let mapped = classify_commit(&commit('G', true, Some("SHA256:key")));
    let signer = mapped.signer.expect("signer");
    assert_eq!(signer.mapping, Mapping::Mapped);
    assert_eq!(signer.principal.as_deref(), Some("p"));

    let untrusted = classify_commit(&commit('U', false, Some("SHA256:key")));
    assert_eq!(untrusted.signer.expect("signer").mapping, Mapping::Unmapped);
    // A good grade without a principal is not mapped as anything.
    let nameless = classify_commit(&CommitObservation {
        format: Some(SignatureFormat::Ssh),
        git: Some(GitGrade {
            verify_accepted: true,
            grade: 'G',
            fingerprint: None,
            principal: None,
        }),
    });
    assert_eq!(nameless.status, Status::Unverifiable);
    assert_eq!(nameless.signer, None);
    for (grade, accepted) in [('B', false), ('N', false), ('E', false)] {
        assert_eq!(classify_commit(&commit(grade, accepted, None)).signer, None);
    }

    let ssh = SignatureFormat::Ssh;
    let tag = classify_tag(&signed(true, None, ssh));
    assert_eq!(tag.signer.expect("signer").mapping, Mapping::Mapped);
    let untrusted_tag = classify_tag(&signed(false, Some(SshCheck::Accepted), ssh));
    assert_eq!(
        untrusted_tag.signer.expect("signer").mapping,
        Mapping::Unmapped
    );
    let no_lookup = classify_tag(&TagObservation::Signature {
        format: ssh,
        verify_accepted: true,
        check: None,
        principal: PrincipalLookup::Rejected,
    });
    assert_eq!(no_lookup.status, Status::Unverifiable);
    assert_eq!(no_lookup.signer, None);
}

#[test]
#[trace("TC-274", "FR-037-AC-5", "FR-037-AC-6", "FR-037-CON-2")]
fn tc_274_trailer_output_yields_present_values_in_order_or_missing() {
    let one = signed_off_meaning("Signed-off-meaning: reviewed\n");
    assert_eq!(one.presence, Presence::Present);
    assert_eq!(one.values, ["reviewed"]);
    let two = signed_off_meaning(
        "Signed-off-by: A\nsigned-off-meaning: first\nSigned-off-meaning:   second  \n",
    );
    assert_eq!(two.values, ["first", "second"]);
    let empty = signed_off_meaning("Signed-off-meaning:\n");
    assert_eq!(empty.presence, Presence::Present);
    assert_eq!(empty.values, [""]);
    for text in ["", "Signed-off-by: A\n", "Signed-off-meaning-x: v\n"] {
        let missing = signed_off_meaning(text);
        assert_eq!(missing.presence, Presence::Missing, "{text:?}");
        assert_eq!(missing.values, [] as [std::string::String; 0]);
    }
}

#[test]
#[trace("TC-275", "FR-037-AC-8")]
fn tc_275_a_message_is_everything_after_the_header_block() {
    assert_eq!(
        object_message(b"tree x\nauthor a\n\nsubject\n\nbody\n"),
        b"subject\n\nbody\n"
    );
    assert_eq!(
        object_message(b"tree x\ngpgsig -----BEGIN\n  sig\n \n -----END\n\nmsg\n"),
        b"msg\n"
    );
    assert_eq!(object_message(b"tree x\n"), b"");
}

const ISSUER: &str = concat!("https:", "//issuer.example.test");
const OTHER_ISSUER: &str = concat!("https:", "//other.example.test");

fn identity() -> GitsignIdentity {
    GitsignIdentity::new("alice@example.test".to_owned(), ISSUER.to_owned())
        .expect("valid identity")
}

fn entry(identity: &str, issuer: &str, principal: &str) -> RegistryEntry {
    RegistryEntry::new(identity.to_owned(), issuer.to_owned(), principal.to_owned())
        .expect("valid entry")
}

/// A trust whose registry names Alice for the fixture identity and issuer.
fn trust() -> GitsignTrust {
    let registry = IdentityRegistry::new(vec![
        entry("bob@example.test", ISSUER, "Bob Example"),
        entry("alice@example.test", ISSUER, "Alice Example"),
    ])
    .expect("valid registry");
    GitsignTrust::new(identity(), registry)
}

/// Runs the classifier against a script of outcomes by probe; returns the
/// verdict and the probes asked for, in order. The classifier takes no message
/// text, so replacing the text of a captured outcome cannot change a verdict.
fn gitsign(
    trust: Option<&GitsignTrust>,
    repository_sets_key: bool,
    outcomes: &[(Probe, Run)],
) -> (engineering_assurance::signature_verify::Verdict, Vec<Probe>) {
    let mut asked = Vec::new();
    let verdict = classify_gitsign(TargetKind::Commit, trust, repository_sets_key, |probe| {
        asked.push(probe);
        outcomes
            .iter()
            .find(|(p, _)| *p == probe)
            .map_or(Run::Exit(99), |(_, run)| *run)
    });
    (verdict, asked)
}

#[test]
#[trace("TC-293", "FR-040-AC-10", "FR-040-CON-1")]
fn tc_293_gitsign_outcomes_map_to_statuses_mismatches_and_reasons() {
    let id = trust();
    let (valid, asked) = gitsign(Some(&id), false, &[(Probe::Exact, Run::Exit(0))]);
    assert_eq!(valid.status, Status::SignedValid);
    assert_eq!(valid.signature_kind, Some(SignatureKind::Gitsign));
    assert_eq!(asked, [Probe::Exact]);
    let signer = valid.signer.expect("signer");
    assert_eq!(signer.source, SignerSource::GitsignRegistry);
    assert_eq!(signer.mapping, Mapping::Mapped);
    assert_eq!(signer.principal.as_deref(), Some("Alice Example"));
    assert_eq!(
        valid.certificate.expect("certificate").identity,
        "alice@example.test"
    );

    let wild = (Probe::AnyIdentityAnyIssuer, Run::Exit(0));
    let cases = [
        (Run::Exit(0), Run::Exit(1), Mismatch::Issuer, 3),
        (Run::Exit(1), Run::Exit(0), Mismatch::Identity, 4),
        (Run::Exit(1), Run::Exit(1), Mismatch::IdentityAndIssuer, 4),
    ];
    for (issuer, identity_probe, mismatch, count) in cases {
        let (verdict, asked) = gitsign(
            Some(&id),
            false,
            &[
                (Probe::Exact, Run::Exit(1)),
                wild,
                (Probe::ExactIdentityAnyIssuer, issuer),
                (Probe::AnyIdentityExactIssuer, identity_probe),
            ],
        );
        assert_eq!(verdict.status, Status::SignedUntrusted);
        assert_eq!(verdict.mismatch, Some(mismatch));
        assert_eq!(verdict.certificate, None);
        assert_eq!(verdict.signer.expect("signer").mapping, Mapping::Unmapped);
        assert_eq!(asked.len(), count);
    }

    let unverifiable = |outcomes: &[(Probe, Run)], reason| {
        let (verdict, _) = gitsign(Some(&id), false, outcomes);
        assert_eq!(verdict.status, Status::Unverifiable, "{outcomes:?}");
        assert_eq!(verdict.reason, Some(reason), "{outcomes:?}");
        assert_eq!(verdict.signer, None);
    };
    let failed = Reason::GitsignVerificationFailed;
    unverifiable(
        &[
            (Probe::Exact, Run::Exit(1)),
            (Probe::AnyIdentityAnyIssuer, Run::Exit(1)),
        ],
        failed,
    );
    unverifiable(&[(Probe::Exact, Run::Exit(2))], failed);
    unverifiable(&[(Probe::Exact, Run::Signaled)], failed);
    unverifiable(
        &[(Probe::Exact, Run::SnapshotChanged)],
        Reason::SnapshotRefChanged,
    );
    unverifiable(
        &[
            (Probe::Exact, Run::Exit(1)),
            (Probe::AnyIdentityAnyIssuer, Run::SnapshotChanged),
        ],
        Reason::SnapshotRefChanged,
    );
    unverifiable(
        &[
            (Probe::Exact, Run::Exit(1)),
            (Probe::AnyIdentityAnyIssuer, Run::Exit(2)),
        ],
        failed,
    );
    unverifiable(
        &[(Probe::Exact, Run::NotStarted)],
        Reason::GitsignNotStarted,
    );
    unverifiable(&[(Probe::Exact, Run::TimedOut)], Reason::Timeout);
    unverifiable(&[(Probe::Exact, Run::CutShort)], Reason::OutputExceeded);
    unverifiable(
        &[(Probe::Exact, Run::Unobserved)],
        Reason::UnrecognizedOutput,
    );
    unverifiable(
        &[
            (Probe::Exact, Run::Exit(1)),
            wild,
            (Probe::ExactIdentityAnyIssuer, Run::Exit(1)),
            (Probe::AnyIdentityExactIssuer, Run::TimedOut),
        ],
        Reason::Timeout,
    );

    // No identity or a repository `gitsign.*` key: nothing is run, never valid.
    let (missing, asked) = gitsign(None, false, &[(Probe::Exact, Run::Exit(0))]);
    assert_eq!(missing.reason, Some(Reason::GitsignIdentityMissing));
    assert_eq!(
        asked,
        [] as [engineering_assurance::signature_verify::Probe; 0]
    );
    let (configured, asked) = gitsign(Some(&id), true, &[(Probe::Exact, Run::Exit(0))]);
    assert_eq!(configured.status, Status::Unverifiable);
    assert_eq!(configured.reason, Some(Reason::GitsignRepositoryConfig));
    assert_eq!(
        asked,
        [] as [engineering_assurance::signature_verify::Probe; 0]
    );
}

#[test]
#[trace("TC-1041", "FR-040-AC-25", "FR-040-CON-3")]
fn tc_1041_an_accepted_identity_no_registry_entry_names_is_valid_and_unmapped() {
    let stranger = GitsignTrust::new(
        identity(),
        IdentityRegistry::new(vec![entry("bob@example.test", ISSUER, "Bob Example")])
            .expect("valid registry"),
    );
    let (unmapped, _) = gitsign(Some(&stranger), false, &[(Probe::Exact, Run::Exit(0))]);
    assert_eq!(unmapped.status, Status::SignedValid);
    let signer = unmapped.signer.expect("signer");
    assert_eq!(signer.source, SignerSource::GitsignRegistry);
    assert_eq!(signer.mapping, Mapping::Unmapped);
    assert_eq!(signer.principal, None);
}

#[test]
#[trace("TC-294", "FR-040-AC-9")]
fn tc_294_cms_headers_are_detected_and_mixed_formats_are_unsupported() {
    let cms = "-----BEGIN SIGNED MESSAGE-----";
    let ssh = "-----BEGIN SSH SIGNATURE-----";
    let object = |headers: &[(&str, &str)]| {
        let mut text = String::from("tree t\nauthor a\n");
        for (name, value) in headers {
            text.extend([name, " ", value, "\n AAAA\n"]);
        }
        text.push_str("\nmessage\n");
        text.into_bytes()
    };
    let format = |headers: &[(&str, &str)]| commit_signature_format(&object(headers));
    assert_eq!(format(&[("gpgsig", cms)]), Some(SignatureFormat::Cms));
    assert_eq!(
        format(&[("gpgsig", "-----BEGIN SIGNED MESSAGE----- trailing")]),
        Some(SignatureFormat::Cms)
    );
    assert_eq!(
        format(&[("gpgsig", cms), ("gpgsig-sha256", cms)]),
        Some(SignatureFormat::Cms)
    );
    assert_eq!(
        format(&[("gpgsig", cms), ("gpgsig-sha256", ssh)]),
        Some(SignatureFormat::Other)
    );
    assert_eq!(
        format(&[("gpgsig", ssh), ("gpgsig-sha256", cms)]),
        Some(SignatureFormat::Other)
    );
    assert_eq!(format(&[("gpgsig", ssh)]), Some(SignatureFormat::Ssh));
    let tag = format!("object o\n\nmsg\n{cms}\nAAAA\n");
    let split = split_tag_object(tag.as_bytes()).expect("signature");
    assert_eq!(split.format, SignatureFormat::Cms);
}

#[test]
#[trace("TC-296", "FR-040-AC-11")]
fn tc_296_gitsign_identity_values_are_bounded() {
    let make =
        |identity: &str, issuer: &str| GitsignIdentity::new(identity.to_owned(), issuer.to_owned());
    assert!(make("a", "issuer").is_some());
    assert!(make(&"x".repeat(256), "i").is_some());
    for (identity, issuer) in [
        ("", "i"),
        ("a", ""),
        ("-a", "i"),
        ("a", "-i"),
        ("a\n", "i"),
        ("a", "i\u{7}"),
    ] {
        assert!(make(identity, issuer).is_none(), "{identity:?}");
    }
    assert!(make(&"x".repeat(257), "i").is_none());
    assert!(make("a", &"x".repeat(257)).is_none());
}

#[test]
#[trace("TC-1041", "FR-040-AC-25", "FR-040-CON-3")]
fn tc_1041_the_registry_maps_only_by_exact_identity_and_issuer() {
    let registry = IdentityRegistry::new(vec![
        entry("bob@example.test", ISSUER, "Bob Example"),
        entry("alice@example.test", ISSUER, "Alice Example"),
    ])
    .expect("valid registry");
    assert_eq!(
        registry.principal("alice@example.test", ISSUER),
        Some("Alice Example")
    );
    for (identity, issuer) in [
        ("ALICE@example.test", ISSUER),
        ("alice@example", ISSUER),
        ("alice@example.test.evil", ISSUER),
        ("alice@.*", ISSUER),
        (".*", ".*"),
        ("alice@example.test", OTHER_ISSUER),
        ("alice@example.test", ""),
        ("bob@example.test", OTHER_ISSUER),
    ] {
        assert_eq!(registry.principal(identity, issuer), None, "{identity}");
    }
}

#[test]
#[trace("TC-1043", "FR-040-AC-27")]
fn tc_1043_registry_entries_and_size_are_bounded_and_duplicates_refused() {
    let make = |identity: &str, issuer: &str, principal: &str| {
        RegistryEntry::new(identity.to_owned(), issuer.to_owned(), principal.to_owned())
    };
    assert!(make("a", "i", "p").is_some());
    assert!(make(&"x".repeat(256), "i", &"p".repeat(256)).is_some());
    // A principal may begin with `-`: it is never an argument.
    assert!(make("a", "i", "-p").is_some());
    for (identity, issuer, principal) in [
        ("", "i", "p"),
        ("a", "", "p"),
        ("a", "i", ""),
        ("-a", "i", "p"),
        ("a", "-i", "p"),
        ("a\n", "i", "p"),
        ("a", "i\u{7}", "p"),
        ("a", "i", "p\u{7}"),
    ] {
        assert!(make(identity, issuer, principal).is_none(), "{identity:?}");
    }
    assert!(make("a", "i", &"p".repeat(257)).is_none());

    let numbered = |count: usize| -> Vec<RegistryEntry> {
        (0..count)
            .map(|n| entry(&format!("user{n}@example.test"), ISSUER, "P"))
            .collect()
    };
    assert!(IdentityRegistry::new(numbered(MAX_REGISTRY_ENTRIES)).is_some());
    assert!(IdentityRegistry::new(numbered(MAX_REGISTRY_ENTRIES + 1)).is_none());
    // The same identity and issuer twice is invalid, whatever the principals.
    for second in ["Alice", "Someone Else"] {
        let duplicate = vec![
            entry("a", "i", "Alice"),
            entry("b", "i", "Bob"),
            entry("a", "i", second),
        ];
        assert!(IdentityRegistry::new(duplicate).is_none(), "{second}");
    }
    // The same identity under two issuers is two entries.
    assert!(IdentityRegistry::new(vec![entry("a", "i", "P"), entry("a", "j", "P")]).is_some());
}

#[test]
#[trace("TC-1038", "FR-037-AC-20")]
fn tc_1038_a_trust_source_address_splits_at_the_first_colon_and_is_bounded() {
    let parsed = TrustSourceAddress::parse("main:spec/registry.json").expect("well formed");
    assert_eq!(
        (parsed.revision(), parsed.path()),
        ("main", "spec/registry.json")
    );
    // Only the first colon splits: the rest is the path.
    let parsed = TrustSourceAddress::parse("HEAD@{12:00}").expect("well formed");
    assert_eq!((parsed.revision(), parsed.path()), ("HEAD@{12", "00}"));
    let parsed = TrustSourceAddress::parse("a:b:c").expect("well formed");
    assert_eq!((parsed.revision(), parsed.path()), ("a", "b:c"));
    for address in [
        "",
        "main",
        ":path",
        "main:",
        "-main:path",
        "main:/path",
        "main:a/../b",
        "main:../b",
        "main:./b",
        "main:a//b",
        "main:a/",
        "ma\u{7}in:path",
        "main:pa\nth",
    ] {
        assert!(TrustSourceAddress::parse(address).is_none(), "{address:?}");
    }
    assert!(TrustSourceAddress::parse(&format!("{}:p", "r".repeat(256))).is_some());
    assert!(TrustSourceAddress::parse(&format!("{}:p", "r".repeat(257))).is_none());
    assert!(TrustSourceAddress::parse(&format!("r:{}", "p".repeat(1024))).is_some());
    assert!(TrustSourceAddress::parse(&format!("r:{}", "p".repeat(1025))).is_none());
}

#[test]
#[trace("TC-298", "FR-040-AC-17")]
fn tc_298_config_output_is_read_line_by_line_for_a_gitsign_key() {
    assert!(config_sets_gitsign_key(
        b"core.bare false\ngitsign.fulcioroot /x\n"
    ));
    assert!(config_sets_gitsign_key(b"GitSign.Rekor y\n"));
    assert!(config_sets_gitsign_key(b"gitsign.connectorid"));
    assert!(!config_sets_gitsign_key(b""));
    assert!(!config_sets_gitsign_key(
        b"core.bare false\nfoo.bar gitsign.x\n"
    ));
    assert!(!config_sets_gitsign_key(b"gitsignfoo.x 1\n gitsign.y\n"));
    // Unicode folds that gitsign's comparison would accept: long s, Kelvin sign.
    assert!(config_sets_gitsign_key(
        "git\u{17f}ign.fulcioRoot x".as_bytes()
    ));
    assert!(config_sets_gitsign_key(
        "foo.bar x\n\u{212a}.x y\n".as_bytes()
    ));
    assert!(config_sets_gitsign_key("gitsign\u{212a}.x y".as_bytes()));
    // A non-ASCII subsection or value is not a key section.
    assert!(!config_sets_gitsign_key(
        "branch.\u{e9}.remote origin\n".as_bytes()
    ));
    assert!(!config_sets_gitsign_key(
        "core.editor caf\u{e9}\n".as_bytes()
    ));
}
