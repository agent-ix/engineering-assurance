---
id: FR-038
title: "Generate a CycloneDX SBOM with Syft through one producer-execution request"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "extends"
  - target: "ix://agent-ix/engineering-assurance/FR-029"
    type: "references"
---

# FR-038: Generate a CycloneDX SBOM with Syft through one producer-execution request

## Description

Engineering Assurance SHALL define the Syft producer as one request shape of the
FR-019 `engineering-assurance.producer-execution-request/v1` protocol, run by
the FR-029 `producer-execute` command, so that Syft is the single tool that
produces a software bill of materials and the run needs no executable-launch
code beyond the existing bounded executor.

A producer is a caller-supplied request, not a registry entry. The Syft
producer definition is therefore the fixed shape below. A caller fills in the
variable fields and passes the request to `producer-execute` unchanged. There
is no request builder: the shape is checked directly against the published
request schema and this definition.

## Inputs

The Syft producer definition fixes these fields of the request schema
(`producer-execute --schema request`). Every required schema field is present.

| Request field | Syft definition |
| --- | --- |
| `protocol` | `engineering-assurance.producer-execution-request/v1`. |
| `producer` | `name` `syft`, `version` the exact Syft release being run, `executable` the absolute path of that Syft binary. |
| `caller` | `{"kind": "engineering-assurance.sbom-generation", "version": "1"}`. |
| `procedure` | `direct`. |
| `capabilityRoot` | An absolute directory holding exactly two subtrees: `scan/`, the only scanned content, and `out/`, the SBOM and every Syft cache and home directory. |
| `arguments` | `{"kind":"literal","value":...}` entries, in order: `scan`, the source selector, `-o`, `cyclonedx-json=out/sbom.cdx.json`, `-q`, `--config`, `out/empty-config.yaml`. The file `out/empty-config.yaml` is an empty file the caller writes before the run, so Syft reads no other configuration. The selector is `dir:scan` for a source tree, or `oci-archive:scan/<file>` or `docker-archive:scan/<file>` for a container image archive. |
| `environment` | `SYFT_CHECK_FOR_APP_UPDATE=false`, `SYFT_CACHE_DIR=<capabilityRoot>/out/cache`, `SYFT_FILE_METADATA_SELECTION=none`, `HOME=<capabilityRoot>/out/home`, `XDG_CONFIG_HOME=<capabilityRoot>/out/home`, `XDG_CONFIG_DIRS=<capabilityRoot>/out/xdg` and `TMPDIR=<capabilityRoot>/out/tmp`. The caller creates `out/home`, `out/xdg` and `out/tmp` empty before the run. No other variable is inherited (FR-019-CON-1). |
| `inputs` | Empty. Syft reads the scan target from the capability root; an image archive is read there too and is not declared as an input, because a retained input snapshot cannot carry a source-scheme prefix. |
| `stdin` | `{"kind": "null"}`. |
| `outputs` | One entry: role `sbom`, path `out/sbom.cdx.json`, `required` true. |
| `containment` | `{"profile": "process-group-v1", "contract": {"kind": "engineering-assurance.syft-containment", "version": "1"}}`. |
| `cancellation` | The caller's choice of `{"kind":"disabled"}` or an exact `event` binding. |
| `budget` | All eight bounds present and within the FR-019 maxima; the caller chooses the values. |
| `response` | The built-in `engineering-assurance.process-evidence` version `1` protocol and `engineering-assurance.cli-process-evidence-adapter` version `1` adapter, with an exit policy admitting exit code 0 only. |

## Outputs

- The FR-029 result line for the request. In the `completed` state it carries
  one retained output artifact, role `sbom`, whose bytes are one CycloneDX JSON
  document with `bomFormat` equal to `CycloneDX`.
- No other SBOM format is produced. SPDX output selectors are not part of the
  Syft producer definition.

## Behavior

- The request SHALL run through `producer-execute` or the FR-019 library
  boundary and through no other launch path.
- The Syft producer request SHALL carry exactly one `-o` pair, of the form
  `cyclonedx-json=out/sbom.cdx.json`, and no other output selector.
- The caller SHALL NOT place a Syft configuration file (`.syft.yaml`,
  `.syft.yml` or `.syft/`) at the top level of the capability root, because
  Syft reads its working directory for configuration. The scan SHALL cover only
  `scan/`, so the scan neither catalogues its own SBOM or cache nor treats a
  configuration file inside `scan/` as anything but scanned content.
- If the Syft executable named in `producer.executable` cannot be opened or
  launched, then the result state SHALL be `unavailable`.
- If Syft exits with a non-zero code, is killed by a signal, or exceeds a stream
  bound, then the result state SHALL be `failed`, and no SBOM is ingested.
- If Syft does not finish within `budget.timeoutMillis`, then the result state SHALL be `timed_out`.
- If the required `sbom` output is absent after a normal exit, then the result
  state SHALL be `failed`.
- If any state other than `completed` is returned, then the consumer SHALL NOT read an SBOM from the result.
  A stale file at the output path is removed before launch by FR-019.
- The definition SHALL NOT add a closed result state, a request field, a
  Cargo dependency or a Cargo feature, and SHALL NOT interpret the SBOM content
  (FR-019-CON-2).

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-038-CON-1 | If `EA_TEST_SYFT`, the absolute path of the Syft binary a test runs, is unset or does not name an executable file, then `make syft-integration` SHALL exit non-zero with a message naming `EA_TEST_SYFT`. A default `cargo test` does not run the Syft-dependent tests, and the repository holds no Syft binary. | Test integrity | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-038-AC-1 | A request meeting the Syft producer definition over a fictional source-tree fixture under `scan/` validates against the published request schema, is accepted by `producer-execute`, yields state `completed`, exit status 0, and one retained `sbom` artifact that parses as CycloneDX JSON with `bomFormat` `CycloneDX`. | Test |
| FR-038-AC-2 | The same request shape over a fictional container image archive under `scan/` with an `oci-archive:` selector and empty `inputs` yields state `completed` and a CycloneDX JSON artifact. | Test |
| FR-038-AC-3 | A request naming a Syft executable path that does not exist yields state `unavailable` and no SBOM artifact. | Test |
| FR-038-AC-4 | A Syft fixture executable that exits non-zero yields state `failed`, and one that outlives a one-second timeout yields `timed_out`; neither result carries an SBOM artifact for ingestion. | Test |
| FR-038-AC-5 | The request used by the tests carries exactly one `-o cyclonedx-json=` pair and no `spdx-json` selector, and its `environment`, `inputs`, `stdin` and `containment` equal the definition. | Inspection |
| FR-038-AC-6 | A stale file left at the SBOM output path before a run that fails is absent from the failed result. | Test |
| FR-038-AC-7 | With a `.syft.yaml` and a `.syft/config.yaml` planted inside `scan/`, and an earlier SBOM and cache under `out/`, the produced SBOM still holds exactly the declared components, lists none originating from `out/`, and none configured by the planted files. | Test |
| FR-038-AC-8 | `make syft-integration` with `EA_TEST_SYFT` unset, or naming a file that is not executable, exits non-zero with a message naming the variable and runs no test. | Test |

## Dependencies

- **Upstream**: [FR-019](./FR-019-bounded-producer-execution.md) owns the executor, request and states; [FR-029](./FR-029-producer-execute-command.md) owns the command.
- **Downstream**: [FR-039](./FR-039-syft-sbom-evidence-ingestion.md) states what happens to the produced artifact.
