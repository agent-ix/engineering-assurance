---
id: FR-045
title: "Run the document converters pandoc and typst through producer-execute requests"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-029"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-038"
    type: "references"
---

# FR-045: Run the document converters pandoc and typst through producer-execute requests

## Description

The document converters of `quire export` (pandoc, and typst for PDF) are
producers of the FR-019 `engineering-assurance.producer-execution-request/v1`
protocol, run by the FR-029 `producer-execute` command exactly as the FR-038
Syft producer is, so a converter is a declared, bounded external tool and needs
no executable-launch code beyond the existing bounded executor.

A producer is a caller-supplied request, not a registry entry. The request
field table of each converter is owned by quire-cli (`FR-032`, "Run the export
converter through producer-execute") and is not repeated here. Engineering
Assurance adds no request builder, no request field, no closed result state, no
Cargo dependency or feature, and ships no converter binary, template or
reference document.

## Inputs

A request of the quire-cli FR-032 shape for pandoc (DOCX, or Typst source) or
typst (compile Typst source to PDF), over a capability root holding `in/`,
`typ/` and `out/`.

## Outputs

The FR-029 result line. In the `completed` state it carries one retained output
artifact, role `document`.

## Behavior

- A converter request SHALL run through `producer-execute` or the FR-019
  library boundary and through no other launch path.
- If the path in `producer.executable` does not exist or cannot be opened
  without following a symbolic link, then the result state SHALL be
  `unavailable` and carry no artifact. If it opens but is not a regular file,
  then the result state SHALL be `refused`. In both cases the result's
  `producer.name` and `producer.executable` echo the request, so a consumer
  names the converter from them; the user-facing diagnostic is owned by
  quire-cli `FR-033`.
- If the converter exits non-zero, then the result state SHALL be `failed`, the
  executor SHALL report the exit code in `process.terminalStatus` and retain the
  converter's stderr in `process.stderr`, up to the request's `maxStderrBytes`.
- If the converter exits 0 without writing the required output, then the result
  state SHALL be `failed`.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-045-CON-1 | If `EA_TEST_PANDOC` or `EA_TEST_TYPST`, the absolute path of the binary a test runs, is unset or does not name an executable file, then `make converter-integration` SHALL exit non-zero with a message naming the variable. A default `cargo test` does not run the real-converter tests, and the repository holds no converter binary. | Test integrity | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-045-AC-1 | A pandoc request over a fixture Markdown document, with a stand-in executable, is accepted by `producer-execute` and yields state `completed` with one retained `document` artifact at the requested path. | Test |
| FR-045-AC-2 | A pandoc request writing `typ/document.typ` followed by a typst request compiling it to `out/document.pdf`, over one capability root, each yield `completed` with one retained `document` artifact. | Test |
| FR-045-AC-3 | A request naming a pandoc or typst executable that does not exist yields state `unavailable`, no artifact, and a result whose `producer.name` and `producer.executable` equal the request's; one naming a directory yields `refused`. | Test |
| FR-045-AC-4 | A converter that exits 7 with a message on stderr yields state `failed` with exit code 7 and that message in `process.stderr`; one that exits 0 without writing its output yields `failed`. | Test |
| FR-045-AC-5 | The requests used by the tests pass the published request schema of `producer-execute --schema request` and match the quire-cli FR-032 field table for `caller`, `environment`, `inputs`, `stdin`, `outputs`, `containment`, `cancellation` and `arguments`. | Test |
| FR-045-AC-6 | With real pandoc and typst, the fixture converts to a DOCX that opens as a zip whose `word/document.xml` holds the fixture headings and code text, and to a PDF that starts with `%PDF`. A missing tool fails the test, naming the variable. | Test |

## Dependencies

- **Upstream**: [FR-019](./FR-019-bounded-producer-execution.md) owns the executor, request and states; [FR-029](./FR-029-producer-execute-command.md) owns the command; [FR-038](./FR-038-syft-sbom-producer-request.md) is the precedent.
- **Downstream**: quire-cli `FR-032` builds and runs these requests.
