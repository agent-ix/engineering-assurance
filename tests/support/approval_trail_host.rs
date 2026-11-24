// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

use std::{cell::RefCell, process::Command, rc::Rc};

use engineering_assurance::attestation_signing::TransparencyRequirement;
use ix_trace_rs::trace;

use super::*;

fn git(repo: &std::path::Path, arguments: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
        .args(arguments)
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .status()
        .expect("git must start");
    assert!(status.success(), "{arguments:?}");
}

#[test]
#[trace("TC-365", "FR-043-AC-16")]
fn an_expired_total_time_truncates_before_the_first_tag() {
    let directory = tempfile::tempdir().expect("tempdir");
    let repo = directory.path().join("repo");
    std::fs::create_dir(&repo).expect("repo");
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["commit", "-q", "--allow-empty", "-m", "base"]);
    git(&repo, &["tag", "a-light"]);
    git(&repo, &["tag", "-a", "-m", "one", "b-annotated"]);
    git(&repo, &["tag", "-a", "-m", "two", "c-annotated"]);
    let signers = directory.path().join("allowed_signers");
    std::fs::write(&signers, "").expect("signers");
    let request = serde_json::json!({
        "protocol": REQUEST_PROTOCOL,
        "repository": repo.display().to_string(),
        "record_id": "r-1",
        "record_digest": "d-1",
        "allowed_signers": signers.display().to_string(),
    });
    let line = execute_within(request.to_string().as_bytes(), Format::Json, Duration::ZERO)
        .expect("a trail");
    let result: Value = serde_json::from_slice(&line).expect("json");
    assert_eq!(result["entries"], serde_json::json!([]));
    assert_eq!(result["truncated"], true);
    assert_eq!(result["truncation"], "time_limit");
    // Only the lightweight ref before the first annotated tag was reached.
    assert_eq!(result["considered"], 1);
    assert_eq!(result["complete"], false);
}

// ---------------------------------------------------------------------------
// Tag receipts (FR-043, FR-081, FR-082): receipts are verified through a
// verifier double over real git repositories.
// ---------------------------------------------------------------------------

mod receipts {
    use std::collections::BTreeSet;

    use serde_json::json;

    use super::*;
    use crate::tag_receipt_host::{
        Attempt, TagReceiptHostError, Trust, VerifierSource, read_candidates, repository_findings,
        tests::{
            FakeSource, FakeVerifier, add_line, git, identities, identities_json, policy,
            repository, rev, tag_message, write_receipt,
        },
    };

    const RECORD: &str = "r-1";

    fn message(record: &str) -> String {
        format!("Approve\n\nApproved-Record-Id: {record}\nApproved-Record-Digest: d-1")
    }

    struct Setup {
        directory: tempfile::TempDir,
        repo: std::path::PathBuf,
        commit: String,
    }

    fn setup() -> Setup {
        let directory = tempfile::tempdir().expect("tempdir");
        let repo = repository(directory.path());
        let commit = rev(&repo, "HEAD");
        std::fs::write(directory.path().join("allowed_signers"), "").expect("signers");
        std::fs::write(
            directory.path().join("workflow-identities.json"),
            identities_json().to_string(),
        )
        .expect("identities");
        Setup {
            directory,
            repo,
            commit,
        }
    }

    fn trail_for(setup: &Setup, record: &str, accepted: &Value, verifier: FakeVerifier) -> Value {
        let root = setup.directory.path();
        std::fs::write(
            root.join("receipt-policy.json"),
            json!({
                "protocol": "engineering-assurance.receipt-policy/v1",
                "tag_receipts": {"prefixes": ["approve-"], "accepted_tags": accepted}
            })
            .to_string(),
        )
        .expect("policy");
        let request = json!({
            "protocol": REQUEST_PROTOCOL,
            "repository": setup.repo.display().to_string(),
            "record_id": record,
            "record_digest": "d-1",
            "allowed_signers": root.join("allowed_signers").display().to_string(),
            "tag_receipts": {
                "workflow_identities": root.join("workflow-identities.json").display().to_string(),
                "receipt_policy": root.join("receipt-policy.json").display().to_string(),
            },
        });
        let line = execute_with(
            request.to_string().as_bytes(),
            Format::Json,
            Duration::from_secs(60),
            &FakeSource(verifier),
        )
        .expect("a trail");
        serde_json::from_slice(&line).expect("json")
    }

    fn trail(setup: &Setup, record: &str) -> Value {
        trail_for(setup, record, &json!([]), FakeVerifier::default())
    }

