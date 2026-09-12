---
id: SR-119
title: "Review of the traceability gate scope and the re-homed TC-016 coverage"
type: SpecReview
analysis: code-review
scope: "FR-003-AC-3, FR-017-AC-3, TC-016, TC-111; integration-traceability coverage validation"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
---

## Summary

`make integration-traceability` refused the coverage document with
`integration_evidence_coverage_invalid`. Three separate causes were present; two
were defects in the gate or in this repository, and the third is real
outstanding work that this change deliberately does not hide.

### The gate judged other repositories' code

All twelve reported untracked trace tags lay outside Engineering Assurance:
eleven in the `corpus` submodule and one in `vendor/ix-trace-rs`. The corpus
cases are the qa-corpus detection fixtures — `stale-name-correct-trace`,
`tag-names-unminted-tc`, `tag-names-undeclared-section` — whose entire purpose
is to carry ids that bind to nothing. Counting them as Engineering Assurance
traceability gaps made the gate unpassable by construction, and would have
pressured a future reader into editing a vendored or fixture tree to satisfy it.

`CoverageReference` decoded to an empty struct, so the host could not even see
which file a symbol came from. It now decodes `path`, and an untracked tag is a
local gap only when it lies outside every directory declared in `.gitmodules`.
A reference carrying no path is still treated as first-party, so the gate fails
closed. `validate_coverage` already applied exactly this locality reasoning to
diagnostics through `is_local_diagnostic`; untracked symbols simply never
received it.

### TC-016 claimed evidence it had lost

TC-016 `Local-source installation preserves discovery` was marked `✅ Passing`
in both the coverage table and the test-case table, while no test carried the
tag. Its only backing test was the Python
`test_manifest_schema_resolves_from_installed_module_root`, deleted in `a02e555`
(#56) when the Python manifest wrapper was retired. The row was never updated,
so a P0 row advertised evidence that had been removed one commit earlier.

The obligation itself survived the port: `manifest_host::execute` reads the
module manifest schema and edge registry from the authoritative module root
while reading the manifest and its resources from the repository root. TC-016 is
now backed by a Rust test that proves the installed tree is authoritative — a
valid copy placed inside the repository at the same relative path does not stand
in for a missing one in the module root.

### A pinned population that could never be met

`REQUIRED_TEST_CASES` was `68` against a declared population of `132`. The gate
asserts the test-case group is backed *and* equal to that constant, so it could
not have passed even with perfect coverage. The constant is raised to the
declared population; it stays pinned rather than derived, because pinning is
what catches a silently deleted test case.

## What this change does not fix

Twenty-one unbacked rows remain, from seven test cases that honestly declare
`🚧`: TC-039, TC-097, TC-108, TC-114, TC-115, TC-118 and TC-126. Several are
blocked outside this repository — TC-108 on the ix-flow host interface, TC-126
on quire-verification consumer integration, TC-115 on an owner disposition for
FR-016-CON-2. `make integration-traceability` therefore still fails, and
`make release-gate` with it. That is a truthful report of outstanding work, not
a defect this change should paper over, and no status was relaxed to make the
gate quieter.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-221 | high | Closed. The traceability gate counted trace tags inside the `corpus` and `vendor/ix-trace-rs` submodules as Engineering Assurance gaps, making it unpassable by construction and inviting edits to trees this repository does not own. Locality is now decided from `.gitmodules`, failing closed when a reference carries no path. | `src/integration_evidence_host.rs:96`; `src/integration_evidence_host.rs:296` | wrong-requirement |
| FND-222 | high | Closed. TC-016 advertised `✅ Passing` after its only backing test was deleted with the Python manifest wrapper in #56. The obligation is re-homed to a Rust test that proves the installed module bundle is authoritative and that a repository copy does not substitute for it. | `src/manifest_host.rs:352`; `spec/tests.md:194` | correct-requirement-no-evidence |
| FND-223 | medium | Closed. `REQUIRED_TEST_CASES` pinned 68 against a declared population of 132, so the gate could not pass even at full coverage. Raised to the declared population and shared with the fixtures that assert it. | `src/integration_evidence_host.rs:38`; `tests/integration_evidence_cli.rs:26` | implementation-bug-despite-evidence |

## Gate results

| Gate | Result |
| --- | --- |
| `make rust-foundation-gate` | pass |
| `make validate-docs` | pass |
| `quire coverage` before | 246/270 backed, 22 unbacked rows, 124/132 test cases |
| `quire coverage` after | 247/270 backed, 21 unbacked rows, 125/132 test cases |
| `make integration-traceability` | still fails on the seven `🚧` test cases above |
