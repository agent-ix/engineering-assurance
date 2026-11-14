---
id: EA-110
title: "EA-110 PR #257 round 1 finding dispositions"
type: Review
---

# EA-110 PR #257 round 1 finding dispositions

Review findings recorded on EA-110 for PR #257, addressed in this spec-only
change:

| Review | Finding | Disposition | Change |
| --- | --- | --- | --- |
| SR-20261114084846, base (Linear comment `c98404b4-a1a8-4e7d-be1e-73eea07037b5`) | FND-001: Quoin could upload the committed empty `spec/evidence` skeleton without running its evidence producer or proving current records and bindings. | Fixed in the specification. | FR-070 now requires the tag-attestation build to run documented `make evidence`, validate generated output, require a run record for the tagged commit and a real binding from a passing case, and fail with `no_retained_evidence_store` before upload or statement creation for absent, empty, malformed, stale, or unbound output. TC-666 specifies those producer and refusal cases. |
| SR-20261114084849, dependency (Linear comment `97032d19-6911-44f1-9036-a2314a48b1cf`) | FND-001: Quoin tooling installation did not pin the npm registry, allowing ambient npm configuration to redirect the package request. | Fixed in the specification. | FR-070 and TC-669 require the exact published package version through `env`, set `NPMJS_REGISTRY` explicitly to the HTTPS registry origin with host `registry.npmjs.org`, and install with `--ignore-scripts --registry="$NPMJS_REGISTRY"`; ambient registry configuration, `latest`, and caller-built binaries are excluded. |

No workflow or code implementation is included. The findings remain subject to
the reviewer's round 1 re-review of the updated PR head.
