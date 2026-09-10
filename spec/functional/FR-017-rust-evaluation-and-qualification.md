---
id: FR-017
title: "Run evaluations and repository qualification through Rust"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-003"
    type: "implements"
  - target: "ix://agent-ix/engineering-assurance/FR-006"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/NFR-003"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: "requires"
---

# FR-017: Run evaluations and repository qualification through Rust

## Description

Engineering Assurance SHALL port its agent-evaluation planning, result
validation, aggregation, package staging, publication refusal, content-rights
checking, manifest validation, and qualification assertions to Rust.

## Inputs

- Canonical agent-evaluation scenarios and supported hosts.
- Versioned cli-agent-evals results and retained transcripts.
- Package manifests, staged archives, rights policy, and integration evidence.
- For the pure package-membership boundary, one explicit expected-member
  allowlist and one observed file-member sequence supplied by an archive or
  staging adapter.
- For the pure aggregation boundary, one
  `engineering-assurance.evaluation-aggregate-request/v1` document containing
  evaluation envelopes for the closed host and scenario populations. Each
  envelope carries its execution status, pass state, immutable suite, fixture,
  source, host, and governing identities, relative transcript reference and
  lowercase SHA-256 digest, non-negative effort counts, observed outcome,
  unsupported-addition list, optional diagnostic, and the complete terminal
  event when the scenario requires a human decision.

## Outputs

- Versioned per-run and aggregate evaluation results.
- Deterministic qualification diagnostics and exit statuses.
- Audited package contents and enforced publication refusal.
- The pure aggregation boundary emits one
  `engineering-assurance.evaluation-aggregate-result/v1` value containing the
  pass decision, required-cell count, complete-cell count, and a deterministic
  ordered list of stable failure codes. It does not persist evidence or infer a
  release decision.
- The pure content-rights boundary emits typed findings containing only a
  normalized repository-relative path when path validation succeeds, a
  one-based line number (or zero for a whole-file finding), and one closed
  finding category. An unsafe path finding carries no path value. A finding
  never returns the rejected unsafe path, matched source text, or a protected
  token.
- The pure package-membership boundary emits one accepted/withheld result and
  a deterministic list of typed findings. A finding is one of
  `actual_path_invalid`, `actual_path_duplicate`, `unexpected_member`, or
  `missing_member`; only a lexically safe normalized path may appear in a
  finding. An invalid observed path carries no path value.

## Behavior

- Preserve the evaluation fields and explicit terminal events required by
  FR-006.
- Treat the supported host population as `claude`, `codex`, `opencode`, and
  `copilot`, and the scenario population as `existing-profile`, `no-profile`,
  `malformed-producer`, `unavailable-producer`, `interruption-resume`,
  `human-acceptance`, and `human-rejection`; their Cartesian product is exactly
  28 required cells.
- Validate envelopes through closed Rust types rather than untyped object
  walking. Unknown envelope fields, malformed request structure, an unknown
  protocol, unsupported host or scenario values, and invalid scalar types are
  request errors and produce no aggregate result. The byte boundary refuses
  input larger than 8 MiB before JSON decoding or aggregate allocation.
- An executed cell is complete only when it passed; every immutable identity is
  present and valid; its transcript reference is a normalized,
  forward-slash-delimited, relative and non-traversing protocol path; its
  transcript digest is lowercase SHA-256; every effort count is a non-negative
  integer; the observed outcome is the scenario's declared outcome; and
  unsupported additions are empty.
- A not-executed cell requires a non-empty diagnostic and remains incomplete.
  It cannot satisfy the aggregate even if it carries otherwise complete
  observations.
- Human-acceptance and human-rejection cells require complete terminal events
  with the corresponding choice and outcome, the governing workflow name and
  version, a non-empty owner, a safe run ID, and an RFC 3339 timestamp. Every
  other scenario requires no terminal event. For each host the two decision
  cells use distinct run IDs and identical source, fixture, and governing
  identities.
