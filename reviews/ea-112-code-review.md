---
id: SR-20261113192938
title: "Code review of the solo-maintainer controls SOP (EA-112, PR #236)"
type: SpecReview
analysis: code-review
scope: "agent-ix/engineering-assurance@9fccbb17d8dd2370cc5371ccb5e3929f2729d770; docs/sop/solo-maintainer-controls.md; EA-112 acceptance criteria; cited FR-061/062/063/065/077 and NFR-011"
review_set: subset
---

## Summary

Reviewed the single-document EA-112 draft against its six-control acceptance scope, the cited requirements, and the Regulated-Software Support rules. The SOP is in scope and uses generic language, but the signing-job description omits NFR-011's restriction on credentials reachable by an agent.

## Scope and method

Reviewed PR #236 frozen head 9fccbb17d8dd2370cc5371ccb5e3929f2729d770; the diff contains only docs/sop/solo-maintainer-controls.md. Examined all six control rows, the opening policy statements, the open operational proposals, EA-112 acceptance criteria 1–5, and cited requirements FR-061, FR-062, FR-063, FR-065, FR-077, and NFR-011.

The code-review method ran as a document-only review. No Rust, Python, or React lane applied. No AssuranceProfile applies in spec/. No build or test was needed. The ticket criteria were checked manually against the document; no production code, spec, or plan changed, so no separate gap-analysis or spec-review method applied. The designator search found no standards-body designators in the SOP. I did not independently rerun the coder's stated checks.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The signing-job safeguard omits NFR-011's ban on agent-reachable credentials, so the SOP leaves a material signing-permission condition unstated. Include that condition alongside the no-agent and no-repository-code conditions. | docs/sop/solo-maintainer-controls.md:21 |

## Verdict

**CONDITIONAL** — fix FND-001 in this draft. The proposed five-working-day deadline and four higher-risk triggers are clearly marked as pending Peter's confirmation; that owner decision remains a separate merge gate.
