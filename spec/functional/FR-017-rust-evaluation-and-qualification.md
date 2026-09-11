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
- Versioned `cli-agent-evals.report/v1` results, retained transcripts, one
  explicit evaluation-workspace root, and one expected immutable source
  revision.
- Package manifests, staged archives, rights policy, and integration evidence.
- For the npm package-lifecycle adapter, one explicit repository root and one
  closed operation: `stage`, `clean`, or `refuse-publication`.
- For the content-rights tree adapter, one explicit repository root and the
  optional newline-delimited `ASSURANCE_PROTECTED_TOKENS` environment value.
- For the pure package-membership boundary, one explicit expected-member
  allowlist and one observed file-member sequence supplied by an archive or
  staging adapter.
- For the package-audit adapter, one explicit repository root containing the
  retained Python-wheel and private-npm distribution definitions. The adapter
  invokes the fixed local `python3`, `npm`, and package-installer commands; it
  does not accept caller-selected executable names or command fragments.
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
- One `engineering-assurance.content-rights-tree-result/v1` result containing
  the accepted/withheld outcome, inspected regular-file and symbolic-link
  count, and deterministic typed findings from the pure classifier.
- One `engineering-assurance.package-audit-result/v1` success result containing
  the accepted outcome and the audited wheel, npm, and installed-canonical file
  counts. Package, archive, process, rights, membership, or installed-bundle
  failures produce a typed adapter error and no success result.
- The pure aggregation boundary emits one
  `engineering-assurance.evaluation-aggregate-result/v1` value containing the
  pass decision, required-cell count, complete-cell count, and a deterministic
  ordered list of stable failure codes. It does not persist evidence or infer a
  release decision.
- The report adapter emits the retained `evaluation-aggregate-v1` artifact
  shape, including exact source revision, selected report identities, host
  models, failed-attempt diagnostics, complete-cell counts, and aggregate
  failures. It does not infer a terminal or release decision.
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
- Decode each cli-agent-evals report through closed Rust structures. Require
  the exact `cli-agent-evals.report/v1` discriminator and reject unknown fields
  at the report, result, sample, token-usage, and Engineering Assurance result
  levels. Treat cli-agent-evals metric maps, aggregate maps, and non-EA check
  values as host-owned observations rather than reinterpreting them.
- Require each report to name one supported host, exactly one repeat, and no
  more than 256 results. Require each result selected for aggregation to name
  one supported scenario and exactly one sample. Preserve a missing model as
  the explicit `runner-default` selection, and reject multiple model values for
  one host across the selected report collection.
- Require the report outcome, each result outcome and `1/1` or `0/1` pass rate,
  and its single sample outcome to agree before the report contributes an
  observation.
- Preserve failed samples as bounded diagnostics without admitting an
  envelope. Admit a successful sample only when its exit is `complete`, its
  retention state is `retained`, its lowercase SHA-256 digest and normalized
  forward-slash relative transcript path are present, and its typed
  `evaluation_result` observation is complete.
- The report host adapter SHALL accept no more than 64 explicit
  repository-relative report paths beneath one canonical repository root.
  Each report SHALL be a non-symbolic-link regular file no larger than
  8,388,608 bytes.
- The report host adapter SHALL accept one canonical non-symbolic-link
  evaluation-workspace root.
- Every reported work directory and transcript SHALL resolve beneath the
  evaluation-workspace root without following a symbolic link or reading a
  special file.
- A transcript SHALL be no larger than 67,108,864 bytes.
- The report host adapter SHALL hash the exact retained transcript bytes and
  reject a missing, changed, linked, special, escaping, or oversized
  transcript before constructing an evaluation envelope.
- The report adapter SHALL compare every result source revision with the
  caller-supplied immutable expected revision before aggregation.
- The report adapter SHALL sort report identities, models, failed attempts,
  and adapter diagnostics so input path order cannot change the retained
  aggregate artifact.
- The `agent-evals-aggregate` repository command SHALL invoke the Rust CLI
  after same-revision parity with the retained Python loader and script. The
  cutover SHALL be reversible before the replaced Python paths are deleted.
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
- The content-rights tree adapter SHALL accept only an explicit existing
  non-symbolic-link repository root whose canonical path equals the Git
  worktree top level reported for that root.
- The content-rights tree adapter SHALL invoke `git` directly without a shell
  and SHALL select exactly the NUL-delimited population returned by `git
  ls-files -z --cached --others --exclude-standard` at that root.
- The content-rights tree adapter SHALL ignore selected directories, including
  gitlink roots, and SHALL inspect every selected regular file and symbolic
  link; a missing or special entry SHALL fail closed rather than silently
  reducing the population.
- The content-rights tree adapter SHALL reject an unavailable, timed-out, or
  unsuccessful Git process, malformed or non-UTF-8 Git output, duplicate
  selected paths, more than 65,536 selected entries, a selected path longer
  than 4,096 bytes, a regular file larger than 8,388,608 bytes, or more than
  67,108,864 inspected regular-file bytes.
- The content-rights tree adapter SHALL limit each Git response stream to
  8,388,608 bytes.
