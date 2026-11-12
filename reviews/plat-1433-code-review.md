---
id: SR-16407
title: "code-review and rust-review of signer mapping and Signed-off-meaning (engineering-assurance PR 211)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@cb6e620; src/signature_verify.rs; src/signature_verify_host.rs; tests/signature_verify.rs; tests/signature_verify_cli.rs; spec/tests.md; README.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-037
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-036
    type: references
---

# Code review and Rust review of signer mapping and Signed-off-meaning (engineering-assurance PR 211)

## Summary

Ticket: PLAT-1433. Head cb6e620, diff `origin/main...HEAD` (6 files, +742/-51). The Rust-review lane is folded in, with Agent IX rust-style loaded.

Checked and clean:

- `mapped` is never produced without a principal. A commit is `mapped` only for grade `G` with `verify-commit` accepted and a non-empty `%GS`. A tag is `mapped` only when `verify-tag` exits 0 and `find-principals` exits 0 with a non-empty first line. Every other accepted case is `unverifiable`/`unrecognized_output`, so FR-037-CON-1 holds.
- `signed_untrusted` always carries `unmapped` with no principal. `signed_bad`, `unsigned` and `unverifiable` carry no `signer`. No person is taken from author, committer, tagger, email or key comment.
- Trailers are parsed by `git interpret-trailers --parse`. The Rust side only splits that output at `:`, and `trailer.separators=:` is pinned on the command line.
- A tag's message is read from the payload `split_tag_object` returns, so the signature block is stripped. Missing, empty and repeated trailers behave as FR-037 says. `signed_bad` and `unsigned` keep their status and still report the trailer.
- The allowed-signers check rejects a missing file, a directory, more than 1 MiB and non-UTF-8 before git starts. Exit is 2 with `signature_verify_allowed_signers_invalid`. Both the metadata length and the bounded `take` read are checked.
- The request still requires an absolute `allowed_signers`, so a relative path or `~` cannot resolve differently in git (cwd = repository) and in `ssh-keygen` (cwd = temp dir). The path is the whole value of `-c gpg.ssh.allowedSignersFile=` and the operand of `-f`, so nothing can be injected.
- The PR #210 hardening is intact: prefix header matching, `gpg.program`, `gpg.openpgp.program` and `gpg.x509.program` pinned to `false`, `gpg.ssh.program=ssh-keygen`, and `--no-show-signature`.
- FR-036 fields are unchanged. `signer` and `signed_off_meaning` are additive optional members, and no new `reason` value was introduced.
- Fixtures generate ed25519 keys at test time and use `example.invalid` identities. No artifacts and no ceremony.

Measured on git 2.34.1 and OpenSSH 8.9p1:

- `cargo test --features full --test signature_verify --test signature_verify_cli`: 30/30 pass (8 unit, 22 CLI).
- `cargo clippy --features full --all-targets -D warnings`: clean.

Probes against the built binary agree with git:

- A multi-principal line `alice,bob`: commit `%GS` and tag `find-principals` both give `alice@example.invalid`.
- An expired line plus a valid line for the same key: commit and tag both give the valid principal.
- A quoted principal containing `"` and a space: commit and tag are both `signed_untrusted`, as git grades it.

Judgement calls:

- Find-principals failure maps to `unverifiable`. This is safe and consistent with CON-1, but unspecified (FND-004).
- `signed_off_meaning` is omitted when no message was read, and non-UTF-8 output is converted lossily. Both are acceptable but unspecified (FND-005).
- The `---` divider behaviour is wrong (FND-002).

