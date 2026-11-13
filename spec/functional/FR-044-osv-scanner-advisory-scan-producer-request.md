---
id: FR-044
title: "Scan the Syft CycloneDX SBOM for known advisories with OSV-Scanner through one producer-execution request"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/StR-002"
    type: "traces_to"
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "extends"
  - target: "ix://agent-ix/engineering-assurance/FR-029"
    type: "depends_on"
  - target: "ix://agent-ix/engineering-assurance/FR-038"
    type: "depends_on"
  - target: "ix://agent-ix/engineering-assurance/FR-039"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-149"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-142"
    type: "references"
---

# FR-044: Scan the Syft CycloneDX SBOM for known advisories with OSV-Scanner through one producer-execution request

## Description

Engineering Assurance SHALL define the advisory-scan producer as one request shape of the FR-019
`engineering-assurance.producer-execution-request/v1` protocol, run by the FR-029 `producer-execute`
command. The scanner is **OSV-Scanner** (`google/osv-scanner`, Go, Apache-2.0), chosen by the owner. It reads
the CycloneDX SBOM retained by a `completed` FR-038 run and produces a JSON report. Syft stays the only tool
that generates an SBOM (NFR-006). The request names the retained SBOM as the one scan target and no
directory, so OSV-Scanner is never given a source tree or image.

