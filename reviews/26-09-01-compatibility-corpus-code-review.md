---
id: SR-024
title: "Code review — accepted compatibility fixture corpus"
type: SpecReview
analysis: code-review
scope: "engineering_assurance/compatibility_corpus.py, engineering_assurance/fixture_codegen.py, tests/test_compatibility_corpus.py, .gitmodules, corpus (pinned qa-corpus submodule), pyproject.toml, spec/functional/FR-011, spec/tests.md"
review_set: subset
relationships:
  - target: "ix://agent-ix/engineering-assurance/FR-011"
    type: "references"
---

# SR-024: Code review — accepted compatibility fixture corpus

## Summary

Reviews the FR-011 gate added for #9: real PGM-01 records, real producer
output, and one exact Quire-to-Quoin receipt chain, retained in the private
`qa-corpus` repository and pinned here as a submodule read in place. This
replaces the deferral FR-010 carried.

## Verdict

**CONDITIONAL** — no high findings in the change. One high finding is recorded
against the environment; one medium finding records a design correction made
during the work.

## Gates

- `make lint` (ruff) — clean, with the submodule excluded (see Notes).
- `make validate-docs` — clean; FR-011 raises no EARS or quality warning.
- `scripts/check_content_rights.py --tree` — passes, **unmodified**.
- `python3 -m pytest tests/` — 20 new tests pass. The 11 pre-existing failures
  (FND-001) are unchanged.
- `build_compatibility_corpus.py --check`, run inside the submodule — the
  corpus reproduces byte-for-byte from its recorded sources.

## Findings

| ID      | Severity | Summary                                                                      | Refs                        |
| ------- | -------- | ---------------------------------------------------------------------------- | --------------------------- |
| FND-001 | high     | Eleven ix-flow workflow tests fail against the installed CLI, on a clean tree | tests/test_workflows.py:338 |
| FND-002 | medium   | The corpus was first built inside this public repository, against its own policy | AGENTS.md:18                |

## Finding detail

### FND-001 — installed ix-flow emits a payload the tests do not recognise

`tests/test_workflows.py::test_ix_flow_can_load_every_canonical_definition`
fails with `KeyError: 'data'`: the installed ix-flow CLI returns a payload with
no `data` key, and ten `test_workflow_resume.py` cases fail with it.

Failure scenario: every canonical workflow definition this repository owns is
unverified against the CLI that would run it, and the human-decision half of
the shared contract — the half FR-011's chain depends on for its decision
history — has no passing test on this machine.

Not caused here and not fixed here: all eleven reproduce with this change
stashed. Filed as `agent-ix/engineering-assurance#15`. Same class as
`agent-ix/quoin#326`, filed during #322: a pinned contract and an installed CLI
drifted apart, and the direction has to be established by reading both rather
than by relaxing the assertion.

### FND-002 — the corpus was built in the wrong repository first

The first implementation retained the real PGM-01 records, producer output, and
chain artifacts **inside this repository**, and widened
`ALLOWED_URL_PREFIXES` in `check_content_rights.py` to get them past the gate.

`AGENTS.md` says "Use fictional fixtures. Do not commit operational data",
`content-rights.yaml` lists `operational-evidence` as prohibited, and the same
document states that a failed post-publication rights check "requires
repository deletion and a new root". Widening a publishability gate to admit the
work being gated is exactly the move that policy exists to prevent, and the
consequence of getting it wrong here is not a red test.

Corrected before anything was pushed: the corpus now lives in the private
`agent-ix/qa-corpus` repository and is pinned here as a submodule. The
content-rights checker is byte-identical to `main`, and the gate reads real
records without this public tree ever holding one.

Recorded rather than quietly fixed because the failure mode is worth keeping: a
gate that blocks a change is evidence about the change, not an obstacle in it.

## Notes

- `.gitmodules` uses the relative URL `../qa-corpus.git` rather than an absolute
  one. Git resolves it against this repository's own origin, and it keeps the
  content-rights URL rule intact with no allowance added — the absolute form was
  itself flagged.
- `corpus/` is excluded from ruff. It is another repository's tree whose Python
  fixtures are deliberately invalid — an undefined name IS the defect a case
  banks — so linting it would make this gate depend on that repository's house
  style and on its fixtures being valid Python, which they are not allowed to
  be. Same reasoning quoin records for the same submodule.
- TC-078 asserts the checked-out corpus equals the gitlink recorded in the
  index. Without it, a submodule tracking `main` could change the accepted
  fixture set under the gate with no reviewed commit here.
- The gate refuses to pass silently over an uninitialized submodule:
  `load_corpus` raises rather than skipping, so "no corpus" can never read as
  "corpus clean".
- TC-076's static check uses call-shaped tokens (`import subprocess`,
  `.write_bytes(`) rather than words. An earlier word-level check failed on the
  module's own docstring sentence saying it writes nothing.
- The corpus does not claim more than it proves. Its `limitations` state that
  retention proves fixture integrity and mapping behaviour offline and does not
  re-observe the source repositories — the same distinction SR-114 drew for
  `quire-contract-ir` TC-026.