Not raised: the allowed-signers file is read twice, by git and then by `find-principals`. An actor who can rewrite it between the two reads can already authorise any key, so this is not a defect.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `check_allowed_signers` calls `File::open` before checking the file type, so a FIFO as `allowed_signers` blocks the command forever. Reproduced: `mkfifo fifo` then a request naming it was still running when `timeout 10` killed it (exit 124). FR-037 requires a non-regular file to be refused (exit 2). Fix: open with `O_NONBLOCK` (rustix `OFlags::NONBLOCK`, already a dependency) or reject on `fs::metadata(path)` before opening. Better: write the validated bytes to a private temp file and give that path to git and `ssh-keygen`, which also makes them read exactly what was checked. Add a FIFO case to TC-266. | src/signature_verify_host.rs:144-160 |
| FND-002 | medium | `git interpret-trailers --parse` runs without `--no-divider`, so it treats a `---` line in a commit or tag message as a patch divider and ignores everything after it. Reproduced: message `subject / body / --- / more / Signed-off-meaning: approved` returns `presence: missing`, but `git log --format=%(trailers)` and `interpret-trailers --parse --no-divider` both print the trailer. A message is not a mailed patch. Fix: pass `--no-divider`, state in FR-037 that the message is parsed with no patch divider, and add a CLI case to TC-269. | src/signature_verify_host.rs:351-366 |
| FND-003 | high | `interpret-trailers` runs with cwd set to the repository under verification, so repository-local `trailer.<name>.key` config rewrites trailer tokens. Reproduced: with `git config trailer.fake.key Signed-off-meaning`, a commit whose only trailer is `fake: injected` returns `signed_off_meaning: {presence: present, values: [injected]}`. That violates FR-037-CON-2: a missing trailer is reported present. The PR #210 review (SR-16405 FND-001) already treated repository-local config as untrusted. `core.commentChar` can likewise hide a real trailer. Fix: run `interpret-trailers` outside the repository, with cwd set to the message temp dir and `GIT_CEILING_DIRECTORIES` set to it, keeping the existing env scrub. Then only command-line config applies; this was verified to work on git 2.34.1. Add a CLI test with a repo-local alias. | src/signature_verify_host.rs:351-366 |
| FND-004 | low | The code turns an accepted signature with no principal into `unverifiable`/`unrecognized_output`. That covers commit grade `G` with an empty `%GS`, and a tag where `verify-tag` exits 0 but `find-principals` exits nonzero or prints nothing. This is the right choice, since `signed_valid` without `mapped` would contradict FR-037 Outputs, but FR-037 never states it. Fix: add one sentence to FR-037 Outputs: an accepted signature for which no principal is reported is `unverifiable` with `unrecognized_output`. | src/signature_verify.rs:291-294; src/signature_verify.rs:321-331 |
| FND-005 | low | FR-037 Outputs does not say when `signed_off_meaning` is absent, or how a non-UTF-8 value is exposed. The code omits it when no message was read: a lightweight tag, an unresolved revision, or `interpret-trailers` failing or not starting. It replaces invalid UTF-8 with U+FFFD. Both are reasonable, but a consumer cannot tell them from the spec. Fix: state in FR-037 Outputs that `signed_off_meaning` is absent when the message could not be read, and that values are the trailer bytes decoded as UTF-8 with invalid sequences replaced. | src/signature_verify_host.rs:351-366 |

## Verdict

The design is right. Signer mapping is delegated entirely to git and `ssh-keygen`. `mapped` cannot appear without a principal, and trailers come from git's own parser. Tests are real, tag-bound and pass.

It is not mergeable:

- FND-003 is a reproduced FR-037-CON-2 violation under the threat model PR #210 already adopted.
- FND-001 is a reproduced hang on input FR-037 says must be refused.
- FND-002 is a reproduced wrong `missing` on an ordinary message.

FND-004 and FND-005 are one-line spec clarifications of the coder's correct choices.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The FND-003 fix sets `GIT_CEILING_DIRECTORIES` to the temp dir itself. Git ignores a ceiling that equals the working directory: only proper ancestors count. So git still walks up from the temp dir and loads the config of any repository that encloses `TMPDIR`. Reproduced on git 2.34.1 against the rebuilt binary: with `TMPDIR` inside a repository whose config has `trailer.fake.key Signed-off-meaning`, a commit whose only trailer is `fake: injected` returns `presence: present, values: [injected]`, an FR-037-CON-2 violation. Running git directly confirms it: ceiling = cwd gives `Signed-off-meaning: injected`, ceiling = parent gives `fake: injected`. The code comment's claim that git "cannot discover a repository above it" is false. Fix: set the ceiling to the temp dir's parent (`directory.parent()`), and add a TC-276 case that runs the CLI with `TMPDIR` inside a repository carrying the alias. | src/signature_verify_host.rs:451-455 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fix commit e86cc95 checks `std::fs::metadata(path)` (type and size) before `File::open`. Re-run against the rebuilt binary, a FIFO returns `signature_verify_allowed_signers_invalid` at exit 2 immediately, and so does `/dev/null`. A symlink to a regular file is still accepted. TC-266 now has a FIFO case behind a 30 s hang guard. Using stat before open instead of a validated temp copy is acceptable: swapping the path between stat and open needs write access to the file's directory, and that actor can already rewrite the allowed-signers file. |
| FND-002 | fixed | `interpret-trailers` now runs with `--parse --no-divider`. Re-run: a commit message with `---` before the trailer returns `present` / `approved`, and a tag message with `---` returns `present` / `tagmeaning`. FR-037 Behavior states the no-divider rule, and TC-269 has a `---` case. |
| FND-003 | fixed | `interpret-trailers` now runs with cwd set to the message temp dir, not the verified repository. Re-run: with repository-local `trailer.fake.key Signed-off-meaning`, `core.commentChar S` and `trailer.separators =`, the `fake: injected` commit returns `missing`, and the real `Signed-off-meaning: approved` commit still returns `present`. TC-276 covers the alias and the commentChar case. The remaining case, a repository enclosing `TMPDIR`, is FND-006. |
| FND-004 | fixed | FR-037 Outputs now states that an accepted signature without a principal is `unverifiable` with `unrecognized_output`, for commit `G` with an empty `%GS` and for a tag whose `find-principals` fails or prints nothing. This matches `classify_commit` and `classify_tag`. |
| FND-005 | fixed | FR-037 Outputs now states that `signed_off_meaning` is omitted when no message was read (a lightweight tag, an unresolved revision, or a failed trailer read), and that non-UTF-8 bytes are replaced lossily. This matches `Git::meaning`. |