    fn trail_without_tag_receipt_policy(
        setup: &Setup,
        record: &str,
        verifier: FakeVerifier,
    ) -> Value {
        let root = setup.directory.path();
        std::fs::write(
            root.join("receipt-policy.json"),
            json!({"protocol": "engineering-assurance.receipt-policy/v1"}).to_string(),
        )
        .expect("policy");
        let request = json!({
            "protocol": REQUEST_PROTOCOL,
            "repository": setup.repo.display().to_string(),
            "record_id": record,
            "record_digest": "d-1",
            "allowed_signers": root.join("allowed_signers").display().to_string(),
            "tag_receipts": {
                "workflow_identities": root.join("workflow-identities.json").display().to_string(),
                "receipt_policy": root.join("receipt-policy.json").display().to_string(),
            },
        });
        let line = execute_with(
            request.to_string().as_bytes(),
            Format::Json,
            Duration::from_secs(60),
            &FakeSource(verifier),
        )
        .expect("a trail");
        serde_json::from_slice(&line).expect("json")
    }

    fn statuses(result: &Value) -> Vec<String> {
        result["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .map(|entry| entry["status"].as_str().expect("status").to_owned())
            .collect()
    }

    /// An annotated tag naming `record`, receipted at its current object.
    fn receipted_tag(setup: &Setup, name: &str, record: &str) -> String {
        tag_message(&setup.repo, name, &message(record), None);
        let object = rev(&setup.repo, &format!("refs/tags/{name}"));
        write_receipt(
            &setup.repo,
            &format!("refs/tags/{name}"),
            &object,
            &setup.commit,
            &[record],
            true,
        );
        object
    }

    #[test]
    #[trace("TC-1126", "FR-043-AC-20")]
    fn a_deleted_tag_is_a_missing_tag_entry_never_approved() {
        let setup = setup();
        let object = receipted_tag(&setup, "approve-1", RECORD);
        git(&setup.repo, &["tag", "-d", "approve-1"]);
        let result = trail(&setup, RECORD);
        assert_eq!(result["tag_receipts"]["checked"], true);
        assert_eq!(result["tag_receipts"]["considered"], 1);
        let entries = result["entries"].as_array().expect("entries");
        assert_eq!(entries.len(), 1, "{result}");
        let entry = &entries[0];
        assert_eq!(entry["status"], "missing_tag");
        assert_eq!(entry["reason"], "tag_absent");
        assert_eq!(entry["tag"], "approve-1");
        assert_eq!(entry["object"], object);
        assert_eq!(entry["commit"], setup.commit);
        assert_eq!(entry["attributed"], true);
        assert!(entry.get("current_object").is_none());
        assert!(entry.get("claimed_at").is_none() && entry.get("approver").is_none());
        assert_eq!(result["complete"], false);
    }

    #[test]
    #[trace("TC-1127", "FR-043-AC-21")]
    fn a_tag_pushed_again_at_the_same_object_passes_and_a_re_signed_tag_is_changed() {
        let setup = setup();
        let object = receipted_tag(&setup, "approve-1", RECORD);
        git(&setup.repo, &["tag", "-d", "approve-1"]);
        git(&setup.repo, &["update-ref", "refs/tags/approve-1", &object]);
        let again = trail(&setup, RECORD);
        let entries = again["entries"].as_array().expect("entries");
        assert_eq!(entries.len(), 1, "{again}");
        assert_eq!(entries[0]["tag"], "approve-1");
        assert_eq!(entries[0]["tag_receipt"], "verified");
        assert!(!statuses(&again).contains(&"missing_tag".to_owned()));
        assert_eq!(again["complete"], true);

        git(
            &setup.repo,
            &[
                "tag",
                "-f",
                "-a",
                "-m",
                &message(RECORD).replacen("Approve", "Approve again", 1),
                "approve-1",
            ],
        );
        let resigned = rev(&setup.repo, "refs/tags/approve-1");
        let changed = trail(&setup, RECORD);
        let entries = changed["entries"].as_array().expect("entries");
        assert_eq!(entries.len(), 2, "{changed}");
        let current = entries
            .iter()
            .find(|entry| entry["tag_receipt"].is_string())
            .expect("the tag's own entry");
        assert_eq!(current["tag_receipt"], "none");
        assert_eq!(current["object"], resigned);
        let missing = entries
            .iter()
            .find(|entry| entry["status"] == "missing_tag")
            .expect("the earlier receipt");
        assert_eq!(missing["reason"], "tag_changed");
        assert_eq!(missing["object"], object);
        assert_eq!(missing["current_object"], resigned);
        assert_eq!(changed["complete"], false);
    }

    #[test]
    #[trace("TC-1128", "FR-043-AC-22", "FR-043-AC-33", "FR-043-AC-34")]
    fn receipts_that_do_not_verify_are_listed_as_invalid_receipt_and_never_dropped() {
        let setup = setup();
        let tampered = write_receipt(
            &setup.repo,
            "refs/tags/approve-1",
            &"a".repeat(40),
            &setup.commit,
            &[RECORD],
            true,
        );
        let unlisted = write_receipt(
            &setup.repo,
            "refs/tags/approve-2",
            &"b".repeat(40),
            &setup.commit,
            &[RECORD],
            true,
        );
        let verifier = FakeVerifier {
            invalid: BTreeSet::from([tampered.to_string()]),
            untrusted: BTreeSet::from([unlisted.to_string()]),
            ..FakeVerifier::default()
        };
        let result = trail_for(&setup, RECORD, &json!([]), verifier);
        let entries = result["entries"].as_array().expect("entries");
        assert_eq!(entries.len(), 2, "{result}");
        let mut reasons: Vec<&str> = entries
            .iter()
            .map(|entry| {
                assert_eq!(entry["status"], "invalid_receipt");
                assert_eq!(entry["attributed"], false);
                assert_eq!(entry["commit"], setup.commit);
                assert!(
                    entry.get("tag").is_none(),
                    "an unverified receipt names no tag"
                );
                assert!(
                    entry.get("ref").is_none(),
                    "an unverified receipt exposes no untrusted ref"
                );
                entry["reason"].as_str().expect("reason")
            })
            .collect();
        reasons.sort_unstable();
        assert_eq!(reasons, ["signature_invalid", "untrusted_signer"]);
        assert_eq!(result["complete"], false);
    }

    #[test]
    #[trace("TC-1129", "FR-043-AC-23")]
    fn a_receipt_belongs_to_the_record_its_ids_name_and_an_unreadable_one_to_every_record() {
        let setup = setup();
        write_receipt(
            &setup.repo,
            "refs/tags/approve-other",
            &"a".repeat(40),
            &setup.commit,
            &["another-record"],
            true,
        );
        write_receipt(
            &setup.repo,
            "refs/tags/approve-blind",
            &"b".repeat(40),
            &setup.commit,
            &[],
            false,
        );
        for record in [RECORD, "r-2"] {
            let result = trail(&setup, record);
            let entries = result["entries"].as_array().expect("entries");
            assert_eq!(entries.len(), 1, "{record}: {result}");
            assert_eq!(entries[0]["tag"], "approve-blind");
            assert_eq!(entries[0]["status"], "missing_tag");
            assert_eq!(entries[0]["attributed"], false);
        }
        let only_other = trail(&setup, "another-record");
        let tags: BTreeSet<String> = only_other["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .map(|entry| entry["tag"].as_str().expect("tag").to_owned())
            .collect();
        assert_eq!(
            tags,
            BTreeSet::from(["approve-blind".to_owned(), "approve-other".to_owned()])
        );
    }

    #[test]
    #[trace("FR-043-AC-28", "FR-043-AC-30", "FR-082-AC-12")]
    fn an_unreceipted_policy_tag_for_another_record_is_visible_and_can_be_accepted() {
        let setup = setup();
        tag_message(
            &setup.repo,
            "approve-other",
            &message("another-record"),
            None,
        );
        let object = rev(&setup.repo, "refs/tags/approve-other");

        let missing = trail(&setup, RECORD);
        assert_eq!(missing["entries"].as_array().expect("entries").len(), 1);
        let finding = &missing["entries"][0];
        assert_eq!(finding["tag"], "approve-other");
        assert_eq!(finding["object"], object);
        assert_eq!(finding["status"], "tag_no_receipt");
        assert_eq!(finding["tag_receipt"], "none");
        assert!(finding.get("reason").is_none());
        assert!(finding.get("commit").is_none());
        assert_eq!(missing["complete"], false);

        let accepted = trail_for(
            &setup,
            RECORD,
            &json!([{
                "ref": "refs/tags/approve-other",
                "tag_object": object,
                "accepted_by": "alice",
                "reason": "existing tag predates receipts"
            }]),
            FakeVerifier::default(),
        );
        let finding = &accepted["entries"][0];
        assert_eq!(finding["status"], "accepted");
        assert_eq!(finding["finding"], "tag_no_receipt");
        assert_eq!(finding["tag_receipt"], "none");
        assert_eq!(finding["accepted_by"], "alice");
        assert_eq!(finding["accepted_reason"], "existing tag predates receipts");
        assert_eq!(accepted["complete"], true);
    }

    #[test]
    #[trace("FR-043-AC-32")]
    fn a_policy_without_tag_receipts_skips_existing_receipt_notes() {
        let setup = setup();
        tag_message(&setup.repo, "approve-record", &message(RECORD), None);
        write_receipt(
            &setup.repo,
            "refs/tags/approve-deleted",
            &"a".repeat(40),
            &setup.commit,
            &[RECORD],
            true,
        );

        let visits = Rc::new(RefCell::new(Vec::new()));
        let result = trail_without_tag_receipt_policy(
            &setup,
            RECORD,
            FakeVerifier {
                policy_visits: Some(Rc::clone(&visits)),
                ..FakeVerifier::default()
            },
        );
        assert_eq!(result["tag_receipts"]["checked"], false);
        assert_eq!(result["tag_receipts"]["considered"], 0);
        let entries = result["entries"].as_array().expect("entries");
        assert_eq!(entries.len(), 1, "{result}");
        assert_eq!(entries[0]["tag"], "approve-record");
        assert_eq!(entries[0]["status"], "unsigned");
        assert!(entries[0].get("tag_receipt").is_none());
        assert!(visits.borrow().is_empty(), "no stored receipt was verified");
        assert_eq!(result["complete"], true);
    }

    struct TwoRootFakeSource(FakeVerifier);

    impl VerifierSource for TwoRootFakeSource {
        fn attempts(&self) -> Result<Vec<Attempt<'_>>, TagReceiptHostError> {
            Ok(vec![
                Attempt {
                    verifier: &self.0,
                    transparency: TransparencyRequirement::PublicLog,
                },
                Attempt {
                    verifier: &self.0,
                    transparency: TransparencyRequirement::TimestampOnly,
                },
            ])
        }
    }

    #[test]
    #[trace("FR-043-AC-31")]
    fn receipt_verification_uses_both_roots_and_each_tag_receipt_identity() {
        let directory = tempfile::tempdir().expect("tempdir");
        let repo = repository(directory.path());
        let commit = rev(&repo, "HEAD");
        let bundle = write_receipt(
            &repo,
            "refs/tags/approve-1",
            &"a".repeat(40),
            &commit,
            &[RECORD],
            true,
        );
        let mut identity_document = identities_json();
        let first = identity_document["identities"][0].clone();
        let mut second = first.clone();
        second["repository"] = json!("fictional/second-assurance");
        second["workflow_path"] = json!(".github/workflows/second-receipt.yml");
        let mut branch_receipt_writer = first.clone();
        branch_receipt_writer["repository"] = json!("fictional/branch-assurance");
        branch_receipt_writer["workflow_path"] = json!(".github/workflows/branch-receipt.yml");
        branch_receipt_writer
            .as_object_mut()
            .expect("identity object")
            .remove("source_ref_prefix");
        branch_receipt_writer["source_ref"] = json!("refs/heads/main");
        identity_document["identities"] = json!([first, second, branch_receipt_writer]);
        let identity_document =
            WorkflowIdentityDocument::parse(identity_document.to_string().as_bytes())
                .expect("identities");
        let visits = Rc::new(RefCell::new(Vec::new()));
        let source = TwoRootFakeSource(FakeVerifier {
            invalid: BTreeSet::from([bundle.to_string()]),
            policy_visits: Some(Rc::clone(&visits)),
            ..FakeVerifier::default()
        });
        let trust = Trust::new(&identity_document, &source).expect("trust");
        let candidates = read_candidates(&repo, &trust).expect("verified candidate list");
        assert_eq!(candidates.len(), 1);
        assert!(matches!(
            &candidates[0],
            engineering_assurance::tag_receipt::Candidate::Invalid(_)
        ));

        let visits = visits.borrow();
        assert_eq!(
            visits
                .iter()
                .map(|policy| policy.repository.as_str())
                .collect::<Vec<_>>(),
            [
                "fictional/engineering-assurance",
                "fictional/second-assurance",
                "fictional/engineering-assurance",
                "fictional/second-assurance",
            ]
        );
        assert_eq!(
            visits
                .iter()
                .map(|policy| policy.transparency)
                .collect::<Vec<_>>(),
            [
                TransparencyRequirement::PublicLog,
                TransparencyRequirement::PublicLog,
                TransparencyRequirement::TimestampOnly,
                TransparencyRequirement::TimestampOnly,
            ]
        );
    }

    #[test]
    #[trace("TC-1130", "FR-043-AC-25", "FR-082-AC-5")]
    fn the_trail_and_the_repository_check_list_the_same_findings_for_the_same_inputs() {
        let setup = setup();
        receipted_tag(&setup, "approve-1", RECORD);
        git(&setup.repo, &["tag", "-d", "approve-1"]);
        let tampered = write_receipt(
            &setup.repo,
            "refs/tags/approve-2",
            &"c".repeat(40),
            &setup.commit,
            &[RECORD],
            true,
        );
        let verifier = || FakeVerifier {
            invalid: BTreeSet::from([tampered.to_string()]),
            ..FakeVerifier::default()
        };
        let result = trail_for(&setup, RECORD, &json!([]), verifier());
        let from_trail: Vec<(String, String, String)> = result["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .map(|entry| {
                (
                    entry["status"].as_str().expect("status").to_owned(),
                    entry["reason"].as_str().expect("reason").to_owned(),
                    entry["commit"].as_str().expect("commit").to_owned(),
                )
            })
            .collect();
        let judged = repository_findings(
            &setup.repo,
            &identities(),
            &policy(&json!([])),
            &FakeSource(verifier()),
        )
        .expect("findings");
        let from_check: Vec<(String, String, String)> = judged
            .iter()
            .map(|item| {
                let line =
                    serde_json::to_value(crate::tag_receipt_host::TagFindingLine::from(item))
                        .expect("line");
                (
                    line["status"].as_str().expect("status").to_owned(),
                    line["reason"].as_str().expect("reason").to_owned(),
                    line["commit"].as_str().expect("commit").to_owned(),
                )
            })
            .collect();
        assert_eq!(from_trail, from_check);
        assert_eq!(from_trail.len(), 2);
    }

