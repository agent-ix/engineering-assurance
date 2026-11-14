---
id: SR-20261113192946
title: failure-domain review of bounded producer timeout cleanup
type: SpecReview
analysis: failure-domain
scope: agent-ix/engineering-assurance; spec/functional/FR-019-bounded-producer-execution.md;
  spec/tests.md TC-125
review_set: subset
---

## Summary

PASS. The change closes the specified topology failure: an observed nested descendant changes process group and holds an invocation stream open beyond the deadline. No additional in-scope failure-domain defect was found.

Ticket: VER-95. Reviewed engineering-assurance PR #250 at c527778babc01206d8c027f62760a3bb73b520d1; only FR-019 Behavior/AC-4 and the TC-125 row are change scope. No active applicable AssuranceProfile exists in this repository: engineering_assurance/skeletons/AssuranceProfile.md is a proposed fictional Juniper template, not policy for this subject. Direct subset review is therefore applicable. No source review, gap analysis, build, or runtime test is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Assessment

Tracking is retained after group departure, cleanup covers both the invocation group and each observed descendant, and both reader workers must finish. A direct-child exit alone cannot satisfy the postcondition. Observation is the boundary: unseen descendants and adversarial full-tree escape are explicitly excluded, without turning observed escape into permission to leak. Callback and domain-oracle boundaries remain unchanged.

## Examined scope

```yaml
scope:
- id: FR-019
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: context_only
  excerpt: "- The supported `process-group-v1` confinement profile SHALL require an exact\n  producer confinement-contract\
    \ binding that prohibits session/group escape,\n  daemonization and untracked descendants. The executor SHALL establish\
    \ an\n  invocation-owned process group, terminate and reap that group before return,\n  and return `containment_failure`\
    \ if the host cannot establish the profile or\n  if it observes a descendant leaving the group."
- id: FR-019
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: examined
  excerpt: "When the executor observes a\n  descendant outside the invocation-owned process group, the executor SHALL\n  keep\
    \ that process tracked for cleanup."
- id: FR-019
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: examined
  excerpt: "When cleanup runs, the executor SHALL\n  terminate each observed descendant as well as the invocation-owned group."
- id: FR-019
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: examined
  excerpt: "  When the request deadline is reached, the executor SHALL complete process\n  cleanup, reap the direct child,\
    \ and finish both invocation stream-reader\n  workers within 1,000 milliseconds of that deadline."
- id: FR-019
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: examined
  excerpt: "While cleanup remains\n  pending, the executor SHALL NOT return with an observed fixture descendant\n  alive or\
    \ an invocation stream-reader worker still running."
- id: FR-019
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: examined
  excerpt: "This profile SHALL\n  NOT claim\n  adversarial full-tree containment: it does not guarantee cleanup of a\n  descendant\
    \ that was never observed, and a consumer requiring that stronger\n  property must use a separately accepted provider\
    \ mechanism."
- id: FR-019-AC-4
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: examined
  excerpt: 'Exact wall-clock, stream, input-byte, output-count/byte and concurrency boundaries are admitted while the next
    value is structurally invalid or terminated; an admitted non-zero exit reaches the adapter; ordinary descendants are reaped;
    an escaping-descendant mutant produces `containment_failure`; '
- id: FR-019-AC-4
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: examined
  excerpt: and when an observed nested descendant leaves the invocation process group, keeps an invocation stream open, and
    remains alive past the request deadline, the executor terminates it, completes both stream-reader workers, and returns
    within 1,000 milliseconds after the deadline with no surviving fixture descendant or reader worker.
- id: TC-125
  path: spec/tests.md
  role: examined
  excerpt: 'Exact wall-clock, stream, input-byte, output-count/byte, and concurrency boundaries are admitted and the next
    value is invalid or terminates; admitted non-zero exits reach the adapter; ordinary descendants are reaped; an escaping-descendant
    mutant becomes containment-failure under the explicitly cooperative profile; '
- id: TC-125
  path: spec/tests.md
  role: examined
  excerpt: and a Rust fixture whose observed nested child moves to a distinct process group, holds an invocation stream open,
    and survives the request deadline is terminated with both reader workers complete within 1,000 ms after the deadline,
    leaving no fixture descendant alive
- id: TC-125
  path: spec/tests.md
  role: examined
  excerpt: "🚧 Specified; existing Rust fixture coverage does not yet verify bounded timeout cleanup for an observed escaped child holding an invocation stream open"
- id: FR-019
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: context_only
  excerpt: "- The executor SHALL release its concurrency slot and invocation-owned process\n  resources before returning on\
    \ every path."
- id: FR-019
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: context_only
  excerpt: "- First-party executable fixtures for TC-122 through TC-125 SHALL be Rust\n  binaries. Foreign-language/runtime\
    \ producers remain supported inputs to the\n  public protocol, but the tests SHALL NOT execute Python, JavaScript, or\
    \ shell\n  fixture programs as first-party evidence for this implementation."
- id: FR-019
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: context_only
  excerpt: '| `timed_out` | Monotonic elapsed execution time reaches the request deadline before terminal observation. |'
- id: FR-019
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: context_only
  excerpt: '| `containment_failure` | The selected confinement profile cannot be established or an escaping descendant/group
    breach is observed. |'
- id: FR-019-CON-4
  path: spec/functional/FR-019-bounded-producer-execution.md
  role: context_only
  excerpt: '| FR-019-CON-4 | The `process-group-v1` profile SHALL NOT be represented as adversarial full-descendant containment.
    | Safety | Test (TC-125) |'
```

## Verdict

PASS for specification review. The new timeout cleanup fixture remains specified, not passing; this review does not clear the downstream runtime blocker.
