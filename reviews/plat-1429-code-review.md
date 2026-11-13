---
id: SR-16413
title: "code-review and rust-review of the Syft producer request tests (engineering-assurance PR 216)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@e7d8da9; Cargo.toml; spec/tests.md; tests/syft_producer_cli.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-038
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-039
    type: references
---

# Code review and Rust review of the Syft producer request tests (engineering-assurance PR 216)

## Summary

Ticket: PLAT-1429. Head e7d8da9, diff `origin/main...HEAD` (3 files, +533): one `[[test]]` entry with `required-features = ["full"]`, ten TC rows, and `tests/syft_producer_cli.rs`. Rust-review lane folded in (Agent IX rust-style loaded). No production code, no new execution path and no new Cargo dependency (`tar`, `jsonschema`, `tempfile` are existing optional `full` dependencies; digests use `sha256sum` rather than a new crate).

Request helper checked against the published schema `src/producer_execute_schemas/producer-execution-request-v1.schema.json` and FR-038's table: protocol, producer, caller, `direct`, capabilityRoot, the five literal arguments in order, the five environment variables, empty inputs, null stdin, one required `sbom` output at `out/sbom.cdx.json`, `process-group-v1` with the `syft-containment` contract, disabled cancellation, all eight budget bounds and the exit-0-only response all match. TC-286 validates it with `jsonschema`.

Executor assumptions verified by reading `src/producer_execution.rs`: `build_command` sets `current_dir(/proc/<pid>/fd/<root>)` with `env_clear()`, so the child's cwd is the capability root and relative `scan/` and `out/sbom.cdx.json` resolve there; the only unlink is `remove_declared_outputs` before launch, so the produced file stays on disk after the run and the test can read it; artifact JSON fields are `role`, `path`, `byteLength`.

Syft behaviour verified by reading Syft v1.57.1 source (not by running Syft, which is not installed here): cargo-lock packages with a non-empty `source` get `pkg:cargo` purls, so a Rust fixture without a root entry yields 3; package-lock v2/v3 catalogues the `""` root when it carries a `name`; dpkg purls are empty when no distro is detected; config is searched in the cwd (`./.syft.<ext>`, `./.syft/config.<ext>`), `~/.syft.<ext>`, and the XDG config home and config dirs.

Measured: `cargo test --features full --test syft_producer_cli`: TC-283..TC-287 pass; TC-278..TC-282 fail with "EA_TEST_SYFT must name the absolute path of the Syft binary under test". `cargo clippy --features full --test syft_producer_cli -D warnings` clean; `rustfmt --check` clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Merging turns `make rust-foundation-gate` (CI step, via `rust-tests` = `cargo test --all-targets --all-features`) red on CI and every machine without Syft: TC-278..TC-282 panic by design (FR-038-CON-1) and `ci.yml` installs no Syft and sets no `EA_TEST_SYFT`. `make test` is unaffected (runs only `python_port`). Owner decision needed: provision a pinned Syft in CI and export `EA_TEST_SYFT` (the quire precedent), or reword CON-1 so the Syft tests are `#[ignore]`d and run from a dedicated target that fails when the variable is unset (the `manifest_host_cli` precedent). | Cargo.toml:154; Makefile:rust-tests; .github/workflows/ci.yml:52; tests/syft_producer_cli.rs:67 |
| FND-002 | high | TC-282 plants `.syft.yaml` at the capability-root top level, which is Syft's cwd, so Syft reads it (fangs FindInCwd) and the `exclude: ["**"]` either empties the SBOM or is rejected; the test cannot pass. The spec contradicts itself: FR-038 Behavior forbids a Syft config at the capability root top level while FR-038-AC-7 plants one "outside scan/". Fix AC-7 and the test to plant `.syft.yaml` and `.syft/config.yaml` inside `scan/`, the repository-supplied location the requirement is about. | tests/syft_producer_cli.rs:402; spec/functional/FR-038-syft-sbom-producer-request.md |
| FND-003 | high | TC-279's lock-file root entry `""` carries `"name": "fx-root"`, and Syft v1.57.1 `parsePackageLock` catalogues a named root, so the artifact holds 4 components, not 3. Drop `name` (and `version`) from the `""` entry so the root is skipped, as FR-039's declared-count rule requires. | tests/syft_producer_cli.rs:235 |
| FND-004 | high | TC-281 asserts every component purl starts with `pkg:deb/`, but Syft's dpkg `packageURL` returns an empty purl when no distro is detected, and the fixture deliberately has no `os-release`. Every purl reads `""`, so the test fails. FR-039-AC-4 does not require purls. Assert component names and type `library` instead. If downstream needs purls (the quoin adapter, FR-041), use a language package database in the layer, such as a Python `dist-info/METADATA`. | tests/syft_producer_cli.rs:390 |
| FND-005 | medium | FR-038's config isolation is incomplete. Syft also reads `.syft.json`/`.toml` and other viper extensions in the cwd, `~/.syft.*` and `$XDG_CONFIG_HOME/syft/config.*` (both under `out/home`, which the caller creates), and `/etc/xdg/syft/config.*` from the host, because `XDG_CONFIG_DIRS` is unset. Image unpacking also writes under `/tmp`, because `TMPDIR` is unset and so falls outside `out/`. Consider a `--config` argument naming an empty caller-written file under `out/`, plus `TMPDIR=<root>/out/tmp`, or state these limits in the spec. | spec/functional/FR-038-syft-sbom-producer-request.md |
| FND-006 | low | TC-286 checks FR-038-AC-5 "equal the definition" only partially. It checks the environment by count plus one value, `containment.contract` but not `profile`, and that the `-o` value is present but not the argument order. Compare `arguments`, `environment` and `containment` against literal expected JSON. | tests/syft_producer_cli.rs:487-506 |
| FND-007 | low | TC-283..TC-285 gate on Linux with a runtime `if !cfg!(target_os = "linux") { return; }`, so on another OS they report a pass that asserts nothing. Use `#[cfg(target_os = "linux")]` on the test functions, as `tests/producer_execution.rs` does. | tests/syft_producer_cli.rs:426,441,461 |
| FND-008 | low | The `@@` placeholder and the `concat!("https:", "//")` constant exist only to keep URLs out of sight of the content-rights URL scanner. The npm `resolved` and `integrity` fields are not read by Syft, so drop them. A cargo `source` needs only to be non-empty for a purl. Removing them avoids the workaround in test fixtures. | tests/syft_producer_cli.rs:25,77,236-238 |

## Verdict

The request helper is correct against FR-038 and the FR-019 schema, and the executor assumptions hold. The non-Syft tests (TC-283..TC-287) pass and their ✅ rows are truthful. The TC-278..TC-282 rows correctly carry no ✅. Reading Syft v1.57.1 source predicts that three of the five Syft-dependent tests (TC-279, TC-281, TC-282) fail even with Syft installed. TC-278 and TC-280 are predicted to pass. The OCI archive layout (oci-layout, index.json, an uncompressed layer whose digest equals its diff_id, config and manifest blobs) is plausible but untested. Not mergeable until FND-002..FND-004 are fixed and run against a real Syft, and the owner decides FND-001.
