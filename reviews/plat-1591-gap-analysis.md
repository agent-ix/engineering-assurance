---
id: SR-16418
title: "gap-analysis of FR-042 approval-tag (engineering-assurance PR 219)"
type: SpecReview
analysis: base
scope: "agent-ix/engineering-assurance@a40b2fdb; FR-042; spec/tests.md; tests/approval_tag.rs; tests/approval_tag_cli.rs; src/approval_tag.rs; src/approval_tag_host.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/engineering-assurance/FR-042
    type: references
---

# Gap analysis of FR-042 approval-tag (engineering-assurance PR 219)

## Summary

Ticket: PLAT-1591. Plan completion: not assessed. `quoin matrix --repo . --json` (quoin 0.30.1) binds every Test criterion of FR-042 (AC-1..AC-12, CON-1) to a `#[trace]`-tagged test in tests/approval_tag.rs or tests/approval_tag_cli.rs. AC-13 and CON-2 are Inspection and unbound, as expected. Inspection was performed: the approval_tag modules hold no cryptography, signature, certificate or digest code, run only rev-parse, check-ref-format, config --get and tag, and never pass -f, -u or push. AC-13 is honest. TC-338..TC-350 are new, collide with neither origin/main nor #218 (TC-331..337), and are marked 🚧 with no hand-written passing claim. The FR-014 package_boundary count (15) matches the test table. Fixtures (ed25519 keys, repositories, stand-ins) are generated at test time and nothing is checked in. All 13 tests pass on git 2.34.1.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-344 cannot fail on the global configuration half of FR-042-AC-5. It compares `fs::read_to_string(global)` with `after.1`, which was read moments earlier from the same file, so a write to the global config would pass. Refs are compared only by added lines, so a deleted ref also passes. Fix: snapshot the global file after the last `ssh_config` write and compare it after the run, and compare the full before and after ref sets apart from the two new tags. | tests/approval_tag_cli.rs:422-453 |
| FND-002 | medium | FR-042-AC-11 is exercised only on the x509 path, where git checks the signer's exit status. The SSH signer-failure path (missing key file, locked key with no agent) is not tested, and it is exactly where git 2.34.1 writes an unsigned tag at exit 0 (SR-16417 FND-001). Fix: add an SSH case to TC-350 with `user.signingkey` naming a missing file, expect `approval_tag_create_failed` and no tag. | tests/approval_tag_cli.rs:720-731 |

## Verdict

Traceability is complete and every binding is correct. Two tests are weaker than their criteria: AC-5's config assertion is vacuous, and AC-11 misses the SSH failure that actually fails open. Fix both alongside SR-16417.
