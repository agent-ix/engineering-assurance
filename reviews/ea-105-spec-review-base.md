---
id: SR-20261113163611
title: "Base spec review of the record sink and role-based approval statement requirements (engineering-assurance PR 231)"
type: SpecReview
analysis: base
review_set: subset
scope: "agent-ix/engineering-assurance; spec/functional/FR-054..FR-065, spec/non-functional/NFR-008..NFR-010, spec/spec.md (PR #231 diff, head dd5a124)"
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-057
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-058
    type: references
  - target: ix://agent-ix/engineering-assurance/FR-061
    type: references
  - target: ix://agent-ix/engineering-assurance/NFR-010
    type: references
---

# SR-20261113163611: Base spec review of the record sink and role-based approval statement requirements (engineering-assurance PR 231)

## Summary

Ticket: EA-105 and PLAT-1685. Base checklist over the PR diff (FR-054 to FR-065, NFR-008 to NFR-010, spec/spec.md index) at head dd5a124, against both tickets' acceptance criteria and Peter's relayed rulings recorded on them. IDs are sequential with no gap or duplicate, every AC has a Verification method and no TC id, the index lists every new artifact, links resolve, and `quire validate --scope . "spec/**/*.md"` exits 0 with no warning on any file of this PR. Hash/digest/pin check: the public record's `hashes` and the in-toto subject's git commit digest are the attestation content the ruling requires (trust is the feature), not file or version tracking; no ceremony found. No compatibility layer, no paywalled text, no roadmap content, no private Linear text quoted. tests.md is untouched: acceptable, because the matrix is computed from test trace tags and the TC rows arrive with the code PRs; it is not a finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-061-AC-2 says `missing` names "the role left uncovered", but with one individual holding developer and engineer statements either role may be the uncovered one; the result depends on the assignment the matcher picks, so two implementations (and two runs of a tie-breaking matcher) report different roles. Fix: define the report deterministically (for example, every role the shared individual could have covered, each with `shared_individual`), and say the same in the Outputs. | spec/functional/FR-061-role-policy-and-distinct-individuals.md:63 |
| FND-002 | medium | NFR-010's statement delegates to "the Sigstore signing and verification libraries and in-toto statement types named in Linear PLAT-1481", so the obligation cannot be checked from this public spec: a reader cannot tell which library a source inspection should find. Fix: name the libraries in the NFR (cite the ticket only as history). | spec/non-functional/NFR-010-no-cryptography-of-our-own.md:20 |
| FND-003 | low | The predicate type spelled as scheme/host/path is unambiguous and implementable, but the parenthetical about the content-rights check is tooling commentary inside a requirement, and the code PR will need the literal anyway. Recommendation: in the first code PR, admit the org's own `schemas.agent-ix.ai/attestation/` prefix in the content-rights URL allowlist (as `schemas.agent-ix.org/semantic-core/` already is), then write the literal URI here and drop the parenthetical. | spec/functional/FR-058-approval-statement-and-predicate.md:29-30, 54 |
| FND-004 | low | FR-057-AC-3 (a lookup that errors or times out -> `visibility_unknown`) and AC-4 (no visibility source reachable -> `public_sink_visibility_unavailable`) overlap: when the hosting service is down every repo lookup errors, and nothing says which code wins. Both refuse, but the two ACs can be implemented to contradict. Fix: define when a failure is per-reference and when it is source-wide. | spec/functional/FR-057-fail-closed-pre-publish-leak-check.md:58-59 |

## Verdict

Base checklist passes on structure. FND-001 makes one AC non-deterministic and FND-002 makes NFR-010 unverifiable from the public spec; both should be fixed in this PR. FND-003 records the recommendation on the predicate-type spelling (acceptable as written; implementable). See the integrity, failure-domain, security and scope-boundary reviews for the blocking findings.
