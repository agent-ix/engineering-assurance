---
id: SR-126
title: "Code review — manifest-validate schema/registry root split (PLAT-994)"
type: SpecReview
analysis: code-review
scope: "src/manifest_host.rs, src/main.rs, tests/manifest_host_cli.rs, tests/manifest_parity.rs, Makefile, .github/workflows/ci.yml, FR-017, spec/tests.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

Reviewed PR #126, which replaces `manifest-validate`'s single `--module-root`
with explicit `--schema-root` and `--registry-root`, and adds an `#[ignore]`d
real-conformance test (TC-121). The ignored test was run independently against
`filament-core-service` `origin/main` and `spec-artifacts-iso` `main`, and it
passes. Negative controls show it is a real gate: a schema requiring an extra
key gives `manifest_schema_mismatch`, and a registry without `references`
gives five `unregistered_edge` findings. Both runs exit 1. No schema or
registry content is copied into this repository. Both roots are read at
runtime and neither is defaulted.

## Verdict

**CONDITIONAL** — only low findings. The dead CI environment variable
(FND-001) and the unused CI checkout (FND-002) still need a follow-up that
can edit workflow files.

## Findings

| ID      | Severity | Summary                                                                                                                                                                                                  | Refs                              |
| ------- | -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------- |
| FND-001 | low      | `MODULE_MANIFEST_SCHEMA` has had no reader since a02e555 retired the Python wrapper. It also names a file PLAT-902 deleted, so it suggests CI checks a schema when it does not. Left open because a workflow-file change could not be pushed with the available token. | .github/workflows/ci.yml:55       |
| FND-002 | low      | The CI `.deps/spec-artifacts-iso` checkout now has no consumer in the repository. It was left in place as out of scope and flagged for follow-up.                                                        | .github/workflows/ci.yml:16       |
| FND-003 | low      | The make-target test named "requires_and_forwards" checks forwarding only. It does not check refusal when only one of `MANIFEST_SCHEMA_ROOT`/`MANIFEST_REGISTRY_ROOT` is set. This gap existed before this PR. | tests/manifest_host_cli.rs:105    |
| FND-004 | low      | A missing or linked registry *file* under the registry root is not tested separately. It uses the same `read_file` path that the tested schema-file and repository-resource cases use.                    | src/manifest_host.rs:131          |
