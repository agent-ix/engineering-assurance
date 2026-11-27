// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Contract tests for the pure tag receipt module: the statement, its
//! verification, the note edit and the one comparison (FR-081, FR-082).

use std::collections::{BTreeMap, BTreeSet};

use engineering_assurance::attestation_signing::{
    AuthenticatedWorkflowClaims, BundleVerifier, SourceWorkflowPolicy, TransparencyRequirement,
    VerificationFailure, WorkflowIdentityPolicy,
};
use engineering_assurance::tag_receipt::{
    Candidate, FindingStatus, InvalidReason, InvalidReceipt, Judged, MissingTagReason, NoteUpdate,
    RecordTrailers, SignerSet, TagFinding, TagReceipt, TagReceiptError, TagReceiptPolicy,
    TagReceiptStatement, add_entry, compare, is_receipted, note_lines, verify_line,
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

const REPOSITORY: &str = "fictional/project";
const SOURCE_WORKFLOW: &str = ".github/workflows/tag-receipt-push.yml";

fn id(character: char) -> String {
    character.to_string().repeat(40)
}

fn bundle_of(payload: &[u8]) -> Value {
    let envelope = sigstore_types::DsseEnvelope::new(
        "application/vnd.in-toto+json",
        sigstore_types::PayloadBytes::from(payload.to_vec()),
        sigstore_types::DsseSignature::new(
            sigstore_types::SignatureBytes::from(vec![0]),
            sigstore_types::KeyId::default(),
        ),
    );
    let bundle = sigstore_types::Bundle::new(
        sigstore_types::MediaType::Bundle0_3,
        sigstore_types::VerificationMaterial::new(
            sigstore_types::VerificationMaterialContent::PublicKey(
                sigstore_types::PublicKeyIdentifier::new("fictional"),
            ),
        ),
        sigstore_types::SignatureContent::DsseEnvelope(envelope),
    );
    serde_json::to_value(bundle).expect("bundle encodes")
}

fn statement(reference: &str, tag_object: &str, commit: &str) -> TagReceiptStatement {
    TagReceiptStatement::new(
        REPOSITORY,
        reference,
        tag_object,
        commit,
        "2026-11-20T10:00:00Z",
        RecordTrailers::from_values(vec!["rec-1".to_owned()]),
    )
}

fn signed(statement: &TagReceiptStatement) -> Value {
    bundle_of(&statement.canonical_bytes().expect("canonical"))
}

fn claims(reference: &str, trigger: &str, repository: &str) -> AuthenticatedWorkflowClaims {
    AuthenticatedWorkflowClaims {
        source_repository_uri: Some(format!("{}{}github.com/{repository}", "https:", "//")),
        source_repository_ref: Some(reference.to_owned()),
        build_config_uri: Some(format!(
            "{}{}github.com/{repository}/{SOURCE_WORKFLOW}@{reference}",
            "https:", "//"
        )),
        build_trigger: Some(trigger.to_owned()),
    }
}

/// A verifier double: the signature of the listed bundles is invalid, a flag
/// makes every signer untrusted, and the certificate claims are fixed.
struct Verifier {
    invalid: BTreeSet<String>,
    untrusted: bool,
    claims: AuthenticatedWorkflowClaims,
}

impl Verifier {
    fn trusting(claims: AuthenticatedWorkflowClaims) -> Self {
        Self {
            invalid: BTreeSet::new(),
            untrusted: false,
            claims,
        }
    }
}

impl BundleVerifier for Verifier {
    fn verify(
        &self,
        _payload: &[u8],
        bundle_json: &str,
        _policy: &WorkflowIdentityPolicy,
    ) -> Result<(), VerificationFailure> {
        if self.invalid.contains(bundle_json) {
            Err(VerificationFailure::InvalidSignature)
        } else if self.untrusted {
            Err(VerificationFailure::UntrustedSigner)
        } else {
            Ok(())
        }
    }

    fn verify_with_claims(
        &self,
        payload: &[u8],
        bundle_json: &str,
        policy: &WorkflowIdentityPolicy,
    ) -> Result<Option<AuthenticatedWorkflowClaims>, VerificationFailure> {
        self.verify(payload, bundle_json, policy)?;
        Ok(Some(self.claims.clone()))
    }
}

fn signers() -> Vec<(WorkflowIdentityPolicy, SourceWorkflowPolicy)> {
    vec![(
        WorkflowIdentityPolicy {
            repository: "fictional/engineering-assurance".to_owned(),
            workflow_path: ".github/workflows/tag-receipt.yml".to_owned(),
            references: BTreeSet::from(["refs/heads/main".to_owned()]),
            issuer: "issuer.example".to_owned(),
            transparency: TransparencyRequirement::PublicLog,
        },
        SourceWorkflowPolicy {
            repository: REPOSITORY.to_owned(),
            workflow_path: SOURCE_WORKFLOW.to_owned(),
            event: "push".to_owned(),
            source_ref: None,
            source_ref_prefix: Some("refs/tags/approve-".to_owned()),
        },
    )]
}

fn verify(
    bundle: &Value,
    commit: &str,
    verifier: &Verifier,
    list: &[(WorkflowIdentityPolicy, SourceWorkflowPolicy)],
) -> Candidate {
    verify_line(
        Some(bundle),
        commit,
        &[SignerSet {
            verifier,
            signers: list,
        }],
    )
}

fn reason_of(candidate: &Candidate) -> InvalidReason {
    match candidate {
        Candidate::Invalid(invalid) => invalid.reason,
        Candidate::Verified(receipt) => panic!("unexpectedly verified: {receipt:?}"),
    }
}

fn receipt(name: &str, object: char, commit: char, record_ids: &[&str]) -> TagReceipt {
    TagReceipt {
        reference: format!("refs/tags/{name}"),
        tag_object: id(object),
        commit: id(commit),
        pushed_at: "2026-11-20T10:00:00Z".to_owned(),
        trailers_readable: true,
        record_ids: record_ids.iter().map(|value| (*value).to_owned()).collect(),
    }
}

fn policy(accepted: &Value) -> TagReceiptPolicy {
    TagReceiptPolicy::parse(
        &json!({"prefixes": ["approve-", "release-"], "accepted_tags": accepted}),
    )
    .expect("valid policy")
}

fn tags(entries: &[(&str, char)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(name, object)| (format!("refs/tags/{name}"), id(*object)))
        .collect()
}

fn findings(judged: &[Judged]) -> Vec<(FindingStatus, bool)> {
    judged
        .iter()
        .map(|item| (item.finding.status(), item.accepted.is_some()))
        .collect()
}

#[test]
#[trace("TC-1102", "FR-081-AC-1")]
fn the_statement_has_one_git_tag_subject_and_exactly_the_closed_predicate_members() {
    let value = serde_json::to_value(statement("refs/tags/approve-1", &id('a'), &id('c')))
        .expect("encodes");
    assert_eq!(value["subject"].as_array().map(Vec::len), Some(1));
    assert_eq!(value["subject"][0]["name"], "refs/tags/approve-1");
    assert_eq!(value["subject"][0]["digest"], json!({"gitTag": id('a')}));
    assert_eq!(
        value["predicateType"],
        concat!("https:", "//schemas.agent-ix.ai/attestation/tag-receipt/v1")
    );
    let members: BTreeSet<&str> = value["predicate"]
        .as_object()
        .expect("predicate")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        members,
        BTreeSet::from([
            "repository",
            "ref",
            "tag_object",
            "commit",
            "pushed_at",
            "trailers_readable",
            "record_ids"
        ])
    );
    assert_eq!(value["predicate"]["record_ids"], json!(["rec-1"]));
}

