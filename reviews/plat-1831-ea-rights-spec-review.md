---
id: SR-20261116045020
title: "PLAT-1831 engineering-assurance PR302 spec-review"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance PR302 head f6af8f2; spec/functional/FR-017-rust-evaluation-and-qualification.md"
review_set: subset
date: 2026-11-24
---

## Summary

Ticket: PLAT-1831. Spec review of the FR-017 edit: two new clause-source table rows and one new prose paragraph about the EU GMP Annex 11 set. EARS and integrity checks were applied inline to the changed text. The rights basis was checked against spec-clauses-eu-law main: `spec/spec.md` (Annex 11 Set Design), FR-002, NFR-004, `CONTENT_RIGHTS.md`, and the set JSON.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new paragraph says the Annex 11 set SHALL cite the Commission legal-notice URL "as its rights-attribution reference". The pack does not do this. Its `spec.md` and NFR-004-AC-1 fix that URL inside `rights.licence`, and `rights.attribution` carries only the PDF URL. The FR-017 claim contradicts the source it describes, unlike the accurate GDPR/AI Act and MDCG paragraphs next to it. | spec/functional/FR-017-rust-evaluation-and-qualification.md:254-257 |

## Finding details

- FND-001 fix: for example, "The EU GMP Annex 11 clause set cites the Annex 11 PDF in `source.officialUrl` and `rights.attribution`, and the Commission legal-notice URL (`commission.europa.eu/legal-notice_en`) in `rights.licence`; both are admitted only at that set's member paths." The paragraph could also say that the legal notice is the reuse basis and does not imply a licence statement in the Annex 11 document itself, matching the MDCG paragraph's caveat and NFR-004.

## Scope examined

```yaml
scope:
- id: FR-017 clause-source table (Annex 11 rows)
  path: spec/functional/FR-017-rust-evaluation-and-qualification.md
  role: examined
  excerpt: "| https | health.ec.europa.eu | /document/download/8d305550-dd22-4dad-8463-2ddb4a1345f1_en | filename=annex11_01-2011_en.pdf | spec_clauses_eu_law/clauses/eu-eu-gmp-annex-11-2011-01.json | package/clauses/eu-eu-gmp-annex-11-2011-01.json | / | https | commission.europa.eu | /legal-notice_en | none | (same member paths) |"
- id: FR-017 Annex 11 rights paragraph
  path: spec/functional/FR-017-rust-evaluation-and-qualification.md
  role: examined
  excerpt: "The EU GMP Annex 11 clause set SHALL cite the Commission legal-notice URL (commission.europa.eu/legal-notice_en) as its rights-attribution reference and the Annex 11 PDF as its source; both are admitted only at that set's member paths."
- id: NFR-004
  path: agent-ix/spec-clauses-eu-law spec/non-functional/NFR-004-annex-11-source-and-rights.md
  role: context_only
  excerpt: "The Commission reuse policy (Decision 2011/833/EU) and the Commission legal notice license Commission content under CC BY 4.0 with attribution, but the Annex 11 document carries no licence statement of its own."
- id: FR-002
  path: agent-ix/spec-clauses-eu-law spec/functional/FR-002-truthful-and-publishable-rights-metadata.md
  role: context_only
  excerpt: "Each set SHALL declare structure: citation_only, text: original, clearance_ref: null, nonblank licence, and nonblank attribution, using the wording and legal-notice URLs recorded in spec.md."
```

## Verdict

**Approve after FND-001.** The rows are right, and the rights basis supports them.

- (1) The two rows match the URLs in the Annex 11 JSON on spec-clauses-eu-law main exactly. The table's exact-match rule admits nothing broader.
- (2) Rights basis: the set is `citation_only` / `text: original` / `clearance_ref: null` and reproduces no Annex 11 text (NFR-004). Its licence states the Commission reuse policy (Decision 2011/833/EU) and that the legal notice gives CC BY 4.0 with attribution, and says the Annex 11 document has no licence statement of its own. This matches how the GDPR and AI Act sets are approved: an official source URL plus its publisher's legal-notice URL, both scoped to that set's member paths. It is a closer fit than MDCG, whose PDF is not a Commission document. Approving the URLs here only admits citations of them; no source text enters any package. `CONTENT_RIGHTS.md` there asks that approvals be recorded. Peter's 2026-11-23 approval stated in the brief is the human decision; the FR-017 rows are its record in this repo.
- FND-001 is a prose accuracy defect only. The classifier does not read the paragraph, so behaviour is unaffected.
