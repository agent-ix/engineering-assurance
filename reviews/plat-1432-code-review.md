---
id: SR-16405
title: "code-review and rust-review of the signature-verify command (engineering-assurance PR 210)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@c58cb1d; src/signature_verify.rs; src/signature_verify_host.rs; src/producer_execution.rs; src/process_host.rs; src/main.rs; src/lib.rs; Cargo.toml; tests/signature_verify.rs; tests/signature_verify_cli.rs; tests/package_boundary.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-036
    type: references
---

# Code review and Rust review of the signature-verify command (engineering-assurance PR 210)

## Summary

Ticket: PLAT-1432. Head c58cb1d, diff `origin/main...HEAD` (12 files, +1531/-3). The Rust-review lane is folded in, with Agent IX rust-style loaded.

Checked and clean:

- No cryptography or signature parsing. Only git and `ssh-keygen` verify, and no crate dependency was added (`tempfile` was already in `full`).
- Classification uses exit status, `%G?`/`%GF` and `cat-file -t` output only, never message text.
- The tag grading order matches FR-036.
- Revisions are refused before git starts, and `--end-of-options` is passed to `rev-parse`. Later git calls take only the resolved hex id.
- `gpg.ssh.program` is pinned. User and system config and repository-selecting variables are removed.
- The `run_configured_stdin` edit binds stdin to a regular `File`, so it cannot deadlock. `run_configured` still passes `None`, so it still uses `Stdio::null`.
- Each process gets a 60 s timeout and an 8 MiB bound.
- The `signature-verify` feature builds and tests on its own (`--no-default-features --features signature-verify`).
- Test keys are generated at test time and nothing is checked in.
- There are no pins and no ceremony.

Measured on git 2.34.1:

- `cargo test --features full --test signature_verify --test signature_verify_cli --test package_boundary`: 16/16 pass.
- The isolated-feature `signature_verify` test: 5/5 pass.

Defects: the classifier detects the signature format with stricter rules than git uses. Repository-local config still reaches git for every non-SSH format program. Together these let a tag signed by an unlisted key, and a crafted commit, come back `signed_valid`. Both were reproduced against the built binary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Tag format detection differs from git's, so `signed_valid` can come without allowed-signers acceptance (violates FR-036-CON-1). `split_tag_object` accepts only an exact header line. Git's `parse_signed_buffer` takes the last line that merely *starts with* any header. Reproduction: a tag signed by an unlisted SSH key, with a line `-----BEGIN PGP SIGNATURE----- ` (trailing space) and a PGP body appended. The classifier sees the SSH block, but git verifies the trailing PGP block with OpenPGP. `gpg.program`, `gpg.openpgp.program` and `gpg.x509.program` are not pinned and repository-local config is still read. With a local `gpg.program` reporting GOODSIG, the binary returns `signed_valid`. With the real `gpg`, any key in the user's keyring produces a good signature that gives `verify-tag` exit 0. Fix: match header lines by prefix exactly as git does, and pin every non-SSH program to one that cannot verify (for example `-c gpg.program=false -c gpg.openpgp.program=false -c gpg.x509.program=false`). Add CLI tests for both crafted cases. | src/signature_verify.rs:283-295; src/signature_verify_host.rs:258-266 |
| FND-002 | high | Commit format detection reads the first `gpgsig`/`gpgsig-sha256` header, but in a SHA-1 repository git verifies the `gpgsig` header. Reproduction: a commit with `gpgsig-sha256 <SSH block>` followed by `gpgsig <PGP block>`, in a repository whose local `gpg.program` reports GOODSIG and TRUST_ULTIMATE. The result is `signed_valid` with an OpenPGP fingerprint as `key_fingerprint`. With the real `gpg` this needs a fully trusted key. Fix: classify as `unsupported_signature_format` when any `gpgsig*` header is not SSH. Also apply the program pinning from FND-001. | src/signature_verify.rs:266-281; src/signature_verify_host.rs:148-158 |
| FND-003 | medium | Repository-local `log.showSignature=true` makes every commit `unverifiable`/`unrecognized_output`, including a valid one. `git log -1 --format=%G?%x00%GF` then prints the signature check text ahead of the format line. Reproduced with a validly signed commit. Fix: pass `--no-show-signature` to `git log`, or `-c log.showSignature=false`, and add a test. | src/signature_verify_host.rs:163 |
| FND-004 | low | `kind: tag` with a revision that resolves directly to a commit returns `unsigned` with reason `lightweight_tag`. That includes a branch name, `HEAD` or a commit id. Only a ref under `refs/tags/` is a lightweight tag, so the reason is wrong for any other revision. Fix: report `lightweight_tag` only when the revision names a `refs/tags/` ref (for example via `rev-parse --symbolic-full-name`), and `revision_unresolved` otherwise. | src/signature_verify_host.rs:180-190 |
| FND-005 | low | The final `assert_ne!(verdict.status, SignedValid)` in the TC-255 unit test is tautological: `Verdict::unverifiable` can never be `SignedValid`. Delete it. | src/signature_verify_host.rs:358-362 |

## Verdict

The design is sound: git and ssh-keygen are delegated to, classification uses fixed signals, the stdin edit is safe and the feature boundary is clean. It is not mergeable. FND-001 and FND-002 are reproduced FR-036-CON-1 violations, where `signed_valid` is reported without the allowed-signers file accepting an SSH key. FND-003 is a reproduced wrong result on a real configuration. FND-004 and FND-005 are low.