#[test]
#[trace("TC-1103", "FR-081-AC-4")]
fn record_ids_keep_message_order_and_unreadable_trailers_carry_none() {
    let two = RecordTrailers::from_values(vec!["b".to_owned(), "a".to_owned()]);
    assert!(two.readable);
    assert_eq!(two.record_ids, ["b", "a"]);
    let none = RecordTrailers::from_values(Vec::new());
    assert!(none.readable && none.record_ids.is_empty());
    let too_many = RecordTrailers::from_values(vec!["x".to_owned(); 65]);
    assert!(!too_many.readable && too_many.record_ids.is_empty());
    assert!(RecordTrailers::from_values(vec!["x".to_owned(); 64]).readable);
    let long = RecordTrailers::from_values(vec!["x".repeat(257)]);
    assert!(!long.readable && long.record_ids.is_empty());
    assert!(RecordTrailers::from_values(vec!["x".repeat(256)]).readable);
    let unreadable = RecordTrailers::unreadable();
    assert!(!unreadable.readable && unreadable.record_ids.is_empty());
}

#[test]
#[trace("TC-1104", "FR-081-AC-2")]
fn the_tag_receipts_member_is_a_closed_object_with_bounds() {
    let accepted =
        json!({"ref": "refs/tags/a-1", "tag_object": id('a'), "accepted_by": "p", "reason": "r"});
    assert!(
        TagReceiptPolicy::parse(&json!({"prefixes": ["a-"], "accepted_tags": [accepted]})).is_ok()
    );
    let null_object =
        json!({"ref": "refs/tags/a-1", "tag_object": null, "accepted_by": "p", "reason": "r"});
    assert!(
        TagReceiptPolicy::parse(&json!({"prefixes": ["a-"], "accepted_tags": [null_object]}))
            .is_ok()
    );
    let invalid = [
        json!({}),
        json!({"prefixes": []}),
        json!({"prefixes": [""]}),
        json!({"prefixes": ["x".repeat(129)]}),
        json!({"prefixes": vec!["p"; 17]}),
        json!({"prefixes": ["a-"], "extra": 1}),
        json!({"prefixes": ["a-"], "accepted_tags": [{"ref": "refs/tags/a-1"}]}),
        json!({"prefixes": ["a-"], "accepted_tags": [{"ref": "refs/heads/a", "tag_object": null, "accepted_by": "p", "reason": "r"}]}),
        json!({"prefixes": ["a-"], "accepted_tags": [{"ref": "refs/tags/a", "tag_object": "short", "accepted_by": "p", "reason": "r"}]}),
        json!({"prefixes": ["a-"], "accepted_tags": [{"ref": "refs/tags/a", "tag_object": null, "accepted_by": "", "reason": "r"}]}),
        json!({"prefixes": ["a-"], "accepted_tags": [{"ref": "refs/tags/a", "tag_object": null, "accepted_by": "p", "reason": "r".repeat(1025)}]}),
        json!({"prefixes": ["a-"], "accepted_tags": [{"ref": "refs/tags/a", "tag_object": null, "accepted_by": "p", "reason": "r", "x": 1}]}),
        json!("not an object"),
    ];
    for member in invalid {
        assert!(TagReceiptPolicy::parse(&member).is_err(), "{member}");
    }
    let too_many = json!({"prefixes": ["a-"], "accepted_tags": vec![accepted; 1001]});
    assert!(TagReceiptPolicy::parse(&too_many).is_err());
    // A policy without the member is no tag receipts; a bad member is an error.
    let absent = br#"{"protocol":"engineering-assurance.receipt-policy/v1"}"#;
    assert_eq!(TagReceiptPolicy::from_receipt_policy(absent), Ok(None));
    let bad = br#"{"tag_receipts":{"prefixes":[]}}"#;
    assert!(TagReceiptPolicy::from_receipt_policy(bad).is_err());
}