- Fail aggregation when a required host-scenario cell is absent, invalid,
  unavailable, unsuccessful, or duplicated. An unsupported host or scenario is
  a malformed request rather than an aggregate cell.
- Require one source revision, suite revision, fixture revision, and governing
  tuple across the aggregate. Require one workflow identity within each
  scenario while permitting the declared intake and architecture workflows to
  differ between scenarios.
- Order missing, duplicate, cell-validation, aggregate-identity,
  scenario-workflow, and terminal-pair failures deterministically by closed
  host/scenario order so equivalent input permutations produce the same result.
- Classify content-rights inputs through one deterministic Rust policy that
  accepts explicit repository-relative paths, file/symbolic-link kind, file
  bytes, and protected tokens from its caller. Admit text files with no suffix
  or one of `.cfg`, `.css`, `.html`, `.js`, `.json`, `.lock`, `.md`, `.mjs`,
  `.py`, `.rs`, `.sh`, `.toml`, `.ts`, `.txt`, `.yaml`, and `.yml`; classify
  `.doc`, `.docx`, `.epub`, `.gif`, `.jpg`, `.jpeg`, `.ods`, `.odt`, `.pdf`,
  `.png`, `.ppt`, `.pptx`, `.xls`, `.xlsx`, and `.zip` as forbidden, and every
  other suffix as unreviewed. Preserve the 512,000-byte non-license ceiling; NUL,
  non-UTF-8, Git LFS pointer, workstation-path, external-publication-identifier,
  prohibited semantic-content, unapproved-URL, long encoded-payload, and
  protected-token findings; the exact Cargo registry URL exceptions for
  `Cargo.lock` and `deny.toml`; the JSON Schema draft-07 URL-prefix exception;
  and the `LICENSE` URL exception.
- Exempt semantic-policy phrases only in `AGENTS.md`, `CONTENT_RIGHTS.md`,
  `content-rights.yaml`, `src/content_rights.rs`, and
  `tests/content_rights_parity.rs`. During additive parity, apply the same
  exception to the retained `scripts/check_content_rights.py` and
  `tests/test_content_rights.py`; deleting those two exceptions is part of the
  later governed removal, not this slice.
- Reject an empty, absolute, backslash-delimited, dot-segment, parent-traversing,
  control-character, or non-normalized content path. Order findings by path,
  line, and closed category rather than discovery or hash-map order. Protected
  token matching preserves Unicode case-fold behavior, ignores empty tokens,
  and emits no matched bytes. One-based line numbers treat LF, CR, CRLF,
  vertical tab, form feed, file/group/record separators, NEL, Unicode line
  separator, and Unicode paragraph separator as line boundaries, matching the
  retained classifier.
- Compare complete staged package membership with explicit allowlists and reject
  missing, extra, or escaping members.
- When package membership is classified, the Rust policy SHALL reject an expected
  allowlist containing an empty, duplicate, absolute, backslash-delimited,
  trailing-slash, empty-segment, dot-segment, parent-traversing,
  control-character, non-normalized, or greater-than-4,096-byte path.
- The Rust membership classifier SHALL withhold acceptance for every observed
  invalid path, duplicate safe path, unexpected safe member, or missing
  expected member.
- The Rust membership classifier SHALL compare safe member names as exact,
  case-sensitive strings without decoding archive-specific escaping.
- The Rust membership classifier SHALL order invalid-path findings first,
  followed by duplicate, unexpected, and missing findings.
- The Rust membership classifier SHALL order path-bearing findings lexically
  within each category.
- The Rust membership classifier SHALL return an identical result for
  equivalent input permutations.
- The pure membership boundary SHALL reject an expected or observed population
  larger than 65,536 entries before building comparison state.
- The pure membership boundary SHALL return a typed error and no membership
  result for an expected-policy or resource-limit refusal.
