---
id: SR-16413
title: "code-review and rust-review of gitsign verification in signature-verify (engineering-assurance PR 215)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@aac6677; src/signature_verify.rs; src/signature_verify_host.rs; tests/signature_verify.rs; tests/signature_verify_gitsign.rs; spec/tests.md; Cargo.toml; README.md"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-040
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-036
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-037
    type: references
---

# Code review and Rust review of gitsign verification in signature-verify (engineering-assurance PR 215)

## Summary

Ticket: PLAT-1508. Head aac6677, diff `origin/main...HEAD` (7 files, +1453/-49). The Rust-review lane is folded in, with Agent IX rust-style loaded. gitsign is not installed. The stand-in's assumptions were checked against the sigstore/gitsign v0.18.0 source (cobra v1.10.2, cosign v3.1.3 options).

Confirmed against the gitsign v0.18.0 source:

- `main.go` exits 2 when `config.Get()` fails and 1 when any `rootCmd.Execute()` returns an error.
- `verify [commit]` uses `MaximumNArgs(1)` and `verify-tag <tag>` uses `ExactArgs(1)`. cobra intersperses flags, so putting the flags before the target is valid.
- The flags `--certificate-identity`, `--certificate-identity-regexp`, `--certificate-oidc-issuer` and `--certificate-oidc-issuer-regexp` come from cosign `CertVerifyOptions`. The exact flags compare literally, so regexp metacharacters in an exact value are inert.
- Both commands require the PEM type `SIGNED MESSAGE`.
- `verify` prefers `gpgsig` and falls back to `gpgsig-sha256`. `verify-tag` reads `refs/tags/<name>` and the in-body block.
- The wildcard probes set `CertIdentityRegexp`, so `defaultCertIdentity` never substitutes the committer email.
- gitsign reads its configuration by running `git config --get-regexp .*` in its cwd. It reads `SIGSTORE_*` and `GITSIGN_*` from the environment.

Checked and clean:

- No crypto, certificate or Sigstore code, and no new crate. `Cargo.lock` is untouched.
- The program name is the constant `gitsign`.
- The environment is built by inherit-and-remove: every inherited name except `PATH`, `HOME`, `GIT_CONFIG_NOSYSTEM` and `GIT_CONFIG_GLOBAL` is removed, and the latter two are set. This covers `GIT_CONFIG_COUNT`, `GIT_CONFIG_PARAMETERS`, `TUF_ROOT`, `SIGSTORE_*` and `GITSIGN_*` by construction. TC-290 measures it.
- The config check runs in the same environment and cwd as gitsign. It catches a local key, an `include.path` file, `config.worktree` and an uppercase `[GitSign]` section, since git lowercases the key (reproduced on git 2.34.1).
- Mixed CMS+SSH headers give `unsupported_signature_format`.
- Tag names are checked with `check-ref-format` and must resolve to the same object. A tag object id, a branch, an unknown ref and `^{tag}` are all rejected.
- `mapped` comes only from `Probe::Exact` exiting 0.
- Exit 2, other statuses, timeout and the output cap are all unverifiable. A signal maps to `gitsign_verification_failed`, which is sound.
- A principal starting with `-` is fine, because it is never an argument.
- The TC-248 and TC-249 edits follow the FR-036 text already on main.
- SSH behaviour is unchanged apart from the intended additive `signer.source` and `signature_kind`.

Rights-check workaround: `concat!("https:", "//...")` is the repo's established idiom, and `src/content_rights.rs` uses it itself. The domains are reserved `.test` names, so this is acceptable.

The live test is honestly opt-in. It is `#[ignore]` with a reason, and under `--ignored` it panics rather than passing when an `FR_040_LIVE_*` variable is missing.

Measured on git 2.34.1:

