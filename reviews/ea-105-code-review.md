---
id: SR-20261113192936
title: "code-review and rust-review of pluggable record sinks with a fail-closed public-log check (engineering-assurance PR 233)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/engineering-assurance@94daf21; src/record_sink/{mod,config,form,leak,sink}.rs; src/lib.rs; Cargo.toml; tests/record_sink.rs; spec/tests.md"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-057
    type: references
  - target: ix://agent-ix/engineering-assurance/NFR-009
    type: references
---

# Code review and Rust review of pluggable record sinks (engineering-assurance PR 233)

## Summary

Ticket: EA-105. Scope is `git diff origin/main...94daf21` only. Rust lane (rust-style, rust-review) folded in.
Gates run by the reviewer at 94daf21: `cargo test --locked --features record-sink --test record_sink` 29 passed;
`cargo clippy --locked --features record-sink --lib --test record_sink -- -D warnings` clean; `cargo fmt --check` clean. No workflow files are touched; no new dependency (the `record-sink`
feature reuses `serde`, `serde_json`, `serde_json_canonicalizer`, `thiserror`); the module performs no I/O and
implements no cryptographic primitive; no compatibility layer, no vendored content, no ceremony found.

Examined clean: the closed public/private form grammars (form.rs), `Record::new` refusing a signed input form,
RFC 8785 `signed_bytes` via `serde_json_canonicalizer` and the exact byte comparison in `attach_signature`
(a non-canonical or foreign payload is refused), the leak check's coverage of publisher, subject and every
reference with `Unknown`/lookup error treated as an offender and every offender reported once, private form
written before the leak check and a write failure refusing the publish, config resolution and
`read_policy_document`'s `Some(true)`-only protection rule, and absence of panics on untrusted input
(all slice indexing is behind length checks).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The public-log send is not gated by `PublicPathProof`. `PublicLogClient::send(&self, endpoint, entry: &PublicRecord)` takes no proof, and `Sinks.public_log` is any `&dyn RecordSink`, so any registered implementation (FR-054-AC-4 invites them; EA-97/EA-98 will add callers) or any caller can reach the public log with no leak check. The PR body claims the send "requires" the proof; the only enforcement is TC-651's text grep of one file. Fix: make `send` take an entry type that only sink.rs can construct from a `PublicPathProof` plus the verified bundle (private fields, no public constructor), e.g. `send(&self, endpoint, entry: &CheckedPublicEntry<'_>)`, and drop the pluggable `public_log` slot (the layer owns `PublicLogSink`) or require it to be `&PublicLogSink`. | src/record_sink/sink.rs:128-135; src/record_sink/sink.rs:290-297; src/record_sink/sink.rs:240 |
| FND-002 | medium | `Signer::sign_for_public_log(proof, canonical: &[u8])` does not bind the bytes to the proof: a proof for record A authorises a public-log upload of arbitrary bytes. With a real keyless signer the upload happens inside this call, so the `signature_payload_mismatch` check (PR decision 2) only runs after the upload. Fix: `sign_for_public_log(&self, proof: &PublicPathProof<'_>)` with the proof carrying the checked record's canonical bytes (`proof.signed_bytes()`), so the public path can only ever receive the checked public form's RFC 8785 bytes; keep the post-sign payload equality as a check on the delegated library. | src/record_sink/sink.rs:152-156; src/record_sink/leak.rs:59-69 |
| FND-003 | medium | Settings are duplicated between `SinkSelection` and the sinks and nothing ties them. `SinkLayer` checks `selection.public_log_endpoint`, `selection.private_store_destination` and carries `selection.public_linear_teams`, but `PublicLogSink` sends to its own `endpoint` and checks its own `public_linear_teams`, and `PrivateStoreSink` writes the public form to its own `destination`. A caller can check Linear teams other than the protected-branch config's list, or store the linked public form at a different destination than the private form. Fix: build both sinks from `&SinkSelection` (constructor taking the selection) or have the layer pass endpoint, destination and teams from the selection into the sink. | src/record_sink/sink.rs:200-213; src/record_sink/sink.rs:250-257; src/record_sink/sink.rs:318-332 |
| FND-004 | low | The publish result schema closes the member set but not the combinations: `error` is accepted on `published`/`discarded`, `refused` without `error` parses, and `offending` is accepted on any status or error code. FR-054 puts `error` only on `refused`; FR-057 puts `offending` only on `public_sink_private_reference`. Fix: deserialize through a raw struct with `TryFrom` enforcing error iff refused and offending only with that code; add the cases to tc_628. | src/record_sink/sink.rs:41-58 |
| FND-005 | low | Optional members of the closed documents accept JSON `null` as absent (`"hashes": {"statement": null}`, `"sink": null`, `"public_log": {"endpoint": null}`). `null` is in no member's grammar. Fix: reject `null` for optional members (one small `deserialize_with` helper that errors on null; no new dependency) and add one null case per document kind. | src/record_sink/form.rs:398-440; src/record_sink/config.rs:111-132 |
| FND-006 | low | The test stand-in `Store.events` is written but never read, and `let _ = (References::default(), Subject::repo(PUBLISHER).unwrap())` exists only to use imports. Fix: record signer and log calls in `events` and assert in tc_629 that `private_write` precedes every lookup, sign and send (which strengthens the "private form first" check), or delete both. | tests/record_sink.rs:49; tests/record_sink.rs:925 |

## Verdict

Not mergeable yet. FND-001 is the gate: the fail-closed leak check is enforced by code layout, not by the type the PR
says enforces it, and the public log cannot delete what it accepts. FND-002 and FND-003 are the same class of
unbound-capability gap and should land in the same fix round. Everything else in the leak check, the forms and the
private-first ordering is sound.

On the author's decisions: #2 (mismatch detected after a real keyless upload) is not a leak of private content today,
because the crate itself only hands the signer the canonical bytes of a public form that passed the check; it becomes
a non-issue by construction once FND-002 binds the bytes to the proof. #7 (default `public_log` without an endpoint
resolves, then the publish refuses with `sink_config_incomplete` before anything is written) is forced by FR-055-AC-1,
fails closed and is accepted; the spec has no default endpoint, so such a repository never publishes until it adds
one — a spec question, not a code defect. #1, #3-#6, #8, #9 are accepted. Config is read only through
`PolicySource::read_from_base_branch` behind a `Some(true)` protection check; sink `none` sends, stores and signs nothing.
