# Shared assurance compatibility matrix

The reviewed set of component versions that work together for the contract
campaign. Answers `agent-ix/engineering-assurance#8`.

`engineering_assurance/compatibility-matrix.json` is the machine-readable
source; this document is what a human reads before accepting it.

## The pinned set

| Component | Version | Release | Provides |
| --- | --- | --- | --- |
| quire-cli | 0.31.0 | npm `@agent-ix/quire-cli@0.31.0` | Static assurance export consumed by Quoin intake |
| quire-rs (engine) | 0.46.0 | inside quire-cli 0.31.0 | The export the CLI delegates to |
| quoin | 0.23.1 | npm `@agent-ix/quoin@0.23.1` | Evidence, measurements, change-assurance records, attestations, intake, audit, receipts |
| ix-flow | 0.0.4 | npm `@agent-ix/ix-flow@0.0.4` | Human decision events as an integrity-verified chain |
| engineering-assurance | 0.2.0 | git tag `v0.2.0` | Shared semantics, PGM-01 compatibility mapping, the accepted corpus gate |

Every one is a released artifact. No pin is a branch head, a bare revision, or
a floating tag — that is `FR-012-AC-1`, and TC-079 enforces it.

`engineering-assurance` ships as a source distribution only: its
`prepublishOnly` hook refuses npm publication by design, so its release is the
tag.

## Three answers, and why the third matters

`scripts/check_compatibility_matrix.py` classifies what is installed:

| Verdict | Meaning |
| --- | --- |
| `compatible` | the exact pinned version |
| `incompatible` | a version this matrix names and rules out, with the reason |
| `unknown` | a version this matrix has never seen, **or a component that could not be observed** |

`unknown` is not a polite `incompatible`, and it is certainly not a pass. A
toolchain nobody has tested against this matrix is a fact about the matrix, not
a verdict about the toolchain. The gate requires every component to be
`compatible`; one unobserved component withholds it.

Two quoin versions are named and ruled out:

- **0.22.5** predates the `quoin change-assurance` surface entirely.
- **0.23.0** was tagged and never published. Its release run failed at the test
  leg on `agent-ix/quoin#329`, where an adapter accepted an inherited property
  name as a declared status. The tag was bumped rather than deleted, so it is
  named here to save the next reader the archaeology.

## Registry

Every npm pin above resolves from the **public npm registry**
(`registry.npmjs.org`).

**The internal `npm.ix` mirror must not appear in any requirement, pin,
lockfile, or `.npmrc`.** It is unreachable from CI, so a pin that names it
cannot be installed or published by anyone outside the network.

It is also not an oracle for whether a release landed. While this matrix was
being prepared, `quoin 0.23.1` was live on the public registry and the mirror
still reported `0.22.5` — which read, wrongly, as "the publish failed". Verify a
publish against the public registry.

## Cross-component fixtures

The accepted compatibility corpus (`FR-011`) is the cross-component fixture
set, pinned as the `corpus` submodule gitlink. Its chain carries one Quire
static export through Quoin seal, intake, and audit to a verification receipt,
and every artifact is retained with its digest.

## Upgrade

Order: **quire-cli → quoin → this repository's tag → the corpus gitlink.** Each
step is independently verifiable, and only the last moves the gate.

After each step:

```bash
python3 scripts/check_compatibility_matrix.py
```

It exits non-zero unless every component is `compatible` **and** the matrix
records human acceptance. Those are two independent conditions: a perfectly
pinned toolchain on an unaccepted matrix still exits non-zero, because exit 0
answers "may migration begin", not "are the versions right".

No campaign repository is touched by an upgrade of these components. Migrations
are `agent-ix/engineering-assurance#10`, and they begin only after this matrix
is accepted.

## Rollback

Every pin is a released artifact, so rolling back is installing the previous
release. Nothing here requires a rebuild from source to undo.

| Component | Rollback | What is lost |
| --- | --- | --- |
| quoin | install `@agent-ix/quoin@0.22.5` | the `change-assurance` commands; retained records, attestations, and receipts are unaffected, because 0.23.x added a surface over existing contracts rather than changing the stored layout |
| quire-cli | install `@agent-ix/quire-cli@0.30.2` | the `provenance` command; exports produced by 0.31.0 remain readable |
| engineering-assurance | check out the previous tag | there is none before `v0.2.0`, so rollback means removing the dependency rather than downgrading it |
| corpus | move the gitlink to the earlier commit and re-run the FR-011 gate | nothing; the corpus is content-addressed, so an earlier pin is a complete self-verifying set |

**Nothing in this matrix is irreversible.** No pin migrates data, rewrites
evidence, or changes a stored schema.

## Hosted CI

Publishing these versions enables no automatic hosted CI anywhere. Every
workflow in this campaign remains manual-dispatch only, including the release
workflow that published quoin 0.23.1.

## Acceptance

```json
"accepted": {
  "state": "accepted",
  "accepted_by": "Peter Krenesky",
  "accepted_at": "2026-09-01"
}
```

**Accepted 2026-09-01.** An agent prepared this matrix and did not decide to
accept it; the named human did, and directed an agent to transcribe that
decision here. That distinction is the whole point of the field, so it is worth
stating rather than leaving to the commit log.

TC-082 no longer asserts the fields are unset — it now asserts acceptance is in
one of its two honest shapes: pending with nothing filled in, or accepted with
both a named human and a date. The shape it rejects is a `state` that reads as
accepted while nobody is on record as having accepted it.

This unblocks `#10`.