- The content-rights tree adapter SHALL terminate each Git command within 60
  seconds.
- When a resource limit is exceeded, the content-rights tree adapter SHALL
  return no tree result without reading a path outside the selected root.
- `ASSURANCE_PROTECTED_TOKENS` SHALL be read only by the binary adapter. The
  adapter SHALL reject a non-UTF-8 value, more than 65,536 encoded bytes, more
  than 256 nonblank newline-delimited tokens, or a token longer than 4,096
  bytes before inspecting repository content.
- The tree result SHALL sort and deduplicate findings by safe path, line, and
  closed category; an invalid path finding SHALL carry no path, and no result
  or diagnostic SHALL contain source text or a protected-token value.
- A direct content-rights tree invocation SHALL emit exactly one versioned JSON
  result on stdout, return status zero only for an accepted tree, return status
  one for a well-formed withheld result, and reserve status two for a typed
  adapter or rendering failure.
- The repository test dispatch SHALL invoke the Rust content-rights tree
  command declaratively. Until the Rust and retained paths agree at the same
  candidate revision on the complete repository and the governed positive,
  negative, Git-population, environment, path, file-kind, and resource cases,
  the repository SHALL retain the Python tree checker and its Python tests.
- When that correspondence and a reversible dispatch cutover are recorded, the
  repository SHALL move the direct tree gate to Rust while retaining the
  Python checker and its parity tests until no package/archive audit imports
  its classification functions.
- When the final package/archive consumer has moved to Rust, the repository SHALL
  remove the retained Python checker, its Python tests, and their temporary
  semantic-policy exemptions together without changing the inert corpus or
  historical evidence bytes.
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
- The package-audit adapter SHALL require an explicit existing non-symbolic-link
  repository root.
- When the complete `engineering_assurance` source tree, `pyproject.toml`,
  `setup.cfg`, or another fixed root/plugin/pilot package source contains a
  missing, linked, special, unsafe, or over-limit entry, the package-audit
  adapter SHALL reject the root before invoking a builder.
- The package-audit adapter SHALL NOT treat a later archive observation as
  retroactive authorization for a source the builder already followed.
- The package-audit adapter SHALL create all package-builder outputs,
  package-manager cache entries, archives, and installation outputs beneath one
  invocation-owned temporary directory.
- The package-audit adapter SHALL treat compiler caches selected by the
  reviewed Rust lifecycle dispatch as tool-owned caches rather than package-
  audit artifacts.
- The package-audit adapter SHALL require the six fixed npm lifecycle staging
  destinations to be absent before invoking `npm pack`.
- While `npm pack` executes, the package-audit adapter SHALL admit only the six fixed
  transient staging destinations created by the reviewed Rust package-
  lifecycle hook.
- When a successful or failed `npm pack` process terminates, the package-audit adapter SHALL remove only an exact staging population whose pre-invocation absence it established.
- When exact post-process staging cleanup cannot be proven, the package-audit adapter SHALL fail closed.
- The package-audit adapter SHALL invoke child programs directly without a
  shell.
- The package-audit adapter SHALL build exactly one wheel with `python3 -m pip
  wheel . --no-deps --no-build-isolation` and install that wheel with `python3
  -m pip install --no-index --no-deps --target`.
- The package-audit adapter SHALL build exactly one npm archive with `npm pack
  --json --pack-destination` and install that archive with `npm install
  --ignore-scripts --offline --prefix` and an invocation-owned npm cache.
- The package-audit adapter SHALL direct each package manager's cache and
  temporary workspace beneath the invocation-owned temporary directory.
- Each package-audit child process and every descendant retaining its process
  or output-stream resources SHALL terminate within 180 seconds.
- The package-audit adapter SHALL isolate the direct child in an
  invocation-owned process group and
  terminate that group before joining captured output on timeout or after the
  direct child exits.
- When the host cannot provide invocation-owned process-group isolation and
  descendant termination, the package-audit adapter SHALL fail before spawning
  a package-manager child.
- Each package-audit child process SHALL produce no more than 8,388,608 bytes
  on either stdout or stderr.
- The package-audit adapter SHALL fail an unavailable, timed-out, unsuccessful,
  malformed, or over-output process without interpreting diagnostic prose.
- The package-audit adapter SHALL select exactly one `.whl` and one `.tgz`
  output.
- Each selected archive SHALL be a regular file no larger than 67,108,864
  bytes.
- For each archive, the package-audit adapter SHALL accept no more than 65,536
  total entries, an individual expanded regular file no larger than 8,388,608
  bytes, or more than 67,108,864 aggregate expanded regular-file bytes.
- The package-audit adapter SHALL reject a non-UTF-8, absolute,
  backslash-delimited, empty-segment, dot-segment, parent-traversing,
  control-character, over-4,096-byte, or duplicate file-member name.
- The package-audit adapter SHALL reject symbolic-link, hard-link, device,
  FIFO, socket, tar extension-header, or otherwise unsupported archive members.
- The package-audit adapter SHALL iterate npm tar members without extension-
  header preprocessing so that every raw header is subject to the entry and
  kind ceilings before decoder allocation or path substitution.
