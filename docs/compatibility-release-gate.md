# v0.4.1 compatibility release gate

`make compatibility-release-gate` qualifies the v0.4.1 patch boundary. It runs
Ruff, Python tests, package audit, Quire document validation, Rust format,
Clippy, check, tests, and docs. Its final check reconstructs the exact matrix
bytes accepted by Peter Krenesky on 2026-09-23 from the released decision
metadata and verifies SHA-256
`26fb2ea02d8a9bc3cb97d0e43914be00dbcc57eee927267cba6c3ab139a22cc3`.
It also checks all ten schema digests against shipped files, the EA version in
the Cargo, wheel, npm, Quire, and agent-plugin manifests, and the CLI, module,
Codex, and Claude install references in the README.

This target qualifies a compatibility and plugin patch release. It does not
claim completion of the broader agent-evaluation and staged-runtime-migration
program. `make integration-gate` and `make release-gate` retain their existing
whole-repository meaning; neither is an alias of this scoped gate.

At the v0.4.1 candidate, `make integration-traceability` reports 334/351 backed
rows and 17 unbacked rows, with a partial census from the retained corpus
fixture marker plus a hollow denominator. The test-case population is
162/167, while the whole-repository gate requires 164/164. Clean v0.4.0
reports 327/346 backed rows, 20 unbacked rows, two false passing statuses,
and the same partial-census faults. The v0.4.1 patch fixes those two false
statuses by backing TC-014 with the real archive audit and explicit wheel
membership checks. The remaining findings stay open and must not be reported
as a passing whole-repository gate. The exact diagnostics are produced by
running `make integration-traceability`.

Dependency review remains separate from the scoped target. On macOS with an
offline Cargo cache, `cargo deny --locked --offline --target
aarch64-apple-darwin check` and `cargo audit --no-fetch` can review the host
dependency graph with a retained advisory database. The unrestricted
`make rust-deps` target requires target-specific crates that may not be in an
offline cache; a host-target pass does not claim the unrestricted check passed.
