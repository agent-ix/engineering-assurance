---
id: SR-084
title: "Rust review of package audit, cutover, and governed deletion"
type: SpecReview
analysis: code-review
scope: "FR-003-AC-1, FR-003-AC-2, FR-003-AC-4, FR-003-AC-5, FR-003-AC-6, NFR-003-AC-1, NFR-003-AC-2, FR-017-AC-3, FR-017-AC-4, FR-017-CON-2, FR-017-CON-3, FR-018-AC-2, FR-018-AC-3, TC-014, TC-015, TC-017, TC-018, TC-019, TC-037, TC-040, TC-111, TC-112, TC-113, TC-114; src/package_archive.rs; src/package_audit_host.rs; src/process_host.rs; tests/package_audit_cli.rs; Makefile"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-003"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/FR-018"
    type: reviews
  - target: "ix://agent-ix/engineering-assurance/NFR-003"
    type: reviews
---

## Summary

The repository rules and exact `agent-skills/rust-review/SKILL.md` checklist
were applied to the complete package-audit host adapter, archive decoders,
bounded child-process mechanic, direct Make dispatch, and governed deletion.
The review also reran the owner-selected QUOIN base review after implementation
findings changed the required boundary.

The adapter builds the retained wheel and npm distributions offline, validates
their reports and raw archive populations independently against explicit
allowlists, applies the pure content-rights policy, installs both packages into
invocation-owned roots, inspects their canonical discovery surfaces, and
requires their canonical bytes to agree. Package-manager processes execute
without a shell, with cleared and explicitly reconstructed environments,
bounded output and time, invocation process groups, confined named output
classes, and before/after repository-root population checks.

The direct package-audit target is now Rust and the superseded Python package
audit and content-rights implementations and their executable references have
been removed. Historical evidence remains inert. Thirteen review findings were
closed; no open Rust finding remains in this slice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-185 | high | Closed: timing out only the direct package-manager child could leave a descendant holding an output pipe, causing the reader joins to exceed the declared deadline. Each invocation now owns a process group, terminates the group before joining output, and has a mutation-tested descendant case. | `src/process_host.rs:100`; `src/process_host.rs:117`; `src/process_host.rs:141`; TC-111 |
| FND-186 | high | Closed through `/specify`: builder and installer cache/temp output and new repository-root entries were not fully confined or compared. The adapter reconstructs package-manager cache/temp variables beneath its temporary root and compares bounded top-level populations before and after every external invocation. | `src/package_audit_host.rs:303`; `src/package_audit_host.rs:364`; `src/package_audit_host.rs:440`; FR-017 Behavior; TC-111 |
| FND-187 | medium | Closed: the deletion census initially missed retained documentation-order assertions for local/repository and module/plugin installation. Rust TC-019/TC-037 now preserves those requirements before the Python test is removed. | `tests/package_audit_cli.rs:143`; FR-003-AC-6; FR-007-AC-3 |
| FND-188 | high | Closed through `/specify`: archive rejection occurs after a builder reads its sources, so a linked selected source could escape before the archive gate. The complete selected module tree and fixed build/package sources are preflighted as regular, link-free entries before either builder executes. | `src/package_audit_host.rs:245`; `src/package_audit_host.rs:270`; FR-017 Behavior; TC-111 |
| FND-189 | medium | Closed: the first reconstructed child environment retained package-registry credential variables. Protected token, index, proxy, and registry variables are removed rather than inherited by offline local package operations. | `src/package_audit_host.rs:403`; FR-017-CON-3 |
| FND-190 | high | Closed: accepting npm's JSON report without independently checking decoded archive membership allowed the producer's two outputs to agree with themselves rather than repository policy. Both observations must independently equal the repository-owned allowlist, and mutation coverage breaks each gate separately. | `src/package_audit_host.rs:595`; `src/package_audit_host.rs:732`; TC-018; TC-111 |
| FND-191 | high | Closed: content-rights policy was composed into the real integration path but lacked a direct adverse assertion. A protected finding now refuses archive acceptance, and bypassing the production scan fails that test. | `src/package_audit_host.rs:604`; `src/package_audit_host.rs:811`; TC-111 |
| FND-192 | high | Closed through `/specify`: normal tar iteration preprocesses GNU/PAX extension headers before the adapter can apply population, kind, path, and byte ceilings. npm decoding now uses raw iteration, rejects extension headers before preprocessing, counts every raw header, and refuses directory payloads. | `src/package_archive.rs:98`; `src/package_archive.rs:118`; `src/package_archive.rs:380`; FR-017 Behavior; TC-111 |
| FND-193 | high | Closed through `/specify`: `pyproject.toml` selects the wheel build backend but was omitted from fixed-source preflight. It is now required to be a regular selected-root file, and a linked build-configuration fixture is refused before construction. | `src/package_audit_host.rs:260`; `src/package_audit_host.rs:912`; TC-111 |
| FND-194 | high | Closed through `/specify`: the non-Unix process adapter silently degraded descendant containment to direct-child termination. Unsupported hosts now fail before spawn rather than claim a deadline they cannot enforce. | `src/process_host.rs:68`; `src/process_host.rs:93`; FR-017 Behavior; TC-111 |
| FND-195 | medium | Closed through `/specify`: “no output outside the temporary directory” overclaimed an operating-system sandbox and contradicted the accepted reusable compiler cache. The requirement now names only invocation-created package-manager cache/temp, archive, installation, and repository-staging classes and explicitly excludes a general sandbox claim. | FR-017 Behavior; FR-017-CON-2 |
| FND-196 | high | Closed: final Python deletion removed the only TC-015, TC-017, TC-018, and NFR-003 package-stability trace bindings. Rust tests now preserve the npm contract, repository-source installed discovery, independent membership refusal, archive-membership refusal, and both explicit allowlist obligations. Aggregate traceability improves rather than regresses across deletion. | `tests/package_audit_cli.rs:42`; `tests/package_audit_cli.rs:87`; `src/package_audit_host.rs:732`; `src/package_audit_host.rs:758`; `src/package_audit_host.rs:865`; TC-015; TC-017; TC-018; TC-040 |
| FND-197 | low | Closed: TC-111 still described full slice review and promotion as pending after this review completed the final package/archive slice. Its matrix status now records the reviewed, passing composed capability population without promoting the still-pending TC-112 host census or aggregate TC-113/TC-114 removal work. | `spec/tests.md:286`; TC-111 |
| FND-198 | low | No open Rust finding remains. Production code adds no unsafe block, panic, lint suppression, shell command, network access, async/lock surface, unbounded process output, unbounded archive population, or arbitrary deletion. New Rust requirement tests use canonical bare `ix_trace_rs::trace` markers. | reviewed source; NFR-005 |