- The package-audit adapter SHALL skip a directory entry only after validating
  its name and kind and confirming that its declared payload size is zero.
- The wheel file-member population SHALL equal the retained package-root,
  fixed data-file, and fixed distribution-metadata contract.
- The wheel SHALL contain exactly one of the admitted wheel license locations.
- The emitted wheel-license bytes SHALL equal the repository `LICENSE`.
- The emitted wheel `METADATA` SHALL retain the private-package refusal
  classifier.
- The npm pack report and decoded npm archive SHALL independently equal the
  same repository-owned npm allowlist after removal of exactly one `package/`
  archive prefix.
- The package-audit adapter SHALL construct the npm allowlist only from the
  fixed root files and declared package subtrees.
- The package-audit adapter SHALL NOT enlarge an allowlist from an archive
  observation.
- Every regular archive member SHALL pass the pure Rust content-rights policy.
- A member whose final path component is `LICENSE` SHALL use the license policy
  identity.
- Every other archive member SHALL use its validated normalized package path
  as the content-rights policy identity.
- Package-audit protected-token input SHALL use the same bounded environment
  parser as the repository-tree adapter.
- A package-audit failure SHALL NOT disclose source bytes or a protected token.
- Each offline installed bundle SHALL contain the retained module roots,
  exactly four supported thin host manifests resolving the one canonical
  assurance-onboarding skill, and exactly the four declared canonical and pilot
  workflows.
- Installed host-manifest validation SHALL reject unknown keys, missing or
  multiple skill sources, noncanonical targets, and escaping paths.
- Installed canonical and pilot definitions for each workflow SHALL parse as
  equivalent YAML values without accepting duplicate or merge keys.
- Installed-tree traversal SHALL use the same member-path, entry-population,
  per-file, and aggregate-byte ceilings as archive inspection.
- Installed-tree traversal SHALL reject symbolic links or special entries
  instead of following them.
- The canonical skill file population and bytes installed from the wheel and
  npm archive SHALL be identical.
- A direct package-audit invocation SHALL emit exactly one versioned JSON
  success result on stdout and return status zero only after both archives and
  both installations pass.
- A typed package-audit adapter failure SHALL return status two and write no
  success result.
- At package-audit return, the package-audit adapter SHALL leave no invocation-
  created distribution, package-manager cache, package-manager temporary,
  installation, or npm-staging output outside its invocation-owned temporary
  directory; the separately declared tool-owned compiler cache is not such an
  output and this rule does not claim operating-system sandboxing of external
  package managers.
- The adapter SHALL compare the selected repository's top-level entry
  population before and after each package-manager invocation, after exact npm
  staging cleanup where applicable, and SHALL fail closed if a process creates,
  removes, or changes the kind of any top-level entry. A population larger than
  4,096 entries or an unreadable entry kind SHALL fail before invocation.
- The repository `package-audit` target SHALL invoke the exact Rust 1.98.1 CLI
  declaratively.
- When the retained Python audit becomes eligible for deletion, the repository SHALL demonstrate that the Python and Rust paths both pass at one candidate revision.
- When the retained Python audit becomes eligible for deletion, the repository SHALL record a reversible dispatch cutover.
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
An invalid repository root, unavailable or invalid Git response, unstable or
unsupported selected entry, invalid protected-token population, or exceeded
tree resource ceiling refuses the content-rights tree operation without a
success result. A classified content-rights finding returns a well-formed
withheld result rather than an adapter error.
An unknown cli-agent-evals report version, malformed or open report shape,
unsupported host or scenario, invalid sample population, non-retained success,
model drift, unsafe path, transcript identity failure, or report/transcript
resource breach withholds the report collection without admitting an envelope
from the affected sample.
An invalid package-audit root, package-builder or installer failure, malformed
npm report, missing or multiple archive, unsafe or unsupported archive member,
resource-limit breach, membership mismatch, content-rights finding, installed
discovery mismatch, link or special installed entry, or cross-format canonical
byte mismatch refuses the package audit without a success result.

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
| FR-017-AC-3 | Rust package, rights, manifest, integration, and publication-refusal checks match the retained pass/fail corpus; the package-audit adapter builds, bounds, decodes, audits, installs, and compares the retained wheel and npm distributions offline and rejects extra, missing, duplicate, unsafe, linked, special, oversized, rights-denied, escaping, or byte-divergent members. | Property (TC-111) |
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
- **Sequence**: the package-audit adapter consumes the completed content-rights,
  membership, package-lifecycle, and manifest/discovery seams. It may cut over
  only after same-revision correspondence and rollback evidence. Removal of the
  retained package audit, content-rights checker, their Python tests, temporary
  policy exemptions, and Rust-test subprocess references is the final step.
- **Sequence**: the Rust report/transcript adapter and aggregate-command
  cutover are independently reviewable under TC-129. They do not satisfy the
  live 28-cell TC-109 gate, provide the still-missing external scenario
  provider, authorize a token-bearing agent run, or permit aggregate legacy
  removal before same-revision parity and rollback evidence.
