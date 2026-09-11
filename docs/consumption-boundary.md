# Engineering Assurance v0.3.1 consumption boundary

`v0.3.1` is the current immutable Rust-port beta. It is deliberately a
pre-stabilization release: consumers may depend on it now, but its surface is
expected to change in response to real consumer evidence. It is not a v1.0.0
compatibility commitment.

## Pinning

Use the immutable `v0.3.1` tag, never a branch name or a pull-request head.
For a Rust consumer, pin the repository dependency by tag:

```toml
engineering-assurance = { git = "<Engineering Assurance repository remote>", tag = "v0.3.1" }
```

For the configuration module or CLI, check out `v0.3.1` from this repository's
canonical remote and record the resolved commit alongside the consuming change.
The repository intentionally refuses npm publication, so a tag is the
distribution boundary. Before adopting the tag, run the consumer's relevant
native tests and, where the compatibility matrix is in scope, run
`engineering-assurance compatibility-observe --root <consumer-root>`.

## What a consumer may use

- The `engineering_assurance` Cargo crate and the `engineering-assurance` CLI.
- The versioned JSON request/result interfaces documented by the native boundary
  requirements, including compatibility observation, onboarding, workflow host,
  manifest validation, package audit, and content-rights commands.
- The configuration module rooted at `engineering_assurance/`, including its
  canonical onboarding skill, manifest, schemas, contracts, fixtures, and
  skeletons.

The Rust crate's exported symbols and the named v1 protocol discriminators are
the public surface at this beta tag. Files under `src/` that are binary-host
adapters, test fixtures, internal review artifacts, and the compatibility
matrix's current pins are implementation detail rather than a frozen consumer
contract. No consumer should depend on an unreleased branch commit, a private
package registry, Python implementation behavior, or generated fixture layout.

## Stabilization feedback

Report a boundary defect in the `agent-ix/engineering-assurance` issue tracker
with the title prefix `v0.3.1 boundary:`. Include the tag, resolved commit,
consumer repository, exact interface used, expected and actual typed result,
and a minimal reproduction. The maintainers use those reports as the evidence
set for the later v1.0.0 decision; elapsed time alone is not stabilization.

When an upgrade is published, pin the new immutable tag, rerun the same
consumer checks, and retain any observed compatibility or migration refusal as
boundary evidence.