OSV-Scanner is a replaceable tool. Engineering Assurance holds no OSV-Scanner report reader (FR-044-AC-10);
the report is read only by the quoin adapter [FR-149](ix://agent-ix/quoin/FR-149), so a later scanner
replaces OSV-Scanner by changing this request shape and that adapter, and nothing else.

As with FR-038, the producer is a caller-supplied request, not a registry entry, and there is no request
builder: the shape is checked against the published request schema (`producer-execute --schema request`)
and this definition.

### Research status

The documented behavior below was reviewed from the OSV-Scanner project documentation and source. A fixture-only
runtime check confirmed that the retained SBOM must be passed under a recognized `.cdx.json` filename and that
`-L` is accepted for a named CycloneDX SBOM; this check did not use a vulnerability database or verify advisory
matching. The Rust integration tests decide each UNVERIFIED row using caller-provided fixtures and database.

| # | Question | Status | Source and finding |
| --- | --- | --- | --- |
| 1 | CLI to scan a CycloneDX SBOM, JSON output | VERIFIED | `docs/scan-source.md`: `osv-scanner scan source -L <sbom>`; an SBOM is recognised by file name (`*.cdx.json`, `bom.json`). `--sbom` is deprecated in favour of `-L` (`cmd/osv-scanner/scan/source/command.go`). `docs/output.md`: `--format json`; `--output-file <path>` writes the result to a file (`internal/helper/flags.go`). |
| 2 | SBOM from stdin | UNVERIFIED | The scan target is a path. A fixture-only runtime check rejected a filename-less input path and accepted a named `.cdx.json` SBOM with `-L`; this definition therefore passes the retained SBOM at `in/sbom.cdx.json`. Stdin behavior remains unverified and is not used. |
| 3 | Offline mode | VERIFIED | `docs/offline-mode.md`: `--offline` scans against a previously downloaded local database, downloads nothing and fails with an error when no database is present. The directory is `OSV_SCANNER_LOCAL_DB_CACHE_DIRECTORY`, laid out `{dir}/osv-scalibr/<ecosystem>/all.zip`. Mirrors of `all.zip` come from the OSV bucket path `<ECOSYSTEM>/all.zip` named in `docs/offline-mode.md`; the ecosystem list is `ecosystems.txt` in that bucket. `--download-offline-databases` downloads and is barred here. |
| 4 | JSON envelope and database identity | VERIFIED absent | `pkg/models/results.go`: the top-level object holds `results` (each `source` and `packages`), `experimental_config`, optional `image_metadata` and `license_summary`. No field names the database, the zip, or a database version or build time. Each package carries `package.name`, `version`, `ecosystem`, optional `purl`, and `vulnerabilities` (full OSV records) with `groups` of aliased ids. |
| 5 | Partial lookup failure signal | UNVERIFIED | `docs/output.md` Return Codes: `0` packages found and no vulnerabilities; `1` packages found and vulnerabilities found; `127` general error; `128` no packages found; `129-255` reserved for non-result errors. Whether a partial lookup failure is signaled is UNVERIFIED. The requirement fails closed whenever completion cannot be established; it does not treat an incomplete or unverified scan as clean. |
| 5a | `--all-packages` lists clean packages in the JSON | UNVERIFIED | `docs/usage.md` states the flag outputs all packages in JSON, so a clean scan names the packages it read; the quoin adapter FR-149 refuses a report with no packages. Not run. |
| 6 | Configuration and ignore files | VERIFIED | `docs/configuration.md`: `osv-scanner.toml` in a scanned file's directory applies to it; `--config <file>` applies that one file to every scanned file and ignores every `osv-scanner.toml`. `.gitignore` handling applies to directory scans (`docs/scan-source.md`). |
| 7 | Syft OS-package purl and OSV advisory match | UNVERIFIED | A fixture-only Syft run emitted `pkg:deb/debian/libc6@2.36-9%2Bdeb12u4?arch=amd64&distro=debian-12`. Whether OSV-Scanner matches an advisory to this purl is UNVERIFIED; the available local database did not permit a scan. The Rust integration test SHALL include a container OS-package fixture and assert the expected advisory match using its supplied fixture database. The runtime check did not verify vulnerability matching. |

### Data terms

The advisory data OSV-Scanner reads is third-party data with its own terms. These are recorded as the data
terms this definition works to.

| Source | Term | Consequence here |
| --- | --- | --- |
| GitHub Security Advisories, PyPI advisories, Go vulnerability database | CC-BY-4.0, attribution required | A retained advisory report SHALL carry attribution to the advisory source of each record it keeps, which the OSV record's `id` prefix and `database_specific` fields name. |
| Ubuntu, Alpine advisories | CC-BY-SA | A retained report holding such records is share-alike for those records. |
| Wolfi advisories | non-commercial | Wolfi data SHALL NOT be in a database directory whose retained records can be redistributed. The caller does not place a Wolfi mirror in `db/`. |

## Inputs

The OSV-Scanner producer definition fixes these fields of the request schema.

| Request field | OSV-Scanner definition |
| --- | --- |
| `protocol` | `engineering-assurance.producer-execution-request/v1`. |
| `producer` | `name` `osv-scanner`, `version` the exact OSV-Scanner release being run, `executable` the absolute path of that binary. |
| `caller` | `{"kind": "engineering-assurance.advisory-scan", "version": "1"}`. |
| `procedure` | `direct`. |
| `capabilityRoot` | An absolute directory holding `in/` (the SBOM), `db/` (the vulnerability database mirror) and `out/` (the report and every home and cache directory). |
| `arguments` | In order and with no others: literal `scan`, literal `source`, literal `--offline`, literal `--all-packages`, literal `--format`, literal `json`, literal `--output-file`, literal `out/osv-scanner.json`, literal `--config`, literal `out/empty-config.toml`, literal `-L`, then literal `in/sbom.cdx.json`. The file `out/empty-config.toml` is an empty file the caller writes before the run, so OSV-Scanner reads no `osv-scanner.toml` and no ignore rule. |
| `environment` | `OSV_SCANNER_LOCAL_DB_CACHE_DIRECTORY=<capabilityRoot>/db`, `HOME=<capabilityRoot>/out/home`, `XDG_CACHE_HOME=<capabilityRoot>/out/home`, `XDG_CONFIG_HOME=<capabilityRoot>/out/home`, `XDG_CONFIG_DIRS=<capabilityRoot>/out/xdg` and `TMPDIR=<capabilityRoot>/out/tmp`. The caller creates `out/home`, `out/xdg` and `out/tmp` empty before the run. No other variable is inherited (FR-019-CON-1). |
| `inputs` | One entry, `{"role":"sbom","path":"in/sbom.cdx.json"}`: the retained FR-038 `sbom` artifact copied there by the caller, preserving this `.cdx.json` filename. |
| `stdin` | `{"kind": "null"}`. |
| `outputs` | One entry: role `advisory-report`, path `out/osv-scanner.json`, `required` true. |
| `containment` | `{"profile": "process-group-v1", "contract": {"kind": "engineering-assurance.osv-scanner-containment", "version": "1"}}`. |
| `cancellation` | The caller's choice of `{"kind":"disabled"}` or an exact `event` binding. |
| `budget` | All eight bounds present and within the FR-019 maxima; the caller chooses the values. |
| `response` | The built-in `engineering-assurance.process-evidence` version `1` protocol and `engineering-assurance.cli-process-evidence-adapter` version `1` adapter, with an exit policy admitting exit codes 0 and 1 only. |

The caller places the immutable FR-019 retained snapshot at `in/sbom.cdx.json`; the executor verifies the
declared input against the retained bytes before launch. OSV-Scanner receives this named path so its SBOM
classification can use the `.cdx.json` suffix (research row 2). The fixture-only runtime check established
path recognition only, not vulnerability matching or report completeness.

The `db/` directory is a per-run mirror the caller placed before the run: `db/osv-scalibr/<ecosystem>/all.zip`
for each ecosystem the SBOM holds. It is not declared as an input, because a retained input snapshot is one
file. The run makes no network connection. The report does not state the database's build time or version
(research row 4), so the report cannot show how current the advisories were; whether a report is recent
enough stays with the consumer's gate policy.

## Fixtures

The FR-039 fixtures are fictional and do not establish advisory matches. Rust integration tests in the existing
`tests/` harness SHALL invoke the producer against the retained SBOM and supplied local mirror, and separately
test the report contract with authentic captured OSV-Scanner outputs. This requirement adds fixture inputs under
`scan/` for npm, Python, Go, and a container image with an `os-release` and an OS package. The container fixture
is required; OS-package coverage is not downgraded or marked unsupported. Each expected match is fixed by the
fixture test contract, while fixture-only tool runs are not compliance evidence and do not discharge any
assurance obligation. No captured scanner output is fabricated; the implementation adds captures only after an
actual scanner run. The quoin criteria FR-142-AC-19 and FR-149-AC-10 consume authentic reports from these
producer fixtures.

## Outputs

- The FR-029 result line for the request. In the `completed` state it carries one retained output
  artifact, role `advisory-report`, whose bytes are one OSV-Scanner JSON document.
- No second SBOM is produced, and OSV-Scanner is never asked for an SBOM output format.
- Database identity: if the scanner exposes the database, zip or version it used, the retained evidence SHALL
  record it. The reviewed report shape exposes none (research row 4), so the consumer SHALL treat advisory
  currency as not assessed. Runtime-observed report fields are recorded without pinning a tool release.

## Behavior

- The request SHALL run through `producer-execute` or the FR-019 library boundary and through no other
  launch path.
- The request SHALL carry exactly the `arguments` listed in Inputs, and no other.
- The request SHALL name no directory, container image or lockfile as a scan target, so OSV-Scanner generates
  no SBOM from a tree or image.
- The request SHALL NOT carry `--download-offline-databases`, `--offline-vulnerabilities`, `--allow-no-lockfiles`,
  `--call-analysis`, `--licenses`, `--serve`, `--x-exclude` or `--all-vulns`. A scan that downloads, hides,
  or turns "no packages found" into success is not a scan whose silence can be read as clean.
- If the OSV-Scanner executable cannot be opened or launched, then the result state SHALL be `unavailable`.
- If OSV-Scanner exits with a code other than 0 or 1 (including 127, general error, and 128, no packages
  found), is killed by a signal, or exceeds a stream bound, then the result state SHALL be `failed`, and no
  report is ingested. A missing local database makes OSV-Scanner report an error (research row 3) and is
  therefore `failed`. Exit 1 means vulnerabilities were found and is `completed`.
- If the scanner signals a partial lookup, or the executor or report cannot establish that all declared package
  lookups completed, then the result state SHALL be `failed`; no partial or unverified report is ingested or
  interpreted as clean. A successful exit code alone SHALL NOT override an incomplete-scan signal.
- If OSV-Scanner does not finish within `budget.timeoutMillis`, then the result state SHALL be `timed_out`.
- If the required `advisory-report` output is absent after a normal exit, then the result state SHALL be
  `failed`.
- If any state other than `completed` is returned, then the consumer SHALL NOT read a report from the
  result. A stale file at the output path is removed before launch by FR-019.
- A retained advisory report SHALL carry the data-term attribution of Data terms.
- The caller SHALL place no Wolfi data in `db/` when the retained report can be redistributed.
- The definition SHALL NOT add a closed result state, a request field, a Cargo dependency or a Cargo
  feature.
- The definition SHALL NOT interpret the report (FR-019-CON-2).
- Ingestion is `quoin evidence record` through the adapter of `ix://agent-ix/quoin/FR-149`, as FR-039
  does for the SBOM.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-044-CON-1 | If `EA_TEST_OSV_SCANNER`, the absolute path of the OSV-Scanner binary, is unset or does not name an executable regular file, or `EA_TEST_OSV_DB`, the absolute path of a directory laid out as `osv-scalibr/<ecosystem>/all.zip`, is unset or does not name a directory, then `make osv-scanner-integration` SHALL exit non-zero with a message naming the variable. A default `cargo test` does not run OSV-Scanner-dependent tests, and the repository holds neither OSV-Scanner nor a database. | Test integrity | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-044-AC-1 | A request meeting the OSV-Scanner producer definition over the retained SBOM of the npm fixture of this requirement validates against the published request schema, is accepted by `producer-execute`, yields state `completed`, exit status 1, and one retained `advisory-report` artifact that parses as a JSON object whose `results` holds a package with a vulnerability whose id (or alias) and package purl or name and version equal those in the fixture's expected-result file. The test fails if `results` holds no vulnerability. | Test |
| FR-044-AC-2 | The same request shape over the retained SBOMs of the Python and Go module fixtures each yields `completed` and a report holding at least one vulnerability equal to that fixture's expected id and package; every reported package names a component of that SBOM. The test fails if no vulnerability is reported. | Test |
| FR-044-AC-3 | A request naming an OSV-Scanner executable path that does not exist yields state `unavailable` and no report artifact. | Test |
| FR-044-AC-4 | An OSV-Scanner fixture executable that exits 127 or 128 yields `failed`, one that exits 1 yields `completed`, and one that outlives a one-second timeout yields `timed_out`; a `failed` or `timed_out` result carries no report for ingestion. | Test |
| FR-044-AC-5 | A run whose `db/` is empty or holds no database yields `failed` and no report, so a missing database is never a clean scan. | Test |
| FR-044-AC-6 | Rust tests in the existing `tests/` harness assert the producer request's `arguments` equal the list in Inputs element for element, with none of the options barred in Behavior, and its `environment`, `inputs`, `stdin`, `containment` and exit policy equal the definition. | Test |
| FR-044-AC-7 | A stale file left at the report output path before a run that fails is absent from the failed result. | Test |
| FR-044-AC-8 | The run makes no network connection: with networking unavailable to the scanner, the OSV-Scanner run over the fixture SBOM and mirror still completes. | Test |
| FR-044-AC-9 | `make osv-scanner-integration` with `EA_TEST_OSV_SCANNER` or `EA_TEST_OSV_DB` unset, naming a path that is not an executable regular file (`EA_TEST_OSV_SCANNER`) or not a directory (`EA_TEST_OSV_DB`), exits non-zero with a message naming the variable and runs no test. | Test |
| FR-044-AC-10 | Design review confirms Engineering Assurance contains no OSV-Scanner report reader, no second SBOM tool and no launch path other than `producer-execute` in production code; test code that checks a fixture report is the only reader. | Inspection |
| FR-044-AC-11 | Rust tests in the existing `tests/` harness assert the scanner is passed the retained input at `in/sbom.cdx.json`; replacing the source artifact after retention does not change the bytes staged at that path. | Test |
| FR-044-AC-12 | The documentation of this requirement records the data terms of Data terms (CC-BY-4.0 attribution, CC-BY-SA for Ubuntu and Alpine, Wolfi excluded from redistributable mirrors), and a test over the fixture mirror fails if its directory holds a Wolfi ecosystem. | Test |
| FR-044-AC-13 | The Research status table states VERIFIED or UNVERIFIED for each of its seven questions. Rust integration tests in the existing `tests/` harness exercise producer invocation, incomplete-scan refusal, captured-report parsing, and a container OS-package fixture; they record outcomes without treating fixture results as compliance evidence. No test may infer a clean result from an incomplete or unverified scan. | Test |
| FR-044-AC-14 | A Rust integration test in the existing `tests/` harness scans the retained SBOM from the container OS-package fixture with the supplied local database and asserts that the expected fixture advisory is reported for the matching OS-package purl; the test does not use a live database or live project SBOM. | Test |

## Dependencies

- **Upstream**: [FR-019](./FR-019-bounded-producer-execution.md), [FR-029](./FR-029-producer-execute-command.md), [FR-038](./FR-038-syft-sbom-producer-request.md) and [FR-039](./FR-039-syft-sbom-evidence-ingestion.md) for the SBOM it reads.
- **Downstream**: `ix://agent-ix/quoin/FR-149` ingests the report; `ix://agent-ix/quoin/FR-142` joins it onto SOUP records.
