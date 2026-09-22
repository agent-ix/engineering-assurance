// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! FR-022: the claim-strength vocabulary, its published contract entry, its
//! absence of any ordering, and its reachability without the `full` feature.

use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    process::{Command, Output},
};

use engineering_assurance::{
    claim_strength::ClaimStrength,
    semantics::{SemanticErrorKind, validate_ownership_registry_bytes},
};
use ix_trace_rs::trace;
use serde_json::{Value, json};

const WIRE_NAMES: [&str; 4] = ["proven", "bounded-checked", "tested", "observed"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn ownership_registry() -> Value {
    let bytes = fs::read(
        root().join("engineering_assurance/contracts/verification-semantics-ownership-v1.json"),
    )
    .expect("ownership registry must be readable");
    serde_json::from_slice(&bytes).expect("ownership registry must parse")
}

fn registry_refusal(registry: &Value) -> SemanticErrorKind {
    validate_ownership_registry_bytes(&serde_json::to_vec(registry).expect("mutant must serialize"))
        .expect_err("a misdeclared claim-strength vocabulary must be refused")
        .kind()
}

#[test]
#[trace("TC-150", "FR-022-AC-1")]
#[trace("TC-150", "FR-022-AC-5")]
fn tc_150_wire_names_round_trip_and_the_contract_publishes_exactly_them() {
    let listed: Vec<&str> = ClaimStrength::ALL
        .into_iter()
        .map(ClaimStrength::wire_name)
        .collect();
    assert_eq!(listed, WIRE_NAMES);

    for strength in ClaimStrength::ALL {
        let encoded = serde_json::to_value(strength).expect("a strength must serialize");
        assert_eq!(encoded, json!(strength.wire_name()));
        assert_eq!(strength.to_string(), strength.wire_name());
        let decoded: ClaimStrength =
            serde_json::from_value(encoded).expect("a wire name must deserialize");
        assert_eq!(decoded, strength);
    }

    // The ownership registry is the published statement of the vocabulary: the
    // exact values, one owner, one strength per advanced result, and no order.
    let registry = ownership_registry();
    assert_eq!(
        registry["claim_strength"],
        json!({
            "authority": "engineering_assurance",
            "authoritative_type": "engineering_assurance::claim_strength::ClaimStrength",
            "cardinality": "exactly_one_per_advanced_result",
            "ordering": "unordered",
            "values": WIRE_NAMES,
        })
    );
    validate_ownership_registry_bytes(&serde_json::to_vec(&registry).expect("registry encodes"))
        .expect("the committed registry must validate");

    // Every way of misdeclaring the entry is refused with its own typed reason,
    // so dropping any one comparison from the validator fails here.
    for (field, value) in [
        ("authority", json!("quire")),
        ("authoritative_type", json!("quire_protocol::ClaimStrength")),
        ("cardinality", json!("one_or_more_per_advanced_result")),
        ("ordering", json!("strongest_first")),
        ("values", json!(["proven", "bounded-checked", "tested"])),
        (
            "values",
            json!(["bounded-checked", "proven", "tested", "observed"]),
        ),
        (
            "values",
            json!([
                "proven",
                "bounded-checked",
                "tested",
                "observed",
                "observed"
            ]),
        ),
    ] {
        let mut mutant = registry.clone();
        mutant["claim_strength"][field] = value.clone();
        assert_eq!(
            registry_refusal(&mutant),
            SemanticErrorKind::InvalidClaimStrengthVocabulary,
            "{field} = {value}"
        );
    }
    // An unknown value and a missing entry are claim-strength refusals too, not
    // encoding failures: values are parsed through `FromStr` after decoding.
    let mut unknown = registry.clone();
    unknown["claim_strength"]["values"] = json!(["proven", "bounded-checked", "tested", "checked"]);
    assert_eq!(
        registry_refusal(&unknown),
        SemanticErrorKind::InvalidClaimStrengthVocabulary
    );
    let mut absent = registry.clone();
    absent
        .as_object_mut()
        .expect("the registry is an object")
        .remove("claim_strength");
    assert_eq!(
        registry_refusal(&absent),
        SemanticErrorKind::InvalidClaimStrengthVocabulary
    );
    // A non-string value is still malformed input, not a vocabulary question.
    let mut malformed = registry;
    malformed["claim_strength"]["values"] = json!(["proven", 1]);
    assert_eq!(
        registry_refusal(&malformed),
        SemanticErrorKind::InvalidInputEncoding
    );

    // The foreign-language copies list the same names in all three languages.
    // (Byte identity with the generator is TC-100's comparison over the whole
    // generated directory.)
    let generated = root().join("engineering_assurance/fixtures/verification-semantics/generated");
    for (name, prefix, suffix) in [
        ("claim_strengths.py", "    \"", "\","),
        ("claim_strengths.ts", "  \"", "\","),
        ("claim_strengths.rs", "    \"", "\","),
    ] {
        let body = fs::read_to_string(generated.join(name)).expect("committed fixture must exist");
        let names: Vec<&str> = body
            .lines()
            .filter_map(|line| line.strip_prefix(prefix)?.strip_suffix(suffix))
            .collect();
        assert_eq!(names, WIRE_NAMES, "{name} lists a different vocabulary");
    }
}

#[test]
#[trace("TC-151", "FR-022-AC-2")]
fn tc_151_decoding_refuses_every_label_outside_the_vocabulary() {
    for label in [
        json!("Proven"),
        json!("bounded_checked"),
        json!("BoundedChecked"),
        json!("verified"),
        json!("checked"),
        json!("strong"),
        json!(""),
        json!(0),
        json!(null),
    ] {
        assert!(
            serde_json::from_value::<ClaimStrength>(label.clone()).is_err(),
            "{label} must not decode as a claim strength"
        );
    }
    let error = "strong"
        .parse::<ClaimStrength>()
        .expect_err("a label outside the vocabulary is not a claim strength");
    assert_eq!(error.value(), "strong");
}

/// A throwaway downstream crate depending on Engineering Assurance with only
/// the `claim-strength` feature, checked offline in its own target directory.
struct Consumer {
    root: tempfile::TempDir,
}

impl Consumer {
    fn new() -> Self {
        let root = tempfile::tempdir().expect("consumer root");
        fs::create_dir(root.path().join("src")).expect("consumer source directory");
        let manifest_dir: &str = env!("CARGO_MANIFEST_DIR");
        fs::write(
            root.path().join("Cargo.toml"),
            format!(
                "[package]\nname='claim-strength-consumer-fixture'\nversion='0.0.0'\nedition='2024'\nrust-version='1.98.1'\n[dependencies]\nengineering-assurance={{path={manifest_dir:?},default-features=false,features=['claim-strength']}}\n"
            ),
        )
        .expect("consumer manifest");
        Self { root }
    }

    fn check(&self, main: &str) -> Output {
        fs::write(self.root.path().join("src/main.rs"), main).expect("consumer source");
        Command::new(env!("CARGO"))
            .args([
                "check",
                "--offline",
                "--message-format",
                "short",
                "--manifest-path",
            ])
            .arg(self.root.path().join("Cargo.toml"))
            .env("CARGO_BUILD_JOBS", "2")
            .env("CARGO_TARGET_DIR", self.root.path().join("target"))
            .output()
            .expect("consumer cargo check must launch")
    }
}

#[test]
#[trace("TC-152", "FR-022-AC-3")]
fn tc_152_no_consumer_can_order_rank_or_default_a_claim_strength() {
    // The in-crate trait-absence probe fails the library build if `PartialOrd`
    // or `Ord` is ever derived. This is the consumer-side view of the same
    // property: each use a ranking consumer would reach for must fail to
    // compile, and for the reason named, not for an unrelated one.
    let consumer = Consumer::new();
    let baseline = consumer.check(
        "use engineering_assurance::claim_strength::ClaimStrength;\nfn main(){let _ = ClaimStrength::Proven == ClaimStrength::Tested;}\n",
    );
    assert!(
        baseline.status.success(),
        "the probe harness must compile a valid use before its refusals mean anything: {}",
        String::from_utf8_lossy(&baseline.stderr)
    );
    for (probe, expected_code) in [
        (
            "let _ = ClaimStrength::Proven < ClaimStrength::Tested;",
            "E0369",
        ),
        (
            "let _ = ClaimStrength::Proven.cmp(&ClaimStrength::Tested);",
            "E0599",
        ),
        (
            "let _ = ClaimStrength::Proven.partial_cmp(&ClaimStrength::Tested);",
            "E0599",
        ),
        ("let _ = [ClaimStrength::Tested].iter().max();", "E0277"),
        (
            "let _: std::collections::BTreeSet<ClaimStrength> = ClaimStrength::ALL.into_iter().collect();",
            "E0277",
        ),
        ("let _ = ClaimStrength::default();", "E0599"),
    ] {
        let output = consumer.check(&format!(
            "use engineering_assurance::claim_strength::ClaimStrength;\nfn main(){{{probe}}}\n"
        ));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "`{probe}` compiled");
        assert!(
            stderr.contains(expected_code),
            "`{probe}` failed for a reason other than {expected_code}: {stderr}"
        );
    }
}