- The pure membership boundary SHALL accept caller-supplied member names.
- The pure membership boundary SHALL NOT select a wheel, npm, Cargo, or other
  distribution format. Archive decoding,
  member-kind validation, package construction, installation, and filesystem
  traversal remain responsibilities of later binary adapters.
- Inspect the complete selected tree and staged members for content rights.
- Keep publication refused.
- Keep package-manager and host configuration declarative; first-party semantic
  assertions live in Rust.

## Error Conditions

Missing hosts, incomplete scenarios, invalid results, revision mismatches,
changed governing files, unexpected package members, rights denials, and
attempted publication fail their corresponding local gate.
An invalid or duplicate expected package path and an over-limit member
population refuse the pure package-membership request. Invalid, duplicate,
unexpected, and missing observed membership withhold package acceptance.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-017-CON-1 | The Rust evaluator SHALL NOT infer a human terminal decision. | Responsibility | Test |
| FR-017-CON-2 | Package-manager and host configuration SHALL contain no first-party semantic assertion. | Architecture | Test |
| FR-017-CON-3 | The reusable Rust qualification library SHALL NOT access the filesystem, environment, child programs, network, or system clock; tree enumeration, transcript-byte loading, host execution, and current-HEAD comparison belong to explicit binary adapters. | Responsibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-017-AC-1 | The Rust evaluation path completes all required host-scenario cells and matches retained success and declared failure behavior. | E2E (TC-109) |
| FR-017-AC-2 | The pure Rust boundary preserves the 28-cell retained evaluation contract and deterministically withholds aggregation for every missing, duplicate, malformed, unsupported, unavailable, failed, stale-revision, changed-governing-identity, changed-workflow, unsupported-addition, invalid-transcript-reference, invalid-count, outcome-mismatch, and terminal-pair case; input permutation cannot change the result. | Property (TC-110) |
| FR-017-AC-3 | Rust package, rights, manifest, integration, and publication-refusal checks match the retained pass/fail corpus and reject extra, missing, or escaping package members. | Property (TC-111) |
| FR-017-AC-4 | Static inspection finds only declarative dispatch in package-manager and host configuration; qualification is performed locally and real-agent evaluation, publication, and release operations remain explicit manual actions. | Test (TC-112) |
| FR-017-AC-5 | The pure Rust content-rights classifier matches every retained finding class and exception, rejects unsafe paths without echoing them, preserves Unicode protected-token matching, emits no matched content, and returns deterministic typed findings without filesystem, environment, child-program, network, or clock access. | Property (TC-119) |
| FR-017-AC-6 | For every caller-supplied expected allowlist and observed file-member sequence within the declared ceilings, the pure Rust membership classifier matches retained extra/missing behavior on the shared safe unique domain, additionally rejects unsafe or duplicate membership without disclosing unsafe paths, remains permutation-invariant, and performs no filesystem, environment, child-program, network, clock, archive-decoding, or package-format selection. | Property (TC-120) |

## Dependencies

- **Upstream**: FR-006, FR-014, FR-015, and a cli-agent-evals interface
  supported by that host's owner.
- **Downstream**: FR-018 requires these checks before old implementations are removed.
- **Sequence**: FR-017-AC-2's pure typed validation and aggregation is an
  independently reviewable additive slice. It does not satisfy FR-017-AC-1,
  verify transcript bytes, compare the current repository revision, authorize
  host execution, or permit removal under FR-018.
- **Sequence**: FR-017-AC-5 is an independently reviewable pure classifier
  slice. It does not enumerate a repository, replace the tree adapter, select a
  package format, satisfy the combined FR-017-AC-3 gate, or permit removal under
  FR-018.
- **Sequence**: FR-017-AC-6 is an independently reviewable format-neutral
  membership slice. It does not parse an archive, validate member kinds, build
  or install a package, select the Rust CLI distribution format, satisfy the
  combined FR-017-AC-3 gate, or permit removal under FR-018.
