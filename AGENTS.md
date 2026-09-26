# engineering-assurance

## Setup

A fresh clone or `git worktree add` does not check out the `corpus/`
submodule (`qa-corpus`) on its own. Run this once per worktree before any of
the checks below -- without it, `rust-clippy`/`rust-tests` fail on missing
`corpus/compatibility/...` fixtures with a "No such file or directory" that
has nothing to do with whatever change is actually being reviewed:

```bash
git submodule update --init --recursive
```

## Required checks

```bash
make lint
make test
make package-audit
```

## Tests are Rust only

New tests go in Rust, never Python. The former Python suite (EA-19) lives in
the single `python_port` test binary (`tests/python_port/`, one module per
retired file); extend it or add a `tests/*.rs` target. Do not add or extend
`tests/test_*.py`.

## Publication boundary

- Keep the repository public; registry publication remains a separate,
  explicitly controlled distribution decision.
- Author content in this repository; do not copy or closely paraphrase external
  publications, private research, prior repository prose, or review evidence.
- Use fictional fixtures. Do not commit operational data, legal-review
  material, external rule inventories, mappings, excerpts, or workstation
  locations.
- Run the full rights check before every commit and push.
- A failed post-publication rights check requires repository deletion and a new
  root; do not rewrite exposed history.
