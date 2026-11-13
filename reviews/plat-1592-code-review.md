---
id: SR-16417
title: "code-review and rust-review of approval-verify signing time (engineering-assurance PR 218)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@fbb6e2f; README.md; spec/spec.md; spec/tests.md; src/approval_verify.rs; src/approval_verify_host.rs; src/signature_verify.rs; src/signature_verify_host.rs; tests/approval_verify.rs; tests/approval_verify_cli.rs; tests/approval_verify_gitsign.rs; Cargo.toml"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-041
    type: references
---

# Code review and Rust review of approval-verify signing time (engineering-assurance PR 218)

## Summary

Ticket: PLAT-1592. Frozen head fbb6e2f, diff `origin/main...HEAD` (11 files, +544/-19). The Rust-review lane is folded in (Agent IX rust-style loaded).

The time source is sound. `tagger_signed_at` reads the `payload` that `split_tag_object` returns. That split takes the last line-anchored signature marker, as git's `parse_signed_buffer` does, so the bytes read are exactly the bytes that `git verify-tag` checks for SSH. Only the header block counts (up to the first blank line), and it must hold exactly one `tagger ` line. A tagger-shaped line in the message, and any text after the signature, is never read.

For gitsign, v0.18.0 `verify-tag` verifies the raw object through `pkg/git.SplitTag`: it takes the last in-body `-----BEGIN ` block, as git-core does, and strips only `gpgsig`/`gpgsig-sha256` headers and their continuation lines. Those stripped lines can never begin with `tagger `, so EA cannot read a tagger line that gitsign excluded from the signed payload. An attacker who adds a second tagger header gets `signed_at_missing`, not a forged time. The epoch is at most 12 digits and capped at 253402300799, so the `u64` civil-date arithmetic cannot overflow or underflow. The zone never moves the instant, and malformed zones and epochs fail closed. `decide` takes the time only inside `decide_valid` for the `approved` arm. TC-310 shows the status and reason are unchanged with a time supplied, and non-approved statuses carry no approval members.

The hardening carries over unchanged: there is no new git run, no gitsign run, no change to the PATH resolution, and no message-text classification. There is no new dependency, and Cargo.lock is unchanged. The `signature-verify` output is unchanged (TC-334, and the signature_verify_cli 24 and signature_verify_gitsign 2 tests pass).

Measured with locked-build.sh, debug=0. `cargo test --features full` passed: approval_verify 9, approval_verify_cli 15, approval_verify_gitsign 4, signature_verify 12, signature_verify_cli 24, signature_verify_gitsign 2, and package_boundary 16 (1 ignored). `cargo clippy --features full --all-targets -D warnings` and `cargo fmt --check` are clean. `cargo test --no-default-features --features approval-verify --test approval_verify` fails to compile (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Test-case id collision with open PR #219 (PLAT-1591, FR-042). That PR adds TC-331..TC-343 to spec/tests.md and tags `tests/approval_tag*.rs` with `TC-331`..`TC-339`. This PR adds TC-331..TC-337 with different meanings. Whichever PR lands second either produces duplicate TC rows and trace tags, or conflicts at the same tail of spec/tests.md. spec/spec.md (the FR-041 index line versus a new FR-042 line) and the README command table (an adjacent line) also conflict textually. Fix: renumber the PR that lands second, for example this PR's rows and `#[trace]` tags to TC-344..TC-350 if #219 lands first, and rebase. | spec/tests.md:562-568; tests/approval_verify.rs:246,295,322; tests/approval_verify_cli.rs:780,827; tests/approval_verify_gitsign.rs:261,284 |
| FND-002 | medium | Cargo feature rule broken: `tests/approval_verify.rs` now imports `serde_json`, but its target declares `required-features = ["approval-verify"]`, and that feature (`["signature-verify", "dep:serde"]`) does not enable the optional `serde_json`. `cargo test --no-default-features --features approval-verify --test approval_verify` fails with E0432/E0433. It passed on main (as measured in SR-16415). `make rust-features` checks only `--lib`, so CI does not catch this. Fix: add `serde_json` to `[dev-dependencies]` (the same crate already in Cargo.lock, so no new dependency), or assert TC-333 without `serde_json`. | tests/approval_verify.rs:15,334; Cargo.toml:194-197,379 |
| FND-003 | low | The signing time crosses the public API as an unvalidated `Option<String>`. `decide(.., tagger_signed_at: Option<String>)` and `SigningTime::from_tagger` accept any text and emit it as `signed_at`, so the `YYYY-MM-DDTHH:MM:SSZ` form and CON-3's "read from a well-formed tagger line" hold only because the one caller passes `tagger_signed_at`'s output (rust-style: parse, don't validate; make invalid states unrepresentable). Fix: have `tagger_signed_at` return a `SignedAt` newtype with a private constructor and `Serialize` as the string, and take `Option<SignedAt>` in `decide`. | src/approval_verify.rs:154-165,217-221; src/signature_verify.rs:757 |
| FND-004 | low | The README command row says only "the signing time (`signed_at` in UTC with its `signed_at_source`...)". It does not say the time is the signer's own claim and not attested or checked for plausibility, although FR-041 requires the result to name that source so it is not read as attested. The PR's "Fields quoin must add" list also omits that qualifier for the quoin rendering. Fix: add "the signer's claimed tagger date, not an attested time" to the README row, and carry the same instruction in the quoin-half field list. | README.md:294 |

## Verdict

The time is read from the same bytes the signature covers. No tagger forgery was found for SSH or gitsign v0.18.0, there is no overflow, and the time is never defaulted. Status and reason are independent of the time, and `signature-verify` is unchanged. Not mergeable as is: FND-001 (the id collision with PR #219) must be resolved by renumbering in whichever PR lands second, and FND-002 breaks the declared feature set of a test target. FND-003 and FND-004 are small and belong in the same fix round.
