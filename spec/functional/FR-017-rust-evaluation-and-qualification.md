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
- For the npm package-lifecycle adapter, one explicit repository root and one
  closed operation: `stage`, `clean`, or `refuse-publication`.
- For the pure package-membership boundary, one explicit expected-member
  allowlist and one observed file-member sequence supplied by an archive or
  staging adapter.
- For the pure module-manifest boundary, the expected module name and version,
  module-manifest YAML bytes, authoritative module-manifest JSON Schema bytes,
  edge-registry manifest YAML bytes, and one supplied schema/skeleton resource
  pair for each declared artifact type.
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
- One `engineering-assurance.package-lifecycle-result/v1` result identifying
  the requested operation and its closed outcome.
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
- The pure module-manifest boundary emits one accepted/withheld result and a
  deterministic list of typed findings. Findings carry a closed category and,
  only when safe and applicable, an artifact name or normalized schema
  reference; findings never contain source bytes or an unsafe reference.

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
- The npm staging adapter SHALL copy only `manifest.yaml`,
  `compatibility-matrix.json`, `contracts/`, `fixtures/`, `schemas/`, and
  `skeletons/` from the repository-owned `engineering_assurance/` module root
  to the package root.
- The npm staging adapter SHALL preflight every selected source as a regular
  file or directory containing only regular files and directories.
- The npm staging adapter SHALL reject symbolic links, special files, missing
  sources, non-portable UTF-8 relative paths, and any pre-existing staged
  destination before it writes.
- A portable staged path SHALL contain only non-empty UTF-8 segments without
  dot, parent, backslash, or control-character segments.
- The npm staging adapter SHALL reject more than 4,096 filesystem entries, an
  individual file larger than 8,388,608 bytes, or total staged bytes larger
  than 16,777,216 bytes before it writes.
- A staging error after writing begins SHALL remove only destinations that the
  current invocation proved absent during preflight and created itself.
- A successful npm staging result SHALL mean every staged byte matches its
  selected source byte after copying.
- The npm cleanup adapter SHALL validate every present staged destination as an
  exact regular-file tree copy of its current selected source before deletion.
- The npm cleanup adapter SHALL delete no destination when any staged member is
  missing, extra, changed, symbolic, special, or otherwise unverifiable.
- The npm cleanup adapter SHALL treat an entirely absent staged population as a
  successful idempotent cleanup.
- The publication-refusal operation SHALL always return the closed refused
  outcome without reading package contents or contacting a registry.
- Each direct package-lifecycle operation SHALL emit one versioned JSON result
  on stdout and diagnostics only on stderr.
- An explicitly selected npm-hook rendering mode SHALL leave stdout empty so
  npm retains ownership of its own machine-readable output.
- The npm-hook rendering mode SHALL communicate stage or cleanup success only
  through exit status.
- The npm-hook rendering mode SHALL communicate publication refusal through a
  non-success status and a diagnostic on stderr.
- The npm lifecycle configuration SHALL invoke the Rust CLI through the pinned
  local Cargo package without embedding staging or refusal semantics.
- The npm distribution SHALL remain a configuration and artifact bundle.
- The npm lifecycle adapter SHALL NOT claim that the npm archive distributes a
  native Rust executable.
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
- The Rust manifest classifier SHALL parse both supplied manifests as YAML
  mappings and SHALL reject duplicate mapping keys or merge keys rather than
  inheriting language-specific merge behavior.
- The Rust manifest classifier SHALL validate the module manifest against the
  caller-supplied authoritative schema using its declared JSON Schema draft,
  offline reference resolution, and enabled known-format assertions.
- The Rust manifest classifier SHALL withhold acceptance when the manifest name
  or version differs from the explicit expected identity.
- The Rust manifest classifier SHALL require unique artifact-type names.
- The Rust manifest classifier SHALL require every declared allowed-link verb
  to exist in the supplied edge registry.
- The Rust manifest classifier SHALL require exactly one supplied resource for
  each declared artifact type.
- The Rust manifest classifier SHALL withhold for missing, duplicate, or
  undeclared resources.
- The Rust manifest classifier SHALL require every frontmatter schema reference
  to be a non-empty normalized forward-slash relative path no longer than 4,096
  bytes, without an absolute root, Windows drive root, backslash, empty, dot,
  parent, trailing-slash, or control-character segment.
- The Rust manifest classifier SHALL validate each supplied artifact schema
  against its declared JSON Schema meta-schema without network or filesystem
  retrieval.
- The Rust manifest classifier SHALL parse each skeleton's exact Markdown YAML
  frontmatter envelope, reject duplicate or merge keys, and validate the
  resulting instance against its declared artifact schema with known-format
  assertions enabled.
- The Rust manifest classifier SHALL require every declared body-extraction
  locator to carry `required: true` and SHALL require the skeleton to contain
  its exact level-two `## <after_heading>` heading.
- The Rust manifest classifier SHALL sort findings by closed category, safe
  artifact name, and safe schema reference so input ordering cannot change the
  result.
- The pure manifest boundary SHALL refuse any individual manifest, schema, or
  skeleton input larger than 1,048,576 bytes, more than 256 artifact resources,
  or more than 33,554,432 combined resource bytes before schema compilation.
- The pure manifest boundary SHALL return a typed error and no result for a
  resource-ceiling refusal.
- The pure manifest boundary SHALL NOT discover schemas, read files or
  environment variables, invoke Quire or another child program, access a
  network or clock, or treat an independently authored Rust structure as a
  replacement for validation against the authoritative manifest schema.
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
Oversized manifest inputs refuse the pure module-manifest request. Malformed
YAML or JSON, invalid schemas, unavailable offline references, manifest-schema
mismatches, identity drift, duplicate artifact names, unregistered edges,
resource mismatch, unsafe schema references, invalid skeleton frontmatter, and
missing required headings withhold module-manifest acceptance.
Missing, linked, special, oversized, excessive, pre-existing, changed, or
non-corresponding npm staging members refuse the package-lifecycle operation.
A preflight-refused staging or cleanup operation reports no successful outcome
and deletes no pre-existing destination.
An npm-hook invocation that writes to stdout fails package-manager integration.

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
| FR-017-AC-7 | For the retained valid module and every declared malformed-manifest, schema, registry, resource, frontmatter, and heading case within the declared ceilings, the pure Rust manifest classifier matches retained acceptance on the shared supported domain, fails closed through typed deterministic findings, consumes rather than copies authoritative schemas, and performs no filesystem, environment, child-program, network, or clock access. | Property (TC-121) |

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
- **Sequence**: FR-017-AC-7 is an independently reviewable pure manifest slice.
  It does not discover the shared schema, traverse a module tree, replace Quire
  artifact validation, select a distribution format, satisfy the combined
  FR-017-AC-3 gate, or permit removal under FR-018.