#[test]
#[trace("TC-1105", "FR-081-AC-8")]
fn a_receipt_verifies_only_through_a_tag_identity_and_matching_certificate_claims() {
    let reference = "refs/tags/approve-1";
    let bundle = signed(&statement(reference, &id('a'), &id('c')));
    let list = signers();
    let trusting = Verifier::trusting(claims(reference, "push", REPOSITORY));
    match verify(&bundle, &id('c'), &trusting, &list) {
        Candidate::Verified(verified) => {
            assert_eq!(verified.reference, reference);
            assert_eq!(verified.tag_object, id('a'));
            assert_eq!(verified.record_ids, ["rec-1"]);
        }
        Candidate::Invalid(invalid) => panic!("not verified: {invalid:?}"),
    }
    // An entry with only `source_ref` is no tag identity: nothing is listed.
    assert_eq!(
        reason_of(&verify(&bundle, &id('c'), &trusting, &[])),
        InvalidReason::UntrustedSigner
    );
    // Certificate source ref differing from the statement ref.
    let moved = Verifier::trusting(claims("refs/tags/approve-2", "push", REPOSITORY));
    assert_eq!(
        reason_of(&verify(&bundle, &id('c'), &moved, &list)),
        InvalidReason::UntrustedSigner
    );
    // A pull request ref, another source repository and another trigger event.
    for other in [
        claims("refs/pull/1/merge", "push", REPOSITORY),
        claims(reference, "push", "fictional/other"),
        claims(reference, "workflow_dispatch", REPOSITORY),
        claims("refs/heads/main", "push", REPOSITORY),
    ] {
        assert_eq!(
            reason_of(&verify(
                &bundle,
                &id('c'),
                &Verifier::trusting(other),
                &list
            )),
            InvalidReason::UntrustedSigner
        );
    }
    // The delegated verifier rejecting the identity is untrusted too.
    let unlisted = Verifier {
        untrusted: true,
        ..Verifier::trusting(claims(reference, "push", REPOSITORY))
    };
    assert_eq!(
        reason_of(&verify(&bundle, &id('c'), &unlisted, &list)),
        InvalidReason::UntrustedSigner
    );
}