#[test]
#[trace("TC-153", "FR-022-AC-4", "FR-022-CON-1")]
fn tc_153_a_minimal_downstream_uses_claim_strength_without_the_full_feature() {
    let consumer = Consumer::new();
    let output = consumer.check(
        "use engineering_assurance::claim_strength::{ClaimStrength, UnknownClaimStrength};\n\
         fn main(){\n\
         let parsed: Result<ClaimStrength, UnknownClaimStrength> = \"tested\".parse();\n\
         let _ = (parsed, ClaimStrength::ALL, ClaimStrength::Observed.wire_name(), ClaimStrength::Proven.to_string());\n\
         }\n",
    );
    assert!(
        output.status.success(),
        "minimal claim-strength consumer must compile: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--offline",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(consumer.root.path().join("Cargo.toml"))
        .output()
        .expect("consumer metadata must launch");
    assert!(metadata.status.success());
    let graph: Value = serde_json::from_slice(&metadata.stdout).expect("metadata JSON");
    let resolved: BTreeSet<&str> = graph["packages"]
        .as_array()
        .expect("metadata packages")
        .iter()
        .filter_map(|package| package["name"].as_str())
        .collect();
    assert!(
        resolved.contains("serde") && resolved.contains("engineering-assurance"),
        "the census read no meaningful graph: {resolved:?}"
    );
    // Engineering Assurance is resolved with exactly the one feature asked for,
    // so no other optional dependency of it can be active; this reads the
    // resolver's answer rather than a hand-kept list of crate names.
    let assurance_id = graph["packages"]
        .as_array()
        .expect("metadata packages")
        .iter()
        .find(|package| package["name"] == "engineering-assurance")
        .and_then(|package| package["id"].as_str())
        .expect("Engineering Assurance package id");
    let features = graph["resolve"]["nodes"]
        .as_array()
        .expect("metadata resolve nodes")
        .iter()
        .find(|node| node["id"] == assurance_id)
        .map(|node| node["features"].clone())
        .expect("Engineering Assurance resolve node");
    assert_eq!(features, json!(["claim-strength"]));
    // Whole-graph: nothing may pull in serde_json, so the consumer cannot
    // inherit `arbitrary_precision` (FR-022-CON-1).
    assert!(
        !resolved.contains("serde_json"),
        "a claim-strength-only consumer resolved serde_json: {resolved:?}"
    );
}
