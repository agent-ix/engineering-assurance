---
id: FR-044
title: "Scan the Syft CycloneDX SBOM for known advisories with Grype through one producer-execution request"
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

# FR-044: Scan the Syft CycloneDX SBOM for known advisories with Grype through one producer-execution request

## Description

Engineering Assurance SHALL define the advisory-scan producer as one request shape of the FR-019
`engineering-assurance.producer-execution-request/v1` protocol, run by the FR-029 `producer-execute`
command. The scanner is **Grype**. It reads the CycloneDX SBOM retained by a `completed` FR-038 run and
produces a JSON match report. Syft stays the only tool that generates an SBOM (NFR-006). The request
delivers the SBOM to Grype on stdin and names no source selector, so Grype is never given a source tree or
image.

As with FR-038, the producer is a caller-supplied request, not a registry entry, and there is no request
builder: the shape is checked against the published request schema (`producer-execute --schema request`)
and this definition.

### Why Grype

Chosen by a feasibility check against the four properties in the table, with osv-scanner as the
alternative considered.

| Property | Grype | osv-scanner |
| --- | --- | --- |
| Reads a Syft CycloneDX SBOM by purl | Yes: reads a CycloneDX SBOM on stdin; the report carries each match's `artifact.purl`. | Yes (`--sbom`), reads purls; matches are keyed by package name and ecosystem, so the purl is re-derived. |
| Runs offline with a per-run database | Yes: a database directory the caller places (`GRYPE_DB_CACHE_DIR`, `GRYPE_DB_AUTO_UPDATE=false`); the report states the database's build time. | Offline mode needs a locally downloaded per-ecosystem database set; the report does not state a single database build time. |
| Not Rust-only | Single static Go binary; no Rust or Cargo dependency. Covers npm, PyPI, Go modules, and Debian, Alpine and Red Hat family OS packages. | Go binary; OS-package coverage is by distro ecosystem names that differ from purl types. |
| Machine-readable result | `-o json` to a file (`--file`), carrying `matches`, `ignoredMatches`, `descriptor` and the database status. | JSON, with no `ignored` section and no database status in the envelope. |