#[test]
#[trace("TC-1106", "FR-081-AC-9", "FR-082-AC-4")]
fn a_bad_signature_schema_or_subject_gives_its_own_reason() {
    let reference = "refs/tags/approve-1";
    let list = signers();
    let trusting = Verifier::trusting(claims(reference, "push", REPOSITORY));
    let good = statement(reference, &id('a'), &id('c'));

    let bundle = signed(&good);
    let mut changed = Verifier::trusting(claims(reference, "push", REPOSITORY));
    changed
        .invalid
        .insert(serde_json::to_string(&bundle).expect("json"));
    assert_eq!(
        reason_of(&verify(&bundle, &id('c'), &changed, &list)),
        InvalidReason::SignatureInvalid
    );

    let mut extra = serde_json::to_value(&good).expect("value");
    extra["predicate"]["extra"] = json!(1);
    assert_eq!(
        reason_of(&verify(
            &bundle_of(extra.to_string().as_bytes()),
            &id('c'),
            &trusting,
            &list
        )),
        InvalidReason::PredicateInvalid
    );
    let mut missing = serde_json::to_value(&good).expect("value");
    missing["predicate"]
        .as_object_mut()
        .expect("predicate")
        .remove("pushed_at");
    assert_eq!(
        reason_of(&verify(
            &bundle_of(missing.to_string().as_bytes()),
            &id('c'),
            &trusting,
            &list
        )),
        InvalidReason::PredicateInvalid
    );

    let mut other_object = serde_json::to_value(&good).expect("value");
    other_object["predicate"]["tag_object"] = json!(id('b'));
    assert_eq!(
        reason_of(&verify(
            &bundle_of(other_object.to_string().as_bytes()),
            &id('c'),
            &trusting,
            &list
        )),
        InvalidReason::WrongSubject
    );
    // The note sits on a commit other than the statement's `commit`.
    assert_eq!(
        reason_of(&verify(&bundle, &id('d'), &trusting, &list)),
        InvalidReason::WrongSubject
    );
    // The repository differs from the matched entry's `source_repository`.
    let mut other_repository = serde_json::to_value(&good).expect("value");
    other_repository["predicate"]["repository"] = json!("fictional/other");
    assert_eq!(
        reason_of(&verify(
            &bundle_of(other_repository.to_string().as_bytes()),
            &id('c'),
            &trusting,
            &list
        )),
        InvalidReason::WrongSubject
    );
    // A line that is not a bundle at all cannot be parsed.
    assert_eq!(
        reason_of(&verify_line(None, &id('c'), &[])),
        InvalidReason::PredicateInvalid
    );
    assert_eq!(
        reason_of(&verify(
            &json!({"not": "a bundle"}),
            &id('c'),
            &trusting,
            &list
        )),
        InvalidReason::PredicateInvalid
    );
}

#[test]
#[trace("TC-1107", "FR-081-AC-5")]
fn an_identical_entry_is_not_doubled_and_a_new_object_adds_one_and_keeps_the_first() {
    let first = signed(&statement("refs/tags/approve-1", &id('a'), &id('c')));
    let NoteUpdate::Updated(note) = add_entry(None, &first).expect("first entry") else {
        panic!("a new entry updates the note");
    };
    assert_eq!(note_lines(&note).len(), 1);
    // The same ref at the same object id, pushed again: nothing changes.
    let again = signed(&statement("refs/tags/approve-1", &id('a'), &id('c')));
    assert_eq!(add_entry(Some(&note), &again), Ok(NoteUpdate::Unchanged));
    // Re-signed: a different object id gets its own new entry.
    let resigned = signed(&statement("refs/tags/approve-1", &id('b'), &id('c')));
    let NoteUpdate::Updated(two) = add_entry(Some(&note), &resigned).expect("second entry") else {
        panic!("a different object id adds an entry");
    };
    assert!(
        two.starts_with(&note),
        "the earlier entry is never replaced"
    );
    assert_eq!(note_lines(&two).len(), 2);
    // A different ref on the same commit is its own entry too.
    let other_ref = signed(&statement("refs/tags/approve-2", &id('a'), &id('c')));
    assert!(matches!(
        add_entry(Some(&two), &other_ref),
        Ok(NoteUpdate::Updated(_))
    ));
    // Only a bundle of a tag receipt statement is stored.
    assert_eq!(
        add_entry(Some(&note), &json!({"x": 1})),
        Err(TagReceiptError::NoteInvalid)
    );
}

