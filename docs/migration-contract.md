# Contract-campaign assurance migration contract

The reviewed procedure Agents A, B, and C apply to the eight campaign
repositories once the shared interfaces are released. Answers
`agent-ix/engineering-assurance#10`.

**Nothing in this playbook may begin until the compatibility matrix records
human acceptance.** `accepted_by` and `accepted_at` in
`engineering_assurance/compatibility-matrix.json` are null, and
`python3 scripts/check_compatibility_matrix.py` reports the gate. An agent
cannot grant that acceptance; TC-082 fails if one tries.

## The repositories, and who holds them

| Agent | Repositories |
| --- | --- |
| A | `quire-contract-ir`, `quire-contract-runtime`, `quire-contract-codegen` |
| B | `quire-analyze`, `tl-syntax` |
| C | `tl-parse`, `tl-mltl`, `tl-rewrite` |

The existing allocation is preserved. A repository does not change hands as
part of this migration.

## The decision table

Every recurring script family across the eight repositories, with its
disposition. This is a census, not a sample — the eight `scripts/` trees were
read at `origin/main`.

| Family | Where | Decision | Why |
| --- | --- | --- | --- |
| `check_unsafe_comments.sh` + `unsafe_comment_baseline.txt` | all eight | **KEEP** | Domain safety discipline over Rust `unsafe`. Produces findings; transcribe with the `audit-script` adapter. |
| `check_panic_surface.sh` | runtime | **KEEP** | Domain check on the panic surface. Same transcription path. |
| `check_linked_footprint.sh` | runtime | **KEEP** | Domain check on linked size. Same transcription path. |
| `measure_rlib_size.sh` | runtime | **KEEP** | Produces a measurement. Feeds `quoin measurement record`; it is not evidence intake. |
| `generate_conformance_corpus.py` | contract-ir | **KEEP** | Domain corpus generator. The corpus is a verification *definition*. |
| `build_evidence_envelope.py` | runtime, tl-mltl | **DELETE** | A repository-local PGM-01 envelope, manifest, and canonical form — the parallel generic evidence family the campaign exists to remove. Replaced by `quoin change-assurance seal-record` and `seal-attestation`. |
| `collect_evidence.sh` | runtime, tl-mltl | **DELETE** | A generic collector that runs commands and retains their stdout, stderr, and exit status as the record. Replaced by native runners plus `quoin change-assurance intake`. |
| `verify_evidence.sh`, `verify_evidence.py` | tl-mltl, contract-ir | **DELETE** | A repository-local verifier over the local envelope. Replaced by `quoin change-assurance receipt` and `verify-receipt`. |
| `finalize_collection.py` | tl-mltl | **DELETE** | Closes the local collection. Nothing to close once intake owns retention. |
| `test_evidence_tool.py` | tl-mltl | **DELETE** | Tests of the deleted machinery. They go with it, not before it. |
| `validate_json_schema.py` | runtime, tl-mltl | **REPLACE** | Generic schema validation. Spec artifacts are validated by `quire validate`; retained records by Quoin's packaged schemas. |
| `validate_governance.py`, `validate_matrix_status.py` | contract-ir | **KEEP** | Governance-specific to that repository's own PGM-01 obligations, not a generic evidence family. |
| `schemas/*-evidence-*.schema.json`, `schemas/pgm01-*` | runtime, tl-mltl, contract-ir | **DELETE, read-only-preserved** | Repository-local generic evidence schemas. Historical records stay readable through the FR-010 compatibility view; the schemas stop being written against. |

A family not in this table is domain logic until somebody argues otherwise, in
writing, on the migration issue.

## The two rules that decide the hard cases

**No repository-local generic evidence schema.** If a schema describes an
envelope, a manifest, a collection, a run identity, or an audit — not a domain
artifact — it is generic, and Quoin owns the shape. A repository may keep a
schema that describes *its own domain output*: `tl-mltl`'s differential summary
and `quire-contract-ir`'s conformance manifest both stay.

**No universal stdout corroboration.** A verdict recovered from console text is
a verdict the producer never made. Native structured output is retained when it
exists; a console stream is retained only when it *is* the material diagnostic
artifact, and never as the thing a pass or fail is read from.

## Domain output validation is not evidence intake

They are different jobs and they stay in different places.

