# Engineering Assurance consumption boundary

This is deliberately a
pre-stabilization release: consumers may depend on it now, but its surface is
expected to change in response to real consumer evidence. It is not a v1.0.0
compatibility commitment.

## Dependency

For a Rust consumer, add the repository dependency:

```toml
engineering-assurance = { git = "<Engineering Assurance repository remote>" }
```

For the configuration module or CLI, use this repository's
canonical remote. The repository intentionally refuses npm publication, so a tag is the
distribution boundary. Before adopting the tag, run the consumer's relevant
native tests.

## What a consumer may use

- The `engineering_assurance` Cargo crate and the `engineering-assurance` CLI.
- The versioned JSON request/result interfaces documented by the native boundary
  requirements, including onboarding, workflow host,
  manifest validation, package audit, and content-rights commands.
- The configuration module rooted at `engineering_assurance/`, including its
  canonical onboarding skill, manifest, schemas, contracts, fixtures, and
  skeletons.

The Rust crate's exported symbols and the named v1 protocol discriminators are
the public surface at this beta tag. Files under `src/` that are binary-host
adapters, test fixtures, and internal review artifacts are implementation detail rather than a frozen consumer
contract. No consumer should depend on an unreleased branch commit, a private
package registry, Python implementation behavior, or generated fixture layout.

## Stabilization feedback

Report a boundary defect in the `agent-ix/engineering-assurance` issue tracker
with the title prefix `boundary:`. Include the
consumer repository, exact interface used, expected and actual typed result,
and a minimal reproduction. The maintainers use those reports as the evidence
set for the later v1.0.0 decision; elapsed time alone is not stabilization.

When an upgrade is published, rerun the same
consumer checks, and retain any observed compatibility or migration refusal as
boundary evidence.
