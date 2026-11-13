---
id: FR-044
title: "Scan the Syft CycloneDX SBOM for known advisories with OSV-Scanner through one producer-execution request"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "extends"
  - target: "ix://agent-ix/engineering-assurance/FR-029"
    type: "references"
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
the report is read only by the quoin adapter [FR-149](ix://agent-ix/quoin/FR-149), so a later Rust scanner
replaces OSV-Scanner by changing this request shape and that adapter, and nothing else.

As with FR-038, the producer is a caller-supplied request, not a registry entry, and there is no request
builder: the shape is checked against the published request schema (`producer-execute --schema request`)
and this definition.

### Research status

Every statement about OSV-Scanner behaviour below was read from its documentation or source at release
`v2.7.1` (docs at `docs/*.md` and source at `cmd/osv-scanner/`, `internal/output/`, `pkg/models/` of
`github.com/google/osv-scanner`). No OSV-Scanner binary was run by the spec author. The integration tests
decide each UNVERIFIED row.

| # | Question | Status | Source and finding |
| --- | --- | --- | --- |
| 1 | CLI to scan a CycloneDX SBOM, JSON output | VERIFIED | `docs/scan-source.md`: `osv-scanner scan source -L <sbom>`; an SBOM is recognised by file name (`*.cdx.json`, `bom.json`). `--sbom` is deprecated in favour of `-L` (`cmd/osv-scanner/scan/source/command.go`). `docs/output.md`: `--format json`; `--output-file <path>` writes the result to a file (`internal/helper/flags.go`). |
| 2 | SBOM from stdin | UNVERIFIED | Neither `docs/scan-source.md` nor the `scan source` command source documents stdin for a scan; the scan target is a path. This definition therefore passes the retained snapshot as an FR-019 `input_artifact` argument (a descriptor path). Whether a descriptor path with no `.cdx.json` name is accepted as a CycloneDX SBOM is UNVERIFIED (see Inputs). |
| 3 | Offline mode | VERIFIED | `docs/offline-mode.md`: `--offline` scans against a previously downloaded local database, downloads nothing and fails with an error when no database is present. The directory is `OSV_SCANNER_LOCAL_DB_CACHE_DIRECTORY`, laid out `{dir}/osv-scalibr/<ecosystem>/all.zip`. Mirrors of `all.zip` come from `https://osv-vulnerabilities.storage.googleapis.com/<ECOSYSTEM>/all.zip`; the ecosystem list is `ecosystems.txt` in that bucket. `--download-offline-databases` downloads and is barred here. |
| 4 | JSON envelope and database identity | VERIFIED absent | `pkg/models/results.go`: the top-level object holds `results` (each `source` and `packages`), `experimental_config`, optional `image_metadata` and `license_summary`. No field names the database, the zip, or a database version or build time. Each package carries `package.name`, `version`, `ecosystem`, optional `purl`, and `vulnerabilities` (full OSV records) with `groups` of aliased ids. |
| 5 | Exit codes | VERIFIED | `docs/output.md` Return Codes: `0` packages found and no vulnerabilities; `1` packages found and vulnerabilities found; `127` general error; `128` no packages found; `129-255` reserved for non-result errors. A partial scan (some lookup failed but the run continued) is not distinguished from a complete one by any documented code or field: UNVERIFIED. |
| 5a | `--all-packages` lists clean packages in the JSON | UNVERIFIED | `docs/usage.md` states the flag outputs all packages in JSON, so a clean scan names the packages it read; the quoin adapter FR-149 refuses a report with no packages. Not run. |
| 6 | Configuration and ignore files | VERIFIED | `docs/configuration.md`: `osv-scanner.toml` in a scanned file's directory applies to it; `--config <file>` applies that one file to every scanned file and ignores every `osv-scanner.toml`. `.gitignore` handling applies to directory scans (`docs/scan-source.md`). |
| 7 | Syft purl carries `distro=` for OS packages | UNVERIFIED | `syft/pkg/cataloger/debian/package.go` builds a `deb` purl and passes the detected distro to `pkg.PURLQualifiers`; the helper that emits the `distro=` qualifier was not read, and whether OSV-Scanner maps an OS-package purl to a distro ecosystem from it was not read. OS-package coverage is UNVERIFIED. The report carries an optional `package.purl`. |

### Data terms

The advisory data OSV-Scanner reads is third-party data with its own terms. These are recorded as the data
terms this definition works to; they are not legal advice.

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
| `arguments` | In order and with no others: literal `scan`, literal `source`, literal `--offline`, literal `--all-packages`, literal `--format`, literal `json`, literal `--output-file`, literal `out/osv-scanner.json`, literal `--config`, literal `out/empty-config.toml`, literal `-L`, then `{"kind":"input_artifact","role":"sbom"}`. The file `out/empty-config.toml` is an empty file the caller writes before the run, so OSV-Scanner reads no `osv-scanner.toml` and no ignore rule. |
| `environment` | `OSV_SCANNER_LOCAL_DB_CACHE_DIRECTORY=<capabilityRoot>/db`, `HOME=<capabilityRoot>/out/home`, `XDG_CACHE_HOME=<capabilityRoot>/out/home`, `XDG_CONFIG_HOME=<capabilityRoot>/out/home`, `XDG_CONFIG_DIRS=<capabilityRoot>/out/xdg` and `TMPDIR=<capabilityRoot>/out/tmp`. The caller creates `out/home`, `out/xdg` and `out/tmp` empty before the run. No other variable is inherited (FR-019-CON-1). |
| `inputs` | One entry, `{"role":"sbom","path":"in/sbom.cdx.json"}`: the retained FR-038 `sbom` artifact placed there by the caller. |
| `stdin` | `{"kind": "null"}`. |
| `outputs` | One entry: role `advisory-report`, path `out/osv-scanner.json`, `required` true. |
| `containment` | `{"profile": "process-group-v1", "contract": {"kind": "engineering-assurance.osv-scanner-containment", "version": "1"}}`. |
| `cancellation` | The caller's choice of `{"kind":"disabled"}` or an exact `event` binding. |
| `budget` | All eight bounds present and within the FR-019 maxima; the caller chooses the values. |
| `response` | The built-in `engineering-assurance.process-evidence` version `1` protocol and `engineering-assurance.cli-process-evidence-adapter` version `1` adapter, with an exit policy admitting exit codes 0 and 1 only. |

The `input_artifact` argument is the FR-019 descriptor path of the immutable snapshot of the `sbom` input, so
OSV-Scanner reads the exact bytes the executor retained and the scan is bound to the SBOM it read (FR-019).
OSV-Scanner recognises an SBOM by file name, and the descriptor path has none (research row 2). This was not
run by the spec author; the integration test of FR-044-AC-1 decides it. If it fails to recognise the
descriptor path, this definition is amended to pass the literal path `in/sbom.cdx.json` in place of the
`input_artifact` argument; the retained-snapshot binding of FR-044-AC-11 then falls back to the FR-019 check
that a declared input changed after preflight is refused, and FR-044-AC-11 is amended to match.

The `db/` directory is a per-run mirror the caller placed before the run: `db/osv-scalibr/<ecosystem>/all.zip`
for each ecosystem the SBOM holds. It is not declared as an input, because a retained input snapshot is one
file. The run makes no network connection. The report does not state the database's build time or version
(research row 4), so the report cannot show how current the advisories were; whether a report is recent
enough stays with the consumer's gate policy.

## Fixtures

The FR-039 fixtures are fictional and pin no vulnerable package, so they cannot show a match. This
requirement therefore adds its own fixtures under `scan/`, each pinning one package version that the mirror
named by `EA_TEST_OSV_DB` holds a long-published advisory for, with that advisory id and package purl written
in an expected-result file beside the fixture: an npm package with a lock file, a Python project with a pinned
requirements file, and a Go module with `go.mod` and `go.sum`. A container image archive fixture holding an
`os-release` and one pinned OS package is added only when the integration test shows OSV-Scanner matches the
Syft OS-package purl (research row 7); until then OS-package coverage is stated as not covered. Their SBOMs
are the retained Syft artifacts of FR-038 runs over them. The quoin criteria FR-142-AC-19 and FR-149-AC-10
rely on the reports captured from these fixtures.

## Outputs

- The FR-029 result line for the request. In the `completed` state it carries one retained output
  artifact, role `advisory-report`, whose bytes are one OSV-Scanner JSON document.
- No second SBOM is produced, and OSV-Scanner is never asked for an SBOM output format.
- Database identity: if the scanner exposes the database, zip or version it used, the retained evidence SHALL
  record it. OSV-Scanner `v2.7.1` exposes none (research row 4), so nothing is recorded and the consumer
  SHALL treat advisory currency as not assessed. This status is UNVERIFIED for later releases.

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
| FR-044-AC-6 | The `arguments` of the request used by the tests equal the list in Inputs element for element, with none of the options barred in Behavior, and its `environment`, `inputs`, `stdin`, `containment` and exit policy equal the definition. | Inspection |
| FR-044-AC-7 | A stale file left at the report output path before a run that fails is absent from the failed result. | Test |
| FR-044-AC-8 | The run makes no network connection: with networking unavailable to the scanner, the OSV-Scanner run over the fixture SBOM and mirror still completes. | Test |
| FR-044-AC-9 | `make osv-scanner-integration` with `EA_TEST_OSV_SCANNER` or `EA_TEST_OSV_DB` unset, naming a path that is not an executable regular file (`EA_TEST_OSV_SCANNER`) or not a directory (`EA_TEST_OSV_DB`), exits non-zero with a message naming the variable and runs no test. | Test |
| FR-044-AC-10 | Design review confirms Engineering Assurance contains no OSV-Scanner report reader, no second SBOM tool and no launch path other than `producer-execute` in production code; test code that checks a fixture report is the only reader. | Inspection |
| FR-044-AC-11 | With the `sbom` input file replaced on disk after the executor retained its snapshot, the report still reflects the retained bytes, because the scanner read the descriptor snapshot and not the path. | Test |
| FR-044-AC-12 | The documentation of this requirement records the data terms of Data terms (CC-BY-4.0 attribution, CC-BY-SA for Ubuntu and Alpine, Wolfi excluded from redistributable mirrors), and a test over the fixture mirror fails if its directory holds a Wolfi ecosystem. | Test |
| FR-044-AC-13 | The Research status table states VERIFIED or UNVERIFIED for each of its seven questions, and the integration run records the outcome of each UNVERIFIED row (descriptor-path acceptance, partial-scan signal, OS-package purl matching). | Inspection |

## Dependencies

- **Upstream**: [FR-019](./FR-019-bounded-producer-execution.md), [FR-029](./FR-029-producer-execute-command.md), [FR-038](./FR-038-syft-sbom-producer-request.md) and [FR-039](./FR-039-syft-sbom-evidence-ingestion.md) for the SBOM it reads.
- **Downstream**: `ix://agent-ix/quoin/FR-149` ingests the report; `ix://agent-ix/quoin/FR-142` joins it onto SOUP records.
