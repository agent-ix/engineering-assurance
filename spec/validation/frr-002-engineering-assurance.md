---
id: FRR-002
title: "Engineering Assurance proposed not-high process-risk supplier assessment"
type: FeatureRiskRating
scope: supplier
rating: not_high_process_risk
status: proposed
relationships:
  - target: "ix://agent-ix/engineering-assurance/IU-001"
    type: uses_intended_use
  - target: "ix://agent-ix/engineering-assurance/FR-001"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-002"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-003"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-004"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-006"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-007"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-008"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-013"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-014"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-015"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-016"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-017"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-018"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-019"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-020"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-021"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-022"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-024"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-026"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-027"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-028"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-029"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-030"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-031"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-033"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-034"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-035"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-038"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-039"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-044"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-045"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-046"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-050"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-052"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-053"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-054"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-055"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-056"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-057"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-059"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-070"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-071"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-072"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-073"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-076"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-079"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-083"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-087"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-088"
    type: rates_feature
  - target: "ix://agent-ix/engineering-assurance/FR-089"
    type: rates_feature
---
# Engineering Assurance proposed not-high process-risk supplier assessment

## Rationale

This is a proposed supplier-side rating for IU-001. The named human assigned to review and accept it is Peter Krenesky. The record is unsigned and unaccepted pending his decision through the signed-commit workflow. No score or product-safety classification is assigned.

Under IU-001, the features in this record are proposed not-high because their own behavior does not falsely claim that evidence is verified, a requirement is traced, a clause is discharged, or a person approved a result. Failures may disrupt use or affect ancillary outputs without making one of those unsupported claims. These remain human-proposed ratings, not computed scores.

## Assurance activities

