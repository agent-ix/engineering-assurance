---
id: SR-16299
title: "code-review and rust-review of owned npm platform release exemption (engineering-assurance PR 187)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@1b7ff8c3; src/language_boundary.rs; src/language_boundary_host.rs; tests/language_boundary_cli.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/NFR-005
    type: references
---

# Code review and Rust review of the owned npm platform release exemption (engineering-assurance PR 187)

## Summary

Ticket: IR-1008. Frozen head 1b7ff8c3 on #181 merge 6e8c0d97. The diff from 6e8c0d97 touches exactly four paths (NFR-005, `src/language_boundary.rs`, `src/language_boundary_host.rs`, `tests/language_boundary_cli.rs`), so the ten paths #181 brought in are kept unchanged. The PR does not edit README, spec/tests.md, schemas, manifests, lockfiles or workflows. Rust-review lane folded in.

`classify_delta` keeps the old head-minus-base set difference. It removes only `npm-dependency` findings whose evidence is `optionalDependencies:<family member>:<new version>` at the same manifest path. And it removes them only when `owned_release_versions` proves all of these: the base owns the package, the family is non-empty, every family member moves in lockstep to the new launcher version, and the head manifest equals the base manifest once those versions are restored. The exemption is narrow, as the PR claims. A foreign dependency, an own-looking new dependency, a rename, a third-party bump, an unsynchronized family, an unchanged wrapper version and a changed launcher are all still refused. Measured: 38/38 `language_boundary_cli` tests pass. `cargo fmt --check` is clean. `cargo clippy --features full --lib --bins --test language_boundary_cli -D warnings` is clean (Rust 1.98.1). The locally built guard returns `accepted` with zero findings on the real quire-cli #136 pair (base 374d5085, head 76b29c2b), whose only manifest change is the four `@agent-ix/quire-cli-*` optional dependencies plus `version` going from 0.36.2 to 0.37.0. I found no correctness defect, no panic surface (the `after["version"]` index-assign runs only after `after.get("version")` has proven `after` is an object), no unsafe code and no integer conversion.

The defects are test-strength defects. A reviewer mutation probe disabled the whole base-ownership predicate (lines 121-144). A second probe disabled the mode check, the SemVer-validity check and the family-version-membership check together. Both platform-release tests still passed each time, and every adverse scenario is refused by the final `after != before` equality alone. None of the base-side authority conditions that make the exemption narrow is pinned by a test.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | No test pins the base-authority ownership predicate (base `name == @agent-ix/<repo>`, `repository.type == git`, exact `git+https` agent-ix URL, `private` absent or false, non-empty `bin` map, default registry). Replacing the whole predicate with `false` leaves both platform-release tests green. The only owner test (`spoof-owner`) changes the head URL, and the final equality check catches that by itself. Add adverse cases where the base itself lacks ownership (foreign name, foreign URL, `private: true`, empty or string `bin`, non-default `publishConfig.registry`) while base and head differ only by a lockstep bump. | src/language_boundary.rs:121-144; tests/language_boundary_cli.rs:1080-1156 | correct-requirement-no-evidence |
| FND-002 | medium | No test pins the regular-file mode check, the SemVer-validity check or the family-membership-by-base-version check. Neutralising all three together leaves both platform-release tests green. Without the line-168 check, a base optional dependency `@agent-ix/<repo>-x` at an unrelated version that jumps to the new launcher version would be exempted. That widens the exemption beyond the matching released version. Add adverse cases: a member whose base version differs from the base launcher version; a non-SemVer or partial version (for example `1.2` or `^1.2.4`); a manifest whose mode is not `100644`. | src/language_boundary.rs:107; src/language_boundary.rs:151; src/language_boundary.rs:168 | correct-requirement-no-evidence |

## Verdict

The implementation is correct and narrow. Behaviour on the real quire-cli #136 pair is repaired, adverse cases still fail, and all 38 guard tests pass with fmt and clippy clean. It is not mergeable as is: FND-001 and FND-002 (medium) leave the conditions that make the exemption narrow unprotected against regression. Both close with focused adverse test scenarios only, and need no production change.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Head 56bed14d adds `platform_release_requires_ownership_in_the_safe_base` (seven base-side scenarios: foreign name, foreign URL, foreign repository type, private, empty bin, string bin, non-default registry). The reviewer re-ran the mutation probes: disabling the whole predicate, and separately the name, repository-type, URL, private, bin and registry checks, each fails that test. |
| FND-002 | fixed | Head 56bed14d adds `platform_release_requires_exact_semver_and_matching_family_versions` (v-prefix, caret, leading zero, numeric pre-release with a leading zero, empty build, member not at the base launcher version), `platform_release_does_not_exempt_executable_manifest_modes` (mode 100755), and an unchanged `@agent-ix/toolbox-unrelated` 4.5.6 member in the accepting test. The mode, SemVer and line-168 family-membership mutations each fail at least one test. Production source is unchanged from 1b7ff8c3. |
