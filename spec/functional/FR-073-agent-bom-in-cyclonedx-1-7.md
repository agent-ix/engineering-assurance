---
id: FR-073
title: "Record the agent bill of materials as a CycloneDX 1.7 document labelled as producer-reported"
type: FR
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-072"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/FR-056"
    type: "references"
  - target: "ix://agent-ix/engineering-assurance/NFR-008"
    type: "references"
---

# FR-073: Record the agent bill of materials as a CycloneDX 1.7 document labelled as producer-reported

## Description

Tracked by Linear EA-98.. The agent bill of materials describes what produced a change. It is a CycloneDX 1.7 document built by Engineering Assurance from the session descriptor that the producer reported, and it is labelled as reported by the producer: Engineering Assurance observes none of it. The mapping is fixed: the harness is a component of type `application`; the model is a component of type `machine-learning-model` and a `service`; each plugin is a component with a package URL; each MCP server is a `service`; every other fact is a property whose name starts with `agent-ix:`. Each property `agent-ix:reported-by` has the value `producer`. The document uses no `formulation` section and no declarations.

The document names the model, so it belongs to the private form ([FR-056](./FR-056-public-and-private-record-split.md)) and never to a public sink or to git notes; the predicate of [FR-072](./FR-072-change-record-predicate.md) carries only its digest. The document is validated against the CycloneDX 1.7 JSON schema supplied by the CycloneDX library dependency; no copy of the schema is committed. A missing session descriptor produces no BOM, and the receipt is `failed` with the failure code `contract_missing`.

## Behavior

- The BOM builder SHALL map the harness to a component of type `application`.
- The BOM builder SHALL map the model to a component of type `machine-learning-model` and to a service.
- The BOM builder SHALL map each plugin to a component with a package URL.
- The BOM builder SHALL map each MCP server to a service.
- The BOM builder SHALL put every other session fact in a property named with the `agent-ix:` prefix.
- The BOM builder SHALL set `agent-ix:reported-by` to `producer` on every property it writes.
- The BOM builder SHALL validate the document against the CycloneDX 1.7 schema before use.
- The BOM builder SHALL NOT write a `formulation` section or a declarations section.
- The BOM builder SHALL NOT store the BOM outside the private form.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-073-AC-1 | A fixture session descriptor yields a document that validates against the CycloneDX 1.7 schema and holds one application component, one machine-learning-model component with a matching service, plugin components with purls and one service per MCP server. | Test |
| FR-073-AC-2 | Every property the builder writes has the prefix `agent-ix:`, and each `agent-ix:reported-by` property equals `producer`. | Test |
| FR-073-AC-3 | A document with a `formulation` section is rejected. | Test |
| FR-073-AC-4 | The public record bytes and the git note content for the fixture change contain no model name; the private store holds the BOM under the record's id. | Test |
| FR-073-AC-5 | A missing session descriptor yields no BOM and a failed receipt with the failure `contract_missing`. | Test |

## Dependencies

- **Upstream**: [FR-072](./FR-072-change-record-predicate.md), [FR-056](./FR-056-public-and-private-record-split.md), [NFR-008](../non-functional/NFR-008-closed-schemas.md).
- **Downstream**: [FR-074](./FR-074-per-change-receipt-bundle-records-failures-too.md).