#[test]
#[trace("TC-1108", "FR-081-AC-6")]
fn a_note_over_one_mebibyte_is_refused() {
    let entry = signed(&statement("refs/tags/approve-1", &id('a'), &id('c')));
    let line = serde_json::to_string(&entry).expect("json");
    let mut note = Vec::new();
    while note.len() + line.len() < 1024 * 1024 {
        note.extend_from_slice(b"{\"filler\":true}\n");
    }
    assert_eq!(
        add_entry(Some(&note), &entry),
        Err(TagReceiptError::NoteTooLarge)
    );
    // Nothing in an oversized note is trusted on read either.
    note.extend_from_slice(&vec![b'x'; 1024]);
    assert_eq!(note_lines(&note), vec![None]);
}

#[test]
#[trace("TC-1109", "FR-082-AC-1", "FR-082-AC-2", "FR-082-AC-3", "FR-082-AC-5")]
fn each_receipt_is_judged_on_its_own_object_id() {
    let policy = policy(&json!([]));
    let one = Candidate::Verified(receipt("approve-1", 'a', 'c', &[]));

    // Deleted.
    let deleted = compare(std::slice::from_ref(&one), &tags(&[]), &policy);
    assert_eq!(findings(&deleted), [(FindingStatus::MissingTag, false)]);
    assert!(matches!(
        &deleted[0].finding,
        TagFinding::MissingTag {
            reason: MissingTagReason::TagAbsent,
            current_object: None,
            ..
        }
    ));
    // Re-created as a different object.
    let changed = compare(
        std::slice::from_ref(&one),
        &tags(&[("approve-1", 'b')]),
        &policy,
    );
    assert_eq!(
        changed.len(),
        2,
        "the changed receipt and the unreceipted tag"
    );
    assert!(matches!(
        &changed[0].finding,
        TagFinding::MissingTag { reason: MissingTagReason::TagChanged, current_object: Some(current), .. } if *current == id('b')
    ));
    assert!(matches!(
        &changed[1].finding,
        TagFinding::TagNoReceipt { .. }
    ));
    // Deleted and pushed again at the identical object id: nothing.
    assert_eq!(
        compare(
            std::slice::from_ref(&one),
            &tags(&[("approve-1", 'a')]),
            &policy
        ),
        [] as [engineering_assurance::tag_receipt::Judged; 0]
    );
    // Two receipts for two objects, only the second current: exactly the first.
    let two = Candidate::Verified(receipt("approve-1", 'b', 'c', &[]));
    let repeated = compare(
        &[one.clone(), two.clone()],
        &tags(&[("approve-1", 'b')]),
        &policy,
    );
    assert_eq!(findings(&repeated), [(FindingStatus::MissingTag, false)]);
    assert!(matches!(
        &repeated[0].finding,
        TagFinding::MissingTag { receipt, reason: MissingTagReason::TagChanged, .. } if receipt.tag_object == id('a')
    ));
    // Empty receipts and an empty tag set; the order of the receipts is irrelevant.
    assert_eq!(
        compare(&[], &tags(&[]), &policy),
        [] as [engineering_assurance::tag_receipt::Judged; 0]
    );
    assert_eq!(
        compare(
            &[two.clone(), one.clone()],
            &tags(&[("approve-1", 'b')]),
            &policy
        ),
        repeated
    );
}

#[test]
#[trace("TC-1110", "FR-082-AC-4")]
fn an_invalid_receipt_is_listed_and_never_judged_missing_or_accepted() {
    let invalid = |reason| {
        Candidate::Invalid(InvalidReceipt {
            commit: id('c'),
            reason,
        })
    };
    let candidates = [
        invalid(InvalidReason::SignatureInvalid),
        invalid(InvalidReason::PredicateInvalid),
        invalid(InvalidReason::UntrustedSigner),
    ];
    let accepting = policy(
        &json!([{"ref": "refs/tags/approve-1", "tag_object": null, "accepted_by": "p", "reason": "r"}]),
    );
    let judged = compare(&candidates, &tags(&[]), &accepting);
    assert_eq!(
        findings(&judged),
        [(FindingStatus::InvalidReceipt, false); 3]
    );
}

