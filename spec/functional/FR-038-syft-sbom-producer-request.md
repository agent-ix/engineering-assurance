---
id: FR-038
title: "Generate a CycloneDX SBOM with Syft through one producer-execution request"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "extends"
  - target: "ix://agent-ix/engineering-assurance/FR-029"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-006"
    type: "constrains"
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
variable fields and passes the request to `producer-execute` unchanged.

## Inputs

The Syft producer definition fixes these request fields:

| Request field | Syft definition |
| --- | --- |
| `producer` | `name` `syft`, `version` the exact Syft release being run, `executable` the absolute path of that Syft binary. |
| `caller` | kind `engineering-assurance.sbom-generation`, version `1`. |
| `procedure` | `direct`. |
| `capabilityRoot` | An absolute directory that contains the scan target, and receives the SBOM and the Syft cache. |
| `arguments` | Literals, in order: `scan`, the scan source selector, `-o`, `cyclonedx-json=<sbom path>`, `-q`. The selector is `dir:<relative path>` for a source tree or `oci-archive:<relative path>` or `docker-archive:<relative path>` for a container image archive. Every path is relative to the capability root. |
| `environment` | At least `SYFT_CHECK_FOR_APP_UPDATE=false` and a cache directory variable pointing beneath the capability root. No other variable is inherited (FR-019-CON-1). |
| `inputs` | One entry for an image archive scan target. A source-tree target is read through the capability root and needs no input entry. |
| `stdin` | `null`. |
| `outputs` | One required output, role `sbom`, at the relative `<sbom path>` named in `arguments`. |
| `containment` | Profile `process-group-v1` with a Syft confinement-contract binding. |
| `budget` | All eight bounds present and within the FR-019 maxima; the caller chooses the values. |
| `response` | The built-in `engineering-assurance.process-evidence` version `1` protocol and `engineering-assurance.cli-process-evidence-adapter` version `1` adapter, exit policy admitting exit code 0 only. |

## Outputs

- The FR-029 result line for the request. In the `completed` state it carries
  one retained output artifact, role `sbom`, whose bytes are one CycloneDX JSON
  document with `bomFormat` equal to `CycloneDX`.
- No other SBOM format is produced. SPDX output selectors are not part of the
  Syft producer definition.

## Behavior

- The request SHALL run through `producer-execute` or the FR-019 library
  boundary and through no other launch path.
- The Syft producer definition SHALL select exactly one scan source, and a
  request whose `arguments` carry no `-o cyclonedx-json=` pair or carry a second
  `-o` pair is not a Syft producer request.
- If the Syft executable named in `producer.executable` cannot be opened or
  launched, then the result state SHALL be `unavailable`.
- If Syft exits with a non-zero code, is killed by a signal, or exceeds a stream
  bound, then the result state SHALL be `failed`, and no SBOM is ingested.
- If Syft does not finish within `budget.timeoutMillis`, then the result state SHALL be `timed_out`.
- If the required `sbom` output is absent after a normal exit, then the result
  state SHALL be `failed`.
- If any state other than `completed` is returned, then the consumer SHALL NOT read an SBOM from the result.
- Before launch the executor removes any file at the fixed output path (FR-019), so a stale SBOM is never presented as the output of a failed run.
- The definition SHALL NOT add a closed result state, a request field, a
  Cargo dependency or a Cargo feature, and SHALL NOT interpret the SBOM content
  (FR-019-CON-2).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-038-AC-1 | A request built to the Syft producer definition over a fictional source-tree fixture is accepted by `producer-execute`, yields state `completed`, exit status 0, and one retained `sbom` artifact that parses as CycloneDX JSON with `bomFormat` `CycloneDX`. | Test |
| FR-038-AC-2 | The same request shape over a fictional container image archive fixture, with the archive declared as an input, yields state `completed` and a CycloneDX JSON artifact. | Test |
| FR-038-AC-3 | A request naming a Syft executable path that does not exist yields state `unavailable` and no SBOM artifact. | Test |
| FR-038-AC-4 | A Syft fixture executable that exits non-zero yields state `failed`, and one that outlives a one-second timeout yields `timed_out`; neither result carries an SBOM artifact for ingestion. | Test |
| FR-038-AC-5 | A request that selects `spdx-json` output, carries two `-o` pairs, or names no `-o cyclonedx-json=` pair is rejected by the Syft request builder before `producer-execute` is called. | Test |
| FR-038-AC-6 | A stale file left at the SBOM output path before a run that fails is absent from the failed result. | Test |

## Dependencies

- **Upstream**: [FR-019](./FR-019-bounded-producer-execution.md) owns the executor, request and states; [FR-029](./FR-029-producer-execute-command.md) owns the command.
- **Downstream**: [FR-039](./FR-039-syft-sbom-evidence-ingestion.md) ingests the produced SBOM.
