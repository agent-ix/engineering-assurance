// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Additive-migration parity and adverse coverage for package membership.

use std::{
    io::Write,
    process::{Command, Stdio},
};

use engineering_assurance::package_membership::{
    MAX_PACKAGE_MEMBERS, PackageMembershipCategory, PackageMembershipError,
    PackageMembershipFinding, PackageMembershipOutcome, PackageMembershipPolicy,
};
use ix_trace_rs::trace;
use serde::{Deserialize, Serialize};

const PYTHON_REFERENCE: &str = r#"
import importlib.util
import json
import pathlib
import sys

root = pathlib.Path.cwd()
path = root / "scripts" / "audit_packages.py"
spec = importlib.util.spec_from_file_location("retained_package_audit", path)
module = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = module
spec.loader.exec_module(module)

cases = json.load(sys.stdin)
results = []
for case in cases:
    extra, missing = module.member_mismatch(set(case["actual"]), set(case["expected"]))
    results.append({"extra": extra, "missing": missing})
print(json.dumps(results, separators=(",", ":")))
"#;

#[derive(Deserialize, Eq, PartialEq, Debug)]
struct ReferenceResult {
    extra: Vec<String>,
    missing: Vec<String>,
}

#[derive(Serialize)]
struct ReferenceCase<'a> {
    expected: &'a [String],
    actual: &'a [String],
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn reference_results(cases: &[ReferenceCase<'_>]) -> Vec<ReferenceResult> {
    let mut child = Command::new("python3")
        .args(["-c", PYTHON_REFERENCE])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("retained Python package audit must start during additive migration");
    child
        .stdin
        .take()
        .expect("piped stdin must exist")
        .write_all(&serde_json::to_vec(cases).expect("fixtures must serialize"))
        .expect("fixtures must be writable");
    let output = child.wait_with_output().expect("reference must terminate");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("reference results must be JSON")
}

fn result_paths(
    findings: &[PackageMembershipFinding],
    category: PackageMembershipCategory,
) -> Vec<String> {
    findings
        .iter()
        .filter(|finding| finding.category == category)
        .map(|finding| finding.path.clone().expect("safe path finding"))
        .collect()
}

#[test]
#[trace("TC-120", "FR-017-AC-6", "FR-017-CON-3")]
fn safe_unique_membership_matches_retained_extra_and_missing_behavior() {
    let inputs = [
        (
            strings(&["a.txt", "dir/b.json"]),
            strings(&["a.txt", "dir/b.json"]),
        ),
        (Vec::new(), Vec::new()),
        (strings(&["a.txt"]), Vec::new()),
        (Vec::new(), strings(&["z.txt"])),
        (strings(&["a.txt", "b.txt"]), strings(&["a.txt", "c.txt"])),
        (strings(&["Case.txt"]), strings(&["case.txt"])),
        (strings(&["dir/file.txt"]), strings(&["dir%2Ffile.txt"])),
    ];
    let cases: Vec<_> = inputs
        .iter()
        .map(|(expected, actual)| ReferenceCase { expected, actual })
        .collect();
    let references = reference_results(&cases);

    for ((expected, actual), reference) in inputs.iter().zip(references) {
        let result = PackageMembershipPolicy::new(expected)
            .expect("safe unique policy")
            .compare(actual)
            .expect("bounded observation");
        assert_eq!(
            result_paths(
                &result.findings,
                PackageMembershipCategory::UnexpectedMember
            ),
            reference.extra
        );
        assert_eq!(
            result_paths(&result.findings, PackageMembershipCategory::MissingMember),
            reference.missing
        );
        assert_eq!(
            result.outcome,
            if result.findings.is_empty() {
                PackageMembershipOutcome::Accepted
            } else {
                PackageMembershipOutcome::Withheld
            }
        );
    }
}

#[test]
#[trace("TC-120", "FR-017-AC-6", "FR-017-CON-3")]
fn unsafe_and_duplicate_members_are_typed_without_unsafe_path_disclosure() {
    let maximum_path = "a".repeat(4_096);
    let overlong = "a".repeat(4_097);
    let unsafe_paths = vec![
        String::new(),
        "/private".to_owned(),
        "C:/private".to_owned(),
        "dir\\file".to_owned(),
        "dir/".to_owned(),
        "dir//file".to_owned(),
        "./file".to_owned(),
        "dir/../file".to_owned(),
        "dir/\u{0}file".to_owned(),
        overlong,
    ];
    for path in &unsafe_paths {
        let error = PackageMembershipPolicy::new(std::slice::from_ref(path))
            .expect_err("unsafe expected path must refuse");
        assert_eq!(error, PackageMembershipError::ExpectedPathInvalid);
        if !path.is_empty() {
            assert!(!error.to_string().contains(path));
        }
    }
    let maximum_policy = PackageMembershipPolicy::new(std::slice::from_ref(&maximum_path))
        .expect("4,096-byte path must be admitted");
    assert_eq!(
        maximum_policy
            .compare(std::slice::from_ref(&maximum_path))
            .expect("bounded maximum path")
            .outcome,
        PackageMembershipOutcome::Accepted
    );

    let duplicate = strings(&["safe/file.txt", "safe/file.txt"]);
    assert_eq!(
        PackageMembershipPolicy::new(&duplicate),
        Err(PackageMembershipError::ExpectedPathDuplicate {
            path: "safe/file.txt".to_owned(),
        })
    );

    let expected = strings(&["safe/file.txt"]);
    let mut observed = unsafe_paths;
    observed.extend(strings(&["safe/file.txt", "safe/file.txt"]));
    let result = PackageMembershipPolicy::new(&expected)
        .expect("safe policy")
        .compare(&observed)
        .expect("bounded observation");
    assert_eq!(result.outcome, PackageMembershipOutcome::Withheld);
    assert_eq!(
        result
            .findings
            .iter()
            .filter(|finding| finding.category == PackageMembershipCategory::ActualPathInvalid)
            .count(),
        10
    );
    assert!(
        result
            .findings
            .iter()
            .filter(|finding| finding.category == PackageMembershipCategory::ActualPathInvalid)
            .all(|finding| finding.path.is_none())
    );
    assert_eq!(
        result_paths(
            &result.findings,
            PackageMembershipCategory::ActualPathDuplicate
        ),
        ["safe/file.txt"]
    );
    let encoded = serde_json::to_string(&result).expect("result must serialize");
    assert!(!encoded.contains("/private"));
    assert!(!encoded.contains("C:/private"));
}

#[test]
#[trace("TC-120", "FR-017-AC-6", "FR-017-CON-3")]
fn findings_are_canonical_and_input_permutation_invariant() {
    let expected = strings(&["b.txt", "a.txt", "missing/z.txt"]);
    let observed = strings(&[
        "extra/z.txt",
        "a.txt",
        "duplicate.txt",
        "../unsafe",
        "duplicate.txt",
    ]);
    let policy = PackageMembershipPolicy::new(&expected).expect("safe policy");
    let first = policy.compare(&observed).expect("bounded observation");
    let reversed = policy
        .compare(&observed.iter().rev().cloned().collect::<Vec<_>>())
        .expect("bounded observation");
    let reversed_policy =
        PackageMembershipPolicy::new(&expected.iter().rev().cloned().collect::<Vec<_>>())
            .expect("reordered policy is safe");
    let both_reversed = reversed_policy
        .compare(&observed.iter().rev().cloned().collect::<Vec<_>>())
        .expect("bounded observation");
    assert_eq!(first, reversed);
    assert_eq!(first, both_reversed);
    assert_eq!(
        first.findings,
        vec![
            PackageMembershipFinding {
                path: None,
                category: PackageMembershipCategory::ActualPathInvalid,
            },
            PackageMembershipFinding {
                path: Some("duplicate.txt".to_owned()),
                category: PackageMembershipCategory::ActualPathDuplicate,
            },
            PackageMembershipFinding {
                path: Some("duplicate.txt".to_owned()),
                category: PackageMembershipCategory::UnexpectedMember,
            },
            PackageMembershipFinding {
                path: Some("extra/z.txt".to_owned()),
                category: PackageMembershipCategory::UnexpectedMember,
            },
            PackageMembershipFinding {
                path: Some("b.txt".to_owned()),
                category: PackageMembershipCategory::MissingMember,
            },
            PackageMembershipFinding {
                path: Some("missing/z.txt".to_owned()),
                category: PackageMembershipCategory::MissingMember,
            },
        ]
    );
}

#[test]
#[trace("TC-120", "FR-017-AC-6", "FR-017-CON-3")]
fn population_ceiling_accepts_maximum_and_refuses_over_maximum() {
    let maximum: Vec<_> = (0..MAX_PACKAGE_MEMBERS)
        .map(|index| format!("members/{index:05}.txt"))
        .collect();
    let policy = PackageMembershipPolicy::new(&maximum).expect("maximum policy is admitted");
    assert_eq!(
        policy
            .compare(&maximum)
            .expect("maximum observation is admitted")
            .outcome,
        PackageMembershipOutcome::Accepted
    );

    let mut too_many = maximum;
    too_many.push("members/overflow.txt".to_owned());
    assert_eq!(
        PackageMembershipPolicy::new(&too_many),
        Err(PackageMembershipError::ExpectedPopulationTooLarge)
    );
    let small_policy = PackageMembershipPolicy::new(&[]).expect("empty policy is valid");
    assert_eq!(
        small_policy.compare(&too_many),
        Err(PackageMembershipError::ObservedPopulationTooLarge)
    );
}
