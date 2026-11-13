---
id: SR-16415
title: "code-review and rust-review of the approval-verify command (engineering-assurance PR 217)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@1993257; Cargo.toml; README.md; spec/tests.md; src/approval_verify.rs; src/approval_verify_host.rs; src/lib.rs; src/main.rs; src/signature_verify.rs; src/signature_verify_host.rs; tests/approval_verify.rs; tests/approval_verify_cli.rs; tests/approval_verify_gitsign.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-041
    type: references
---

# Code review and Rust review of the approval-verify command (engineering-assurance PR 217)

## Summary

Ticket: PLAT-1510. Frozen head 1993257, diff `origin/main...HEAD` (12 files, +1699/-30). Rust-review lane folded in (Agent IX rust-style loaded).

Status ordering in `decide` follows FR-041 exactly: rule 1 maps every non-valid FR-036 status before any binding is read, `trailer_unreadable` is decided before the binding, the binding before the meaning, and `approved` needs `signed_valid`, both bindings `match`, a present meaning, a mapped principal and a signature kind. The impossible valid-without-principal state falls to `unverifiable`, with no panic. `bind` compares exact `String` equality and counts every case variant of the key as an occurrence.

The reuse of the signature-verify adapter is sound. `tag()` now delegates to `tag_checked()`, and the meaning and the bindings come from the same `git interpret-trailers --parse --no-divider` output of one git run. That run has the same hardening as before: absolute-PATH program resolution, pinned `gpg.*` programs, `trailer.separators=:`, `GIT_CONFIG_NOSYSTEM`/`GIT_CONFIG_GLOBAL=/dev/null`, and an empty isolated directory with a ceiling, so the verified repository's config cannot change how trailers are read. The gitsign config refusal and the `is_plain_revision` and record-value bounds also carry over. The signature-verify suites pass unchanged (signature_verify 12, signature_verify_cli 24, signature_verify_gitsign 16 + 1 ignored), and TC-317 shows that the embedded `signature` equals the `signature-verify` output for the same tag.

Forgery attempts measured on git 2.34.1 with the built binary:
- Trailers appended after the SSH `END` line verify as `signed_valid`, but the split puts them in the signature part, so they do not bind.
- A second fake signature block carrying trailers makes the tag `signed_bad`.
- Trailers in the tagged commit's message do not bind (TC-323).
- A continuation line is unfolded by git into the value, so it is a mismatch.
- CRLF is trimmed by git as trailer parsing.
- A trailing NBSP is kept, so it is a mismatch.
- A case-variant duplicate key is `repeated`.
- A `---` line with `--no-divider` turns the paragraph into a non-trailer block, so the binding is missing.
- An over-long trailer value is a mismatch.

Two inputs do forge `approved` (FND-001).

Measured: focused `cargo test --features full` over approval_verify (6), approval_verify_cli (12), approval_verify_gitsign (2), signature_verify*, package_boundary all pass. `cargo test --no-default-features --features approval-verify --test approval_verify` passes. `cargo check --lib --no-default-features --features approval-verify` and `cargo clippy --features full --all-targets -D warnings` are clean. `make package-audit` is accepted. No new dependencies, and Cargo.lock is unchanged. Fixtures are generated at test time with fictional `example.test` identities, and nothing generated is checked in.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Binding is not byte-exact, so `approved` is forged for bytes the signer did not sign as the value (violates FR-041-CON-1 and the exact-byte rule). Reproduced: (a) a signed tag carrying `Approved-Record-Digest: d<NUL>EXTRA-SIGNED-BYTES` is `approved` for request digest `d`, because git interpret-trailers stops at the NUL; (b) a signed tag carrying `Approved-Record-Id: <0xFF>` is `approved` for request id U+FFFD, because `parsed_trailers` decodes with `String::from_utf8_lossy` (every invalid sequence collapses to U+FFFD). Fix: in `parsed_trailers`/`tag_checked`, return `None` (so `trailer_unreadable`) when the tag message holds a NUL byte or git's output is not valid UTF-8 (`String::from_utf8`, not lossy). Add a CLI test for both tags. State this in FR-041-AC-15 ("cannot be read" includes NUL and non-UTF-8). | src/signature_verify_host.rs:632-647; src/signature_verify_host.rs:483-494 |
| FND-002 | low | Two readers for one trailer block. `trailer_values` folds continuation lines and trims ASCII whitespace; `signed_off_meaning` reads line by line and trims Unicode whitespace. On real `--parse` output (git unfolds and normalizes `token: value`) they agree on tokens. Measured on git 2.34.1: the folding branch is unreachable, and TC-314's folded-value assertions test input git never emits. They differ only in that a meaning value ending in NBSP is trimmed while a binding value is not. Fix: make `signed_off_meaning` return `trailer_values(parsed, MEANING_TOKEN)`, delete the folding branch, and replace TC-314's folded-input case with one run through real git output. | src/signature_verify.rs:744-790; tests/approval_verify.rs:199-213 |
| FND-003 | low | Fourth error code `approval_verify_result_encoding_failed` is not in FR-041 Outputs, which lists three codes. The same gap exists for `signature_verify_result_encoding_failed` in FR-036. Fix: add the code to the FR-041 Outputs list (and the signature-verify one to FR-036), as FR-029 does for producer-execute. Do not collapse it into a panic. | src/approval_verify_host.rs:80-93; spec/functional/FR-041-signed-tag-approval-of-a-change-assurance-record.md:96-100 |

## Verdict

The decision logic, the status ordering, the adapter reuse and the hardening are correct, and the signature-verify behaviour is unchanged. Not mergeable: FND-001 lets `approved` be reported for a record id or digest that is not byte-equal to the signed trailer value. FND-002 and FND-003 are small and should be fixed in the same round.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit 6600bdf: `parsed_trailers` returns `None` for a message holding a NUL byte and decodes git's output with `String::from_utf8` (no lossy decoding), so both cases report `trailer_unreadable`; FR-041-AC-15 and FR-037 updated; TC-330 covers both. Re-run against the rebuilt binary: NUL-truncated id, 0xFF, an encoded lone surrogate (ED A0 80), an overlong (C0 AF) and a NUL inside a trailer key are all `unverifiable`/`trailer_unreadable`. Same-class probes found no bypass: a mid-value CR and a CR-injected second key give `mismatch`; CRLF trims as git trims (ASCII rule); a Cyrillic look-alike key breaks the trailer block (`missing`); a 3 MB padding line and a NUL after the SSH END line both leave the signed trailers read exactly. |
| FND-002 | fixed | Fix commit 6600bdf: `signed_off_meaning` now returns `trailer_values(parsed, MEANING_TOKEN)`; the folding branch is deleted; FR-037 states the ASCII-only trim; TC-314 uses real `--parse` output and TC-319 adds a git-folded value (`mismatch`). The signature-verify suites pass unchanged (12/24/16+1 ignored). |
| FND-003 | fixed | Fix commit 6600bdf adds `approval_verify_result_encoding_failed` to the FR-041 Outputs codes and `signature_verify_result_encoding_failed` to FR-036. |
