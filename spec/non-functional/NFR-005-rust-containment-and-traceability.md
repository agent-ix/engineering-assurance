---
id: NFR-005
title: "Enforce Rust containment and test traceability"
type: NFR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "constrains"
---

# NFR-005: Enforce Rust containment and test traceability

## Statement

Engineering Assurance first-party production and qualification semantics SHALL
be implemented, tested, and locally qualified in Rust.

## Scope

This constraint covers libraries, CLIs, generators, validators,
canonicalization, result adapters, workflow and evaluation assertions, fixture
and package audits, and rights checks. Declarative host configuration and inert
foreign-language fixture samples are not semantic implementations.
The Rust language-boundary checker applies the same containment policy to
tracked changes in participating repositories, using their own Git history as
the baseline.

## Existing npm package release transitions

The Git-delta checker SHALL distinguish a version-only release of a
repository-owned npm launcher family from adding or expanding a foreign
dependency. Authority SHALL come from the selected safe base, not names or
metadata introduced only in the proposed head.

The sanctioned transition SHALL require the same regular `100644` manifest
path in both revisions, with the base package named `@agent-ix/<repository>`
and its Git repository declared using the `git+https` scheme, the
`github.com` host and the `agent-ix/<repository>.git` path, without a port,
user information, query or fragment. The base SHALL have a
nonempty bin map, no private-package flag except absent or false, and no
non-default npm registry. An existing optional dependency belongs to this
release family only when its name extends the exact base package name with
a nonempty hyphen suffix and its exact version equals the base launcher
version. The transition SHALL require a nonempty family, valid complete
SemVer values, a changed launcher version, and every family member changing
to that same new exact version.

After restoring only the launcher version and those existing family-member
versions to their base values, the head manifest SHALL equal the base
manifest as a JSON value. A changed dependency name, added dependency,
changed owner, launcher command, registry, third-party version or other
manifest member SHALL NOT qualify. Only the exact corresponding family
version findings SHALL be absent from the Git delta; all other source,
launch, dependency and unsafe-input findings SHALL retain their existing
classification. This transition SHALL NOT authorize publication or attest
that a registry release occurred.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Unapproved first-party non-Rust semantic or assertion paths | 0 | 0 | architecture-conformance |
| Unsafe Rust blocks | 0 | 0 | compile-time-check |
| Requirement-verifying Rust tests with canonical ix-trace-rs markers | 100% | 100% | static-quality |

## Rationale

The port must cover actual executable paths, while test traces use the exact
macro form Quire recognizes.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-005-AC-2 | Every first-party Rust test under `src/` or `tests/`, excluding vendored crates, imports `ix_trace_rs::trace` without an alias and uses at least one bare `#[trace(...)]` carrying a `TC-XXX` and an `*-AC-N` literal; missing, aliased, path-qualified, or malformed markers are rejected by parsed-syntax inspection. | Analysis (TC-117) |
| NFR-005-AC-3 | Quire reconciles Rust criterion markers to the test matrix with no missing, orphaned, or duplicate binding. | Test (TC-118) |
| NFR-005-AC-4 | The final executable-path and host-configuration audit reports zero unapproved non-Rust semantic or assertion paths. | Analysis (TC-115) |
| NFR-005-AC-5 | Every test-case id that a Test Matrix coverage row names has its own row in the Test Case Summary table, whatever padding the table cells carry; a coverage reference to a test case with no row fails. | Test (TC-205) |
| NFR-005-AC-6 | Given a repository and two full Git revisions, the Rust language-boundary checker compares findings by path and launch context against a safe base, allows unchanged historical findings, and refuses a missing, shallow, divergent, or otherwise unsafe base before reporting acceptance. | Test (TC-209) |
| NFR-005-AC-7 | For a safe base and head, newly added or expanded tracked non-Rust source, executable files, package scripts, bins and dependencies, Make and Actions launchers, action runtimes, Rust child-program launch sites, and opaque submodules produce typed refusal findings, except the exact version findings for the sanctioned existing-family release transition in AC-9; unclassifiable selected manifests or source fail closed. | Test (TC-210) |
| NFR-005-AC-8 | Only the named fixture-input, PyO3 binding-test, plugin-test-double, and documentation-figure paths are excluded from source findings. Actions checkout, local reusable workflows, and the Engineering Assurance Rust checker workflow referenced at `main` or a full 40-character hexadecimal revision do not count as foreign launchers; other external actions do. Accepted and refused results conform to the versioned result schema. | Test (TC-211) |
| NFR-005-AC-9 | The sanctioned existing npm family release transition accepts only the safe-base ownership, regular-mode, complete-SemVer, existing matching-version family and exact non-version manifest equality conditions in this requirement. New or renamed dependencies, foreign version changes, invalid base ownership or publication metadata, nonregular modes, non-SemVer values and changed non-family or unsynchronized members retain their dependency findings; an unchanged unrelated dependency does not join the release family. The result grants no publication authority. | Test (TC-210) |

## Verification

Run the repository's formatting, Clippy, build, test,
documentation, dependency-policy, Quire traceability, package, and rights checks
locally. Inspect Rust syntax with the Rust-owned pure source-audit boundary, and
inspect first-party executable and host-dispatch paths against ADR-002. The
source audit refuses a non-UTF-8, syntactically invalid, or greater-than-
2,097,152-byte individual source before analysis. Its alias resolution is
lexical and per document; it does not replace compiler or whole-program gates.

## Dependencies

FR-014 through FR-018 define the port scope constrained by this requirement.
`ix-trace-rs` is a Rust test-only dependency whose license and source
disposition are part of the package audit.
