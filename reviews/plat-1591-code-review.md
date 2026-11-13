---
id: SR-16417
title: "code-review and rust-review of the approval-tag command (engineering-assurance PR 219)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@a40b2fdb; src/approval_tag.rs; src/approval_tag_host.rs; src/main.rs; src/lib.rs; src/signature_verify.rs; src/signature_verify_host.rs; Cargo.toml; README.md; tests/approval_tag.rs; tests/approval_tag_cli.rs; tests/package_boundary.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-042
    type: references
---

# Code review and Rust review of the approval-tag command (engineering-assurance PR 219)

## Summary

Ticket: PLAT-1591. Frozen head a40b2fdb, diff `origin/main...HEAD` (14 files, +1602/-6). Rust-review lane folded in (Agent IX rust-style loaded).

Measured on git 2.34.1 / OpenSSH 8.9p1: `cargo test --features full --test approval_tag --test approval_tag_cli --test approval_verify --test package_boundary` 21/21 pass; `cargo check --lib --no-default-features --features approval-tag` clean; `cargo clippy --all-targets --features full -D warnings` clean; feature_matrix tc_196 and source_audit tc_101/tc_127 pass. source_audit tc_117 fails on `tests/signature_verify_cli.rs` tc_276/tc_277 (CON-only traces), a file this PR does not touch: pre-existing on main, not this PR.

Round trip rerun: SSH with real ed25519 keys and x509 with the stand-in signer plus stand-in gitsign both verify `approved` under `approval-verify`. Trailer-injection probes against the built binary (meaning or record_id holding `Approved-Record-Id: x`, `Signed-off-meaning: x`, U+2028, U+3000, NBSP, `#`, emoji, 256-byte values; NEL refused) all read back exactly and cannot add or alter a trailer. `git tag -s --cleanup=verbatim -F <file> -- <name> <resolved id>` is correctly built: the message file lives in a 0700 tempdir removed on drop, `-f`/`-u`/`-c` are never passed, the target is the resolved commit id, the existing tag is never replaced. The record discriminators (schema_version 2, record_type change_assurance, record_id, review_workflow.signed_approval object) match quoin origin/main FR-063 and `change-assurance-record-v2.schema.json`, and a canonical stored record file is accepted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | On git 2.34.1 a failed SSH signature still writes an unsigned annotated tag and exits 0, and the command reports `created`. Reproduced: `user.signingkey` pointing at a missing file, and a passphrase-protected key with no agent (also under a real pty). `approval-verify` then reports `unsigned`, and the tag name is taken (never replaced). Fix: after `git tag -s`, read the tag object and require a signature block, using FR-036's pure tag split already in the library; if it is missing, delete only that ref with `git update-ref -d refs/tags/<name> <tag_object>` and return `approval_tag_create_failed`. Add an SSH-failure case to TC-350. | src/approval_tag_host.rs:180-187; src/approval_tag_host.rs:363-382 |
| FND-002 | medium | Tag names that pass `check-ref-format` but that `approval-verify` resolves to something else are created and never verify. Measured: tags `HEAD` and `@` are `created`, and `approval-verify` returns `unverifiable` with `revision_unresolved`. `heads/<branch>` and `tags/<x>` fall to the same rev-parse precedence. Fix: before creating, refuse a name that `git rev-parse --verify --quiet --end-of-options <name>` already resolves, or after creating require that `rev-parse --symbolic-full-name <name>` is `refs/tags/<name>`. Add HEAD and @ to TC-347. | src/approval_tag_host.rs:158-176 |
| FND-003 | medium | Interactive signing cannot work, despite the 120 s passphrase allowance. Git runs in its own process group with stdin on /dev/null and stderr captured. Under a pty, a passphrase key without an agent showed no prompt and produced FND-001's unsigned tag at once. A gitsign URL or device prompt on stderr is swallowed the same way. Fix: either state that the signer must be non-interactive (ssh-agent, or gitsign with a browser) and drop the passphrase claim, or run `git tag -s` with the caller's terminal. | src/approval_tag_host.rs:41-43; src/approval_tag_host.rs:256-279 |
| FND-004 | low | The diagnostics mislead. A timeout during `git tag -s` is reported as `approval_tag_git_unavailable` ("could not run git from an absolute PATH entry"). A tempdir or message-file failure is reported as `approval_tag_git_failed` ("could not read git's answer"). A kill after the ref update would leave a tag behind a refusal. Fix: map a timeout or failure during the create to `approval_tag_create_failed` (or a timeout code) and re-check the ref, and give local I/O its own code. | src/approval_tag_host.rs:273-279; src/approval_tag_host.rs:364-366 |
| FND-005 | low | `gpg.format=x509` is accepted without checking `gpg.x509.program`. Unset, git signs with gpgsm, a CMS tag that FR-040 (gitsign) never verifies. That contradicts the module doc ("a tag that FR-036 and FR-040 can verify"). Fix: under x509, require `gpg.x509.program` to be set and name it in the refusal, or document that x509 is not checked further. | src/approval_tag.rs:102-112 |
| FND-006 | low | The record file is checked with `metadata` and then opened by path. A swap to a FIFO in between blocks `read_to_end` forever, because the file read has no timeout. Fix: open with `O_NONBLOCK`, then check `file.metadata()` on the handle before reading. | src/approval_tag_host.rs:213-222 |

## Verdict

The structure is sound: a pure composer and precondition module, a thin host adapter, the right feature wiring, and no new dependency, crypto or digest code. The `git tag` invocation and the trailer handling hold up under adversarial probes. Using the user's full git config is acceptable for a command a person runs in their own repository. Repository-local `gpg.*.program`, `defaultKeyCommand` and the `reference-transaction` hook run exactly as they would for `git tag -s` typed by hand, and the absolute-PATH cut keeps relative entries out. Not mergeable: FND-001 (unsigned tag reported as created on the required git version) must be fixed, and FND-002 and FND-003 should be fixed in the same round.
