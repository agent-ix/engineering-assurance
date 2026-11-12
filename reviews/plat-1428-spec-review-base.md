---
id: SR-16409
title: "Base spec review of the Syft SBOM producer specification (PLAT-1428)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance; spec/functional/FR-038-syft-sbom-producer-request.md, spec/functional/FR-039-syft-sbom-evidence-ingestion.md, spec/non-functional/NFR-006-single-sbom-tool-single-execution-path.md, spec/spec.md (PR #212 diff vs origin/main)"
review_set: subset
---

## Summary

Ticket: PLAT-1428. Base checklist review of the PR #212 diff: FR-038, FR-039,
NFR-006 and their spec.md index lines. Context read: FR-019, FR-029, the
published request schema `src/producer_execute_schemas/producer-execution-request-v1.schema.json`,
`src/producer_execution.rs` argument resolution, quoin `FR-041` and
`quoin-evidence/src/adapters/sbom.rs` on quoin `origin/main`, and the Syft
CLI and configuration reference (oss.anchore.com).

No active AssuranceProfile applies (only a skeleton exists under
`engineering_assurance/skeletons/`). Review set: subset (base,
failure-domain, dependency, ears-conformance). `quire validate` on the four
files exits 0.

Clean units: IDs are well formed and sequential (FR-038, FR-039, NFR-006 follow
FR-037 and NFR-005); quoin FR-041 is cited by `ix://` id only with no copied
text; FR-038 adds no new execution path (it requires `producer-execute` or the
FR-019 library and nothing else); the result-state mapping in FR-038 Behavior
matches the FR-019 closed state table; the Syft flags `scan`, `-o
<format>=<file>`, `-q`, the source schemes `dir:`, `oci-archive:`,
`docker-archive:` and the variable `SYFT_CHECK_FOR_APP_UPDATE` are real per the
Syft reference; no hash, digest, ledger or version pin appears in prose
(`producer.version` is a required runtime request field, not a pin).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-039 never states how Engineering Assurance reaches the Quoin SBOM adapter. `--adapter sbom` is a `quoin evidence record` CLI flag that records rather than returns entries; `quoin-evidence` is `publish = false` and Engineering Assurance has no Cargo dependency on quoin; a `quoin` subprocess would be a launch path NFR-006 does not cover. The ingestion mechanism (library link, `quoin` run through producer-execute, or a consumer step outside this repo per quoin FR-041-CON-1) must be chosen and stated. | spec/functional/FR-039-syft-sbom-evidence-ingestion.md:18-23,38-40 |
| FND-002 | medium | FR-038 Inputs table omits the required request fields `cancellation` and `protocol`, and states `stdin` as `null` and arguments as bare literals where the schema requires `{"kind":"null"}` and `{"kind":"literal","value":...}`; NFR-006-AC-3 requires schema-valid output, which the table as written does not yield. | spec/functional/FR-038-syft-sbom-producer-request.md:31-45 |
| FND-003 | medium | The "Syft request builder" named by FR-038-AC-5, NFR-006-AC-2 and NFR-006-AC-3 is defined nowhere: FR-038 Description says the caller fills the request, and no Behavior or Output names a builder, its inputs, its home (library or CLI) or its refusal type. | spec/functional/FR-038-syft-sbom-producer-request.md:83 |
| FND-004 | medium | `containment` is fixed as "a Syft confinement-contract binding" with no kind or version, while FR-019 requires an exact binding and FR-038 names the `caller` kind and version exactly; two implementers will choose different contracts. | spec/functional/FR-038-syft-sbom-producer-request.md:43 |
| FND-005 | low | `environment` requires "a cache directory variable" without naming it. Syft's configuration reference documents `SYFT_CACHE_DIR` (config `cache.dir`); naming it resolves the author's open "cache env var unverified" gap. | spec/functional/FR-038-syft-sbom-producer-request.md:39 |
| FND-006 | high | NFR-006-AC-2 checks that no Cargo "dependency, feature or lock entry" changes, which is lockfile tracking (checklist hash/pin rule) and duplicates the FR-038 Behavior prohibition on new Cargo dependencies or features. Remove the criterion. | spec/non-functional/NFR-006-single-sbom-tool-single-execution-path.md:38 |
| FND-007 | medium | NFR-006-AC-1 and FR-039-AC-5 are verified by scanning source text for tool names and reader definitions ("names no SBOM generator other than Syft", "No source file defines a CycloneDX component reader"); a name scan is defeated by renaming and flags comments. State them as design constraints checked in code review, not as criteria a source scan discharges. | spec/non-functional/NFR-006-single-sbom-tool-single-execution-path.md:37; spec/functional/FR-039-syft-sbom-evidence-ingestion.md:59 |
| FND-008 | medium | FR-038-AC-1, FR-038-AC-2 and FR-039-AC-1..4 need a real Syft binary, but the spec does not say how the test obtains one or that a missing Syft fails the test rather than skipping it; a skip would leave six criteria passing with no evidence. | spec/functional/FR-038-syft-sbom-producer-request.md:79-80 |

## Verdict

Not ready to implement until FND-001, FND-006 and the medium findings are
fixed. The structure, ID scheme, reuse-by-id of quoin FR-041 and the single
execution path are sound; the gaps are in the ingestion interface, the exact
request shape, and two inspection criteria.