| Activity | Acceptance criterion |
|---|---|
| Use valid, absent, conflicting, malformed, unavailable, and incomplete repository fixtures; assert reuse/no-authoring recommendations, bounded paths, and summary statuses for each case. | FR-001-AC-1–9 |
| Inspect the packaged repository for exactly one canonical skill and its declared workflows; remove the target and move it outside the bundle in separate fixtures and assert discovery fails. | FR-002-AC-1–5 |
| Build the wheel in a temporary directory and compare its member set with the required bundle; validate each included schema against its skeleton and the documented manifest contracts. | FR-003-AC-1–8 |
| Execute the five required scenario classes on all four supported hosts, including malformed and unavailable producer cases; assert both explicit terminal choices per host and no inferred decision. | FR-006-AC-1–7 |
| Load each of the four original pilot invocations with ix-flow and compare the exported workflow names with the pre-promotion set; assert added or missing names fail. | FR-007-AC-1–4 |
| Walk every family in the migration decision table and assert exactly one action with a reason; verify the contract makes no qualification claim. | FR-013-AC-1–8 |
| Build a minimal downstream consumer against the root library and CLI; verify the default feature set is empty and feature-specific consumers resolve only the declared dependencies, including no serde_json for default-off consumers. | FR-014-AC-1–9, FR-014-AC-11 |
| Attempt retirement with differing old/new outputs and assert refusal; verify direct invocations use the Rust interface, then scan executable paths for unapproved non-Rust semantics while excluding inert fixtures. | FR-018-AC-1–4 |
| Run successful and rejected fictional workflow requests through the real CLI and compare bytes, stderr, and status; place the shared dependency URL in root metadata, nested metadata, prose, and lookalike forms and assert only the allowed root entry passes. | FR-028-AC-1–4 |
| Inspect the adoption guide for the exact tag and private-record branch patterns, deletion/update restrictions, and explicit notes-ref limits; confirm no command or workflow claims to apply or inspect a ruleset. | FR-083-AC-1–4 |
| Inspect README for all four gesture names, whether each is declared or evidenced, and the shared-session limit; assert the document contains no paywalled standard designator. | FR-087-AC-1–5 |
| Inspect SECURITY.md for a direct private GitHub reporting link, an explicit exclusion of public issues/discussions/PRs, and absence of time, version, safe-harbor, severity, or disclosure promises. | FR-088-AC-1–3 |
| Compare each emitted schema with the command's request type and validate required, optional, enumerated, nullable, and closed-object behavior; exercise invalid schema-option combinations and confirm no request runs. | FR-089-AC-1–5 |
| Run fixtures for observed, unavailable, not-computed, and not-applicable producers; compare the complete producer tuple and command outcome, then assert zero, duplicate, or conflicting states are rejected. | FR-004-AC-1–7 |
| Validate that each semantic concept has one authoritative owner and link direction; inspect the contract for forbidden runner, shell execution, evidence persistence, and human-decision inference capabilities. | FR-008-AC-1–4 |
| Run focused fictional fixtures including integers beyond i64/u64 and compare canonical outputs and identities; inspect shipped source for copied portable-contract, persisted-evidence, or foreign-language executable logic. | FR-015-AC-1–5 |
| Run existing, absent, conflicting, malformed, unavailable, unjustified, and incomplete onboarding fixtures; break one binding and assert fail-closed status and agreement with the retained reference. | FR-016-AC-1–5 |
| Execute every required host/scenario cell and compare success/failure outputs; submit integration-evidence fixtures with multiple failing conditions and assert all are returned with condition code, document, and line. | FR-017-AC-1–10 |
| Run synthetic producers through the library boundary and assert one execution, exact executable/argv identity, bounded evidence, timing, provenance, retained artifact, and the declared timeout/output-limit errors. | FR-019-AC-1–13 |
| Validate plans for each supported direction with absent, numeric, and required bounds; assert target-without-bound and unknown fields fail, then compile the default-off measurement consumer. | FR-020-AC-1–4 |
| Exercise every comparator with threshold and baseline forms, including margin, unknown/missing comparator and malformed numeric boundaries; verify the schema rejects every invalid form and preserves the decision rule. | FR-021-AC-1–20 |
| Round-trip all four wire spellings through render/serialize/parse/deserialize; compare generated language listings with the ownership registry and assert the registry has one owner. | FR-022-AC-1–5 |
| Validate below-gate and gate plans with accepted apparatus paths, then test empty, repeated, non-file, and unlisted entries; exercise each negative-control kind and its required ground-truth fields. | FR-024-AC-1–12 |
| Validate every combination of optional steering fields, including zero and large finite values; assert negative, non-finite, and invalid types fail and compile the minimal consumer. | FR-026-AC-1–6 |
| Publish exact bytes to an absent path, then retry against an existing path and run concurrent publishers; assert no replacement, no sidecars, and successful writes flush the sidecar and parent directory on Unix and Windows. | FR-027-AC-1–5 |
| Run valid, invalid, timed-out, and bounded-output requests through the real binary; assert one canonical newline-terminated result, exact machine error and exit status, then invoke it with an empty environment from an unrelated directory. | FR-029-AC-1–5 |
| Round-trip a package containing every fact kind and multiple adopter subjects; vary input order and compare bytes, feed malformed/unknown members and assert refusal, then compile the minimal default-off consumer. | FR-030-AC-1–7 |
| Publish valid and invalid packages to candidate/attempt paths; assert valid bytes read back equal, invalid/missing-root cases write nothing, and distinct candidates/attempts coexist idempotently. | FR-031-AC-1–6 |
| Validate true, false, and absent values; submit string, number, null, array, and object values and assert each is rejected. | FR-033-AC-1 |
| Validate the absent relation, all three supported relations, and minimum_pairs boundaries; assert unknown relations, zero/negative/non-numeric thresholds, and relation-specific invalid fields are refused. | FR-034-AC-1 |
| Validate absent population declarations, distinct ids, and a reasoned exemption; assert empty/duplicate ids and invalid exemptions fail, including cases that declare both forms. | FR-035-AC-1 |
| Run the declared Syft request over the fictional source fixture with stand-in and supported real executable; validate the request/result schema, component set, provenance, exit state, and version token. | FR-038-AC-1–10 |
| Validate retained Rust, npm, Python, and container SBOM fixtures; compare component counts and pkg:cargo purls, then run the adapter path and confirm it reads the exact retained artifact. | FR-039-AC-1–9 |
| Run the declared request against retained npm, Rust, and container SBOM fixtures with the supplied local advisory database; assert known advisories and producer/version provenance in output. | FR-044-AC-1–14 |
| Use stand-in converters to assert executable and argv, then convert the fixture with real pandoc and typst; inspect the DOCX document XML and PDF signature/content for the expected headings and code. | FR-045-AC-1–6 |
| Build Baseline fixtures with exact path sets and commit ids; modify, omit, duplicate, or misname an item and assert schema and commit-tree checks reject the mismatch. | FR-046-AC-1–6 |
| Inspect README and master spec for the named-person signed release decision statement and ensure each says the tool records/verifies the person’s choice rather than making it. | FR-050-AC-1–4 |
| Construct a first-parent history with records added on selected commits; compare the ordered record list and adding commit, then test previous-tag, shallow-history, deleted-path, overflow, and non-UTF-8 failures. | FR-052-AC-1–17 |
| Compare a release with an unsigned tag and present attestations against a valid signed release with attestations removed; assert only the signed tag determines released status. | FR-053-AC-1–3 |
| Publish the same valid record through public_log, private_store, and none; assert published/discarded results and minimal public bytes, then race same-id preparations and verify one winner with no accepted incomplete marker. | FR-054-AC-1–18 |
| Resolve public and private repository fixtures with private-store-only and explicit configurations; verify default source and sink, then alter destination case and assert refusal before writing private data. | FR-055-AC-1–10 |
| Serialize a record containing ticket text, criteria text, and model names; assert none appear in public bytes and all remain in the private bytes, including exact approval statement/signature fields. | FR-056-AC-1–11 |
| Submit records referencing private, public, and unknown repositories; assert the exact private/unknown offender list, refusal status, and zero public-log calls. | FR-057-AC-1–11 |
| Feed approved-review fixtures with valid and invalid meaning/Role lines and account mappings; verify the resulting source-neutral approval fields and assert labels alone never create a role. | FR-059-AC-1–10 |
| Parse each checked-in release-attestation, change-receipt, and tag-receipt workflow; compare triggers, jobs, files, and per-job permissions with the declared policy and mutate a forbidden input. | FR-070-AC-1–14 |
| Publish public/private release fixtures through each sink; inspect trimmed public bytes and full private storage, then assert refused releases stay private and not_released outcomes publish nothing. | FR-071-AC-1–6 |
| Build a fixture change with ticket, criteria, reviews, and three roles; verify the exact predicate fields/status and recompute every document and artifact digest from canonical/raw bytes. | FR-072-AC-1–7 |
| Build BOMs from fictional session descriptors and validate them against CycloneDX 1.7; compare application, model/service, and plugin components and assert serial number and digest behavior. | FR-073-AC-1–6 |
| Write and push a fixture note, push again and confirm it persists under the notes ref; rerun for verified and verified_failed receipts and assert no duplicate note is written. | FR-076-AC-1–7 |
| Run the exact literal predicate URIs in Markdown and Rust source, then test lookalike URLs and other locations; assert only the declared prefix passes and the full tree check succeeds. | FR-079-AC-1–4 |