#[test]
#[trace("TC-1111", "FR-082-AC-6")]
fn a_prefix_tag_without_a_receipt_for_its_current_object_is_unreceipted() {
    let policy = policy(&json!([]));
    let current = tags(&[("approve-1", 'a'), ("release-9", 'a'), ("other-1", 'a')]);
    // The notes ref being absent is an empty receipt set.
    let judged = compare(&[], &current, &policy);
    assert_eq!(
        findings(&judged),
        [
            (FindingStatus::TagNoReceipt, false),
            (FindingStatus::TagNoReceipt, false)
        ]
    );
    assert!(judged.iter().all(|item| match &item.finding {
        TagFinding::TagNoReceipt { reference, .. } => !reference.contains("other-"),
        _ => false,
    }));
    // A tag that gains its receipt is not listed.
    let receipted = [Candidate::Verified(receipt("approve-1", 'a', 'c', &[]))];
    assert_eq!(compare(&receipted, &current, &policy).len(), 1);
    assert!(is_receipted(&receipted, "refs/tags/approve-1", &id('a')));
    assert!(!is_receipted(&receipted, "refs/tags/approve-1", &id('b')));
}

#[test]
#[trace("TC-1112", "FR-082-AC-10", "FR-082-AC-12")]
fn an_acceptance_matches_exactly_and_a_null_object_only_a_tag_absent_finding() {
    let accept = |name: &str, object: Option<char>| {
        json!({
            "ref": format!("refs/tags/{name}"),
            "tag_object": object.map(id),
            "accepted_by": "alice",
            "reason": "reviewed"
        })
    };
    let one = Candidate::Verified(receipt("approve-1", 'a', 'c', &[]));

    // A changed tag accepted by its earlier ref and object.
    let accepted = compare(
        std::slice::from_ref(&one),
        &tags(&[("approve-1", 'b')]),
        &policy(&json!([
            accept("approve-1", Some('a')),
            accept("approve-1", Some('b'))
        ])),
    );
    assert_eq!(accepted.len(), 2);
    let missing = &accepted[0];
    assert_eq!(missing.finding.status(), FindingStatus::MissingTag);
    let acceptance = missing.accepted.as_ref().expect("accepted");
    assert_eq!(acceptance.accepted_by, "alice");
    assert_eq!(acceptance.accepted_reason, "reviewed");
    // The second entry accepts the unreceipted current object; nothing is open.
    assert!(accepted.iter().all(|item| !item.is_open()));

    // The same ref with another object id matches nothing.
    let other = compare(
        std::slice::from_ref(&one),
        &tags(&[]),
        &policy(&json!([accept("approve-1", Some('b'))])),
    );
    assert!(other[0].is_open());
    // A null entry accepts a tag_absent finding ...
    let null_absent = compare(
        std::slice::from_ref(&one),
        &tags(&[]),
        &policy(&json!([accept("approve-1", None)])),
    );
    assert!(!null_absent[0].is_open());
    // ... and neither a tag_changed nor a tag_no_receipt finding.
    let null_changed = compare(
        &[one],
        &tags(&[("approve-1", 'b')]),
        &policy(&json!([accept("approve-1", None)])),
    );
    assert_eq!(null_changed.len(), 2);
    assert!(null_changed.iter().all(Judged::is_open));
    // An unreceipted tag accepted at its current object; moved, it is a finding again.
    let unreceipted = policy(&json!([accept("release-1", Some('a'))]));
    assert!(!compare(&[], &tags(&[("release-1", 'a')]), &unreceipted)[0].is_open());
    assert!(compare(&[], &tags(&[("release-1", 'b')]), &unreceipted)[0].is_open());
}

#[test]
#[trace("TC-1113", "FR-082-AC-8")]
fn the_comparison_reads_no_attestation_and_no_attestation_reader_reads_a_receipt() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let read = |name: &str| std::fs::read_to_string(root.join(name)).expect("source");
    let comparison = read("tag_receipt.rs");
    for forbidden in [
        "VerificationSummary",
        "release_attester",
        "ReleaseAttester",
        "release::",
    ] {
        assert!(!comparison.contains(forbidden), "{forbidden}");
    }
    for name in ["release.rs", "attestation_verify.rs", "release_history.rs"] {
        let source = read(name);
        assert!(!source.contains("tag_receipt"), "{name}");
        assert!(!source.contains("assurance-tag-receipts"), "{name}");
    }
}