    #[test]
    #[trace("TC-1131", "FR-043-AC-29")]
    fn an_accepted_missing_tag_is_listed_accepted_and_does_not_make_the_trail_incomplete() {
        let setup = setup();
        let object = receipted_tag(&setup, "approve-1", RECORD);
        git(&setup.repo, &["tag", "-d", "approve-1"]);
        let entry = |tag_object: Option<&str>| {
            json!({
                "ref": "refs/tags/approve-1",
                "tag_object": tag_object,
                "accepted_by": "alice",
                "reason": "retired on purpose"
            })
        };
        let accepted = trail_for(
            &setup,
            RECORD,
            &json!([entry(Some(&object))]),
            FakeVerifier::default(),
        );
        let listed = &accepted["entries"][0];
        assert_eq!(listed["status"], "accepted");
        assert_eq!(listed["finding"], "missing_tag");
        assert_eq!(listed["accepted_by"], "alice");
        assert_eq!(listed["accepted_reason"], "retired on purpose");
        assert_eq!(listed["reason"], "tag_absent");
        assert_eq!(accepted["complete"], true);

        // An entry that differs in either member changes nothing.
        let other_object = entry(Some(&"d".repeat(40)));
        let mut other_ref = entry(Some(&object));
        other_ref["ref"] = json!("refs/tags/approve-9");
        for entries in [json!([other_object]), json!([other_ref])] {
            let result = trail_for(&setup, RECORD, &entries, FakeVerifier::default());
            assert_eq!(result["entries"][0]["status"], "missing_tag");
            assert_eq!(result["complete"], false);
        }
        // A null object accepts a tag that is absent.
        let null = trail_for(
            &setup,
            RECORD,
            &json!([entry(None)]),
            FakeVerifier::default(),
        );
        assert_eq!(null["entries"][0]["status"], "accepted");
        assert_eq!(null["complete"], true);

        // An invalid receipt is never accepted.
        let tampered = write_receipt(
            &setup.repo,
            "refs/tags/approve-2",
            &"e".repeat(40),
            &setup.commit,
            &[RECORD],
            true,
        );
        let verifier = FakeVerifier {
            invalid: BTreeSet::from([tampered.to_string()]),
            ..FakeVerifier::default()
        };
        let mut everything = entry(None);
        everything["ref"] = json!("refs/tags/approve-2");
        let result = trail_for(&setup, RECORD, &json!([entry(None), everything]), verifier);
        let invalid: Vec<_> = result["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .filter(|entry| entry["reason"] == "signature_invalid")
            .collect();
        assert_eq!(invalid.len(), 1);
        assert_eq!(invalid[0]["status"], "invalid_receipt");
        assert_eq!(result["complete"], false);
    }

    #[test]
    #[trace("TC-1132", "FR-043-AC-22")]
    fn a_line_that_is_not_a_bundle_is_an_invalid_receipt() {
        let setup = setup();
        add_line(&setup.repo, &setup.commit, "this is not a bundle");
        let result = trail(&setup, RECORD);
        assert_eq!(result["entries"][0]["status"], "invalid_receipt");
        assert_eq!(result["entries"][0]["reason"], "predicate_invalid");
    }
}
