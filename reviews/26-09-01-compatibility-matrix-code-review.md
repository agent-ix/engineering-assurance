---
id: SR-026
title: "Code review — pinned shared-assurance compatibility matrix"
type: SpecReview
analysis: code-review
scope: "engineering_assurance/compatibility.py, engineering_assurance/compatibility-matrix.json, scripts/check_compatibility_matrix.py, scripts/audit_packages.py, scripts/stage-npm.mjs, setup.cfg, package.json, docs/compatibility-matrix.md, spec/functional/FR-012, spec/tests.md, corpus gitlink"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-012"
    type: "references"
---

# SR-026: Code review — pinned shared-assurance compatibility matrix

## Summary

Reviews the FR-012 matrix added for #8: four released components with their
artifact digests, a pure classifier, a separate observing program, and the
upgrade and rollback notes a human needs before accepting it.

## Verdict

**CONDITIONAL** — no high findings. Two medium findings record real defects
found and fixed during the release, and one low finding records a mistake I
made reading the registry.

## Gates

- `make lint` (ruff) — clean.
- `make validate-docs` — clean; FR-012 raises no EARS or quality warning.
- `scripts/check_content_rights.py --tree` — passes, **unmodified**.
- `python3 -m pytest` — 174 pass, 0 fail.
- `python3 scripts/audit_packages.py` — passes; the matrix ships in both the
  wheel and the npm archive.
- `python3 scripts/check_compatibility_matrix.py` against the installed
  toolchain — quire-cli, quoin, and ix-flow classify compatible;
  engineering-assurance classifies unknown until its tag exists, which is the
  correct answer and the reason the gate is not satisfied yet.

## Findings

| ID      | Severity | Summary                                                                     | Refs                                          |
| ------- | -------- | --------------------------------------------------------------------------- | --------------------------------------------- |
| FND-001 | medium   | The v0.23.0 release caught an adapter defect that had passed locally         | quoin#329                                      |
| FND-002 | medium   | The matrix shipped in neither the wheel nor the npm archive until the audit failed | scripts/audit_packages.py:91                   |
| FND-003 | low      | I read the internal mirror and reported a successful publish as failed        | engineering_assurance/compatibility-matrix.json:139 |

## Finding detail

### FND-001 — the release gate earned itself

`quoin v0.23.0` was tagged and dispatched. Its release run failed at the test
leg with the fast-check counterexample `["valueOf"]`: both adapters added in
quoin#323 resolved a status through an object literal, so
`STATUS["valueOf"]` returned `Object.prototype.valueOf` rather than
`undefined`, the refusal never fired, and an entry was transcribed whose
outcome was a function.

Failure scenario: a producer emitting `"status": "valueOf"` — or `toString`,
`constructor` — is accepted rather than refused, and a run record is written
with a non-outcome in a field the auditor reads.

Fixed as quoin#329 and released as `v0.23.1`. The tag was bumped, not deleted,
and `0.23.0` is named in this matrix as incompatible so nobody has to work out
later why it does not exist on the registry.

Worth stating plainly: the same property passed on my machine and failed in CI
on a different seed. The property is the only reason this was caught — a fixed
list of statuses I thought of would not have contained `valueOf`.

### FND-002 — the matrix was not in either package

`compatibility-matrix.json` sits beside `manifest.yaml` in the package
directory, and neither `setup.cfg`'s `package_data` nor the npm `files` list
carried it. `scripts/audit_packages.py` caught both, one after the other.

Failure scenario: a consumer installs `engineering-assurance` and the matrix
the gate reads is not there.

Fixed in three places: `setup.cfg` for the wheel, `package.json` plus
`scripts/stage-npm.mjs` for the npm archive, and the audit's own allowlist. The
allowlist entry is named rather than globbed, for the reason recorded in the
comment: a glob over the package root would start shipping whatever anyone
drops beside it.

The npm side needed both `compatibility-matrix.json` and
`engineering_assurance/compatibility-matrix.json` in the allowlist, because npm
treats a bare name in `files` as a pattern matching at any depth — so the
staged root copy and the real one are both packed. `manifest.yaml` has always
behaved this way; the matrix is listed the same way rather than given an
anchored form nobody else uses.

### FND-003 — I checked the wrong registry and reported a false negative

After `v0.23.1` published, I ran `npm view @agent-ix/quoin version` several
times and reported that the publish had not landed. It had. This machine's
`@agent-ix` scope resolves to the internal `npm.ix` mirror, which lags; the
public registry had `0.23.1` as `latest` the whole time.

Failure scenario: a release is treated as failed and re-cut, or a matrix pins
the older version because the mirror said so.

Recorded in the matrix as `registry.mirror_is_not_an_oracle`, asserted by
TC-084, alongside the harder rule the same reading exposed: **`npm.ix` must not
appear in any requirement, pin, lockfile, or `.npmrc`**, because it is
unreachable from CI and a pin naming it cannot be installed or published
outside the network. TC-084 asserts no component's release or version string
contains it.

## Notes

- The classifier is pure and the observer is a separate file. TC-086 asserts
  both directions — no subprocess or write in `compatibility.py`, and
  `subprocess` and `shutil.which` present in the script — so the rules stay
  testable for versions nobody has installed.
- `unknown` covers two different facts: a version the matrix never saw, and a
  component that could not be observed. Both withhold the gate, and neither is
  reported as `incompatible`, because "untested" and "ruled out" are different
  claims.
- The digest check skips an artifact the matrix names and this tree does not
  contain. A consumer's tree is not drift.
- Acceptance is unset and TC-082 requires it to stay that way. An agent that
  filled in `accepted_by` would fail its own gate — which is the point: the
  gate exists to keep an agent from granting itself permission to migrate.