## Gate results

| Gate | Result |
| --- | --- |
| QUOIN `/specify` and owner-selected base `/spec-review` | pass after implementation findings returned to the specification; SR-083 validates with inherited duplicate-provider diagnostics only |
| Exact Rust 1.98.1 formatting | pass |
| Exact Rust 1.98.1 Clippy and compile | pass; workspace/all targets/all features/locked, `-D warnings`, two build jobs |
| Exact Rust 1.98.1 tests | pass; 125 tests across library, binary, integration, and documentation targets |
| Exact Rust 1.98.1 rustdoc | pass; workspace/all features/no dependencies/locked, `-D warnings` |
| `cargo deny --locked check` | pass; advisories, bans, licenses, and sources; duplicate transitive versions remain policy warnings |
| `cargo audit` | pass; 151 locked dependencies scanned against 1,243 advisories |
| `make lint` | pass |
| `make test` | pass; content-rights tree accepted 278 entries, 170 retained Python tests passed, and module validation passed |
| `make package-audit` | pass; wheel 70 files, npm 60 files, and six canonical installed files agreed |
| Direct cutover and rollback | pass; Rust cutover, revert to the retained path, and revert-of-revert were committed and independently exercised without history rewriting |
| Local Quire validation | pass; inherited duplicate-provider diagnostics only |
| Whole-port integration traceability | expected incomplete; this branch has 231/248 relationships and 115/121 test cases versus main at 230/248 and 114/121. The remaining rows belong to later Rust-port slices, not this package-audit implementation |
| Hosted CI | not run; local-only policy preserved |

## Mutation evidence

- Removing descendant process-group termination lets a background child retain
  the output pipe and fails the bounded timeout test.
- Removing selected-root `pyproject.toml` preflight admits its symlink fixture
  and fails the owning TC-111 test.
- Restoring tar extension preprocessing admits the GNU/PAX header fixture and
  fails the raw-member test.
- Bypassing npm archive membership while retaining report membership fails the
  independent TC-018 assertion.
- Bypassing the content-rights scan admits the denied content fixture and fails
  its owning TC-111 assertion.
- Changing an exact resource ceiling from `>` to `>=` rejects the accepted
  boundary and fails its boundary test.
- Disabling failed-pack cleanup leaves the invocation-created staging
  population and fails the cleanup ownership test.
- Replacing the Make target with `true` fails TC-112 with the exact observed
  dry-run mismatch; restoring the Rust 1.98.1 dispatch returns it to green.

## Review disposition

**PASS for the reviewed package-audit Rust implementation, direct cutover, and
governed deletion.** The result completes this port slice without introducing
a public producer-execution API, new language work, a cross-repository registry,
hosted CI, or public package publication. Aggregate Engineering Assurance Rust
port completion remains governed by its separately traced remaining slices.