- `signature_verify`: 11/11 pass.
- `signature_verify_cli`: 24/24 pass.
- `signature_verify_gitsign`: 14 pass, 1 ignored.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A tag name that begins with `-` reaches gitsign as an option. `check-ref-format` accepts `refs/tags/--help` and `refs/tags/-h`, and `is_plain_revision` checks only the leading byte of the whole revision. So request revision `refs/tags/--help` yields argv `verify-tag --certificate-identity X --certificate-oidc-issuer Y --help`. In cobra v1.10.2, `execute` returns `flag.ErrHelp` before `ValidateArgs`, and `ExecuteC` turns ErrHelp into nil, so gitsign exits 0. Reproduced against the built binary with a stand-in that models this: a garbage CMS tag at `refs/tags/--help` returned `signed_valid`, `mapped`, principal `Alice`. Fix: reject a tag name beginning with `-` as `revision_unresolved`, also put `--` before the target (pflag honours it), and add a `refs/tags/--help` case to TC-291 plus an argv assertion for `--` to TC-279. | src/signature_verify_host.rs:806-813; src/signature_verify_host.rs:873-882 |
| FND-002 | high | The `^gitsign\.` refusal can be bypassed by a newline embedded in another key's value. gitsign runs `git config --get-regexp .*`, splits stdout on newlines, takes the first space-separated token as the key and matches it with EqualFold. A repository config `[foo] bar = "x\ngitsign.fulcioRoot /tmp/evil"` makes the scoped check exit 1 (proceed). The unscoped output, though, has a line `gitsign.fulcioRoot /tmp/evil`, which gitsign applies as its Fulcio root (`fulcioroots.NewFromConfig`); `gitsign.rekor` works the same way. Reproduced on git 2.34.1. So the repository can choose the roots that verify it, which breaks FR-040-CON-1's intent. Fix: run exactly gitsign's `git config --get-regexp .*` in the same environment, and refuse when any newline-separated line's first token case-insensitively starts with `gitsign.`. Amend the FR-040 Behavior bullet, and add the injection case to TC-289. | src/signature_verify_host.rs:751-777 |
| FND-003 | medium | A relative `PATH` entry resolves `gitsign`, and the config-check `git`, inside the repository under verification, because the child's cwd is the repository. Reproduced: with `PATH=hook:/usr/bin` and the caller's cwd at `/`, an executable `hook/gitsign` in the repo ran and the verdict was `signed_valid`. FR-040 says the program name or path is never taken from the repository. Go's LookPath refuses this for gitsign's own `git`, but Rust's Command does not. Fix: resolve `gitsign` to an absolute path using only absolute PATH entries before spawning (not found gives `gitsign_not_started`), pass a PATH with relative entries dropped, and add a test with a relative PATH entry. | src/signature_verify_host.rs:821-828; src/signature_verify_host.rs:763-770 |
| FND-004 | low | `Run::Exit(output.status.code().unwrap_or(-1))` encodes a signal as the sentinel exit status -1, which is indistinguishable from a real status. The mapping to `gitsign_verification_failed` is fine, but the observation should not lie. Fix: add `Run::Signaled` and map it explicitly in `probe_gitsign`, and change the TC-293 `Exit(-1)` case to it. | src/signature_verify_host.rs:686-688 |
| FND-005 | low | `repository_sets_gitsign_key` and `gitsign_run` duplicate the `tool_environment()` borrow-conversion boilerplate. `gitsign_run` also maps an impossible missing identity to `Run::NotStarted`, a valid-looking value for an unreachable state. Fix: add one helper `run_tool(program, &[&OsStr]) -> Result<CompletedProcess, ProcessError>`, and pass the identity into the `classify_gitsign` closure so there is no `Option` re-check. | src/signature_verify_host.rs:751-829 |
| FND-006 | low | The new `spec/tests.md` rows TC-278 to TC-296 carry hand-written status claims ("✅ ... passing", "⏸ ... ignored"). This is status tracking that the computed `quoin matrix` supersedes, and it goes stale silently. Fix: drop the status claims from these rows (and repo-wide in one sweep), leaving the computed matrix as the only coverage record. | spec/tests.md:509-527 |

## Verdict

The structure is right. The classifier is pure and reads exit statuses only. It drives at most four probes, and `mapped` is reachable only from the exact probe exiting 0. The environment is exact by construction, and the SSH path is untouched. The tests are real, tag-bound and pass on git 2.34.1.

It is not mergeable:

- FND-001 is a reproduced forged `signed_valid` through a `--help` tag name.
- FND-002 is a reproduced bypass of the repository-config refusal, which lets the verified repository pick its own Fulcio root or Rekor.
- FND-003 is a reproduced program hijack, but only when the caller's PATH has a relative entry.

FND-004 to FND-006 are small cleanups.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | The FND-002 fix compares the first token with ASCII case-insensitivity (`eq_ignore_ascii_case`), but gitsign matches keys with Go `strings.EqualFold`, which uses Unicode simple folding and maps U+017F (long s) to `s`. A value `[foo] bar = "x\ngitſign.fulcioRoot /tmp/evil"` makes git print a line `gitſign.fulcioRoot /tmp/evil`. `config_sets_gitsign_key` returns false and the stand-in runs, so the verdict is `signed_valid`, yet gitsign applies that line as its Fulcio root (Go 1.x confirms `EqualFold("gitſign.fulcioRoot", "gitsign.fulcioRoot") == true`). Reproduced on git 2.34.1 against the clean-rebuilt head 4da9f97. Fix: before the ASCII prefix check, map U+017F to `s` and U+212A (Kelvin sign) to `k`, the only non-ASCII members of any ASCII letter's simple-fold orbit. Alternatively, refuse any line whose first token has a non-ASCII byte before its first `.`. Change FR-040's "ASCII case-insensitive" to Go EqualFold semantics, and add a `gitſign.` case to TC-298 and TC-289. | src/signature_verify.rs:634-646 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `tag_name` now returns `revision_unresolved` for a name beginning with `-`, and `gitsign_run` puts `--` before the target. Re-tested against the rebuilt binary: a CMS tag at `refs/tags/--help` gives `unverifiable`/`revision_unresolved` and the stand-in is never started, and a commit run records argv `verify --certificate-identity … --certificate-oidc-issuer … -- <id>`. TC-291 and TC-279 now assert both. |
| FND-002 | fixed | The check now runs gitsign's own `git config --get-regexp .*` through `run_tool` (same environment and cwd) and reads the output line by line for a first token starting `gitsign.`. Re-tested: the `\ngitsign.fulcioRoot` injection gives `gitsign_repository_config` without starting the stand-in. TC-289 has the injected case and TC-298 the reader. FR-040 Behavior and AC-17 are amended. The remaining Unicode-fold bypass is FND-007. |
| FND-003 | fixed | `resolve_program` finds `git`, `ssh-keygen` and `gitsign` only in absolute PATH entries, runs each by absolute path, and passes on a PATH with relative entries dropped. That covers every spawn site (`run`, `run_tool`, both `ssh-keygen` calls), and git's own `ssh-keygen` lookup through the passed PATH. Re-tested with hook `git`/`gitsign`/`ssh-keygen` reachable only through a relative entry: SSH commit and tag and a gitsign commit all used the absolute programs, no hook ran, only-relative and unset PATH give `git_not_started`, and a missing absolute `ssh-keygen` gives `ssh_keygen_not_started`. With resolution reverted in a scratch copy, TC-297 and TC-299 both FAIL, so they can fail. The PATH-unset/only-relative behaviour change is stated in FR-036 and FR-040 and TC-299 covers it. The legitimate SSH path still verifies. |
| FND-004 | fixed | `observed` now maps a missing exit code to `Run::Signaled`, which `probe_gitsign` maps to `gitsign_verification_failed`. TC-295 builds a signal-9 `ExitStatus` and TC-293 uses `Run::Signaled`. |
| FND-005 | fixed | `run_tool` replaces the duplicated environment boilerplate. `gitsign_verdict` returns early without an identity and passes it into the closure, so `gitsign_run` takes `&GitsignIdentity` and no impossible `NotStarted` remains. The kept set is `PATH` and `HOME`, and the set names are excluded from removal, so TC-290 still records exactly four variables. |
| FND-006 | fixed | The new TC-278..TC-299 rows no longer claim "✅ passing" or "⏸ ignored". They carry the template's `🚧` placeholder in the schema-required Status column. Removing the column repo-wide is a separate change for the lead. |