Grype is chosen because its report envelope proves the scan ran (descriptor and database status are
present even when `matches` is empty) and because it matches OS packages from distribution advisories,
which the container fixtures need. Grype identifies the distribution from the `operating-system` component of the SBOM (Syft emits one only when the scanned image has an `os-release`), so an image without one yields no OS-package matches; the container fixture of this requirement therefore carries an `os-release`. Lean (lake manifest) packages are in neither tool's coverage; see
the quoin join amendment, [FR-142](ix://agent-ix/quoin/FR-142), for how they are reported.

## Inputs

The Grype producer definition fixes these fields of the request schema.

| Request field | Grype definition |
| --- | --- |
| `protocol` | `engineering-assurance.producer-execution-request/v1`. |
| `producer` | `name` `grype`, `version` the exact Grype release being run, `executable` the absolute path of that Grype binary. |
| `caller` | `{"kind": "engineering-assurance.advisory-scan", "version": "1"}`. |
| `procedure` | `direct`. |
| `capabilityRoot` | An absolute directory holding `in/` (the SBOM), `db/` (the vulnerability database) and `out/` (the report and every Grype home and cache directory). |
| `arguments` | `{"kind":"literal","value":...}` entries, in order and with no others: `-o`, `json`, `--file`, `out/grype.json`, `-c`, `out/empty-config.yaml`. The file `out/empty-config.yaml` is an empty file the caller writes before the run, so Grype reads no other configuration and no ignore rule. |
| `environment` | `GRYPE_DB_CACHE_DIR=<capabilityRoot>/db`, `GRYPE_DB_AUTO_UPDATE=false`, `GRYPE_DB_VALIDATE_AGE=false`, `GRYPE_CHECK_FOR_APP_UPDATE=false`, `HOME=<capabilityRoot>/out/home`, `XDG_CONFIG_HOME=<capabilityRoot>/out/home`, `XDG_CONFIG_DIRS=<capabilityRoot>/out/xdg` and `TMPDIR=<capabilityRoot>/out/tmp`. The caller creates `out/home`, `out/xdg` and `out/tmp` empty before the run. No other variable is inherited (FR-019-CON-1). |
| `inputs` | One entry, `{"role":"sbom","path":"in/sbom.cdx.json"}`: the retained FR-038 `sbom` artifact placed there by the caller. |
| `stdin` | `{"kind": "input_artifact", "role": "sbom"}`. FR-019 delivers the retained snapshot of the `sbom` input on stdin, so Grype reads the exact bytes the executor retained and not the pathname, and the scan is bound to the SBOM it read. Grype reads an SBOM from stdin when it is given no source argument. |
| `outputs` | One entry: role `advisory-report`, path `out/grype.json`, `required` true. |
| `containment` | `{"profile": "process-group-v1", "contract": {"kind": "engineering-assurance.grype-containment", "version": "1"}}`. |
| `cancellation` | The caller's choice of `{"kind":"disabled"}` or an exact `event` binding. |
| `budget` | All eight bounds present and within the FR-019 maxima; the caller chooses the values. |
| `response` | The built-in `engineering-assurance.process-evidence` version `1` protocol and `engineering-assurance.cli-process-evidence-adapter` version `1` adapter, with an exit policy admitting exit code 0 only. |

Grype given no source argument and a CycloneDX document on stdin is the one stdin form this definition relies on.
This was not run by the spec author (no Grype binary was available); the integration test of AC-1 decides it,
and if this Grype release requires an explicit stdin argument, this definition is amended to carry exactly that
one argument, and no other source selector.

The `db/` directory is a per-run vulnerability database the caller imported before the run (for example
with `grype db import`) into `GRYPE_DB_CACHE_DIR`. It is not declared as an input, because a retained
input snapshot is one file. The run makes no network connection; the database's build time appears in the
report descriptor, so the report states how current the advisories were. `GRYPE_DB_VALIDATE_AGE=false`
keeps a pinned fixture database usable; whether a report is recent enough stays with the consumer's gate
policy.

## Fixtures

The FR-039 fixtures are fictional and pin no vulnerable package, and its container fixture has no `os-release`,
so they cannot show a match. This requirement therefore adds its own fixtures under `scan/`, each pinning one
package version that the database named by `EA_TEST_GRYPE_DB` holds a long-published advisory for, with that
advisory id and package purl written in an expected-result file beside the fixture: an npm package with a lock
file, a Python project with a pinned requirements file, a Go module with `go.mod` and `go.sum`, and a container
image archive holding an `os-release` and a package database with one pinned OS package. Their SBOMs are the
retained Syft artifacts of FR-038 runs over them. The quoin criteria FR-142-AC-19 and FR-149-AC-10 rely on the
reports captured from these fixtures.

## Outputs

- The FR-029 result line for the request. In the `completed` state it carries one retained output
  artifact, role `advisory-report`, whose bytes are one Grype JSON document.
- No second SBOM is produced, and Grype is never asked for an SBOM output format.

## Behavior

- The request SHALL run through `producer-execute` or the FR-019 library boundary and through no other
  launch path.
- The request SHALL carry exactly the `arguments` listed in Inputs, and no other: one `-o json` pair, one `--file out/grype.json` pair and one `-c out/empty-config.yaml` pair.
- The request SHALL carry no source selector argument, so Grype generates no SBOM from a tree or image.
- The request SHALL NOT carry `-q`, `--quiet`, `--only-fixed`, `--only-notfixed`, `--ignore-states`, `--fail-on`, `--vex`, `--exclude` or `--add-cpes-if-none`. A scan that hides or suppresses is not a scan whose silence can be read as clean.
- If the Grype executable cannot be opened or launched, then the result state SHALL be `unavailable`.
- If Grype exits non-zero, is killed by a signal, or exceeds a stream bound, then the result state SHALL
  be `failed`, and no report is ingested. A missing, unreadable or incompatible database makes Grype exit
  non-zero and is therefore `failed`. Database age is not checked by Grype here (`GRYPE_DB_VALIDATE_AGE=false`).
- If Grype does not finish within `budget.timeoutMillis`, then the result state SHALL be `timed_out`.
- If the required `advisory-report` output is absent after a normal exit, then the result state SHALL be
  `failed`.
- If any state other than `completed` is returned, then the consumer SHALL NOT read a report from the
  result. A stale file at the output path is removed before launch by FR-019.
- The definition SHALL NOT add a closed result state, a request field, a Cargo dependency or a Cargo
  feature.
- The definition SHALL NOT interpret the report (FR-019-CON-2).
- Ingestion is `quoin evidence record`
  through the adapter of `ix://agent-ix/quoin/FR-149`, as FR-039 does for the SBOM.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-044-CON-1 | If `EA_TEST_GRYPE`, the absolute path of the Grype binary, is unset or does not name an executable regular file, or `EA_TEST_GRYPE_DB`, the absolute path of a Grype database directory already imported by `grype db import`, is unset or does not name a directory, then `make grype-integration` SHALL exit non-zero with a message naming the variable. A default `cargo test` does not run Grype-dependent tests, and the repository holds neither Grype nor a database. | Test integrity | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-044-AC-1 | A request meeting the Grype producer definition over the retained SBOM of the npm fixture of this requirement validates against the published request schema, is accepted by `producer-execute`, yields state `completed`, exit status 0, and one retained `advisory-report` artifact that parses as a JSON object with a `descriptor`, an empty `ignoredMatches` and a `matches` array holding at least one match whose vulnerability id and `artifact.purl` equal those in the fixture's expected-result file. The test fails if `matches` is empty. | Test |
| FR-044-AC-2 | The same request shape over the retained SBOM of the Python, Go module and container fixtures of this requirement each yields `completed`, an empty `ignoredMatches`, and a `matches` array holding at least one match equal to that fixture's expected vulnerability id and `artifact.purl`; every match's `artifact.purl` names a component of that SBOM; the container match is for the OS package. The test fails if `matches` is empty. | Test |
| FR-044-AC-3 | A request naming a Grype executable path that does not exist yields state `unavailable` and no report artifact. | Test |
| FR-044-AC-4 | A Grype fixture executable that exits non-zero yields `failed`, and one that outlives a one-second timeout yields `timed_out`; neither result carries a report for ingestion. | Test |
| FR-044-AC-5 | A run whose `db/` is empty or holds no database yields `failed` and no report, so a missing database is never a clean scan. | Test |
| FR-044-AC-6 | The `arguments` of the request used by the tests equal the list in Inputs element for element, with none of the options barred in Behavior, and its `environment`, `inputs`, `stdin` and `containment` equal the definition. | Inspection |
| FR-044-AC-7 | A stale file left at the report output path before a run that fails is absent from the failed result. | Test |
| FR-044-AC-8 | The run makes no network connection: with networking unavailable to the process, the Grype run over the fixture SBOM and imported database still completes. | Test |
| FR-044-AC-9 | `make grype-integration` with `EA_TEST_GRYPE` or `EA_TEST_GRYPE_DB` unset, naming a path that is not an executable regular file (`EA_TEST_GRYPE`) or not a directory (`EA_TEST_GRYPE_DB`), exits non-zero with a message naming the variable and runs no test. | Test |
| FR-044-AC-11 | With the `sbom` input file replaced on disk after the executor retained its snapshot, Grype's report still reflects the retained bytes, because it read stdin and not the path. | Test |
| FR-044-AC-10 | Design review confirms Engineering Assurance contains no Grype report reader, no second SBOM tool and no launch path other than `producer-execute` in production code; test code that checks a fixture report is the only reader. | Inspection |

## Dependencies

- **Upstream**: [FR-019](./FR-019-bounded-producer-execution.md), [FR-029](./FR-029-producer-execute-command.md), [FR-038](./FR-038-syft-sbom-producer-request.md) and [FR-039](./FR-039-syft-sbom-evidence-ingestion.md) for the SBOM it reads.
- **Downstream**: `ix://agent-ix/quoin/FR-149` ingests the report; `ix://agent-ix/quoin/FR-142` joins it onto SOUP records.