| Question | Owner |
| --- | --- |
| Is this a well-formed contract package / MLTL formula / temporal rewrite? | the domain repository, in its own tests |
| Did this producer run, at this revision, producing these exact bytes? | Quoin intake |
| Do the retained bytes still hash to what was recorded? | Quoin audit |
| Was the evidence sufficient to accept the change? | a human, through ix-flow |

A migration that moves domain validation into Quoin has misread this table. A
migration that leaves intake in a repository-local script has misread it the
other way.

## Procedure

1. **Inventory before touching anything.** Classify every script in the
   repository against the decision table. Anything unclassified is written up
   on the migration issue before any deletion.
2. **Preserve the domain.** Runners, oracles, corpora, schemas describing
   domain output, structured result formats, and failure behaviour are not
   touched by this migration.
3. **Register static facts through Quire.** Obligations, symbols, relations,
   and locators come from `quire coverage --json`. The repository stops
   maintaining a second graph.
4. **Register dynamic results through Quoin.** Producer output reaches the
   store through an adapter — `junit`, `cargo-mutants`, `sarif`,
   `audit-script`, `cargo-audit`, `contract-conformance`,
   `differential-report`, or the normalized `entries` shape. Arbitrary stdout
   is not an adapter.
5. **Replace, do not reimplement.** Local envelope, manifest, identity,
   retention, audit, history, traceability, and anchor implementations are
   deleted and their jobs handed to the shared components. A "thin wrapper"
   around a deleted family is the family.
6. **Preserve legacy history read-only.** Existing evidence directories are
   never rewritten. They stay readable through the FR-010 compatibility view,
   which reports `lossy`, `unreadable`, or `incompatible` rather than
   synthesizing a field the legacy record never had.
7. **Simplify the Makefile to native orchestration.** A Makefile calls the
   native toolchain. It is not a trust root, not a self-attesting qualification
   boundary, and not the place a verdict is computed.
8. **Demonstrate every state.** Pass, fail, unavailable, not-computed,
   malformed, stale, and tampered each need a demonstrated case in the
   repository's own evidence. A migration that only demonstrates pass has
   demonstrated nothing about the states that matter.
9. **Delete last.** Old generic code is removed only after the shared path
   passes at the **same exact candidate revision**. Until then both exist, and
   the old one is the fallback.

## Rollback

Per step, and none of it is irreversible:

| If | Then |
| --- | --- |
| the shared path fails at the candidate revision | stop at step 9; the old machinery is still there and still runs |
| a released component turns out to be wrong | roll it back per the matrix's rollback table; the repository is untouched by that |
| legacy history reads as `unreadable` | that is the compatibility view working; do not edit the legacy record to make it read |
| a domain result has no adapter | file it against `agent-ix/quoin` with a real producer and a pinned sample; do not scrape stdout as a stopgap |

Legacy history is never rewritten in any of these paths.

## Migration PR review checklist

A migration PR is reviewable only if every line below has an answer in the PR
body, not in a reviewer's head.

- [ ] The script inventory is in the PR, with a decision per family and a
      reason for anything unclassified.
- [ ] No repository-local generic evidence schema remains, and no new one is
      introduced.
- [ ] No verdict is read from stdout or stderr anywhere in the change.
- [ ] Domain runners, oracles, corpora, and result formats are unchanged, or
      the change is justified as a domain change and not a migration one.
- [ ] Every existing evidence directory is byte-identical to its pre-migration
      state.
- [ ] The compatibility view is exercised over that repository's real legacy
      records, and its `lossy` and `unreadable` outcomes are shown.
- [ ] Pass, fail, unavailable, not-computed, malformed, stale, and tampered are
      each demonstrated.
- [ ] The shared path and the old path both pass at the same candidate
      revision, and the deletion commit is separate and last.
- [ ] The Makefile is native orchestration; no target computes a verdict.
- [ ] `python3 scripts/check_compatibility_matrix.py` passes in the migrating
      repository's environment.
- [ ] No workflow changed from manual dispatch.

## Hosted CI

Every workflow in every campaign repository stays manual-dispatch only. This
playbook dispatches nothing and changes no trigger. A migration PR that enables
automatic CI is out of scope regardless of its other merits.

## What this playbook does not claim

It makes no certification, accreditation, authorization, identity, or
non-repudiation claim, and completing it does not qualify any repository for
anything. Digests establish content integrity; recorded actor labels are
attribution. Qualification is use-specific and lives outside this campaign.
