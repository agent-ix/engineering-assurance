---
id: SR-16420
title: "code-review and rust-review of the approval-trail command (engineering-assurance PR 220)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@be9a4c9; src/approval_trail.rs; src/approval_trail_host.rs; src/approval_verify_host.rs; src/signature_verify_host.rs; src/main.rs; src/lib.rs; Cargo.toml; README.md; tests/approval_trail.rs; tests/approval_trail_cli.rs; tests/approval_trail_gitsign.rs; tests/package_boundary.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-043
    type: references
---

# Code review and Rust review of the approval-trail command (engineering-assurance PR 220)

## Summary

Ticket: PLAT-1594. Frozen head be9a4c9, diff `origin/main...HEAD` (15 files, +1588/-29). The Rust-review lane is folded in, with Agent IX rust-style loaded.

Measured on git 2.34.1: `cargo test --features full` over approval_trail, approval_trail_cli, approval_trail_gitsign, approval_verify, approval_verify_cli and package_boundary passed 39 of 39. `cargo clippy --all-features --all-targets -D warnings` is clean, `cargo fmt --check` is clean, and `cargo check --lib --no-default-features --features approval-trail` is clean. The branch contains origin/main, and there is no FR or TC id collision with PR 219 (FR-042, TC-338 to TC-350). There is no new dependency.

The probes below ran against the built binary.
- **Cost.** 300 SSH-signed annotated tags that do not name the record took 7.5 s, about 25 ms per tag and about 7 git or ssh-keygen processes each. At 10000 tags that is about 250 s, which is close to the 300 s limit.
- **Giant message.** A 9 MB tag naming the record with a `revoked` meaning is omitted and only counted in `skipped`, while `approval-verify` gives `unverifiable` / `output_exceeded` for the same tag.
- **Non-UTF-8 name.** A tag with a Latin-1 name is omitted and counted.
- **Bidi override.** A U+202E override in a tag name is printed raw in the table.

The reuse of FR-041 holds: `assess` was extracted verbatim, the approval-verify tests pass unchanged, TC-355 compares each entry with the approval-verify result, and the enumeration format, the full `refs/tags/<name>` revision, `--end-of-options`, the absolute-PATH resolution and the pinned gpg programs all hold.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Any annotated tag for which `check_tag` returns no trailers is dropped from `entries` and reduced to an anonymous `skipped` count, and `truncated` stays false. That covers a NUL byte or non-UTF-8 output, and also a mechanics failure: a git timeout, output over 8 MiB (measured: a 9 MB revocation tag that approval-verify reports `unverifiable`/`output_exceeded`), a rev-parse or cat-file failure, or a ref moved after listing. A real approval or revocation can therefore vanish from the trail with `truncated: false`, which breaks CON-3 and the ticket's "nothing is hidden". The count is also repository-wide, so it says nothing about this record. Fix: never drop silently. Report each such tag in the result, as a `skipped` array of `{tag, reason}` with the FR-041 unverifiable reason (non-UTF-8 names escaped or as bytes), or list it as an entry whose binding is unknown. At minimum, a mechanics failure (anything other than a NUL or a non-UTF-8 message) must make the trail `truncated` or listed, never merely counted. | src/approval_trail_host.rs:168-177; src/signature_verify_host.rs:287-290 |
| FND-002 | medium | Every annotated tag is fully signature-verified (verify-tag, then ssh-keygen find-principals or the gitsign probes) before its `Approved-Record-Id` is looked at, and each tag re-runs `require_repository`. A repository with many release tags pays about 25 ms per tag (measured) and nears the 300 s limit at 10000 tags. Past the limit the partial result depends on machine speed. Fix: split `tag_checked` into a load step (resolve, cat-file, `parsed_trailers`, which are the same FR-041 reads) and a verify step. Run `names_record` on the loaded trailers, then verify only the matching tags against the already-resolved object id. That forks no FR-041 rule and also closes the gap between listing and verifying. | src/approval_trail_host.rs:160-187; src/signature_verify_host.rs:538-568 |
| FND-003 | medium | After a `time_limit` stop, `considered` still reports every listed ref (`listing.considered`), not the refs actually examined, so the result overstates its coverage. The spec defines `considered` as "the number of refs under refs/tags examined". Fix: count the refs reached in the loop. | src/approval_trail_host.rs:202 |
| FND-004 | low | The table cell sanitiser replaces only `char::is_control` characters. Unicode bidi and format characters such as U+202E and U+2066 to U+2069, and zero-width characters, pass through. Measured: a U+202E in a tag name is printed raw, which allows visual spoofing of the TAG cell, and of the following cells when it is in a trusted signer's MEANING. ESC and C1 are already replaced. Fix: also replace `General_Category=Cf` (or every non-printing character), and add an ESC/C1/RLO case to TC-360. | src/approval_trail_host.rs:225-233 |
| FND-005 | low | An entry does not carry the tag object id, so one signed object published under several ref names (TC-358 makes 10) reads as several independent approvals. Anyone who can push refs can multiply an old approval this way without a key. Fix: add the object id to each entry so a consumer can de-duplicate. | src/approval_trail_host.rs:62-69 |
| FND-006 | low | `TagListing::refs` is `Vec<(String, bool)>`, a tuple with an anonymous bool (rust-style API design: prefer a named struct or enum over a bare bool). The table also re-serialises each entry to `serde_json::Value` and reads members by string name, so a renamed field silently prints `-`. Fix: use a small `ListedRef { name, annotated }` struct and build the row from the typed `Entry` and `Assessment`. | src/signature_verify_host.rs:241; src/approval_trail_host.rs:238-258 |

## Verdict

The design is right. A pure ordering and record test sits in the library, a thin adapter reuses `check_tag` and the extracted `assess` in process, and the change adds no signature, trailer-parsing or decision code. The listing uses `%(refname)%00%(objecttype)` with no message text, and each tag is verified through its full `refs/tags/<name>` ref with `--end-of-options`, so hostile names stay data. The ordering is total and deterministic: fixed-width UTC instants, absent times last, and ties broken by tag-name bytes. It orders only the signer's claim, so a back-dated revocation or a forged unsigned tag sorts wherever it claims. The output says so, and `signed_at` stays approved-only, as intended.

Not mergeable. FND-001 must be fixed, because a real tag that names the record can be omitted with `truncated: false`. FND-002 and FND-003 should land in the same round.
